//! One module per library window.
//!
//! Each owns its `render_*` method plus the label lookups and setters only
//! that pane uses - so a pane can be read, or changed, without the others in
//! view. The window's own state lives in [`super`].

pub(super) mod bench;
pub(super) mod personas;
pub(super) mod prompts;
