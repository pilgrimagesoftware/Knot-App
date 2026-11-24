# Tasks

## 1. Library

- [x] 1.1 `Settings::duplicate_prompt`: new id, every other field copied,
      inserted after the original, persisted; unknown id changes nothing
- [x] 1.2 `copy_name`: "`<name>` copy", then numbered from 2 past taken names,
      through the catalogue

## 2. UI

- [x] 2.1 Duplicate action on Prompts window rows
- [x] 2.2 Duplicate button on Insert Prompt rows, closing the list and leaving
      the composer alone
- [x] 2.3 The prompt editor opens on the copy with the caret in its name

## 3. Tests and docs

- [x] 3.1 Store: order, fields copied, persisted, repeated copies get distinct
      names, unknown id writes nothing
- [x] 3.2 `copy_name` numbering and catalogue keys
- [x] 3.3 Prompts window and Insert Prompt list duplicate, editor caret
- [x] 3.4 `docs/site/latest/personas-prompts.md`
