## ADDED Requirements

### Requirement: View > Collapse Tool Calls

The View menu SHALL list a **Collapse Tool Calls** item directly after
Artifacts, bound to ⌥⌘C (Ctrl+Alt+C off macOS). Choosing it SHALL collapse
every tool call in the selected agent's conversation, as
`collapsed-tool-call-summary`'s "Collapsing every tool call at once"
describes. It SHALL be enabled on the same terms as Jump to Bottom: a
conversation is on screen for the selected agent.

#### Scenario: Collapsing from the menu

- **WHEN** a Panel-mode agent's conversation shows open tool-call cards and
  the user chooses View > Collapse Tool Calls
- **THEN** every card collapses to its header

#### Scenario: No conversation on screen

- **WHEN** the selected agent is in Terminal mode, or a Dashboard or Pull
  Requests panel is showing
- **THEN** View > Collapse Tool Calls is disabled
