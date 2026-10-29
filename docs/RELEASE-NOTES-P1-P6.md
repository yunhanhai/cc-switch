 # 发版草稿 — 同模型跨渠道无缝切换 P1–P6
 
> 分支：`feature/v3.20.1-custom-routing`（基于 `v3.20.1`）。最新提交以 `git log -1` 为准
> ——本文档**不再内嵌自身提交哈希**（自指哈希永远滞后：修改头部即改变该提交的哈希）。
> 交付件目录同样以构建时的 `HEAD` 短哈希动态命名，见 `AGENTS.md`。
> 状态：P1–P6 全部完成；后续追加 Claude→Codex 转换、集成测试基建修复与同一供应商重试（见"追加篇章"）。
> `cargo test --lib 2849 passed` / 集成 128 / 前端 `vitest 串行 139 文件 1075 用例` / `typecheck` / `format` 全绿，`git status` 干净。
 
 ## 摘要
 
 一个逻辑模型（如 `sonnet` / `gemini-pro`）在多个渠道各有不同物理名时，配置一次即可：
 - 选路阶段跳过不认识该逻辑模型的渠道
 - 转发阶段按当前渠道把模型名换成该渠道的物理名
 - 渠道失败自动切到下一渠道，模型名跟着换，用户侧无感（对标 yetone/magpie）。
 
 Gemini 的模型名在 URL 路径中（`/v1beta/models/<model>:generateContent`），P6 已通过路径重写使过滤与换名在路径维度成对，故障转移时路径模型名随渠道一起切换。
 
 ## 提交流水
 
 | 阶段 | 提交 | 内容 |
 | --- | --- | --- |
 | P1 | `fbddf72` | 数据层 `v20`：`model_aliases` 表 + 索引 + DAO |
 | P2a | `458f2d2` | 选路过滤：`select_providers_for_model` + 双安全阀（无配置不过滤 / 全滤回退完整链） |
 | P2b | `7590560` | 转发重映射：`resolve_alias_model` 优先级插到最前 |
 | P3 | `51bb23c` | 6 个 Tauri 命令 + `ModelAliasPanel` + 四语（20 键） |
 | P4 | `2f15108` | 归并建议：`suggest_aliases_from_mappings`（同 source 跨渠道归并） |
 | P5 | `f5709d8` / `cea6cb2` | Gemini 临时成对退化与前端禁用（为 P6 前置） |
 | P6 | `1e0244f` | **Gemini 路径别名打通**：`supports_model_alias` + `rewrite_gemini_model_in_path`（保留 `:action`/`?query`，list 端点 `None`）+ `correct_providers_for_gemini_model`（`handle_gemini` 在 `with_model_from_uri` 后用真实模型重筛）+ `forwarder` 路径重写与 `outbound_model` 刷新，前端已放开 |
 | — | `ad555ff` | 文档补齐 P6 哈希 |
 | — | `640dd00` | 清理 `geminiUnsupported` 四语死键 |
 | — | `4071f39` | 修正文档 20 键一致 |
 | — | `44034d1` | 落盘本草稿 `docs/RELEASE-NOTES-P1-P6.md` |
 | — | `349f00f` | 补 `groupModelAliases` 前端单测（4 用例，面板渠道行序/排序/字段投影） |
 | — | `ba34c90` | 同步本发版说明至 `349f00f`（含测试说明与交付路径） |
 | — | `a5bd6a29` | 对齐发版说明至 `ba34c90`；记录 bundle `clone` 验证（`fetch --unshallow` 修复浅克隆） |
 | — | `b5ba7dc5` | 补 Gemini 选路纠正端到端测试（3 用例，两段式选路/未知模型保链/非 Gemini no-op） |
 | — | `299b3b04` | 补 Gemini 路径重写决策测试（3 用例）并合并 P6 重复注释 |
 | — | `0cfd60e9` | 发版说明同步至 `299b3b04`，新增 P1–P6 覆盖矩阵 |
 | — | `9d576b07` | **修复**：Gemini 选路纠正改为不可失败（原返回 `Err` + `?` 会把可服务请求打成 500），并补对应测试 |
 | — | `39304a12` | 记录已核实的 Gemini full-URL 路径重写缺口与修法方向 |
 | — | `2fdbdf9f` | **修复**：full-URL 模式补齐 Gemini 路径别名（三重守卫），使「过滤与换名成对」覆盖全部 URL 组装分支 |
 | — | `0716521a` | 记录 Claude + `gemini_native` 无需路径重写的审查结论（模型名来自 body，已天然成对） |
 | — | `cebf2ace` | 清理 `zh-TW` 孤儿键 `settings.oneClickInstall`（上游遗留，无调用点） |
 | — | `2daeba47` | 新增 i18n 全局守卫：四语键集合双向比对 + 非 `pi.*` 占位符一致性（6 用例，已负向验证） |
 | — | `844169c8` | 记录连通性检测与 alias 无关的审查结论（只探 base_url，不涉模型名/熔断器） |
 | — | `73c1abcf` / `1cf66bee` / `d6164e5d` | **修复归因契约**：`outbound_model` 改取最终 URL 真值（前两次按 alias 记账推断在 full-URL 守卫拒绝时会错报物理名），并抽出 `resolve_gemini_outbound_model` + 4 用例 |
 | — | `c2ac2295` | 设计文档固化 `outbound_model` 真值来源契约（第 8 条） |
 | — | `ef1af4a3` | 审计无调用点的 CRUD 入口：DAO 层全部在用（夹具/安全阀断言），命令层保留并写明理由 |
 | — | `7e183b44` | **修复**：三处渠道模型覆写（GrokBuild + Codex Chat/Anthropic）在 alias 命中时不再覆写，避免冲掉刚换好的物理名 |
 | — | `4928e85e` | 固化「alias 不得被渠道级改写覆盖」不变量（设计文档 §6 第 9 条）并交叉引用相关测试 |
 | — | `05c7e9ff` | 审计 `forwarder` 全部 4 处 `model` 写入点，记录两处 Copilot 有意例外 |
 | — | `8f90c6b9` | 审计扩至整个 `proxy` 模块（13 处）：`model_mapper` 在 alias 命中时不可达，`transform_*` 为纯搬运 |
 | — | `faaef158` | 补全审计边界：模块外 5 处已核查且不在 alias 路径；措辞由"零越权"改为"零未记录越权" |
 | — | `bcf151d0` | 发版说明同步至 `faaef158`（写入点审计留痕） |
 | — | `b397db32` | **修复（独立于 alias 的既有缺陷）**：`ModelAlias` 与 `RoutingGroup` 系列缺 serde rename，Tauri 返回值是 snake_case 而前端读 camelCase，导致 UI 中 `appType`/`providerId`/`targetModel`/`sortOrder` 等**静默为 undefined**；补 `rename_all = "camelCase"` + 2 条契约测试 |
 | — | `ffedc2c7` | 发版说明同步至 `b397db32`（serde 命名修复） |

 ## 追加篇章 A：集成测试基建修复（`afcb678d`）

 此前 `src-tauri/tests/` 12 个集成测试文件**从未运行过**，两层障碍一次清除：

 1. **rustc 编译崩溃**：编译大测试目标（`support.rs`/`provider_service.rs`）时栈溢出
    （`STATUS_STACK_BUFFER_OVERRUN`，exit 0xc0000409）。解法：`RUST_MIN_STACK=33554432`（32 MB）。
 2. **路径错位（根因）**：`build.rs` 无条件注入 `cc_switch_dev_build`，而 `cfg(test)`
    只对 lib 自身单元测试为真——集成测试编译 lib 时走 `.cc-switch-dev` 布局，夹具按
    `.cc-switch` 读写。修复：`config.rs` 新增集中判定 `uses_dev_isolated_layout()`
    （= dev 构建 && 非 `#[cfg(test)]` && 非 `CC_SWITCH_TEST_HOME` 沙盒），全部 16 处
    dev 隔离分支（目录/端口 15721↔15722/scheme）从编译期 `#[cfg]` 改为运行时判定。
    **dev 版正常运行的路径/端口/scheme 行为完全不变**。

 结果：集成 128 用例全绿（12 文件）；`tests/skill_sync.rs` 的 symlink 用例按仓库既有
 模式在无 SeCreateSymbolicLinkPrivilege 的 Windows 上优雅跳过（错误 1314）。

 ## 追加篇章 B：Claude → Codex 供应商一键转换（`079791a7` 起）

 Claude 第三方供应商卡片上的 ⇄ 按钮，把端点 + API Key + 模型名转换为 Codex 供应商。
 **协议依据**（上游源码取证）：`openai/codex` 的 `WireApi` 枚举只认 `"responses"` 与
 `"anthropic"`（`chat` 已于 2026-02 移除），故转换产物 `wire_api = "anthropic"` 是
 目标 CLI 的官方一等公民配置。完整协议审计与运行条件见
 `docs/claude-to-codex-conversion.md`。关键提交：

 | 提交 | 内容 |
 | --- | --- |
 | `079791a7` | 功能落地：转换按钮（仅第三方卡片）+ payload 构造 + 四语 |
 | `a30d5c4b` | **协议修复**：写 `wire_api = "anthropic"` + `meta.apiFormat`（原 `"responses"` 会对 Anthropic 网关发 `/responses` 必然 404） |
 | `e6d67d9e` | **模型搬运**：搬源 `ANTHROPIC_MODEL` 为 config 顶层 model（桥接路径会覆写请求模型为此值） |
 | `08f05e9c` | `ANTHROPIC_API_VERSION`（`2023-06-01`）抽为共享常量 + 契约测试 |
 | `23c7d165` | 防重（同名同密钥不重复创建）+ 成功 toast 与 notes 声明"需先启动本地路由" |
 | `a2679a59` / `eea6417d` | **运行时证据**：两个回环端到端测试（真实 ProxyServer + 真实 forwarder → mock Anthropic 上游），wire 级断言路径重写/版本头/凭证交换/Accept 归一化/零指纹泄漏/system 升级/响应回转，含流式 SSE 事件生命周期 |
 | `0b3e08dc` | **双协议探测**：Anthropic 端点带已知兼容后缀时剥离并探测 OpenAI 候选 `/v1/models`，命中则生成 `wire_api="responses"` 直连产物（无需本地路由），失败静默回退桥接 |

 测试：转换 20 例 + 直连模式 11 例 + UI 门控 6 例 + 回环端到端 2 例 + locale 守卫。
 
 ## 追加篇章 C：同一供应商失败后重试（`9bb9a646`）

 原有 `max_retries` 数的是「最多换几家」：故障转移关闭或队列里只有一家时，一次瞬时失败
 （连接断开、上游 5xx/524）就直接报错，客户端得手动重试。新增独立配置 `same_provider_retries`：
 `forward()` 在响应交付给客户端**之前**失败时，退避后重发同一家，用完次数才走原有故障转移。
 需求原文见上级目录 `PROMPT-同供应商重试.md`。

 | 项 | 行为 |
 | --- | --- |
 | 重试边界 | 只在 `forward()` 返回 `Err` 时重发；流式要等首个 chunk 到达才返回 `Ok`，已交付的流中途出错不重试 |
 | 配额与熔断 | 不占 `attempted_providers`；熔断器在本家重试结束后按最终结果只记一次 |
 | 默认与上限 | 默认 0（关）；退避 1000ms 起翻倍、8000ms 封顶；后端夹到 10 次 / 60000ms |
 | 迁移 | 三列走 `ensure_custom_columns`，`SCHEMA_VERSION` 不变（20）；`backup.rs` 按 `PRAGMA` 动态取列，无需改 |
 | 日志 | `FWD-004` |

 与需求原文的差异（均为有意）：
 - 故障转移关闭时**仍然**重试同一家（原文 §3.2 写强制为 0；单供应商正是要解决的场景，用户 2026-10-29 确认）。
 - 默认 0 而非 2：升级后行为不变，需要时手动开启（用户确认）。
 - 只重试瞬时故障：连接失败与上游 408/409/425/5xx。**超时不重试本家**：这次已经等满超时，
   再等只会把失败耗时翻倍（故障转移关闭时单次兜底 600s），非流式读 body 超时还会重复计费；换家照旧。
   401/402/403/404 等确定性 4xx、认证/配置/转换错误、额度耗尽、429（有独立的等待重试）和
   整流器能处理的错误也不重试本家。

 测试：`cargo test --lib -- same_provider` 共 12 例，其中 3 例用本地 mock 上游驱动真实重试循环
 （用尽次数后放弃且熔断只记 1 次、第三次恢复成功、重试不占换家配额），另在全新安装迁移链上断言定制列存在。
 三处关键行为（不占换家配额、超时不重试、整流排除）做过变异验证：植入回归后对应用例均失败。

 已知限制（本次未处理）：
 - 上游在 HTTP 2xx 里返回的失败信封（`overloaded_error`、`response.failed`）归为 `TransformError`，不重试本家；
 - 托管账号（Copilot / Codex OAuth / xAI）取 token 时的网络抖动被包成 `AuthError`，不重试本家；
 - Codex 官方路由的错误一律判为 `NonRetryable`，同一供应商重试对它不生效。

 ## 行为与兼容
 
 - **零配置升级安全**：某 app 完全没配 alias 时不过滤；过滤后为空时回退完整链，宁可让上游返回 `model_not_found` 也不本地静默 `NoAvailableProvider`。
 - **优先级**：`alias 声明 > 自有 model_mapping（含 * 通配）> env 映射 > 原样`；存量配置零迁移。
 - **回滚**：`select_providers(..., None)` 等价旧行为；删表不影响既有表。
 - **观测**：跳过渠道打 `跳过渠道 X：未声明模型 M`（debug）；命中打 `model alias: alias → target (渠道 Y)`。
 - **Gemini 两段式选路**：`new()` 先按 body 选路（Gemini 此时不过滤），URI 解析后由
   `correct_providers_for_gemini_model` 按真实模型收窄。该纠正**不可失败**：
   任何异常只记 `warn` 并保留原链，绝不把可服务请求打成 500（见 `9d576b07`）。
 - **Gemini full-URL 渠道**：`is_full_url` 模式下 URL 会覆盖 endpoint，因此路径别名
   在 `rewrite_gemini_full_url_for_alias` 中补做，三重守卫（命中 alias、URL 含
   `models/<id>` 段、该段仍是本次逻辑模型）；任一不满足即保持原 URL（见 `2fdbdf9f`）。
 - **计费归因真值契约**：`outbound_model`（供 usage 归因）必须取自真实出站载体——
    body-based app 取定稿 `filtered_body["model"]`，Gemini 取最终拼装的 `url`；
   **不得**从 alias 记账推断（见 `d6164e5d` 与设计文档 §6 第 8 条）。
 - **alias 换名不变量**：alias 命中的请求，出站 `body["model"]` 必须等于该渠道的
   alias 物理名。三处渠道级模型覆写（GrokBuild 与 Codex 两条转换路径）已加守卫；
   任何新增的"渠道级模型改写"都要遵守它（见 `7e183b44` 与设计文档 §6 第 9 条）。
 - **Tauri 返回值字段名**：Tauri **只转换命令参数，不转换返回值**；返回值必须靠
   `#[serde(rename_all = "camelCase")]`（或 DTO 层）自己保证字段名。已修
   `ModelAlias` / `RoutingGroup` 两处，并加契约测试防回归（见 `b397db32`）。
 
 ## 交付件

 - 主交付：`D:\dev-cache\cc-switch-patches-<HEAD 短哈希>/`（`git format-patch v3.20.1..HEAD`，`git am` 可完整还原）
  ——目录名随构建时的 `HEAD` 变化，不在此固定（固定即过期）。
 - 备件：`D:\dev-cache\cc-switch-feature-v3.20.1-custom-routing.bundle`（`--all`，含完整历史）
 - 两者均已演练（2026-10-27）：patch 走 `git am` 还原树哈希一致；bundle `git clone` 后 commit hash 完全一致
   （本机已 `fetch --unshallow` 修复浅克隆；`git bundle verify` 对浅仓库会误报，须以真实 `clone` 为准）。
 - 远端：`origin` 为上游 `farion1231/cc-switch`，当前账号 `yunhanhai` 直接 `push` 为 403 属预期 fork 流程；`gh` 未安装，需 `gh auth login` 后 `fork + push + 建 PR`。
 - **i18n 一致性**：四语完全等值；`localeCoverage.test.ts` 已加全局守卫，
   今后漏译或多余/拼错键会被 CI 直接拦下（见 `2daeba47`）。

 ## 测试说明

 - 前端全量默认并行会误报：`tests/integration/App.test.tsx` 的 2 个用例因 jsdom 环境并行开销超时，放宽 `--testTimeout` 无效。判定真伪请用 `npx vitest run --no-file-parallelism`（2026-10-29：139 文件 / 1075 用例全绿）。
 - 另有 i18n 全局守卫用例（`tests/config/localeCoverage.test.ts`，15 例，含四语键集合与占位符一致性）。
 - **集成测试**：`$env:RUST_MIN_STACK = "33554432"` 后 `cargo test --tests` 可跑全部 12 文件（128 用例；rustc 编译大测试目标会栈溢出，见追加篇章 A）。
 - **P1–P6 覆盖矩阵**（P6 的「过滤」与「换名」两半均有端到端测试）：

  | 环节 | 位置 | 用例数 | 关键断言 |
  | --- | --- | --- | --- |
  | 纯函数解析 | `proxy::model_alias` | 11 | 命中/未声明/空白 target/空索引安全阀/归并规则与过滤 |
  | 选路过滤 | `proxy::provider_router` | 3 | 无配置不过滤、跳过未声明渠道、有自有映射保留、全滤回退 |
  | Gemini 选路纠正 | `proxy::handler_context::tests::gemini_correction` | 4 | 两段式筛选、未知模型保链、非 Gemini no-op、纠错失败仍保链（不 500） |
  | 路径重写（纯函数） | `proxy::handler_context` | 4 | 保留 `:action`/`?query`、list 端点返回 `None` |
  | 路径重写（转发决策） | `proxy::forwarder` | 3 | 命中改写、未命中不动、仅 Gemini 生效 |
  | 路径重写（full-URL） | `proxy::forwarder` | 4 | 钉在逻辑模型时改名、未命中/钉在其它模型/无 `models` 段均不动 |
  | 计费归因（Gemini） | `proxy::forwarder` | 4 | 取最终 URL 真值：重写后/拒绝改写/被钉住/无模型段 |
  | 别名聚合（前端） | `tests/lib/api/modelAliases.group.test.ts` | 4 | 分组、组内渠道序、别名排序、字段投影 |

   `cargo test --lib -- model_alias` / `-- handler_context` / `-- proxy::forwarder::tests` 与
  `npx vitest run tests/lib/api/modelAliases.group.test.ts` 可分别复现。
 
 ## 下一步
 
 1. `gh` 安装后自动 fork 推送建 PR（含本草稿正文）
 2. 或提供已有 fork 地址后加远端推送
 3. 端到端手动验证：Starlight 多渠道 + Gemini CLI 真实重写
 4. 同一供应商重试手动验证：配一个必然返回 500 的 baseUrl、`sameProviderRetries=2`，日志应出现
    3 次请求与 2 条 `FWD-004`；正常地址无额外延迟；关闭故障转移后仍重试；切到 en/ja 确认文案已翻译
 
