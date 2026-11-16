# FuzzyScorer snapshot

The corpus and snapshot now live beside the port, in
`crates/knot/src/fuzzy_score/`; this directory keeps the generator.

The ordering the port must reproduce, produced by the Swift reference's own
scorer rather than reconstructed from it.

- `corpus.txt`: `git ls-files crates` at Knot-App `7a68acce`, minus test
  fixtures; 952 paths.
- `snapshot.jsonl`: for each query, the top 50 matches by `scoreFile`, one
  JSON object per line: `query`, `path`, `score`, `indices` (character
  offsets into `path`). Ties are broken by path ascending; the reference
  sorted with Swift's unstable `sort`, so its tie order was undefined.
- `generate.swift`: the driver. The scorer itself is
  `Skwad/Utilities/FuzzyScorer.swift` at `d0a84963^`, the commit before the
  Swift tree was removed.

Regenerate:

```sh
mkdir -p /tmp/fz && git show d0a84963^:Skwad/Utilities/FuzzyScorer.swift > /tmp/fz/FuzzyScorer.swift
swiftc -O -o /tmp/fz/gen /tmp/fz/FuzzyScorer.swift reference/generate.swift
/tmp/fz/gen crates/knot/src/fuzzy_score/corpus.txt lib kgs mod readme WsWin spec.md agentrow Cargo \
  fuzzy panelstate consts tests.rs ACP knot-git/src > crates/knot/src/fuzzy_score/snapshot.jsonl
```
