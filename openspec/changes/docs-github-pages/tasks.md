# Tasks

## 1. Site foundation and authoring rules

- [ ] 1.1 Inventory the Rust application's authoritative colors, typography, spacing, and icon assets, record their source locations for the documentation theme, and verify the selected assets render in the application.
- [ ] 1.2 Create the `docs/site` Jekyll configuration, dependency lockfiles, shared layouts, includes, and semantic stylesheet tokens, then verify `bundle exec jekyll build` completes locally.
- [ ] 1.3 Create `docs/site/AGENTS.md` with rules for directory structure, front matter, Markdown, Liquid, links, assets, local validation, release snapshots, and publishing, then verify a new page can be authored from its instructions without undocumented conventions.
- [ ] 1.4 Seed the latest documentation landing page and navigation using the shared layout and application-derived theme, then verify the local site renders navigation, styles, and approved icons.

## 2. Versioned documentation navigation

- [ ] 2.1 Add the supported-version manifest and the latest plus initial historical documentation source trees, then verify the manifest lists each published version in display order.
- [ ] 2.2 Implement the root-to-latest entry point and version-aware Jekyll layout, then verify opening the built root output displays the latest documentation.
- [ ] 2.3 Implement the visible version selector with static fallback links and corresponding-page routing where available, then verify a reader can select the historical version and reaches its documentation.
- [ ] 2.4 Implement the fallback for pages absent in a selected version and accessible selected-version labeling, then verify switching from an unavailable page reaches the selected version landing page with its version identified.

## 3. GitHub Pages publication

- [ ] 3.1 Add the GitHub Actions workflow that builds `docs/site`, uploads the Pages artifact, and deploys it with least-privilege Pages permissions, then verify workflow syntax and the Jekyll build command pass locally or in CI.
- [ ] 3.2 Configure workflow triggers for documentation source and workflow changes on the publishing branch, then verify a documentation change queues the build while a failed build cannot reach the deploy job.
- [ ] 3.3 Document the repository GitHub Pages configuration and release verification steps, then verify a successful workflow deployment serves the root, latest, and historical-version URLs.

## 4. Integration validation

- [ ] 4.1 Run the pinned local Jekyll build and link checks for the latest and historical trees, then verify the generated site has no broken internal version-selector targets.
- [ ] 4.2 Review the published layout against the Rust application's documented visual tokens and icon assets, then verify navigation, the version selector, and primary layout match those sources.

## Workflow follow-up

- Archive the change after the repository's review requirements are satisfied.
- Verify the archived result.
