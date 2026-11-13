//! The crate's fixed values, in one place.

/// Arguments for the user's own shell in an agent terminal: interactive, so
/// it reads the user's rc files and the agent gets their `PATH`, aliases and
/// tools - the same environment a terminal they opened themselves would have.
pub(crate) const USER_SHELL_ARGS: &[&str] = &["-i"];

/// Set in every agent terminal's environment, to [`KNOT_AGENT_VALUE`], to
/// mark the shell as a Knot agent's rather than one the user opened.
///
/// Dotfiles can test it to skip what only a human at a prompt needs. The
/// case that prompted it: powerlevel10k starts a gitstatusd daemon (and two
/// helper shells) per interactive shell, so a workspace of agents ran one
/// per agent - for a prompt segment no agent reads, while Knot tracks git
/// itself. Knot cannot turn that off from outside: the config p10k generates
/// begins by unsetting every `POWERLEVEL9K_*` variable it inherits. The user
/// opts out after that line instead;
/// `docs/site/latest/reference/agent-shells.md` has the snippet.
pub(crate) const KNOT_AGENT_ENV: &str = "KNOT_AGENT";

/// [`KNOT_AGENT_ENV`]'s value: any non-empty one would do for `[[ -n ]]`.
pub(crate) const KNOT_AGENT_VALUE: &str = "1";

/// The terminal type every agent terminal declares, set whatever `TERM` Knot
/// itself inherited (issue #271).
///
/// Launched from Finder, Knot has no `TERM` at all, and shells, `clear`,
/// pagers and full-screen tools then refuse or degrade ("TERM environment
/// variable not set."). Launched from a terminal, it inherits *that*
/// terminal's type - `tmux-256color`, say - which describes an emulator the
/// agent is not attached to. The emulator here is gpui-terminal's alacritty
/// grid, which implements xterm's sequences and 256-colour palette.
pub(crate) const AGENT_TERM: &str = "xterm-256color";

/// The `COLORTERM` every agent terminal declares: gpui-terminal draws 24-bit
/// colour (`Color::Spec`), and tools only use it when this says so.
pub(crate) const AGENT_COLORTERM: &str = "truecolor";

/// The notices, lower-case, that open a line of Claude Code's output when
/// its session hits a usage limit (#36) without naming the limit: the
/// wrap-up allowance it may grant first ("Usage limit reached · wrapping
/// up"). Matched at the start of a line and followed by nothing or by
/// [`SESSION_LIMIT_SEPARATORS`], so an agent writing about limits does not
/// stop itself. The hard stop names its limit - "You've hit your 5-hour
/// limit" - and is recognised by
/// `knot_agents::SessionLimit::opens_with_named_limit` instead, whatever
/// the limit is called (#397).
pub(crate) const SESSION_LIMIT_NOTICES: &[&str] = &["usage limit reached"];

/// Longest line the session-limit scan reads, in bytes. The notices are a
/// short status line; a longer line is prose and is skipped unread, which
/// also bounds what the scan buffers between chunks.
pub(crate) const SESSION_LIMIT_MAX_LINE: usize = 256;

/// What may follow a notice's phrase on its line: Claude Code's "·" before
/// the reset time or "wrapping up", or a sentence's end.
pub(crate) const SESSION_LIMIT_SEPARATORS: &[char] =
    &['\u{b7}', '.', ',', '\u{2014}', '-', '(', '!'];
