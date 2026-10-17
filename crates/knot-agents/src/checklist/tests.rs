use super::*;

fn checklist() -> Checklist {
    Checklist::republished(Some("Release".to_owned()),
                           vec![ChecklistItem::new("tests",
                                                   "Tests pass",
                                                   ChecklistItemKind::Check),
                                ChecklistItem::new("name",
                                                   "What should it be called?",
                                                   ChecklistItemKind::Question)],
                           None)
}

#[test]
fn the_kinds_round_trip_through_their_spelling() {
    for kind in [ChecklistItemKind::Check, ChecklistItemKind::Question] {
        assert_eq!(kind.to_string().parse::<ChecklistItemKind>(), Ok(kind));
    }
    assert!("todo".parse::<ChecklistItemKind>().is_err());
}

#[test]
fn a_check_is_ticked_and_a_question_answered_but_not_the_other_way_round() {
    let mut list = checklist();

    assert!(list.set_checked("tests", true));
    assert!(list.set_answer("name", "Knot"));
    assert!(!list.set_checked("name", true),
            "a question has nothing to tick");
    assert!(!list.set_answer("tests", "yes"),
            "a check has nothing to answer");
    assert!(!list.set_checked("missing", true));

    assert!(list.item("tests").unwrap().checked);
    assert_eq!(list.item("name").unwrap().answer, "Knot");
}

/// The agent refines its list; what the user already said survives for the
/// items that are still the same item.
#[test]
fn republishing_keeps_the_answers_of_unchanged_items() {
    let mut list = checklist();
    list.set_checked("tests", true);
    list.set_answer("name", "Knot");
    list.submitted = true;

    let again = Checklist::republished(None,
                                       vec![ChecklistItem::new("tests",
                                                               "All tests pass",
                                                               ChecklistItemKind::Check),
                                            ChecklistItem::new("name",
                                                               "Pick a name",
                                                               ChecklistItemKind::Check),
                                            ChecklistItem::new("new",
                                                               "Anything else?",
                                                               ChecklistItemKind::Question)],
                                       Some(&list));

    let tests = again.item("tests").unwrap();
    assert!(tests.checked, "a reworded item keeps its tick");
    assert_eq!(tests.text, "All tests pass");
    assert!(!again.item("name").unwrap().checked && again.item("name").unwrap().answer.is_empty(),
            "an item whose kind changed starts over");
    assert!(again.item("new").unwrap().answer.is_empty());
    assert!(!again.submitted,
            "a changed checklist has not been sent yet");
}

#[test]
fn a_change_after_sending_marks_the_list_unsent() {
    let mut list = checklist();
    list.submitted = true;

    assert!(!list.set_checked("tests", false),
            "no change, nothing to resend");
    assert!(list.submitted);
    list.set_checked("tests", true);
    assert!(!list.submitted);
}

#[test]
fn the_summary_names_every_item_and_its_state() {
    let mut list = checklist();
    list.set_checked("tests", true);

    let summary = list.summary();

    assert!(summary.contains("\"Release\""), "{summary}");
    assert!(summary.contains("- [x] Tests pass (id: tests)"),
            "{summary}");
    assert!(summary.contains("- Q: What should it be called? (id: name)\n  A: (no answer)"),
            "{summary}");
    assert!(summary.contains("get-checklist"), "{summary}");
}

/// A terminal agent gets the same content with no newline to submit early.
#[test]
fn the_one_line_summary_keeps_everything_and_has_no_newline() {
    let mut list = checklist();
    list.set_answer("name", "Knot\nor Knotty");

    let line = list.summary_line();

    assert!(!line.contains('\n'), "{line}");
    for part in ["[ ] Tests pass",
                 "Q: What should it be called?",
                 "Knot",
                 "or Knotty"]
    {
        assert!(line.contains(part), "{part} is missing from {line}");
    }
}
