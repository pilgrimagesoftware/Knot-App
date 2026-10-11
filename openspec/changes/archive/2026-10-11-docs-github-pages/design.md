# Design

## Context

See `proposal.md` for motivation and the three specification deltas for behavior. The Rust application has no documentation-site capability. The site must remain a static GitHub Pages deployment, place source in `docs/site`, and present documentation from more than one released version without a server.

## Goals / Non-Goals

**Goals:**

- Build `docs/site` with Jekyll in GitHub Actions and deploy the generated output through the GitHub Pages artifact and deployment actions.
- Keep authored content grouped by a `latest` directory and version directories, with a data file declaring supported versions and labels.
- Make the root page resolve to the latest version and render a version selector from the data file.
- Reuse the Rust application's documented colors, typography, spacing, and approved icon assets after an implementation-time UI inventory.
- Place agent documentation instructions next to the site source.

**Non-Goals:**

- Serving documentation dynamically or adding a documentation CMS.
- Automatically deriving historical documentation from Git tags during the initial implementation.
- Guaranteeing a page-for-page mapping when an older version lacks a newer page.
- Publishing preview sites for pull requests.

## Decisions

### Use Jekyll with a repository-managed Pages workflow

The site will be built with Jekyll in GitHub Actions and deployed with the official Pages artifact and deployment actions. The workflow will use `pages: write` and `id-token: write` permissions, run on the configured default branch, and publish only after a successful build. A workflow file makes the deployment contract reviewable and permits local build parity.

GitHub's branch-source Pages build was considered. It provides less control over the Jekyll toolchain and deployment verification, so the workflow approach is selected.

### Keep versions as static source trees with a manifest

`docs/site` will contain a latest source tree, historical version trees, and a Jekyll data manifest that lists supported versions in display order. The build emits stable paths such as `/latest/...` and `/vX.Y/...`; the site root redirects or links to `/latest/`.

Generating historical documentation from Git tags was considered. It requires fetching and checking out older states in the Pages workflow and makes historical edits difficult to audit. Checked-in snapshots keep the published result repeatable and reviewable.

### Render static version navigation with a graceful page fallback

Each page renders a version selector using manifest data. The selector maps a current page path into the selected version when the page exists; otherwise it navigates to the selected version's landing page. It renders direct links so readers can select versions without JavaScript.

An external versioning plugin was considered. It adds a dependency without helping a finite static set of release snapshots.

### Define a shared Jekyll layout and application-derived design tokens

Layouts and includes own the header, sidebar, version selector, and footer. A site stylesheet defines semantic tokens sourced from the Rust application's theme, and documentation icons reuse its approved assets or icon library. An independent documentation theme was rejected because it would drift from application identity.

### Put authoring rules under the site root

`docs/site/AGENTS.md` will define directory roles, front matter, Markdown, Liquid boundaries, linking, assets, version snapshots, local Jekyll validation, and publishing. A scoped file keeps site instructions close to the content they govern.

## Risks / Trade-offs

- [Historical copies drift from their release state] -> Treat version directories as release snapshots and update them only for corrections that apply to that release.
- [Application styling changes] -> Centralize documentation token values and icon references in one theme layer and document their source locations.
- [A version lacks the current page] -> Send readers to the selected version's landing page and state this behavior in selector access text.
- [GitHub Pages is not configured] -> Document the repository Pages source setting and make the workflow fail visibly until deployment is enabled.
- [Local and CI Jekyll builds differ] -> Pin Ruby and Bundler inputs and use the same `bundle exec jekyll build` command in both places.

## Migration Plan

1. Add the Jekyll source tree, latest documentation seed, first historical snapshot, shared layout, and version manifest.
2. Add scoped authoring instructions and local build configuration.
3. Add the GitHub Pages workflow and configure the repository Pages source to GitHub Actions.
4. Verify the root path shows latest, each supported version is selectable, and a failed build leaves the active Pages deployment intact.
5. Roll back a faulty deployment by reverting the publishing commit or rerunning the workflow from the last known good commit.
