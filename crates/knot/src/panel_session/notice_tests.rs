//! A launch's notices reach the conversation before the session goes live
//! (`mcp-auth` task 4.2): the command-line delivery notice is the first
//! thing the panel shows, not something that arrives after the first turn.

use std::collections::BTreeMap;

use knot_agent_launch::AdapterConfig;

use super::*;

/// Completes the handshake and opens a session; nothing more is asked of
/// it, since no prompt is sent.
fn quiet_adapter() -> AdapterConfig {
    AdapterConfig { command:                   "sh",
                    args:                      &[
                                                 "-c",
                                                 r#"while IFS= read -r line; do
                      id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
                      method=$(echo "$line" | sed -nE 's/.*"method":"([^"]+)".*/\1/p')
                      case "$method" in
                        initialize) echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"protocolVersion\":1,\"capabilities\":{}}}" ;;
                        session/new) echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"sessionId\":\"sess-1\"}}" ;;
                      esac
                    done"#,
    ],
                    supports_resume:           false,
                    supports_permission_modes: false,
                    install:                   None, }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_launch_notice_is_in_the_conversation_when_the_slot_goes_live() {
    let (connecting, progress) = PanelSessionSlot::connecting();
    let slot = Arc::new(Mutex::new(connecting));
    let notice = "the credential travels on the command line".to_owned();
    let request = ConnectRequest { agent_id:                   uuid::Uuid::nil(),
                                   config:                     &quiet_adapter(),
                                   cwd:                        "/tmp/project",
                                   prior_session_id:           None,
                                   mcp:                        None,
                                   registration_prompt:        None,
                                   resume_registration_prompt: None,
                                   session_config:             BTreeMap::new(),
                                   default_mode:               None,
                                   option_mode:                None,
                                   session_meta:               None,
                                   env:                        Vec::new(),
                                   args:                       Vec::new(),
                                   subagents:                  None,
                                   startup_prompt:             None,
                                   times:                      None,
                                   notices:                    vec![notice.clone()], };

    connect_into(&slot, request, &progress, |_| {}).await;

    let (state, session) = match &*slot.lock() {
        PanelSessionSlot::Ready(handle) => (handle.state(), handle.session()),
        _ => panic!("the session did not connect"),
    };
    let shown = state.lock()
                     .messages
                     .iter()
                     .any(|message| {
                         matches!(message, crate::panel_state::PanelMessage::Notice(text) if *text == notice)
                     });
    session.stop().await;
    assert!(shown, "the notice is in the conversation");
}
