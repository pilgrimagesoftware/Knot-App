//! The inputs sheet on its own (`prompt-library`: "Inserting a prompt with
//! inputs asks for them" and "At most five inputs are asked for"): its
//! fields, its keys, and the one outcome it reports however it closes.

use std::cell::RefCell;
use std::rc::Rc;

use gpui_kit::AnyWindowHandle;
use gpui_kit::App;
use gpui_kit::Focusable;
use gpui_kit::TestAppContext;
use gpui_kit::WeakEntity;
use gpui_kit::Window;
use gpui_kit::WindowHandle;
use gpui_kit::component::Root;
use knot_agent_launch::InputValues;
use knot_agent_launch::user_inputs;

use super::PromptInputsSheet;
use super::SheetOutcome;
use super::SheetRequest;
use super::more_inputs_note;
use super::open_prompt_inputs_sheet;
use super::sheet_in;

struct Sheet {
    handle:   WindowHandle<Root>,
    /// Weak, as nothing but its window holds the sheet in the app: a strong
    /// one here would outlive the window and keep its release from firing.
    sheet:    WeakEntity<PromptInputsSheet>,
    outcomes: Rc<RefCell<Vec<SheetOutcome>>>,
}

impl Sheet {
    fn window(&self) -> AnyWindowHandle {
        self.handle.into()
    }

    /// Reads the sheet with its window, while the window is open.
    fn read<R>(&self, cx: &mut TestAppContext,
               read: impl FnOnce(&PromptInputsSheet, &Window, &App) -> R)
               -> R {
        let sheet = self.sheet.upgrade().expect("the sheet is alive");
        self.handle
            .update(cx, |_, window, cx| read(sheet.read(cx), window, cx))
            .expect("the sheet is open")
    }
}

/// The sheet for a prompt whose text is `text`.
fn open(cx: &mut TestAppContext, text: &str) -> Sheet {
    let outcomes = Rc::new(RefCell::new(Vec::new()));
    let recorded = Rc::clone(&outcomes);
    let request = SheetRequest { title:  "Quest".into(),
                                 inputs: user_inputs(text), };
    let handle =
        cx.update(|cx| {
              gpui_kit::init(cx);
              open_prompt_inputs_sheet(request,
                                       move |outcome, _| recorded.borrow_mut().push(outcome),
                                       cx)
          })
          .expect("the sheet opens");
    cx.run_until_parked();
    let sheet = cx.update(|cx| sheet_in(handle, cx))
                  .expect("the window shows the sheet")
                  .downgrade();
    Sheet { handle,
            sheet,
            outcomes }
}

fn values(sheet: &Sheet, cx: &mut TestAppContext) -> Vec<String> {
    sheet.read(cx, |sheet, _, cx| {
             sheet.field_states()
                  .iter()
                  .map(|state| state.read(cx).value().to_string())
                  .collect()
         })
}

fn focused_field(sheet: &Sheet, cx: &mut TestAppContext) -> Option<usize> {
    sheet.read(cx, |sheet, window, cx| {
             sheet.field_states()
                  .iter()
                  .position(|state| state.focus_handle(cx).is_focused(window))
         })
}

fn is_open(sheet: &Sheet, cx: &mut TestAppContext) -> bool {
    cx.update(|cx| cx.windows().contains(&sheet.window()))
}

fn inserted(pairs: &[(&str, &str)]) -> SheetOutcome {
    SheetOutcome::Insert(pairs.iter().copied().collect::<InputValues>())
}

/// "One sheet asks for every input" and "A default prefills its field".
#[gpui_kit::test]
fn one_field_per_input_in_order_prefilled_with_the_first_focused(cx: &mut TestAppContext) {
    let sheet = open(cx,
                     "{{user_input:Branch|develop}} {{user_input:Name}} {{user_input:Branch}}");

    let labels = sheet.read(cx, |sheet, _, _| sheet.labels());
    assert_eq!(labels, ["Branch", "Name"]);
    assert_eq!(values(&sheet, cx), ["develop", ""]);
    assert_eq!(focused_field(&sheet, cx), Some(0));

    // The default is selected, so typing replaces it.
    cx.simulate_input(sheet.window(), "main");
    assert_eq!(values(&sheet, cx), ["main", ""]);
}

/// "Five inputs are all asked for" and "A sixth input uses its default".
#[gpui_kit::test]
fn past_five_inputs_the_rest_are_noted_not_shown(cx: &mut TestAppContext) {
    let five = open(cx,
                    "{{user_input:A}}{{user_input:B}}{{user_input:C}}{{user_input:D}}{{user_input:E}}");
    assert_eq!(five.read(cx, |sheet, _, _| (sheet.labels().len(), sheet.more())),
               (5, 0));
    assert_eq!(more_inputs_note(0), None);

    let six = open(cx,
                   "{{user_input:A}}{{user_input:B}}{{user_input:C}}{{user_input:D}}{{user_input:E}}{{user_input:F|six}}");
    assert_eq!(six.read(cx, |sheet, _, _| (sheet.labels().len(), sheet.more())),
               (5, 1));
    let note = more_inputs_note(1).expect("a note for one more input");
    assert!(note.contains('1'),
            "the count survives substitution: {note}");
    assert_ne!(note, "prompt_inputs.more_defaulted_one");
}

/// Return moves on and inserts from the last field, with the values typed.
#[gpui_kit::test]
fn return_moves_to_the_next_field_then_inserts(cx: &mut TestAppContext) {
    let sheet = open(cx,
                     "{{user_input:Name}} seeks {{user_input:What is your quest?}}");

    cx.simulate_input(sheet.window(), "Arthur");
    cx.simulate_keystrokes(sheet.window(), "enter");
    assert_eq!(focused_field(&sheet, cx), Some(1));
    assert!(sheet.outcomes.borrow().is_empty());
    cx.simulate_input(sheet.window(), "the Grail");
    cx.simulate_keystrokes(sheet.window(), "enter");

    assert_eq!(*sheet.outcomes.borrow(),
               [inserted(&[("Name", "Arthur"), ("What is your quest?", "the Grail")])]);
    assert!(!is_open(&sheet, cx));
}

#[gpui_kit::test]
fn cmd_return_inserts_from_any_field(cx: &mut TestAppContext) {
    let sheet = open(cx, "{{user_input:A|1}} {{user_input:B|2}}");

    // `secondary`, as the input binds it: Cmd on macOS, Ctrl elsewhere. A
    // literal `cmd-enter` is Super-Return on Linux and matches nothing.
    cx.simulate_keystrokes(sheet.window(), "secondary-enter");

    assert_eq!(*sheet.outcomes.borrow(),
               [inserted(&[("A", "1"), ("B", "2")])]);
}

#[gpui_kit::test]
fn tab_and_shift_tab_move_between_fields(cx: &mut TestAppContext) {
    let sheet = open(cx, "{{user_input:A}} {{user_input:B}} {{user_input:C}}");

    cx.simulate_keystrokes(sheet.window(), "tab tab");
    assert_eq!(focused_field(&sheet, cx), Some(2));
    cx.simulate_keystrokes(sheet.window(), "shift-tab");
    assert_eq!(focused_field(&sheet, cx), Some(1));
    // A tab moves focus; it is not typed into the field.
    assert_eq!(values(&sheet, cx), ["", "", ""]);
}

/// "An emptied default expands to nothing": an empty value is reported as
/// typed, not replaced by the default.
#[gpui_kit::test]
fn an_emptied_field_is_inserted_empty(cx: &mut TestAppContext) {
    let sheet = open(cx, "{{user_input:Branch|develop}}");

    cx.simulate_keystrokes(sheet.window(), "backspace enter");

    assert_eq!(*sheet.outcomes.borrow(), [inserted(&[("Branch", "")])]);
}

/// "Cancel inserts nothing": Esc reports a cancel, once.
#[gpui_kit::test]
fn escape_cancels(cx: &mut TestAppContext) {
    let sheet = open(cx, "{{user_input:A|1}}");

    cx.simulate_keystrokes(sheet.window(), "escape");

    assert_eq!(*sheet.outcomes.borrow(), [SheetOutcome::Cancel]);
    assert!(!is_open(&sheet, cx));
}

/// Closing the window any other way - its close button, or the owner
/// removing it - is a cancel too, reported once.
#[gpui_kit::test]
fn closing_the_window_cancels(cx: &mut TestAppContext) {
    let sheet = open(cx, "{{user_input:A}}");

    sheet.handle
         .update(cx, |_, window, _| window.remove_window())
         .unwrap();
    cx.run_until_parked();

    assert_eq!(*sheet.outcomes.borrow(), [SheetOutcome::Cancel]);
}

/// Every word the sheet shows comes from the catalogue, so a missing key
/// would ship the key itself as a button label. Kept beside the sheet
/// rather than in `tests/l10n_catalog.rs`, which is at the size limit.
#[test]
fn the_sheet_labels_resolve_in_both_plural_forms() {
    for key in ["prompt_inputs.insert",
                "prompt_inputs.cancel",
                "prompt_inputs.more_defaulted_one",
                "prompt_inputs.more_defaulted_other"]
    {
        assert_ne!(knot_core::l10n::t(key),
                   key,
                   "{key} is missing from the catalog");
    }
    let many = more_inputs_note(3).expect("a note for three more inputs");
    assert!(many.contains('3'),
            "the count survives substitution: {many}");
    assert_ne!(more_inputs_note(1),
               more_inputs_note(3).map(|_| many.replace('3', "1")),
               "one and many use different forms");
}
