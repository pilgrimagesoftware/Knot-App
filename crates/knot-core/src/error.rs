use std::io;

use thiserror::Error;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("i/o error: {0}")]
    Io(#[from] io::Error),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

impl Error {
    /// What kind of failure this was, without the text it carried.
    ///
    /// For logging a failure that came out of handling data which must not
    /// reach a log - writing a roster that holds an ACP session id, say. An
    /// I/O error is named by its kind, a serialization error by its category,
    /// and a configuration error by neither, since its whole message is the
    /// detail.
    pub fn summary(&self) -> String {
        match self {
            Error::Config(_) => "configuration error".to_string(),
            Error::Io(error) => format!("i/o error: {}", error.kind()),
            Error::Serde(error) => format!("serialization error: {:?}", error.classify()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_summary_names_the_kind_and_drops_the_detail() {
        let detail = "sess-0123456789abcdef";
        let io = Error::Io(io::Error::new(io::ErrorKind::PermissionDenied, detail));
        let config = Error::Config(detail.to_string());

        assert!(io.summary()
                  .contains(&io::ErrorKind::PermissionDenied.to_string()));
        for error in [io, config] {
            assert!(!error.summary().contains(detail),
                    "the summary must not carry the failure's text: {}",
                    error.summary());
        }
    }
}
