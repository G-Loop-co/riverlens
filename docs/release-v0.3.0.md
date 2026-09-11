# RiverLens v0.3.0

AI coach and local MCP integration for Hero-only post-session review. The existing offline Spot Explorer → Leak Finder → Preflop reference → Trainer workflow remains available.

## New capabilities

- Enable/revoke a local MCP connection in AI Connections, then copy the displayed client configuration. The installed desktop binary exposes 17 bounded tools for catalog/metric discovery, statistics, hand and decision lookup, leak candidates, strategy references and learning drafts. No separate database server is needed.
- Ask AI from a replay decision, or create local evidence-backed reports, practice and study-plan drafts. Drafts require explicit user acceptance. Pre-decision practice excludes later actions, board cards and outcomes.
- OpenAI, Anthropic and Gemini provider adapters are available in the coach. Provider keys are optional and stored in the OS credential store. Without a key, local tools, offline study and compatible external MCP clients remain usable. An external client may require its own subscription/authentication and can send selected data to its model.
- AI records and existing offline study records survive rebuild and restore together. AI learning/strategy libraries are currently separate from the offline study libraries; the connector does not expose every existing study item.

## Validation and fixes

Windows CI functional waits now allow shared-runner scheduling without removing assertions or adding automatic test retries. Frontend CI runs test files serially with a 20-second per-test deadline. Rust lifecycle polling is bounded at 120 seconds with job-state diagnostics.

Release assets are built by the three-platform desktop workflow. See the linked workflow result in release notes for the exact revision. Packaged MCP protocol smoke runs on Windows and Apple Silicon; Intel macOS is cross-built.

Live cloud-provider validation was explicitly skipped because no API key was supplied. Windows/Intel native GUI acceptance is not claimed. No solver, decision EV, live assistance or bundled private hand histories/strategy packs are included. References with unmatched configuration remain reference-only.

## Installation

Download the asset matching your system from the v0.3.0 release. macOS app ZIPs preserve executable permissions; extract and move RiverLens.app to Applications. macOS packages are ad-hoc signed and not notarized, and Windows installers are unsigned. Preserve application data when updating; older databases are backed up before migration.

See [AI setup and scope](ai-agent-coach.md) for connection instructions and limitations.
