# Spec Delta

## MODIFIED Requirements

### Requirement: Supported providers

The system SHALL support OpenAI, Anthropic, Google, OpenCode Zen and OpenRouter
as classification and custom-action providers, each called with a fixed, cheap
model for the provider and the configured API key. A missing or
whitespace-only API key SHALL fail the request without making any call.

#### Scenario: Blank API key fails fast

- **WHEN** the configured API key is empty or whitespace-only
- **THEN** no network request is made and the call fails

#### Scenario: A gateway provider

- **WHEN** the provider is OpenCode Zen or OpenRouter
- **THEN** the request is an OpenAI-style Chat Completions call to that
  provider's own endpoint, with the key as a bearer token
