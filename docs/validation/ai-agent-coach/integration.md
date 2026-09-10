# Public v0.2.0 integration acceptance

Base: 8930bfa7ad73fa5f392e492c3be244d1982e93ca. Original AI feature: c6db30f.

Resolved source conflicts while preserving StudyWorkspace, StudyRequest, both index initializers, both rebuild persistence paths, existing replayer training controls and Paper default. One shared replay step serves both entry points. Public multilingual README and licensing text preserved.

Verified locally:
- TypeScript/Vite production build passed.
- 27 frontend tests passed (including existing study/theme suites and AI draft/spot tests).
- Strict all-target Clippy passed for core, AI and desktop.
- Combined AI-report/offline-saved-spot rebuild and restore regression passed.
- Full core/AI suite running; 100k index resume/pagination test in progress.
- Native integrated bundle build in progress.

Earlier native/Codex evidence refers to the isolated original AI build, not automatically to this merged binary.

Provider API-key handoff is pending. No new version tag or installer release published. AI and offline study learning namespaces remain separate; see feature scope in docs/ai-agent-coach.md.
