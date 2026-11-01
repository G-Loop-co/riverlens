# RiverLens v0.6.0 — Leak trend snapshots, study CSV export, full MDA localisation

v0.6.0 closes the learning feedback loop for leaks: every saved target can now be tracked over time, and all study history leaves the app as plain CSV. The MDA workspace and research wizard are fully localised. Personal review stays in SQLite, MDA in independent DuckDB datasets; dataset cloud / MCP sharing remains off by default.

## Changes

- Leak trend snapshots: each saved benchmark gains "Record snapshot" and "Trend" actions. Snapshots are recomputed by the engine from the indexed ledger at record time — the browser client never supplies the numbers — then stored append-only in a new additive `study_leak_trend` table with a 1,000-per-target cap. The trend panel charts actual frequency against the target range (dashed guides) using the app's existing chart stack.
- Study history export: practice attempts, range-drill records and leak snapshots export to brand-new CSV files (`create_new` refuses silent overwrites; PDF-style injection guard applied to text fields). Exports live on the progress page (desktop only; web preview cannot write files).
- Full MDA localisation: every workspace and research-wizard label, message, preset and step now routes through i18n for English, Traditional Chinese and Simplified Chinese, including interpolated job lines and wizard warnings. Dataset names, source text, player labels and metric identifiers remain untouched.
- Roadmap and README refreshed: completed roadmap items moved out; language-support statements updated across all thirteen README translations.

## Validation and limits

- Branch evidence at the release commit: `cargo fmt --check` clean; `cargo clippy -p poker-core -p riverlens-ai -p riverlens --all-targets -- -D warnings` clean; 112 Rust tests across 15 suites and 54 frontend tests pass; `npm run build` passes.
- Live proof on a development machine against a disposable copy of a real local store (deleted after the run, per cleanup discipline): 120 synthetic cash hands imported and indexed; a saved "BB fold vs open" benchmark snapshotted twice by the engine (368 legal opportunities, 246 folds, 66.8% with Wilson 61.9–71.5% against a 50–60% target); the trend panel charted both snapshots with target guides; all three CSV kinds exported and verified; the English UI rendered every MDA tab, wizard step and dataset label. Eight automated browser checks pass with zero page errors.
- No database migration: `study_leak_trend` is created additively on first open of an existing store; restorations of older backups recreate it the same way.
- MDA boundaries are unchanged: GG Regular USD NLH 6-max only, descriptive Wilson 95% intervals, missing baselines stay research hypotheses. Snapshot numbers describe observed frequencies, not EV or significance. GG SEP restrictions on collecting other players' histories and systematic MDA sharing still apply. Live provider calls and room-client certification claims are unchanged from v0.5.0.

## Downloads and updates

Apple Silicon and Intel macOS `.app.zip`, Windows x64 NSIS installer, `build-manifest.json` and `SHA256SUMS.txt` accompany the release. Verify downloaded assets against the manifest and checksums.

macOS packages are **ad-hoc signed, not Developer ID notarized**. Archive extraction is verified with deep / strict codesign checks and executable permissions. Intel cross-compilation proves packaging, not Intel native runtime acceptance.

No in-app updater is included. Back up active stores before replacing an installed application; retain the app-data directory. This publication does not replace the user's local app or restore / overwrite their database. Private histories, local databases, strategy packs and research corpora are excluded.

[Study workflow](study-workflow.md) · [MDA scope and restrictions](mda-population.md) · [User guide (Traditional Chinese)](../docs/user-guide.md) · [Changelog](../CHANGELOG.md)
