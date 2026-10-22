 # 发版草稿 — 同模型跨渠道无缝切换 P1–P6
 
 > 分支：`feature/v3.20.1-custom-routing`（基于 `v3.20.1`），最新 `4071f39`。
 > 状态：P1–P6 全部完成，`cargo check / 2816 passed / typecheck / format` 全绿，`git status` 干净。
 
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
 
 ## 行为与兼容
 
 - **零配置升级安全**：某 app 完全没配 alias 时不过滤；过滤后为空时回退完整链，宁可让上游返回 `model_not_found` 也不本地静默 `NoAvailableProvider`。
 - **优先级**：`alias 声明 > 自有 model_mapping（含 * 通配）> env 映射 > 原样`；存量配置零迁移。
 - **回滚**：`select_providers(..., None)` 等价旧行为；删表不影响既有表。
 - **观测**：跳过渠道打 `跳过渠道 X：未声明模型 M`（debug）；命中打 `model alias: alias → target (渠道 Y)`。
 
 ## 交付件
 
 - 主交付：`D:\dev-cache\cc-switch-patches-4071f39/`（`v3.20.1..HEAD` 共 58 个 patch，其中 P1–P6 10 个，`git am` 可靠还原）
 - 备件：`D:\dev-cache\cc-switch-feature-v3.20.1-custom-routing.bundle`（`--all`，`verify` 通过；`clone` 因本仓库浅历史/缺祖先受限，不影响主交付）
 - 远端：`origin` 为上游 `farion1231/cc-switch`，当前账号 `yunhanhai` 直接 `push` 为 403 属预期 fork 流程；`gh` 未安装，需 `gh auth login` 后 `fork + push + 建 PR`。
 
 ## 下一步
 
 1. `gh` 安装后自动 fork 推送建 PR（含本草稿正文）
 2. 或提供已有 fork 地址后加远端推送
 3. 端到端手动验证：Starlight 多渠道 + Gemini CLI 真实重写
 
