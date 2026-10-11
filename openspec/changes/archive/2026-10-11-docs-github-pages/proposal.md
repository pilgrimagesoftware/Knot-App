# Proposal

## Why

Knot needs a public, versioned reference site for the Rust application so users and contributors can find documentation that matches their installed release. Contributors and agents need shared authoring rules so published content remains consistent with the application.

## What Changes

- Add a Jekyll documentation site rooted at `docs/site` and publish it to GitHub Pages through GitHub Actions.
- Add release-versioned documentation with a selector for historical versions and the latest version selected by default.
- Establish documentation authoring instructions for agents covering structure, formatting, syntax, asset use, version updates, and publishing.
- Create a site shell whose colors, typography, navigation, and icons follow Knot's application style.

## Capabilities

### New Capabilities

- `documentation-publishing`: Builds and publishes the versioned Jekyll documentation site from `docs/site` to GitHub Pages.
- `documentation-authoring`: Defines the documentation structure and rules that agents follow when creating or updating content.
- `documentation-versioning`: Serves the latest documentation by default and lets readers select supported prior documentation versions.

### Modified Capabilities

None.

## Impact

Affected areas include the `docs/site` content and Jekyll configuration, GitHub Actions Pages deployment, release documentation assets, agent guidance, and application design tokens or icon assets reused by the site.
