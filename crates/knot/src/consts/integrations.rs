//! External-service constants: the help page, Codex usage, adapter update
//! checks and the Knot-Library submit URL.

use std::time::Duration;

/// Where Help > Knot Help goes: the app's page, which is also the
/// repository's listed homepage. There is no bundled help book.
pub(crate) const KNOT_HELP_URL: &str = "https://pilgrimagesoftware.com/apps/knot";

/// How much of a Codex usage window must be used before an agent's pane says
/// it is approaching the limit (#291). Codex's own CLI warns from here up.
pub(crate) const CODEX_USAGE_NEAR_LIMIT_PERCENT: f64 = 75.0;

/// How much of the end of a Codex session file is read for its latest usage
/// snapshot. Snapshots come with every model response, so the newest is near
/// the end; a long session's file runs to megabytes.
pub(crate) const CODEX_USAGE_TAIL_BYTES: u64 = 256 * 1024;

/// How often Codex's usage is re-read while a Codex agent is in the window.
/// A file listing and a tail read, off the main thread.
pub(crate) const CODEX_USAGE_MAX_AGE: Duration = Duration::from_secs(60);

/// Where Codex's usage can be extended, as its own limit message names it.
pub(crate) const CODEX_USAGE_URL: &str = "https://chatgpt.com/codex/settings/usage";

/// How long after launch the first background adapter update check runs
/// (#367): late enough to stay out of startup's way.
pub(crate) const ADAPTER_UPDATE_FIRST_CHECK_DELAY: Duration = Duration::from_secs(60);
/// How often adapters are checked for updates after that.
pub(crate) const ADAPTER_UPDATE_CHECK_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);
/// An adapter update notification's tag, before the agent type. Not an
/// agent id, so a click only brings Knot forward.
pub(crate) const ADAPTER_UPDATE_NOTIFICATION_TAG_PREFIX: &str = "adapter-update:";

/// The longest Knot-Library new-file address handed to the browser (#43).
/// GitHub refuses a much longer one, so past this the file goes to the
/// clipboard and the page opens empty.
pub(crate) const LIBRARY_SUBMIT_URL_MAX: usize = 8000;

/// The Keychain service the MCP client token is stored under
/// (`openspec/changes/mcp-auth/design.md` D1).
#[cfg_attr(not(target_os = "macos"), allow(dead_code))] // No Keychain elsewhere.
pub(crate) const MCP_CLIENT_TOKEN_SERVICE: &str = "com.pilgrimagesoftware.knot.mcp-client-token";

/// `errSecItemNotFound`: the one Keychain read error that means there is no
/// token yet. Every other error leaves the stored token alone.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))] // No Keychain elsewhere.
pub(crate) const KEYCHAIN_ITEM_NOT_FOUND: i32 = -25300;

/// The account name of that Keychain item. One token per user.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))] // No Keychain elsewhere.
pub(crate) const MCP_CLIENT_TOKEN_ACCOUNT: &str = "client-token";

/// The environment variable carrying an agent's hook credential to its
/// adapter's hook scripts (D3, D6). The only agent credential in an
/// environment.
pub(crate) const MCP_HOOK_TOKEN_ENV: &str = "KNOT_MCP_TOKEN";

/// The environment variable carrying the MCP server's base URL to the same
/// scripts.
pub(crate) const MCP_HOOK_URL_ENV: &str = "KNOT_MCP_URL";

/// The environment variable naming the agent to the same scripts, which
/// send it as the hook body's `agent_id`. Not a secret.
pub(crate) const MCP_HOOK_AGENT_ENV: &str = "KNOT_AGENT_ID";
