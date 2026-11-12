//! Markdown table column sizing (#400; `acp-panel-ui`, "Markdown tables size
//! their columns from their content"), drawn through Knot's own
//! [`crate::markdown_view`] so the layout under test is the one the panel
//! uses.
//!
//! The first column holds single words, so its narrowest and widest content
//! are the same width: it should keep that width whenever the table can
//! afford it, and the prose column beside it should do the wrapping.

use gpui_kit::AppContext;
use gpui_kit::Bounds;
use gpui_kit::Context;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Pixels;
use gpui_kit::Render;
use gpui_kit::Styled;
use gpui_kit::TestAppContext;
use gpui_kit::VisualTestContext;
use gpui_kit::Window;
use gpui_kit::WindowOptions;
use gpui_kit::component::Root;
use gpui_kit::div;
use gpui_kit::px;

use crate::markdown_view::markdown_view;

const TABLE: &str = "\
| Fix | Notes |
| --- | --- |
| Now | Tear down the stale runners and move the remaining jobs onto the shared pool before the freeze. |
| Observability | Ship the dashboards and alerts for the new pipeline, with runbooks linked from every alert. |
| Watson | Retire the old reporting job once the replacement has run cleanly for a full week. |
";
/// The #411 review case: a column whose widest word carries a hyphen. gpui
/// never wraps at a hyphen, so the column has to fit the whole word.
const HYPHENATED: &str = "\
| When | Notes |
| --- | --- |
| Short-term | Tear down the stale runners and move the remaining jobs onto the shared pool before the freeze. |
| Long-term | Ship the dashboards and alerts for the new pipeline, with runbooks linked from every alert. |
| Near-term | Retire the old reporting job once the replacement has run cleanly for a full week. |
";
const ROWS: usize = 4;
const WIDE: f32 = 1200.;
const NARROW: f32 = 360.;
/// Too narrow for "Observability" beside any prose at all.
const CRAMPED: f32 = 120.;

struct TableProbe {
    width:  f32,
    source: &'static str,
}

impl Render for TableProbe {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().w(px(self.width)).child(markdown_view("table-probe",
                                                    self.source,
                                                    "Helvetica".into(),
                                                    "Helvetica".into(),
                                                    px(14.)))
    }
}

/// Every cell's bounds, by row and column, with `source` drawn `width` wide.
fn cells(source: &'static str, width: f32, cx: &mut TestAppContext) -> Vec<[Bounds<Pixels>; 2]> {
    let window = cx.update(|cx| {
                       gpui_kit::init(cx);
                       cx.open_window(WindowOptions::default(), |window, cx| {
                             let view = cx.new(|_| TableProbe { width, source });
                             cx.new(|cx| Root::new(view, window, cx))
                         })
                         .expect("the probe window should open")
                   });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();
    (0..ROWS).map(|row| [cell(&mut cx, row, 0), cell(&mut cx, row, 1)])
             .collect()
}

fn cell(cx: &mut VisualTestContext, row: usize, column: usize) -> Bounds<Pixels> {
    // `debug_bounds` wants a `'static` selector; the table is small and fixed.
    const SELECTORS: [[&str; 2]; ROWS] = [["markdown-table-cell-0-0", "markdown-table-cell-0-1"],
                                          ["markdown-table-cell-1-0", "markdown-table-cell-1-1"],
                                          ["markdown-table-cell-2-0", "markdown-table-cell-2-1"],
                                          ["markdown-table-cell-3-0", "markdown-table-cell-3-1"]];
    cx.debug_bounds(SELECTORS[row][column])
      .unwrap_or_else(|| panic!("cell {row},{column} is laid out"))
}

fn width(bounds: Bounds<Pixels>) -> f32 {
    f32::from(bounds.size.width)
}

fn assert_columns_line_up(table: &[[Bounds<Pixels>; 2]]) {
    for column in 0..2 {
        let first = table[0][column];
        for row in table {
            assert_eq!(row[column].origin.x, first.origin.x,
                       "column {column} starts together");
            assert_eq!(row[column].size.width, first.size.width,
                       "column {column} is one width");
        }
    }
}

#[gpui_kit::test]
fn a_word_column_keeps_its_width_while_the_prose_column_wraps(cx: &mut TestAppContext) {
    let wide = cells(TABLE, WIDE, cx);
    let narrow = cells(TABLE, NARROW, cx);
    assert_columns_line_up(&wide);
    assert_columns_line_up(&narrow);

    let (wide_words, narrow_words) = (width(wide[0][0]), width(narrow[0][0]));
    assert!((narrow_words - wide_words).abs() <= 0.5,
            "the word column keeps its width ({narrow_words} narrow, {wide_words} wide): \
             no word in it breaks while the prose column has room to give");
    assert!(width(narrow[0][1]) < width(wide[0][1]),
            "the prose column is the one that narrows");
    assert!(narrow[2][1].size.height > wide[2][1].size.height,
            "and its text wraps onto more lines");
}

#[gpui_kit::test]
fn words_break_only_when_the_table_cannot_fit_them(cx: &mut TestAppContext) {
    let wide = cells(TABLE, WIDE, cx);
    let cramped = cells(TABLE, CRAMPED, cx);
    assert_columns_line_up(&cramped);

    assert!(width(cramped[0][0]) < width(wide[0][0]),
            "with no room left anywhere, the word column gives way too");
    assert!(width(cramped[0][0]) + width(cramped[0][1]) <= CRAMPED,
            "and the table still fits its frame");
}

/// A hyphenated word is one word to the renderer, so its column keeps room
/// for all of it rather than for the part before the hyphen.
#[gpui_kit::test]
fn a_hyphenated_word_keeps_its_column_wide_enough(cx: &mut TestAppContext) {
    let wide = cells(HYPHENATED, WIDE, cx);
    let narrow = cells(HYPHENATED, NARROW, cx);
    assert_columns_line_up(&narrow);

    let (wide_words, narrow_words) = (width(wide[0][0]), width(narrow[0][0]));
    assert!((narrow_words - wide_words).abs() <= 0.5,
            "the hyphenated column keeps its width ({narrow_words} narrow, {wide_words} wide): \
             \"Short-term\" does not break while the prose column has room to give");
}
