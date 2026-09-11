# RiverLens v0.3.1

Adds DeepSeek and OpenCode Go to AI Coach and AI Connections, alongside OpenAI, Anthropic and Gemini. The README lists all five providers, external stdio MCP access, and Go model IDs by protocol.

## Use

Save the matching API key in AI Connections, select the provider in AI Coach, enter a bare tool-capable model ID, choose the sharing scope and ask a question. Without an API key, offline study and external stdio MCP connections remain available.

## Changes

- Route Go model families to Chat Completions, Anthropic Messages or Responses as documented by the provider.
- Preserve DeepSeek reasoning for tool continuation without showing it as coaching evidence; return tool results in each provider's format.
- Identify Go requests as RiverLens and attach a stable conversation session header.
- Keep provider selection consistent between the coach and key settings; clear the model when switching providers.

## Validation and limits

Provider stream, tool-result and UI regression tests cover the new adapters. Three-platform CI builds Apple Silicon, Intel Mac and Windows packages; native packaged MCP smoke runs on Apple Silicon and Windows. Intel is cross-built. CI packages do not establish native GUI acceptance.

Live provider requests were skipped because no API key was supplied. Go is intended for coding-agent traffic; poker coaching eligibility and live compatibility remain unverified. Model availability is controlled by the provider. Vision-capable models receive text only.

macOS apps are ad-hoc signed and not notarized; Windows installers are unsigned. This remains local-first, Hero-only post-session study; no live assistance or solver EV. Private hand histories, databases and strategy packs are excluded.

See [AI setup](ai-agent-coach.md) and [supported connections](../README.md#supported-ai-connections).
