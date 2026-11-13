# RiverLens v0.6.2 — Settings, BB review and safer desktop delivery

v0.6.2 separates application preferences from data management, adds BB-first hand review, starts replay after the forced opening posts, and ships the measured desktop build improvements without the CGU16 GUI regression.

## Changes

- **Dedicated Settings workspace:** theme and language preferences are no longer mixed into Data Center. Data Center now contains Import and Data sources only; saved `/data/preferences` links redirect to `/settings/preferences`.
- **Show BB switch:** Hand review can display net results in big blinds while preserving the existing currency columns when the switch is off. The choice is stored locally.
- **Blind-aware replay start:** replay opens after the opening SB, BB, ante, straddle and dead-blind posts. Explicit replay-step links still open at the requested action.
- **Correct shell metadata:** the desktop footer reads the installed Tauri bundle version instead of the engine protocol version, and the English sidebar includes “Study workspace”.
- **Safer delivery:** lazy-loaded workspaces and Latin-only Geist fonts reduce startup payload. The release keeps CGU1 after CGU16 reproduced a GUI window-disappearance regression.

## Validation and limits

- Frontend suite: 65 tests passed across 14 files; production build and Prettier checks passed.
- Desktop CI run 43426938571 passed macOS Apple Silicon, macOS Intel and Windows x64, including Rust tests, Clippy, packaged MCP smoke and signed bundle/installer creation.
- Local Apple Silicon runtime verification kept the CGU1 desktop window alive after launch; the compared CGU16 build lost its window.
- Existing SQLite and DuckDB data remain compatible. No database migration is required.

## Downloads and updates

Apple Silicon and Intel macOS `.app.zip`, the Windows x64 NSIS installer, `build-manifest.json` and `SHA256SUMS.txt` accompany the release. Verify every download against the manifest and checksums.

macOS packages remain **ad-hoc signed, not Developer ID notarized**. Intel cross-compilation proves packaging, not native Intel runtime acceptance.

No in-app updater is included. Preserve the app-data directory and back up active stores before replacing an installed application. Private hand histories, notes, databases, strategy packs and research corpora are excluded.

[Architecture](architecture.md) · [User guide](user-guide.md) · [Changelog](../CHANGELOG.md)
