## Purpose

Adds repository trust to `git-operations`: git runs no program that a
repository's own configuration names until the user trusts that repository.

## ADDED Requirements

### Requirement: Repository trust

Before running git in a folder, the system SHALL read the repository's own
configuration entries (scopes `local` and `worktree`, following includes)
and find the keys that name a program for git to run. If there are any and
the trust policy doesn't trust the repository with exactly those entries,
the call SHALL NOT run, SHALL fail with an untrusted-repository error, and
SHALL be reported to the policy.

#### Scenario: An untrusted clean filter

- **WHEN** a repository configures `filter.x.clean` for a file and Knot
  stages that file before the repository is trusted
- **THEN** the filter does not run and the call fails as untrusted

#### Scenario: A trusted clean filter

- **WHEN** the user has trusted the repository with that filter
- **THEN** staging the file runs the filter

#### Scenario: A program added after trust

- **WHEN** a trusted repository's configuration gains another program entry
- **THEN** the next git call there fails as untrusted until it is trusted
  again

#### Scenario: A hook defined in configuration

- **WHEN** a repository configures `hook.<name>.command`
- **THEN** git calls there fail as untrusted until it is trusted

#### Scenario: A repository that names no programs

- **WHEN** a repository's own configuration names no programs
- **THEN** git runs there without asking

### Requirement: Keys that name programs

The keys checked SHALL include `filter.*.{clean,smudge,process}`,
`diff.*.command`, `merge.*.driver`,
`core.{sshCommand,editor,pager,askPass,gitProxy,alternateRefsCommand}`,
`credential[.*].helper`, `gpg[.*].program`, `gpg.ssh.defaultKeyCommand`,
`hook.*.command`, `sequence.editor`, `pager.*`,
`remote.*.{uploadpack,receivepack,vcs}`, `trailer.*.{cmd,command}`,
`include.path`, `includeIf.*.path`, and `protocol` settings that enable `ext`.

#### Scenario: A filter driver

- **WHEN** a repository configures `filter.x.smudge`
- **THEN** it names a program

### Requirement: Keys Knot already turns off do not prompt

Keys Knot turns off on every call it makes on its own - `core.hooksPath`,
`core.fsmonitor`, `diff.external`, `diff.*.textconv` - SHALL NOT be
checked.

#### Scenario: A hooks folder alone

- **WHEN** a repository's only such setting is `core.hooksPath`
- **THEN** git runs there without asking, and the folder's hooks do not run
  on Knot's own calls

#### Scenario: A textconv alone

- **WHEN** a repository's only such setting is `diff.x.textconv`
- **THEN** Knot reads its diffs without asking and without running it

### Requirement: Submodules answer with their repository

The configuration of each checked-out submodule, nested ones included,
SHALL be checked with the repository's own, each entry tagged with the
submodule's path.

#### Scenario: A submodule's clean filter

- **WHEN** a checked-out submodule configures a clean filter for a changed
  file and Knot reads the repository's status
- **THEN** the filter does not run until the repository is trusted

### Requirement: Asking the user

The workspace window SHALL show the first repository waiting for an answer,
listing each configured program with control characters shown as `\xNN`,
with Trust and Not now. Trust SHALL be saved per repository with the
entries it was given. Not now SHALL stop asking until the next launch,
and the git panel SHALL offer to review such a repository again. Settings
SHALL list trusted repositories, each removable after confirmation.

#### Scenario: Trusting from the banner

- **WHEN** the user chooses Trust on the banner
- **THEN** the repository and its entries are saved and the banner closes

#### Scenario: Reviewing a repository after Not now

- **WHEN** the user chose Not now and then chooses Review in the git panel
- **THEN** the banner asks about the repository again

#### Scenario: Removing trust

- **WHEN** the user removes a trusted repository in Settings and confirms
- **THEN** the next git call there is refused and asked about again
