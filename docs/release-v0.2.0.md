# RiverLens v0.2.0

Paper (white) is the default on a fresh installation or an invalid stored preference. An existing Forest, Midnight or Paper preference is preserved. The initial HTML and native window background are light as well.

## Study workflow

Spot Explorer → Leak Finder → Preflop reference → Trainer operates entirely offline. Presets match conditions before the Hero decision and retain ordered action paths. The C-bet preset supports IP and OOP and excludes donk bets; the open-versus-3-bet preset requires that Hero opened.

Without a benchmark, Leak Finder displays observed frequencies and Wilson intervals with no target, deviation or review-priority judgement. Custom benchmarks use a default minimum of 100 opportunities. Hand results are counted once even when a hand contributes multiple decisions.

The current adapter cannot establish an identical complete source and hand-history model, including rake. Historical comparisons therefore remain reference-only and do not produce GTO grading. Internally consistent theory cells can be used for frequency drills. Ancestor weight issues propagate to dependent cells when importing or loading old packs. Raw source values and earlier attempts are preserved.

Strategy pages include a decision-before-action cohort with matching/reference separation, hand-class selection and cursor pagination. Each historical decision can open replay or enter training. From any marked-hand replay, stop before a Hero action and select **Train this Hero decision**; **Before next Hero decision** helps locate eligible steps. Annotations, reviewed state, question-bank membership and attempts are separate.

The stable decision reference remains `hero-decisions/2`. The derived index is `hero-index/3`; existing databases are backed up before migration and reindexed in resumable batches. Training sessions survive restarts, including submitted but unrated answers.

## Validation

Local release checks include frontend tests, Rust regressions, Clippy, formatting and TypeScript/Vite builds. The carried-forward study acceptance includes 100,000 synthetic hands / 416,667 decisions, restart after a 50-hand index batch, 300 disjoint cursor rows and incremental import to 100,001 hands. Rebuild and staging restore preserve study tables and annotations. Private archive acceptance is excluded from public CI.

The release workflow builds Apple Silicon, Intel macOS and Windows artifacts. macOS app ZIPs are created with `ditto` before upload to preserve executable permissions. Check the workflow run for the actual platform results; a successful cross-build is not native runtime acceptance. Apple Silicon runtime checks cover source-pack import, replay, training and restart persistence. No private hand histories, local databases, strategy files or private audit artifacts are included in this repository or installers.

## Installation and limits

Download packages from the [v0.2.0 release](https://github.com/G-Loop-co/riverlens/releases/tag/v0.2.0). The macOS packages are ad-hoc signed and not notarized; Windows packages are unsigned. Keep the existing application data when updating. The first launch backs up older study indexes before rebuilding them.

Strategy files must be supplied locally by the user. No decision EV, EV-loss scoring, live assistance, solver or cloud service is added. Previously selected themes stay selected; choose **Settings → Appearance → Paper** to switch an existing installation to white.
