//! The client token's lifecycle against an in-memory store (task 1.2).

use std::cell::RefCell;

use knot_mcp::{CredentialKind, CredentialRegistry, Principal, Secret};

use super::*;

#[derive(Default)]
struct MemoryStore {
    value: RefCell<Option<String>>,
    fails: bool,
}

impl ClientTokenStore for MemoryStore {
    fn load(&self) -> Option<String> {
        self.value.borrow().clone()
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
    let token = load_or_create(&store, &registry);

    assert_eq!(store.load().as_deref(), Some(token.expose()));
    assert_eq!(registry.authenticate(token.expose()),
               Some(Principal::External));
}

#[test]
fn a_later_launch_reuses_the_stored_token() {
    let store = MemoryStore::default();
    let first = load_or_create(&store, &CredentialRegistry::new());
    let registry = CredentialRegistry::new();

    assert_eq!(load_or_create(&store, &registry), first);
    assert_eq!(registry.authenticate(first.expose()),
               Some(Principal::External));
}

#[test]
fn a_damaged_stored_token_is_replaced() {
    let store = MemoryStore::default();
    let agent_shaped = Secret::generate(CredentialKind::AgentTool);
    *store.value.borrow_mut() = Some(agent_shaped.expose().to_owned());
    let registry = CredentialRegistry::new();

    let token = load_or_create(&store, &registry);

    assert_ne!(token.expose(), agent_shaped.expose());
    assert_eq!(registry.authenticate(agent_shaped.expose()), None);
}

#[test]
fn regenerating_refuses_the_old_token_and_saves_the_new() {
    let (store, registry) = (MemoryStore::default(), CredentialRegistry::new());
    let old = load_or_create(&store, &registry);

    let new = regenerate(&store, &registry);

    assert_ne!(old, new);
    assert_eq!(registry.authenticate(old.expose()), None);
    assert_eq!(registry.authenticate(new.expose()),
               Some(Principal::External));
    assert_eq!(store.load().as_deref(), Some(new.expose()));
}

#[test]
fn a_token_that_cannot_be_saved_still_serves_this_launch() {
    let store = MemoryStore { fails: true,
                              ..MemoryStore::default() };
    let registry = CredentialRegistry::new();

    let token = load_or_create(&store, &registry);

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
