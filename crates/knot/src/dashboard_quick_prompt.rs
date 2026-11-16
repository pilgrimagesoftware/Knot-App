//! The dashboard card's quick prompt (#10,
//! `dashboard-status-and-quick-prompt`): a one-line field that sends a prompt
//! to the card's agent without opening its workspace.
//!
//! What the field shows is a pure function of the card's snapshot
//! ([`field_state`]); the view that draws the grid owns the fields
//! ([`QuickPromptFields`]) and does the sending, through
//! `WorkspaceWindow::quick_prompt` - the composer's path, which queues behind
//! a turn, a permission or a usage limit.

use std::collections::{BTreeMap, BTreeSet};

use gpui_kit::component::input::InputState;
use gpui_kit::{Entity, Subscription};
use uuid::Uuid;

use crate::dashboard::DashboardAgent;

/// What a card's field is, from its agent's snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FieldState {
    /// A shell agent: no field (writing to a shell needs rules of its own).
    Hidden,
    /// No session to send to: shown, disabled.
    Disabled,
    /// Free: what is sent goes now.
    Send,
    /// Working, awaiting input or held by a usage limit: what is sent
    /// queues until the agent can take it.
    Queues,
}

/// The field for `agent`. "Queues" whenever `deliver_panel_prompt` would
/// queue: a turn in progress, a pending permission (shown as awaiting
/// input), or a session-limit hold (#368).
pub(crate) fn field_state(agent: &DashboardAgent) -> FieldState {
    if agent.is_shell {
        return FieldState::Hidden;
    }
    if !agent.has_session {
        return FieldState::Disabled;
    }
    match agent.state {
        knot_agents::AgentState::Running | knot_agents::AgentState::Input => FieldState::Queues,
        _ if agent.session_limit.is_some() => FieldState::Queues,
        knot_agents::AgentState::Idle | knot_agents::AgentState::Error => FieldState::Send,
    }
}

/// The placeholder for a field in `state`, as a catalog key.
pub(crate) fn placeholder_key(state: FieldState) -> &'static str {
    match state {
        FieldState::Hidden | FieldState::Send => "dashboard.quick_prompt.send",
        FieldState::Disabled => "dashboard.quick_prompt.not_running",
        FieldState::Queues => "dashboard.quick_prompt.queues",
    }
}

/// One view's quick-prompt fields, by agent: one map, pruned in one place
/// (`.claude/rules/rust-structure.md`, "Parallel per-key maps").
#[derive(Default)]
pub(crate) struct QuickPromptFields {
    fields: BTreeMap<Uuid, Field>,
}

/// One card's field.
struct Field {
    input:  Entity<InputState>,
    _sends: Subscription,
    /// The state its placeholder was last set for.
    shown:  Option<FieldState>,
}

impl QuickPromptFields {
    /// `id`'s field, if it has one.
    pub(crate) fn get(&self, id: Uuid) -> Option<&Entity<InputState>> {
        self.fields.get(&id).map(|field| &field.input)
    }

    /// Whether `id` has a field yet.
    pub(crate) fn contains(&self, id: Uuid) -> bool {
        self.fields.contains_key(&id)
    }

    /// Keeps `id`'s new field and the subscription that sends from it.
    pub(crate) fn insert(&mut self, id: Uuid, input: Entity<InputState>, sends: Subscription) {
        self.fields.insert(id,
                           Field { input,
                                   _sends: sends,
                                   shown: None });
    }

    /// Records that `id`'s placeholder now says `state`'s, returning whether
    /// it said something else before - the only time it needs setting.
    fn show(&mut self, id: Uuid, state: FieldState) -> bool {
        self.fields
            .get_mut(&id)
            .is_some_and(|field| field.shown.replace(state) != Some(state))
    }

    /// Drops the fields of agents no longer on the grid.
    pub(crate) fn retain(&mut self, shown: &BTreeSet<Uuid>) {
        self.fields.retain(|id, _| shown.contains(id));
    }
}

/// What a view does when Enter is pressed in `id`'s field: `secondary` is
/// Cmd+Enter, which also opens the agent.
pub(crate) type OnEnter<V> =
    fn(&mut V, Uuid, bool, &mut gpui_kit::Window, &mut gpui_kit::Context<V>);

/// Gives each of `agents` its field - creating missing ones, keeping each
/// placeholder in step with the agent's state - and drops the fields of
/// agents no longer shown. A shell agent gets none.
///
/// Called while drawing, so it reads only the snapshots and the fields it
/// owns; the sending happens in `on_enter`, on the field's own event.
pub(crate) fn attach<'a, V: 'static>(fields: &mut QuickPromptFields,
                                     agents: impl IntoIterator<Item = &'a mut DashboardAgent>,
                                     window: &mut gpui_kit::Window,
                                     cx: &mut gpui_kit::Context<V>, on_enter: OnEnter<V>) {
    use gpui_kit::AppContext;
    use gpui_kit::component::input::InputEvent;

    let mut shown = BTreeSet::new();
    for agent in agents {
        let state = field_state(agent);
        if state == FieldState::Hidden {
            continue;
        }
        let id = agent.id;
        shown.insert(id);
        if !fields.contains(id) {
            let input = cx.new(|cx| InputState::new(window, cx));
            let sends = cx.subscribe_in(&input, window, move |view, _, event, window, cx| {
                              if let InputEvent::PressEnter { secondary, .. } = event {
                                  on_enter(view, id, *secondary, window, cx);
                              }
                          });
            fields.insert(id, input, sends);
        }
        let Some(input) = fields.get(id).cloned()
        else {
            continue;
        };
        if fields.show(id, state) {
            let wanted = knot_core::l10n::t(placeholder_key(state));
            input.update(cx, |field, cx| field.set_placeholder(wanted, window, cx));
        }
        agent.quick_prompt = Some(input);
    }
    fields.retain(&shown);
}

#[cfg(test)]
mod tests {
    use knot_agents::AgentState;

    use super::*;

    fn agent(state: AgentState, has_session: bool) -> DashboardAgent {
        DashboardAgent { id: Uuid::new_v4(),
                         avatar: String::new(),
                         name: String::new(),
                         folder_name: String::new(),
                         state,
                         is_shell: false,
                         header_title: String::new(),
                         input_reason: None,
                         session_limit: None,
                         git_stats: None,
                         has_session,
                         quick_prompt: None }
    }

    #[test]
    fn the_field_follows_the_agent() {
        let mut shell = agent(AgentState::Idle, true);
        shell.is_shell = true;
        assert_eq!(field_state(&shell), FieldState::Hidden);
        assert_eq!(field_state(&agent(AgentState::Idle, false)),
                   FieldState::Disabled);
        assert_eq!(field_state(&agent(AgentState::Idle, true)),
                   FieldState::Send);
        assert_eq!(field_state(&agent(AgentState::Error, true)),
                   FieldState::Send);
        assert_eq!(field_state(&agent(AgentState::Running, true)),
                   FieldState::Queues);
        assert_eq!(field_state(&agent(AgentState::Input, true)),
                   FieldState::Queues);
    }

    /// Held by a usage limit, the prompt waits for it to lift.
    #[test]
    fn a_session_limit_queues() {
        let mut limited = agent(AgentState::Error, true);
        limited.session_limit = Some(knot_agents::SessionLimit { resets: None });
        assert_eq!(field_state(&limited), FieldState::Queues);
    }

    #[test]
    fn every_placeholder_key_resolves() {
        for state in [FieldState::Disabled, FieldState::Send, FieldState::Queues] {
            let key = placeholder_key(state);
            assert_ne!(knot_core::l10n::t(key), key);
        }
    }
}
