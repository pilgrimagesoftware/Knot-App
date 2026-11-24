//! A library prompt with user inputs, end to end through the one insertion
//! path the picker and the `/` lookup share (`prompt-library`: "Inserting a
//! prompt with inputs asks for them" and "At most five inputs are asked
//! for"; `panel-slash-commands`: "Inserting a library prompt expands its
//! text").

use std::ops::Range;

use gpui_kit::AnyWindowHandle;
use gpui_kit::Focusable;
use gpui_kit::TestAppContext;

use super::Fixture;
use super::window_with_prompt;

impl Fixture {
    /// Set the composer to `value` with the caret at its end.
    fn type_into_composer(&mut self, value: &str) {
        let (input, value) = (self.input.clone(), value.to_string());
        let caret = value.len();
        input.update_in(&mut self.window, |state, window, cx| {
                 state.set_value(value, window, cx);
                 state.set_selected_range(caret..caret, cx);
             });
    }

    /// Choose the prompt over `target`, as the lookup does with its token.
    fn choose_over(&mut self, target: Range<usize>) {
        let (agent, prompt, input) = (self.agent, self.prompt, self.input.clone());
        self.view.update_in(&mut self.window, |view, window, cx| {
                     view.expand_library_prompt(agent, prompt, target, &input, window, cx)
                 });
        self.window.run_until_parked();
    }

    /// The agent's open inputs sheet, if any.
    fn sheet(&mut self) -> Option<AnyWindowHandle> {
        let agent = self.agent;
        self.view.update(&mut self.window, |view, _| {
                     view.prompt_inputs_sheets
                         .get(&agent)
                         .map(|handle| (*handle).into())
                 })
    }

    fn open_windows(&mut self) -> usize {
        self.window.update(|_, cx| cx.windows().len())
    }

    fn composer_is_focused(&mut self) -> bool {
        let input = self.input.clone();
        self.window
            .update(|window, cx| input.focus_handle(cx).is_focused(window))
    }
}

const QUEST: &str = "{{user_input:Name}} seeks {{user_input:What is your quest?}}";

/// "Insert substitutes the values", through the picker's caret insertion.
#[gpui_kit::test]
fn insert_substitutes_the_values(cx: &mut TestAppContext) {
    let mut fixture = window_with_prompt(cx, QUEST);

    fixture.choose();
    let sheet = fixture.sheet()
                       .expect("a prompt with inputs opens the sheet");
    cx.simulate_input(sheet, "Arthur");
    cx.simulate_keystrokes(sheet, "enter");
    cx.simulate_input(sheet, "the Grail");
    cx.simulate_keystrokes(sheet, "enter");

    assert_eq!(fixture.composer(), "Arthur seeks the Grail");
    assert_eq!(fixture.sheet(), None);
    assert!(fixture.composer_is_focused());
}

/// "Cancel inserts nothing" and "Cancel returns focus to the composer".
#[gpui_kit::test]
fn cancel_inserts_nothing_and_refocuses_the_composer(cx: &mut TestAppContext) {
    let mut fixture = window_with_prompt(cx, QUEST);
    fixture.type_into_composer("keep me");

    fixture.choose_over(7..7);
    let sheet = fixture.sheet().expect("the sheet");
    cx.simulate_input(sheet, "typed then dropped");
    cx.simulate_keystrokes(sheet, "escape");

    assert_eq!(fixture.composer(), "keep me");
    assert_eq!(fixture.sheet(), None);
    assert!(fixture.composer_is_focused());
}

/// "A prompt without inputs inserts directly".
#[gpui_kit::test]
fn a_prompt_without_inputs_opens_no_sheet(cx: &mut TestAppContext) {
    let mut fixture = window_with_prompt(cx, "plain text");
    let before = fixture.open_windows();

    fixture.choose();

    assert_eq!(fixture.sheet(), None);
    assert_eq!(fixture.open_windows(), before);
    assert_eq!(fixture.composer(), "plain text");
}

/// "A sixth input uses its default": only five fields are shown, and the
/// sixth expands to its default.
#[gpui_kit::test]
fn a_sixth_input_takes_its_default(cx: &mut TestAppContext) {
    let text = "{{user_input:A|1}} {{user_input:B|2}} {{user_input:C|3}} \
                {{user_input:D|4}} {{user_input:E|5}} {{user_input:F|six}}";
    let mut fixture = window_with_prompt(cx, text);

    fixture.choose();
    let sheet = fixture.sheet().expect("the sheet");
    cx.simulate_keystrokes(sheet, "cmd-enter");

    assert_eq!(fixture.composer(), "1 2 3 4 5 six");
}

/// "Editing the composer while the sheet is open": Insert lands nothing and
/// the typing is kept.
#[gpui_kit::test]
fn an_edit_while_the_sheet_is_open_drops_the_insert(cx: &mut TestAppContext) {
    let mut fixture = window_with_prompt(cx, QUEST);

    fixture.choose();
    let sheet = fixture.sheet().expect("the sheet");
    fixture.type_into_composer("typed meanwhile");
    cx.simulate_keystrokes(sheet, "cmd-enter");

    assert_eq!(fixture.composer(), "typed meanwhile");
}

/// "A second choice brings the open sheet forward".
#[gpui_kit::test]
fn a_second_choice_keeps_the_open_sheet(cx: &mut TestAppContext) {
    let mut fixture = window_with_prompt(cx, QUEST);

    fixture.choose();
    let first = fixture.sheet().expect("the sheet");
    cx.simulate_input(first, "Arthur");
    let windows = fixture.open_windows();
    fixture.choose();

    assert_eq!(fixture.sheet(), Some(first));
    assert_eq!(fixture.open_windows(), windows);
    cx.simulate_keystrokes(first, "cmd-enter");
    assert_eq!(fixture.composer(), "Arthur seeks ");
}

/// "Removing the agent closes its sheet".
#[gpui_kit::test]
fn removing_the_agent_closes_its_sheet(cx: &mut TestAppContext) {
    let mut fixture = window_with_prompt(cx, QUEST);
    fixture.choose();
    let sheet = fixture.sheet().expect("the sheet");

    let agent = fixture.agent;
    fixture.view.update(&mut fixture.window, |view, cx| {
                    view.remove_agent_unpersisted(agent);
                    cx.notify();
                });
    fixture.window.run_until_parked();

    assert!(!fixture.window.update(|_, cx| cx.windows().contains(&sheet)));
    assert_eq!(fixture.composer(), "");
}

/// `panel-slash-commands`: "Inputs are asked for before the token is
/// replaced" and "Cancelling the inputs sheet keeps the token".
#[gpui_kit::test]
fn the_slash_token_is_replaced_on_insert_and_kept_on_cancel(cx: &mut TestAppContext) {
    let mut fixture = window_with_prompt(cx, "Review {{user_input:PR}}");
    fixture.type_into_composer("/rev");

    fixture.choose_over(0..4);
    let sheet = fixture.sheet().expect("the sheet");
    cx.simulate_keystrokes(sheet, "escape");
    assert_eq!(fixture.composer(), "/rev");

    fixture.choose_over(0..4);
    let sheet = fixture.sheet().expect("the sheet again");
    cx.simulate_input(sheet, "#466");
    cx.simulate_keystrokes(sheet, "enter");
    assert_eq!(fixture.composer(), "Review #466");
}
