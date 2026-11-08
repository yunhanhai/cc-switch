<div align="center">

# YunSwitch

Configuration and routing manager for AI coding tools.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Tauri 2](https://img.shields.io/badge/Tauri-2-orange.svg)](https://tauri.app/)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](#development)

English | [中文](README_ZH.md) | [日本語](README_JA.md) | [Deutsch](README_DE.md)

[Gitee](https://gitee.com/yunhanhai/yunswitch) · [GitHub](https://github.com/yunhanhai/yunswitch) · [Project notes](docs/PROJECT.md) · [Changelog](CHANGELOG.md)

</div>

## About this project

This is an open-source distribution **independently maintained by yunhanhai**, based on **v3.20.1** of [farion1231/cc-switch](https://github.com/farion1231/cc-switch). It adds routing groups, model aliases across providers, model catalog assistance, Claude → Codex provider configuration conversion, and request resilience improvements to the inherited configuration manager and local proxy.

The project currently has one maintainer. Future work will draw on useful ideas from other open-source projects as practical needs emerge. [Project notes](docs/PROJECT.md) document provenance, versioning and planned work. The upstream copyright and MIT terms remain in [LICENSE](LICENSE), with an additional notice for changes by yunhanhai.

Use the Gitee and GitHub repositories linked here for this distribution. The upstream website, packages, package-manager sources and sponsorship arrangements belong to the original project; releases and support for this distribution are maintained separately.

## Inherited capabilities and added work

The inherited application manages configurations for Claude Code, Claude Desktop, Codex, Gemini CLI, Grok Build, OpenCode, OpenClaw and Hermes Agent. It also provides provider switching, MCP, Prompts, Skills, session management, usage tracking, a local proxy and basic failover. See the [user manual](docs/user-manual/en/README.md) for client-specific capabilities and requirements.

The main additions relative to the v3.20.1 baseline are:

| Area                | Work in this distribution                                                                                                                                  | Details                                                |
| ------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------ |
| Routing groups      | Per-app provider groups, enable/disable controls, ordering, effective route previews and health indicators                                                 | [Routing groups](docs/routing-groups.en.md)            |
| Model aliases       | Map one logical model to provider-specific names and remap on failover; suggest aliases from existing mappings                                             | [Alias design](docs/model-alias-routing-design.md)     |
| Gemini routing      | Handle aliases in model URL paths, provider selection and the model actually sent upstream                                                                 | [Development record](docs/RELEASE-NOTES-P1-P6.md)      |
| Model catalogs      | Dynamic catalogs, caching and mapping assistance                                                                                                           | [User manual](docs/user-manual/en/README.md)           |
| Provider conversion | Convert third-party Claude providers into Codex configurations, preserve source models, prevent duplicates and probe direct-mode candidates                | [Conversion notes](docs/claude-to-codex-conversion.md) |
| Request resilience  | Observe 429 waits, cooldowns and circuit states; optionally retry the same provider before failover                                                        | [Development record](docs/RELEASE-NOTES-P1-P6.md)      |
| Maintenance         | Preserve unrelated configuration fields/comments, isolate development and test directories, check four locales and exercise routing through loopback tests | [Contributing](CONTRIBUTING.md)                        |

These additions build on inherited code. In particular, the Responses ↔ Anthropic bridge core comes from upstream; this distribution adds the conversion interface, configuration and model handling, guards and integration tests around it.

### Boundaries

- Model aliases are configured mappings. Missing mappings or an empty filtered result preserve a fallback route; aliases do not prove provider capability.
- Same-provider retries are off by default and capped at 10. They apply to connection failures and selected retryable statuses before a response is delivered. Delivered streams cannot be restarted; 429 and authentication failures follow other handling paths.
- Bridged Claude → Codex configurations require the local proxy. Direct-mode candidates are selected using the model catalog and source-model presence; actual Responses support still needs a real request to verify.
- Test totals in the historical development record describe checks on the previous machine. Validation for the current machine and release must be recorded separately.

## Preview

This existing repository screenshot illustrates provider configuration and local routing. The current code determines the exact layout.

![Provider and local routing example](docs/images/codex-kimi-routing/01-codex-providers-require-routing.png)

## Getting the distribution

The source version remains `3.20.1` to continue the existing code version. As checked on 2026-11-08, this repository has no GitHub Release. Build from source for now; independently published packages will be listed in this repository's release channel.

- [Gitee repository](https://gitee.com/yunhanhai/yunswitch)
- [GitHub Releases](https://github.com/yunhanhai/yunswitch/releases)
- [Version and release plan](docs/PROJECT.md)

The typical flow is to add a provider, enable local routing when the client requires it, configure groups and aliases, and inspect the resulting route and usage. Read the relevant guide before using a provider that requires protocol conversion.

Useful documentation:

- [User manual](docs/user-manual/en/README.md)
- [Routing documentation index](docs/routing-groups.index.md)
- [Claude → Codex configuration conversion](docs/claude-to-codex-conversion.md)
- [Kimi with Codex](docs/guides/codex-kimi-routing-guide-en.md)
- [Claude with Codex](docs/guides/codex-claude-routing-guide-en.md)
- [Codex with Claude Code](docs/guides/claude-codex-routing-guide-en.md)
- [Custom models in Codex Desktop](docs/guides/codex-desktop-custom-model-visibility-en.md)

Inherited installation instructions in older manuals and release notes are historical references. Follow this distribution's own release instructions when installing its packages.

## Development

| Tool          | Requirement                                                                      |
| ------------- | -------------------------------------------------------------------------------- |
| Node.js       | 22.12+; `.node-version` pins 22.12.0                                             |
| pnpm          | 10.12.3, pinned by `package.json`                                                |
| Rust          | Toolchain 1.95, rustfmt and clippy, pinned by `rust-toolchain.toml`              |
| Windows       | MSVC C++ Build Tools, Windows SDK and WebView2                                   |
| macOS / Linux | Corresponding [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/) |

```bash
git clone https://gitee.com/yunhanhai/yunswitch.git
cd yunswitch
corepack pnpm install --frozen-lockfile
corepack pnpm dev
```

You can also clone `https://github.com/yunhanhai/yunswitch.git`. When using Corepack, prefix the following pnpm commands with `corepack`.

| Command                                           | Purpose                                              |
| ------------------------------------------------- | ---------------------------------------------------- |
| `pnpm dev`                                        | Start Tauri desktop development                      |
| `pnpm dev:renderer`                               | Start the frontend; desktop APIs still require Tauri |
| `pnpm typecheck`                                  | Check TypeScript types                               |
| `pnpm test:unit`                                  | Run frontend unit tests                              |
| `pnpm build:renderer`                             | Build the frontend                                   |
| `pnpm build`                                      | Build the desktop application and packages           |
| `cargo test --manifest-path src-tauri/Cargo.toml` | Run backend tests                                    |

Development builds use `CC Switch Dev`, `~/.cc-switch-dev` and proxy port `15722` by default. Production branding and data migration will be planned with the independent release. See [CONTRIBUTING.md](CONTRIBUTING.md) for the workflow.

## Contributing and next steps

Use [Gitee Issues](https://gitee.com/yunhanhai/yunswitch/issues) or [GitHub Issues](https://github.com/yunhanhai/yunswitch/issues) for questions and suggestions. Read [CONTRIBUTING.md](CONTRIBUTING.md) before submitting code and [SECURITY.md](SECURITY.md) for security reports.

Planned work includes independent releases and branding, client/provider compatibility checks, and integrations with other projects that clearly record their source and license. See the [maintenance plan](docs/PROJECT.md).

## Attribution and license

Thanks to [the upstream CC Switch project](https://github.com/farion1231/cc-switch), original author Jason Young, and its contributors and dependencies.

This distribution remains under **MIT**, preserving the upstream copyright and permission notice and adding attribution for modifications and additions by yunhanhai. See [LICENSE](LICENSE) for the full terms.
