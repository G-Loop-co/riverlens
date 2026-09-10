# Public preview validation

Version: 0.1.0. Public source snapshot based on local commit `70e4f9d`.

- Frontend: 18 tests passed across 3 files.
- Rust core: 34 tests passed. The private-archive acceptance case is excluded from the public source; no private-data acceptance is claimed.
- Production frontend build: passed; Vite reports a large chunk and mixed static/dynamic dialog imports.
- Browser preview: overview and observed starting-hand matrix loaded from an isolated 240-hand synthetic database; English, Traditional Chinese and Simplified Chinese rendered successfully.
- Screenshots are real browser captures of synthetic fixtures. Their frequencies and results do not represent real player performance or recommended strategy.
- Native installers were not rebuilt or revalidated for this public source release. No signed/notarized binary is attached. Windows and Intel native runtime acceptance remain unverified for this release.

Historical private acceptance logs, screenshots, databases and local paths are intentionally excluded from this public repository. GitHub Actions results are separate from the local checks above.
