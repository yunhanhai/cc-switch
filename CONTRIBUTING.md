# Contributing / 贡献指南

YunSwitch is independently maintained by **yunhanhai**. Please submit issues and contributions to this project's [Gitee repository](https://gitee.com/yunhanhai/yunswitch) or [GitHub repository](https://github.com/yunhanhai/yunswitch). Read the [Code of Conduct](CODE_OF_CONDUCT.md) before participating.

本版本由 **yunhanhai** 独立维护。请向本项目的 [Gitee 仓库](https://gitee.com/yunhanhai/yunswitch)或 [GitHub 仓库](https://github.com/yunhanhai/yunswitch)提交问题与贡献，参与前请阅读[行为准则](CODE_OF_CONDUCT.md)。

## Development setup / 开发环境

- Node.js **22.12+**.
- pnpm **10.12.3**, pinned in `package.json`'s `packageManager` field / 版本以 `package.json` 的 `packageManager` 字段为准。
- Rust **1.95** with Cargo, rustfmt and Clippy, as specified in `rust-toolchain.toml` / 按 `rust-toolchain.toml` 安装工具链。
- [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/), including the native build tools for your OS / 安装对应操作系统的原生构建依赖。

```bash
git clone https://gitee.com/yunhanhai/yunswitch.git
cd yunswitch
corepack pnpm install --frozen-lockfile
corepack pnpm dev
```

`pnpm dev` starts the desktop app. For the browser renderer only, run `pnpm dev:renderer`.

`pnpm dev` 启动桌面应用；只调试浏览器前端时运行 `pnpm dev:renderer`。

Use `corepack pnpm` to select the pinned pnpm version when your system pnpm differs.

系统 pnpm 版本不同时，使用 `corepack pnpm` 选择仓库固定的版本。

| Command / 命令        | Purpose / 用途                           |
| --------------------- | ---------------------------------------- |
| `pnpm dev`            | Desktop development / 桌面开发           |
| `pnpm dev:renderer`   | Vite development server / 前端开发服务器 |
| `pnpm build`          | Desktop build / 桌面构建                 |
| `pnpm build:renderer` | Frontend build / 前端构建                |
| `pnpm typecheck`      | TypeScript checking / 类型检查           |
| `pnpm test:unit`      | Unit tests / 单元测试                    |
| `pnpm format`         | Format frontend source / 格式化前端源码  |
| `pnpm format:check`   | Check frontend formatting / 检查前端格式 |

Run Rust checks from `src-tauri`:

在 `src-tauri` 目录运行 Rust 检查：

```bash
cd src-tauri
cargo fmt --check
cargo clippy
cargo test --locked -j 1
```

## Issues and pull requests / 问题与贡献

1. Search this project's existing issues before opening another. For bugs, include the version or commit, OS, reproduction steps, expected behavior and actual behavior. Redact API keys, tokens and personal data from logs and screenshots. See [SECURITY.md](SECURITY.md) for vulnerabilities.
2. Discuss substantial features before implementation. Small fixes and documentation improvements can be submitted directly.
3. Create a focused branch from **main** and target `main` in your PR. The former `feature/v3.20.1-custom-routing` branch is retained for history.
4. Explain the change and the checks you ran. For behavior changes, add or update relevant tests where useful; state any platforms or checks you could not verify. Documentation-only changes need checks for formatting, links and factual accuracy; frontend and Rust tests are unnecessary when their sources and behavior are unchanged.
5. Preserve upstream copyright and license notices. Record third-party code or assets using the [source ledger](docs/PROVENANCE.md): project URL, version or commit, license, affected files and your adaptations.

对应的中文流程：

1. 提交前搜索本项目已有 Issue。Bug 报告请包含版本或提交、操作系统、复现步骤、预期及实际行为；日志和截图须隐藏 API Key、令牌与个人信息。漏洞报告参见 [SECURITY.md](SECURITY.md)。
2. 较大功能先讨论方案；小修复和文档改进可直接提交。
3. 从 **main** 创建专注于单一改动的分支，并将 PR 指向 `main`。旧 `feature/v3.20.1-custom-routing` 分支保留用于追溯。
4. 说明改动及已运行的检查。行为改动按需要补充相关测试；明确未验证的平台或检查。纯文档改动检查格式、链接与事实准确性；前端和 Rust 源码及行为未变时，无需运行其测试。
5. 保留上游版权与许可证声明，按[来源台账](docs/PROVENANCE.md)记录新引入的第三方代码或资源：项目地址、版本或提交、许可证、涉及文件及修改方式。

Use [Conventional Commits](https://www.conventionalcommits.org/) where practical, for example `fix(proxy): correct model routing` or `docs(readme): clarify project origin`.

提交信息建议采用 [Conventional Commits](https://www.conventionalcommits.org/)，例如 `fix(proxy): correct model routing` 或 `docs(readme): clarify project origin`。

## Provider presets / 供应商预设

Follow the [provider catalog guide](docs/PROVIDER-CATALOG.md) when adding or changing presets. Preserve existing identifiers and authentication compatibility, use neutral website links, and record the client, model, protocol and routing mode you verified. Distinguish mock checks from real provider requests; preset configuration alone does not establish compatibility.

新增或修改预设请遵循[供应商目录维护指南](docs/PROVIDER-CATALOG.md)。保留已有标识与认证兼容性，使用中性网站链接，并说明已验证的客户端、模型、协议和路由方式。区分模拟检查与真实供应商请求；预设配置本身不等于兼容性验证。

## Style and translations / 代码风格与翻译

Frontend code uses Prettier and strict TypeScript; Rust code uses rustfmt and Clippy. Follow the surrounding code and existing IPC conventions.

前端使用 Prettier 和严格 TypeScript，Rust 使用 rustfmt 与 Clippy。遵循附近代码及已有 IPC 约定。

Use i18next's `t()` for user-facing text and update the relevant entries in all four locale files:

用户可见文本使用 i18next 的 `t()`，同时更新四个语言文件中的相关词条：

- `src/i18n/locales/en.json`
- `src/i18n/locales/zh.json`
- `src/i18n/locales/zh-TW.json`
- `src/i18n/locales/ja.json`

AI-assisted contributions are welcome. The contributor remains responsible for understanding, reviewing and verifying the submitted work, and for explaining its limitations.

欢迎使用 AI 辅助贡献。贡献者仍须理解、审查和验证所提交的内容，并说明验证限制。

For help, see [SUPPORT.md](SUPPORT.md).

获取帮助请参阅 [SUPPORT.md](SUPPORT.md)。
