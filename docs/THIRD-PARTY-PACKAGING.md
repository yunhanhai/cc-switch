# 第三方通知随包分发

源码仓库保存已核实的 [第三方通知](../THIRD-PARTY-NOTICES.md)、[CodexBar MIT 原文](../licenses/CodexBar-MIT.txt)与 [LobeHub MIT 原文](../licenses/LobeHub-Icons-MIT.txt)。来源及尚未确认的资源范围见[来源台账](PROVENANCE.md)和[图标记录](ICON-SOURCES.md)。许可副本保持原文字节，不通过格式化工具改写。

## 后续打包配置

- Windows MSI 在安装目录放置原文 `LICENSE`、已有 `LICENSE.rtf`、`THIRD-PARTY-NOTICES.md` 和 `licenses/` 中两份 MIT 文本。文件通过独立 WiX 组件安装，使用 HKCU key path；正式版与开发版具有不同且固定的组件 GUID。
- 便携 ZIP 放置 `LICENSE`、`THIRD-PARTY-NOTICES.md`、两份许可、运行文件与数据目录说明。通知中的许可链接可在包内按相对路径读取。
- macOS/Linux 使用通用 Tauri 资源映射保留通知与 `licenses/`。这里只完成配置，实际各平台包仍需在相应发布中构建和验收。
- Windows 验收使用 `--require-third-party-notices true` 时，在便携包、首次 MSI 安装和升级后的安装目录逐字节检查通知与许可文本。包复验还比较许可源文件，避免复用许可内容不同的构建产物。

## 本次验证

2026-11-11，使用本机已有可执行文件完成 MSI **重新打包与文件提取**，未作为正式新版本构建。提取的 `LICENSE`、通知和两份许可与源码字节完全一致；开发版 WiX fragment 编译通过，正式版与开发版组件 GUID 不重叠。便携 ZIP 使用发布工作流的模拟输入打包，解压后同样逐字节核验通过。

| 许可副本                | SHA-256                                                            |
| ----------------------- | ------------------------------------------------------------------ |
| `CodexBar-MIT.txt`      | `14293556b79940745123d0160c71d27ed0e9fe9b8a848093f3ed78f4853caafe` |
| `LobeHub-Icons-MIT.txt` | `add9d7531d1b21646317a8958e38fc727506fa39d24bdecb44154d943c82753a` |

发布工作流的 YAML、Bash、PowerShell 与 Node 语法、便携包文件集合及本地文档链接已检查。此记录确认打包内容，尚未验证新增组件的实际安装、升级和卸载；下一版需从本次源码重新构建并执行干净 runner 验收。

已公开的 `3.20.1-yunswitch.1`、`.2` 包保持历史内容。本次没有替换发布附件，也不将本机重新打包文件作为这两个版本的公开产物。新加入许可通知应在新的版本标签与发布记录中注明。
