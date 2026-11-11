# YunSwitch 独立发布准备

`3.20.1-yunswitch.3` Windows x64 预发布已在 Gitee、GitHub 公开。许可内容、文件校验与实际验收见[`.3` 发布记录](releases/3.20.1-yunswitch.3.md)。已公开 `.2` 的文件校验与验收见[`.2` 发布记录](releases/3.20.1-yunswitch.2.md)，首版记录见[历史发布](releases/3.20.1-yunswitch.1.md)。两边默认分支仍为 `main`。

## `.4` 验收准备

- [x] Codex 兼容开关、429 错误响应修复及模拟上游测试完成，功能提交的源码 CI 通过。
- [x] 源码版本统一为 `3.20.1-yunswitch.4`，MSI 版本为 `3.20.1.4`，增加 shim 的 MIT 原文随包分发。
- [ ] 最终版本源码 CI 与 Windows 全新构建通过。
- [ ] 便携版及 `.3` 升级后的 MSI 验证开关、JSON/SSE、429 等待重试和状态头、旧设置兼容及重启保存。
- [ ] 首次安装、升级、卸载、数据保留及三处许可内容验收通过。
- [ ] 下载回读最终产物、核对 SHA-256，并同步两边标签和发布附件。

来源与完整边界见[`.4` 发布记录](releases/3.20.1-yunswitch.4.md)。当前公开包仍为 `.3`。

## `.3` 已完成验收

- [x] 已核实的第三方许可副本和通知加入打包配置，Windows 正式版与开发版文件组件采用各自固定 GUID。
- [x] 源码版本统一为 `3.20.1-yunswitch.3`，Windows MSI 版本为 `3.20.1.3`，沿用独立 UpgradeCode。
- [x] [最终源码 CI](https://github.com/yunhanhai/yunswitch/actions/runs/42860851450)通过，构建源码及许可文件与验收来源一致。
- [x] 干净 Windows runner 完成新包首次安装、界面启动、模拟代理请求、合成旧 SQL 导入与重启验收。
- [x] 从已公开 `.2` MSI 升级，确认产品替换、供应商与设置保留、协议与快捷方式，以及通知和许可文本内容。
- [x] 首次安装及升级后逐字节核验通知和许可副本；卸载移除程序、通知组件与登记，并保留数据，便携包仍可读取。
- [x] 核对最终 MSI / 便携 ZIP 及 SHA-256，记录源码、验收运行与截图。
- [x] 合并 GitHub #14 / Gitee #7，标签 `v3.20.1-yunswitch.3` 指向 `722a3f7a`，两边发布同一批已验收文件。

## `.2` 已完成验收

- [x] 目录清理 PR 已合并：内置中性链接、取消商业优先排序，保留 Gemini 认证兼容键与配置模板。
- [x] 源码版本统一为 `3.20.1-yunswitch.2`，Windows MSI 版本为 `3.20.1.2`，沿用独立 UpgradeCode。
- [x] [新版本源码 CI](https://github.com/yunhanhai/yunswitch/actions/runs/42835576117)通过，最终构建源码与验收源码的产品文件一致。
- [x] 干净 Windows runner 使用新包完成安装、实际界面、代理模拟请求、重启、合成旧 SQL 导入、卸载与数据保留验收。
- [x] 从已公开 `3.20.1-yunswitch.1` MSI 升级，确认版本替换、单实例、协议登记与合成数据保留。
- [x] 核对最终 MSI / 便携 ZIP 和 SHA-256，记录验收运行与源码提交。
- [x] 合并 GitHub #11 / Gitee #4，标签 `v3.20.1-yunswitch.2` 指向 `fc24f4de`，两边公开同一批已验收文件。

## 首版已落实

- [x] 源码版本统一，YunSwitch 名称、应用 ID、深链、独立安装 UpgradeCode 与 Flatpak ID 已配置。
- [x] release / debug 分别使用 `.yunswitch` / `.yunswitch-dev`，新库端口独立，原有端口保留。
- [x] 自启动登记与 Claude Desktop profile 分配独立名称和 ID。
- [x] 四语品牌与本人仓库入口一致，保留 MIT 原始来源，继承促销文案不冒称本项目合作关系。
- [x] [旧配置迁移指南](MIGRATION-YUNSWITCH.md)已编写，免安装模式的真实数据位置已说明。
- [x] 自动更新关闭，继承公钥移除；R2 镜像默认关闭并要求本人显式配置。
- [x] 发布流程改为手动执行，默认只构建 Windows x64；所选平台全部构建与检查成功后生成草稿预发布，验收记录核对后再公开。Apple 签名、公证及 updater 签名需要明确启用。
- [x] 安装包随附 MIT 许可证，免安装 ZIP 同时附数据目录与手动升级说明。
- [x] 本次提交已同步至 [GitHub PR #9](https://github.com/yunhanhai/yunswitch/pull/9) 与 [Gitee PR #2](https://gitee.com/yunhanhai/yunswitch/pulls/2)，[对应跨平台 CI](https://github.com/yunhanhai/yunswitch/actions/runs/42456495217)全部通过，计数见预发布说明。
- [x] 本机 Windows x64 release 构建、MSI 身份与协议登记检查通过；已生成最终 MSI、免安装 ZIP 及 SHA-256，记录见预发布说明。

## 首版 Windows 验收与发布

- [x] 在干净 Windows CI 环境完成安装、实际界面、代理请求、重启、单实例、合成旧 SQL 导入、卸载及数据保留验收，保存截图与 JSON：[通过记录](https://github.com/yunhanhai/yunswitch/actions/runs/42464519875)。真实旧数据迁移仍应按迁移指南用私有备份另行验证。
- [x] 两边 PR 合并并保留各自历史，`main` 同步为 `16c09ead`；标签 `v3.20.1-yunswitch.1` 指向该版本，Windows x64 预发布在两边公开。

## 后续可选工作

- [ ] 对实际发布的 macOS、Linux / ARM64 包分别记录构建与安装检查，不能以 Windows 结果代替。
- [ ] 如需自动更新，配置 YunSwitch 自有公钥、私钥和地址，并验证签名与更新链路；处理本机打包器的 `__TAURI_BUNDLE_TYPE` 标记警告后再验收更新安装。
- [ ] 如需可信平台签名，为 Windows 配置 Authenticode、macOS 配置 Developer ID 与公证。当前没有此类验证声明。

## Windows 发布流程

在 GitHub Actions 中手动运行 `YunSwitch prerelease`，`scope` 选 `windows-x64`。填写 `source_ref` 时仅构建和验收指定分支或提交，不创建 Release；留空时要求已经存在且版本一致的标签，并生成草稿 Release。标签推送本身不触发发布。

验收工作流使用全新的 Windows 用户配置，并且只允许在 GitHub Actions Windows runner 上运行安装检查。它通过 WebView2 调试接口检查实际生产界面并保存截图；这属于自动运行验收，不代表人工审查所有交互。`all` 模式用于后续跨平台构建，其他平台仍需独立安装检查。公开 Release 只列出本次实际验收并上传的平台。

CI 固定 Node 与 Rust 工具链；Windows 大测试目标使用 32 MB 编译线程栈，后端串行编译。验证结果以对应提交的运行记录为准。
