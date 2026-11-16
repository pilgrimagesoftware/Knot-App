# Spec Delta

## MODIFIED Requirements

### Requirement: The markdown section can be approved or reviewed

While a markdown section is expanded, its header SHALL offer the review
verdicts of the Swift reference's `MarkdownPanelView`, each with a localized
label and tooltip:

- While the user is reading the file, **Approve** and **Review**.
- **Approve** SHALL send the agent `Approved: <file>` as one submitted prompt,
  naming the file - relative to the agent's folder when it lies inside it, in
  full otherwise - so the approval cannot be mistaken for agreement to
  something else pending (#30).
- **Review** SHALL start an unsent reply naming the file -
  `While reviewing <file>, user made the following comments:` followed by a new
  line - and then offer **Submit Review** in place of the two. For a Panel-mode
  agent, **Approve** SHALL stay on offer beside it: approving then sends
  `Approved: <file>, with these review comments:` followed by what the user
  wrote after the reply's opening line, and leaves in the composer any draft
  that was there before the review began.
- **Submit Review** SHALL send the reply with whatever the user added to it.
- Once a verdict is sent, none SHALL be offered until the file is read again.
  The agent revising the file is what reopens the question.

Each verdict SHALL reach the agent the way the user typing it would. For a
Panel-mode agent, Approve is delivered as a prompt, queued behind a running turn
as any other prompt is. Review appends the reply to the composer, keeping a
draft the user had begun, and focuses it. Submit Review is the composer's own
send. For a Terminal-mode agent, the text is typed at the prompt and a
submission is Escape then Return, with the Swift reference's delays between
them, so an agent CLI's autocomplete does not take the Return.

The messages are sent to the agent rather than shown as chrome, so they SHALL
NOT be localized.

#### Scenario: Approving a plan

- **WHEN** an idle Panel-mode agent has a markdown file open and the user
  activates Approve
- **THEN** the agent receives `Approved: <file>` as a prompt, and the section
  offers no verdict until the file is read again

#### Scenario: Reviewing with comments

- **WHEN** the user activates Review, types a comment in the composer, and
  activates Submit Review
- **THEN** the agent receives the reply naming the file followed by the comment

#### Scenario: Approving with review comments

- **WHEN** a Panel-mode agent has `plan.md` open in its folder, the user
  activates Review, types `keep step 2 short`, and activates Approve
- **THEN** the agent receives `Approved: plan.md, with these review comments:`
  followed by `keep step 2 short`
