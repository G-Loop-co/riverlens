# Native follow-up, 2026-09-10

All data was synthetic: 30 hands in an isolated /private/tmp data directory. UI interaction used native accessibility via CUA. No private data or provider API keys were transmitted.

- Local draft evidence: 30 hands, VPIP 30/30, PFR 25/30, matching dashboard.
- Edited report title and accepted; persisted as accepted.
- Enabled MCP from the UI; packaged stdio client listed 17 tools and read the running desktop database.
- Catalog, bounded spot cursor, redacted hand, decision-before context passed.
- Created practice draft through MCP; native refresh displayed it as draft with no practice button. Acceptance exposed Practice 1.
- Before answer: Hero As Kd, previous actions only; no future board or result.
- Submitted Raise 3: actual raise shown, explicit self-review caveat, attempts incremented to 1.
- Revoked connection in UI: same connector returned isError with Enable AI connection in RiverLens first.
- Spot result BTN AKo #30 opened matching replay SYN000000000029.

Found and fixed: decision blinds/stacks had raw micro-units while serialized actions had decimal currency amounts. Native practice displayed SB 5e-7 bb / BB 0.000001 bb. Decision projection now uses the shared money formatter, with regression assertions against stored hand values; zero-amount actions no longer display an empty bb suffix. Catalog describes the unit boundary.

After fix: 10 core agent tests and 2 frontend AI tests passed, strict all-target Clippy passed. macOS bundle rebuilt.

Provider cloud calls, actual Codex/Claude clients, Windows and native strategy-file picker remain outside this follow-up proof.

Final corrected bundle SHA256: bd1bc3c0237cb86d91504ac525f86cb4226b96dfb11e310d5502093fcb52c907

Final corrected native process launched, but CUA app acquisition returned timeoutReached. Therefore corrected 0.5 bb / 1 bb rendering has regression/build coverage but no final native screenshot confirmation. Earlier native flow and revocation proofs above were completed before this transport failure.

2026-09-10 subsequent native retry PASSED: corrected app launched with explicit synthetic data directory; SB 0.5 bb, BB 1 bb, Fold without suffix. Actual Codex CLI created a one-question draft through seven successful tools; UI accepted it, submitted Raise 3, displayed actual raise with self-review notice, and attempts became 2. MCP revoked after test. See codex-client.json.
