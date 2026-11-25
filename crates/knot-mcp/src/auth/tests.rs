//! The registry's contract (tasks 1.1): issue, authenticate, revoke and
//! rotate, and that no credential is ever printed.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use uuid::Uuid;

use super::*;
use crate::consts;

#[test]
fn an_issued_pair_proves_its_agent_and_kind() {
    let registry = CredentialRegistry::new();
    let agent = Uuid::new_v4();
    let pair = registry.issue_agent_pair(agent);

    assert_eq!(registry.authenticate(pair.tool.expose()),
               Some(Principal::Agent(agent)));
    assert_eq!(registry.authenticate(pair.hooks.expose()),
               Some(Principal::AgentHooks(agent)));
    assert!(registry.has_agent_pair(agent));
}

#[test]
fn credentials_carry_their_kind_and_256_bits() {
    for (kind, prefix) in [(CredentialKind::AgentTool, consts::AGENT_TOOL_PREFIX),
                           (CredentialKind::AgentHooks, consts::AGENT_HOOK_PREFIX),
                           (CredentialKind::Client, consts::CLIENT_TOKEN_PREFIX)]
    {
        let secret = Secret::generate(kind);
        let encoded = secret.expose()
                            .strip_prefix(prefix)
                            .expect("the kind's prefix");
        // 32 bytes in unpadded base64url.
        assert_eq!(encoded.len(), 43, "{kind:?}");
        assert_ne!(secret, Secret::generate(kind), "two draws differ");
    }
}

#[test]
fn an_unknown_credential_and_a_wrong_prefix_prove_nothing() {
    let registry = CredentialRegistry::new();
    let pair = registry.issue_agent_pair(Uuid::new_v4());
    let swapped = pair.tool
                      .expose()
                      .replacen(consts::AGENT_TOOL_PREFIX, consts::AGENT_HOOK_PREFIX, 1);

    assert_eq!(registry.authenticate(&swapped), None);
    assert_eq!(registry.authenticate(Secret::generate(CredentialKind::AgentTool).expose()),
               None);
    assert_eq!(registry.authenticate(""), None);
}

#[test]
fn revoking_an_agent_refuses_both_its_credentials() {
    let registry = CredentialRegistry::new();
    let (agent, other) = (Uuid::new_v4(), Uuid::new_v4());
    let pair = registry.issue_agent_pair(agent);
    let kept = registry.issue_agent_pair(other);

    registry.revoke_agent(agent);

    assert_eq!(registry.authenticate(pair.tool.expose()), None);
    assert_eq!(registry.authenticate(pair.hooks.expose()), None);
    assert!(!registry.has_agent_pair(agent));
    assert_eq!(registry.authenticate(kept.tool.expose()),
               Some(Principal::Agent(other)));
}

#[test]
fn rotating_refuses_the_old_pair_and_accepts_the_new() {
    let registry = CredentialRegistry::new();
    let agent = Uuid::new_v4();
    let old = registry.issue_agent_pair(agent);
    let new = registry.issue_agent_pair(agent);

    assert_eq!(registry.authenticate(old.tool.expose()), None);
    assert_eq!(registry.authenticate(old.hooks.expose()), None);
    assert_eq!(registry.authenticate(new.tool.expose()),
               Some(Principal::Agent(agent)));
    assert_eq!(registry.entries_for(agent), 2);
}

#[test]
fn no_instant_of_a_rotation_holds_both_pairs_or_neither() {
    let registry = CredentialRegistry::new();
    let agent = Uuid::new_v4();
    registry.issue_agent_pair(agent);
    let done = Arc::new(AtomicBool::new(false));

    let watcher = {
        let (registry, done) = (registry.clone(), Arc::clone(&done));
        std::thread::spawn(move || {
            let mut observed = 0_u32;
            while !done.load(Ordering::Relaxed) {
                assert_eq!(registry.entries_for(agent), 2, "mid-rotation state seen");
                observed += 1;
            }
            observed
        })
    };
    let mut previous = registry.issue_agent_pair(agent);
    for _ in 0..2_000 {
        let next = registry.issue_agent_pair(agent);
        assert_eq!(registry.authenticate(previous.tool.expose()), None);
        assert_eq!(registry.authenticate(next.tool.expose()),
                   Some(Principal::Agent(agent)));
        previous = next;
    }
    done.store(true, Ordering::Relaxed);
    assert!(watcher.join().expect("the watcher finished") > 0);
}

#[test]
fn a_new_client_token_revokes_the_previous_one() {
    let registry = CredentialRegistry::new();
    let first = Secret::generate(CredentialKind::Client);
    let second = Secret::generate(CredentialKind::Client);
    registry.set_client_token(&first);
    registry.set_client_token(&second);

    assert_eq!(registry.authenticate(first.expose()), None);
    assert_eq!(registry.authenticate(second.expose()),
               Some(Principal::External));
}

#[test]
fn a_stored_client_token_is_trusted_only_in_the_generated_shape() {
    let token = Secret::generate(CredentialKind::Client);
    assert_eq!(Secret::from_stored_client_token(token.expose()),
               Some(token));

    let agent = Secret::generate(CredentialKind::AgentTool);
    for damaged in [agent.expose(), "knot_c_", "knot_c_short", "knot_c_!!!!", ""] {
        assert_eq!(Secret::from_stored_client_token(damaged), None, "{damaged}");
    }
}

#[test]
fn no_debug_output_shows_a_credential() {
    let pair = CredentialRegistry::new().issue_agent_pair(Uuid::new_v4());
    let printed = format!("{pair:?} {:?}", Secret::generate(CredentialKind::Client));

    for prefix in [consts::AGENT_TOOL_PREFIX,
                   consts::AGENT_HOOK_PREFIX,
                   consts::CLIENT_TOKEN_PREFIX]
    {
        assert!(!printed.contains(prefix), "{printed}");
    }
}

#[test]
fn each_principal_reaches_only_its_routes() {
    let agent = Uuid::new_v4();
    let routes = [Route::Info, Route::Mcp, Route::StatusRead, Route::Hook];
    let reach = |principal: Principal| -> Vec<Route> {
        routes.into_iter()
              .filter(|route| principal.may_reach(*route))
              .collect()
    };

    assert_eq!(reach(Principal::Agent(agent)),
               [Route::Mcp, Route::StatusRead]);
    assert_eq!(reach(Principal::AgentHooks(agent)), [Route::Hook]);
    assert_eq!(reach(Principal::External), routes);
}

#[test]
fn a_refusal_is_counted_against_the_hint_then_the_user_agent() {
    let log = RefusalLog::new();
    let agent = Uuid::new_v4();
    log.record(RefusalSource::of(Some(agent), Some("claude-code/2.0")),
               RefusalKind::Request);
    log.record(RefusalSource::of(None, Some("claude-code/2.0")),
               RefusalKind::Request);
    log.record(RefusalSource::of(None, Some("curl knot_c_abc")),
               RefusalKind::Request);
    log.record(RefusalSource::of(None, None), RefusalKind::Request);

    assert_eq!(log.refused_for(agent), 1);
    assert_eq!(log.total(), 4);
    let sources: Vec<RefusalSource> = log.snapshot().into_iter().map(|(s, _)| s).collect();
    assert!(sources.contains(&RefusalSource::UserAgent("claude-code/2.0".into())));
    assert!(sources.iter()
                   .all(|source| !format!("{source:?}").contains("knot_c_")),
            "a credential-shaped User-Agent is not kept: {sources:?}");
}

#[test]
fn the_refusal_table_stops_growing_at_its_cap() {
    let log = RefusalLog::new();
    for index in 0..consts::REFUSAL_SOURCES_MAX + 10 {
        log.record(RefusalSource::of(None, Some(&format!("client-{index}"))),
                   RefusalKind::Request);
    }
    let snapshot = log.snapshot();

    assert_eq!(snapshot.len(), consts::REFUSAL_SOURCES_MAX + 1);
    assert_eq!(log.total(), (consts::REFUSAL_SOURCES_MAX + 10) as u64);
    assert!(snapshot.contains(&(RefusalSource::Other, 10)));
}

#[test]
fn refusals_older_than_the_window_are_not_counted() {
    let log = RefusalLog::new();
    let start = std::time::Instant::now();
    let source = RefusalSource::of(None, Some("old-client/1.0"));
    log.record_for_test(source.clone(), RefusalKind::Request, start);
    log.record_for_test(source.clone(),
                        RefusalKind::Request,
                        start + std::time::Duration::from_secs(30 * 60));

    let later = start + consts::REFUSAL_WINDOW + std::time::Duration::from_secs(60);
    assert_eq!(log.snapshot_for_test(later), [(source, 1)]);
    let much_later = later + consts::REFUSAL_WINDOW;
    assert!(log.snapshot_for_test(much_later).is_empty(),
            "the table empties out");
}

/// D10 and the security review of #521: an agent's tools read as
/// unavailable after a refused request, until it next authenticates - and a
/// refused SSE stream alone (agy's header-less GET) never says so.
#[test]
fn tools_read_unavailable_from_a_refused_request_until_the_next_success() {
    let log = RefusalLog::new();
    let agent = Uuid::new_v4();
    let named = || RefusalSource::of(Some(agent), None);

    log.record(named(), RefusalKind::Stream);
    assert!(!log.tools_unavailable(agent),
            "a refused stream alone is not a lost tool");

    log.record(named(), RefusalKind::Request);
    assert!(log.tools_unavailable(agent));

    log.authenticated(agent);
    assert!(!log.tools_unavailable(agent));
    assert_eq!(log.refused_for(agent),
               2,
               "the hour's count is kept for the MCP tab");

    log.record(named(), RefusalKind::Request);
    assert!(log.tools_unavailable(agent),
            "dropping the header after a good start is still caught");
    log.forget(agent);
    assert!(!log.tools_unavailable(agent));
}
