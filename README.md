# RiverLens

**Your hands. Your data. A local workspace for post-session poker review.**

[English](README.md) · [繁體中文](README.zh-TW.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Português (Brasil)](README.pt-BR.md) · [Italiano](README.it.md) · [Nederlands](README.nl.md) · [Polski](README.pl.md) · [Türkçe](README.tr.md)

RiverLens is a Tauri desktop application for reviewing your own completed Natural8 / GGPoker cash-game hand histories. React and TypeScript power the interface; a Rust core handles parsing, statistics and local SQLite storage.

> **v0.1.0 public preview — source release.** Build locally using the instructions below. No signed installers are attached. Windows and Intel runtime acceptance are not complete. See [validation](docs/validation.md).

![RiverLens overview in English — synthetic data](docs/screenshots/overview-en.jpg)

README translations: **13 languages**. App interface: **English, Traditional Chinese and Simplified Chinese**. Current `main` adds appearance themes; the `v0.1.0` release tag remains the original source snapshot.

## Features

| Feature | What you can do |
| --- | --- |
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

This release publishes a clean source snapshot. Private hand histories, local databases and historical private acceptance artifacts are excluded. The original v0.1.0 tag does not include appearance themes; current main includes the reviewed theme implementation. Work on other local branches remains excluded.

- [User guide (Traditional Chinese)](docs/user-guide.md)
- [Architecture and statistical definitions](docs/architecture.md)
- [Research and existing components](docs/research-matrix.md)
- [Validation and limitations](docs/validation.md)
- [Changelog](CHANGELOG.md)
- [Third-party components](THIRD_PARTY.md)
- [Releases](https://github.com/G-Loop-co/riverlens/releases)

## Licensing

The repository is public, but a project-wide reuse license has not yet been selected. Do not assume MIT or Apache licensing for RiverLens itself. Third-party components retain their respective licenses; see [THIRD_PARTY.md](THIRD_PARTY.md).
