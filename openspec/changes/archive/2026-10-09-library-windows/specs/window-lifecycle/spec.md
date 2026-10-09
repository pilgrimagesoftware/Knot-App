# Spec Delta

## ADDED Requirements

### Requirement: Each library window is a single window

A request to open the Personas, Prompts or Bench window SHALL bring that
window to the front when it is already open, and SHALL open it only when it
isn't. Closing one SHALL leave it closed, and the next request SHALL open it
again. The three are independent: having one open SHALL NOT affect opening
another.

#### Scenario: Opening the Prompts window twice

- **WHEN** the Prompts window is open and the user opens it again
- **THEN** the existing window is brought to the front and focused, and no
  second Prompts window appears

#### Scenario: Reopening after closing

- **WHEN** the user closes the Bench window and opens it again
- **THEN** a Bench window opens
