# 内联图标与运行时资源核查

2026-11-11，以源码提交 `d35e65fefaf5360bee998a00ed17b5748a48355d` 核查 [图标索引](../src/icons/extracted/index.ts)的实际值。该索引的 Git blob 为 `0e459c9e4924f15ff533b294fb8ebb31d9d9bfba`，与 CC Switch 基线 `3217f72596f2d1c0f879f0a05f83803825d9809f` 相同。本次只补来源记录，未修改图标或展示代码。

[ProviderIcon.tsx](../src/components/ProviderIcon.tsx)会从 `icons` 读取内联 SVG，或从 `iconUrls` 读取导入资源。独立 `.svg` 文件与索引中的字符串可能不同，因此[独立文件核查](ICON-SOURCES.md)不能直接替代运行时索引核查。

## 核查方法与数量

使用 TypeScript AST 读取两个对象的属性，按解码后的字符串值计算 UTF-8 SHA-256，与固定 `@lobehub/icons-static-svg@1.73.0` 包及本地独立 SVG 比较。包来源、完整性和许可快照见[图标来源记录](ICON-SOURCES.md#lobehub-svg-集合)。

| 范围                                | 数量 | 本次结论                                       |
| ----------------------------------- | ---: | ---------------------------------------------- |
| `icons` 内联 SVG                    |   73 | 全部为字符串或无插值模板字面量                 |
| `iconUrls` 导入引用                 |   29 | 26 个位图、3 个 SVG；引用文件均存在            |
| 注册名称合计                        |  102 | 两个对象之间无重名                             |
| 内联值与固定 LobeHub 包逐字节相同   |   50 | 仅确认下表中的值                               |
| 内联值与某个本地独立 SVG 逐字节相同 |   52 | 含 3 个颜色文件名称映射                        |
| 仅文件外围空白不同                  |    2 | Bailian、PPIO 的独立文件含尾部换行，内联值没有 |
| 未匹配任何本地独立 SVG 文本         |   19 | 包括修改后的内联变体及只存在于索引中的值       |

上述 52、2、19 项合计 73。只去除文件外围空白的比较用于描述差异，不作为“逐字节相同”的结论；没有通过图形外观相似推断来源。

## 50 个内联值的固定包匹配

| 索引名称                                                                                                                                                                                                                                                                                                                                                                          | LobeHub 包文件           |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------ |
| `anthropic`、`github`、`githubcopilot`、`grok`、`midjourney`、`notion`、`ollama`、`openai`、`openrouter`、`vercel`、`xai`、`zeroone`、`mcp`、`bailian`                                                                                                                                                                                                                            | 同名 `.svg`              |
| `alibaba`、`aws`、`azure`、`baidu`、`bytedance`、`chatglm`、`claude`、`cloudflare`、`cohere`、`copilot`、`deepseek`、`doubao`、`gemini`、`gemma`、`google`、`googlecloud`、`huawei`、`huggingface`、`hunyuan`、`meta`、`minimax`、`mistral`、`palm`、`perplexity`、`qwen`、`stability`、`tencent`、`wenxin`、`yi`、`zhipu`、`nvidia`、`longcat`、`modelscope`、`aihubmix`、`ppio` | 基础名称加 `-color.svg`  |
| `siliconflow`                                                                                                                                                                                                                                                                                                                                                                     | `siliconcloud-color.svg` |

`longcat`、`modelscope`、`aihubmix` 的本地文件分别使用 `longcat-color.svg`、`modelscope-color.svg`、`aihubmix-color.svg`，属于名称映射，并非缺少独立资源。

这 50 个内联值与固定包的匹配，是独立于“49 个独立 SVG 文件完全一致”的另一项结果。主要差别是 Kimi 的索引已改写，而 Bailian 与 PPIO 的内联文本与包相同、独立文件多一个尾部换行。LobeHub 许可副本见 [LobeHub-Icons-MIT.txt](../licenses/LobeHub-Icons-MIT.txt)；匹配集合不证明其他资源的许可或品牌背书。

## 需要分别追溯的变体

12 个内联值与同名独立文件内容不同：`aicodemirror`、`aicoding`、`crazyrouter`、`cubence`、`kimi`、`rc`、`lioncc`、`micu`、`ucloud`、`sssaicode`、`stepfun`、`novita`。本次确认存在差异，不将格式优化或路径改写后的值自动归入固定包匹配集合。

另有 7 个值没有匹配任何独立 SVG：`aigocode`、`amux`、`zenmux`、`openclaw`、`pi`、`opencode`、`jiekou`。其中 Amux 的独立文件名为 `amuxapi-icon.svg`，OpenClaw 使用 `claw.svg`，OpenCode 有 `opencode-logo-light.svg`，但这些文件与对应内联文本并不相同。它们均需要按实际使用值追溯，不能仅按名称关联便认定来源。

| 重点条目 | 可核实的历史记录                                                                                                                                                                                                                                        | 当前边界                                                                      |
| -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| Kimi     | 上游 [`26be9324dd529c792f5e7541044ce143e886ae2b`](https://github.com/farion1231/cc-switch/commit/26be9324dd529c792f5e7541044ce143e886ae2b)（2026-06-22）修改索引，提交说明为刷新 Kimi 标志并使用 `currentColor` 适配主题                                | 独立 `kimi.svg` 仍匹配固定包颜色版；运行时内联值已不同，原始素材地址尚未确认  |
| Pi       | 上游 [`84e75ad24a31ef5c1693e655459c1a7527a31c17`](https://github.com/farion1231/cc-switch/commit/84e75ad24a31ef5c1693e655459c1a7527a31c17)（2026-08-14）加入内联值与 [piIcon.test.ts](../tests/config/piIcon.test.ts)，测试说明为“official README mark” | 没有独立 `pi.svg`；已有测试是来源线索，尚未给出固定原始 README 版本或素材许可 |
| Bailian  | 上游 [`5502c74a793f3eed3211334f8d8b1b872cc54243`](https://github.com/farion1231/cc-switch/commit/5502c74a793f3eed3211334f8d8b1b872cc54243)（2026-02-08）加入对应索引项                                                                                  | 内联值与固定包 `bailian.svg` 完全一致；独立文件仅外围换行不同                 |
| PPIO     | 上游 [`3711e1a0463c8f15df237d69a819b459f21adc63`](https://github.com/farion1231/cc-switch/commit/3711e1a0463c8f15df237d69a819b459f21adc63)（2026-08-11）加入对应索引项                                                                                  | 内联值与固定包 `ppio-color.svg` 完全一致；独立文件仅外围换行不同              |

Kimi 内联值 SHA-256 为 `40faf32d978f249cd4a92a1f21394bd28daa58782dc8bf38a66b049887c144d9`；Pi 为 `b51648fa48c2684537d58bd394d395bdca6eab5873e7a9574e2a364c9838fb16`。这两个摘要用于标识本次实际检查的内容，不表示已经确认其外部许可。

## URL 资源与后续维护

三个通过 URL 导入的 SVG 是 `ccsub.svg`、`shengsuanyun.svg`、`subrouter.svg`，其余 26 个导入为位图。这里确认了索引映射和文件存在，不将运行时 URL 映射当作新的来源证据；外部来源和许可仍按[图标来源记录](ICON-SOURCES.md)逐项核查。

后续更改图标时，应同时核对独立文件、内联索引值和 `ProviderIcon` 的使用路径；记录修改的是哪个实际资源。新的来源证据纳入[来源台账](PROVENANCE.md)，不会因本次文档更新而更换已公开安装包中的资源。
