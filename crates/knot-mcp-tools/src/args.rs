use knot_mcp::ToolCallResult;
use serde_json::Value;

fn missing(key: &str) -> ToolCallResult {
    ToolCallResult::error(format!("Missing required parameter: {key}"))
}

/// A required string argument. Missing or wrong-typed both report the same
/// "Missing required parameter" text - the spec only distinguishes present
/// vs absent, not present-but-wrong-type.
pub fn require_str<'a>(args: &'a Value, key: &str) -> Result<&'a str, ToolCallResult> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| missing(key))
}

/// `value`, the path argument `key`, if it holds no ASCII control character
/// (C0 or DEL).
///
/// A path from a tool call can become the folder an agent's terminal starts
/// in, and a terminal's line editor acts on those characters wherever they
/// appear. A real folder name has no use for them, so they are refused here,
/// naming the parameter, rather than carried into a launch.
pub fn path_text<'a>(key: &str, value: &'a str) -> Result<&'a str, ToolCallResult> {
    if value.chars().any(|character| character.is_ascii_control()) {
        return Err(ToolCallResult::error(format!("{key} contains a control character, which a folder path cannot hold")));
    }
    Ok(value)
}

/// A required path argument: [`require_str`], then [`path_text`].
pub fn require_path<'a>(args: &'a Value, key: &str) -> Result<&'a str, ToolCallResult> {
    require_str(args, key).and_then(|value| path_text(key, value))
}

pub fn optional_str<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(Value::as_str)
}

pub fn optional_bool(args: &Value, key: &str) -> Option<bool> {
    args.get(key).and_then(Value::as_bool)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn require_str_returns_value_when_present() {
        let args = json!({"agentId": "abc"});
        assert_eq!(require_str(&args, "agentId").unwrap(), "abc");
    }

    #[test]
    fn require_str_errors_when_missing() {
        let args = json!({});
        let err = require_str(&args, "agentId").unwrap_err();
        assert_eq!(err.is_error, Some(true));
        assert!(err.content[0].text.contains("agentId"));
    }

    #[test]
    fn require_str_errors_when_wrong_type() {
        let args = json!({"agentId": 42});
        let err = require_str(&args, "agentId").unwrap_err();
        assert_eq!(err.is_error, Some(true));
        assert!(err.content[0].text.contains("agentId"));
    }

    #[test]
    fn a_path_with_a_control_character_is_refused() {
        for path in ["/tmp/a\u{15}b",
                     "/tmp/a\rb",
                     "/tmp/a\nb",
                     "/tmp/a\u{7f}b",
                     "/tmp/a\u{0}b"]
        {
            let err = require_path(&json!({ "repoPath": path }), "repoPath").unwrap_err();
            assert_eq!(err.is_error, Some(true), "{path:?}");
            assert!(err.content[0].text.contains("repoPath"), "{path:?}");
        }
    }

    #[test]
    fn a_path_with_spaces_quotes_and_unicode_is_kept() {
        let path = "/tmp/it's a $(name) \u{e9}\u{1f600}";
        assert_eq!(require_path(&json!({ "repoPath": path }), "repoPath").unwrap(),
                   path);
    }

    #[test]
    fn optional_str_returns_none_when_absent() {
        let args = json!({});
        assert_eq!(optional_str(&args, "sessionId"), None);
    }

    #[test]
    fn optional_bool_returns_value_when_present() {
        let args = json!({"markAsRead": false});
        assert_eq!(optional_bool(&args, "markAsRead"), Some(false));
    }

    #[test]
    fn optional_bool_returns_none_when_wrong_type() {
        let args = json!({"markAsRead": "false"});
        assert_eq!(optional_bool(&args, "markAsRead"), None);
    }
}
