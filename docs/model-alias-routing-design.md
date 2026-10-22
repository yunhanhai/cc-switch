 # 同模型跨渠道无缝切换（Model Alias Routing）设计
 
 > 目标：一个逻辑模型（如 `sonnet`）在多个渠道各有不同物理名，
 > 任一渠道失败时自动切到下一个渠道，并**自动换用该渠道的物理名**。
 > 对标 yetone/magpie 的「一堆渠道随意切换、同模型无缝」体验。
 
 ## 1. 现状与缺口
 
 当前链路（`handler_context.rs` → `provider_router.rs` → `forwarder.rs`）：
 
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
 
 `select_providers` 增加可选模型上下文：
 
 ```rust
 pub async fn select_providers(
     &self,
     app_type: &str,
     request_model: Option<&str>,   // 新增
 ) -> Result<Vec<Provider>, AppError>
 ```
 
 过滤规则（仅在 `request_model` 有值且该 app 存在 alias 定义时生效）：
 
 ```text
 候选 provider 保留条件：
   - 在 model_aliases 中声明了该 alias，或
   - 有 model_mapping 能匹配该模型（含 * 通配），或
   - 该 alias 在本 app 无任何声明（全量放开，保持旧行为）
 ```
 
 关键安全阀：**若某 app 完全没配 alias，则不做任何过滤**——否则用户升级后
 所有请求都会因为"没有 alias 声明"而被判定为不支持。
 
 ### 2.4 转发阶段按 provider 重映射
 
 `forwarder` 在每次尝试前，用**当前正在尝试的 provider** 解析物理名：
 
 ```rust
 let physical = resolve_upstream_model(app_type, &logical_model, provider);
 body["model"] = physical;
 ```
 
 这里必须放在**重试循环内部**（而不是循环外只算一次），否则切到 P2 时
 仍会用 P1 的物理名。现有 `apply_model_mapping` 调用点在循环外的
  `build_request` 阶段，需要下沉或在循环内重算。
 
 ### 2.5 UI
 
 复用 `ModelMappingEditor` 的交互形态，新增「模型别名」页签：
 
 ```text
 逻辑模型 [sonnet ▾]  ─┬─ 渠道 A → claude-sonnet-4-5-20250929
                       ├─ 渠道 B → anthropic/claude-sonnet-4.5
                       └─ 渠道 C → sonnet-latest
 ```
 
 - 渠道下拉只列出**该 app 下已启用分组内的 provider**，避免配了永远走不到的别名。
 - 支持"从现有 mapping 导入"：扫描各 provider 的 `model_mapping`，按 target
   相似度归并成 alias 建议（可选，第二期）。
 
 ## 3. 改动清单

 | 文件 | 改动 | 风险 |
 | --- | --- | --- |
 | `database/schema.rs` | v20 建表 + 迁移 | 低 |
 | `database/dao/model_aliases.rs` | 新 DAO（CRUD + `resolve` + `providers_for_alias`） | 低 |
 | `commands/model_aliases.rs` | Tauri 命令 5 个 | 低 |
 | `proxy/provider_router.rs` | `select_providers` 加参数 + 过滤 | **中**（签名变更影响 3 处调用） |
 | `proxy/handler_context.rs` | 先解析模型再选路 | **中**（执行顺序调整） |
 | `proxy/forwarder.rs` | 循环内按 provider 重映射 | **中**（核心转发路径） |
 | `lib.rs` | 注册命令 | 低 |
 | 前端 `ModelAliasPanel.tsx` + i18n 四语 | 新 UI | 低 |

 调用点（需同步改）：

 - `handler_context.rs:134`（主路径）
 - `handlers.rs:155`（claude-desktop 网关）
 - `provider_router.rs:553`（单测）

 ## 4. 兼容与回滚

 - **零配置升级安全**：无 alias 数据时行为与现在完全一致（2.3 的安全阀）。
 - **回滚**：删表不影响任何既有表；`select_providers` 传 `None` 等价旧行为。
 - **观测**：`failover_events.reason` 增加 `model_alias_switch` 值，时间线可见
   "因模型不匹配跳过渠道 X"。

 ## 5. 分期

 | 阶段 | 内容 | 价值 |
 | --- | --- | --- |
 | **P1** | 数据模型 + DAO + `resolve` + 单测 | 打通数据层，可先只读验证 |
 | **P2** | `select_providers` 过滤 + `forwarder` 重映射 | 核心效果生效 |
 | **P3** | UI + i18n + 事件时间线 | 可配置、可观测 |
 | **P4** | 从现有 mapping 自动归并建议 | 降低配置成本 |
 
 ## 6. 未决问题

 1. **Claude Code 的 `[1M]` 后缀**：alias 的 `target_model` 是否允许携带？
    倾向允许，转发前统一由 `strip_one_m_suffix_for_upstream` 剥离。
 2. **Codex Official 渠道**：`provider_supports_failover` 已禁止其参与重试，
    alias 是否需要支持？倾向支持（单渠道场景下仍有意义），但不进重试链。
 3. **Claude Desktop**：走独立的 `map_proxy_request_model`，是否纳入 alias？
    倾向第二期再评估，先不改变其"未知 route 直接报错"的严格语义。
 
