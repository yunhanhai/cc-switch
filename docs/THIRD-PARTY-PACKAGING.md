# 第三方通知随包分发

源码仓库保存已核实的 [第三方通知](../THIRD-PARTY-NOTICES.md)、[CodexBar MIT 原文](../licenses/CodexBar-MIT.txt)、[LobeHub MIT 原文](../licenses/LobeHub-Icons-MIT.txt)与 [AgentRouter Responses Shim MIT 原文](../licenses/AgentRouter-Responses-Shim-MIT.txt)。来源及尚未确认的资源范围见[来源台账](PROVENANCE.md)和[图标记录](ICON-SOURCES.md)。许可副本保持原文字节，不通过格式化工具改写。

## 已发布 Windows 包与其他平台配置

- Windows MSI 在安装目录放置原文 `LICENSE`、已有 `LICENSE.rtf`、`THIRD-PARTY-NOTICES.md` 和 `licenses/` 中三份 MIT 文本。文件通过独立 WiX 组件安装，使用 HKCU key path；正式版与开发版具有不同且固定的组件 GUID。
- 便携 ZIP 放置 `LICENSE`、`THIRD-PARTY-NOTICES.md`、三份许可、运行文件与数据目录说明。通知中的许可链接可在包内按相对路径读取。
- macOS/Linux 使用通用 Tauri 资源映射保留通知与 `licenses/`。这里只完成配置，实际各平台包仍需在相应发布中构建和验收。
- Windows 验收使用 `--require-third-party-notices true` 时，在便携包、首次 MSI 安装和升级后的安装目录逐字节检查通知与许可文本。包复验还比较许可源文件，避免复用许可内容不同的构建产物。

## 本次验证

2026-11-11，使用本机已有可执行文件完成 MSI **重新打包与文件提取**，未作为正式新版本构建。提取的 `LICENSE`、通知和两份许可与源码字节完全一致；开发版 WiX fragment 编译通过，正式版与开发版组件 GUID 不重叠。便携 ZIP 使用发布工作流的模拟输入打包，解压后同样逐字节核验通过。

| 许可副本                | SHA-256                                                            |
| ----------------------- | ------------------------------------------------------------------ |
| `CodexBar-MIT.txt`      | `14293556b79940745123d0160c71d27ed0e9fe9b8a848093f3ed78f4853caafe` |
| `LobeHub-Icons-MIT.txt` | `add9d7531d1b21646317a8958e38fc727506fa39d24bdecb44154d943c82753a` |

随后从 `6f91d0cb9ab71dfd17f2bc8b7d37c8539f5f1787` 重新构建 `.3`，在[干净 Windows runner](https://github.com/yunhanhai/yunswitch/actions/runs/42860850983)完成 40 项运行验收。便携包、首次安装和 `.2`→`.3` 升级后的通知与许可逐字节核对通过；两次卸载均清除许可文件、目录和注册项，同时保留应用数据。源码 CI 和最终文件校验见[`.3` 发布记录](releases/3.20.1-yunswitch.3.md)。

`3.20.1-yunswitch.3` 已在 Gitee、GitHub 公开分发这些许可文件。已公开 `.1`、`.2` 包保持历史内容，本次未替换其附件；本机较早重新打包文件仍仅作为打包验证记录。其他平台的实际安装包许可内容仍需分别验收。

## `.4` 新增 shim 许可分发

`.4` 增加 `AgentRouter-Responses-Shim-MIT.txt`（SHA-256 `2a9691565b7d9d3770d90c035f865153f1d830c1edd7887a338ec4165202f84c`）。[实际新包验收](https://github.com/yunhanhai/yunswitch/actions/runs/42999640461)在便携 ZIP、首次 MSI 安装和 `.3` 升级后的目录逐字节核对三份原文许可；卸载文件/目录/登记清除检查通过。[发布记录](releases/3.20.1-yunswitch.4.md)包含同批包校验值，旧版包保持原内容。
