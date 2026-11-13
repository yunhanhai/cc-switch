# Third-party notices / 第三方来源与许可

YunSwitch is independently maintained by yunhanhai. Retained third-party code and assets keep their original attribution and license terms. The root [LICENSE](LICENSE) covers the CC Switch baseline and this project's modifications; the notices below supplement it.

YunSwitch 由 yunhanhai 独立维护。继承的代码和资源保留各自署名与许可条款；以下通知补充根 [LICENSE](LICENSE)，不将全部第三方内容声明为本项目原创。

| Component / 来源                                                                                                      | Included scope / 涉及范围                                                                                                  | Notice and license / 署名与许可                                                                                             |
| --------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| [CC Switch](https://github.com/farion1231/cc-switch)                                                                  | Desktop configuration manager and local proxy baseline, v3.20.1 / 桌面配置管理与本地代理基础                               | Copyright (c) 2025 Jason Young; MIT text in [LICENSE](LICENSE)                                                              |
| [CodexBar](https://github.com/steipete/CodexBar)                                                                      | Grok subscription query port inherited through CC Switch / 经 CC Switch 继承的 Grok 订阅额度查询移植                       | Copyright (c) 2026 Peter Steinberger; [MIT license](licenses/CodexBar-MIT.txt)                                              |
| [LobeHub Icons](https://github.com/lobehub/lobe-icons)                                                                | SVG files traced to `@lobehub/icons-static-svg@1.73.0` / 可追溯至该固定包的 SVG 文件                                       | Copyright (c) 2023 LobeHub; [MIT license](licenses/LobeHub-Icons-MIT.txt)                                                   |
| [agentrouter-responses-shim and yunhanhai's patch](https://gitee.com/yunhanhai/agentrouter-responses-shim-sentry-fix) | Optional Codex Responses history repair adapted to Rust / 移植为 Rust 的可选 Codex Responses 历史兼容处理                  | Copyright (c) 2026 stevenarai; [MIT license](licenses/AgentRouter-Responses-Shim-MIT.txt)                                   |
| [async-compression](https://github.com/Nullus157/async-compression)                                                   | Incremental response decoding; async-compression 0.4.41, compression-codecs 0.4.37, compression-core 0.4.31 / 增量响应解码 | Copyright (c) 2018 the rustasync developers; MIT option of MIT OR Apache-2.0, [license](licenses/Async-Compression-MIT.txt) |

Fixed sources and the verified file subset are recorded in the repository's `docs/PROVENANCE.md` and `docs/ICON-SOURCES.md`. The CodexBar license was verified at a commit predating the CC Switch port; the exact original code revision remains unrecorded. Matching LobeHub files does not establish the license of every icon, bitmap or inline variant.

固定来源与已核实文件范围见源码仓库的 `docs/PROVENANCE.md` 及 `docs/ICON-SOURCES.md`。CodexBar 许可已核对移植前的提交，原代码移植版本仍未找到。LobeHub 文件比对结论不覆盖全部图标、位图或内联变体。

Brand names and logos identify their respective products and do not imply sponsorship or endorsement. Package dependencies retain their own licenses; this file is a record of confirmed copied or ported components, not a complete dependency or asset audit.

品牌名称和标志用于识别对应产品，不表示商业合作或背书。包管理依赖保留其各自许可证；本文件记录已确认的复制或移植来源，尚不构成全量依赖和资源审计。
