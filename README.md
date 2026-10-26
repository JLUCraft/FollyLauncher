<img src="docs/figs/banner.png" alt="Folly Launcher" />

> **Folly Launcher** is the official Minecraft launcher of JLUCraft, the Minecraft club of Jilin University. It is built for club members: accounts use the club's skin station ([skin.jlucraft.com](https://skin.jlucraft.com), registered with a school email), interconnecting across Jilin universities through the JLMUA provincial union and, beyond the province, following the national MUA conventions.
>
> This project is based on [SJMCL](https://github.com/UNIkeEN/SJMCL) (SJTU Minecraft Club) and is licensed under GPLv3 with additional terms in [`LICENSE.EXTRA`](./LICENSE.EXTRA). For the federation and branding architecture, see the [technical documentation](https://www.jlucraft.com/tech/clients).

**English** · [简体中文](docs/README.zh-Hans.md) · [繁體中文](docs/README.zh-Hant.md)

## Features

Folly Launcher keeps the essentials of a modern launcher and adds capabilities built for the club and its federation:

* **Union Federation**: Connects to the club's authority node and binds identity to devices, so members interoperate across the Jilin provincial union (JLMUA) and the national MUA conventions.
* **Member Rooms**: Server and room listings for club members, with per-room network settings.
* **Seasons and Competitions**: Sign-up, teams, scoring, points and AI-request settlement for club leagues and events.
* **Unified Identity**: A skin-station account plus device identity, using Yggdrasil third-party authentication.
* **Multiple Directories and Instances**: Manages multiple game directories and instances, keeping every instance's resources (saves, mods, resource packs, shaders, screenshots, etc.) and settings in one place.
* **Resource Download**: Downloads game clients, mod loaders, various game resources and modpacks from CurseForge and Modrinth.
* **Multi-Account Support**: Built-in support for multiple accounts and third-party authentication servers, following the OAuth login flow of the Yggdrasil Connect proposal.
* **Extensions and MCP**: Extend the launcher through its extension system, and collaborate with external webpages, applications and agent services through deeplinks and MCP services.

> Note: some features may be limited by region, platform, or bundle type.

### Built with

[![Tauri](https://img.shields.io/badge/Tauri-v2-FFC131?style=for-the-badge&logo=tauri&logoColor=white&labelColor=24C8DB)](https://tauri.app/)
[![Next JS](https://img.shields.io/badge/next.js-000000?style=for-the-badge&logo=nextdotjs&logoColor=white)](https://nextjs.org/)
[![Chakra UI](https://img.shields.io/badge/chakra_ui-v2-38B2AC?style=for-the-badge&logo=chakraui&logoColor=white&labelColor=319795)](https://v2.chakra-ui.com/)

## Getting Started

Download the latest release from [GitHub Releases](https://github.com/JLUCraft/FollyLauncher/releases).

Folly Launcher currently supports the following platforms:

| Platform  | Versions            | Architectures              | Provided Bundles                        |
|-----------|---------------------|----------------------------|-----------------------------------------|
| Windows   | 10 and above       | `aarch64`, `i686`, `x86_64`| installer `.exe`, portable `.exe`                 |
| macOS     | 10.15 and above     | `aarch64`, `x86_64`        | `.app`, `.dmg`                          |
| Linux     | webkit2gtk 4.1 (e.g., Ubuntu 22.04) | `aarch64`, `x86_64`   | `.deb`, `.rpm`, portable binary |

To learn how to use the launcher and browse frequently asked questions, please refer to the [Player Documentation](https://www.jlucraft.com/docs/).

## Development and Contributing

Clone the repository and install the front-end dependencies:

```bash
git clone git@github.com:JLUCraft/FollyLauncher.git
bun install
```

Run the project in development mode:

```bash
bun run tauri dev
```

Before committing, make sure the standard checks pass:

```bash
eslint src --max-warnings=0
tsc --noEmit
node scripts/locale/index.js diff en
```

Branding is data-driven: it is defined by `brands/<id>/brand.json` and applied with `bun run brand <id>`. Adding a school means adding data, not code.

We welcome contributions and bug reports through [GitHub Issues](https://github.com/JLUCraft/FollyLauncher/issues). More developer notes are available in the [technical documentation](https://www.jlucraft.com/tech/clients).

## Developer environment and CLI

Use Bun, Node.js, Rust, and the Tauri platform dependencies declared by the repository configuration. Configure the variables documented in `.env.template` in your local environment; keep credentials outside source control. `bun run tauri build` creates application bundles in the existing build directories.

The CLI connects to the running GUI through its MCP server. Build it from the repository root with `cargo build --release --manifest-path cli/Cargo.toml`.

Project instructions live in [AGENTS.md](AGENTS.md). Release history is available in [CHANGELOG.md](CHANGELOG.md) and [GitHub Releases](https://github.com/JLUCraft/FollyLauncher/releases). The inherited [contribution guide](CONTRIBUTING.md), [code of conduct](CODE_OF_CONDUCT.md), and [security policy](SECURITY.md) are retained. Use Conventional Commits and include the problem, resulting behavior, and validation evidence in pull requests. Select one validation channel for each batch.

## Security reports

Report vulnerabilities privately rather than opening a public issue. For upstream SJMCL vulnerabilities, email [launcher@sjmc.club](mailto:launcher@sjmc.club) with the subject prefix `[SJMCL Security]` and reproduction details.

## Upstream and Acknowledgments

This project is based on [SJMCL](https://github.com/UNIkeEN/SJMCL), developed by the SJTU Minecraft Club and its contributors. We sincerely thank the SJMCL Team for the upstream launcher, extension system and documentation that make this fork possible.

## Copyright

Copyright © 2026 Folly Launcher Team. Based on SJMCL © 2024-2026 SJMCL Team.

> NOT AN OFFICIAL MINECRAFT SERVICE. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.

The software is distributed under [GNU General Public License v3.0](/LICENSE).

By GPLv3 License term 7, when you distribute a modified version of the software, you must obey GPLv3 as well as the following [additional terms](/LICENSE.EXTRA):

1. Use a different software name than SJMCL or SJMC Launcher;
2. Mark clearly in your repository README file, your distribution website or thread, support documents, and the About page in the software that your program is based on SJMCL, and give out the URL of the origin repository.
3. When your modifications to this software are limited solely to **adding** (without modifying or deleting) preset authentication servers (`src-tauri/src/account/helpers/authlib_injector/constants.rs`), the restriction set forth in Clause 1 above shall not apply. In this case, you may continue to compile and distribute the software under its original name.

In addition, per the terms of use of the upstream website, when distributing a modified version of the software, please report version numbers with a prefix (e.g. `folly-1.3.1`) to the statistics server (`src-tauri/src/utils/sys_info.rs`), unless your modifications meet Clause 3 above.

## Contact Us

- QQ Group: 361033103
- Issues: https://github.com/JLUCraft/FollyLauncher/issues
- Website: https://www.jlucraft.com/
