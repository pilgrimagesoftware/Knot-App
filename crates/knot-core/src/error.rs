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
    /// Which kind of failure this was, and nothing it carried.
    ///
    /// For logging a failure that came out of handling data which must not
    /// reach a log - writing a roster that holds an ACP session id, say.
    /// Deliberately only the variant: anything read *out of* the error (an
    /// `io::ErrorKind`, a serde category) is data derived from the failed
    /// write, and CodeQL's `rust/cleartext-logging` rightly treats it as
    /// carrying what that write held.
    pub fn category(&self) -> &'static str {
        match self {
            Error::Config(_) => "configuration error",
            Error::Io(_) => "i/o error",
            Error::Serde(_) => "serialization error",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_category_names_the_variant_and_drops_the_detail() {
        let detail = "sess-0123456789abcdef";
        let io = Error::Io(io::Error::new(io::ErrorKind::PermissionDenied, detail));
        let config = Error::Config(detail.to_string());

        assert_eq!(io.category(), "i/o error");
        assert_eq!(config.category(), "configuration error");
        for error in [io, config] {
            assert!(!error.category().contains(detail));
        }
    }
}
