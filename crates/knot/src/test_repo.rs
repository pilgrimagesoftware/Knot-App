//! Throwaway git repositories for tests.
//!
//! Every fixture that commits goes through [`init`], so none of them
//! inherits the developer's global `commit.gpgsign`. A fixture that does
//! asks the developer's signing agent (1Password's `op-ssh-sign`, a GPG
//! pinentry) to sign a throwaway commit, and fails whenever that agent is
//! locked. CI never notices, because its runners don't sign.

use std::path::Path;

use knot_git::Runner;

/// Runs `git <args>` in `dir`, failing the test if git does.
pub(crate) fn git(dir: &Path, args: &[&str]) -> String {
    Runner::new(dir).run(args)
                    .unwrap_or_else(|error| panic!("git {args:?} in {}: {error}", dir.display()))
}

/// Makes `dir` a repository on `branch` with a local identity, and with
/// commit and tag signing off whatever the global config says.
pub(crate) fn init(dir: &Path, branch: &str) {
    git(dir, &["init", "-q", "-b", branch]);
    git(dir, &["config", "user.email", "test@example.com"]);
    git(dir, &["config", "user.name", "Test"]);
    git(dir, &["config", "commit.gpgsign", "false"]);
    git(dir, &["config", "tag.gpgsign", "false"]);
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::{git, init};

    /// The helper's whole point: a commit succeeds even when the global
    /// config demands a signature from a signer that always fails, because
    /// the repository's local setting outranks the global one.
    #[test]
    fn a_commit_ignores_a_global_signing_requirement() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let repo = dir.path().join("repo");
        std::fs::create_dir(&repo).expect("create the repository folder");
        init(&repo, "main");
        let global = dir.path().join("global.gitconfig");
        std::fs::write(&global,
                       "[commit]\n\tgpgsign = true\n[gpg]\n\tprogram = false\n")
            .expect("write the global config");

        let commit = Command::new("git").args(["commit", "-q", "--allow-empty", "-m", "unsigned"])
                                        .current_dir(&repo)
                                        .env("GIT_CONFIG_GLOBAL", &global)
                                        .output()
                                        .expect("git should be on PATH");

        assert!(commit.status.success(), "{commit:?}");
        assert_eq!(git(&repo, &["log", "-1", "--format=%G?"]), "N");
    }
}
