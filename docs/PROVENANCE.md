# 开源来源记录

YunSwitch 由 yunhanhai 独立维护，继承代码、移植代码与本项目修改保留各自来源。本文记录已核实的关系，作为后续引入代码、资源和设计的登记入口；它不是全仓依赖、图片、图标或打包许可证审计报告。项目定位见[项目说明](PROJECT.md)，当前版权与许可见[LICENSE](../LICENSE)。

## 已核实的代码来源

| 来源                                                                    | 固定记录                                                                                                                                            | 本仓库范围与处理                                                                                                                                                                                      | 许可证与核查范围                                                                                                                                                                                                                      |
| ----------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [CC Switch](https://github.com/farion1231/cc-switch)                    | `9485cf2f3d36b9ef77e8a6bf7ccc35bed2e67be6` 为 v3.20.1 版本提交；`3217f72596f2d1c0f879f0a05f83803825d9809f` 为发布说明提交，也是首个定制提交的父提交 | 继承桌面应用、客户端配置管理、本地代理和协议转换等基础；此后增加独立路由、模型别名、配置转换、请求韧性、应用身份与发布维护。差异见 [README](../README_ZH.md) 与[历史开发记录](RELEASE-NOTES-P1-P6.md) | 基线 `LICENSE` 为 MIT，署名 `Copyright (c) 2025 Jason Young`；根许可证保留该通知，并增加 yunhanhai 的修改与新增署名                                                                                                                   |
| [CodexBar](https://github.com/steipete/CodexBar)，经 CC Switch 间接继承 | CC Switch 提交 `15d5dbe065560eaca1b5dbd49b3f995b55d37391`（2026-07-23）注明 Grok 订阅额度协议移植自 CodexBar；该提交位于上述基线之前                | [subscription_grok.rs](../src-tauri/src/services/subscription_grok.rs) 注释指出凭据选择及 gRPC-web billing 解析来源。本文件相对 `3217f725` 基线未改动，属于继承能力                                   | MIT，署名 `Copyright (c) 2026 Peter Steinberger`；[原文许可副本](../licenses/CodexBar-MIT.txt)来自移植日期之前的许可提交 `c1e05761e55ddd3d66cdd43edbe7823f627ee6a8`，与当前核查快照许可文本一致。许可快照不等于已确认的原代码移植版本 |

本仓库首个定制提交为 `c6c6c4d7a579fc33259aa5e400c230bc7faaff41`（2026-10-18），其父提交是 `3217f725`。Git 中缺少本地 `v3.20.1` 标签时，仍可使用上述固定提交核查基线，不应通过重新创建上游标签来补写来源。

CodexBar 对应的公开参考文件为 [GrokAuth.swift](https://github.com/steipete/CodexBar/blob/7447bc77c2497323d9c41e40164ce705533493c8/Sources/CodexBarCore/Providers/Grok/GrokAuth.swift) 和 [GrokWebBillingFetcher.swift](https://github.com/steipete/CodexBar/blob/7447bc77c2497323d9c41e40164ce705533493c8/Sources/CodexBarCore/Providers/Grok/GrokWebBillingFetcher.swift)。现有移植提交未记录 CodexBar 的原始提交号，因此尚不能确认原移植快照与完整文件对应范围。

### CodexBar 许可副本核查

2026-11-11，核查 [2026-01-22 的许可提交](https://github.com/steipete/CodexBar/commit/c1e05761e55ddd3d66cdd43edbe7823f627ee6a8)与 [当前核查快照许可](https://github.com/steipete/CodexBar/blob/7447bc77c2497323d9c41e40164ce705533493c8/LICENSE)：两者 `LICENSE` 的 Git blob 均为 `0ae0cb57d8c6c1417f796b39fc0d6a7f2f7c5c39`。本仓库按字节保存原文至 `licenses/CodexBar-MIT.txt`，1074 字节，SHA-256 为 `14293556b79940745123d0160c71d27ed0e9fe9b8a848093f3ed78f4853caafe`。

该记录确认许可文本在 CC Switch 移植前已存在，保留完整版权与 MIT 条款；它不补写尚未找到的原代码提交号。许可副本已随 `.3` Windows 包分发，并通过实际安装与升级内容核对；见[许可分发记录](THIRD-PARTY-PACKAGING.md)。已公开 `.1`、`.2` 包保留历史内容。

### 图标与第三方通知

图标来源核查见 [ICON-SOURCES.md](ICON-SOURCES.md)：49 个独立 SVG 与历史固定包 `@lobehub/icons-static-svg@1.73.0` 逐字节一致，对应 LobeHub MIT 许可原文保存于 [LobeHub-Icons-MIT.txt](../licenses/LobeHub-Icons-MIT.txt)。未匹配的 SVG、位图和自定义内联资源继续逐项追溯，不能用该子集结论覆盖全部图标。

已确认来源的汇总见 [THIRD-PARTY-NOTICES.md](../THIRD-PARTY-NOTICES.md)。已核实副本与通知随 `.3` Windows 包公开分发，实际检查见[`.3` 发布记录](releases/3.20.1-yunswitch.3.md)。其他资源和平台的许可范围仍按逐项记录核查。

### Codex Responses shim 兼容移植

本次工作分支将 [agentrouter-responses-shim-sentry-fix](https://gitee.com/yunhanhai/agentrouter-responses-shim-sentry-fix) 提交 `ff6277e05beff1bfc61e2a3abc0741deb0d42b3c` 中 `src-patched/sanitize.mjs` 的协议历史清理和嵌套密文修复改编为 Rust，位于 `src-tauri/src/proxy/codex_responses_compat.rs`。Node HTTP 服务、重试与路由代码未复制执行，继续使用 YunSwitch 的原生实现。

基础包 `agentrouter-responses-shim@2.0.1` 使用 MIT，通知为 `Copyright (c) 2026 stevenarai`。原文副本见 [AgentRouter-Responses-Shim-MIT.txt](../licenses/AgentRouter-Responses-Shim-MIT.txt)；输入补丁由 yunhanhai 维护，已保留出处。副本取自固定提交的原始 Git blob，1067 字节，SHA-256 为 `2a9691565b7d9d3770d90c035f865153f1d830c1edd7887a338ec4165202f84c`。

改编内容包括默认关闭的 UI 开关、仅原生 Codex Responses/API Key 路径生效、协议容器限定、压缩历史/状态引用保护、对出站副本处理和回归测试。具体范围与限流边界见 [Codex 兼容说明](CODEX-RESPONSES-COMPATIBILITY.md)。新增副本将随后续新包分发，当前公开 `.3` 包不因此改变。

## 文档与设计参考

[RuoYi](https://gitee.com/y_project/RuoYi)、[RuoYi-Vue-Plus](https://gitee.com/dromara/RuoYi-Vue-Plus) 和 [RuoYi-Vue-Pro](https://gitee.com/zhijiantianya/ruoyi-vue-pro) 用于参考如何说明上游关系、独立维护主体和功能差异；当前没有因此新增这些项目的框架代码。今后若引入代码或资源，应新增对应条目，核查所选版本的许可证，不能沿用“仅参考”的描述。

## 后续引入记录

复制、移植或改编第三方实现与资源时，在对应 PR 中填充以下记录，合并后纳入本台账。仅借鉴设计也应说明参考依据与自行实现范围。

```text
能力或资源：
原项目与固定来源链接：
原版本 / 提交 / 文件：
许可证及版权通知（所选快照）：
本仓库涉及文件：
采用方式（复制 / 移植 / 改编 / 设计参考）：
本项目修改及未采用部分：
许可通知的保留位置与随包分发方式：
验证记录、限制与关联 PR：
```

来源记录应让后来维护者找到具体实现与许可文本。保留原文件中的版权和许可通知；针对新增来源，根据实际许可证处理署名、许可副本及资源使用要求，不能用本仓库的 MIT 声明覆盖第三方原有条款。

## 尚待补齐

- 追溯 CodexBar 原代码移植版本与范围；许可副本已在 `.3` Windows 包中验收，但不宣称覆盖所有间接来源通知。
- 逐步核查继承的图标、图片和其他复制资源，分别记录来源与使用条款；包管理依赖以固定锁文件为入口另行核查。
- 新引入能力在对应 PR 中记录固定来源，避免仅使用会随项目更新变化的默认分支链接。
