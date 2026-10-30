# Proposal

## Why

Autopilot could only call OpenAI, Anthropic or Google (#254). Users with an
OpenCode or OpenRouter account had to keep a separate key just for Autopilot.

## What Changes

- Two more providers, both reached over OpenAI's Chat Completions request
  shape at their own endpoints:
  - **OpenCode Zen** (`https://opencode.ai/zen/v1/chat/completions`), with
    `deepseek-v4-flash`. A paid model: OpenCode refuses its free models (HTTP
    403) to every client but OpenCode itself.
  - **OpenRouter** (`https://openrouter.ai/api/v1/chat/completions`), with
    `openai/gpt-5-mini`.
- Considered and not added: **Zed**. Its hosted models are available only
  inside the Zed editor; there is no public API for another app to call.

## Impact

- `knot-core`: `AiProvider::{OpenCode, OpenRouter}`, stored as `opencode` and
  `openrouter`.
- `knot-autopilot`: their models and endpoints; OpenAI's request function is
  now shared as `call_chat_completions`.
- `knot`: the provider picker labels. The picker is built from
  `AiProvider::ALL`, so it lists them with no other change.
