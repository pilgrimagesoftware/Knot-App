# Spec Delta

## MODIFIED Requirements

### Requirement: Autopilot tab

The Autopilot tab SHALL show: an "Enable autopilot" toggle bound to
`autopilot_enabled`; an AI Provider section with a provider picker
(OpenAI/Anthropic/Google) bound to `ai_provider`, a secure API key field
bound to `ai_api_key`, which draws its value masked and offers a control to
reveal it, and a read-only model-name display derived from the
selected provider; and an Action section with a picker (Mark
conversation/Ask me/Auto-continue/Custom) bound to `autopilot_action`, plus
a custom-prompt text area bound to `autopilot_custom_prompt` shown only
when "Custom" is selected. Every control persists on change. Masking SHALL
change only how the API key is drawn, never the value stored. The tab SHALL
NOT claim or imply the feature is functional beyond storing these
preferences - no decision loop runs as a result of this change.

#### Scenario: Enabling autopilot persists

- **WHEN** the user turns on "Enable autopilot"
- **THEN** `autopilot_enabled` is saved as `true` immediately

#### Scenario: Changing provider persists and updates the model display

- **WHEN** the user selects "Anthropic" in the provider picker
- **THEN** `ai_provider` is saved as `"anthropic"` and the model display
  updates to the Anthropic model name

#### Scenario: API key field persists

- **WHEN** the user types a value into the API Key field
- **THEN** `ai_api_key` is saved with that exact value immediately

#### Scenario: API key field is masked

- **WHEN** the Autopilot tab shows a saved API key
- **THEN** the field draws it masked rather than as plain text, and the
  stored `ai_api_key` is unchanged

#### Scenario: Custom prompt only shown for the Custom action

- **WHEN** `autopilot_action` is not `"custom"`
- **THEN** the custom-prompt text area is not shown

#### Scenario: Selecting Custom reveals the prompt editor

- **WHEN** the user selects "Custom" in the action picker
- **THEN** the custom-prompt text area appears, bound to
  `autopilot_custom_prompt`, persisting on edit
