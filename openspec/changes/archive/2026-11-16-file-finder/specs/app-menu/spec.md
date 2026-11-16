# Spec Delta

## ADDED Requirements

### Requirement: Edit > Find File…

The Edit menu SHALL carry Find File…, bound to ⌘⇧F as in the Swift reference,
after its text-editing items. It SHALL open `file-finder` for the selected
agent of the focused workspace window, and SHALL be disabled when no workspace
window is focused or it has no selected agent.

#### Scenario: Find File… with no agent selected

- **WHEN** the focused workspace window has no selected agent
- **THEN** Edit > Find File… is disabled
