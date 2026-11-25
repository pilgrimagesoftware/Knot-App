//! Knot's look applied to the app: the embedded fonts registered with GPUI
//! and CoreText, the UI font as the default, and the platform palette laid
//! over the theme - again whenever the OS appearance flips.

use gpui_kit::App;
use gpui_kit::Window;
use gpui_kit::component::Theme;
use gpui_kit::component::ThemeTokens;
use gpui_kit::px;

use super::assets::*;

/// Registers the embedded faces with CoreText as well as with GPUI's text
/// system, for the process only.
///
/// GPUI's registration is private to GPUI, so AppKit resolves none of these
/// families by name: `NSFont::fontWithName_size("JetBrains Mono", _)` returns
/// `None`, and the font panel opens with nothing selected because
/// `setSelectedFont:` never gets a font to record. Registering here is what
/// lets the panel open on the font the row actually names.
///
/// Process scope deliberately: these faces ship with the app and have no
/// business outliving it in the user's font book.
#[cfg(target_os = "macos")]
fn register_embedded_fonts_with_core_text() {
    use objc2_core_foundation::CFData;
    use objc2_core_text::{
        CTFontManagerCreateFontDescriptorsFromData, CTFontManagerRegisterFontDescriptors,
        CTFontManagerScope,
    };

    for face in [ADAMINA_REGULAR,
                 MANROPE_REGULAR,
                 MANROPE_MEDIUM,
                 MANROPE_SEMIBOLD,
                 MANROPE_BOLD,
                 JETBRAINS_MONO_REGULAR,
                 JETBRAINS_MONO_BOLD]
    {
        let data = CFData::from_bytes(face);
        // SAFETY: `data` holds well-formed font data (the bytes are embedded
        // at build time), and the descriptors are used only for this call.
        let descriptors = unsafe { CTFontManagerCreateFontDescriptorsFromData(&data) };
        // SAFETY: the array holds `CTFontDescriptor`s, as the call above
        // returns. A `None` handler means failures are reported nowhere,
        // which is what a face already registered should do - nothing.
        unsafe {
            CTFontManagerRegisterFontDescriptors(&descriptors,
                                                 CTFontManagerScope::Process,
                                                 true,
                                                 None);
        }
    }
}

/// Registers the embedded font families and sets the UI font (Adamina) as the
/// app-wide default (the title font, Manrope, stays registered for the
/// workspace header, sidebar cell text, About credits, panel input and
/// Markdown headers that apply it explicitly), then lays the platform palette
/// over the theme, so the app doesn't rely on the platform's generic UI font
/// and neutral-gray default theme.
pub(crate) fn apply_visual_identity(settings: &knot_core::Settings, cx: &mut App) {
    if let Err(error) = cx.text_system()
                          .add_fonts(vec![std::borrow::Cow::Borrowed(ADAMINA_REGULAR),
                                          std::borrow::Cow::Borrowed(MANROPE_REGULAR),
                                          std::borrow::Cow::Borrowed(MANROPE_MEDIUM),
                                          std::borrow::Cow::Borrowed(MANROPE_SEMIBOLD),
                                          std::borrow::Cow::Borrowed(MANROPE_BOLD),
                                          std::borrow::Cow::Borrowed(JETBRAINS_MONO_REGULAR),
                                          std::borrow::Cow::Borrowed(JETBRAINS_MONO_BOLD),])
    {
        eprintln!("failed to register embedded fonts: {error}");
    }
    #[cfg(target_os = "macos")]
    register_embedded_fonts_with_core_text();

    // The app-wide default is the UI font (`ui_font_name`, Adamina), so
    // dialogs, buttons, settings labels and Markdown body text all inherit
    // it. The title font (`title_font_name`, Manrope) is applied explicitly
    // at the few sites that draw titles and headers.
    let theme = cx.global_mut::<Theme>();
    theme.font_family = settings.ui_font_name.clone().into();
    theme.mono_font_family = "JetBrains Mono".into();
    theme.font_size = px(settings.ui_font_size as f32);

    apply_system_palette(cx);
}

/// Lays the platform palette (see [`crate::macos::system_color`]) over the
/// theme: the accent family drives every tinted surface, the system's
/// dynamic neutrals drive the untinted ones.
///
/// Semantic colors are deliberately left alone - `danger`, `info` and the
/// agent-state colors carry meaning, and must read the same whatever tint
/// the user has chosen.
///
/// Safe to call again at any time: it reads the palette afresh and rewrites
/// only the colors it owns, which is how an appearance flip repaints (see
/// [`observe_system_appearance`]).
pub(crate) fn apply_system_palette(cx: &mut App) {
    let palette = crate::macos::system_color::resolve();
    let theme = cx.global_mut::<Theme>();

    theme.colors.primary = palette.accent;
    theme.colors.primary_hover = palette.accent_hover;
    theme.colors.primary_active = palette.accent_active;
    theme.colors.primary_foreground = palette.accent_foreground;
    theme.colors.button_primary = palette.accent;
    theme.colors.button_primary_hover = palette.accent_hover;
    theme.colors.button_primary_active = palette.accent_active;
    theme.colors.button_primary_foreground = palette.accent_foreground;
    // Focus reaches every control through `ring`, so one assignment tints
    // every focused border rather than each widget naming a color.
    theme.colors.ring = palette.accent;
    theme.colors.selection = palette.accent.opacity(0.25);

    // Absent off macOS, where the theme keeps the neutrals it ships with.
    if let Some(neutrals) = palette.neutrals {
        theme.colors.background = neutrals.window_background;
        theme.colors.title_bar = neutrals.window_background;
        // `secondary` is the theme's raised neutral surface - what the
        // panel's tool-call cards and message bubbles sit on - and `input`
        // is the field surface; both are controls, so both take
        // `controlBackgroundColor`.
        theme.colors.secondary = neutrals.control_background;
        theme.colors.input = neutrals.control_background;
        theme.colors.border = neutrals.separator;
        theme.colors.title_bar_border = neutrals.separator;
    }

    // `tokens` is a legacy snapshot of `colors` taken at construction time,
    // not re-derived on mutation (that's what Button/Switch actually read
    // for paint colors, e.g. `cx.theme().tokens.button_primary`).
    theme.tokens = ThemeTokens::from(&theme.colors);

    // Radius/scrollbar/typography reach rendering through a separately
    // mirrored Base layer that only `Theme::sync_base` re-derives.
    Theme::sync_base(cx);
}

/// Re-resolves the theme whenever the OS appearance changes under `window`,
/// so a light/dark flip repaints without a settings round-trip or a restart.
///
/// Through [`crate::appearance::apply`] rather than straight to
/// `Theme::change`: the OS appearance is one of two inputs, and under an
/// explicit Light or Dark it is not the deciding one. Handing
/// `cx.window_appearance()` to the theme here is exactly what used to
/// overwrite a chosen appearance on the next flip.
///
/// Every window registers this: the palette is global, so a second window
/// re-resolving it is harmless, and the app must keep tracking the appearance
/// after any one window closes.
pub(crate) fn observe_system_appearance(window: &Window) {
    window.observe_window_appearance(|window, cx| {
              crate::appearance::apply(Some(window), cx);
              window.refresh();
          })
          .detach();
}
