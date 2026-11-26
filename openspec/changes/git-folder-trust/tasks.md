## 1. knot-git

- [x] 1.1 Key matcher for program-naming configuration keys, with unit tests
- [x] 1.2 Detection with `rev-parse` and `config --list --show-scope --show-origin --includes -z`, cached per folder and invalidated by the stamps of the config, origin and include-target files
- [x] 1.3 `TrustPolicy`, `configure_trust`, `repository_programs`, and the `Runner` gate returning `GitError::Untrusted`
- [x] 1.4 Integration tests: untrusted clean filter, trusted filter, program added after trust, configured hook, included filter, repository with no programs

## 2. knot-core

- [x] 2.1 `Settings::trusted_repositories` with persistence tests

## 3. knot

- [x] 3.1 `repository_trust` registry installed at startup, with unit tests
- [x] 3.2 Workspace banner (Trust / Not now) reached through `repaint_poll_tick`, with GPUI tests
- [x] 3.3 Settings > Coding "Trusted repositories" with confirmed removal
- [x] 3.4 l10n keys under `repository_trust`
- [x] 3.5 Docs: `docs/site/latest/reference/repository-trust.md`
