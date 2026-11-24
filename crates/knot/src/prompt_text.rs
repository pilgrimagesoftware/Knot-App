//! What every field that edits prompt text offers: the list of prompt
//! variables to insert at the caret, and the warnings - variables it does
//! not recognise, malformed or too many user inputs, and inputs a startup
//! prompt leaves empty.
//!
//! Contract: `openspec/specs/prompt-library/spec.md`, "Prompt variables" and
//! "Unknown variables and escaping", "User inputs", "At most five inputs are
//! asked for" and "Startup prompts never ask for inputs". Shared by the agent
//! editor's custom startup prompt and the Settings window's prompt editor, so
//! the two cannot offer different lists or warn differently.

use gpui_kit::App;
use gpui_kit::Entity;
use gpui_kit::Window;
use gpui_kit::component::input::TextareaState;
use gpui_kit::component::menu::PopupMenu;
use gpui_kit::component::menu::PopupMenuItem;
use knot_agent_launch::MAX_PROMPT_INPUTS;
use knot_agent_launch::PromptVariable;

/// The catalogue key describing what `variable` expands to.
fn description_key(variable: PromptVariable) -> &'static str {
    match variable {
        PromptVariable::AgentName => "prompt_variables.agent_name",
        PromptVariable::AgentId => "prompt_variables.agent_id",
        PromptVariable::AgentType => "prompt_variables.agent_type",
        PromptVariable::Folder => "prompt_variables.folder",
        PromptVariable::FolderName => "prompt_variables.folder_name",
        PromptVariable::Workspace => "prompt_variables.workspace",
        PromptVariable::Branch => "prompt_variables.branch",
        PromptVariable::Date => "prompt_variables.date",
        PromptVariable::Clipboard => "prompt_variables.clipboard",
    }
}

/// A variable's menu label: the placeholder and what it expands to.
pub(crate) fn variable_label(variable: PromptVariable) -> String {
    format!("{}  {}",
            variable.placeholder(),
            knot_core::l10n::t(description_key(variable)))
}

/// `menu` with one item per prompt variable, each inserting its placeholder
/// into `target` at the caret.
pub(crate) fn variable_menu(mut menu: PopupMenu, target: &Entity<TextareaState>) -> PopupMenu {
    for variable in PromptVariable::ALL {
        let target = target.clone();
        menu = menu.item(PopupMenuItem::new(variable_label(variable)).on_click(
            move |_, window, app| insert_variable(&target, variable, window, app),
        ));
    }
    menu
}

/// Write `variable`'s placeholder at `target`'s caret, replacing any
/// selection, the way typing it would.
pub(crate) fn insert_variable(target: &Entity<TextareaState>, variable: PromptVariable,
                              window: &mut Window, app: &mut App) {
    target.update(app, |state, cx| {
              state.replace(variable.placeholder(), window, cx);
          });
}

/// The warning to show under prompt text naming the variables it does not
/// recognise, or `None` when every `{{...}}` in it is a known variable.
/// A warning, never an error: the text still saves and still sends, with
/// the unknown names left as typed.
pub(crate) fn unknown_variables_warning(text: &str) -> Option<String> {
    let unknown = knot_agent_launch::unknown_variables(text);
    (!unknown.is_empty()).then(|| {
                             knot_core::l10n::t_with("prompt_variables.unknown",
                                                     &[("names", &unknown.join(", "))])
                         })
}

/// The warning to show under a custom startup prompt that uses
/// `{{clipboard}}`, which is always empty there. A warning, never an error:
/// the prompt still saves.
pub(crate) fn startup_clipboard_warning(text: &str) -> Option<String> {
    knot_agent_launch::uses_variable(text, PromptVariable::Clipboard)
        .then(|| knot_core::l10n::t("prompt_variables.clipboard_in_startup"))
}

/// What the user-input entry inserts: an input with no label yet, so the
/// caret goes before the closing braces.
const USER_INPUT_STUB: &str = "{{user_input:}}";
/// How far before the end of [`USER_INPUT_STUB`] the caret goes.
const USER_INPUT_CARET_FROM_END: usize = "}}".len();

/// `menu` with the user-input entry at its end - the Prompts editor's list
/// only, since an input in a startup prompt is never asked for.
pub(crate) fn with_user_input_item(menu: PopupMenu, target: &Entity<TextareaState>) -> PopupMenu {
    let target = target.clone();
    let label = format!("{USER_INPUT_STUB}  {}",
                        knot_core::l10n::t("prompt_variables.user_input"));
    menu.separator()
        .item(PopupMenuItem::new(label).on_click(move |_, window, app| {
                                           insert_user_input(&target, window, app)
                                       }))
}

/// Write [`USER_INPUT_STUB`] at `target`'s caret and leave the caret where
/// the label goes.
pub(crate) fn insert_user_input(target: &Entity<TextareaState>, window: &mut Window,
                                app: &mut App) {
    target.update(app, |state, cx| {
              state.replace(USER_INPUT_STUB, window, cx);
              let caret = state.cursor().saturating_sub(USER_INPUT_CARET_FROM_END);
              state.set_selected_range(caret..caret, cx);
          });
}

/// Every warning the Prompts editor shows under a library prompt's text:
/// unknown variables, a blank-label input, and more inputs than the sheet
/// asks for. Warnings, never errors: the prompt still saves.
pub(crate) fn library_prompt_warnings(text: &str) -> Vec<String> {
    let malformed = knot_agent_launch::malformed_inputs(text)
        .then(|| knot_core::l10n::t("prompt_variables.malformed_input"));
    let too_many = (knot_agent_launch::user_inputs(text).len() > MAX_PROMPT_INPUTS).then(|| {
                       knot_core::l10n::t_with("prompt_variables.too_many_inputs",
                                               &[("max", &MAX_PROMPT_INPUTS.to_string())])
                   });
    [unknown_variables_warning(text), malformed, too_many].into_iter()
                                                          .flatten()
                                                          .collect()
}

/// The warning to show under a custom startup prompt whose inputs have no
/// default, naming them: a startup prompt never asks, so they expand to
/// nothing. A warning, never an error: the prompt still saves.
pub(crate) fn startup_inputs_warning(text: &str) -> Option<String> {
    let empty: Vec<String> =
        knot_agent_launch::user_inputs(text).into_iter()
                                            .filter(|input| input.default.is_none())
                                            .map(|input| input.label)
                                            .collect();
    (!empty.is_empty()).then(|| {
                           knot_core::l10n::t_with("prompt_variables.inputs_in_startup",
                                                   &[("names", &empty.join(", "))])
                       })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_variable_has_a_description_that_resolves() {
        for variable in PromptVariable::ALL {
            let key = description_key(variable);
            assert_ne!(knot_core::l10n::t(key), key, "{key} must resolve");
            assert!(variable_label(variable).starts_with(&variable.placeholder()));
        }
    }

    /// The spec's "Startup prompt editor warns" scenario: `{{clipboard}}`
    /// in a startup prompt draws a warning that resolves.
    #[test]
    fn a_startup_prompt_using_the_clipboard_is_warned() {
        assert_eq!(startup_clipboard_warning("Context: {{branch}}"), None);
        assert_eq!(startup_clipboard_warning(r"\{{clipboard}}"), None);
        let key = "prompt_variables.clipboard_in_startup";
        assert_eq!(startup_clipboard_warning("Context: {{clipboard}}").as_deref(),
                   Some(knot_core::l10n::t(key).as_str()));
        assert_ne!(knot_core::l10n::t(key), key, "{key} must resolve");
    }

    #[test]
    fn the_warning_names_each_unknown_variable() {
        assert_eq!(unknown_variables_warning("{{folder}} and \\{{x}}"), None);
        let warning = unknown_variables_warning("{{foldr}} {{brnach}}").expect("two unknown names");
        assert_ne!(warning, "prompt_variables.unknown", "the key must resolve");
        assert!(warning.contains("foldr") && warning.contains("brnach"),
                "{warning}");
    }

    /// "Startup prompt editor warns" for inputs: only those without a
    /// default are named, and the key resolves with the names in it.
    #[test]
    fn a_startup_prompt_names_inputs_without_a_default() {
        assert_eq!(startup_inputs_warning("{{user_input:Branch|develop}} {{branch}}"),
                   None);
        assert_eq!(startup_inputs_warning(r"\{{user_input:Ticket}}"), None);
        let warning = startup_inputs_warning("{{user_input:Ticket}} {{user_input:B|x}} {{user_input:PR}}")
            .expect("two inputs without a default");
        assert_ne!(warning, "prompt_variables.inputs_in_startup",
                   "the key must resolve");
        assert!(warning.contains("Ticket") && warning.contains("PR") && !warning.contains('B'),
                "{warning}");
    }

    /// "A user input is not unknown", "A blank-label input is flagged but
    /// saveable" and "More than five inputs are flagged but saveable".
    #[test]
    fn the_prompts_editor_warns_about_malformed_and_too_many_inputs() {
        assert!(library_prompt_warnings("{{user_input:Ticket}} {{folder}}").is_empty());

        let malformed = library_prompt_warnings("{{user_input:}}");
        assert_eq!(malformed,
                   [knot_core::l10n::t("prompt_variables.malformed_input")]);
        assert_ne!(malformed[0], "prompt_variables.malformed_input",
                   "the key must resolve");

        let five =
            "{{user_input:A}}{{user_input:B}}{{user_input:C}}{{user_input:D}}{{user_input:E}}";
        assert!(library_prompt_warnings(five).is_empty());
        let six = library_prompt_warnings(&format!("{five}{{{{user_input:F}}}}"));
        assert_eq!(six.len(), 1, "{six:?}");
        assert_ne!(six[0], "prompt_variables.too_many_inputs",
                   "the key must resolve");
        assert!(six[0].contains('5'),
                "the limit survives substitution: {}",
                six[0]);

        let both = library_prompt_warnings("{{foldr}} {{user_input: }}");
        assert_eq!(both.len(), 2, "{both:?}");
    }

    #[test]
    fn the_user_input_entry_describes_itself() {
        let key = "prompt_variables.user_input";
        assert_ne!(knot_core::l10n::t(key), key, "{key} must resolve");
    }
}
