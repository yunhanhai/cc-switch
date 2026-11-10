# 图标来源核查

2026-11-11，按 Git 历史、固定 npm 包与文件内容核查现有图标。本文说明已确认的来源和仍缺少外部许可证据的资源，不将所有品牌标志归为本项目原创，也不将根 MIT 许可证套用于未知来源。

## 已确认的继承范围

`src/icons/extracted/` 的 101 个文件（73 SVG、26 位图资源、2 个 TypeScript 文件）、`src-tauri/icons/` 的 54 个文件，以及 `src/components/BrandIcons.tsx`，共 156 个已跟踪路径，均与 CC Switch 基线 `3217f72596f2d1c0f879f0a05f83803825d9809f` 的 Git blob 相同。本次核查没有修改图标或展示代码。

`BrandIcons.tsx` 引用本地 Claude、OpenAI、Gemini、OpenClaw 图标；它属于显示组件，不能作为外部图标许可的证明。目录中的文件和索引内联 SVG 可能并不完全相同，例如 Kimi 的索引内容在上游 `26be9324` 单独调整过，因此以下比对结论仅覆盖列出的 SVG 文件。

## LobeHub SVG 集合

上游提交 [`d38fcd63ea38d96e8c35733a8a7e2163ec62a713`](https://github.com/farion1231/cc-switch/commit/d38fcd63ea38d96e8c35733a8a7e2163ec62a713)（2025-11-22）添加图标目录及提取脚本。该提交的 `scripts/extract-icons.js` 明确从 `node_modules/@lobehub/icons-static-svg/icons` 复制 SVG，`pnpm-lock.yaml` 固定版本为 **1.73.0**。配套 `filter-icons.js` 会优先保留颜色版并重命名为基础文件名。提取脚本与依赖后来在 `3c1154bed95a9e7cc8fe8664f046cde5560141a0` 删除，已复制的图标继续保留。

本次使用 [npm 1.73.0 元数据](https://registry.npmjs.org/@lobehub%2ficons-static-svg/1.73.0)与其固定发布包比对，元数据中的 integrity 与历史锁文件一致。该包记录的 `gitHead` 为 `798b90d7d65cd764b7a2dee1f95e7bd56eacb602`；对应 [LobeHub LICENSE](https://github.com/lobehub/lobe-icons/blob/798b90d7d65cd764b7a2dee1f95e7bd56eacb602/LICENSE) 为 MIT，版权通知为 `Copyright (c) 2023 LobeHub`。包本身未包含 `LICENSE` 文件，许可副本需从上述固定仓库快照保留。

运行时索引单独核查了 73 个内联 SVG 和 29 个 URL 引用，其中 50 个内联值匹配该固定包，详见[内联图标明细](ICON-INLINE-DETAILS.md)。此结果与下述 49 个独立文件的匹配范围不同，不能直接相加；同名文件与实际内联值也可能不同。

| 固定证据                  | 值                                                                                                |
| ------------------------- | ------------------------------------------------------------------------------------------------- |
| npm 发布包                | `@lobehub/icons-static-svg@1.73.0`                                                                |
| 发布包 SHA-256            | `0eb77ba5ded6be19e93a7135aa524f35c523dc04d98f13da210c51d2e5f36915`                                |
| 历史锁文件 integrity      | `sha512-ydKUCDoopdmulbjDZo/gppaODd5Ju5nPneVcN9A5dAz9IJZUMkLms8bqostMLrqcdMQ8resKjLuV9RhJaWhaag==` |
| 固定 LICENSE 原文 SHA-256 | `add9d7531d1b21646317a8958e38fc727506fa39d24bdecb44154d943c82753a`                                |

当前 73 个独立 SVG 文件中，**49 个与该包中的 SVG 逐字节相同**。以下映射保留颜色版和名称转换信息；该结论不表示后来新增文件都通过同一次脚本引入。

| 本仓库 SVG                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | npm 包 SVG                                                         |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------ |
| `aihubmix-color.svg`、`anthropic.svg`、`github.svg`、`githubcopilot.svg`、`grok.svg`、`longcat-color.svg`、`mcp.svg`、`midjourney.svg`、`modelscope-color.svg`、`notion.svg`、`ollama.svg`、`openai.svg`、`openrouter.svg`、`vercel.svg`、`xai.svg`、`zeroone.svg`                                                                                                                                                                                                           | 同名文件                                                           |
| `alibaba.svg`、`aws.svg`、`azure.svg`、`baidu.svg`、`bytedance.svg`、`chatglm.svg`、`claude.svg`、`cloudflare.svg`、`cohere.svg`、`copilot.svg`、`deepseek.svg`、`doubao.svg`、`gemini.svg`、`gemma.svg`、`google.svg`、`googlecloud.svg`、`huawei.svg`、`huggingface.svg`、`hunyuan.svg`、`kimi.svg`、`meta.svg`、`minimax.svg`、`mistral.svg`、`nvidia.svg`、`palm.svg`、`perplexity.svg`、`qwen.svg`、`stability.svg`、`tencent.svg`、`wenxin.svg`、`yi.svg`、`zhipu.svg` | 基础名称加 `-color.svg`，例如 `claude.svg` 对应 `claude-color.svg` |
| `siliconflow.svg`                                                                                                                                                                                                                                                                                                                                                                                                                                                            | `siliconcloud-color.svg`                                           |

这些文件位于 [src/icons/extracted](../src/icons/extracted)。LobeHub 集合的 MIT 条款不自动证明所有品牌标志的商标使用范围；展示标志也不表示品牌方与 YunSwitch 存在合作关系。

## 其他继承资源

以下 **24 个 SVG 未匹配上述固定包**，部分可能经过改写或来自其他版本，不能仅因外观相似归入 LobeHub：`aicodemirror.svg`、`aicoding.svg`、`algocode.svg`、`amuxapi-icon.svg`、`bailian.svg`、`catcoder.svg`、`ccsub.svg`、`claw.svg`、`crazyrouter.svg`、`cubence.svg`、`lioncc.svg`、`micu.svg`、`newapi.svg`、`novita.svg`、`opencode-logo-light.svg`、`packycode.svg`、`ppio.svg`、`rc.svg`、`shengsuanyun.svg`、`sssaicode.svg`、`stepfun.svg`、`subrouter.svg`、`ucloud.svg`、`xiaomimimo.svg`。

Git 可追溯它们的上游添加记录，例如 OpenClaw `9bbb6c51`、OpenCode `cb1b45ae`、SubRouter `430ddf92`、UCloud `02669cfb`、PPIO `3711e1a0`。添加预设或注册图标的提交说明能证明继承关系，尚不足以确认品牌方素材地址、许可版本或授权范围。

24 个未逐字节匹配的独立 SVG，其首次引入提交及进一步公开源文件核查见[SVG 明细](ICON-SVG-DETAILS.md)。明细分别标注逐字节匹配、仅格式差异和来源仍待确认。

26 个位图资源包括供应商品牌图片和 Hermes 图标；本次仅确认它们从 CC Switch 继承，尚未逐项匹配外部文件或使用条款。内联索引中的自定义 SVG（例如 Pi）也需要独立记录，不能用上述 49 文件的字节比对结果覆盖。

后续[位图明细](ICON-BITMAP-DETAILS.md)已追溯 Hermes 图标的固定原文件、裁切处理和缩小历史，其余 25 个位图仍待确认外部素材和许可；平台图标按生成与变更记录分组。本文最初的字节匹配统计保持原范围，新增证据在各明细中单独说明。

应用及平台图标同样沿用上游。提交 `adee37ab660cba787d24b9c4a132ef12851e86b6`（2025-08-26）记录彩色放射状应用图标更新，`6932e89ea89ef2b84e161c551baa205f50970581` 与 `2a43f1f54dd29562eee810a03676654f75dc065f`（2025-08-27）记录统一源图生成平台尺寸。本次确认 Git 继承关系，未取得独立原始设计来源或第三方素材清单。

## 后续维护

新增、替换图标时记录原始素材地址、固定版本或下载摘要、作者和许可、涉及文件与变形处理，并在[来源台账](PROVENANCE.md)登记。已有未确认资源逐项追溯，能够找到可靠来源后再补许可副本；文件删除、品牌替换和重新设计应作为独立改动处理。

当前 `.3` Windows 包已随附 CodexBar 和 LobeHub 的许可副本，实际分发与安装验收见[`.3` 发布记录](releases/3.20.1-yunswitch.3.md)。本次逐文件核查只更新来源文档；`.1`、`.2`、`.3` 包的资源和许可保持各自发布时的内容，未确认资源不据此声明已完成许可审计。
