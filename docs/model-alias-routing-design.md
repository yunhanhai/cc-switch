 # 同模型跨渠道无缝切换（Model Alias Routing）设计

 > 目标：一个逻辑模型（如 `sonnet`）在多个渠道各有不同物理名，
 > 任一渠道失败时自动切到下一个渠道，并**自动换用该渠道的物理名**。
 > 对标 yetone/magpie 的「一堆渠道随意切换、同模型无缝」体验。

 **状态：已实现（P1–P6 全部完成）**

 | 阶段 | 提交 | 内容 |
 | --- | --- | --- |
 | P1 | `fbddf72` | schema v20 建表 + DAO + 纯函数解析层 |
 | P2a | `458f2d2` | 选路阶段按请求模型过滤渠道 |
 | P2b | `7590560` | 转发阶段按当前渠道重映射模型名 |
 | P3 | `51bb23c` | Tauri 命令 + 编辑 UI + 四语文案 |
 | P4 | `2f15108` | 从现有 `model_mapping` 归并建议 |
 | P5 | `f5709d8` / `cea6cb2` | Gemini 安全兜底（成对退化）→ 路径重写前置 |
 | P6 | `1e0244f` | Gemini URL 路径别名：选路纠正 + 路径重写 + 前端放开 |

 ## 1. 现状与缺口

 改造前的链路（`handler_context.rs` → `provider_router.rs` → `forwarder.rs`）：
 
 ```text
 请求 model="claude-sonnet-4-5"
   → select_providers(app_type)          # 只按渠道顺序，不看模型
   → provider = providers[0]
   → apply_model_mapping(body, provider) # 用该 provider 自己的 model_mapping
   → 失败重试 providers[1]               # 仍用 providers[1] 自己的 mapping
 ```
 
 缺口：
 
 1. **选路不看模型**：渠道 B 即便不支持该模型，也会被当作候选。
 2. **映射是"每渠道各自为政"**：`settings_config.model_mapping` 的 `source` 是
    客户端原始名，不是跨渠道共享的逻辑名。切到 B 时若 B 没配同名规则，
    原始名会被直接发给 B → 404 `model_not_found`。
 3. **选路早于模型解析**：`handler_context` 先选路（第 134 行）后取
    `request_model`（实际在更早的 115 行取 body，但 `select_providers`
    拿不到它），签名里没有 model 参数。
 
 ## 2. 设计
 
 ### 2.1 数据模型（SCHEMA_VERSION 20）
 
 ```sql
 CREATE TABLE model_aliases (
   id           TEXT NOT NULL,
   app_type     TEXT NOT NULL,
   alias        TEXT NOT NULL,          -- 逻辑名，如 "sonnet" / "fast"
   provider_id  TEXT NOT NULL,
   target_model TEXT NOT NULL,          -- 该渠道的物理名
   priority     INTEGER NOT NULL DEFAULT 0,
   enabled      INTEGER NOT NULL DEFAULT 1,
   created_at   INTEGER NOT NULL,
   updated_at   INTEGER NOT NULL,
   PRIMARY KEY (id, app_type),
   UNIQUE (app_type, alias, provider_id)
 );
 CREATE INDEX idx_model_aliases_lookup
   ON model_aliases(app_type, alias, enabled, priority);
 ```
 
 设计取舍：
 
 - **一个 alias 对多 provider**：`(app_type, alias)` 是逻辑模型，
   `provider_id` 是渠道，`target_model` 是该渠道的物理名。
 - **`priority` 而非复用 `routing_groups.sort_order`**：渠道顺序来自分组
   （用户已配置的 P1→Pn），alias 只表达"这个渠道上该逻辑模型叫什么"，
   不应再引入第二套渠道排序。`priority` 仅用于同一 provider 多物理名时的
   兜底选择（少见，保留扩展性）。
 - **不放进 `settings_config.model_mapping`**：那份配置是 provider 私有的
   JSON blob，无法跨 provider 查询、无法做唯一约束、无法在选路阶段高效过滤。
 
 ### 2.2 解析优先级（避免破坏现有行为）
 
 对请求模型 `M` 与 provider `P`，解析顺序：
 
 ```text
 1. model_aliases 中 (app_type, alias = M, provider_id = P) 有记录
      → 用 target_model
 2. P 的 settings_config.model_mapping 命中 M（现有逻辑，含 * 通配）
      → 用其 target（向后兼容）
 3. ModelMapping::from_provider 的 env 映射（现有逻辑）
 4. 原样发送 M
 ```
 
 即：**alias 优先，旧映射作为回退**，存量配置零迁移成本。
 
### 2.3 选路阶段按模型过滤

新增 `select_providers_for_model`，原 `select_providers` 改为以 `None` 委托，
**因此 3 处既有调用点无需改动**（比原计划的签名变更风险更低）：

```rust
pub async fn select_providers_for_model(
    &self,
    app_type: &str,
    request_model: Option<&str>,
) -> Result<Vec<Provider>, AppError>

pub async fn select_providers(&self, app_type: &str) -> Result<Vec<Provider>, AppError> {
    self.select_providers_for_model(app_type, None).await
}
```

过滤规则（仅在 `request_model` 有值且该 app 存在 alias 定义时生效）：

```text
候选 provider 保留条件：
  - 在 model_aliases 中声明了该 alias，或
  - 有自有 model_mapping / env 映射（交给旧链路解析），或
  - 该 alias 在本 app 无任何声明（全量放开，保持旧行为）
```

关键安全阀：**若某 app 完全没配 alias，则不做任何过滤**——否则用户升级后
所有请求都会因为"没有 alias 声明"而被判定为不支持。

第二个兜底：**过滤后若一个渠道都不剩，回退到未过滤的完整链**。
宁可让上游返回真实的 `model_not_found`（用户能看懂），也不要在本地静默
返回 `NoAvailableProvider`（用户无法据此排查是 alias 配错了）。

### 2.4 转发阶段按 provider 重映射

`forwarder` 在每次尝试前，用**当前正在尝试的 provider** 解析物理名：

```rust
let alias_resolved = body.get("model").and_then(Value::as_str)
    .and_then(|requested| self.resolve_alias_model(provider, requested));

let mapped_body = if claude_desktop {
    map_proxy_request_model(...)
} else if let Some(target) = alias_resolved {
    let mut aliased = body.clone();
    aliased["model"] = Value::String(target);
    aliased
} else {
    apply_model_mapping(body.clone(), provider)  // 旧链路
};
```

**关键实现发现**：`forward()` 本来就接收"当前正在尝试的 provider"作为参数，
且重试循环里每个 provider 都会重新调用它。所以「切换渠道时模型名跟着换」
天然成立，只需把 alias 解析插到映射链路最前面 —— 不需要重构循环结构。

索引通过 `with_model_aliases()` 在请求入口注入一次（`create_forwarder`），
转发时在内存里查；字段默认 `None`，因此所有既有构造点与测试行为不变。
 
### 2.5 UI

`ModelAliasPanel` 挂在代理页每个已接管 app 的路由分组下方：

```text
模型别名                                      [已启用]
同一个逻辑模型在不同渠道叫法不同时，配置这里即可跨渠道无缝切换

[从现有映射生成建议]

┌──────────────────────────────────────────────────┐
│ sonnet                                3 个渠道 ✏ 🗑 │
│   渠道A  →  claude-sonnet-4-5-20250929            │
│   渠道B  →  anthropic/claude-sonnet-4.5           │
│   渠道C  →  sonnet-latest                         │
└──────────────────────────────────────────────────┘
```

- 渠道下拉只列出该 app 的真实 provider；保存时后端还会校验归属（拒绝跨 app）。
- **整组保存**（`set_model_alias_entries`）而非逐条增删：逐条保存会让中间状态
  （少一个渠道）短暂生效，而这段时间里的请求可能选到错误渠道。
- 「从现有映射生成建议」按需扫描（不每次进页面都跑），见 §5.1。

### 2.6 Gemini 路径别名（与 body 别名成对）

Gemini 的模型名在 URL 路径中（`/v1beta/models/<model>:generateContent`），
因此 alias 的「过滤」与「换名」都要在**路径**上成对实现：

- **选路**：`RequestContext::new` 对 Gemini 先不过滤，`handle_gemini` 在
  `with_model_from_uri` 解析出真实模型后调用
  `correct_providers_for_gemini_model` 用真实模型重筛一次（`select_providers_for_model`）。
- **转发**：`forwarder::forward` 在映射阶段后，用 `rewrite_gemini_endpoint_for_alias`
  将 `endpoint` 路径中的 `<model>` 替换为该渠道的物理名（保留 `:action` 与 `?query`），
  再进入 `effective_endpoint → url` 拼装；同时刷新 `outbound_model` 用于计费归因。

`model_name_lives_in_body("gemini") == false` 仍用于区分路径/体；但
`supports_model_alias("gemini") == true`，使 Gemini 的过滤与索引注入与其它
app 保持一致。P5 的临时退化与前端禁用已在 P6 移除。

> **成对原则**：过滤与换名必须同时成立或同时不成立。Gemini 通过路径重写
> 使二者在路径维度成对，故障转移时路径模型名随渠道一起切换。

## 3. 实际改动清单

| 文件 | 改动 |
| --- | --- |
| `database/schema.rs` | v20 建表 + `migrate_v19_to_v20`（纯增量） |
| `database/dao/model_aliases.rs` | 新 DAO：upsert / set-entries / resolve / providers_for_alias / 批量索引 |
| `proxy/model_alias.rs` | 纯函数：`resolve_for_provider` / `provider_supports_alias` / 归并建议 |
| `proxy/model_mapper.rs` | 新增 `provider_has_own_model_mapping` / `provider_mapping_pairs` |
| `proxy/provider_router.rs` | 新增 `select_providers_for_model` + 过滤 + 全滤回退 |
| `proxy/handler_context.rs` | 选路传模型名 + 注入 alias 索引到 forwarder |
| `proxy/forwarder.rs` | `resolve_alias_model` + 映射优先级插到最前 |
| `commands/model_aliases.rs` | 6 个 Tauri 命令（含 suggest） |
| `src/lib/api/model-aliases.ts` | API + `groupModelAliases` 聚合函数 |
| `src/lib/query/model-aliases.ts` | 4 个 hooks |
| `src/components/proxy/ModelAliasPanel.tsx` | 编辑 UI + 建议采纳 |
| `src/i18n/locales/{zh,en,ja,zh-TW}.json` | `modelAliases.*` 20 个键（P6 清理 `geminiUnsupported` 后） |
| `src/components/proxy/ProxyPanel.tsx` | 挂载面板 |

**调用点无需改动**：`handler_context` 的主路径改为传模型名，
`handlers.rs` 的 claude-desktop 网关与既有单测继续用 `select_providers`
（等价 `None`，不过滤）。

## 4. 兼容与回滚

- **零配置升级安全**：无 alias 数据时行为与现在完全一致（2.3 的安全阀）。
- **回滚**：`select_providers` 传 `None` 等价旧行为；删表不影响任何既有表。
- **观测**：被跳过的渠道会打 debug 日志
  `跳过渠道 X：未声明模型 M 且无自有映射`；alias 命中会打
  `model alias: sonnet → xxx (渠道 Y)`。
  （`failover_events.reason` 暂未新增 `model_alias_switch`，因为跳过发生在
  选路阶段、还没进入"切换"语义，等有明确 UI 需求再加。）

## 5. 分期

全部完成（见文首提交表）。

### 5.1 归并建议规则（P4）

**同一个 `source` 出现在 2 个及以上渠道时，它就是事实上的逻辑模型** ——
用户已经在用同一个名字打不同渠道，只是还没显式声明成一组。

三条保守规则：

| 情形 | 处理 | 原因 |
| --- | --- | --- |
| 只出现在 1 个渠道 | 跳过 | 只是该渠道私有的改名，归并成 alias 反而会限制它 |
| 含 `*` 的通配 source | 跳过 | 它表达"兜底"而非某个具体模型，归并会改变语义 |
| 同渠道内重复 source | 只保留第一条 | 与 `apply_model_mapping` 的"先配置的精确规则优先"一致 |

已配置过的 alias 会被过滤，避免重复建议。
 
## 6. 未决问题

1. **Claude Code 的 `[1M]` 后缀**：alias 的 `target_model` 允许携带，
   转发前由 `strip_one_m_suffix_for_upstream` 统一剥离（已实现）。
2. **Codex Official 渠道**：`provider_supports_failover` 仍禁止其参与重试，
   alias 对它只影响模型名、不影响重试链（已实现，符合预期）。
3. **Claude Desktop**：仍走独立的 `map_proxy_request_model`，**未纳入 alias**
   —— 它的语义是"未知 route 直接报错"，比 alias 的"回退到旧链路"更严格，
   合并会削弱这层保护。若将来要支持，需要先定义"Desktop route 未声明"时的
   行为，属于独立设计。
4. **`priority` 字段目前恒为 0**：`set_model_alias_entries` 按数组下标写入，
   实际排序由 UI 顺序决定。保留该字段是为将来"同一渠道多物理名"的扩展。

5. **Gemini + full-URL 模式的路径重写缺口（已核实，未修）**：
   `forwarder` 中 `is_full_url == true` 的渠道走
   `append_query_to_full_url(&base_url, ...)`，只保留 base_url、**丢弃
   `effective_endpoint`**，因此 P6 的 `endpoint_for_routing` 重写在该分支不生效——
   若这类渠道同时配了 alias，会出现「选路按逻辑模型过滤、URL 里仍是逻辑模型名」。
   影响面评估：Gemini 表单（`GeminiFormFields`）调 `EndpointField` 时**未传
   `showFullUrlToggle`**（默认 `false`），故 UI 无法为 Gemini 打开该模式；
   但 `meta.isFullUrl` 可来自历史数据或手写 JSON，故分支并非绝对不可达。
   若将来确认需要支持，应让 `append_query_to_full_url` 分支也应用路径重写
   （以 base_url 中是否存在 `models/<id>` 段判定，无该段则保持原样不猜测）。
 
