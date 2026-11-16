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

Table cell text SHALL wrap where the panel's other text wraps: at spaces, and
before characters the text engine does not keep inside a word, such as `/`. A
column's min-content width SHALL be measured to those same break points, so a
hyphenated word such as "Short-term" counts as one word. A word SHALL break
only when the columns' min-content widths together do not fit the table's
width.

#### Scenario: Wrapping prefers spaces

- **WHEN** a cell's text is wider than its column
- **THEN** it wraps at a space, not inside a word

#### Scenario: A hyphenated word stays whole

- **WHEN** a column's widest word is hyphenated, such as "Short-term", and the
  column beside it has room to give
- **THEN** the column is wide enough for the whole word, and the word does not
  break at or after its hyphen

#### Scenario: A table too narrow for its words

- **WHEN** a table's frame is narrower than its columns' min-content widths
  together
- **THEN** the columns shrink to fit the frame and words break, rather than
  the table overflowing
