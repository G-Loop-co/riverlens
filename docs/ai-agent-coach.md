# RiverLens AI agent and coach

Implementation date: 2026-09-10. Original feature commit: c6db30f. Release integration branch: feat/ai-agent-coach-integration, based on public main 8930bfa (v0.2.0).

## Delivered behavior

The desktop includes AI Coach, Spot Explorer, Learning Library and AI Connections. OpenAI, Anthropic and Gemini adapters share the same native Rust tools used by the local MCP connector. The existing core still computes statistics; the model chooses queries and explains evidence.

The app binary doubles as a stdio MCP connector. It does not initialize a second Service or database. It connects to the running desktop through interprocess local sockets (Unix-domain sockets on macOS, named pipes on Windows). Enablement lasts for the running app session. The current client configuration is displayed in AI Connections; update it after restarting the app. Disable revokes future calls. An in-progress core transaction may finish before revocation/cancellation returns.

API keys and the per-session connector token use the OS credential store (keyring). There is no account service, telemetry or central API proxy. Keys are not stored in SQLite, localStorage, logs or backups. Core functionality remains available without a key. The cloud coach requires explicit sharing consent per mounted coach session. Raw HH is never exposed to these tools; opponent names are replaced with seat labels. Notes and tags require the additional sharing option. Saved reports and learning artifacts are part of the agent-accessible library.

## Getting started

1. Build and open the desktop app: npm run desktop:build -- --bundles app.
2. Open AI Coach > AI Connections. Select a provider, enter your own API key and save it.
3. In AI Coach, enter an available model ID supporting tool calling. Select the sharing scope, enter a question and start analysis. The current filter is supplied as context.
4. Inspect evidence cards. Reports, practice sets and plans remain drafts until accepted in Learning Library. Edit a draft before accepting it. Stale evidence requires a fresh analysis.
5. For an external client, enable the local MCP connection and copy the displayed configuration to that client's stdio MCP settings. No HTTP endpoint or public service is exposed.

The external-client configuration uses the desktop executable with arguments --mcp --socket NAME. A standalone riverlens-mcp binary is also available from the riverlens-ai crate; use --socket NAME with that binary. Local transport does not imply local inference: external clients may send returned data to their model providers.

Model requests use documented native APIs: OpenAI Chat Completions, Anthropic Messages, Gemini streamGenerateContent. Model IDs are user-configured rather than tied to a moving default. Each analysis is capped at 12 model rounds, 48 collected evidence references, 250 KB of serialized message context and 4 MB per provider response. No hidden automatic retry or provider fallback. Cancellation drops the active model stream; completed evidence and drafts remain.

## Tool surface

- Discover: get_capabilities, get_data_catalog, get_metric_definitions.
- Analyze: query_stats, query_matrix, list_saved_filters, find_spots, get_hand, get_decision_context, find_leak_candidates.
- Strategy: list_strategy_packs, compare_preflop.
- Learn: get_learning_progress, search_learning, create_report_draft, create_practice_draft, create_study_plan_draft.

17 tool schemas are shared across MCP and all providers. Unknown tools, unknown arguments, unrecognized filter fields, null required values, unbounded pages and invalid identifiers are rejected before dispatch. No administration RPC, SQL, restore, import or annotation-apply tool is exposed.

find_spots matches an ordered subsequence of position/street/action steps; intervening actions are permitted. Page size defaults to 60 and caps at 200; the ID cursor and dataset version must be retained when paging. Existing effective-stack filters retain their documented HU-flop meaning; they are not a new universal multiway preflop effective-stack metric.

Each tool result contains evidence_id, version, tool, args and data. Data-dependent calls are pinned to the initial dataset version in the in-app coach. Changes to hand statistics, annotations or strategy configuration invalidate that version. Restores and rebuilds rotate the dataset epoch. Evidence is immutable and retained with drafts. Model interpretation is displayed separately from engine evidence; citations provide provenance, not a guarantee that every prose inference is correct.

Custom leak targets are explicit percentages. Wilson 95% intervals use the engine's opportunity denominator; an interval outside the declared target is a candidate, not a solver verdict. Without a target, results are review-only. All-in equity is never relabeled as decision EV.

## Strategy formats and scoring boundaries

The native strategy picker accepts the existing Nexus manifest with relative range files. Paths must remain inside the selected manifest directory. Source reach weights are preserved, never normalized into conditional frequencies. Parent comparisons are per player's prior action; missing/overflowing weights are recorded.

The current local Cash6mGeneral_6mNL10R25 manifest was re-read into a temporary database: 81 nodes, 36 nodes with absolute overflow greater than 0.00001. This count uses an explicit absolute-weight tolerance and is not an independent solver re-verification. The entire imported reach-weight pack remains reference-only, including mathematically consistent nodes.

Verified conditional packs use format riverlens.strategy/1. Required top-level fields:
- id.
- source.url (HTTPS) and source.verification_reference (the producer/user's verification provenance).
- configuration: game, currency, bb_units (integer micro-units as string), max_seats, depth_bb, rake_id, tree_id.
- nodes: map keyed by HERO|PATH, with hero, path and actions. Each action has action and weights mapping canonical hand classes to conditional frequencies.

Supported action labels: Fold, Call, Check, Allin, Raise <raise-to BB>. PATH uses F/C/X/R<size>/RAI separated by hyphens. Nonzero conditional frequencies for each represented hand must sum to one within 0.000001. Invalid sums are rejected; they are not repaired. Verification metadata is user-supplied provenance, not a claim that RiverLens independently contacted a solver.

In AI Connections, explicitly confirm the actual source profile's rake_id and tree_id. Exact mode additionally requires matching game, currency, blind size, table size, all starting stacks, Hero position and complete action path. Missing or mismatched context remains reference-only. A supported mixed-strategy action is not marked wrong merely because another action has higher frequency. No action EV is invented.

Practice serves a decision-before projection: no raw HH, future board, net result, annotations or opponent cards. Before submission the UI uses an engine question rather than the AI-authored item prompt. Submission records the attempt, then exposes the actual action. Exact preflop strategies supply action frequencies; otherwise the exercise remains self-review. To receive a selected-action frequency, submit an exact action label; free-form reasoning still records as self-review without inventing a score.

## Storage and verification

Additive ai_* SQLite tables store evidence, drafts, attempts, strategy packages, profile configuration, and ordered action indexes. FTS5 supports local library search. The existing SQLite backup API includes these tables; keys and socket credentials remain outside it. Restoring older backups initializes missing AI tables. Rebuild preserves learning records and rejects any unexpected hand-ID remapping before replacing the live database.

Tests cover engine parity, input boundaries, future-information redaction, ordered paths, SQL parameterization, draft idempotency/conflicts, explicit acceptance, missing/stale evidence, note-sharing, backup/restore/rebuild persistence, exact/reference strategies, mixed frequencies, practice reveal, provider stream assembly and IPC authentication/revocation.

Acceptance status:
- Rust core and AI regressions: 51 test entries passed (45 core, 6 AI); opt-in private archive tests do not run without their environment variables. The real strategy manifest was separately exercised. [Output](validation/ai-agent-coach/rust.txt).
- Frontend and i18n tests: 22 passed, including local draft acceptance and spot navigation. [Output](validation/ai-agent-coach/frontend.txt).
- Native compile, strict Clippy and macOS app bundle: built successfully. [Clippy](validation/ai-agent-coach/clippy.txt), [bundle](validation/ai-agent-coach/bundle.txt).
- Packaged stdio initialize/tools-list and enabled desktop bridge: passed with 17 tools. Catalog returned 30 synthetic hands, hand redaction and decision-before projection passed, a practice draft appeared in the native app, and revoked access returned isError. This is protocol/desktop integration proof, not live model-client acceptance.
- Real source pack: read-only audit passed; reference-only status retained.
- Native UI after unlock: 30 synthetic hands; local statistics evidence matched dashboard, draft title editing/acceptance persisted, spot search opened the matching replay, MCP-created practice required acceptance, answer submission revealed the actual action and incremented attempts. Native proof found a blind/action unit mismatch; fixed with core regression coverage and corrected native 0.5 bb / 1 bb display confirmed. See [follow-up](validation/ai-agent-coach/native-followup.md).
- OpenAI/Anthropic/Gemini live calls: NOT VERIFIED; no provider credentials were supplied or transmitted.
- Codex CLI live-client acceptance: PASSED via session-scoped stdio configuration and --approve-for-me. Seven tool calls created an evidence-linked one-question draft, then native acceptance and answer submission passed. [Evidence](validation/ai-agent-coach/codex-client.json). Claude Code/Claude Desktop remain NOT VERIFIED.
- Windows packaging and named-pipe/native credential acceptance: NOT VERIFIED on this macOS host.
- No claim of new million-hand performance or independent solver correctness.

## References

Official rmcp: https://github.com/modelcontextprotocol/rust-sdk
MCP transports: https://modelcontextprotocol.io/specification/2025-11-25/basic/transports
OpenAI: https://developers.openai.com/api/reference/resources/chat
Anthropic: https://platform.claude.com/docs/en/build-with-claude/streaming
Gemini: https://ai.google.dev/api/generate-content

Native follow-up (2026-09-10): found inconsistent units in decision projections during synthetic UI proof. Normalize blinds/stacks to the same decimal currency strings as actions; regress against stored hand amounts before re-verifying practice.

## Acceptance closeout in progress

2026-09-10: corrected native practice confirmed SB 0.5 bb and BB 1 bb. Final Codex live-client, provider-key handoff, and release integration checks are in progress. Public main is now v0.2.0 (8930bfa7ad73fa5f392e492c3be244d1982e93ca); the AI feature branch predates its offline study suite. Preserve that suite during any release integration. Extend desktop CI to cover riverlens-ai and the desktop crate rather than only poker-core.

## v0.2.0 integration

The AI feature was merged into public v0.2.0 in an isolated checkout. Existing offline StudyWorkspace, schema 3 backup-before-migration, training schedule/history, reference-only strategy rules, Paper default and portable Windows build launcher are preserved. Replayer shares one initial-step state for both study and AI navigation.

AI drafts/strategies and the offline study library currently remain separate namespaces. The 17 AI tools cover the AI library plus shared hand/statistical data; they do not yet expose every offline study card, saved spot or imported study strategy. Do not describe this preview as full access to every v0.2.0 study feature.

A combined persistence regression seeds an offline saved spot and an AI report, then verifies both after Service rebuild and backup restore. The CI matrix now runs AI tests, full desktop Clippy and a packaged stdio smoke test on native ARM/Windows targets. Intel cross-build is packaged but not executed on the ARM runner.

Public release is pending cloud-provider acceptance and review of the integration. Existing v0.2.0 tags/installers are unchanged.
