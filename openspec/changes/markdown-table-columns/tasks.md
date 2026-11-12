# Tasks

## 1. Size Markdown table columns from their content (#400)

- [x] 1.1 Measure each column's min- and max-content widths from shaped text,
      breaking where gpui's line wrapper breaks (a hyphenated word is whole)
- [x] 1.2 Size the default table layout's cells from those widths so every row
      lines up and words break only as a last resort
- [x] 1.3 Unit tests for the wrap points, the column flex settings and the
      measurement (`gpui-base`)
- [x] 1.4 Layout tests through Knot's Markdown view: a word column keeps its
      width while prose wraps, a hyphenated word keeps its column wide enough,
      and words break only when nothing fits
- [x] 1.5 Point Knot's `gpui-base` patch at the new fork branch and record the
      fork commit in an ADR
