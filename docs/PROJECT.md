# 项目来源、版本与维护计划

## 项目定位

**YunSwitch** 是由 [yunhanhai](https://gitee.com/yunhanhai) 独立维护的开源二次开发项目。在 CC Switch 的配置管理与本地代理基础上，重点改善多供应商路由、模型别名、供应商配置转换和请求韧性。

正式展示名为 `YunSwitch`，仓库名为 `yunswitch`，中文副标题为「AI 编程工具配置与路由管理」。两个平台通过现有仓库改名延续历史，不重新建仓。

当前由一人维护，后续会结合实际需求吸收其他开源项目的设计与实现。已实现功能、验证结果和待办计划分别记录；参考项目并不自动意味着已经集成其代码。

| 入口     | 地址                                                                                                                          |
| -------- | ----------------------------------------------------------------------------------------------------------------------------- |
| Gitee    | <https://gitee.com/yunhanhai/yunswitch>                                                                                       |
| GitHub   | <https://github.com/yunhanhai/yunswitch>                                                                                      |
| 问题反馈 | [Gitee Issues](https://gitee.com/yunhanhai/yunswitch/issues) / [GitHub Issues](https://github.com/yunhanhai/yunswitch/issues) |
| 原始上游 | [farion1231/cc-switch](https://github.com/farion1231/cc-switch)                                                               |

## 来源与版权

原始 CC Switch 项目采用 MIT 许可证，版权通知为 `Copyright (c) 2025 Jason Young`。本仓库保留原始 [LICENSE](../LICENSE) 的版权与许可条款，并为 yunhanhai 的新增与修改部分补充署名。

本仓库可以独立维护、修改和发布；上游已有代码的版权与本仓库二次开发成果分别保留。克隆后发布独立仓库是 MIT 允许的方式，无需删除历史或重建仓库。平台是否显示 Fork 标志，不改变代码来源与许可证义务。

已核查的 Git 基线：

- `9485cf2f`：上游 `chore(release): v3.20.1`。
- `3217f725`：上游 v3.20.1 发布说明，也是本仓库首个定制提交的父提交。
- `c6c6c4d7`：2026-10-18 开始的定制开发提交。
- `9b3704be`：本次整理前的最新二次开发提交，日期为 2026-10-29。

这一阶段是连续的二次开发历史。继承功能与本仓库增强的区别见 [中文 README](../README_ZH.md)，具体提交见 [历史开发记录](RELEASE-NOTES-P1-P6.md)。例如 Responses 与 Anthropic 的协议桥接核心来自上游，本仓库在其基础上补充供应商转换、模型搬运、守卫和整链测试。

按已有代码注释和 Git 历史核查的第三方来源，以及后续引入代码、资源的记录格式，见[来源台账](PROVENANCE.md)。当前台账覆盖已确认的主要来源，依赖和全部图标资源仍需分别核查。

已确认的 CodexBar、LobeHub 与 AgentRouter Responses Shim 许可副本见[第三方通知](../THIRD-PARTY-NOTICES.md)，图标匹配范围见[图标来源记录](ICON-SOURCES.md)。`.4` 已公开包的通知配置与实际验收见[许可分发](THIRD-PARTY-PACKAGING.md)；既有发布包保留其历史内容，新增源码文件不等同于旧包已更新。

## 当前版本与发布状态

当前公开版本为 `3.20.1-yunswitch.8`，新增已限流请求的恢复协调、动态共享期限及等待预算，见[恢复说明](CODEX-STREAM-RECOVERY.md)与[`.8` 发布记录](releases/3.20.1-yunswitch.8.md)。

当前源码版本为 `3.20.1-yunswitch.8`，package.json、Cargo.toml、Cargo.lock 与 Tauri 配置保持一致。Windows x64 安装版与便携 ZIP 已在 [GitHub](https://github.com/yunhanhai/yunswitch/releases/tag/v3.20.1-yunswitch.8) 和 [Gitee](https://gitee.com/yunhanhai/yunswitch/releases/tag/v3.20.1-yunswitch.8) 公开；安装、`.7` 升级、并发恢复及共享期限检查通过验收。之前的[`.7` 版本](releases/3.20.1-yunswitch.7.md)保留原附件和验收记录。

已公开的 `3.20.1-yunswitch.2` Windows x64 安装版与免安装版见 [GitHub](https://github.com/yunhanhai/yunswitch/releases/tag/v3.20.1-yunswitch.2) 和 [Gitee](https://gitee.com/yunhanhai/yunswitch/releases/tag/v3.20.1-yunswitch.2)，其安装与首版升级验收通过。文件校验与运行记录见[`.2` 发布记录](releases/3.20.1-yunswitch.2.md)。

首个独立预发布 `3.20.1-yunswitch.1` 的 Windows x64 安装版与免安装版已在 Gitee、GitHub 同步公开，其他平台尚未发布。上游基线仍为 v3.20.1。已公开包的校验与验收记录见[首版记录](releases/3.20.1-yunswitch.1.md)。

| 身份         | 发布构建                        | 开发构建                            |
| ------------ | ------------------------------- | ----------------------------------- |
| 产品名       | YunSwitch                       | YunSwitch Dev                       |
| 应用标识     | `io.github.yunhanhai.yunswitch` | `io.github.yunhanhai.yunswitch.dev` |
| 应用目录     | `~/.yunswitch`                  | `~/.yunswitch-dev`                  |
| 深链         | `yunswitch://`                  | `yunswitch-dev://`                  |
| 新库默认端口 | `15731`                         | `15732`                             |

开发命令通过 `tauri.dev.conf.json` 覆盖身份，Cargo debug profile 使用隔离布局；发布构建使用正式布局。运行时按布局设置窗口标题。显式应用设置可以覆盖 CLI 目录；开发布局默认优先于 Hermes、OpenCode 和 Pi 的环境变量目录。测试沙盒保留旧夹具布局，仅用于兼容现有测试。

Windows MSI 沿用固定独立 UpgradeCode，当前安装版本为 `3.20.1.8`；应用显示与 Git 标签使用完整预发布版本。`.1`→`.2` 至 `.7`→`.8` 各次相邻版本升级均有独立验收记录，包含合成数据与设置保留以及对应版本的代理运行检查。内部可执行文件 `cc-switch`、数据库文件 `cc-switch.db` 和兼容字段暂时保留。配置迁移见[迁移指南](MIGRATION-YUNSWITCH.md)。

自动更新关闭、更新地址为空，上游签名公钥已移除。更新按钮打开本项目的 Releases 页面；后续配置本项目签名后再启用自动更新。R2 镜像默认停用，仅通过本项目显式配置启用。

## 供应商目录维护与验证范围

限流重试的默认值、流输出后无法透明重放的边界，以及本次源码新增的 Codex Responses 兼容开关，见 [Codex 兼容与重试说明](CODEX-RESPONSES-COMPATIBILITY.md)。该开关尚未包含在已公开 `.3` 包中。

已合并的目录维护清理内置供应商的网站与 API Key 获取链接中继承的推广归因参数和邀请路径，移除内置商业合作标记；默认选择器排序为官方预设优先，其余按显示名称排序。预设名称、请求地址、配置模板与已有用户配置保留，部分历史字段继续用于供应商识别与兼容，不代表 YunSwitch 与供应商存在商业合作。

这些目录改动已包含在 `3.20.1-yunswitch.2` 安装包中。首版 `.1` 及旧手册保留其历史描述。

| 范围          | 已验证                                                                                                                                                                                                                                   | 当前边界                                                     |
| ------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| Windows x64   | [干净 runner 安装验收](https://github.com/yunhanhai/yunswitch/actions/runs/43592678793)：便携版与 MSI 的启动、实际界面、模拟代理请求、重启、单实例、合成数据迁移、`.7` 升级及并发限流恢复、共享期限和已有 Codex 功能、许可内容与卸载清理 | 已发布独立包；未统一验证所有真实付费供应商与用户私有数据迁移 |
| macOS / Linux | [跨平台源码 CI](https://github.com/yunhanhai/yunswitch/actions/runs/43592579771)：前端检查、各平台后端测试与 Clippy                                                                                                                      | 尚无独立发布包及安装验收；源码测试通过不等同于安装可用       |
| ARM64         | 尚无独立安装验收记录                                                                                                                                                                                                                     | 尚未发布或确认平台兼容性                                     |

以上记录对应 `.8`。[最终源码 CI](https://github.com/yunhanhai/yunswitch/actions/runs/43592579771)全部通过；[Windows 安装升级](https://github.com/yunhanhai/yunswitch/actions/runs/43592678793)通过 88 项运行检查。来源和校验见[本次发布记录](releases/3.20.1-yunswitch.8.md)。供应商预设仍是配置起点，不保证所有网站或长会话都兼容。

## 仓库维护方式

两边稳定开发线与默认分支统一为 `main`。原 `feature/v3.20.1-custom-routing` 和文档工作分支保留用于追溯，两个平台的文档 PR 合并提交均保留在 `main` 历史中。

GitHub CI 检查面向 `main` 的 PR 与推送，并支持手动执行。Node 版本读取 `.node-version`，Rust 工具链与仓库固定为 1.95.0；Windows 测试编译使用 32 MB 线程栈，后端测试以 `-j 1` 串行编译，避免集成目标加载到不一致的库产物。具体验证结果以每次工作流运行记录为准。

本地远端分工：

```text
origin    https://gitee.com/yunhanhai/yunswitch.git
github    https://github.com/yunhanhai/yunswitch.git
upstream  https://github.com/farion1231/cc-switch.git
```

从 `main` 创建 `feat/*`、`fix/*`、`docs/*` 或 `chore/*` 工作分支。上游更新先在单独分支评估，选择需要的提交并记录来源；保留本仓库修改与测试，再合并到自己的稳定线。向 Gitee 和 GitHub 推送前先 fetch 核对，避免覆盖任一平台上的独有提交。

独立发版准备清单见 [发布准备](RELEASE-READINESS.md)。

新增供应商时按[供应商目录维护指南](PROVIDER-CATALOG.md)记录客户端、请求协议、模型和实际验证结果；引入其他项目的实现时更新[来源台账](PROVENANCE.md)。贡献与审阅说明见 [CONTRIBUTING.md](../CONTRIBUTING.md)。

## 后续调整

具体优先级、完成证据和阶段边界见[开发路线](DEVELOPMENT-ROADMAP.md)。本轮先补限流后的并发恢复与共享等待期限，再推进请求诊断、接入验证和协议扩展。

- Windows x64 首个独立预览版已发布；后续包按版本重新验收，其他平台按实际构建与安装结果逐步加入，签名和自动更新按自有渠道逐步启用。
- 持续整理历史手册与供应商目录，在保留兼容性和来源说明的基础上维护中性链接及排序。
- 按真实请求结果补充路由、模型映射和供应商接入的场景文档与兼容性记录。
- 评估其他开源项目的能力；引入前核查许可证，记录原项目、版本、文件范围和修改内容。
- 逐步整理 CI 与回归测试，按实际验证平台记录发布状态。

## 参考项目

仓库定位与文档组织参考 [RuoYi](https://gitee.com/y_project/RuoYi)、[RuoYi-Vue-Plus](https://gitee.com/dromara/RuoYi-Vue-Plus) 和 [RuoYi-Vue-Pro](https://gitee.com/zhijiantianya/ruoyi-vue-pro)：明确上游关系、维护主体和功能差异。这些是文档组织参考，当前没有据此新增框架代码。
