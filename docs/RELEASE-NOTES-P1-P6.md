 # 发版草稿 — 同模型跨渠道无缝切换 P1–P6
 
> 分支：`feature/v3.20.1-custom-routing`（基于 `v3.20.1`），最新 `c2ac2295`。
> 状态：P1–P6 全部完成，`cargo test --lib 2831 passed` / `vitest 串行 135 文件 1031 用例` / `typecheck` / `format` 全绿，`git status` 干净。
 
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
 
 ## 交付件

 - 主交付：`D:\dev-cache\cc-switch-patches-c2ac2295/`（`git format-patch v3.20.1..HEAD`，`git am` 可完整还原）
 - 备件：`D:\dev-cache\cc-switch-feature-v3.20.1-custom-routing.bundle`（`--all`，含完整历史）
 - 两者均已演练：patch 走 `git am` 还原树哈希一致；bundle `git clone` 后 commit hash 完全一致
   （本机已 `fetch --unshallow` 修复浅克隆；`git bundle verify` 对浅仓库会误报，须以真实 `clone` 为准）。
 - 远端：`origin` 为上游 `farion1231/cc-switch`，当前账号 `yunhanhai` 直接 `push` 为 403 属预期 fork 流程；`gh` 未安装，需 `gh auth login` 后 `fork + push + 建 PR`。
 - **i18n 一致性**：四语现各 3005 键、完全等值；`localeCoverage.test.ts` 已加全局守卫，
   今后漏译或多余/拼错键会被 CI 直接拦下（见 `2daeba47`）。

 ## 测试说明

 - 前端全量默认并行会误报：`tests/integration/App.test.tsx` 的 2 个用例因 jsdom 环境并行开销超时（实测并行 env 381–545s vs 串行 108s），放宽 `--testTimeout` 无效；该文件在 `v3.20.1..HEAD` 内零改动。判定真伪请用 `npx vitest run --no-file-parallelism`（2026-10-24：135 文件 / 1031 用例全绿）。
 - 另有 i18n 全局守卫用例（`tests/config/localeCoverage.test.ts`，15 例，含四语键集合与占位符一致性）。
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
 
