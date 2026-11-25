//! The workspace window's element tree.
//!
//! `Render::render`, in [`root`], is a dispatcher: it snapshots the state
//! the frame needs out of the store once ([`frame`]), then hands each region
//! to the module that owns it - [`sidebar`] for the agent list, [`dashboard`]
//! for the overview, [`title_bar`] for the header, [`content`] for the pane
//! that fills the rest.
//!
//! Splitting it that way is not cosmetic. Every one of those regions
//! captures the same handful of locals and builds closures over the window
//! entity; keeping them in one function meant a change to any region had
//! to be read against all of them.

mod activity_capture;
mod agent_sections;
mod changes_tabs;
mod content;
mod dashboard_row;
pub(super) mod divider;
mod frame;
mod issues_pane;
mod issues_toolbar;
pub(super) mod layout;
pub(super) mod layout_menu;
mod mcp_banner;
pub(super) mod mcp_pane;
mod openspec_pane;
mod openspec_toolbar;
mod overview;
mod pane_frame;
mod panes;
mod processes_pane;
mod processes_summary;
pub(super) mod pull_requests_pane;
mod pull_requests_row;
mod pull_requests_toolbar;
mod root;
mod row_actions;
pub(super) mod send_prompt_menu;
mod sidebar;
mod sidebar_column;
mod sidebar_compact;
mod sidebar_row;
mod status_dot;
mod title_bar;
mod usage_banner;
