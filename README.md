# RiverLens

**Your hands. Your data. A local workspace for post-session poker review.**

[English](README.md) · [繁體中文](README.zh-TW.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Português (Brasil)](README.pt-BR.md) · [Italiano](README.it.md) · [Nederlands](README.nl.md) · [Polski](README.pl.md) · [Türkçe](README.tr.md)

RiverLens is a Tauri desktop application for reviewing your own completed Natural8 / GGPoker cash-game hand histories. React and TypeScript power the interface; a Rust core handles parsing, statistics and local SQLite storage.

> **[Download v0.3.0](https://github.com/G-Loop-co/riverlens/releases/tag/v0.3.0)** — AI coach, local MCP agent connection and offline study. Windows, Apple Silicon and Intel Mac packages. [Release notes](docs/release-v0.3.0.md).

![RiverLens overview in English — synthetic data](docs/screenshots/overview-en.jpg)

README translations: **13 languages**. App interface: **English, Traditional Chinese and Simplified Chinese**.

**v0.3.0** adds AI-assisted review alongside Spot Explorer → Leak Finder → Preflop reference → Trainer. User-supplied strategy packs are required for strategy comparisons; frequencies are not decision EV. [Study workflow](docs/study-workflow.md) · [Themes](docs/theme-selection.md).

## Features

| Feature | What you can do |
| --- | --- |
| AI coach and agent connection | Explore statistics and decision context through 17 local MCP tools; create evidence-linked review, practice and study-plan drafts for your approval. |
| Import center | Import TXT, ZIP or folders; monitor progress, pause/resume, detect duplicates and inspect quarantined records. |
| Results dashboard | Explore net results, bb/100, session and position breakdowns; drill into underlying hands. |
| 13 core statistics | Review VPIP, PFR, RFI, 3-bet, blind defense and postflop/showdown frequencies with explicit opportunity denominators. |
| 13 × 13 starting-hand matrix | Inspect observed hand counts, net bb, bb/100 and action frequency; open the matching hand list. |
| Hand replayer | Step through actions, jump streets and replay known cards. |
| Review workspace | Save notes, tags, reviewed status and reusable filters. |
| Data ownership | Keep everything in local SQLite; export CSV / hand histories and back up or restore your database. |
| Appearance themes | Choose Forest, Midnight or Paper (warm white); the selection persists locally. |
| Three interface languages | Switch instantly between English, Traditional Chinese and Simplified Chinese, including offline use. |
| Eligible all-in equity | Calculate supported heads-up, single-pot, known-hole-card, single-runout situations. This is not decision EV or a GTO score. |

![Observed starting hands — synthetic data](docs/screenshots/starting-hands-en.jpg)

Screenshots capture the complete page at **1920px desktop width**, using the **Paper (warm white)** theme. A [1920 × 1080 overview](docs/screenshots/overview-desktop-en.jpg) is also available.

Screenshots use **240 generated hands**, not private player data. The deliberately repetitive fixture is for demonstrating the interface, not realistic frequencies or strategy advice. The matrix describes observed hands, not a recommended range.

## AI: features and quick start

Ask about your statistics, inspect a specific decision, find spots worth reviewing, and create practice or study-plan drafts linked to hand evidence. Drafts require your approval; AI and offline study libraries currently remain separate.

**Use an external agent — no provider API key in RiverLens:**

1. Import completed hands and keep RiverLens open.
2. Open **AI Coach → AI Connections → Enable local connection**. Copy the displayed configuration into a client supporting **stdio MCP**, such as Codex.
3. Ask: “Review my biggest leak candidates, show sample sizes and supporting hands, then create a practice draft.” Review and accept drafts in **Learning Library**.

Your agent may need its own login/subscription. Update the connection configuration after restarting RiverLens; **Revoke connection** disables access.

**Use the in-app coach — API key required:** save an OpenAI, Anthropic or Gemini key in **AI Connections**, then choose a compatible model and sharing scope in **AI Coach**. Ask a question, or use **Ask AI** from the hand replayer.

Selected context can be sent to the model provider, including through an external agent. Keys stay in the OS credential store. Live provider calls remain unverified; offline study works without a key. [Full setup and limitations](docs/ai-agent-coach.md).

## Quick start

Requires Node.js 22+, Rust stable and the [Tauri platform prerequisites](https://v2.tauri.app/start/prerequisites/) (Xcode Command Line Tools on macOS; MSVC C++ Build Tools and WebView2 on Windows).

```sh
git clone https://github.com/G-Loop-co/riverlens.git
cd riverlens
npm ci
npm run desktop
```

1. Export your own completed hand histories from PokerCraft as TXT or ZIP.
2. In **Data & settings**, confirm the brand, Hero name and the time zone used inside the hand-history text.
3. Import files in **Import center**.
4. Review results, inspect the starting-hand matrix and replay individual hands.
5. Save notes and back up your database.

## Development

```sh
# Terminal 1: local Rust HTTP core, bound to 127.0.0.1
npm run serve:core
# Terminal 2: browser preview at http://127.0.0.1:1420
npm run dev
```

The desktop app uses Tauri IPC and native file dialogs. The browser preview uses the same Rust core with development-only file path entry.

```sh
npm test
npm run core:test
npm run build
node scripts/cargo.mjs clippy -p poker-core --all-targets -- -D warnings
npm run desktop:build -- --bundles app
# On a Windows native runner:
npm run desktop:build -- --target x86_64-pc-windows-msvc --bundles nsis
```

Data lives in the Tauri app-data directory (`app.riverlens.desktop`); the settings page shows the actual location. Browser development uses `.local/riverlens.db`. Use the built-in backup operation for an active database so SQLite WAL data is handled correctly.

## Scope and release status

RiverLens is for offline, personal post-session review. It does not connect to the game client and provides no live HUD, real-time assistance, population mining, cloud sync or GTO best-action scoring. It is not affiliated with Natural8 or GGPoker.

The v0.3.0 release includes AI/MCP integration and the offline study workflow. Private hand histories, local databases and strategy packs are excluded. See the [release notes](docs/release-v0.3.0.md) for validation limits.

- [User guide (Traditional Chinese)](docs/user-guide.md)
- [Architecture and statistical definitions](docs/architecture.md)
- [Research and existing components](docs/research-matrix.md)
- [Validation and limitations](docs/validation.md)
- [Changelog](CHANGELOG.md)
- [Third-party components](THIRD_PARTY.md)
- [Releases](https://github.com/G-Loop-co/riverlens/releases)

## Licensing

The repository is public, but a project-wide reuse license has not yet been selected. Do not assume MIT or Apache licensing for RiverLens itself. Third-party components retain their respective licenses; see [THIRD_PARTY.md](THIRD_PARTY.md).
