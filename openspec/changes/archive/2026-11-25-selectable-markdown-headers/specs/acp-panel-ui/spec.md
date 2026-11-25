## MODIFIED Requirements

### Requirement: Rendered Markdown separates body from headers by font

Every Markdown surface — the panel's assistant messages and the Markdown pane
opened for an agent — SHALL draw body text in the configured UI font and
headers (`#` through `######`) in the configured title font. Body text is
everything but a header. Inline code and code blocks SHALL keep the monospace
family. A header SHALL keep the size and weight its level gives it, and its
inline marks SHALL render as in body text, in the title font. Changing either
font in settings SHALL change the corresponding text without reopening the
surface.

#### Scenario: A response with a header and a paragraph

- **WHEN** an agent's response contains a `##` header followed by a paragraph
- **THEN** the header is drawn in the title font and the paragraph in the UI
  font

#### Scenario: Body constructs stay in the UI font

- **WHEN** a response contains a list, a table and a block quote
- **THEN** each is drawn in the UI font, and none of them in the title font

#### Scenario: Code keeps its own family

- **WHEN** a response contains a fenced code block and an inline code span
- **THEN** both are drawn in the monospace family, and the words around the
  inline span are drawn in the UI font

#### Scenario: A header keeps its level's size and weight

- **WHEN** a response contains an `#` header and a `###` header
- **THEN** both are drawn in the title font, at the sizes and weights their
  levels had before the two faces were split

#### Scenario: A header containing a mark

- **WHEN** a response contains the header `## A **bold** word`
- **THEN** the header line reads "A bold word" in the title font, with "bold"
  drawn as a bold run and no asterisks shown

#### Scenario: The Markdown pane follows the same split

- **WHEN** a Markdown file shown for an agent contains headers and paragraphs
- **THEN** its headers are drawn in the title font and its body in the UI font

#### Scenario: Changing the UI font redraws body text

- **WHEN** the user picks a new UI font family while a response with headers
  and paragraphs is on screen
- **THEN** the body text is redrawn in the new family and the headers are
  unchanged

## ADDED Requirements

### Requirement: Header text is selectable like body text

On every Markdown surface, header text SHALL be selectable and copyable the
same way as body text: a drag may start, end or pass through a header, and the
copy SHALL contain the header's words in reading order with the text around
them. Every other block a response draws (list items, block quotes, table
cells, code blocks) SHALL stay selectable.

#### Scenario: A selection that starts on a header

- **WHEN** the user drags from a `##` header into the paragraph after it and
  copies
- **THEN** the copy holds the header's words followed by the paragraph's

#### Scenario: A selection across a header

- **WHEN** the user drags from a paragraph, across a header, into the next
  paragraph and copies
- **THEN** the copy holds both paragraphs and the header between them, in that
  order
