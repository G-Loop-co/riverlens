# RiverLens v0.3.2

## Learning library

- New AI answers are automatically categorized, tagged and split into topic sections by the selected provider/model.
- External MCP agents can provide organization in new report drafts and use `organize_learning` to classify existing reports without separate in-app provider credentials.
- Existing reports have an AI categorize/split action. Select a provider/model and enable sharing consent in AI Coach first. Reclassification sends that report's text to the selected provider and may incur provider usage charges.
- Filter reports by category or tag. Topic sections retain exact original text, source evidence and acceptance status. Invalid or incomplete AI output is rejected; the original report is retained for retry.
- Editing report text clears the old classification so section offsets cannot become stale.

## Evidence viewer

- Fix the blank screen when a hand's stats are an object rather than the aggregate report's array.
- Preserve saved evidence tool, dataset version and query arguments. Display full evidence for every tool shape, with source-hand replay links where available.
- Collapse long evidence-link lists into an expandable group.

## Downloads and signing

- macOS bundles now receive an explicit complete app signature. CI verifies the bundle before archiving and again after extracting the ZIP, including executable permissions.
- Tag builds produce three validated platform assets, a source/build manifest and SHA-256 checksums, then create a release draft. Publication follows artifact review.
- **This release uses ad-hoc macOS signing, not Developer ID notarization.** Valid signatures prevent the broken bundle-seal problem, but Gatekeeper can still require user approval for downloads. It is not a promise of prompt-free installation.
- The workflow supports Developer ID/notarization through Apple signing and notarization secrets. When configured, packaging also requires stapler and Gatekeeper verification. See [Tauri's macOS signing guide](https://v2.tauri.app/distribute/sign/macos/).
- There is no in-app updater in this version; updates replace the installed application, preserving its data directory.

## Validation scope

Frontend regression tests cover mixed evidence shapes and saved-evidence navigation. Rust regression tests cover evidence envelopes, original-text preservation, complete section coverage and revision conflicts. macOS native/UI validation and downloaded artifact verification are recorded in the release closeout; cross-platform CI does not establish Windows or Intel Mac GUI acceptance.
