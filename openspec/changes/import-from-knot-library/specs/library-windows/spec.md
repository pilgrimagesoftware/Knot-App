# Spec Delta

## ADDED Requirements

### Requirement: The Personas and Prompts windows open the library import

The Personas window and the Prompts window SHALL each have an "Import from
Library…" action beside "Add". Choosing it SHALL open the Import window, or
bring it to the front if it is already open (see `import-ui`), where the
Library section lists what can be imported.

The Bench window SHALL NOT have this action until the library format has a
bench kind.

#### Scenario: Importing from the Personas window

- **WHEN** the user chooses "Import from Library…" in the Personas window
- **THEN** the Import window opens, and its Library section reads
  Knot-Library and lists its personas and prompts

#### Scenario: Imported personas appear in the open window

- **WHEN** the Personas window is open and the user imports a library
  persona
- **THEN** the persona appears in the Personas window's list without
  reopening it
