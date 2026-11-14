//! The fixture is real `agy mcp list` output (agy 1.5.8, names and hosts
//! changed).

use crate::error::ProbeError;
use crate::parse::ListFormat;
use crate::server::{ServerRow, Target};
use crate::state::ServerState;

const LISTING: &str = include_str!("../fixtures/antigravity_mcp_list.txt");

fn read(output: &str) -> crate::error::Result<Vec<ServerRow>> {
    ListFormat::Antigravity.read("agy", output)
}

fn row(name: &str) -> ServerRow {
    read(LISTING).expect("the fixture is a listing")
                 .into_iter()
                 .find(|row| row.name == name)
                 .unwrap_or_else(|| panic!("no row named {name:?}"))
}

#[test]
fn one_row_per_server() {
    assert_eq!(read(LISTING).unwrap().len(), 3);
}

/// `agy` lists configuration; an enabled row never saw the server answer.
#[test]
fn an_enabled_server_is_unknown_not_connected() {
    assert_eq!(row("notes").state, ServerState::Unknown);
}

#[test]
fn a_disabled_server_is_disabled() {
    assert_eq!(row("local").state, ServerState::Disabled);
}

#[test]
fn a_stdio_command_keeps_its_arguments_as_one_string() {
    assert_eq!(row("other").target,
               Target::Stdio { command: "npx -y @scope/pkg --flag".to_owned(), });
}

#[test]
fn an_http_row_carries_its_url() {
    assert_eq!(row("notes").target,
               Target::Http { url: "http://127.0.0.1:9/mcp".to_owned(), });
}

#[test]
fn nothing_configured_is_an_empty_list() {
    assert!(read("No MCP servers configured.\n").unwrap().is_empty());
}

/// A listing this parser cannot read is "could not read", not "no servers".
#[test]
fn an_unreadable_listing_is_unrecognized() {
    assert!(matches!(read("something else entirely\n"),
                     Err(ProbeError::Unrecognized { .. })));
    assert!(matches!(read("NAME TYPE STATUS COMMAND/URL\nx stdio maybe cmd\n"),
                     Err(ProbeError::Unrecognized { .. })));
}
