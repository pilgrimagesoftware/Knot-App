//! Unit tests for [`super`]: every status colour against every surface it is
//! drawn on, in both appearances, by WCAG 2's contrast ratio. The bug (#12)
//! was a palette tuned on dark and never measured on light, so the surfaces
//! come from the theme itself rather than from constants copied out of it.

use gpui_kit::component::{ActiveTheme, Theme, ThemeMode};
use gpui_kit::{Hsla, Rgba, TestAppContext, hsla, rgb};

use super::status_palette;
use crate::consts::{DARK_STATUS_PALETTE, LIGHT_STATUS_PALETTE};

/// WCAG AA for text.
const TEXT_CONTRAST: f32 = 4.5;
/// WCAG AA for graphical objects: a dot, a border, a tint.
const SHAPE_CONTRAST: f32 = 3.0;

/// One surface a status colour is drawn on.
struct Surface {
    name:   &'static str,
    colour: Rgba,
}

/// The surfaces the status colours sit on: the window, the sidebar, a
/// selected sidebar row, the title bar, and a dashboard card - which is the
/// window under the card's own faint grey wash, composited here.
fn surfaces(theme: &Theme) -> [Surface; 5] {
    let background = opaque(theme.background, gpui_kit::white().into());
    [Surface { name:   "background",
               colour: background, },
     Surface { name:   "sidebar",
               colour: opaque(theme.sidebar, background), },
     Surface { name:   "selected row",
               colour: opaque(theme.muted, background), },
     Surface { name:   "title bar",
               colour: opaque(theme.title_bar, background), },
     Surface { name:   "dashboard card",
               colour: opaque(hsla(0., 0., 0.5, 0.04), background), }]
}

/// `colour` painted over the opaque `under`.
fn opaque(colour: Hsla, under: Rgba) -> Rgba {
    let over: Rgba = colour.into();
    let mix = |top: f32, bottom: f32| top * over.a + bottom * (1. - over.a);
    Rgba { r: mix(over.r, under.r),
           g: mix(over.g, under.g),
           b: mix(over.b, under.b),
           a: 1., }
}

/// WCAG 2 relative luminance: linearised sRGB channels, then weighted.
fn luminance(colour: Rgba) -> f32 {
    let linear = |channel: f32| {
        if channel <= 0.039_28 {
            channel / 12.92
        }
        else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(colour.r) + 0.7152 * linear(colour.g) + 0.0722 * linear(colour.b)
}

fn contrast(left: Rgba, right: Rgba) -> f32 {
    let (left, right) = (luminance(left), luminance(right));
    (left.max(right) + 0.05) / (left.min(right) + 0.05)
}

/// The theme `mode` loads, as the app would draw it.
fn theme(mode: ThemeMode, cx: &mut TestAppContext) -> Theme {
    cx.update(|cx| {
          gpui_kit::init(cx);
          Theme::change(mode, None, cx);
          cx.theme().clone()
      })
}

/// Every failing pairing for `mode`, so one run names them all.
fn failures(mode: ThemeMode, cx: &mut TestAppContext) -> Vec<String> {
    let theme = theme(mode, cx);
    let palette = cx.update(|cx| *status_palette(cx));
    let text = [("idle", palette.idle),
                ("running", palette.running),
                ("input", palette.input),
                ("error", palette.error),
                ("stopped", palette.stopped),
                ("card muted", palette.card_muted)];
    let shapes = [("pull request behind", palette.pull_request_behind),
                  ("pull request merged", palette.pull_request_merged)];
    let wanted = text.iter()
                     .map(|&(name, value)| (name, value, TEXT_CONTRAST))
                     .chain(shapes.iter()
                                  .map(|&(name, value)| (name, value, SHAPE_CONTRAST)));

    let mut failures = Vec::new();
    for (name, value, bar) in wanted {
        for surface in surfaces(&theme) {
            let ratio = contrast(rgb(value), surface.colour);
            if ratio < bar {
                failures.push(format!("{mode:?} {name} on {}: {ratio:.2}:1, needs {bar}:1",
                                      surface.name));
            }
        }
    }
    failures
}

#[gpui_kit::test]
fn light_status_colours_reach_contrast(cx: &mut TestAppContext) {
    let failures = failures(ThemeMode::Light, cx);
    assert!(failures.is_empty(), "{failures:#?}");
}

#[gpui_kit::test]
fn dark_status_colours_reach_contrast(cx: &mut TestAppContext) {
    let failures = failures(ThemeMode::Dark, cx);
    assert!(failures.is_empty(), "{failures:#?}");
}

/// The palette follows the theme: a light theme gets the light shades.
#[gpui_kit::test]
fn the_palette_follows_the_theme(cx: &mut TestAppContext) {
    theme(ThemeMode::Light, cx);
    assert_eq!(cx.update(|cx| *status_palette(cx)), LIGHT_STATUS_PALETTE);
    theme(ThemeMode::Dark, cx);
    assert_eq!(cx.update(|cx| *status_palette(cx)), DARK_STATUS_PALETTE);
}

/// The yardstick itself: black on white is the 21:1 ceiling, and a colour
/// against itself the 1:1 floor.
#[test]
fn contrast_spans_one_to_twenty_one() {
    let (black, white) = (gpui_kit::black().into(), gpui_kit::white().into());
    assert!((contrast(black, white) - 21.).abs() < 0.01);
    assert!((contrast(white, white) - 1.).abs() < 0.001);
}
