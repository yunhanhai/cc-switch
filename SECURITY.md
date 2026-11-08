# Security Policy / 安全策略

This policy applies to the edition independently maintained by **yunhanhai**, hosted on [Gitee](https://gitee.com/yunhanhai/yunswitch) and [GitHub](https://github.com/yunhanhai/yunswitch).

本策略适用于 **yunhanhai** 独立维护的版本，仓库位于 [Gitee](https://gitee.com/yunhanhai/yunswitch) 和 [GitHub](https://github.com/yunhanhai/yunswitch)。

## Maintenance / 维护范围

Security work focuses on the current default branch, `feature/v3.20.1-custom-routing`, and this project's own subsequent releases. Historical upstream tags retained in Git are not a promise of security maintenance. Include the affected commit or release in your report.

安全维护优先面向当前默认分支 `feature/v3.20.1-custom-routing` 及本项目后续自行发布的版本。Git 中保留的上游历史标签不代表本项目承诺维护对应版本。报告请注明受影响提交或版本。

## Reporting / 报告方式

**Do not publish vulnerability details, working exploits or credentials in public issues.** Contact the maintainer through the [Gitee profile](https://gitee.com/yunhanhai) or [GitHub profile](https://github.com/yunhanhai) to confirm an available private reporting channel first. If no contact channel is listed, open an issue asking only how to contact the maintainer privately, without vulnerability details or personal information.

**不要在公开 Issue 发布漏洞细节、可利用代码或凭据。** 请先通过维护者的 [Gitee 个人主页](https://gitee.com/yunhanhai)或 [GitHub 个人主页](https://github.com/yunhanhai)联系，确认可用的私密报告渠道。如果主页没有联系方式，可仅发起询问私密联系渠道的 Issue，不附漏洞细节或个人信息。

GitHub private vulnerability reporting has not been verified as enabled for this repository. This policy does not assume that the upstream project's reporting channels are available here.

尚未核实本仓库是否启用 GitHub 私密漏洞报告功能；本策略不将上游项目的报告渠道视为本项目的可用渠道。

Once a private channel is confirmed, include:

确认私密渠道后，请提供：

- Affected version or commit and OS / 受影响版本或提交及操作系统。
- Reproduction steps using dummy credentials / 使用虚构凭据的复现步骤。
- The source of untrusted input and its path to the affected code / 不可信输入来源及其到达受影响代码的路径。
- Potential impact and any workaround / 潜在影响与可用临时措施。

## Scope / 问题范围

Relevant reports include risks in configuration imports, deep links, remote sync data, API responses, the local proxy, rendering, and the build, signing or update pipeline. Credential leakage into logs or shared configuration is also relevant. For code execution reports involving MCP servers or user scripts, explain how an untrusted party can cause execution without the user's informed decision.

相关问题包括配置导入、深层链接、远程同步数据、API 响应、本地代理、前端渲染以及构建、签名或更新链路中的风险，也包括凭据进入日志或共享配置。涉及 MCP 服务或用户脚本执行时，请说明不可信第三方如何在用户未充分知情并作出决定的情况下触发执行。

Provide a demonstrable impact and input path where possible. Scanner output alone or direct invocation from an already modified local environment may not establish a vulnerability in this project.

尽可能提供可复现的影响与输入路径。单独的扫描工具输出，或从已经修改的本地环境直接调用接口，不一定能证明本项目存在漏洞。

## Response and disclosure / 响应与披露

The project is currently maintained by one person. Review and fixes depend on severity and available time; there are no fixed acknowledgment or repair deadlines. Coordinate the disclosure timing with the maintainer. Credits in a release note should follow the reporter's consent and anonymity preference.

目前由一人维护，评估与修复根据严重程度和可用时间安排，不承诺固定确认或修复期限。披露时间请与维护者协调；发布说明中的致谢应遵循报告者的同意及匿名偏好。

Published fixes will be described in this project's [GitHub Releases](https://github.com/yunhanhai/yunswitch/releases) or [Gitee releases](https://gitee.com/yunhanhai/yunswitch/releases). Consult the actual release notes for the affected and fixed versions.

已发布的修复会在本项目的 [GitHub Releases](https://github.com/yunhanhai/yunswitch/releases) 或 [Gitee 发行版](https://gitee.com/yunhanhai/yunswitch/releases)中说明，受影响及已修复版本以具体发布说明为准。
