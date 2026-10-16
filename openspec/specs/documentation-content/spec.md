# documentation-content Specification

## Purpose
Defines the user-facing content the published documentation site carries, how
those pages are addressed and linked, and how the site navigation is derived
from version data, so a reader can learn and operate every capability the Rust
application implements.

## Requirements

### Requirement: Site documents user-facing capabilities

The published site SHALL provide a page for each user-facing capability area of
the application, grouped by the task a reader performs, and SHALL describe
observable behavior rather than internal structure.

#### Scenario: Reader opens a capability guide

- **WHEN** a reader opens a guide page
- **THEN** the page explains the tasks the capability supports and the states a
  reader can observe

#### Scenario: Capability has no user-facing page

- **WHEN** a capability the packaged application exposes is not described by any
  page
- **THEN** the site is incomplete and the omission is a documentation defect

### Requirement: Pages are addressed by stable versioned permalinks

Each page SHALL declare a `title`, a `version`, and a permalink rooted at its
version path, so the page is reachable at a stable URL and belongs to exactly one
documentation version.

#### Scenario: Reader navigates to a guide

- **WHEN** a reader follows a link to a guide or reference page
- **THEN** the page resolves under its version path and reports that version

### Requirement: Navigation is derived from version data

The site SHALL render the sidebar navigation for the selected version from
navigation entries declared in `_data/versions.yml`, and SHALL NOT hard-code
page links in the layout.

#### Scenario: A page is added to a version

- **WHEN** a page is added and listed in a version's navigation entries
- **THEN** the sidebar shows a link to it without a layout change

#### Scenario: An older snapshot omits a page

- **WHEN** a reader selects a version whose navigation does not list a page
- **THEN** the sidebar for that version omits the link rather than pointing at a
  missing path

### Requirement: Reference material lives on the site

The site SHALL carry the agent command-line arguments, the agent shell
environment, and the keybinding reference, and SHALL be the single home for that
material rather than pointing at loose files in the repository.

#### Scenario: Reader looks up an agent CLI option

- **WHEN** a reader needs an argument, shell setting, or keybinding
- **THEN** the site provides it under a reference page reachable from the
  navigation

### Requirement: Pages identify their source contracts

Each guide page SHALL name the capability specs it summarizes, so a contract
change points at the page that must be updated.

#### Scenario: A spec changes

- **WHEN** a capability spec changes
- **THEN** the page naming that spec identifies itself as the place to update
