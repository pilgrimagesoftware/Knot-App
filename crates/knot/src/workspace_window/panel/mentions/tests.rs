//! The `@` lookup ranks off the render path (`panel-file-mentions`,
//! "Ranking stays off the render path"; #438 review).

use gpui_kit::TestAppContext;

use super::PanelMentions;
use crate::workspace_window::shortcuts_tests::window_with_agents;

fn listed(paths: &[&str]) -> PanelMentions {
    PanelMentions::listed(paths)
}

/// What render asks for comes back at once as a "matching" row - nothing
/// is ranked while drawing - and the ranking arrives through the poll.
#[gpui_kit::test]
fn the_popup_reads_a_ranking_it_never_computes(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    let agent = fixture.agents[0];
    fixture.view.update(&mut fixture.window, |view, _| {
                    view.panel_mentions
                        .insert(agent, listed(&["src/lib.rs", "README.md"]));

                    let first = view.mention_ranking(agent, "lib").expect("a listing");
                    assert_eq!(first.len(), 1);
                    assert!(first[0].is_status, "matching, not ranked in render");

                    let mut landed = false;
                    for _ in 0..500 {
                        if view.drain_mention_listings() {
                            landed = true;
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(5));
                    }
                    assert!(landed, "the ranking reaches the poll");

                    let ranked = view.mention_ranking(agent, "lib").expect("a listing");
                    assert_eq!(ranked.iter()
                                     .map(|m| m.entry.token.as_str())
                                     .collect::<Vec<_>>(),
                               vec!["src/lib.rs"]);
                });
}

/// A bare `@` lists the folder as gathered, unranked, with no wait.
#[gpui_kit::test]
fn a_bare_at_lists_the_folder_at_once(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    let agent = fixture.agents[0];
    fixture.view.update(&mut fixture.window, |view, _| {
                    view.panel_mentions.insert(agent, listed(&["a.rs", "b.rs"]));
                    let rows = view.mention_ranking(agent, "").expect("a listing");
                    assert_eq!(rows.len(), 2);
                    assert!(rows.iter().all(|row| !row.is_status));
                });
}
