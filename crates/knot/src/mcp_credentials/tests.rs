//! The client token's lifecycle against an in-memory store (task 1.2).

use std::cell::RefCell;

use knot_mcp::{CredentialKind, CredentialRegistry, Principal, Secret};

use super::*;

#[derive(Default)]
struct MemoryStore {
    value:      RefCell<Option<String>>,
    fails:      bool,
    /// A read error other than "not found": denied, locked, re-signed.
    unreadable: bool,
}

impl ClientTokenStore for MemoryStore {
    fn load(&self) -> Result<Option<String>, String> {
        if self.unreadable {
            return Err("User interaction is not allowed.".to_owned());
        }
        Ok(self.value.borrow().clone())
    }

    fn save(&self, token: &Secret) -> Result<(), String> {
        if self.fails {
            return Err("locked".to_owned());
        }
        *self.value.borrow_mut() = Some(token.expose().to_owned());
        Ok(())
    }
}

#[test]
fn the_first_launch_creates_and_saves_a_token() {
    let (store, registry) = (MemoryStore::default(), CredentialRegistry::new());
    let token = load_or_create(&store, &registry).token;

    assert_eq!(store.load().unwrap().as_deref(), Some(token.expose()));
    assert_eq!(registry.authenticate(token.expose()),
               Some(Principal::External));
}

#[test]
fn a_later_launch_reuses_the_stored_token() {
    let store = MemoryStore::default();
    let first = load_or_create(&store, &CredentialRegistry::new()).token;
    let registry = CredentialRegistry::new();

    assert_eq!(load_or_create(&store, &registry).token, first);
    assert_eq!(registry.authenticate(first.expose()),
               Some(Principal::External));
}

#[test]
fn a_damaged_stored_token_is_replaced() {
    let store = MemoryStore::default();
    let agent_shaped = Secret::generate(CredentialKind::AgentTool);
    *store.value.borrow_mut() = Some(agent_shaped.expose().to_owned());
    let registry = CredentialRegistry::new();

    let token = load_or_create(&store, &registry).token;

    assert_ne!(token.expose(), agent_shaped.expose());
    assert_eq!(registry.authenticate(agent_shaped.expose()), None);
}

#[test]
fn regenerating_refuses_the_old_token_and_saves_the_new() {
    let (store, registry) = (MemoryStore::default(), CredentialRegistry::new());
    let old = load_or_create(&store, &registry).token;

    let new = regenerate(&store, &registry).token;

    assert_ne!(old, new);
    assert_eq!(registry.authenticate(old.expose()), None);
    assert_eq!(registry.authenticate(new.expose()),
               Some(Principal::External));
    assert_eq!(store.load().unwrap().as_deref(), Some(new.expose()));
}

#[test]
fn a_token_that_cannot_be_saved_still_serves_this_launch() {
    let store = MemoryStore { fails: true,
                              ..MemoryStore::default() };
    let registry = CredentialRegistry::new();

    let token = load_or_create(&store, &registry).token;

    assert_eq!(registry.authenticate(token.expose()),
               Some(Principal::External));
}

#[test]
fn issuing_rotates_and_revoking_ends_an_agents_pair() {
    let credentials = McpCredentials::default();
    let agent = uuid::Uuid::new_v4();
    let first = credentials.issue(agent);
    let second = credentials.issue(agent);

    assert_eq!(credentials.registry.authenticate(first.tool.expose()), None);
    assert_eq!(credentials.registry.authenticate(second.tool.expose()),
               Some(Principal::Agent(agent)));
    credentials.revoke(agent);
    assert_eq!(credentials.registry.authenticate(second.tool.expose()),
               None);
}

/// Security review of #521: a read error that is not "not found" must not
/// replace the stored token - every client registered with it would stop
/// working. This launch runs on a token it does not save, and says why.
#[test]
fn an_unreadable_store_keeps_its_token_and_this_launch_uses_an_unsaved_one() {
    let stored = Secret::generate(CredentialKind::Client);
    let store = MemoryStore { value: RefCell::new(Some(stored.expose().to_owned())),
                              unreadable: true,
                              ..MemoryStore::default() };
    let registry = CredentialRegistry::new();

    let loaded = load_or_create(&store, &registry);

    assert!(loaded.problem.is_some(),
            "the error is kept for the MCP tab");
    assert_ne!(loaded.token, stored);
    assert_eq!(store.value.borrow().as_deref(),
               Some(stored.expose()),
               "the stored token is left as it was");
    assert_eq!(registry.authenticate(loaded.token.expose()),
               Some(Principal::External),
               "this launch still serves its own clients");
}

#[test]
fn a_saved_token_reports_no_problem() {
    let (store, registry) = (MemoryStore::default(), CredentialRegistry::new());
    assert!(load_or_create(&store, &registry).problem.is_none());
    let failing = MemoryStore { fails: true,
                                ..MemoryStore::default() };
    assert!(load_or_create(&failing, &registry).problem.is_some(),
            "an unsaved token says so");
}

/// B3's review of #521: off macOS the token is kept in a file, so it does
/// not change every launch - and only its owner can read it.
#[test]
fn a_file_store_keeps_the_token_across_launches_for_its_owner_only() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let store = FileTokenStore { path: dir.path().join("nested/mcp-client-token"), };
    assert_eq!(store.load(), Ok(None), "absent before the first launch");

    let first = load_or_create(&store, &CredentialRegistry::new());
    assert!(first.problem.is_none());
    let again = load_or_create(&store, &CredentialRegistry::new());
    assert_eq!(again.token, first.token,
               "the next launch reads the same token");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&store.path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
}

#[test]
fn an_unreadable_token_file_is_an_error_not_an_absence() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    // A directory where the file should be cannot be read as one.
    let store = FileTokenStore { path: dir.path().to_path_buf(), };
    assert!(store.load().is_err());
}

/// Review of #521: a staging file left by an interrupted save, with wider
/// permissions, does not pass them on to the saved token.
#[cfg(unix)]
#[test]
fn a_leftover_staging_file_does_not_widen_the_tokens_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().expect("a temporary directory");
    let store = FileTokenStore { path: dir.path().join("mcp-client-token"), };
    let staging = store.path.with_extension("tmp");
    std::fs::write(&staging, "stale").unwrap();
    std::fs::set_permissions(&staging, std::fs::Permissions::from_mode(0o644)).unwrap();

    load_or_create(&store, &CredentialRegistry::new());

    let mode = std::fs::metadata(&store.path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
    assert!(!staging.exists());
}
