//! A repository the user has trusted to run the programs its own git
//! configuration names. See `openspec/specs/git-operations/spec.md`,
//! "Repository trust".

use serde::{Deserialize, Serialize};

/// One trusted repository: its folder, and the configuration entries that
/// named programs when the user trusted it, as `key=value`. Trust holds for
/// exactly those entries - a repository whose entries have changed since is
/// asked about again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustedRepository {
    pub path:     String,
    pub settings: Vec<String>,
}
