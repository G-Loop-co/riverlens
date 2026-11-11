# RiverLens v0.6.1 — Structured six-workspace UX

v0.6.1 reorganises the app around six persistent workspaces so review, learning, research, coaching and data control are one click apart. Desktop routes, workspace tabs and replay state are now browser-backed, so refresh and back/forward navigation preserve context without changing local data storage.

## Changes

- **Six workspaces:** Today, Hands, Learn, Research (MDA), AI Coach and Data replace nine flat pages and scattered child tabs.
- **Deep links and restore:** React Router `HashRouter` supports desktop refresh, back/forward navigation and shareable workspace/tab routes. Personal filters, dataset, pack, node, workspace tab and replay step are represented in the URL; AI prompts, raw source text and private review notes stay out of URLs.
- **Route-backed replay:** hands open in `/hands/replay/:handId` with replay step and return context preserved. Closing replay restores the originating hand list, filters and return path.
- **Shared UI foundation:** a common app shell, workspace header, route tabs, scope bar, status pills and empty states replace duplicated navigation and make every workspace predictable.
- **Contextual actions:** hand-import actions appear only where import is the next likely step; “Ask AI” carries the current workspace context without duplicating the global coach entry.
- **Accessibility and resilience:** tabs follow the WAI-ARIA tabs pattern with keyboard movement, localized status text is announced with `aria-live`, and route boundaries replace full-page crashes with recoverable workspace errors.
- **Visual system refresh:** font, control, spacing, radius and status-color tokens are consistent across Forest, Midnight and Paper themes without replacing the UI framework.
- **UX refinements:** Today highlights next review/import/study actions; Learn exposes the spot-to-practice loop; Research leads with evidence instead of setup; AI Coach separates the prompt from connection setup; Data Center separates import, sources and preferences.

## Validation and limits

- Local checks at the release candidate: `npm run build` passes and the frontend suite passes 62 tests across 14 files.
- Desktop CI on release-candidate commit `d08e3443627e8871d40ce075a81a6c908e66bc37` passed macOS Apple Silicon, macOS Intel and Windows x64. The run also produced verified app/installer artifacts and the release job created a reviewable draft from this tag.
- Desktop smoke validation used a disposable isolated data directory and synthetic GG/Natural8 fixture: all six workspace routes loaded; an import scanned and added 6 hands with 0 quarantined; hand review, route-backed replay, replay step navigation and close-to-list restore were exercised. The smoke data, fixture files and temporary app copy were removed afterward.
- This change is frontend information architecture and presentation only. Rust engine logic, Tauri RPC schema, SQLite schema, DuckDB datasets, statistics definitions and import/backup behavior are unchanged.
- Existing local SQLite stores and MDA datasets remain compatible. Back up before replacing an installed application; RiverLens never replaces a store or restores a database as part of an app update.

## Downloads and updates

Apple Silicon and Intel macOS `.app.zip`, Windows x64 NSIS installer, `build-manifest.json` and `SHA256SUMS.txt` accompany the release. Verify downloaded assets against the manifest and checksums.

macOS packages are **ad-hoc signed, not Developer ID notarized**. Archive extraction is verified with deep / strict codesign checks and executable permissions. Intel cross-compilation proves packaging, not Intel native runtime acceptance.

No in-app updater is included. Back up active stores before replacing an installed application; retain the app-data directory. This publication does not replace the user's local app or restore / overwrite their database. Private histories, local databases, strategy packs and research corpora are excluded.

[Study workflow](study-workflow.md) · [MDA scope and restrictions](mda-population.md) · [User guide (Traditional Chinese)](../docs/user-guide.md) · [Changelog](../CHANGELOG.md)
