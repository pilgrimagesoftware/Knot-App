# Spec Delta

## ADDED Requirements

### Requirement: Markdown tables size their columns from their content

Every Markdown surface SHALL size a table's columns from what they hold: each
column gets at least its min-content width (its widest piece of text that
cannot wrap) whenever the table's width allows every column its own, and the
room beyond those widths goes to the columns that would be wider, in
proportion to how much wider. Every row SHALL use the same column widths.

#### Scenario: A column of short words beside a column of prose

- **WHEN** a response contains a table whose first column holds single words,
  such as "Observability", and whose second column holds long sentences
- **THEN** no word in the first column breaks, and the second column narrows
  and wraps its sentences instead

#### Scenario: Columns line up

- **WHEN** a table's rows hold cells of different lengths
- **THEN** every row's cells in a column start at the same position and have
  the same width

### Requirement: Markdown table cells break words only as a last resort

Table cell text SHALL wrap at whitespace and after a hyphen that joins two
words. A word SHALL break only when the columns' min-content widths together
do not fit the table's width.

#### Scenario: Wrapping prefers spaces and hyphens

- **WHEN** a cell's text is wider than its column
- **THEN** it wraps at a space or after a hyphen joining two words, not inside
  a word

#### Scenario: A table too narrow for its words

- **WHEN** a table's frame is narrower than its columns' min-content widths
  together
- **THEN** the columns shrink to fit the frame and words break, rather than
  the table overflowing
