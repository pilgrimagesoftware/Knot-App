use super::*;
use crate::user_inputs::InputValues;

fn context() -> PromptContext {
    PromptContext { agent_name: "Knot 3".into(),
                    agent_id:   "b7f0".into(),
                    agent_type: "claude".into(),
                    folder:     "/src/knot".into(),
                    workspace:  "Main".into(),
                    branch:     "471-restart".into(),
                    date:       "2026-09-25".into(),
                    clipboard:  "panic at main.rs:12".into(),
                    inputs:     InputValues::default(), }
}

#[test]
fn every_name_round_trips_and_names_are_case_sensitive() {
    for variable in PromptVariable::ALL {
        assert_eq!(variable.name().parse::<PromptVariable>(), Ok(variable));
        assert_eq!(variable.to_string(), variable.name());
    }
    assert_eq!("Folder".parse::<PromptVariable>(),
               Err(UnknownVariable("Folder".into())));
}

#[test]
fn each_variable_expands_to_its_value() {
    let ctx = context();
    let cases = [("{{agent.name}}", "Knot 3"),
                 ("{{agent.id}}", "b7f0"),
                 ("{{agent.type}}", "claude"),
                 ("{{folder}}", "/src/knot"),
                 ("{{folder.name}}", "knot"),
                 ("{{workspace}}", "Main"),
                 ("{{branch}}", "471-restart"),
                 ("{{date}}", "2026-09-25"),
                 ("{{clipboard}}", "panic at main.rs:12")];
    for (text, expected) in cases {
        assert_eq!(expand(text, &ctx), expected, "{text}");
    }
    assert_eq!(expand("Work in {{folder.name}} on {{branch}}", &ctx),
               "Work in knot on 471-restart");
}

#[test]
fn whitespace_inside_the_braces_is_allowed() {
    assert_eq!(expand("{{ agent.name }}", &context()), "Knot 3");
}

#[test]
fn an_unknown_name_is_left_as_typed() {
    assert_eq!(expand("see {{issue}} now", &context()), "see {{issue}} now");
}

#[test]
fn an_escaped_variable_is_literal_without_the_backslash() {
    assert_eq!(expand(r"\{{folder}}", &context()), "{{folder}}");
    assert_eq!(expand(r"a \{{b}} {{branch}}", &context()),
               "a {{b}} 471-restart");
}

#[test]
fn an_unclosed_brace_is_left_alone() {
    assert_eq!(expand("use {{ carefully", &context()), "use {{ carefully");
}

#[test]
fn a_value_is_not_expanded_twice() {
    let ctx = PromptContext { agent_name: "{{date}}".into(),
                              ..context() };
    assert_eq!(expand("{{agent.name}}", &ctx), "{{date}}");
}

#[test]
fn empty_values_expand_to_nothing() {
    let ctx = PromptContext { branch: String::new(),
                              ..context() };
    assert_eq!(expand("[{{branch}}]", &ctx), "[]");
}

#[test]
fn unknown_names_are_reported_once_in_order() {
    assert_eq!(unknown_variables("{{foldr}} {{ issue }} {{foldr}} {{branch}}"),
               ["foldr", "issue"]);
    assert!(unknown_variables(r"\{{foldr}} {{folder}} {{ unclosed").is_empty());
}

#[test]
fn clipboard_braces_are_not_expanded() {
    let ctx = PromptContext { clipboard: "{{branch}}".into(),
                              ..context() };
    assert_eq!(expand("Explain: {{clipboard}}", &ctx),
               "Explain: {{branch}}");
}

#[test]
fn uses_variable_skips_escaped_and_unknown_names() {
    assert!(uses_variable("Context: {{ clipboard }}", PromptVariable::Clipboard));
    assert!(!uses_variable(r"\{{clipboard}} {{clipboards}}", PromptVariable::Clipboard));
    assert!(!uses_variable("{{branch}}", PromptVariable::Clipboard));
}

/// `prompt-library` "User inputs": an input expands to the typed value, the
/// label's default, or nothing, in one pass with the variables.
#[test]
fn inputs_expand_with_the_variables() {
    let ctx = PromptContext { inputs: [("PR", "#466")].into_iter().collect(),
                              branch: "main".into(),
                              ..context() };
    assert_eq!(expand("Review {{user_input:PR}} on {{branch}}", &ctx),
               "Review #466 on main");
}

#[test]
fn a_repeated_label_uses_one_value() {
    let ctx = PromptContext { inputs: [("Name", "knot")].into_iter().collect(),
                              ..context() };
    assert_eq!(expand("{{user_input:Name}} / {{ user_input: Name }}", &ctx),
               "knot / knot");
}

#[test]
fn an_input_without_a_value_takes_the_labels_default() {
    let ctx = context();
    assert_eq!(expand("Work on {{user_input:Branch|develop}}", &ctx),
               "Work on develop");
    assert_eq!(expand("Ticket: {{user_input:Ticket}}", &ctx), "Ticket: ");
    // The label's default is the first non-empty one, so an earlier
    // occurrence without one still gets it.
    assert_eq!(expand("{{user_input:T}} {{user_input:T|x}} {{user_input:T|y}}",
                      &ctx),
               "x x x");
}

#[test]
fn a_typed_empty_value_beats_the_default() {
    let ctx = PromptContext { inputs: [("Branch", "")].into_iter().collect(),
                              ..context() };
    assert_eq!(expand("[{{user_input:Branch|develop}}]", &ctx), "[]");
}

#[test]
fn an_input_value_is_not_expanded_twice() {
    let ctx = PromptContext { inputs: [("X", "{{folder}}")].into_iter().collect(),
                              ..context() };
    assert_eq!(expand("{{user_input:X}}", &ctx), "{{folder}}");
}

#[test]
fn a_blank_label_and_an_escaped_input_are_left_as_typed() {
    assert_eq!(expand("{{user_input: }}", &context()), "{{user_input: }}");
    assert_eq!(expand("{{user_input:|x}}", &context()), "{{user_input:|x}}");
    assert_eq!(expand(r"\{{user_input:Name}}", &context()),
               "{{user_input:Name}}");
}

/// "A user input is not unknown" and "A misspelled keyword is unknown".
#[test]
fn inputs_are_not_unknown_but_a_misspelled_keyword_is() {
    assert!(unknown_variables("{{user_input:Ticket}} {{user_input:T|x}}").is_empty());
    assert!(unknown_variables("{{user_input:}}").is_empty());
    assert_eq!(unknown_variables("{{user-input:Ticket}} {{User_input:T}}"),
               ["user-input:Ticket", "User_input:T"]);
}
