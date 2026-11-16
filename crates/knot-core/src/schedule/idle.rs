//! What happens to an agent that has been idle, stored as text: `inherit`,
//! `off`, `deactivate 30m`, `reset 60m`. An unreadable value loads as
//! [`IdlePolicy::Inherit`], the default, so a bad field never stops an agent
//! from loading.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// What idling does to an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IdleAction {
    /// Stop the session, as the context menu's Deactivate does.
    Deactivate,
    /// Restart it in a new conversation, without its startup prompt.
    Reset,
}

impl IdleAction {
    pub const ALL: [Self; 2] = [Self::Deactivate, Self::Reset];

    #[must_use]
    pub const fn token(self) -> &'static str {
        match self {
            Self::Deactivate => "deactivate",
            Self::Reset => "reset",
        }
    }
}

/// After how long, and what to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IdleRule {
    after_minutes: u32,
    action:        IdleAction,
}

impl IdleRule {
    /// `None` for a zero period: idling for no time at all is not a rule.
    #[must_use]
    pub fn new(after_minutes: u32, action: IdleAction) -> Option<Self> {
        (after_minutes > 0).then_some(Self { after_minutes,
                                             action })
    }

    #[must_use]
    pub fn after_minutes(self) -> u32 {
        self.after_minutes
    }

    #[must_use]
    pub fn action(self) -> IdleAction {
        self.action
    }
}

impl fmt::Display for IdleRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}m", self.action.token(), self.after_minutes)
    }
}

impl FromStr for IdleRule {
    type Err = ();

    fn from_str(text: &str) -> Result<Self, ()> {
        let (action, after) = text.trim().split_once(' ').ok_or(())?;
        let action = IdleAction::ALL.into_iter()
                                    .find(|candidate| candidate.token() == action)
                                    .ok_or(())?;
        let minutes = after.trim()
                           .strip_suffix('m')
                           .ok_or(())?
                           .parse()
                           .map_err(|_| ())?;
        Self::new(minutes, action).ok_or(())
    }
}

/// An agent's own idle setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum IdlePolicy {
    /// Follow the settings default.
    #[default]
    Inherit,
    /// Never stop this agent for idling, whatever the default.
    Off,
    Custom(IdleRule),
}

impl IdlePolicy {
    /// The rule that applies, given the settings default.
    #[must_use]
    pub fn effective(self, default: Option<IdleRule>) -> Option<IdleRule> {
        match self {
            Self::Inherit => default,
            Self::Off => None,
            Self::Custom(rule) => Some(rule),
        }
    }
}

impl fmt::Display for IdlePolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Inherit => f.write_str("inherit"),
            Self::Off => f.write_str("off"),
            Self::Custom(rule) => rule.fmt(f),
        }
    }
}

impl FromStr for IdlePolicy {
    type Err = ();

    fn from_str(text: &str) -> Result<Self, ()> {
        match text.trim() {
            "inherit" => Ok(Self::Inherit),
            "off" => Ok(Self::Off),
            rule => rule.parse().map(Self::Custom),
        }
    }
}

impl Serialize for IdlePolicy {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for IdlePolicy {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(String::deserialize(deserializer)?.parse()
                                             .unwrap_or_default())
    }
}

impl Serialize for IdleRule {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

/// Reads the settings default leniently: absent, `null` or unreadable all
/// load as no default (off), so one bad value never fails the preferences
/// document. Use with `#[serde(default, deserialize_with = ...)]`.
///
/// # Errors
///
/// Never for a value it can read as an optional string; only when the
/// document itself is malformed at this field.
pub fn lenient_idle_rule<'de, D: serde::Deserializer<'de>>(
    deserializer: D)
    -> Result<Option<IdleRule>, D::Error> {
    Ok(Option::<String>::deserialize(deserializer)?.and_then(|text| text.parse().ok()))
}
