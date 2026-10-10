use super::*;

#[test]
fn accepts_a_normal_relative_path() {
    assert_eq!(resolve("personas/x.md").unwrap(), "personas/x.md");
}

#[test]
fn rejects_a_parent_dir_escape() {
    assert!(resolve("../../.ssh/config").is_err());
}

#[test]
fn rejects_an_absolute_path() {
    assert!(resolve("/etc/passwd").is_err());
}
