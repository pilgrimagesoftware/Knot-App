# Proposal

## Why

The API Key field in Settings > Autopilot > AI Provider shows the key in plain
text, so anyone looking at the screen, or at a screenshot or screen share, can
read it (#392).

## What Changes

- The API Key field is a secure field: it draws its value masked, with a
  toggle to reveal it while checking a pasted key.
- The stored value is unchanged. `ai_api_key` is still saved exactly as typed;
  masking affects only what is drawn.
- No other Settings field holds a secret. The other text inputs are the custom
  prompt, the extra CLI options and the MCP port, so none of them changes.

## Impact

- `knot`: the settings window's `ai_api_key_input` is created masked, and the
  Autopilot tab's API key row adds the reveal toggle.
- `settings-ui`: the Autopilot tab requirement now says the field is secure.
