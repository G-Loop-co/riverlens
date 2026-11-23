# RiverLens v0.6.3 — Personal bluff analysis

v0.6.3 adds a decision-level bluff report for personal no-limit Hold'em cash hands. It stays local and works across heads-up and multiway postflop pots.

## Changes

- **Single-street bluff classes:** compare flop and turn air, strong draws and river air with immediate-fold rates, segment cashflow and whole-hand net.
- **Postflop lines:** review double-barrel routes, triple barrels, delayed river barrels after a checked-through turn, and check-raise leads.
- **Next-action denominators:** measure whether a bluff continued, stopped or became a made-hand bet only when Hero had a legal bet or raise on the next street.
- **Evidence and replay:** split by bet size, action, response, position, pot type, street-start player count, effective stack and board texture. Open the exact decision in replay.
- **Marked bluffs:** mark a lower-pair bluff decision while replaying and review those choices in a separate view without changing the automatic hand class.
- **Incremental indexing:** existing databases backfill in resumable batches. Missing Hero cards, split pots, side pots, cashouts and multiple runouts remain visible with explicit coverage or exclusion reasons.

## Calculation notes

Air and Draw use only Hero's cards and the board visible at the decision. Draw means at least eight distinct next cards complete a straight or flush using a hole card. Segment cashflow covers the selected street from its first aggressive action and excludes awards that occur only after later decisions. Whole-hand net uses the final stored ledger. Both are historical results, not decision EV.

## Validation

- Frontend suite: 66 tests passed across 15 files; production build passed.
- Bluff engine: 12 synthetic integration tests cover multiway partial folds, three barrels, continuation denominators, repeated raises, check-raise leads, missing cards, side pots, runouts, manual marks, rebuilds and pagination.
- Rust core, study and tournament regression suites passed, including the 100,000-hand index-resume audit. RiverLens AI's 13 tests passed, including local IPC scenarios.
- Local Apple Silicon packaging produced a valid ad-hoc-signed `RiverLens.app` with version `0.6.3`; the archived app signature and packaged MCP smoke passed (protocol `2025-11-25`, 23 tools, disabled access denied).
- Exact tag CI must pass macOS Apple Silicon, macOS Intel and Windows x64 packaging before publication.

## Downloads and updates

Apple Silicon and Intel macOS `.app.zip`, the Windows x64 NSIS installer, `build-manifest.json` and `SHA256SUMS.txt` accompany the release. Verify downloads against the manifest and checksums.

macOS packages remain **ad-hoc signed, not Developer ID notarized** unless release secrets configure Developer ID signing. Intel cross-compilation proves packaging, not native Intel runtime acceptance.

No in-app updater is included. Preserve the app-data directory and back up active stores before replacing an installed application. Private hand histories, notes, databases, strategy packs and research corpora are excluded.

[Architecture](architecture.md) · [User guide](user-guide.md) · [Changelog](../CHANGELOG.md)
