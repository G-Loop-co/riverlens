# AI protocol round-trip validation

Verified against provider documentation on 2026-09-17:

- [DeepSeek thinking mode](https://api-docs.deepseek.com/guides/thinking_mode/): requests with tools retain reasoning from all preceding assistant turns.
- [Anthropic streaming](https://platform.claude.com/docs/en/build-with-claude/streaming): accumulate thinking and signature deltas, preserving original content-block order.
- [Gemini function calling](https://ai.google.dev/gemini-api/docs/function-calling): preserve signed model parts and return provider-supplied function-call IDs when present.
- [OpenAI function calling](https://developers.openai.com/api/docs/guides/function-calling): explicitly use non-strict Responses tools to preserve the engine's optional parameters.

Complete provider messages, including tool calls/results and reasoning, remain in process memory. The UI receives visible text only. History is scoped to conversation, provider, model and note-sharing setting. Model/provider/sharing changes and New conversation reset the UI session. Failed turns do not advance visible conversation history.

The cache holds at most eight conversations, each limited to 250,000 serialized bytes. Restart or eviction requires a new conversation; missing wire history is never reconstructed using fabricated reasoning. No reasoning is written to report drafts or emitted to the UI.

Regression checks cover streamed thinking/signatures, Gemini IDs with and without IDs, DeepSeek tool and final-answer history, model/session isolation, Responses optional tool arguments, UI followups, and retry after failure.

Validation: `cargo test -p riverlens-ai --locked` (12 passed), focused AI UI tests (5 passed), and `npm run build`. These are local contract checks, not authenticated provider acceptance. No paid API requests or desktop release packaging were performed.
