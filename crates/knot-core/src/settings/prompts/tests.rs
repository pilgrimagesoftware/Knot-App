use serde_json::json;

use super::*;

#[test]
fn prompt_refuses_blank_name_or_text() {
    assert!(Prompt::new("  ", "text").is_none());
    assert!(Prompt::new("name", "\n\t ").is_none());
    assert!(Prompt::new("name", "text").is_some());
}

#[test]
fn prompt_keeps_multi_line_text_verbatim() {
    let prompt = Prompt::new("gate", "one\ntwo\n  three").unwrap();
    let reloaded: Prompt = serde_json::from_str(&serde_json::to_string(&prompt).unwrap()).unwrap();
    assert_eq!(reloaded, prompt);
    assert_eq!(reloaded.text, "one\ntwo\n  three");
}

#[test]
fn startup_prompt_variants_round_trip() {
    let id = Uuid::new_v4();
    for value in [StartupPrompt::Library(id),
                  StartupPrompt::Custom("do it".into())]
    {
        let text = serde_json::to_string(&value).unwrap();
        assert_eq!(serde_json::from_str::<StartupPrompt>(&text).unwrap(), value);
    }
    assert_eq!(serde_json::to_value(StartupPrompt::Library(id)).unwrap(),
               json!({ "kind": "library", "value": id }));
}

#[test]
fn blank_custom_text_is_none() {
    assert_eq!(StartupPrompt::custom("   "), None);
    assert_eq!(StartupPrompt::custom("go"),
               Some(StartupPrompt::Custom("go".into())));
}

#[test]
fn copy_name_numbers_past_names_already_taken() {
    let first = copy_name("Review", |_| false);
    assert_ne!(first, "Review");
    assert!(first.contains("Review"),
            "the copy keeps the original's name: {first}");

    let second = copy_name("Review", |candidate| candidate == first);
    assert_ne!(second, first);
    assert!(second.contains("Review") && second.contains('2'),
            "the first numbered form is 2: {second}");

    let third = copy_name("Review", |candidate| {
        candidate == first || candidate == second
    });
    assert!(third.contains('3'), "{third}");
}

#[test]
fn copy_name_keys_resolve() {
    for key in ["settings.prompts.copy_name",
                "settings.prompts.copy_name_numbered"]
    {
        assert_ne!(crate::l10n::t(key), key, "{key} must resolve");
    }
    assert!(crate::l10n::t_with("settings.prompts.copy_name_numbered",
                                &[("name", "x"), ("number", "7")]).contains('7'),
            "the number must survive substitution");
}
