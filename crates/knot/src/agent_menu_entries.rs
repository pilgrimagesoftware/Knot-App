//! The agent-row context menu's model: which entries exist, their labels,
//! and which apply to a given agent.
//!
//! Pure and GPUI-free, so the item set is unit-testable on its own. The menu
//! bar's Agents menu is built from the same list (`agent_menu`), which is
//! what keeps the two menus in the same order and grouping; the wiring that
//! reads an agent's facts and runs an entry is
//! `workspace_window::menus::agent_row`.

/// One entry in the agent-row context menu. An enum rather than a label,
/// so the renderer attaches each handler by matching a variant instead of
/// a string, and the item set stays unit-testable independent of GPUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentMenuEntry {
    Separator,
    NewCompanion,
    NewShellCompanion,
    EditAgent,
    ForkAgent,
    DuplicateAgent,
    MoveToWorkspace,
    SaveToBench,
    BenchAgent,
    OpenIn,
    MarkdownFiles,
    ConfigFiles,
    AgentInfo,
    RegisterAgent,
    LockAgent,
    UnlockAgent,
    AutopilotOff,
    AutopilotOn,
    Deactivate,
    /// Deactivate's Shift alternate (#475): deactivates and forgets the
    /// conversation, so the next start is a new session.
    DeactivateWithNewSession,
    RestartAgent,
    RestartWithNewConversation,
    RemoveAgent,
}

impl AgentMenuEntry {
    // Exhaustiveness fixture: `tests::agent_context_menu` walks this to
    // prove every variant has a label and an ordering.
    #[allow(dead_code)]
    /// Every variant, in the order they appear in a full menu.
    ///
    /// The menu bar's Agents menu pairs each labelled entry with an action,
    /// and `tests::every_agent_menu_entry_is_in_all` walks this list against
    /// an exhaustive match, so a variant added here without an action - or
    /// added to the enum without reaching this list - fails the build.
    pub(crate) const ALL: [Self; 23] = [Self::Separator,
                                        Self::NewCompanion,
                                        Self::NewShellCompanion,
                                        Self::EditAgent,
                                        Self::ForkAgent,
                                        Self::DuplicateAgent,
                                        Self::MoveToWorkspace,
                                        Self::SaveToBench,
                                        Self::BenchAgent,
                                        Self::OpenIn,
                                        Self::MarkdownFiles,
                                        Self::ConfigFiles,
                                        Self::AgentInfo,
                                        Self::RegisterAgent,
                                        Self::LockAgent,
                                        Self::UnlockAgent,
                                        Self::AutopilotOff,
                                        Self::AutopilotOn,
                                        Self::Deactivate,
                                        Self::DeactivateWithNewSession,
                                        Self::RestartAgent,
                                        Self::RestartWithNewConversation,
                                        Self::RemoveAgent];

    /// The user-visible label, or `None` for a separator. Matches the Swift
    /// reference's strings (`Skwad/Views/Components/AgentContextMenu.swift`),
    /// except that the port keeps "Remove Agent" where the reference says
    /// "Close Agent" - `agent-list-ui` already specifies the former.
    pub(crate) fn label(self) -> Option<String> {
        match self {
            Self::Separator => None,
            Self::NewCompanion => Some(knot_core::l10n::t("menu.agent.new_companion")),
            Self::NewShellCompanion => Some(knot_core::l10n::t("menu.agent.new_shell_companion")),
            Self::EditAgent => Some(knot_core::l10n::t("menu.agent.edit_agent")),
            Self::ForkAgent => Some(knot_core::l10n::t("menu.agent.fork_agent")),
            Self::DuplicateAgent => Some(knot_core::l10n::t("menu.agent.duplicate_agent")),
            Self::MoveToWorkspace => Some(knot_core::l10n::t("menu.agent.move_to_workspace")),
            Self::SaveToBench => Some(knot_core::l10n::t("menu.agent.save_to_bench")),
            Self::BenchAgent => Some(knot_core::l10n::t("menu.agent.bench_agent")),
            Self::OpenIn => Some(knot_core::l10n::t("menu.agent.open_in")),
            Self::MarkdownFiles => Some(knot_core::l10n::t("menu.agent.markdown_files")),
            Self::ConfigFiles => Some(knot_core::l10n::t("menu.agent.config_files")),
            Self::AgentInfo => Some(knot_core::l10n::t("menu.agent.agent_info")),
            Self::RegisterAgent => Some(knot_core::l10n::t("menu.agent.register_agent")),
            Self::LockAgent => Some(knot_core::l10n::t("menu.agent.lock_agent")),
            Self::UnlockAgent => Some(knot_core::l10n::t("menu.agent.unlock_agent")),
            Self::AutopilotOff => Some(knot_core::l10n::t("menu.agent.autopilot_off")),
            Self::AutopilotOn => Some(knot_core::l10n::t("menu.agent.autopilot_on")),
            Self::Deactivate => Some(knot_core::l10n::t("menu.agent.deactivate")),
            Self::DeactivateWithNewSession => {
                Some(knot_core::l10n::t("menu.agent.deactivate_new_session"))
            }
            Self::RestartAgent => Some(knot_core::l10n::t("menu.agent.restart_agent")),
            Self::RestartWithNewConversation => {
                Some(knot_core::l10n::t("menu.agent.restart_new_conversation"))
            }
            Self::RemoveAgent => Some(knot_core::l10n::t("menu.agent.remove_agent")),
        }
    }
}

/// What the context menu needs to know about the row it was opened on.
/// Everything here is read when the menu opens, not when the row renders,
/// so the item set reflects the store's current state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct AgentMenuFacts {
    /// A companion can't own companions, be forked, duplicated, moved or
    /// restarted independently of its owner (`agent-lifecycle`).
    pub(crate) is_companion:           bool,
    /// A shell agent has no coding agent to register with MCP.
    pub(crate) is_shell:               bool,
    /// Whether any attached workspace other than this agent's own exists.
    pub(crate) has_move_targets:       bool,
    /// Whether the agent has ever shown a markdown file.
    pub(crate) has_markdown_history:   bool,
    /// Whether Knot knows where this agent type keeps its configuration
    /// (#27). A shell or a custom type has none to list.
    pub(crate) has_config_files:       bool,
    /// Whether the agent is running, i.e. has a session to stop. Deactivate
    /// is absent rather than disabled when it is not, matching how this
    /// menu hides every other item that does not apply.
    pub(crate) is_running:             bool,
    /// Whether the agent runs in Panel mode, i.e. over ACP. Only an ACP
    /// session has an `initialize` response naming the agent and its
    /// version, so Agent Info is absent for a Terminal-mode agent.
    pub(crate) is_panel_mode:          bool,
    /// Whether Lock Agent applies: the agent is unlocked (#310).
    pub(crate) can_lock:               bool,
    /// Whether Unlock Agent applies: the agent is locked. Two facts rather
    /// than one `is_locked`, because the menu bar's every-item shape needs
    /// both items in place at once.
    pub(crate) can_unlock:             bool,
    /// Whether Turn Off Autopilot applies: Autopilot would act on this
    /// agent (#323). False until [`Self::with_autopilot`] says it is on at
    /// all.
    pub(crate) can_autopilot_off:      bool,
    /// Whether Turn On Autopilot applies: the agent has it turned off. Two
    /// facts for the same reason as Lock and Unlock.
    pub(crate) can_autopilot_on:       bool,
    /// Whether `restore-conversation-on-launch` is on. With it off, Restart
    /// and a later start already begin a new conversation, so the "with New
    /// Conversation/Session" alternates would repeat their primaries and
    /// are not offered (#475). False until
    /// [`Self::with_restored_conversations`] says it is on.
    pub(crate) restores_conversations: bool,
}

impl AgentMenuFacts {
    /// The facts of an agent every item applies to.
    ///
    /// The menu bar's Agents menu is built from these, because it shows the
    /// full item set and disables what the selection cannot do rather than
    /// omitting it (`app-menu`). Reading the shape from
    /// [`agent_context_menu_entries`] rather than listing it again is what
    /// keeps the two menus in the same order and grouping.
    pub(crate) const EVERY_ITEM: Self = Self { is_companion:           false,
                                               is_shell:               false,
                                               has_move_targets:       true,
                                               has_markdown_history:   true,
                                               has_config_files:       true,
                                               is_running:             true,
                                               is_panel_mode:          true,
                                               can_lock:               true,
                                               can_unlock:             true,
                                               can_autopilot_off:      true,
                                               can_autopilot_on:       true,
                                               restores_conversations: true, };

    /// These facts with the agent's Autopilot entries kept only while
    /// Autopilot is on in Settings: with it off, there is nothing for one
    /// agent to turn off or back on.
    #[must_use]
    pub(crate) fn with_autopilot(self, enabled: bool) -> Self {
        Self { can_autopilot_off: self.can_autopilot_off && enabled,
               can_autopilot_on: self.can_autopilot_on && enabled,
               ..self }
    }

    /// These facts with the Shift alternates offered only while
    /// `restore-conversation-on-launch` is `enabled`.
    #[must_use]
    pub(crate) fn with_restored_conversations(self, enabled: bool) -> Self {
        Self { restores_conversations: enabled,
               ..self }
    }

    /// Whether this agent is offered the Shift alternates: an owner with a
    /// conversation to start over, while conversations are restored.
    pub(crate) fn offers_alternates(self) -> bool {
        !self.is_companion && !self.is_shell && self.restores_conversations
    }
}

/// `entry`'s Shift alternate for an agent with these facts, if it has one
/// and the agent is offered it (`agent-list-ui`: "Shift alternates in the
/// agent row context menu").
pub(crate) fn shift_alternate(entry: AgentMenuEntry, facts: AgentMenuFacts)
                              -> Option<AgentMenuEntry> {
    let alternate = match entry {
        AgentMenuEntry::RestartAgent => AgentMenuEntry::RestartWithNewConversation,
        AgentMenuEntry::Deactivate => AgentMenuEntry::DeactivateWithNewSession,
        AgentMenuEntry::Separator
        | AgentMenuEntry::NewCompanion
        | AgentMenuEntry::NewShellCompanion
        | AgentMenuEntry::EditAgent
        | AgentMenuEntry::ForkAgent
        | AgentMenuEntry::DuplicateAgent
        | AgentMenuEntry::MoveToWorkspace
        | AgentMenuEntry::SaveToBench
        | AgentMenuEntry::BenchAgent
        | AgentMenuEntry::OpenIn
        | AgentMenuEntry::MarkdownFiles
        | AgentMenuEntry::ConfigFiles
        | AgentMenuEntry::AgentInfo
        | AgentMenuEntry::RegisterAgent
        | AgentMenuEntry::LockAgent
        | AgentMenuEntry::UnlockAgent
        | AgentMenuEntry::AutopilotOff
        | AgentMenuEntry::AutopilotOn
        | AgentMenuEntry::DeactivateWithNewSession
        | AgentMenuEntry::RestartWithNewConversation
        | AgentMenuEntry::RemoveAgent => return None,
    };
    facts.offers_alternates().then_some(alternate)
}

/// Whether `entry` is some entry's Shift alternate, as opposed to a primary
/// or an entry with no alternate.
pub(crate) fn is_shift_alternate(entry: AgentMenuEntry) -> bool {
    matches!(entry,
             AgentMenuEntry::DeactivateWithNewSession | AgentMenuEntry::RestartWithNewConversation)
}

/// One line of the agent row's context menu: an entry, and the Shift
/// alternate the line shows instead while Shift is held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AgentMenuLine {
    pub(crate) entry:     AgentMenuEntry,
    pub(crate) alternate: Option<AgentMenuEntry>,
}

/// The agent row context menu's lines: [`agent_context_menu_entries`] with
/// each Shift alternate folded into its primary's line rather than listed
/// on its own.
pub(crate) fn agent_context_menu_lines(facts: AgentMenuFacts) -> Vec<AgentMenuLine> {
    let entries = agent_context_menu_entries(facts);
    let alternates = entries.iter()
                            .filter_map(|entry| shift_alternate(*entry, facts))
                            .collect::<Vec<_>>();
    entries.into_iter()
           .filter(|entry| !alternates.contains(entry))
           .map(|entry| AgentMenuLine { entry,
                                        alternate: shift_alternate(entry, facts) })
           .collect()
}

/// Every action the agent row offers for an agent, in order, with dividers:
/// each Shift alternate directly below its primary.
///
/// This is the menu bar's item list and the set of actions a window answers
/// for its selected agent. The context menu shows it folded, through
/// [`agent_context_menu_lines`].
///
/// Dividers are emitted between non-empty groups only: a row whose whole
/// group is hidden must not leave a doubled or leading separator behind,
/// which is the failure mode of building this list with unconditional
/// `separator()` calls.
pub(crate) fn agent_context_menu_entries(facts: AgentMenuFacts) -> Vec<AgentMenuEntry> {
    use AgentMenuEntry::*;

    let owner_only = !facts.is_companion;
    let groups =
        [vec![NewCompanion, NewShellCompanion].into_iter()
                                              .filter(|_| owner_only)
                                              .collect::<Vec<_>>(),
         [EditAgent].into_iter()
                    .chain([ForkAgent, DuplicateAgent].into_iter()
                                                      .filter(|_| owner_only))
                    .collect(),
         [MoveToWorkspace].into_iter()
                          .filter(|_| owner_only && facts.has_move_targets)
                          .chain([SaveToBench, BenchAgent].into_iter().filter(|_| owner_only))
                          .collect(),
         [OpenIn].into_iter()
                 .chain([MarkdownFiles].into_iter()
                                       .filter(|_| facts.has_markdown_history))
                 .chain([ConfigFiles].into_iter().filter(|_| facts.has_config_files))
                 .collect(),
         // Deactivate sits with the other session actions, and
         // immediately above Restart Agent: both act on the
         // session rather than on the agent, and Deactivate is
         // the reversible one of the pair. Each is followed by its
         // Shift alternate, when the agent is offered one.
         //
         // Agent Info heads the session group: it reports on the
         // session the actions below it act on.
         [AgentInfo].into_iter()
                    .filter(|_| facts.is_panel_mode)
                    .chain([RegisterAgent].into_iter().filter(|_| !facts.is_shell))
                    // Lock and Unlock sit with Register: all three are about
                    // the agent's place in agent messaging. A shell agent
                    // has none to lock it out of (#310).
                    .chain([LockAgent].into_iter()
                                      .filter(|_| !facts.is_shell && facts.can_lock))
                    .chain([UnlockAgent].into_iter()
                                        .filter(|_| !facts.is_shell && facts.can_unlock))
                    // Autopilot reads a coding agent's replies; a shell has
                    // none (#323).
                    .chain([AutopilotOff].into_iter()
                                         .filter(|_| !facts.is_shell && facts.can_autopilot_off))
                    .chain([AutopilotOn].into_iter()
                                        .filter(|_| !facts.is_shell && facts.can_autopilot_on))
                    .chain([Deactivate].into_iter().filter(|_| facts.is_running))
                    .chain([DeactivateWithNewSession].into_iter().filter(|_| {
                                                                     facts.is_running
                                                                     && facts.offers_alternates()
                                                                 }))
                    .chain([RestartAgent].into_iter().filter(|_| owner_only))
                    .chain([RestartWithNewConversation].into_iter()
                                                       .filter(|_| facts.offers_alternates()))
                    .chain([RemoveAgent])
                    .collect()];

    let mut entries = Vec::new();
    for group in groups.into_iter().filter(|group| !group.is_empty()) {
        if !entries.is_empty() {
            entries.push(Separator);
        }
        entries.extend(group);
    }
    entries
}
