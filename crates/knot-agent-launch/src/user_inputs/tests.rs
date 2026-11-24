use super::*;

fn input(label: &str, default: Option<&str>) -> UserInput {
    UserInput { label:   label.into(),
                default: default.map(str::to_string), }
}

/// "An input is listed once per label".
#[test]
fn inputs_are_listed_once_per_label_in_first_seen_order() {
    let text =
        "Fix {{user_input:Ticket}} and close {{user_input:Ticket}} on {{user_input:Branch|main}}";
    assert_eq!(user_inputs(text),
               [input("Ticket", None), input("Branch", Some("main"))]);
}

/// "A later occurrence can give the default".
#[test]
fn the_first_non_empty_default_wins() {
    assert_eq!(user_inputs("{{user_input:T}} then {{user_input:T|x}} then {{user_input:T|y}}"),
               [input("T", Some("x"))]);
    assert_eq!(user_inputs("{{user_input:T|}} {{user_input:T| z }}"),
               [input("T", Some("z"))]);
}

#[test]
fn labels_and_defaults_are_trimmed_and_case_sensitive() {
    assert_eq!(user_inputs("{{ user_input: Focus | }} {{user_input:focus}}"),
               [input("Focus", None), input("focus", None)]);
    assert_eq!(user_inputs("{{user_input:Q|a | b}}"),
               [input("Q", Some("a | b"))]);
}

/// "A blank label is left as typed" and "An escaped input is literal".
#[test]
fn blank_escaped_and_unclosed_inputs_are_not_inputs() {
    assert!(user_inputs("{{user_input: }} {{user_input:|x}}").is_empty());
    assert!(user_inputs(r"\{{user_input:Name}}").is_empty());
    assert!(user_inputs("{{user_input:Name").is_empty());
    assert!(user_inputs("{{ user_input :Name}} {{branch}}").is_empty());
}

#[test]
fn a_blank_label_is_malformed() {
    assert!(malformed_inputs("ok {{user_input: }}"));
    assert!(malformed_inputs("{{user_input:|default}}"));
    assert!(!malformed_inputs("{{user_input:Name}} {{foldr}}"));
    assert!(!malformed_inputs(r"\{{user_input:}}"));
}

#[test]
fn input_values_collect_and_look_up_by_label() {
    let mut values: InputValues = [("A", "1")].into_iter().collect();
    values.insert("B", "2");
    values.insert("A", "3");
    assert_eq!((values.get("A"), values.get("B"), values.get("C")),
               (Some("3"), Some("2"), None));
}
