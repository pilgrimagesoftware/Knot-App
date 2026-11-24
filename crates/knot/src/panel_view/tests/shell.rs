//! Tests for the `!` shell command card and its row in the list.

use crate::panel_state::PanelState;

/// The `!` command card's own labels. Each status draws differently, and the
/// one that must never be confused with the others is a zero exit: a card
/// reading "Finished" when the command failed is the whole reason
/// `ShellStatus::is_failure` exists rather than a `code == 0` test per site.
#[test]
fn every_shell_status_says_something_different() {
    use knot_processes::ShellStatus;

    use crate::panel_view::shell_card::status_label;

    let labels = [status_label(&ShellStatus::Running),
                  status_label(&ShellStatus::Exited { code: 0 }),
                  status_label(&ShellStatus::Exited { code: 3 }),
                  status_label(&ShellStatus::Signalled),
                  status_label(&ShellStatus::Cancelled),
                  status_label(&ShellStatus::TimedOut),
                  status_label(&ShellStatus::FailedToStart { message: "nope".to_owned(), })];

    for label in &labels {
        assert!(!label.starts_with("panel.shell."),
                "{label} is missing from the catalog");
    }

    let mut seen = labels.to_vec();
    seen.sort();
    seen.dedup();
    assert_eq!(seen.len(),
               labels.len(),
               "two statuses read the same: {labels:?}");
}

#[test]
fn a_failed_exit_names_its_code_and_a_failed_start_its_reason() {
    use knot_processes::ShellStatus;

    use crate::panel_view::shell_card::status_label;

    assert!(status_label(&ShellStatus::Exited { code: 101 }).contains("101"));
    assert!(status_label(&ShellStatus::FailedToStart { message: "no such file".to_owned(), })
        .contains("no such file"));
}

#[test]
fn the_timeout_label_reads_in_minutes_when_it_divides_evenly() {
    use crate::panel_view::shell_card::timeout_label;

    // SHELL_TIMEOUT is 120s, so this is the minutes branch.
    let label = timeout_label();

    assert!(label.contains('2') && label.contains("minute"), "{label}");
}

#[test]
fn only_a_result_worth_sending_is_labelled() {
    use knot_processes::{ShellRunState, ShellStatus};
    use uuid::Uuid;

    use crate::panel_state::ShellCard;
    use crate::panel_view::shell_card::delivery_label;

    let mut card = ShellCard::starting(Uuid::new_v4(),
                                       "ls".to_owned(),
                                       "/tmp".to_owned(),
                                       ShellRunState::new(64));

    assert_eq!(delivery_label(&card),
               None,
               "a running command has nothing to send");

    let mut finished = ShellRunState::new(64);
    finished.status = ShellStatus::Exited { code: 0 };
    card.absorb(finished);

    assert!(delivery_label(&card).is_some_and(|label| !label.starts_with("panel.shell.")));

    card.mark_shared();
    let shared = delivery_label(&card).unwrap();

    assert!(!shared.starts_with("panel.shell."));
}

/// A shell entry is a message like any other as far as the virtualized list
/// is concerned. If it were not, every row after the first `!` command would
/// draw the wrong message.
#[test]
fn a_shell_entry_takes_one_row_like_any_other_message() {
    use knot_processes::ShellRunState;
    use uuid::Uuid;

    use crate::panel_state::ShellCard;
    use crate::panel_view::rows::{PanelRow, row_at, row_count};

    let mut state = PanelState::default();
    state.push_user_message("hello".to_owned());
    state.push_shell_command(ShellCard::starting(Uuid::new_v4(),
                                                 "ls".to_owned(),
                                                 "/tmp".to_owned(),
                                                 ShellRunState::new(64)));
    state.turn_active = false;

    assert_eq!(row_count(&state), 2);
    assert_eq!(row_at(&state, 0), Some(PanelRow::Message(0)));
    assert_eq!(row_at(&state, 1), Some(PanelRow::Message(1)));
    assert_eq!(row_at(&state, 2), None);
}
