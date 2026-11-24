//! Getting a prompt from the composer into the agent: the input entity and
//! its send chord, attached context, the queue when a turn is already in
//! flight, and stopping one that is.
//!
//! The queue's own rules live in [`super::super::prompt_queue`]; this is
//! the window side - which entity holds the text, when it may be sent, and
//! what happens to a delivery result coming back off the runtime.

mod attachments;
mod composer;
mod delivery;
mod queue_edit;

#[cfg(test)]
pub(crate) use composer::PANEL_INPUT_ROWS_COLLAPSED;
#[cfg(test)]
pub(crate) use composer::PANEL_INPUT_ROWS_EXPANDED;
pub(crate) use composer::PanelInput;
pub(crate) use composer::PanelInputState;
#[cfg(test)]
pub(crate) use composer::enter_sends;
#[cfg(test)]
pub(crate) use composer::new_panel_input;
#[cfg(test)]
pub(crate) use composer::panel_input_max_rows;
#[cfg(test)]
pub(crate) use composer::sends_on;
