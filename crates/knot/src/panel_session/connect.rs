//! One connect attempt: what it needs ([`ConnectRequest`]) and the lifecycle
//! that drives a slot from connecting to ready or failed ([`connect_into`]).
//! Split from the session handle, which is what a connected session *is*;
//! this is how one comes to be.

use std::collections::BTreeMap;
use std::sync::Arc;

use knot_agent_launch::{AdapterConfig, ContextSource, DefaultMode};
use knot_terminal::SessionTarget;
use parking_lot::Mutex;

use super::{ConnectProgress, PanelSessionHandle, PanelSessionSlot, TimesStore, default_mode};
use crate::subagent_feed::SubagentSink;

/// What one panel connect attempt needs beyond the slot it publishes
/// into: the adapter to spawn, where to run it, and the first prompt to
/// send once it is live.
pub struct ConnectRequest<'a> {
    pub config:                     &'a AdapterConfig,
    pub cwd:                        &'a str,
    pub prior_session_id:           Option<&'a str>,
    pub mcp_url:                    Option<&'a str>,
    /// The registration prompt a fresh session opens on. Built whether or
    /// not a prior session is named: only the adapter can say whether it
    /// resumed, so [`connect_into`] withholds this once it has - a resumed
    /// agent is already registered - and sends it on the fresh session an
    /// adapter falls back to when it cannot load.
    pub registration_prompt:        Option<String>,
    /// The first turn instead, when the session resumed: the one-line
    /// registration request for an adapter whose connection cannot register
    /// the agent, `None` for one whose can (#552). See
    /// `knot_agent_launch::resume_registration_prompt`.
    pub resume_registration_prompt: Option<String>,
    /// The agent's persisted session setup - config-option id -> value -
    /// replayed onto the new session before its first turn. Empty for an
    /// agent that has never had one chosen, which leaves the adapter's own
    /// defaults in place. See
    /// `openspec/specs/session-setup-persistence/spec.md`.
    pub session_config:             BTreeMap<String, String>,
    /// The mode to select when `session_config` chose none, for an adapter
    /// whose own default differs from its CLI's; see
    /// `knot_agent_launch::unconfigured_default_mode`.
    pub default_mode:               Option<DefaultMode>,
    /// The mode the agent's user options ask for, which wins over both of
    /// the above; see `knot_agent_launch::adapter_options`.
    pub option_mode:                Option<DefaultMode>,
    /// The `_meta` the session opens with, carrying the agent's user
    /// options to an adapter that takes them there, and the standing
    /// instructions to one whose carrier is the session's `_meta`.
    pub session_meta:               Option<serde_json::Value>,
    /// Extra environment for the adapter subprocess: the standing
    /// instructions, for an adapter whose carrier is its environment. See
    /// `knot_agent_launch::instruction_delivery`.
    pub env:                        Vec<(String, String)>,
    /// Arguments after the adapter's own: the agent's user options, for an
    /// adapter that takes them on its command line.
    pub args:                       Vec<String>,
    /// Where this agent's delegations are recorded, or `None` when its type
    /// reports none. Built by the caller, which is the only place that knows
    /// both the agent's id and the window's registry.
    pub subagents:                  Option<SubagentSink>,
    /// The startup prompt to expand and queue behind the registration turn;
    /// `None` when the agent has none. Withheld with the registration prompt
    /// when the session resumed.
    pub startup_prompt:             Option<StartupRequest>,
    /// Where the session's conversation times are kept, or `None` to keep
    /// none - see [`TimesStore`].
    pub times:                      Option<TimesStore>,
}

/// A startup prompt's raw text and what its variables expand with. The
/// branch is read inside `connect_into`, on the runtime rather than the UI
/// thread, because reading it runs a subprocess.
pub struct StartupRequest {
    pub text:    String,
    pub context: ContextSource,
}

/// Connects `request`'s adapter and drives `slot` through the connection
/// lifecycle, calling `on_session_id` with the opened session id so the
/// caller can persist it for a later resume.
pub async fn connect_into(slot: &Arc<Mutex<PanelSessionSlot>>, request: ConnectRequest<'_>,
                          progress: &ConnectProgress, on_session_id: impl FnOnce(&str)) {
    let target = SessionTarget { cwd:              request.cwd,
                                 prior_session_id: request.prior_session_id,
                                 mcp_url:          request.mcp_url,
                                 meta:             request.session_meta.as_ref(),
                                 env:              &request.env,
                                 args:             &request.args, };
    let mut handle = match PanelSessionHandle::start(request.config,
                                                     target,
                                                     progress,
                                                     request.subagents,
                                                     request.times).await
    {
        Ok(handle) => handle,
        Err(error) => {
            *slot.lock() = PanelSessionSlot::Failed(error.to_string());
            return;
        }
    };
    on_session_id(handle.session_id());
    // Decided on what happened, not on what was asked for: a prior session
    // that could not be loaded left a fresh one that nothing has registered.
    let resumed = handle.session().resumed();
    let (registration_prompt, startup_prompt) = if resumed {
        (request.resume_registration_prompt, None)
    }
    else {
        (request.registration_prompt, request.startup_prompt)
    };

    // Replay the persisted setup before the first turn, so the turn runs
    // with the model, permission mode and effort the user last chose. An
    // option the adapter no longer declares is rejected by it and skipped
    // here rather than failing the connection - adapters add and drop
    // config options between versions.
    let mut restored_options = None;
    for (config_id, value) in &request.session_config {
        if let Ok(options) = handle.session().set_config_option(config_id, value).await {
            restored_options = Some(options);
        }
    }
    // Then the mode the user's options ask for (#501), or else the CLI's
    // default where nothing above chose the option: the adapter would
    // otherwise start Claude in Manual where its CLI starts in Auto (#516).
    // Judged against the options the replay left, so "already on it" is
    // what the session is on now. A rejection leaves the mode as it is.
    let options = restored_options.clone()
                                  .unwrap_or_else(|| handle.state().lock().config_options.clone());
    if let Some(mode) = default_mode::mode_to_apply(request.option_mode,
                                                    request.default_mode,
                                                    &request.session_config,
                                                    &options)
       && let Ok(options) = handle.session()
                                  .set_config_option(mode.config_id, mode.value)
                                  .await
    {
        restored_options = Some(options);
    }
    if let Some(options) = restored_options {
        handle.state().lock().config_options = options;
    }

    // Publish the handle *before* sending the registration prompt.
    // `session/prompt` resolves only when the whole turn ends, and an
    // agent that asks permission during that turn (Gemini does, for its
    // first Knot MCP tool call) can only be answered through a `Ready`
    // slot - so awaiting the turn here first is a circular wait: the
    // turn needs a permission answer, the answer needs the slot, the
    // slot needs the turn. The registration prompt is recorded first so
    // the panel opens on the turn already in flight rather than blank.
    // Expanded before `Ready`, so the window only ever sees the handle with
    // its startup prompt already in place, and queues it behind a
    // registration turn that `record_user_message` has already marked
    // active.
    if let Some(startup) = startup_prompt {
        handle.startup_prompt = expand_startup(startup).await;
    }
    let session = handle.session();
    let recorder = handle.recorder();
    if let Some(prompt) = &registration_prompt {
        handle.record_user_message(prompt.clone());
    }
    *slot.lock() = PanelSessionSlot::Ready(Box::new(handle));

    if let Some(prompt) = registration_prompt
       && let Err(error) = session.prompt(&prompt).await
    {
        // Into the conversation, not just the console: the registration
        // turn is the first thing the panel shows, and an agent that
        // refuses it (an exhausted daily quota, say) otherwise leaves the
        // panel sitting on a prompt that never answers.
        recorder.error(knot_core::l10n::t_with("panel.error_first_turn",
                                               &[("error", &error.to_string())]));
        eprintln!("failed to send panel registration prompt: {error}");
    }
}

/// `startup`'s text with its variables expanded, reading the branch on a
/// blocking thread. `None` only if that thread panicked.
async fn expand_startup(startup: StartupRequest) -> Option<String> {
    tokio::task::spawn_blocking(move || {
        knot_agent_launch::expand(&startup.text,
                                  &knot_agent_launch::read_context(startup.context))
    }).await
      .ok()
}
