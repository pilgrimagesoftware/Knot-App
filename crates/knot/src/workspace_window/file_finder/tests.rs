//! The file finder in a window (`file-finder`): when it opens, what it
//! lists, and where a chosen path goes. Panel delivery is checked end to end
//! into a real composer; for Terminal mode the fixture has no PTY to type
//! into, so the test pins the routing and the exact text written, and the
//! write itself is `TerminalSession::send_text`.

use gpui_kit::TestAppContext;
use uuid::Uuid;

use super::state::{FinderRows, FoundPathTarget, terminal_text};
use crate::app_bootstrap::FindFile;
use crate::keymap::SelectAgent1;
use crate::workspace_window::panel::mentions::PanelMentions;
use crate::workspace_window::shortcuts_tests::{Fixture, available, window_with_agents};

/// The finder open for agent 0 over `paths`, with `query` typed.
fn finder_over(paths: &[&str], query: &str, cx: &mut TestAppContext) -> (Fixture, Uuid) {
    let mut fixture = window_with_agents(2, cx);
    let agent = fixture.agents[0];
    fixture.press(SelectAgent1);
    fixture.view.update(&mut fixture.window, |view, _| {
                    view.panel_mentions
                        .insert(agent, PanelMentions::listed(paths));
                });
    fixture.press(FindFile);
    let input = fixture.view.read_with(&fixture.window, |view, _| {
                                view.file_finder
                                    .as_ref()
                                    .expect("the finder opened")
                                    .input
                                    .clone()
                            });
    fixture.window.update(|window, cx| {
                      input.update(cx, |state, cx| {
                               state.set_value(query.to_owned(), window, cx)
                           });
                  });
    (fixture, agent)
}

/// Asks for the rows until the ranking lands, as frames and the poll would.
fn settled_rows(fixture: &mut Fixture) -> FinderRows {
    for _ in 0..400 {
        let rows = fixture.view.update(&mut fixture.window, |view, cx| {
                                   view.drain_file_finder();
                                   view.file_finder_rows(cx)
                               });
        if rows != FinderRows::Matching {
            return rows;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    panic!("the ranking never landed");
}

#[gpui_kit::test]
fn find_file_needs_a_selected_agent(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    fixture.view.update(&mut fixture.window, |view, cx| {
                    view.selected_agent = None;
                    cx.notify();
                });
    // Availability is decided per frame, as the menu bar reads it.
    fixture.window.run_until_parked();
    assert!(!available(&mut fixture, &FindFile));
    fixture.press(SelectAgent1);
    assert!(available(&mut fixture, &FindFile));
}

#[gpui_kit::test]
fn the_ranking_lands_off_the_render_path(cx: &mut TestAppContext) {
    let (mut fixture, _) = finder_over(&["src/lib.rs", "README.md"], "lib", cx);
    let first = fixture.view
                       .update(&mut fixture.window, |view, cx| view.file_finder_rows(cx));
    assert_eq!(first,
               FinderRows::Matching,
               "nothing is ranked while drawing");

    let FinderRows::Results(rows) = settled_rows(&mut fixture)
    else {
        panic!("expected results");
    };
    assert_eq!(rows[0].entry.token, "src/lib.rs");
}

#[gpui_kit::test]
fn an_empty_query_lists_nothing(cx: &mut TestAppContext) {
    let (mut fixture, _) = finder_over(&["src/lib.rs"], "", cx);
    assert_eq!(fixture.view
                      .update(&mut fixture.window, |view, cx| view.file_finder_rows(cx)),
               FinderRows::Empty);
}

#[gpui_kit::test]
fn a_query_with_no_match_says_so(cx: &mut TestAppContext) {
    let (mut fixture, _) = finder_over(&["src/lib.rs"], "zzz", cx);
    assert_eq!(settled_rows(&mut fixture), FinderRows::NoMatches);
}

/// Panel mode: the plain path lands at the composer's caret, no `@`, and
/// the finder closes.
#[gpui_kit::test]
fn choosing_inserts_the_path_into_the_composer(cx: &mut TestAppContext) {
    let (mut fixture, agent) = finder_over(&["src/lib.rs", "README.md"], "lib", cx);
    let composer = fixture.window.update(|window, cx| {
                                     fixture.view.update(cx, |view, cx| {
                                                     view.panel_prompt_input(agent, window, cx)
                                                 })
                                 });
    settled_rows(&mut fixture);

    fixture.window.update(|window, cx| {
                      fixture.view
                             .update(cx, |view, cx| view.choose_file_finder(None, window, cx))
                  });

    assert_eq!(composer.read_with(&fixture.window, |state, _| state.value().to_string()),
               "src/lib.rs");
    assert!(fixture.view
                   .read_with(&fixture.window, |view, _| view.file_finder.is_none()));
}

#[gpui_kit::test]
fn the_selection_stops_at_either_end(cx: &mut TestAppContext) {
    let (mut fixture, _) = finder_over(&["a.rs"], "a", cx);
    fixture.view.update(&mut fixture.window, |view, _| {
                    view.move_file_finder(-1, 3);
                    assert_eq!(view.file_finder.as_ref().unwrap().selected, 0);
                    view.move_file_finder(5, 3);
                    assert_eq!(view.file_finder.as_ref().unwrap().selected, 2);
                });
}

/// Esc closes without delivering anything.
#[gpui_kit::test]
fn escape_closes_and_delivers_nothing(cx: &mut TestAppContext) {
    let (mut fixture, _) = finder_over(&["src/lib.rs"], "lib", cx);
    fixture.window.update(|window, cx| {
                      fixture.view
                             .update(cx, |view, cx| view.close_file_finder(window, cx))
                  });
    assert!(fixture.view
                   .read_with(&fixture.window, |view, _| view.file_finder.is_none()));
}

/// Selecting another agent closes the finder: it belongs to the agent it
/// was opened for.
#[gpui_kit::test]
fn selecting_another_agent_closes_it(cx: &mut TestAppContext) {
    let (mut fixture, _) = finder_over(&["src/lib.rs"], "lib", cx);
    let other = fixture.agents[1];
    fixture.view.update(&mut fixture.window, |view, _| {
                    view.selected_agent = Some(other);
                    assert!(view.drain_file_finder());
                    assert!(view.file_finder.is_none());
                });
}

/// A shell agent is Terminal mode: its path is typed, not inserted; a
/// Panel agent's goes to its composer; and the text typed is the bare path
/// with no newline, so it waits at the prompt.
#[gpui_kit::test]
fn a_terminal_mode_agent_has_the_path_typed_without_a_newline(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    let panel = fixture.agents[0];
    let shell = fixture.view.update(&mut fixture.window, |view, _| {
                                let mut store = view.store.lock();
                                let workspace = store.workspaces()[0].id;
                                store.create("~/shell",
                                             knot_agents::CreateOptions { workspace_id:
                                                                              Some(workspace),
                                                                          agent_type:
                                                                              Some("shell".into()),
                                                                          ..Default::default() })
                            });
    fixture.view.update(&mut fixture.window, |view, _| {
                    assert_eq!(view.found_path_target(panel), FoundPathTarget::Composer);
                    assert_eq!(view.found_path_target(shell), FoundPathTarget::Terminal);
                    assert_eq!(view.found_path_target(Uuid::new_v4()),
                               FoundPathTarget::Nowhere);
                });
    assert_eq!(terminal_text("src/my file.rs"), "src/my file.rs");
    assert!(!terminal_text("src/lib.rs").ends_with('\n'));
}

/// A Panel agent whose composer was never built still gets the path: the
/// composer is created for it.
#[gpui_kit::test]
fn a_panel_agent_without_a_composer_gets_one(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    let agent = fixture.agents[0];
    fixture.window.update(|window, cx| {
                      fixture.view.update(cx, |view, cx| {
                                      assert!(!view.panel_prompt_inputs.contains_key(&agent));
                                      view.deliver_found_path(agent, "src/lib.rs", window, cx);
                                  })
                  });
    let text = fixture.view.read_with(&fixture.window, |view, cx| {
                               view.panel_prompt_inputs[&agent].read(cx)
                                                               .value()
                                                               .to_string()
                           });
    assert_eq!(text, "src/lib.rs");
}
