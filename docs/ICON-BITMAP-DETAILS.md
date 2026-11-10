# 位图与平台图标来源详情

2026-11-11，核查 `src/icons/extracted/` 的 **26 个位图**与 `src-tauri/icons/` 的 **54 个平台资源**。80 个文件的当前 Git blob 均与 CC Switch 基线 `3217f72596f2d1c0f879f0a05f83803825d9809f` 相同；本次没有修改图像或显示组件。

Git 添加记录能够证明文件从上游继承；它不能自动证明上传图片的作者、品牌素材使用范围或外部许可证。文件名、供应商网址和“官方图标”描述不等同于固定来源证据。总体范围见[图标来源](ICON-SOURCES.md)。

## 供应商位图逐项记录

下表链接到文件首次出现的固定提交。图片后来可能缩放、换牌或从历史恢复，因此首次提交并不一定是当前像素的最后修改记录。

| 当前文件                                                                      | 首次添加提交                                                                                       | 已找到的来源或处理记录                                |
| ----------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- | ----------------------------------------------------- |
| [a6-icon.png](../src/icons/extracted/a6-icon.png)                             | [dbb26595](https://github.com/yunhanhai/yunswitch/commit/dbb265956e61300b021b986fe14f2d315f15df72) | 上传素材缩放；未记外部素材地址。                      |
| [apikeyfun.png](../src/icons/extracted/apikeyfun.png)                         | [8302f1e3](https://github.com/yunhanhai/yunswitch/commit/8302f1e36c6834fc4db7e0228206e291727fe787) | 预设添加，未记外部素材地址或许可。                    |
| [apinebula_icon.png](../src/icons/extracted/apinebula_icon.png)               | [9ef14190](https://github.com/yunhanhai/yunswitch/commit/9ef141902e3c311cdbcddeeb3c0afb9a7402c11f) | 预设添加，未记外部素材地址或许可。                    |
| [atlascloud_icon.png](../src/icons/extracted/atlascloud_icon.png)             | [32b30e43](https://github.com/yunhanhai/yunswitch/commit/32b30e43c0a700d6aab4d83ad1e8e8952f405510) | 预设添加，后缩至256px；未记外部素材地址或许可。       |
| [byteplus.png](../src/icons/extracted/byteplus.png)                           | [9050442b](https://github.com/yunhanhai/yunswitch/commit/9050442b650e5e70949f2d3934b10206aaf16506) | 预设添加，未记外部素材地址或许可。                    |
| [cherryin.png](../src/icons/extracted/cherryin.png)                           | [e458e77e](https://github.com/yunhanhai/yunswitch/commit/e458e77e3034a4e9fa38f8145411fbf3e30b3c29) | 提交称官方品牌图标，但未记固定素材地址或许可。        |
| [ClaudeApi.png](../src/icons/extracted/ClaudeApi.png)                         | [df11df4d](https://github.com/yunhanhai/yunswitch/commit/df11df4d9ce73e7c259bcdf442b1e750db8f27f2) | 预设添加，未记外部素材地址或许可。                    |
| [claudecn.png](../src/icons/extracted/claudecn.png)                           | [50a873ca](https://github.com/yunhanhai/yunswitch/commit/50a873ca2497fb40ddddb4c279251992afe9cdc0) | 上传素材缩放至512px，后缩至256px；未记外部素材地址。  |
| [code0.png](../src/icons/extracted/code0.png)                                 | [a8657d22](https://github.com/yunhanhai/yunswitch/commit/a8657d229a300b3a7c791fe98325151c75f72cf9) | 提交称 favicon-derived，未记固定 favicon 地址或许可。 |
| [eflowcode.png](../src/icons/extracted/eflowcode.png)                         | [7d21663e](https://github.com/yunhanhai/yunswitch/commit/7d21663e6dd9889508f5a3fa9106327368e5ecc5) | 预设添加及PNG引用，未记外部素材地址或许可。           |
| [etok.png](../src/icons/extracted/etok.png)                                   | [142c8c1d](https://github.com/yunhanhai/yunswitch/commit/142c8c1da7ce0b0bbfe62b90ea0bdd4c451dd5e4) | CTok→ETok换牌图，未记固定素材地址或许可。             |
| [fenno-icon.webp](../src/icons/extracted/fenno-icon.webp)                     | [238cd5d7](https://github.com/yunhanhai/yunswitch/commit/238cd5d7c9075225ef4d179d3fb197db12ab8ea9) | 预设添加及WebP引用，未记外部素材地址或许可。          |
| [hermes.png](../src/icons/extracted/hermes.png)                               | [5d3cd7eb](https://github.com/yunhanhai/yunswitch/commit/5d3cd7eb8581f64636d0c855138fb785db31dadc) | 固定来源与裁切已核查，后缩至256px；见下文。           |
| [huoshan.png](../src/icons/extracted/huoshan.png)                             | [d94eb672](https://github.com/yunhanhai/yunswitch/commit/d94eb6729b3f053ec8aac066eb953d66f7768a74) | 预设添加，未记外部素材地址或许可。                    |
| [nekocode-icon.png](../src/icons/extracted/nekocode-icon.png)                 | [9079935d](https://github.com/yunhanhai/yunswitch/commit/9079935d72a9025a6f710777af7f873cbde62ae5) | 提交称 logo-derived，未记固定素材地址或许可。         |
| [pateway.jpg](../src/icons/extracted/pateway.jpg)                             | [08cd5ab5](https://github.com/yunhanhai/yunswitch/commit/08cd5ab56e89ac7952603f86359be1f8bd1645f7) | 预设添加，未记外部素材地址或许可。                    |
| [pipellm.png](../src/icons/extracted/pipellm.png)                             | [33722454](https://github.com/yunhanhai/yunswitch/commit/337224546cd2aa609296c4444f531580dd1e7a24) | 预设更新及PNG引用，未记外部素材地址或许可。           |
| [qiniu.png](../src/icons/extracted/qiniu.png)                                 | [34767073](https://github.com/yunhanhai/yunswitch/commit/3476707321d2cde32b2d04109bed8cbc1ca756cd) | 预设添加，未记外部素材地址或许可。                    |
| [relaxcode.png](../src/icons/extracted/relaxcode.png)                         | [58cd5302](https://github.com/yunhanhai/yunswitch/commit/58cd5302ae5ea80b5fcd6f1109e064eb13d05e71) | 提交称自定义图，后缩至256px；未记外部素材地址。       |
| [runapi.jpg](../src/icons/extracted/runapi.jpg)                               | [50a873ca](https://github.com/yunhanhai/yunswitch/commit/50a873ca2497fb40ddddb4c279251992afe9cdc0) | 上传图从800px缩至512px；未记外部素材地址。            |
| [sudocode-us.png](../src/icons/extracted/sudocode-us.png)                     | [7f028632](https://github.com/yunhanhai/yunswitch/commit/7f028632b5eded64596e32b2c26fe61cb4bd59e1) | 恢复旧版历史图，摘要一致；外部来源仍未知。            |
| [sudocode.png](../src/icons/extracted/sudocode.png)                           | [e71b9091](https://github.com/yunhanhai/yunswitch/commit/e71b90916b5e0cfc1875f2779dd5f7ed2c7335fe) | 旧SudoCode图后来换为SudoCode.chat；未记外部素材地址。 |
| [TeamoRouter-icon-dark.png](../src/icons/extracted/TeamoRouter-icon-dark.png) | [01bf0037](https://github.com/yunhanhai/yunswitch/commit/01bf003783aacf38642da709ad8a8707ea32f828) | 预设添加，未记外部素材地址或许可。                    |
| [unity2.png](../src/icons/extracted/unity2.png)                               | [daa5595f](https://github.com/yunhanhai/yunswitch/commit/daa5595f362e44b50ad3c51f15ded924421b7a29) | 预设添加，未记外部素材地址或许可。                    |
| [xycai-icon.png](../src/icons/extracted/xycai-icon.png)                       | [eb69e492](https://github.com/yunhanhai/yunswitch/commit/eb69e4922ee187a261fd29c216a738e838f85bc4) | 预设添加，未记外部素材地址或许可。                    |
| [zetaapi-icon.png](../src/icons/extracted/zetaapi-icon.png)                   | [e54a3423](https://github.com/yunhanhai/yunswitch/commit/e54a3423012d367fad6fd88ead45e90278590c12) | 预设添加，未记外部素材地址或许可。                    |

这些提交中的历史合作身份仅说明上游添加背景，不表示 YunSwitch 与供应商存在合作。未注明外部许可的条目继续待核查，不归入已经确认的 LobeHub SVG 子集。

## Hermes 位图的固定来源与处理

上游 [`5d3cd7eb8581f64636d0c855138fb785db31dadc`](https://github.com/yunhanhai/yunswitch/commit/5d3cd7eb8581f64636d0c855138fb785db31dadc) 明确记录来源为 Hermes Agent 的 `landingpage/icon-512.png`，处理为将外侧 6px 裁掉后放回 512×512 透明画布。

本次找到 [NousResearch/hermes-agent 的固定原文件](https://github.com/NousResearch/hermes-agent/blob/580e6ba2ffd9351b9c2f4b76b1386070c06036dd/landingpage/icon-512.png)，提交 `580e6ba2ffd9351b9c2f4b76b1386070c06036dd` 为该图标的添加记录。原 PNG 为 512×512、137587 字节，Git blob 为 `c5b4c63a57548179183ccce46e3f50f91a3f53c5`，SHA-256 为 `f28c6f497ff49f7f34486879d61a202bc5a046538d28b1a46fc3f979a51490a1`。

逐像素对照确认上游首次 `hermes.png` 的中央 500×500 像素与原文件完全一致，外侧 6px 为全透明；原文件摘要不同是裁切处理的结果。

之后 [`ba87e8f69d92eaf12e0389cfad9fe0a575eec8fc`](https://github.com/yunhanhai/yunswitch/commit/ba87e8f69d92eaf12e0389cfad9fe0a575eec8fc) 将 Hermes、ClaudeCN 与 AtlasCloud 位图缩小。当前 Hermes 文件为 256×256、39304 字节，SHA-256 为 `8cf371872844d07db84bc2831c4c80e6ebc7b65e448930e034ce1fde17f4bc7e`；不能用当前缩小图与原图的字节不一致来否定已验证的来源链。

该固定 Hermes Agent 快照的 [LICENSE](https://github.com/NousResearch/hermes-agent/blob/580e6ba2ffd9351b9c2f4b76b1386070c06036dd/LICENSE) 为 MIT，通知为 `Copyright (c) 2025 Nous Research`；许可文件 Git blob 为 `75410e73319c72cd3e991a501c5455eb78f38375`，SHA-256 为 `821556e6336796450ab852d375117b48a4887e71d255794fd6318d99982a5ab6`。此记录核实仓库许可与素材处理，品牌名称与标志的使用不表示合作或背书。

## SudoCode 的历史恢复

`sudocode.png` 首次添加后，在 [`6f8e12a1ac587da5f0bf832aad29b0615889ec57`](https://github.com/yunhanhai/yunswitch/commit/6f8e12a1ac587da5f0bf832aad29b0615889ec57) 缩小，后来 [`618723b42fd0e8bd76ac44448d3a5fccbf1a5ab4`](https://github.com/yunhanhai/yunswitch/commit/618723b42fd0e8bd76ac44448d3a5fccbf1a5ab4) 将同名预设切换到另一家 SudoCode.chat。

恢复旧 SudoCode.us 的 [`7f028632b5eded64596e32b2c26fe61cb4bd59e1`](https://github.com/yunhanhai/yunswitch/commit/7f028632b5eded64596e32b2c26fe61cb4bd59e1) 新增 `sudocode-us.png`；该文件与 `6f8e12a1` 中的旧图 SHA-256 均为 `583346ffecb9560c50d37d07b7a4f523c8f60f50d582508fd78dc5d209f39165`。这只确认历史恢复，未确认外部原素材或许可证。

## 平台资源的生成与继承

54 个平台文件可按最后一次内容变更归为以下四组：

| 范围                                                | 数量 | 固定处理记录与核查边界                                                                                                                                                                                                                                                                                                                                                                                   |
| --------------------------------------------------- | ---: | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 桌面尺寸、ICO/ICNS、Android 和 iOS 图标             |   50 | [`2a43f1f54dd29562eee810a03676654f75dc065f`](https://github.com/yunhanhai/yunswitch/commit/2a43f1f54dd29562eee810a03676654f75dc065f) 更新全部平台图；此前 [`6932e89ea89ef2b84e161c551baa205f50970581`](https://github.com/yunhanhai/yunswitch/commit/6932e89ea89ef2b84e161c551baa205f50970581) 的提交说明明确“from unified source”。证明生成关系的提交记录，尚未找到可重现的生成脚本和独立原创素材清单。 |
| DMG 背景 `dmg-background.png`                       |    1 | [`141010332badd471e82f0da79610bf6649a91e3a`](https://github.com/yunhanhai/yunswitch/commit/141010332badd471e82f0da79610bf6649a91e3a) 记录重新生成 2× Retina 背景；未附外部设计素材来源。                                                                                                                                                                                                                 |
| macOS `statusTemplate.png`、`statusTemplate@2x.png` |    2 | [`711ad843ce09bffd70105816c7a04fdee8ee04a0`](https://github.com/yunhanhai/yunswitch/commit/711ad843ce09bffd70105816c7a04fdee8ee04a0) 添加托盘模板；未附外部素材地址。                                                                                                                                                                                                                                    |
| macOS `statusbar_template_3x.png`                   |    1 | [`f047960a338004100214aecff3ac11c8578e8625`](https://github.com/yunhanhai/yunswitch/commit/f047960a338004100214aecff3ac11c8578e8625) 使用 macOS 托盘模板；未附外部素材地址。                                                                                                                                                                                                                             |

早期 [`adee37ab660cba787d24b9c4a132ef12851e86b6`](https://github.com/yunhanhai/yunswitch/commit/adee37ab660cba787d24b9c4a132ef12851e86b6) 描述彩色放射状应用图标替换。它与上述提交共同记录上游图标演进，不把现有图标标为 YunSwitch 新设计。新设计或素材替换应作为独立改动，记录作者、来源及平台重新生成方法。

## 后续核查

本次新增了 Hermes 原文件与处理证据，及 SudoCode 历史恢复的摘要证据；其余 25 个供应商位图仍缺少足够的外部素材/许可记录。后续先追溯历史 PR 的附件、明确的品牌素材页或可固定的原文件，再记录处理过程和许可，不依靠外观相似判定。

本次只增加来源文档，未改图片、应用图标或公开安装包。当前已公开包的许可内容以对应[发布记录](releases/3.20.1-yunswitch.3.md)为准。
