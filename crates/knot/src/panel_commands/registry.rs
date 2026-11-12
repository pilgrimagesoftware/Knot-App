//! The entries one lookup draws on, collected once per focused agent.

use std::path::Path;

use knot_core::agent_type::Invocation;

use crate::panel_commands::builtin;
use crate::panel_commands::entry::LookupEntry;
use crate::panel_commands::entry::LookupMatch;
use crate::panel_commands::entry::LookupSource;
use crate::panel_commands::entry::Matcher;
use crate::panel_commands::skills;

/// The entries one lookup draws on.
///
/// Collection reads the filesystem (see [`skills`]), so a registry is
/// built off the render path and cached - never rebuilt per keystroke.
#[derive(Clone, Debug)]
pub(crate) struct LookupRegistry {
    entries: Vec<LookupEntry>,
    matcher: Matcher,
}

impl Default for LookupRegistry {
    fn default() -> Self {
        Self { entries: Vec::new(),
               matcher: Matcher::Substring, }
    }
}

impl LookupRegistry {
    /// Collects every source in order, dropping an entry whose token an
    /// earlier source already claimed - so a skill cannot shadow a
    /// built-in command.
    ///
    /// The registry takes its matcher from the first source. A registry's
    /// sources are alternative places the *same kind* of thing comes from,
    /// so they share one; two kinds of thing are two registries, which is
    /// what commands and files are.
    pub(crate) fn from_sources(sources: &[&dyn LookupSource]) -> Self {
        let matcher = sources.first()
                             .map_or(Matcher::Substring, |source| source.matcher());
        let mut entries: Vec<LookupEntry> = Vec::new();
        for source in sources {
            for entry in source.entries() {
                if entries.iter().any(|seen| seen.token == entry.token) {
                    continue;
                }
                entries.push(entry);
            }
        }
        Self { entries, matcher }
    }

    /// What the command character opens for the agent whose folder is
    /// `folder`: knot's built-in commands, plus whatever skills that agent's
    /// roots hold when its skills share the command character. An agent
    /// with a skill character of its own - Codex (#272) - gets its skills
    /// from [`Self::skills_for`] instead.
    pub(crate) fn for_agent(folder: Option<&Path>, invocation: Invocation) -> Self {
        let builtin = builtin::BuiltinCommands;
        if invocation.separate_skills() {
            return Self::from_sources(&[&builtin]);
        }
        let skills = skills::SkillRoots::for_agent(folder);
        Self::from_sources(&[&builtin, &skills])
    }

    /// The skills alone, for an agent whose skill character is its own.
    pub(crate) fn skills_for(folder: Option<&Path>) -> Self {
        Self::from_sources(&[&skills::SkillRoots::for_agent(folder)])
    }

    /// The entries `filter` selects, in the matcher's order.
    pub(crate) fn matching(&self, filter: &str) -> Vec<LookupMatch> {
        self.matcher.matching(&self.entries, filter)
    }
}
