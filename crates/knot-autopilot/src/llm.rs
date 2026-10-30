//! Calls the configured provider with a system prompt and the agent's last
//! message, returning the raw text response.
//!
//! One function handles both the classification call and the custom-action
//! call: they differ only in which system prompt is sent and whether a token
//! cap applies, not in request shape. Ported from the Swift reference's
//! `callOpenAI(Custom)`/`callAnthropic(Custom)`/`callGoogle(Custom)`, which
//! were six near-identical functions for the same reason.

use std::time::Duration;

use knot_core::AiProvider;
use serde_json::{Value, json};

use crate::error::AutopilotError;

/// Model Autopilot asks for per provider — the cheapest/fastest option,
/// matching `crates/knot/src/settings_window/panes/autopilot.rs`'s
/// `ai_model_for`, which the settings pane calls through here so the two
/// never drift apart.
pub fn ai_model_for(provider: AiProvider) -> &'static str {
    match provider {
        AiProvider::OpenAi => "gpt-5-mini",
        AiProvider::Anthropic => "claude-haiku-4-5",
        AiProvider::Google => "gemini-flash-lite-latest",
        // A paid model: OpenCode refuses its free ones (HTTP 403) to clients
        // other than OpenCode itself. Served over Chat Completions, which
        // the GPT and Claude models on Zen are not.
        AiProvider::OpenCode => "deepseek-v4-flash",
        AiProvider::OpenRouter => "openai/gpt-5-mini",
    }
}

/// The Chat Completions endpoint of each provider that speaks OpenAI's
/// request shape. OpenCode Zen and OpenRouter both do, so they share
/// `call_chat_completions` with OpenAI and differ only here (#254).
const OPENAI_CHAT_COMPLETIONS: &str = "https://api.openai.com/v1/chat/completions";
const OPENCODE_CHAT_COMPLETIONS: &str = "https://opencode.ai/zen/v1/chat/completions";
const OPENROUTER_CHAT_COMPLETIONS: &str = "https://openrouter.ai/api/v1/chat/completions";

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Calls `provider` with `system_prompt` as the system message and
/// `user_message` as the user turn, returning the text of the response.
///
/// `max_tokens` caps the response length for providers whose API exposes
/// that knob per-request (Anthropic); OpenAI and Google are called with no
/// cap, matching the Swift reference, which never set one for those two.
pub fn complete(provider: AiProvider, api_key: &str, system_prompt: &str, user_message: &str,
                max_tokens: Option<u32>)
                -> Result<String, AutopilotError> {
    let api_key = api_key.trim();
    if api_key.is_empty() {
        return Err(AutopilotError::MissingApiKey);
    }
    let model = ai_model_for(provider);
    match provider {
        AiProvider::OpenAi => call_chat_completions(OPENAI_CHAT_COMPLETIONS,
                                                    api_key,
                                                    model,
                                                    system_prompt,
                                                    user_message),
        AiProvider::OpenCode => call_chat_completions(OPENCODE_CHAT_COMPLETIONS,
                                                      api_key,
                                                      model,
                                                      system_prompt,
                                                      user_message),
        AiProvider::OpenRouter => call_chat_completions(OPENROUTER_CHAT_COMPLETIONS,
                                                        api_key,
                                                        model,
                                                        system_prompt,
                                                        user_message),
        AiProvider::Anthropic => call_anthropic(api_key,
                                                model,
                                                system_prompt,
                                                user_message,
                                                max_tokens.unwrap_or(16)),
        AiProvider::Google => call_google(api_key, model, system_prompt, user_message),
    }
}

fn request_failed(error: impl std::fmt::Display) -> AutopilotError {
    AutopilotError::Request(error.to_string())
}

/// One OpenAI-style Chat Completions request to `url`, with the key as a
/// bearer token.
fn call_chat_completions(url: &str, api_key: &str, model: &str, system_prompt: &str,
                         user_message: &str)
                         -> Result<String, AutopilotError> {
    let body = json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_message},
        ],
    });
    let response: Value = ureq::post(url).header("Authorization", format!("Bearer {api_key}"))
                                         .config()
                                         .timeout_global(Some(REQUEST_TIMEOUT))
                                         .build()
                                         .send_json(&body)
                                         .map_err(request_failed)?
                                         .body_mut()
                                         .read_json()
                                         .map_err(request_failed)?;
    Ok(response.pointer("/choices/0/message/content")
               .and_then(Value::as_str)
               .unwrap_or_default()
               .to_string())
}

fn call_anthropic(api_key: &str, model: &str, system_prompt: &str, user_message: &str,
                  max_tokens: u32)
                  -> Result<String, AutopilotError> {
    let body = json!({
        "model": model,
        "system": system_prompt,
        "max_tokens": max_tokens,
        "messages": [
            {"role": "user", "content": user_message},
        ],
    });
    let response: Value =
        ureq::post("https://api.anthropic.com/v1/messages").header("x-api-key", api_key)
                                                           .header("anthropic-version",
                                                                   "2023-06-01")
                                                           .config()
                                                           .timeout_global(Some(REQUEST_TIMEOUT))
                                                           .build()
                                                           .send_json(&body)
                                                           .map_err(request_failed)?
                                                           .body_mut()
                                                           .read_json()
                                                           .map_err(request_failed)?;
    Ok(response.pointer("/content/0/text")
               .and_then(Value::as_str)
               .unwrap_or_default()
               .to_string())
}

fn call_google(api_key: &str, model: &str, system_prompt: &str, user_message: &str)
               -> Result<String, AutopilotError> {
    let url = format!("https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={api_key}");
    let body = json!({
        "contents": [{"parts": [{"text": user_message}]}],
        "systemInstruction": {"parts": [{"text": system_prompt}]},
    });
    let response: Value = ureq::post(url.as_str()).config()
                                                  .timeout_global(Some(REQUEST_TIMEOUT))
                                                  .build()
                                                  .send_json(&body)
                                                  .map_err(request_failed)?
                                                  .body_mut()
                                                  .read_json()
                                                  .map_err(request_failed)?;
    // The first text part, matching the Swift reference's `for part in
    // parts { if case .text = part { return text } }`, which returns on the
    // first match rather than joining every part.
    let text = response.pointer("/candidates/0/content/parts")
                       .and_then(Value::as_array)
                       .and_then(|parts| {
                           parts.iter()
                                .find_map(|part| part.get("text").and_then(Value::as_str))
                       })
                       .unwrap_or_default();
    Ok(text.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_mapping_matches_settings_pane() {
        assert_eq!(ai_model_for(AiProvider::OpenAi), "gpt-5-mini");
        assert_eq!(ai_model_for(AiProvider::Anthropic), "claude-haiku-4-5");
        assert_eq!(ai_model_for(AiProvider::Google), "gemini-flash-lite-latest");
        assert_eq!(ai_model_for(AiProvider::OpenCode), "deepseek-v4-flash");
        assert_eq!(ai_model_for(AiProvider::OpenRouter), "openai/gpt-5-mini");
    }

    /// #254: OpenCode Zen and OpenRouter take OpenAI's request shape at their
    /// own Chat Completions endpoints, from their providers' docs.
    #[test]
    fn the_gateways_chat_completions_endpoints() {
        assert_eq!(OPENCODE_CHAT_COMPLETIONS,
                   "https://opencode.ai/zen/v1/chat/completions");
        assert_eq!(OPENROUTER_CHAT_COMPLETIONS,
                   "https://openrouter.ai/api/v1/chat/completions");
    }

    /// A new provider is still refused without a key, before any request.
    #[test]
    fn the_gateways_fail_fast_without_a_key() {
        for provider in [AiProvider::OpenCode, AiProvider::OpenRouter] {
            assert!(matches!(complete(provider, " ", "system", "user", None),
                             Err(AutopilotError::MissingApiKey)),
                    "{provider}");
        }
    }

    #[test]
    fn empty_api_key_is_rejected_before_any_request() {
        let result = complete(AiProvider::OpenAi, "   ", "system", "user", None);
        assert!(matches!(result, Err(AutopilotError::MissingApiKey)));
    }
}
