# Spec Delta

## MODIFIED Requirements

### Requirement: Prompt variables

Prompt text - a library prompt's and a custom startup prompt's alike - SHALL
support the following variables, each written as its name between `{{` and
`}}`, with optional whitespace inside the braces (`{{ folder }}` is
`{{folder}}`):

| Variable | Expands to |
|---|---|
| `agent.name` | the agent's name |
| `agent.id` | the agent's knot agent ID |
| `agent.type` | the agent's type, e.g. `claude` |
| `folder` | the agent's folder, as an absolute path |
| `folder.name` | the last path component of the agent's folder |
| `workspace` | the name of the workspace the agent belongs to; empty if none |
| `branch` | the current branch of the agent's folder; the short commit hash when HEAD is detached; empty when the folder is not in a git repository |
| `date` | today's date in the local time zone, as `YYYY-MM-DD` |
| `clipboard` | the text on the system clipboard when the prompt is inserted into a composer; empty when the clipboard holds no text, and always empty in a startup prompt |

The set is closed: these are the only names that expand. Names SHALL be
matched case-sensitively.

Expansion SHALL happen when the text is used, never when it is stored: the
library and every startup prompt keep the variables as written, so one
prompt serves every agent, and `branch`, `date` and `clipboard` are current
each time.

Values SHALL be inserted literally; a value that itself contains `{{` SHALL
NOT be expanded again.

The Swift app has no prompt variables.

#### Scenario: Variables expand for the receiving agent

- **WHEN** the prompt `Work in {{folder.name}} on {{branch}}` is sent to an
  agent in `/src/knot` whose folder is on branch `471-restart`
- **THEN** the text sent is `Work in knot on 471-restart`

#### Scenario: One library prompt adapts per agent

- **WHEN** two agents in different folders both reference a library prompt
  that uses `{{folder}}`
- **THEN** each is sent its own folder's path

#### Scenario: Whitespace inside the braces is allowed

- **WHEN** the text holds `{{ agent.name }}` and the agent is named "Knot 3"
- **THEN** it expands to `Knot 3`

#### Scenario: Branch outside a repository is empty

- **WHEN** the text uses `{{branch}}` and the agent's folder is not in a git
  repository
- **THEN** `{{branch}}` expands to the empty string

#### Scenario: A value is not expanded twice

- **WHEN** an agent is named `{{date}}` and the text is `{{agent.name}}`
- **THEN** the expanded text is the literal `{{date}}`

#### Scenario: Clipboard text is inserted

- **WHEN** the clipboard holds `panic at main.rs:12` and the user inserts the
  library prompt `Explain: {{clipboard}}` into a composer
- **THEN** the composer receives `Explain: panic at main.rs:12`

#### Scenario: Clipboard braces are not expanded

- **WHEN** the clipboard holds `{{branch}}` and an inserted prompt uses
  `{{clipboard}}`
- **THEN** the literal `{{branch}}` is inserted

## ADDED Requirements

### Requirement: The clipboard is captured when a prompt is chosen

`{{clipboard}}` SHALL be read from the system clipboard once, when the user
chooses the prompt to insert, before any other variable is resolved. Later
clipboard changes SHALL NOT affect the inserted text. Only text SHALL be
used. A clipboard holding no text SHALL expand to the empty string.

#### Scenario: Clipboard holds an image

- **WHEN** the clipboard holds only an image and the user inserts a prompt
  using `{{clipboard}}`
- **THEN** `{{clipboard}}` expands to the empty string

#### Scenario: Clipboard changes during expansion

- **WHEN** the user chooses a prompt using `{{clipboard}}` while the
  clipboard holds `one`, then copies `two` before expansion completes
- **THEN** the inserted text holds `one`

### Requirement: Startup prompts never read the clipboard

A startup prompt is sent without the user choosing it. In a startup prompt,
`{{clipboard}}` SHALL expand to the empty string, and the clipboard SHALL NOT
be read. The agent editor's custom startup-prompt field SHALL warn that
`{{clipboard}}` is always empty there, and the warning SHALL NOT block save.

#### Scenario: Startup prompt does not read the clipboard

- **WHEN** an agent's custom startup prompt is `Context: {{clipboard}}` and
  the clipboard holds a secret
- **THEN** the agent is sent `Context: ` and the secret is not sent

#### Scenario: Startup prompt editor warns

- **WHEN** the user types `{{clipboard}}` into the agent editor's custom
  startup-prompt field
- **THEN** a warning says the clipboard is empty in startup prompts, and save
  remains enabled

### Requirement: The composer's Insert Prompt list

A panel agent's composer, but not a shell companion's, SHALL offer an Insert
Prompt control in its control bar. It SHALL list the library's prompts in
library order, by name with a one-line preview, filtered as the user types
to those whose name or text contains the typed text. Keys SHALL behave as in
the `/` lookup. An empty library SHALL offer to open the Prompts window.

#### Scenario: Filtering

- **WHEN** the library holds "Review" and "Run the gate" and the user types
  `gate` in the picker
- **THEN** only "Run the gate" is listed

#### Scenario: Empty library

- **WHEN** the library holds no prompts and the user opens Insert Prompt
- **THEN** the list says there are no saved prompts and offers to open the
  Prompts window

#### Scenario: Dismissing changes nothing

- **WHEN** the user opens Insert Prompt and presses Esc
- **THEN** the composer's text and selection are unchanged and it has focus

### Requirement: Choosing a prompt from the Insert Prompt list

Choosing a prompt SHALL insert its text at the composer's caret, or replace
the selection if there is one, with variables expanded for that composer's
agent. The rest of the buffer SHALL be left untouched. Expansion SHALL follow
"Inserting a library prompt expands its text" (`panel-slash-commands`).
Nothing SHALL be sent, and focus SHALL return to the composer with the caret
after the inserted text.

#### Scenario: Insert at the caret

- **WHEN** the composer holds `Please ` with the caret at the end, and the
  user opens Insert Prompt and chooses "Review" whose text is
  `review {{branch}}` on branch `main`
- **THEN** the composer holds `Please review main`, the caret is at the end,
  and nothing is sent

#### Scenario: Replace a selection

- **WHEN** `TODO` is selected in the composer and the user chooses a prompt
  whose text is `run the tests`
- **THEN** `TODO` is replaced by `run the tests`
