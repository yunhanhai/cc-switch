# Claude → Codex 供应商转换：协议审计与运行条件

> 2026-10-27。本文档记录「Claude 供应商 ⇄ Codex 供应商一键转换」功能的协议语义依据、
> 桥接链路保真度盘点与运行时条件。供后续维护者复核；改动转换逻辑前请先读完本文。

## 1. 转换语义（三方对照）

转换把一个 Claude Code 第三方供应商（`env.ANTHROPIC_BASE_URL` + `ANTHROPIC_AUTH_TOKEN`
+ `ANTHROPIC_MODEL`）转成 Codex 供应商（`auth.OPENAI_API_KEY` + `config.toml`）：

| Claude 侧 | Codex 侧 | 依据 |
| --- | --- | --- |
| `ANTHROPIC_BASE_URL` | `[model_providers.custom].base_url` | 端点跨 CLI 复用，同渠道商 |
| `ANTHROPIC_AUTH_TOKEN`（回退 `ANTHROPIC_API_KEY`） | `auth.json` 的 `OPENAI_API_KEY` | 凭据复用 |
| `ANTHROPIC_MODEL` | config 顶层 `model` | 见 §3：桥接路径会覆写请求模型为此值 |
| （协议） | `wire_api = "anthropic"` + `meta.apiFormat = "anthropic"` | 见 §2 |

**协议权威依据（上游源码）**：`openai/codex` 的 `WireApi` 枚举（截至 2026-02 索引，
`codex-rs/core/src/model_provider_info.rs`）只有两个变体——`"responses"`（默认，
OpenAI Responses API）与 `"anthropic"`（Anthropic Messages API，`/v1/messages`）。
`"chat"` 已于 2026-02 移除（`openai/codex#7782`，反序列化硬错误）。
因此 `wire_api = "anthropic"` 是目标 CLI 的**官方一等公民配置**，不是 cc-switch 自造。

## 2. 为什么必须走本地路由桥接（而不是直连）

Claude 供应商的 `ANTHROPIC_BASE_URL` 说的是 Anthropic Messages 协议（`/v1/messages`）；
Codex 客户端向 `model_providers.custom` 发的是 Responses 请求。两条路：

- **新版 Codex 直连**：`wire_api = "anthropic"` 后 Codex 自己说 Anthropic 协议，
  理论上可以不经 cc-switch 直连。此路可行但绕过了 cc-switch 的故障转移、
  模型别名、用量统计、429 等待重试等路由层能力。
- **经 cc-switch 本地路由桥接**（本转换产物采用的形态）：Codex 仍指向本地代理，
  代理做 Responses ⇄ Anthropic 双向转换。产物同时写 `meta.apiFormat = "anthropic"`
  使前端 `providerNeedsRouting` 在切换时提示先启动本地路由。

`wire_api`（TOML）与 `meta.apiFormat`（DB meta）双保险的原因：后端判定链
`codex_provider_uses_anthropic` 只认**显式声明**（meta → settings `apiFormat` →
TOML `wire_api`，按此顺序），绝不从 base_url 猜测——猜测极易误判。

## 3. 桥接链路保真度（逐环节盘点）

请求方向（Responses → Anthropic Messages，`forwarder.rs` + `transform_codex_anthropic.rs`）：

| 环节 | 位置 | 行为 | 测试 |
| --- | --- | --- | --- |
| 协议判定 | `codex_provider_uses_anthropic` | 显式声明三级回退 | 3 例 |
| 端点重写 | `rewrite_codex_responses_endpoint_to_anthropic` | `/responses`→`/v1/messages`，query 保留 | 2 例 |
| 请求模型 | `apply_codex_upstream_model` | 不在 `modelCatalog` 时覆写为 config 顶层 `model`（转换产物已搬 `ANTHROPIC_MODEL`） | 转换侧 18 例 |
| max_tokens | `DEFAULT_CODEX_ANTHROPIC_MAX_TOKENS = 8192` | Codex 不发时的保守兜底；`meta.maxOutputTokens` 可覆盖 | transform 内 |
| 1M 标记 | `[1m]` 剥离 + `context-1m` beta | 最终 body 上再剥一次 | transform 内 |
| tool use | `responses_request_to_anthropic` | function→tool 重嵌、孤立 tool result 清洗、并行轮完整性 | 20+ 例 |
| 思考参数 | `effort_to_thinking_budget` | effort→budget 映射；签名思考块跨轮保真（`ccswitch-anthropic-thinking-v1:` 信封） | 10+ 例 |
| SSE 聚合 | `anthropic_sse` 系列 | message/tool_use 增量聚合、错误事件 | 6+ 例 |
| 指纹头 | `is_codex_client_fingerprint_header` | Codex/OpenAI 指纹头（`originator`/`x-stainless-*` 等）不漏给 Anthropic 网关 | 1 例（denylist） |
| Accept 归一化 | header 循环 | Codex 的 `text/event-stream` → `application/json`（严格网关 406 防御） | 回环端到端 1 例 |
| anthropic-version | `ANTHROPIC_API_VERSION` 常量 | 客户端缺省时补 `2023-06-01`（契约测试钉住） | 1 例（契约） |
| Claude Code 伪装 | `impersonateClaudeCode` | **默认关**，仅显式开启（防指纹泄漏到通用网关）；开时注入 UA/x-app/系统身份行 | 3 例（幂等） |

响应方向：tool_use/思考块/usage/stop_reason 映射均有测试（`transform_codex_anthropic`
共 **73 例**）；2xx 内嵌错误包（`{"type":"error"}`）会被识别为可故障转移失败。

## 4. 运行条件与已知取舍

- **必须先启动本地路由**：转换产物写入了 `meta.apiFormat`，切换时前端会提示；
  成功 toast 与供应商 notes 也携带该条件。
- **运行时证据已具备（回环级，三种形态全覆盖）**：`server.rs` 三个回环测试 ——
  非流式桥接（`converted_codex_provider_reaches_anthropic_upstream_in_messages_shape`）、
  流式桥接（`converted_codex_provider_streams_anthropic_sse_back_as_responses_sse`，
  mock 上游回 Anthropic SSE 事件流，客户端收到 Responses SSE：
  `response.created` → `output_text.delta` → `completed`），以及
  直连模式（`converted_codex_provider_in_direct_mode_passes_responses_through`：
  路径停留在 `/v1/responses`、不发 `anthropic-version`、body 保持 Responses 形状）。
  wire 级断言全部通过。仍未做的只有"真实第三方网关 + 真实 Codex CLI"级验证，
  需密钥与真机，留待日常使用检验。
- **认证头默认 Bearer**（`Authorization: Bearer`）。网关若只收 `x-api-key`，
  在编辑表单（Anthropic 格式 → 认证字段）改为 `ANTHROPIC_API_KEY` 即可；
  两者互斥，避免网关收到双认证头 401。
- **防重**：对同源供应商重复转换时，若 Codex 列表已存在同名同密钥的产物，不重复创建。
- **密钥复制面**：转换会把密钥写入另一份配置。这是功能的本质（两份独立配置），
  但请知悉落盘面扩大；原 Claude 供应商不受影响。
- **可选增强（已实现，`0b3e08dc`）：双协议探测**。Anthropic 端点带已知兼容
  后缀（`/anthropic`、`/claude`、`/claude-code`、`/messages`）时，转换会剥离
  后缀对 OpenAI 候选端点做一次 `GET /v1/models` 探测；返回非空模型列表则生成
  `wire_api = "responses"` 的直连产物（无需本地路由），notes 与成功 toast 均声明
  直连模式。探测失败静默回退桥接产物。保守剥离：裸 origin 与未知路径形态
  返回 undefined，绝不猜测。

## 5. 素材来源

- 上游协议：`openai/codex` 源码级文档（DeepWiki 索引 2026-02-25：`model_provider_info.rs`、
  `config.schema.json`）与 Discussion #7782（chat 弃用公告）
- 本仓库实现：`src/utils/claudeToCodexConvert.ts`（转换 + payload）、
  `src-tauri/src/proxy/forwarder.rs` 与 `transform_codex_anthropic.rs`（桥接）
- 本地约定：`AGENTS.md`（勿改回清单）
