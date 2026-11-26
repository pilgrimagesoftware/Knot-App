//! Which configuration keys name a program for git to run.
//!
//! Only keys git consults during the commands Knot runs (status, diff,
//! numstat, ls-files, add/reset/restore, commit, branch, rev-parse, remote,
//! worktree add) are listed. Keys that only commands Knot never runs read -
//! `alias.*`, `difftool.*.cmd`, `mergetool.*.cmd`, `browser.*.cmd`,
//! `man.*.cmd`, `guitool.*.cmd`, `web.browser`, `submodule.*.update`,
//! `sendemail.*` - are left out: they cannot run through Knot.

use std::fmt;

/// One repository configuration entry that names a program, as git reports
/// it: section and key lower-cased, a subsection as written.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProgramSetting {
    pub key:   String,
    pub value: String,
}

impl ProgramSetting {
    /// The value with every control character written as `\xNN`, so what a
    /// person is shown is exactly what is configured - a carriage return or
    /// a line-erase in a value cannot hide the text before it.
    pub fn visible_value(&self) -> String {
        visible(&self.value)
    }
}

impl fmt::Display for ProgramSetting {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}={}", visible(&self.key), self.visible_value())
    }
}

fn visible(text: &str) -> String {
    text.chars()
        .map(|character| {
            if character.is_ascii_control() {
                format!("\\x{:02x}", u32::from(character))
            }
            else {
                character.to_string()
            }
        })
        .collect()
}

/// Whether `key` (as `git config --list` prints it) set to `value` makes git
/// run a program, or read another file that could.
pub(crate) fn names_a_program(key: &str, value: &str) -> bool {
    let key = key.to_ascii_lowercase();
    let Some((section, rest)) = key.split_once('.')
    else {
        return false;
    };
    // A subsection may itself hold dots (`credential.https://host.helper`),
    // so the name is whatever follows the last one.
    let (subsection, name) = match rest.rsplit_once('.') {
        Some((subsection, name)) => (Some(subsection), name),
        None => (None, rest),
    };
    match (section, subsection, name) {
        ("core", None, "fsmonitor") | ("pager", None, _) => !is_boolean(value),
        ("core",
         None,
         "hookspath"
         | "sshcommand"
         | "editor"
         | "pager"
         | "askpass"
         | "gitproxy"
         | "alternaterefscommand")
        | ("filter", Some(_), "clean" | "smudge" | "process")
        | ("diff", Some(_), "command" | "textconv")
        | ("diff", None, "external")
        | ("merge", Some(_), "driver")
        | ("credential", _, "helper")
        | ("gpg", _, "program")
        | ("gpg", Some("ssh"), "defaultkeycommand")
        | ("hook", Some(_), "command")
        | ("sequence", None, "editor")
        | ("remote", Some(_), "uploadpack" | "receivepack" | "vcs")
        | ("trailer", Some(_), "cmd" | "command")
        | ("include", None, "path")
        | ("includeif", Some(_), "path") => true,
        // `ext::` runs a command as a transport; it is off unless allowed.
        ("protocol", None, "allow") => value.eq_ignore_ascii_case("always"),
        ("protocol", Some("ext"), "allow") => !value.eq_ignore_ascii_case("never"),
        _ => false,
    }
}

/// git's boolean spellings: for `core.fsmonitor`, `true` selects git's own
/// built-in monitor rather than a program.
fn is_boolean(value: &str) -> bool {
    ["true", "false", "yes", "no", "on", "off", "1", "0", ""].iter()
                                                             .any(|word| {
                                                                 value.eq_ignore_ascii_case(word)
                                                             })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_that_run_programs_are_recognised() {
        for (key, value) in [("filter.lfs.clean", "git-lfs clean -- %f"),
                             ("filter.x.smudge", "cat"),
                             ("filter.x.process", "x"),
                             ("diff.x.textconv", "cat"),
                             ("diff.x.command", "x"),
                             ("diff.external", "x"),
                             ("merge.x.driver", "x"),
                             ("core.fsmonitor", "/tmp/monitor"),
                             ("core.hooksPath", "hooks"),
                             ("core.sshCommand", "ssh"),
                             ("core.editor", "vi"),
                             ("core.pager", "less"),
                             ("core.askPass", "x"),
                             ("credential.helper", "store"),
                             ("credential.https://example.com.helper", "x"),
                             ("gpg.program", "gpg"),
                             ("gpg.ssh.program", "x"),
                             ("gpg.ssh.defaultKeyCommand", "x"),
                             ("hook.lint.command", "x"),
                             ("sequence.editor", "x"),
                             ("pager.log", "less"),
                             ("remote.origin.uploadpack", "x"),
                             ("remote.origin.vcs", "x"),
                             ("trailer.sign.command", "x"),
                             ("include.path", "../x.cfg"),
                             ("includeIf.gitdir:~/src/.path", "x.cfg"),
                             ("protocol.ext.allow", "always"),
                             ("protocol.allow", "always")]
        {
            assert!(names_a_program(key, value), "{key}={value}");
        }
    }

    #[test]
    fn keys_that_run_nothing_through_knot_are_not() {
        for (key, value) in [("core.fsmonitor", "true"),
                             ("core.fsmonitor", "false"),
                             ("pager.log", "false"),
                             ("core.bare", "false"),
                             ("diff.x.binary", "true"),
                             ("merge.x.name", "x"),
                             ("user.email", "a@example.com"),
                             ("remote.origin.url", "x"),
                             ("alias.co", "!rm -rf"),
                             ("protocol.ext.allow", "never"),
                             ("protocol.allow", "user"),
                             ("notakey", "x")]
        {
            assert!(!names_a_program(key, value), "{key}={value}");
        }
    }

    #[test]
    fn a_value_with_control_characters_is_shown_escaped() {
        let setting = ProgramSetting { key:   "filter.x.clean".to_owned(),
                                       value: "safe\u{15}\rrm -rf ~".to_owned(), };

        assert_eq!(setting.to_string(), "filter.x.clean=safe\\x15\\x0drm -rf ~");
    }
}
