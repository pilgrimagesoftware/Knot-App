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

Only the repository's own settings are checked, including those of its
checked-out submodules, whether or not `.gitmodules` lists them: Knot's status
and diff reads run git inside them too. A submodule nested more than eight
levels deep isn't read, so it counts as naming a program and the repository
needs trust.
The banner shows a submodule's settings with its path in front, such as
`[vendor/lib] filter.x.clean=…`. Your global `~/.gitconfig` and
the system configuration are yours, so a filter installed with
`git lfs install` (which writes to `~/.gitconfig`) doesn't prompt.

The settings that count:

- `filter.*.clean`, `smudge`, `process`
- `diff.*.command`
- `merge.*.driver`
- `core.sshCommand`, `core.editor`, `core.pager`, `core.askPass`,
  `core.gitProxy`, `core.alternateRefsCommand`
- `credential.helper`, `gpg.program`, `gpg.ssh.defaultKeyCommand`
- `hook.*.command`
- `sequence.editor`, `pager.*`
- `remote.*.uploadpack`, `receivepack`, `vcs`
- `trailer.*.cmd`, `command`
- `include.path`, `includeIf.*.path`
- `protocol` settings that allow `ext`

Some settings don't prompt, because Knot already turns them off on every git
call it makes on its own: `core.hooksPath`, `core.fsmonitor`, `diff.external`
and `diff.*.textconv`. A repository that uses a hooks folder (husky, for
example) isn't asked about. Its hooks don't run on Knot's own calls, but they
do run when you commit from the commit window, as they would in a terminal.

## What trust allows

In a trusted repository, Knot's own git calls run the programs its settings
name. That includes hooks defined in git settings (`hook.<name>.command`) for
events that Knot's calls trigger, such as `post-index-change` when it stages a
file and `post-checkout` when it creates a worktree. Hooks in the hooks folder
still don't run on Knot's own calls.

## Removing trust

**Settings > Coding > Trusted repositories** lists every trusted repository.
Remove one and confirm, and Knot asks again the next time it would run git there.

## Source contracts

- `openspec/specs/git-operations/spec.md`
