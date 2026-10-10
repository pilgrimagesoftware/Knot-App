use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::{LibraryError, Result};

/// The only index format this Knot understands.
pub const SUPPORTED_FORMAT: u32 = 1;

/// Where one `kind`'s items live and how many the index claims, from
/// `index.json`'s `kinds` map. Knot does not read either field today - it
/// classifies items from `items` directly - but both must parse, since a
/// client throwing a field away on write is exactly what `content-index`
/// asks a reader not to do.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KindInfo {
    pub directory: String,
    pub count:     u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Index {
    pub format:       u32,
    pub commit:       String,
    #[serde(rename = "generatedAt")]
    pub generated_at: String,
    pub kinds:        BTreeMap<String, KindInfo>,
    pub items:        Vec<IndexItem>,
}

impl Index {
    /// Parses an index, rejecting any `format` other than [`SUPPORTED_FORMAT`].
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let index: Self = serde_json::from_slice(bytes)?;
        if index.format != SUPPORTED_FORMAT {
            return Err(LibraryError::UnsupportedFormat(index.format));
        }
        Ok(index)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexItem {
    pub kind:        ItemKind,
    pub id:          String,
    pub slug:        String,
    pub title:       String,
    pub description: String,
    #[serde(default)]
    pub tags:        Vec<String>,
    #[serde(default)]
    pub authors:     Vec<String>,
    pub path:        String,
    pub size:        u64,
    pub sha256:      String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemKind {
    Persona,
    Prompt,
    Other(String),
}

impl Serialize for ItemKind {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let s = match self {
            Self::Persona => "persona",
            Self::Prompt => "prompt",
            Self::Other(kind) => kind,
        };
        serializer.serialize_str(s)
    }
}

impl<'de> Deserialize<'de> for ItemKind {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(match s.as_str() {
            "persona" => Self::Persona,
            "prompt" => Self::Prompt,
            _ => Self::Other(s),
        })
    }
}

#[cfg(test)]
#[path = "index/tests.rs"]
mod tests;
