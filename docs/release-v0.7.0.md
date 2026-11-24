# RiverLens v0.7.0 — Review ranges and desktop updates

Compare six personal cash-game frequencies with offline reference ranges, open the hands behind a statistic and review your latest session. Desktop builds now check for signed updates in the app.

## Review

The six-handed dashboard separates currency, stakes and regular or Rush tables. It displays the numerator, opportunities, percentage and teaching band for VPIP, PFR, 3-bet, fold to 3-bet, flop c-bet and fold to flop c-bet. Broad ranges come from [Flopmortem](https://flopmortem.com/stats). [Pokalab’s VPIP study](https://www.pokalab.com/learn/what-is-a-good-vpip) provides additional context. A priority needs at least 100 opportunities and a descriptive 95% Wilson interval wholly outside the band. Position-specific samples retain the overall reference for context.

BB versus BTN provides an offline 100bb model reference for a 2.5bb open. The two aggregate percentages come from Adam / SolvePoker, [NLHE preflop by stack depth](https://solvepoker.app/nlhe-stack-depth), 2026-10-26, under [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/). The app selects the BB-versus-BTN row and translates the labels. The model assumes 5% rake capped at 2bb.

## Installation and later updates

Install v0.7.0 once using the Apple Silicon or Intel macOS archive, or the Windows x64 installer. Keep the existing RiverLens app-data directory. The desktop app checks for subsequent stable releases on startup. Settings includes a manual check and an automatic-check preference. Download begins when you choose to install. RiverLens verifies the update signature and checks for unsaved replay notes and active data operations before installation. The public update manifest is available as `latest.json` on the release.

Manual installation packages, signed updater payloads, the build manifest and SHA256 checksums are provided for each release. On macOS, builds are ad-hoc signed unless the release manifest records Developer ID notarization.
