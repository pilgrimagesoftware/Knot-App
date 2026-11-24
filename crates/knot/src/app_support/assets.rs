//! What ships inside the binary: the embedded fonts, the app icon and the
//! working-knot animation, the asset source that serves them to GPUI, and
//! the two elements drawn straight from them.

use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::Styled;
use gpui_kit::px;

/// Embedded fonts (all SIL OFL licensed; see the matching `*-LICENSE.txt`
/// under `assets/fonts/`), so the app looks the same regardless of what's
/// installed on the system:
/// - Adamina: the UI font's default (`ui_font_name`), which is the app-wide
///   default family - dialogs, buttons, settings labels, agent names, and
///   Markdown body text - the renderer synthesizes bold for
///   `font_semibold`/`font_bold` text set in it.
/// - Manrope: the title font's default (`title_font_name`), applied explicitly
///   at the sites that draw titles and headers: the workspace header, the agent
///   sidebar cell text that isn't the agent's name (persona, status, folder,
///   git stats), the About window's credit lines, the panel input's send hint,
///   and Markdown headers.
/// - JetBrains Mono: the default terminal font (`terminal_font_name`'s
///   default), a real monospace coding font rather than a mono variant of the
///   UI font, reliably resolvable regardless of what's installed on the system.
pub(crate) const ADAMINA_REGULAR: &[u8] = include_bytes!("../../assets/fonts/Adamina-Regular.ttf");
pub(crate) const MANROPE_REGULAR: &[u8] = include_bytes!("../../assets/fonts/Manrope-Regular.ttf");
pub(crate) const MANROPE_MEDIUM: &[u8] = include_bytes!("../../assets/fonts/Manrope-Medium.ttf");
pub(crate) const MANROPE_SEMIBOLD: &[u8] =
    include_bytes!("../../assets/fonts/Manrope-SemiBold.ttf");
pub(crate) const MANROPE_BOLD: &[u8] = include_bytes!("../../assets/fonts/Manrope-Bold.ttf");
pub(crate) const JETBRAINS_MONO_REGULAR: &[u8] =
    include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf");
pub(crate) const JETBRAINS_MONO_BOLD: &[u8] =
    include_bytes!("../../assets/fonts/JetBrainsMono-Bold.ttf");

pub(crate) const APP_ICON_PNG: &[u8] = include_bytes!("../../assets/app-icon-32.png");

/// The panel's turn-in-progress animation: the app icon pulsing through one
/// full cycle, 36 frames of animated WebP at 42ms each. Scaled down from the
/// authored source at `images/knot-progress-pulse.webp` by
/// `scripts/make-working-animation.sh`, which is committed beside it.
pub(crate) const WORKING_KNOT_WEBP: &[u8] = include_bytes!("../../assets/working-knot.webp");

/// The path [`KnotAssets`] serves [`WORKING_KNOT_WEBP`] at, and the string
/// [`working_knot_animation`] hands to `img` to ask for it.
pub(crate) const WORKING_KNOT_PATH: &str = "working-knot.webp";

/// Knot's own embedded assets, layered over gpui-kit's icon catalog.
///
/// The working animation has to reach `img` as an embedded *resource path*
/// rather than as `Image::from_bytes`. Those are two different decoders in
/// GPUI and only one of them animates a WebP: `Image::to_image_data`, which
/// is what an `Arc<Image>` source goes through, sends every format but GIF
/// to `decode_static_image` and yields a single frame. The resource loader
/// behind `img("some/path")` asks `WebPDecoder::has_animation` first and
/// decodes every frame. Serving the bytes from here is what puts the asset
/// on the second path, and is why this exists rather than a sibling of
/// [`app_titlebar_icon`] built the same way that one is.
///
/// `AllAssets` reports a miss as an `Err` rather than `Ok(None)`, so our own
/// path is answered first and everything else is delegated untouched.
pub(crate) struct KnotAssets;

impl gpui_kit::AssetSource for KnotAssets {
    fn load(&self, path: &str) -> gpui_kit::Result<Option<std::borrow::Cow<'static, [u8]>>> {
        if path == WORKING_KNOT_PATH {
            return Ok(Some(std::borrow::Cow::Borrowed(WORKING_KNOT_WEBP)));
        }
        gpui_kit::AssetSource::load(&gpui_kit::assets::AllAssets, path)
    }

    fn list(&self, path: &str) -> gpui_kit::Result<Vec<gpui_kit::SharedString>> {
        let mut listed = gpui_kit::AssetSource::list(&gpui_kit::assets::AllAssets, path)?;
        if WORKING_KNOT_PATH.starts_with(path) {
            listed.push(WORKING_KNOT_PATH.into());
        }
        Ok(listed)
    }
}

/// A small app-icon glyph for the leading edge of a custom `TitleBar`, sat
/// between the traffic lights and the title text.
pub(crate) fn app_titlebar_icon() -> impl IntoElement {
    let image = std::sync::Arc::new(gpui_kit::Image::from_bytes(gpui_kit::ImageFormat::Png,
                                                                APP_ICON_PNG.to_vec()));
    gpui_kit::img(image).w(px(16.))
                        .h(px(16.))
                        .rounded(px(4.))
                        .flex_shrink_0()
}

/// The conversation's turn-in-progress mark: the app icon, animating.
///
/// Everything that makes it an animation belongs to GPUI's `img` - it
/// advances frames on the delay each one declares, holds frame 0 when
/// `reduce_motion` is set, stops while the window is inactive, and requests
/// its own animation frames. Knot schedules nothing, which is the whole
/// reason this is an asset rather than a frame counter like
/// [`crate::working_indicator`]'s.
///
/// The `id` is not decoration. `img` keeps the frame index in element state
/// and both the advance and the `request_animation_frame` call are gated on
/// the element having a global id; without one it renders frame 0 forever
/// and looks like a still image that failed to load. One id is enough
/// because the pane renders one panel at a time.
///
/// The box is fixed in both dimensions so the row reserves the same space
/// animating or held still, per `acp-panel-ui`'s no-reflow scenario.
pub(crate) fn working_knot_animation() -> impl IntoElement {
    gpui_kit::img(WORKING_KNOT_PATH).id("working-knot")
                                    .w(px(24.))
                                    .h(px(24.))
                                    .flex_shrink_0()
}
