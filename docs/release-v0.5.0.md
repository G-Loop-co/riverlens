# RiverLens v0.5.0 — Study loop, range drill, research wizard, progress dashboard

v0.5.0 connects the personal study loop end to end and adds a guided MDA research flow. Personal review stays in SQLite, MDA in independent DuckDB datasets; dataset cloud / MCP sharing remains off by default.

## Changes

- AI practice drafts convert into Trainer self-review cards: the learning library offers "Add to Trainer" per practice draft; references resolve through the indexed decision ledger, stale-index items are counted and never guessed, and repeated adds stay idempotent.
- Range drill: draw starting-hand combos from a strategy-pack node, assign action frequencies, and compare against the conditional strategy. Similarity is 100 minus total variation distance; it is descriptive and not EV, significance or a GTO score. Session records persist as `range_quiz` study items and are included in backups.
- MDA research wizard: guided hypothesis (five common presets) → street/stake filters → Wilson 95% interval review → anonymous hand verification → hand-off to AI exploitation analysis with a composed evidence request. Sub-200-opportunity contexts show an explicit caution; multi-context results are labelled and never merged. The wizard only orchestrates existing scoped MDA queries.
- One-page learning progress dashboard: trainer totals / due / completed, recent practice with replay links (shared component with the Trainer), leak-target status counts and top-priority details, and range-drill history. Target status counts exclude observation presets; small samples are labelled, never conclusions.
- The study workspace now has six steps (progress → spots → leaks → strategy → range drill → trainer) with a consistent step indicator.

## Validation and limits

- Branch evidence at the release commit: `cargo fmt --check` clean; `cargo clippy -p poker-core -p riverlens-ai -p riverlens --all-targets -- -D warnings` clean; 110 Rust tests across 15 suites and 54 frontend tests pass; `npm run build` passes. `scripts/signing-env` unit test passes.
- Live proof on a development machine (web preview against the local core): 400-hand synthetic GG Regular MDA dataset imported through the documented path; the research wizard was walked end to end (preset → filters → real Wilson-interval stats with the sub-200 caution → anonymous hand rows → coach hand-off with cloud sharing left disabled); progress dashboard, range drill and trainer verified in the same session with zero page errors. 16 automated browser checks pass.
- Tag CI gates publication: Apple Silicon, Intel Mac and Windows packaging, packaged MCP smoke and manifest / checksum generation must pass before the reviewable draft release is created. Publication of the draft remains a separate human review step.
- No database migration is introduced. The new `range_quiz` study-item kind is additive and backward compatible with existing study items; existing Trainer cards, attempts, benchmarks and saved spots are untouched.
- MDA boundaries are unchanged: GG Regular USD NLH 6-max only, descriptive Wilson 95% intervals, missing baselines stay research hypotheses, and shown-card samples carry selection bias. GG SEP restrictions on collecting other players' histories and systematic MDA sharing still apply. Live provider calls and room-client certification claims are unchanged from v0.4.1.

## Downloads and updates

Apple Silicon and Intel macOS `.app.zip`, Windows x64 NSIS installer, `build-manifest.json` and `SHA256SUMS.txt` accompany the release. Verify downloaded assets against the manifest and checksums.

macOS packages are **ad-hoc signed, not Developer ID notarized**. Archive extraction is verified with deep / strict codesign checks and executable permissions. Intel cross-compilation proves packaging, not Intel native runtime acceptance.

No in-app updater is included. Back up active stores before replacing an installed application; retain the app-data directory. This publication does not replace the user's local app or restore / overwrite their database. Private histories, local databases, strategy packs and research corpora are excluded.

[Support table](../README.md#personal-review-hand-history-formats-v050) · [Study workflow](study-workflow.md) · [MDA scope and restrictions](mda-population.md) · [Changelog](../CHANGELOG.md)
