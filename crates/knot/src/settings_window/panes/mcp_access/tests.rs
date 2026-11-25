//! Regenerating the client token from the MCP tab (`mcp-auth` task 6.1):
//! the old token is refused, the new one accepted and shown.

use std::cell::RefCell;

use gpui_kit::TestAppContext;
use knot_mcp::{Principal, Secret};

use crate::mcp_credentials::{ClientToken, ClientTokenStore, McpCredentials, load_or_create};

#[derive(Default)]
struct MemoryStore(RefCell<Option<String>>);

impl ClientTokenStore for MemoryStore {
    fn load(&self) -> Result<Option<String>, String> {
        Ok(self.0.borrow().clone())
    }

    fn save(&self, token: &Secret) -> Result<(), String> {
        *self.0.borrow_mut() = Some(token.expose().to_owned());
        Ok(())
    }
}

#[gpui_kit::test]
fn regenerating_refuses_the_old_token_and_shows_the_new(cx: &mut TestAppContext) {
    let store = MemoryStore::default();
    let credentials = McpCredentials::default();
    let old = load_or_create(&store, &credentials.registry);
    cx.update(|cx| {
          cx.set_global(credentials.clone());
          cx.set_global(ClientToken(old.clone()));
      });

    cx.update(|cx| super::regenerate_client_token(&store, cx));

    let shown = cx.update(|cx| cx.global::<ClientToken>().0.token.clone());
    assert_ne!(shown, old.token);
    assert_eq!(credentials.registry.authenticate(old.token.expose()), None);
    assert_eq!(credentials.registry.authenticate(shown.expose()),
               Some(Principal::External));
    assert_eq!(store.0.borrow().as_deref(), Some(shown.expose()), "saved");
}
