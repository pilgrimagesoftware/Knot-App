---
title: Repository trust
version: latest
permalink: /latest/reference/repository-trust/
---

# Repository trust

Knot runs git in your agents' folders on its own: it reads status and diffs
whenever a window is open, and it creates worktrees when an agent asks for one.

A repository's own git settings (`.git/config`, a worktree's `config.worktree`,
and any file they include) can name programs that git runs during those
commands. For example:

```ini
[filter "lfs"]
	clean = git-lfs clean -- %f
	smudge = git-lfs smudge -- %f
```

Git runs `clean` when it reads status or stages a file, and `smudge` when it
checks files out. Knot runs no git in a repository whose settings name a
program until you trust that repository.

## The banner

When Knot finds such a repository, the workspace window shows a banner naming
the repository and each setting, exactly as configured. Any control characters
are shown as `\xNN`.

- **Trust** saves the repository together with the settings shown, and git
  runs there from then on.
- **Not now** stops asking until Knot is next started. Until you trust the
  repository, Knot doesn't run git there: change counts stay empty, and an
  agent's `create-worktree` call fails. The git panel explains why it's empty,
  and its **Review** button shows the banner again.

Trust covers exactly the settings that were shown. If the repository's settings
change later, for example a pulled file that the repository includes adds a
filter, Knot asks again.

## Settings checked

Only the repository's own settings are checked. Your global `~/.gitconfig` and
the system configuration are yours, so a filter installed with
`git lfs install` (which writes to `~/.gitconfig`) doesn't prompt.

The settings that count:

- `filter.*.clean`, `smudge`, `process`
- `diff.*.command`, `textconv`, and `diff.external`
- `merge.*.driver`
- `core.fsmonitor` (unless `true` or `false`), `core.hooksPath`,
  `core.sshCommand`, `core.editor`, `core.pager`, `core.askPass`,
  `core.gitProxy`, `core.alternateRefsCommand`
- `credential.helper`, `gpg.program`, `gpg.ssh.defaultKeyCommand`
- `hook.*.command`
- `sequence.editor`, `pager.*`
- `remote.*.uploadpack`, `receivepack`, `vcs`
- `trailer.*.cmd`, `command`
- `include.path`, `includeIf.*.path`
- `protocol` settings that allow `ext`

## Removing trust

**Settings > Coding > Trusted repositories** lists every trusted repository.
Remove one and confirm, and Knot asks again the next time it would run git there.

## Source contracts

- `openspec/specs/git-operations/spec.md`
