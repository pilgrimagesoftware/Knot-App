//! Steering a running turn (#26); see `crate::protocol::steering`.

use super::AcpClient;
use crate::error::{AcpError, Result};
use crate::protocol::steering::{STEER_METHOD, SteerOutcome, steer_params};

impl AcpClient {
    /// Hands `text` to `session_id`'s running turn with `_session/steering`.
    ///
    /// Refuses with [`AcpError::SteeringNotSupported`], sending nothing, when
    /// the agent did not advertise steering on `initialize`. An outcome this
    /// client does not know is reported as an RPC error rather than guessed
    /// at: the caller keeps the prompt.
    pub async fn session_steer(&self, session_id: &str, text: &str) -> Result<SteerOutcome> {
        if !self.capabilities.supports_steering {
            return Err(AcpError::SteeringNotSupported);
        }
        let response = self.transport
                           .request(STEER_METHOD, Some(steer_params(session_id, text)))
                           .await?;
        SteerOutcome::from_response(&response).ok_or_else(|| {
                                                   AcpError::Rpc { code:    -32603,
                                                                   message: format!(
                "unrecognised steering outcome: {response}"
            ), }
                                               })
    }
}

#[cfg(test)]
mod tests;
