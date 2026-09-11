# Public v0.2.0 integration acceptance

Base: 8930bfa7ad73fa5f392e492c3be244d1982e93ca. Original AI feature: c6db30f.

Resolved source conflicts while preserving StudyWorkspace, StudyRequest, both index initializers, both rebuild persistence paths, existing replayer training controls and Paper default. One shared replay step serves both entry points. Public multilingual README and licensing text preserved.

Verified locally:
- TypeScript/Vite production build passed.
- 27 frontend tests passed (including existing study/theme suites and AI draft/spot tests).
- Strict all-target Clippy passed for core, AI and desktop.
- Combined AI-report/offline-saved-spot rebuild and restore regression passed.
- Full core/AI suite passed, including 100k index resume/pagination (study audit suite 205.33 seconds). 78 Rust entries total including the separately added combined persistence regression.
- Integrated macOS bundle build passed. Native UI verification blocked by locked Mac; no integrated screenshot acceptance claimed.

Earlier native/Codex evidence refers to the isolated original AI build, not automatically to this merged binary.

Provider API-key acceptance is skipped at the user's request (2026-09-11); no live cloud-provider claim is made. No new version tag or installer release published. AI and offline study learning namespaces remain separate; see feature scope in docs/ai-agent-coach.md.

2026-09-11 CI: PR run 34482500718 passed ARM Mac, Intel cross-build and Windows, producing all three unsigned artifacts. Windows and ARM packaged MCP smoke passed. Push run 34482431508 had a Windows-only failure at the existing 30-second import lifecycle wait while the identical PR source passed. The helper now polls at 100 ms (rather than repeatedly opening coverage reads every 10 ms), allows 120 seconds, and reports job state on timeout. This is a functional completion deadline, not a performance claim. New CI verification is required for that test-only fix.

Cloud providers remain unverified: no API key/model response was supplied. The isolated integrated app started with an empty test database after synthetic seed-copy failed; do not count that launch as a successful data migration. The combined Rust persistence test passed independently.

## v0.3.0 candidate

The next Windows failure (PR run 34551230446) was in two DOM-heavy frontend workflows exceeding Vitest's default 5-second deadline; the same source passed push run 34551228026 including packaging and MCP smoke. CI now runs test files serially and permits 20 seconds per test, without retries or removed assertions. Local CI-mode execution passes all 27 tests. This follows Vitest's documented `fileParallelism` and `testTimeout` options: https://vitest.dev/config/.

Package, Tauri and workspace crate versions are aligned at 0.3.0. Final candidate CI and native validation are recorded with the release once completed.
