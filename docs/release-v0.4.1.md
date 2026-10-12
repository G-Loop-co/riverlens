# RiverLens v0.4.1 — Multi-room Cash / MTT hand histories

Import English NLHE personal hand histories across 15 native Cash format families and seven tournament families. README support tables distinguish native, converted-only, historical and unimplemented formats; the catalog contains 94 labels, not 94 native parsers.

## Changes

- Tournament grammars: GG / Natural8, PokerStars, modern WPN, Winamax, 888, iPoker XML and archived Full Tilt. Preserve tournament ID, level and source buy-in text when available; filter by tournament ID.
- TXT, XML, ZIP and folder imports retain source text, ledger reconciliation, deduplication, quarantine, rebuild and export. Windows CRLF originals are preserved; LF / CRLF equivalents remain duplicates.
- Keep tournament CHIPS and USD / EUR / GBP / CAD / USDT separate. All-in chip EV is not prize money, ROI, ICM or bounty EV.
- Converted-only Chico, club apps and WPT Global require existing complete PokerStars Cash exports. RiverLens does not include their native capture or converter.
- Preserve v0.4.0 MDA / AI. Personal-format expansion does not widen MDA beyond GG Regular USD NLH 6-max. Dataset cloud / MCP sharing remains off by default.

## Validation and limits

- Feature-head regression: 107 Rust tests, 45 frontend tests, formatting, strict Clippy, frontend build and three-platform CI passed at `64712efeda8bc4eca31eaf0d67dd518178076f5c`; merged main has the same source tree.
- This release changes version metadata and documentation. Exact-tag CI must pass Apple Silicon, Intel Mac and Windows packaging before publication. Native-target packaged MCP smoke verifies protocol `2025-11-25`, 23 tools and disabled-access denial.
- Earlier native GUI proof used a v0.3.2-based test app: 13 public Cash samples, seven tournament grammars and preserved own Cash / Rush histories. Four incompatible tournament hands remain rejected. This is not native GUI acceptance of integrated v0.4.1 or current poker-room clients.
- CoinPoker native Cash and modern WPN Cash still lack documented native real-sample proof. Archived names do not imply current operation. HHDealer samples, live provider calls, external-client MDA analysis and Windows / Intel native GUI acceptance remain unverified.
- MDA is offline population research, not solver / GTO / decision-EV or platform permission. Existing [MDA restrictions](mda-population.md) apply.

## Downloads and updates

Apple Silicon and Intel macOS `.app.zip`, Windows x64 NSIS installer, `build-manifest.json` and `SHA256SUMS.txt` accompany the release. Verify downloaded assets against the manifest and checksums.

macOS packages are **ad-hoc signed, not Developer ID notarized**. Archive extraction is verified with deep / strict codesign checks and executable permissions. Intel cross-compilation proves packaging, not Intel native runtime acceptance.

No in-app updater is included. Back up active stores before replacing an installed application; retain the app-data directory. This publication does not replace the user's local app or restore / overwrite their database. Private histories, local databases, strategy packs and research corpora are excluded.

[Support table](../README.md#personal-review-hand-history-formats-v041) · [Format evidence](hand-history-formats.md) · [Traditional Chinese guide](../README.zh-TW.md) · [Changelog](../CHANGELOG.md)
