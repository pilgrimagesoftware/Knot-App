use crate::error::SessionEndCause;
use crate::protocol::{ConfigOption, PermissionRequest, SessionUpdate};

#[derive(Debug)]
pub enum SessionEvent {
    Update(SessionUpdate),
    PermissionRequest(PermissionRequest),
    /// The permission request with this `rpc_id` has been answered and is
    /// no longer pending - by the user, as `cancelled` by Stop, or as Deny
    /// when the session ended. Sent for every request, however it was
    /// answered, so a consumer showing it as pending can stop (#344).
    PermissionResolved {
        rpc_id: serde_json::Value,
    },
    Ended(SessionEndCause),
}

#[derive(Debug, Clone, Default)]
pub struct NewSession {
    pub session_id:     String,
    pub config_options: Vec<ConfigOption>,
}
