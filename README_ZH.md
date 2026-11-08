<div align="center">

# YunSwitch

AI 编程工具配置与路由管理，面向多供应商与多模型场景持续改进。

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Tauri 2](https://img.shields.io/badge/Tauri-2-orange.svg)](https://tauri.app/)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](#开发与构建)

[English](README.md) | 中文 | [日本語](README_JA.md) | [Deutsch](README_DE.md)

[Gitee](https://gitee.com/yunhanhai/yunswitch) · [GitHub](https://github.com/yunhanhai/yunswitch) · [项目说明](docs/PROJECT.md) · [更新记录](CHANGELOG.md)

</div>

## 项目介绍

本项目由 **yunhanhai 独立维护**，基于 [farion1231/cc-switch](https://github.com/farion1231/cc-switch) 的 **v3.20.1** 继续开发。除继承的配置管理与代理能力外，已加入路由分组、跨供应商模型别名、模型目录辅助、Claude → Codex 供应商配置转换及请求韧性增强。

项目目前由一人维护，后续会根据实际需求吸收其他开源项目的优点，持续完善供应商接入、路由体验和稳定性。代码来源、已有增强与后续计划见 [项目说明](docs/PROJECT.md)。上游版权与 MIT 许可保留在 [LICENSE](LICENSE) 中，本仓库新增与修改部分另行署名。

**本仓库入口以这里的 Gitee / GitHub 为准。** 上游官网、安装包、包管理源与赞助关系属于原项目；本版本的发布与支持由本仓库维护者负责。

## 已有功能与本仓库增强

继承的基础能力包括 Claude Code、Claude Desktop、Codex、Gemini CLI、Grok Build、OpenCode、OpenClaw 和 Hermes Agent 的配置管理，以及供应商切换、MCP、Prompts、Skills、会话管理、用量统计、本地代理和基础故障转移。各客户端的具体能力与运行要求见 [用户手册](docs/user-manual/zh/README.md)。

以下是相对 v3.20.1 基线的主要增强：

| 方向        | 本仓库已实现的增强                                                                       | 使用说明                                             |
| ----------- | ---------------------------------------------------------------------------------------- | ---------------------------------------------------- |
| 路由分组    | 按应用组织供应商，分组启停、组内排序、生效路由预览与健康状态展示                         | [路由分组](docs/routing-groups.md)                   |
| 模型别名    | 将同一逻辑模型映射到不同渠道的物理模型名，故障转移时随供应商切换；从已有映射生成归并建议 | [别名设计与边界](docs/model-alias-routing-design.md) |
| Gemini 路由 | 同步处理 URL 路径中的模型别名、选路与实际发送模型的记录                                  | [开发记录 P1–P6](docs/RELEASE-NOTES-P1-P6.md)        |
| 模型目录    | 动态模型目录、缓存与模型映射辅助，减少手工填写成本                                       | [用户手册](docs/user-manual/zh/README.md)            |
| 供应商转换  | Claude 第三方供应商转换为 Codex 配置，搬运源模型、防止重复创建，并探测直连候选           | [转换说明](docs/claude-to-codex-conversion.md)       |
| 请求韧性    | 429 等待与冷却观测、熔断状态展示；可先重试同一供应商，再走故障转移                       | [开发记录追加篇章](docs/RELEASE-NOTES-P1-P6.md)      |
| 维护与验证  | 配置字段/注释保留、开发与测试目录隔离、四语一致性检查，以及路由与转换回环测试            | [贡献指南](CONTRIBUTING.md)                          |

这些增强建立在上游已有实现之上。例如 Responses 与 Anthropic 的协议桥接核心来自上游，本仓库补充的是转换入口、配置与模型处理、运行条件判断及整链测试。

### 需要了解的边界

- 模型别名是配置映射；未配置或筛选结果为空时保留回退链，不能视为供应商真实能力检测。
- 同供应商重试默认关闭，最多 10 次，只覆盖连接失败及部分可重试状态码；已经交付给客户端的流不能重新发送。429、认证错误等另行处理。
- Claude → Codex 的桥接模式需要启动本地路由。直连候选依据模型目录与源模型是否存在判断，仍需要用实际请求验证 Responses 支持。
- 历史开发记录中的测试成绩属于旧电脑的验证结果，当前平台与发布版本的验证应单独记录。

## 界面预览

以下为仓库已有截图，具体布局以当前代码为准。

![供应商与本地路由示例](docs/images/codex-kimi-routing/01-codex-providers-require-routing.png)

## 获取与使用

源码中的版本目前仍为 `3.20.1`，用于延续已有代码版本。本仓库 GitHub 在 2026-11-08 核查时尚无正式 Release；请使用源码开发，后续安装包会在本仓库发布入口公布。

- [Gitee 仓库](https://gitee.com/yunhanhai/yunswitch)
- [GitHub Releases](https://github.com/yunhanhai/yunswitch/releases)
- [版本与发布计划](docs/PROJECT.md#当前版本与发布状态)

基本流程：添加供应商配置 → 按客户端需要启用本地路由 → 设置路由分组与模型别名 → 发起请求并查看路由和用量结果。需要协议转换的供应商，请先阅读相应指南。

常用文档：

- [用户手册](docs/user-manual/zh/README.md)
- [路由分组文档导航](docs/routing-groups.index.md)
- [Claude → Codex 配置转换](docs/claude-to-codex-conversion.md)
- [Codex 接入 Kimi](docs/guides/codex-kimi-routing-guide-zh.md)
- [Codex 接入 Claude](docs/guides/codex-claude-routing-guide-zh.md)
- [Claude Code 接入 Codex](docs/guides/claude-codex-routing-guide-zh.md)
- [Codex Desktop 自定义模型显示](docs/guides/codex-desktop-custom-model-visibility-zh.md)

手册与旧版发布说明中保留的上游安装信息供历史参考；安装本仓库版本时，以本仓库实际发布说明为准。

## 开发与构建

环境版本以仓库配置为准：

| 工具          | 要求                                                               |
| ------------- | ------------------------------------------------------------------ |
| Node.js       | 22.12+；`.node-version` 固定为 22.12.0                             |
| pnpm          | 10.12.3，见 `package.json` 的 `packageManager`                     |
| Rust          | 1.95 工具链，含 rustfmt / clippy，见 `rust-toolchain.toml`         |
| Windows       | MSVC C++ Build Tools、Windows SDK、WebView2                        |
| macOS / Linux | 对应 [Tauri 2 系统依赖](https://v2.tauri.app/start/prerequisites/) |

```bash
git clone https://gitee.com/yunhanhai/yunswitch.git
cd yunswitch
corepack pnpm install --frozen-lockfile
corepack pnpm dev
```

GitHub 也可以作为克隆来源：`https://github.com/yunhanhai/yunswitch.git`。

常用命令（使用 Corepack 时，在 `pnpm` 前加 `corepack`）：

| 命令                                              | 用途                                |
| ------------------------------------------------- | ----------------------------------- |
| `pnpm dev`                                        | 启动 Tauri 桌面开发环境             |
| `pnpm dev:renderer`                               | 启动前端；桌面 API 功能仍需要 Tauri |
| `pnpm typecheck`                                  | TypeScript 类型检查                 |
| `pnpm test:unit`                                  | 前端单元测试                        |
| `pnpm build:renderer`                             | 构建前端                            |
| `pnpm build`                                      | 构建桌面应用与安装包                |
| `cargo test --manifest-path src-tauri/Cargo.toml` | Rust 后端测试                       |

开发构建使用 `CC Switch Dev`、`~/.cc-switch-dev` 和默认代理端口 `15722`；正式版本的数据与发布标识将在独立发版时统一规划。详细开发流程见 [贡献指南](CONTRIBUTING.md)。

## 参与与后续计划

问题与建议请提交到 [Gitee Issues](https://gitee.com/yunhanhai/yunswitch/issues) 或 [GitHub Issues](https://github.com/yunhanhai/yunswitch/issues)，代码贡献请阅读 [CONTRIBUTING.md](CONTRIBUTING.md)。安全问题按 [SECURITY.md](SECURITY.md) 联系维护者。

后续重点是独立版本与发布渠道、运行界面与安装标识整理、更多供应商兼容性验证，以及有明确来源记录的跨项目能力整合。具体待办见 [维护计划](docs/PROJECT.md#后续调整)。

## 来源与许可证

感谢 [CC Switch 上游项目](https://github.com/farion1231/cc-switch) 与原作者 Jason Young 提供基础实现，也感谢相关依赖和贡献者。

本仓库基于 **MIT** 许可证继续开放，保留上游版权与许可文字，补充 yunhanhai 对新增和修改部分的署名。完整条款见 [LICENSE](LICENSE)。
