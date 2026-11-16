# Spec Delta

## ADDED Requirements

### Requirement: The markdown section lists the agent's earlier files

While an agent has shown markdown files before the one its section shows, the
section's header SHALL offer a control, with a localized tooltip, listing those
files by name, most recent first, from the same history the agent's Markdown
Files menu reads. Choosing one SHALL show it in the section again, unexpanded,
through the store as that menu does. The control SHALL NOT be shown when there
is no earlier file.

#### Scenario: Getting back to a file the agent replaced

- **WHEN** the user is reading `plan.md` and the agent shows `notes.md` over it
- **THEN** the section's earlier-files control lists `plan.md`, and choosing it
  shows `plan.md` again with `notes.md` now listed as the earlier file

#### Scenario: Nothing earlier

- **WHEN** an agent has shown only one markdown file
- **THEN** the section shows no earlier-files control
