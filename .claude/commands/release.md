---
description: Cut a release - preview the git-cliff version, dispatch Prepare Release, and confirm the release PR, tag, GitHub Release, assets, cask and merge-back
argument-hint: "[expected-version]"
---

Invoke the `release-repo` skill for this repo.

`$ARGUMENTS` is optional: the version (`x.y.z`) the user expects. The workflow takes no version
input - git-cliff computes it from the commits since the last tag - so the skill compares this
against its preview and stops on a mismatch instead of overriding it.
