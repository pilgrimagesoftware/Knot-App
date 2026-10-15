use thiserror::Error;

pub type Result<T, E = ClientError> = std::result::Result<T, E>;

/// Why a server's tools could not be listed. Every variant is said in words
/// beside the row; none of them means "no tools", which is an empty list.
#[derive(Debug, Error)]
pub enum ClientError {
    #[error("could not start {program}: {source}")]
    Spawn {
        program: String,
        #[source]
        source:  std::io::Error,
    },

    /// The listing's whole time ran out, at whatever step it had reached.
    #[error("the server did not answer in time")]
    TimedOut,

    /// HTTP 401 or 403: the server wants a credential Knot does not hold.
    #[error("the server needs sign-in (HTTP {0})")]
    Unauthorized(u16),

    #[error("the server answered HTTP {0}")]
    Http(u16),

    #[error("the server closed the connection")]
    Closed,

    #[error("the server returned error {code}: {message}")]
    Rpc { code: i64, message: String },

    #[error("the server's answer was not understood: {0}")]
    Protocol(String),

    #[error("{0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Transport(String),
}
