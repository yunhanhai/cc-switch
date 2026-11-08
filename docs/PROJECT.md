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

首个独立预发布版本为 `3.20.1-yunswitch.1`，package.json、Cargo.toml、Cargo.lock 与 Tauri 配置保持一致。Windows x64 安装版与免安装版已在 Gitee、GitHub 同步公开，其他平台尚未发布。上游基线仍为 v3.20.1。安装包校验与验收记录见[预发布记录](releases/3.20.1-yunswitch.1.md)。

| 身份         | 发布构建                        | 开发构建                            |
| ------------ | ------------------------------- | ----------------------------------- |
| 产品名       | YunSwitch                       | YunSwitch Dev                       |
| 应用标识     | `io.github.yunhanhai.yunswitch` | `io.github.yunhanhai.yunswitch.dev` |
| 应用目录     | `~/.yunswitch`                  | `~/.yunswitch-dev`                  |
| 深链         | `yunswitch://`                  | `yunswitch-dev://`                  |
| 新库默认端口 | `15731`                         | `15732`                             |

开发命令通过 `tauri.dev.conf.json` 覆盖身份，Cargo debug profile 使用隔离布局；发布构建使用正式布局。运行时按布局设置窗口标题。显式应用设置可以覆盖 CLI 目录；开发布局默认优先于 Hermes、OpenCode 和 Pi 的环境变量目录。测试沙盒保留旧夹具布局，仅用于兼容现有测试。

Windows MSI 使用固定独立 UpgradeCode，安装显示版本为 `3.20.1.1`；应用显示与 Git 标签使用完整预发布版本。内部可执行文件 `cc-switch`、数据库文件 `cc-switch.db` 和兼容字段暂时保留。配置迁移见[迁移指南](MIGRATION-YUNSWITCH.md)。

自动更新关闭、更新地址为空，上游签名公钥已移除。更新按钮打开本项目的 Releases 页面；后续配置本项目签名后再启用自动更新。R2 镜像默认停用，仅通过本项目显式配置启用。

## 仓库维护方式

两边稳定开发线与默认分支统一为 `main`。原 `feature/v3.20.1-custom-routing` 和文档工作分支保留用于追溯，两个平台的文档 PR 合并提交均保留在 `main` 历史中。

GitHub CI 检查面向 `main` 的 PR 与推送，并支持手动执行。Node 版本读取 `.node-version`，Rust 工具链与仓库固定为 1.95.0；Windows 测试编译使用 32 MB 线程栈，后端测试以 `-j 1` 串行编译，避免集成目标加载到不一致的库产物。具体验证结果以每次工作流运行记录为准。

本地远端分工：

```text
origin    https://gitee.com/yunhanhai/yunswitch.git
github    https://github.com/yunhanhai/yunswitch.git
upstream  https://github.com/farion1231/cc-switch.git
```

从 `main` 创建 `feat/*`、`fix/*` 或 `docs/*` 工作分支。上游更新先在单独分支评估，选择需要的提交并记录来源；保留本仓库修改与测试，再合并到自己的稳定线。向 Gitee 和 GitHub 推送前先 fetch 核对，避免覆盖任一平台上的独有提交。

独立发版准备清单见 [发布准备](RELEASE-READINESS.md)。

## 后续调整

- 先完成 Windows x64 的独立包安装与运行验收并发布预览版；其他平台按实际构建与安装结果逐步加入，签名和自动更新按自有渠道逐步启用。
- 继续整理历史手册、供应商预设中继承的推广参数与排序，保留兼容性并说明来源。
- 为路由、模型映射和供应商接入补充场景文档与兼容性矩阵。
- 评估其他开源项目的能力；引入前核查许可证，记录原项目、版本、文件范围和修改内容。
- 逐步整理 CI 与回归测试，按实际验证平台记录发布状态。

## 参考项目

仓库定位与文档组织参考 [RuoYi](https://gitee.com/y_project/RuoYi)、[RuoYi-Vue-Plus](https://gitee.com/dromara/RuoYi-Vue-Plus) 和 [RuoYi-Vue-Pro](https://gitee.com/zhijiantianya/ruoyi-vue-pro)：明确上游关系、维护主体和功能差异。这些是文档组织参考，当前没有据此新增框架代码。
