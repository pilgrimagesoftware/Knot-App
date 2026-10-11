use std::time::Duration;

use crate::error::{LibraryError, Result};
use crate::index::{Index, IndexItem};
use crate::location::Location;

const TIMEOUT: Duration = Duration::from_secs(10);

/// Reads a location's index and item files. Implementations never see
/// `Settings` or do any mapping onto Knot records - that's
/// `knot-core::import::library`'s job.
pub trait LibraryFetcher: Send + Sync {
    fn fetch_index(&self, location: &Location) -> Result<Index>;
    fn fetch_item(&self, location: &Location, index: &Index, item: &IndexItem) -> Result<Vec<u8>>;
}

/// Picks the fetcher for a location's kind. The result runs on
/// `background_executor`, so it must cross a thread boundary.
pub fn fetcher_for(location: &Location) -> Box<dyn LibraryFetcher> {
    match location {
        Location::GitHub { .. } | Location::Web { .. } => Box::new(HttpsFetcher::new()),
        Location::Folder { .. } => Box::new(FolderFetcher),
    }
}

/// Serves GitHub and Web locations over HTTPS with a blocking client, so it
/// runs as-is on `background_executor` with no async runtime.
pub struct HttpsFetcher {
    agent: ureq::Agent,
}

impl HttpsFetcher {
    pub fn new() -> Self {
        let config =
            ureq::Agent::config_builder().timeout_global(Some(TIMEOUT))
                                         .user_agent(concat!("Knot/", env!("CARGO_PKG_VERSION")))
                                         .build();
        Self { agent: config.into(), }
    }

    fn get(&self, address: &str) -> Result<Vec<u8>> {
        let mut response = self.agent
                               .get(address)
                               .call()
                               .map_err(|err| LibraryError::Fetch(err.to_string()))?;
        response.body_mut()
                .read_to_vec()
                .map_err(|err| LibraryError::Fetch(err.to_string()))
    }
}

impl Default for HttpsFetcher {
    fn default() -> Self {
        Self::new()
    }
}

impl LibraryFetcher for HttpsFetcher {
    fn fetch_index(&self, location: &Location) -> Result<Index> {
        let bytes = self.get(&location.index_address())?;
        Index::parse(&bytes)
    }

    fn fetch_item(&self, location: &Location, index: &Index, item: &IndexItem) -> Result<Vec<u8>> {
        let address = location.item_address(&index.commit, item)?;
        self.get(&address)
    }
}

/// Reads a Folder location from disk.
pub struct FolderFetcher;

impl LibraryFetcher for FolderFetcher {
    fn fetch_index(&self, location: &Location) -> Result<Index> {
        let bytes = std::fs::read(location.index_address()).map_err(|err| {
                                                               LibraryError::Io(err.to_string())
                                                           })?;
        Index::parse(&bytes)
    }

    fn fetch_item(&self, location: &Location, index: &Index, item: &IndexItem) -> Result<Vec<u8>> {
        let address = location.item_address(&index.commit, item)?;
        std::fs::read(address).map_err(|err| LibraryError::Io(err.to_string()))
    }
}

#[cfg(test)]
#[path = "fetcher/tests.rs"]
mod tests;
