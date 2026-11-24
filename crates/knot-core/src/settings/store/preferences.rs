//! Load-time upgrades of the preferences document.
//!
//! Owns decoding `preferences.json` into a [`Settings`] and every upgrade that
//! runs on the way: the `"SF Mono"` terminal font, the sidebar width clamp,
//! the proportional-font role swap gated on `settingsVersion`, and the report
//! of an unreadable vocabulary value. It does not read or write any file; the
//! loaders in [`super`] and [`super::legacy`] hand it the decoded object.

use serde_json::Value;

use super::Settings;
use crate::consts::{
    SETTINGS_VERSION_CURRENT, SIDEBAR_WIDTH_MAX, SIDEBAR_WIDTH_MIN, TERMINAL_FONT_DEFAULT,
};
use crate::settings::vocabulary::{AiProvider, AppearanceMode, AutopilotAction, UnknownVariant};

impl Settings {
    /// Decode a preferences object and apply the load-time upgrades. The
    /// collections are left empty; the caller fills them from their own
    /// documents.
    ///
    /// Also the path a legacy document takes once its collection keys have
    /// been lifted out, so an upgrading installation gets exactly the same
    /// upgrades as a document already in the new arrangement.
    pub(super) fn from_preferences(mut value: Value) -> Self {
        migrate_font_roles(&mut value);
        report_unreadable_vocabularies(&value);

        let mut settings: Self = serde_json::from_value(value).unwrap_or_default();
        // "SF Mono" was the terminal font default before JetBrains Mono
        // replaced it; a persisted document from before that change still
        // carries the old value, and SF Mono isn't reliably resolvable
        // through GPUI's font lookup (unlike AppKit, which special-cases
        // it), silently falling back to the UI font. Upgrade it once,
        // the same way a never-customized document already would default.
        if settings.terminal_font_name == "SF Mono" {
            settings.terminal_font_name = TERMINAL_FONT_DEFAULT.to_string();
        }
        // A hand-edited or future-written width outside the divider's range
        // describes an intent the window cannot honour; the nearest legal
        // width is closer to it than the default is. Clamping here rather
        // than in the window keeps every reader of the setting free of the
        // bound.
        settings.sidebar_width = settings.sidebar_width
                                         .clamp(SIDEBAR_WIDTH_MIN, SIDEBAR_WIDTH_MAX);
        settings
    }
}

/// The version a document carrying no `settingsVersion` key was written under:
/// the pre-swap font roles.
pub(super) fn de_legacy_settings_version() -> u32 {
    0
}

/// Says so, once, when a stored vocabulary value could not be read.
///
/// Deserialization is deliberately tolerant - one bad field must not take the
/// whole document down - so without this the substitution is invisible and a
/// corrupt value behaves exactly like the real default. That
/// indistinguishability is what issue #224 was about.
fn report_unreadable_vocabularies(value: &Value) {
    fn check<T: std::str::FromStr<Err = UnknownVariant> + Default + std::fmt::Display>(value: &Value,
                                                                                       field: &str)
    {
        let Some(stored) = value.get(field).and_then(Value::as_str)
        else {
            return;
        };
        if let (_, Some(unknown)) = {
            let parsed = stored.parse::<T>();
            match parsed {
                Ok(v) => (v, None),
                Err(e) => (T::default(), Some(e)),
            }
        } {
            eprintln!("knot-core: settings field `{field}`: {unknown}; using {}",
                      T::default());
        }
    }

    check::<AppearanceMode>(value, "appearanceMode");
    check::<AiProvider>(value, "aiProvider");
    check::<AutopilotAction>(value, "autopilotAction");
}

/// Exchange the two proportional font settings in a pre-swap document.
///
/// Before the roles were swapped, `uiFontName` / `uiFontSize` held the title
/// face and `titleFontName` / `titleFontSize` held the application-wide
/// default. Exchanging the values keeps a user's customized fonts on the text
/// they were chosen for.
///
/// Runs on the raw document because only the raw document says which keys the
/// user actually customized: after deserializing, an absent key is
/// indistinguishable from one holding the default, so exchanging both would
/// invert the defaults for every user who never picked a font. An absent key
/// therefore stays absent and takes the new default.
///
/// Gated on `settingsVersion`, so it runs exactly once. The migrated value is
/// not written eagerly; the next persist records it, the same as the
/// `"SF Mono"` upgrade.
pub(super) fn migrate_font_roles(document: &mut Value) {
    let Some(object) = document.as_object_mut()
    else {
        return;
    };
    let version = object.get("settingsVersion")
                        .and_then(Value::as_u64)
                        .unwrap_or_else(|| u64::from(de_legacy_settings_version()));
    if version >= u64::from(SETTINGS_VERSION_CURRENT) {
        return;
    }

    exchange_entries(object, "uiFontName", "titleFontName");
    exchange_entries(object, "uiFontSize", "titleFontSize");

    object.insert("settingsVersion".to_string(),
                  Value::from(SETTINGS_VERSION_CURRENT));
}

/// Move the values under `left` and `right` past each other. A key the
/// document does not carry is left absent rather than created, so a setting
/// the user never customized keeps its default instead of inheriting the
/// other's value.
fn exchange_entries(object: &mut serde_json::Map<String, Value>, left: &str, right: &str) {
    match (object.remove(left), object.remove(right)) {
        (Some(was_left), Some(was_right)) => {
            object.insert(left.to_string(), was_right);
            object.insert(right.to_string(), was_left);
        }
        (Some(was_left), None) => {
            object.insert(right.to_string(), was_left);
        }
        (None, Some(was_right)) => {
            object.insert(left.to_string(), was_right);
        }
        (None, None) => {}
    }
}
