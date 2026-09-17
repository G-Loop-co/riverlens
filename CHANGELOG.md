# Changelog

## 0.3.2 — 2026-09-17

- AI learning reports: content-based categories, tags, lossless topic sections and reclassification of existing reports.
- Fix saved hand evidence crashing the UI; retain original tool/version/arguments and replay links.
- Seal macOS bundles and verify signatures after ZIP extraction; add tag-driven release drafts, source manifests and checksums.
- macOS downloads remain ad-hoc signed unless Developer ID/notarization secrets are configured.


## v0.3.1

- Add DeepSeek and OpenCode Go providers, with model-specific API routing and correct tool continuation.
- List all five AI providers, external MCP access and Go model IDs in the English and Traditional Chinese READMEs.
- See [release notes](docs/release-v0.3.1.md) for setup and validation limits.

## v0.2.0

- Default new installations and invalid preferences to Paper (white); retain saved theme selections and use a light startup background.
- Correct study-model trust, propagate ancestor weight issues on import and reload, and keep unverified historical comparisons reference-only.
- Correct IP/OOP C-bet and open-versus-3-bet presets; show observation-only leak data without invented targets.
- Add paginated strategy decision tracing and training from marked-hand replay; preserve annotations and restart progress.
- Add offline Spot Explorer, custom-target Leak Finder, source-backed preflop comparison and spaced-repetition Trainer.
- Add Forest, Midnight and Paper themes with persistent selection across the full workspace.
- Upgrade local storage to schema 3 with a safety backup before migration. Strategy packs are user-supplied; no decision EV or live assistance.


## Documentation updates

- Add Japanese, Korean, French, German, Spanish, Brazilian Portuguese, Italian, Dutch, Polish and Turkish READMEs (13 documentation languages total; app UI remains three languages).
- Integrate the existing Forest / Midnight / Paper theme selector with persisted local preferences.
- Replace narrow dark screenshots with complete Paper-theme overviews and the full starting-hand matrix, plus a 1920 × 1080 desktop capture. All images use synthetic hands.
- Keep the original v0.1.0 tag unchanged.

## v0.1.0 — Public preview

Initial public source release: offline TXT/ZIP imports, local SQLite storage, result dashboards, 13 core statistics, observed starting-hand matrix, hand replay, review annotations, export/backup/restore and three interface languages.

Includes English, Traditional Chinese and Simplified Chinese READMEs and fresh synthetic-data screenshots. Local validation: 18 frontend tests, 34 Rust tests and a successful frontend production build. See docs/validation.md for coverage boundaries.

Source-only preview; no signed/notarized installers. Native Windows and Intel runtime acceptance remain pending. No live HUD/RTA, GTO action scoring or cloud synchronization.
