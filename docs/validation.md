# Public preview validation

Version: 0.1.0. Public source snapshot based on local commit `70e4f9d`.

- Frontend: 18 tests passed across 3 files.
- Rust core: 34 tests passed. The private-archive acceptance case is excluded from the public source; no private-data acceptance is claimed.
- Production frontend build: passed; Vite reports a large chunk and mixed static/dynamic dialog imports.
- Browser preview: overview and observed starting-hand matrix loaded from an isolated 240-hand synthetic database; English, Traditional Chinese and Simplified Chinese rendered successfully.
- Screenshots are real browser captures of synthetic fixtures. Their frequencies and results do not represent real player performance or recommended strategy.
- Native installers were not rebuilt or revalidated for this public source release. No signed/notarized binary is attached. Windows and Intel native runtime acceptance remain unverified for this release.

Historical private acceptance logs, screenshots, databases and local paths are intentionally excluded from this public repository. GitHub Actions results are separate from the local checks above.

## Multilingual README and Paper-theme update — 2026-09-10

- 20 frontend tests passed, including two theme preference regressions; production frontend build passed. Existing chunk/import warnings remain.
- Ten new translated READMEs, 13 total; app interface remains English, Traditional Chinese and Simplified Chinese. Translations have not had native-speaker editorial review.
- Browser proof used the production frontend build and an isolated 240-hand synthetic database. Selected Paper through the settings UI; verified all three interface languages.
- Complete overview captures use a 1920 × 1900 CSS viewport; matrix uses 1920 × 2040. Main content clientHeight equals scrollHeight, confirming no hidden lower content.
- The browser capture API emits JPEG at half the CSS dimensions for its clip path. Captured the complete rendered surface at 960 × 950 (overview) and 960 × 1020 (matrix), without padding or duplicated tiles. Separate desktop capture is 1920 × 1080.
- Existing v0.1.0 release tag remains unchanged. This update is on main and does not claim new native installer acceptance or remote CI success.
