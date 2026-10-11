# RiverLens v0.4.0 — MDA population analysis + AI

Import an independent GG Regular USD NLH 6-max population dataset, filter a decision context, inspect legal-opportunity frequencies, ask the in-app Coach or MCP agent how to adjust, and verify the suggestion with anonymous hand replay.

## Population data and analysis

- Local DuckDB datasets remain separate from personal SQLite and learning data. Import TXT, ZIP or folders with streaming batches, atomic data/checkpoint commits, pause/resume, incremental summaries, network/hand-ID deduplication and raw conflict quarantine.
- Share the existing GG parser and exact-money action validation. Count decisions for every eligible player without requiring a Hero; missing hole cards stay unknown.
- Analyze RFI, blind defense, 3-bet and responses, street C-bets and responses, check-raises, and bets after a checked-through street. Filter date, stakes, position matchups, pot type, player count, effective stacks, facing-bet size and decision-time board.
- Display numerator/opportunity denominator, descriptive 95% Wilson intervals, period consistency and shown-card coverage. Automatic candidates default to at least 200 legal opportunities; absent baselines remain research hypotheses.
- Independent backup/restore uses a detached DuckDB snapshot, including schema and sequences. Restore creates a new dataset identity/version with both sharing permissions off.

## AI and evidence

- Five shared tools: `get_mda_catalog`, `query_mda_stats`, `find_mda_candidates`, `find_mda_spots`, `get_mda_hand`. Queries pin dataset/version and enforce bounded pagination.
- In-app providers retain the existing provider/model picker. Cloud Coach and external MCP need separate explicit per-dataset opt-in; both start disabled. Send aggregates and a bounded set of anonymous examples. A data-version change stops analysis.
- Reports require context, counts, adjustment direction/sizing, reasons, exceptions and anonymous hand evidence. Review existing learning drafts and replay supporting hands; local drafts remain available after sharing is revoked.
- Pure-bluff break-even arithmetic is restricted to its stated heads-up river assumptions. No solver, node-lock, GTO action scoring or decision EV is included.
- The first MDA workspace uses Traditional Chinese labels; personal-review pages retain English, Traditional Chinese and Simplified Chinese.

## Validation

- Local regression: 77 Rust core tests, 13 AI tests, 40 frontend tests, build, rustfmt, strict all-targets Clippy and a 23-tool packaged MCP smoke passed.
- Complete 10,000,000-hand synthetic import passed on Apple M3 Pro, 18 GiB RAM, DuckDB 2 GiB buffer limit and four threads. Aggregate p95 16.440 ms; contextual p95 1,198.459 ms; peak process RSS 1.385 GiB; import 64 minutes 43 seconds; database 19.273 GiB.
- The benchmark covers two documented warm-cache core queries; it excludes desktop UI, IPC and provider latency. Its measured source precedes later UI/replay DTO/backup fixes, with unchanged parser, facts, Appender and statistics SQL. [Exact source, hardware, queries and raw results](validation/mda-population-ai.md).
- Implementation CI at `718e350` passed macOS ARM, macOS Intel and Windows, including the Windows backup fix. Release-tag CI and independently downloaded asset verification establish the final version/source/packaging identity; they do not establish native GUI or live provider acceptance.
- HHDealer GG Regular PT/HM/H2N4 needs real sample import/statistics/replay tests. Live provider, external model-client MDA analysis and Windows/Intel native GUI acceptance remain unverified. Local Mac GUI review was blocked by the locked session; browser-to-isolated-core UI and native protocol smoke are separate evidence.

## Downloads, signing and updates

- Packages: Apple Silicon macOS `.app.zip`, Intel macOS `.app.zip`, Windows x64 NSIS installer; source/build manifest and SHA-256 checksums accompany the assets.
- macOS downloads use **ad-hoc signing, not Developer ID notarization**. CI verifies the complete bundle before archiving and after extracting it, including executable permissions; Gatekeeper may require user approval.
- No in-app updater is included. Replace the installed application to update; preserve the app-data directory and use built-in backups for active data stores.
- Personal histories, local databases, strategy packs and the 10-million-hand synthetic dataset are excluded from downloads.

## Platform restrictions

GG's [Security & Ecology Policy](https://ggpoker.com/network/security-ecology-policy/) sections 7 and 19 restrict collecting other players' histories and collecting/sharing MDA. Penalties can include suspension, bans and confiscation of funds. Shipping this feature does not establish platform permission. RiverLens provides offline research, with no game-client connection, live HUD or real-time assistance.

[Quick start](../README.md#mda--ai-quick-start) · [Traditional Chinese guide](../README.zh-TW.md) · [Statistical definitions](mda-population.md) · [Changelog](../CHANGELOG.md)
