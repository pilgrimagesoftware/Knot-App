---
paths:
  - "crates/**/*.rs"
---

### SCAR-KEYS-01: Bind and simulate `secondary-`, not `cmd-`
- **Wound**: #472's tests sent a literal `cmd-enter`. On Linux, `cmd` is Super, so the keystroke matched nothing: 3 tests failed only on the ubuntu CI job, and a fourth passed there without testing anything. `question_card.rs` has the same bug in product code (#482).
- **Trap**: "The app is macOS-first, and Cmd-Return is the shortcut, so write `cmd-enter`." gpui-kit binds `secondary-…`, which is Cmd on macOS and Ctrl elsewhere, so a literal `cmd-` only matches on a Mac.
- **Circuit breaker**: If a binding or a simulated keystroke uses `cmd-` for a cross-platform action, you may switch it to `secondary-` even when the brief says "Cmd".
- **Reflex**:
  1. For any cross-platform shortcut, bind `secondary-…` in `KeyBinding::new`. Use `cmd-` only for macOS-only actions such as Hide and Hide Others.
  2. In tests, simulate exactly what the binding declares.
  3. Check it by simulating the other platform's modifier (`ctrl-…` on macOS). The test must fail.
