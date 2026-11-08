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

## 当前版本与发布状态

源码中的 `package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json` 当前均为 `3.20.1`，这是沿用的源码版本，不能据此宣称本仓库已经正式发布 v3.20.1 安装包。2026-11-08 核查时，本仓库 GitHub 尚无 Release。

建议首个独立版本采用类似 `3.20.1-yunswitch.1` 的预发布版本，并在发布前同步三个版本文件、Cargo.lock 中的本包版本、Git 标签与发布说明；后续根据改动采用自己的版本节奏。此建议尚未作为正式版本执行。

当前开发配置为：

- 产品名 `CC Switch Dev`，应用标识 `com.ccswitch.desktop.dev`。
- 开发配置目录 `~/.cc-switch-dev`，本地代理端口默认 `15722`，深链 `ccswitch-dev://`。
- 自动更新配置为关闭，更新地址为空。

展示名称与仓库名已确定为 YunSwitch；安装包身份、数据目录、深链和更新签名应在发布前一并设计，并提供迁移说明。

## 仓库维护方式

当前远端默认分支为 `feature/v3.20.1-custom-routing`。建议在本轮文档审阅后，将稳定开发线整理为 `main`，保留原分支用于追溯；更改默认分支前核对两边 CI、分支保护与发布规则。

本地远端分工：

```text
origin    https://gitee.com/yunhanhai/yunswitch.git
github    https://github.com/yunhanhai/yunswitch.git
upstream  https://github.com/farion1231/cc-switch.git
```

从当前默认分支创建 `feat/*`、`fix/*` 或 `docs/*` 工作分支。上游更新先在单独分支评估，选择需要的提交并记录来源；保留本仓库修改与测试，再合并到自己的稳定线。向 Gitee 和 GitHub 推送前先 fetch 核对，避免覆盖任一平台上的独有提交。

## 后续调整

- 完成独立发布：版本、应用品牌、安装包、签名密钥与更新服务一起校准。
- 清理运行界面、安装脚本、Flatpak 和发布流水线中的上游品牌/发布假设，并逐项验证。
- 为路由、模型映射和供应商接入补充场景文档与兼容性矩阵。
- 评估其他开源项目的能力；引入前核查许可证，记录原项目、版本、文件范围和修改内容。
- 逐步整理 CI 与回归测试，按实际验证平台记录发布状态。

## 参考项目

仓库定位与文档组织参考 [RuoYi](https://gitee.com/y_project/RuoYi)、[RuoYi-Vue-Plus](https://gitee.com/dromara/RuoYi-Vue-Plus) 和 [RuoYi-Vue-Pro](https://gitee.com/zhijiantianya/ruoyi-vue-pro)：明确上游关系、维护主体和功能差异。这些是文档组织参考，当前没有据此新增框架代码。
