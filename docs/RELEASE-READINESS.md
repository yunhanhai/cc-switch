# YunSwitch 独立发布准备

首个独立预发布源码版本为 `3.20.1-yunswitch.1`。仓库改名保留原历史，两边默认分支为 `main`。本次验证与产物记录见[预发布说明](releases/3.20.1-yunswitch.1.md)。

## 已落实

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

## Windows 验收与发布

- [x] 在干净 Windows CI 环境完成安装、实际界面、代理请求、重启、单实例、合成旧 SQL 导入、卸载及数据保留验收，保存截图与 JSON：[通过记录](https://github.com/yunhanhai/yunswitch/actions/runs/42464519875)。真实旧数据迁移仍应按迁移指南用私有备份另行验证。
- [x] 两边 PR 合并并保留各自历史，`main` 同步为 `16c09ead`；标签 `v3.20.1-yunswitch.1` 指向该版本，Windows x64 预发布在两边公开。

## 后续可选工作

- [ ] 对实际发布的 macOS、Linux / ARM64 包分别记录构建与安装检查，不能以 Windows 结果代替。
- [ ] 如需自动更新，配置 YunSwitch 自有公钥、私钥和地址，并验证签名与更新链路；处理本机打包器的 `__TAURI_BUNDLE_TYPE` 标记警告后再验收更新安装。
- [ ] 如需可信平台签名，为 Windows 配置 Authenticode、macOS 配置 Developer ID 与公证。当前没有此类验证声明。

## 首次 Windows 发布流程

在 GitHub Actions 中手动运行 `YunSwitch prerelease`，`scope` 选 `windows-x64`。填写 `source_ref` 时仅构建和验收指定分支或提交，不创建 Release；留空时要求已经存在且版本一致的标签，并生成草稿 Release。标签推送本身不触发发布。

验收工作流使用全新的 Windows 用户配置，并且只允许在 GitHub Actions Windows runner 上运行安装检查。它通过 WebView2 调试接口检查实际生产界面并保存截图；这属于自动运行验收，不代表人工审查所有交互。`all` 模式用于后续跨平台构建，其他平台仍需独立安装检查。公开 Release 只列出本次实际验收并上传的平台。

CI 固定 Node 与 Rust 工具链；Windows 大测试目标使用 32 MB 编译线程栈，后端串行编译。验证结果以对应提交的运行记录为准。
