# Spec Delta

## ADDED Requirements

### Requirement: The Personas and Prompts windows open the library import

The Personas window and the Prompts window SHALL each have an "Import from
Library…" action beside "Add". Choosing it SHALL open the Import window, or
bring it to the front if it is already open (see `import-ui`), where the
Knot-Library section lists what can be imported.

The Bench window SHALL NOT have this action until Knot-Library publishes
bench entries.

#### Scenario: Importing from the Personas window

- **WHEN** the user chooses "Import from Library…" in the Personas window
- **THEN** the Import window opens, and its Knot-Library section fetches
  and lists the library's personas and prompts

#### Scenario: Imported personas appear in the open window

- **WHEN** the Personas window is open and the user imports a library
  persona
- **THEN** the persona appears in the Personas window's list without
  reopening it
