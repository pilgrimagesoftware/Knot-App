# Documentation site rules

Edit site content only under `docs/site`.

- Put shared templates in `_layouts`, reusable fragments in `_includes`, and navigation data in `_data/versions.yml`.
- Put current pages in `latest/` and release snapshots in `vX.Y/`. Add every supported version to `_data/versions.yml`.
- Start every rendered Markdown page with front matter containing `title` and `version`.
- Use ATX headings, sentence-case link labels, fenced code blocks with a language, and relative internal links filtered through `relative_url` in Liquid templates.
- Keep Liquid limited to layouts and includes. Do not put secrets, credentials, or private URLs in site content.
- Add images and icons below `assets/`; reuse the application-derived visual tokens in `assets/css/site.css`.
- Theme sources: colors are `crates/knot/src/consts.rs` and `crates/knot/src/macos/system_color.rs`; the application icon is `images/knot-icon.png`; the main UI font is Adamina.
- Validate changes with `bundle install` and `bundle exec jekyll build` from this directory. Check the root, `/latest/`, and every version listed in `_data/versions.yml`.
- The Pages workflow publishes changes to `docs/site` from the default branch. Do not edit a historical snapshot except to correct release documentation.
