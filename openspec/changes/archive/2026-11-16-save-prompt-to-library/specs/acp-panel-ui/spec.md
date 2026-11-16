# Spec Delta

## ADDED Requirements

### Requirement: A prompt can be saved to the prompt library

Each user message in the panel SHALL expose, beside its copy control and
revealed and laid out as that control is, a save control with a localized
tooltip. Activating it SHALL open the prompt editor on a new prompt whose text
is exactly the message's text, with the caret in the empty name field, so the
user names it and saves it to the prompt library.

#### Scenario: Saving a prompt

- **WHEN** the user points at a sent prompt `run the gate` and activates its
  save control
- **THEN** the prompt editor opens on a new prompt whose text is
  `run the gate`, with the caret in its name
