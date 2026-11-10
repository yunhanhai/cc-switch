# 供应商目录维护 / Provider catalog maintenance

内置预设提供客户端配置的起点。收录供应商不代表 YunSwitch 与其有商业合作，也不保证所有套餐、模型或协议当前可用。发布平台与安装验收见[项目说明](PROJECT.md#供应商目录维护与验证范围)；本文说明如何维护预设及记录接入验证。

Built-in presets are configuration starting points. Keep links neutral, preserve existing identifiers and record the exact client, protocol and request path verified. A successful mock or model-list request does not establish real provider compatibility.

## 修改入口

供应商可只支持部分客户端，按实际协议修改对应目录，无需复制到所有文件：

| 客户端 / 范围    | 文件                                                                             | 主要导出                                              |
| ---------------- | -------------------------------------------------------------------------------- | ----------------------------------------------------- |
| Claude Code      | [claudeProviderPresets.ts](../src/config/claudeProviderPresets.ts)               | `providerPresets`                                     |
| Claude Desktop   | [claudeDesktopProviderPresets.ts](../src/config/claudeDesktopProviderPresets.ts) | `claudeDesktopProviderPresets`                        |
| Codex            | [codexProviderPresets.ts](../src/config/codexProviderPresets.ts)                 | `codexProviderPresets`                                |
| Gemini CLI       | [geminiProviderPresets.ts](../src/config/geminiProviderPresets.ts)               | `geminiProviderPresets`                               |
| Grok Build       | [grokBuildProviderPresets.ts](../src/config/grokBuildProviderPresets.ts)         | `grokBuildOfficialPreset`、`grokBuildProviderPresets` |
| OpenCode         | [opencodeProviderPresets.ts](../src/config/opencodeProviderPresets.ts)           | `opencodeProviderPresets`                             |
| OpenClaw         | [openclawProviderPresets.ts](../src/config/openclawProviderPresets.ts)           | `openclawProviderPresets`                             |
| Hermes           | [hermesProviderPresets.ts](../src/config/hermesProviderPresets.ts)               | `hermesProviderPresets`                               |
| Pi               | [piProviderPresets.ts](../src/config/piProviderPresets.ts)                       | `piProviderPresets`，由定义与模型辅助配置生成         |
| 跨客户端统一模板 | [universalProviderPresets.ts](../src/config/universalProviderPresets.ts)         | `universalProviderPresets`                            |

Claude、Gemini 使用各自的环境变量与配置模板；Codex 和 Grok Build 使用认证配置与 TOML；OpenCode 使用 SDK 包及其选项；OpenClaw、Hermes 和 Pi 有自己的模型与协议字段。以对应接口类型和客户端文档为准，不能仅替换网址后跨客户端复用。

## 链接、分类与排序

- `websiteUrl` 指向供应商中性网站，`apiKeyUrl` 可指向注册页或密钥控制台。保留访问所需路径、查询与 SPA 路由，例如控制台区域、购买页重定向和 `#/api-key`。
- 内置链接不附推广归因或邀请编码，例如 `aff`、`ref`、`invitecode`、`utm_*` 及带邀请人的路径。参数是否承担路由功能应按供应商文档判断，不能按字符串匹配批量删除所有查询。
- 目录不新增 `isPartner`、`primePartner` 商业标记；这些类型字段保留用于历史数据兼容。默认选择器保持 `category: "official"` 的条目在前，其余按显示名称排序；A–Z 模式对全部条目按名称排序。名称可由 `nameKey` 翻译，因此具体顺序可能随显示语言变化。
- `category` 描述接入类型，`isOfficial` 及 OAuth 字段应依据实际客户端行为填写。分类不构成推荐等级，也不能为获得排序优先而更改。
- 新增或修改 `nameKey` 时同步四种界面语言；新增图标或引入代码时记录其来源与许可证，参见[贡献指南](../CONTRIBUTING.md)。

目前内置 `partnerPromotionKey` 仅保留 Gemini 的 `google-official` 与 `packycode`。前者参与 OAuth 表单识别，两者参与后端 Gemini 认证类型检测。它们是功能兼容键；清理命名或移除字段之前，应同时调整[认证检测](../src-tauri/src/services/provider/gemini_auth.rs)、表单及兼容测试。

## 标识与已有配置

多数表单用预设数组索引生成选择器 ID，例如 `codex-0`、`pi-0`、`claude-desktop-0`，并通过 `presetId` 传递。Claude Code 在生成索引前还会过滤 `hidden` 条目。文件重排、插入、删除或隐藏状态变化都可能改变 ID；新增预设优先追加到目录末尾，不通过重排源数组控制展示次序。

修改前后核对名称、数量、顺序和隐藏状态。必须删除或改变标识时，检查消费方及已有记录，说明兼容处理。Pi 的 `providerKey`、统一模板的 `providerType`、模型 ID 与路由 ID 也有独立用途，不能与选择器索引或显示名称混为一谈。

目录更新针对今后选择预设生成的配置。它不应顺带改写用户已保存的自定义网址、密钥、模型别名或请求设置；数据迁移属于单独的行为改动，应有明确规则和验证。

## 请求配置与能力

网站或注册链接不等于 API 请求地址。按供应商协议文档核对实际 endpoint、认证字段、模型 ID 和协议；`endpointCandidates` 应为客户端可使用的请求地址。模型列表可读取，不代表生成接口或流式响应可用。

接入记录应区分客户端直连与 YunSwitch 本地代理。需要协议转换时注明转换方向，例如 Claude Messages → OpenAI Responses；对流式响应、工具调用、推理字段等能力分别记录结果。未验证能力保留为未知，不用品牌名或模型名称推断上下文长度、费用与能力。

## 接入验证记录

每次接入或行为调整在 PR 或对应场景文档记录以下信息；只记录已执行范围：

| 字段         | 需要记录的内容                                                                              |
| ------------ | ------------------------------------------------------------------------------------------- |
| 版本与日期   | YunSwitch 提交或版本、客户端版本、操作系统、执行日期                                        |
| 供应商与模型 | 供应商名称、地区或套餐（影响结果时）、实际发送的模型 ID                                     |
| 路径         | 客户端直连 / YunSwitch 代理，以及转换方向                                                   |
| 协议与操作   | Messages / Responses / Chat Completions / Gemini 等；普通生成、流式生成、工具调用或模型列表 |
| 输入环境     | 模拟上游 / 真实供应商；自动测试 / 手动操作                                                  |
| 结果与证据   | 通过、失败或未验证；相关测试、已脱敏日志或 Issue                                            |
| 边界         | 未验证的模型、能力、套餐和平台，及失败时的限制                                              |

例如“模拟上游通过 Responses 流式转发测试”只说明已测试的转发行为；“真实供应商模型列表读取成功”只说明目录接口可访问。只有实际生成请求通过，才能将对应客户端、模型、协议与路径记为生成验证通过。

测试配置使用虚构密钥和本地模拟地址。真实请求需要使用者自己的授权配置，但密钥、令牌、会话 Cookie、个人路径和私人对话不得写入预设、测试夹具、提交或公开日志；共享证据前先脱敏。

## 检查与提交

先运行覆盖所有内置目录中性规则的检查，以及本次修改对应的预设测试：

```bash
corepack pnpm exec vitest run src/config/providerCatalogNeutrality.test.ts tests/components/ProviderPresetSelector.test.tsx
corepack pnpm typecheck
```

例如修改 SubRouter 或 Pi 时，再分别运行现有场景测试：

```bash
corepack pnpm exec vitest run tests/config/subrouterProviderPresets.test.ts
corepack pnpm exec vitest run tests/config/piProviderPresets.test.ts
```

新增条目的测试应检查有意义的协议行为和配置约束，例如模型与路由引用一致、认证字段正确、控制台路径未损坏，避免只断言新代码中原样复制的字符串。涉及表单序列化或协议转换时追加对应测试；修改 Rust 认证或代理代码时运行相关后端检查。文档或链接整理按改动范围检查，无需为每次预设编辑重新执行安装验收。

提交前检查改动文件格式与 diff，并在 PR 写明来源、改动客户端、已运行检查和未验证能力。目录中性检查是回归保护，不能替代供应商文档核对或真实请求验证。
