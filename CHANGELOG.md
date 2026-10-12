# Changelog

## 0.4.1 — 2026-10-12 — Multi-room Cash / MTT hand histories

- Support seven English NLHE tournament grammars (GG, PokerStars, modern WPN, Winamax, 888, iPoker XML and archived Full Tilt); use CHIPS, indexed tournament-ID filters and optional source metadata. Cash and chips cannot be summed. All-in chip EV does not represent ROI, prize money, ICM or bounty EV.

- Expand to 15 fixture-tested English NLHE cash hand-history families, including text and iPoker / Merge / MPN XML; retain source text and existing reconciliation, quarantine, duplicate and rebuild paths.
- Add a source-linked catalog of 94 room, network, archive and standard entries, with separate native, converted-only and unimplemented statuses.
- Support XML files inside ZIPs; retain per-hand session metadata and reject DTDs or unknown actions.
- Keep USD, EUR, GBP, CAD and USDT separate in reports and matrices; label replay/results and CSV exports with the actual currency.
- Respect explicit source timezone tags and use requested timezone boundaries for date filters.
- See [format/export research and limits](docs/hand-history-formats.md). Release packaging is validated separately; this does not establish current room-client certification.

## 0.4.0 — 2026-10-11 — MDA population analysis

- Add independent local DuckDB datasets for GG Regular USD NLH 6-max, with streaming TXT/ZIP/folder imports, atomic checkpoints, resume, deduplication, conflict quarantine and independent backup/restore.
- Count legal all-player decision opportunities using the shared exact-money GG parser. Unknown cards stay unknown; anonymous replay supports hands without a Hero.
- Add population context filters, opportunity-weighted rates, descriptive Wilson intervals, period comparisons, shown-card coverage and bounded hand evidence.
- Share five version-pinned MDA tools between the in-app Coach and MCP. Cloud and MCP access require separate per-dataset opt-in; reports use existing evidence-linked learning drafts.
- Keep missing baselines as research hypotheses, limit pure-bluff arithmetic to its stated HU river assumptions, and document GG SEP restrictions and pending HHDealer/live-provider/client compatibility.
- Add a reproducible 10-million-hand synthetic acceptance runner; measured results and platform packaging gates are recorded separately in the validation documents.

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
