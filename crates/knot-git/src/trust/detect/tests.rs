use super::*;

#[test]
fn a_listing_parses_into_scoped_entries() {
    let listing =
        "global\0file:/home/u/.gitconfig\0user.name\nU\0local\0file:.git/config\0core.bare\0";

    let entries = parse_listing(listing).unwrap();

    assert_eq!(entries.len(), 2);
    assert_eq!((entries[0].scope, entries[0].key, entries[0].value),
               ("global", "user.name", "U"));
    assert_eq!((entries[1].scope, entries[1].key, entries[1].value),
               ("local", "core.bare", ""));
}

#[test]
fn a_truncated_listing_is_an_error() {
    assert!(parse_listing("local\0file:.git/config\0").is_err());
}

#[test]
fn includes_are_recognised_and_resolved_beside_their_file() {
    assert!(is_include("include.path"));
    assert!(is_include("includeIf.gitdir:~/src/.path"));
    assert!(!is_include("filter.x.clean"));
    assert_eq!(include_target(Path::new("/r/.git/config"), "../x.cfg"),
               Path::new("/r/.git/../x.cfg"));
    assert_eq!(include_target(Path::new("/r/.git/config"), "/abs.cfg"),
               Path::new("/abs.cfg"));
}

/// A repository with `vendor/library` checked out as a submodule, which
/// configures nothing.
fn app_with_submodule(dir: &Path) -> PathBuf {
    let git = |folder: &Path, args: &[&str]| {
        Runner::new(folder).without_trust_check().run(args).unwrap();
    };
    let init = |folder: &Path| {
        fs::create_dir_all(folder).unwrap();
        git(folder, &["init", "-q", "-b", "main"]);
        git(folder, &["config", "user.email", "test@example.com"]);
        git(folder, &["config", "user.name", "Test"]);
        git(folder, &["config", "commit.gpgsign", "false"]);
        fs::write(folder.join("f.txt"), "one\n").unwrap();
        git(folder, &["add", "."]);
        git(folder, &["commit", "-qm", "root"]);
    };
    let (library, app) = (dir.join("library"), dir.join("app"));
    init(&library);
    init(&app);
    git(&app,
        &["-c",
          "protocol.file.allow=always",
          "submodule",
          "add",
          "-q",
          library.to_str().unwrap(),
          "vendor/library"]);
    app
}

/// Past the depth limit a submodule is not read, so its configuration is
/// unknown: it counts as naming a program, and the repository needs trust,
/// rather than counting as naming none.
#[test]
fn a_submodule_past_the_depth_limit_needs_trust() {
    let dir = tempfile::tempdir().unwrap();
    let app = app_with_submodule(dir.path());

    let within = read_repository(&app, 0, 1).unwrap().expect("a repository");
    let past = read_repository(&app, 0, 0).unwrap().expect("a repository");

    assert!(within.settings.is_empty(), "{:?}", within.settings);
    assert_eq!(past.settings,
               vec![ProgramSetting { submodule: Some("vendor/library".to_owned()),
                                     key:       UNREAD_SUBMODULE_KEY.to_owned(),
                                     value:     UNREAD_SUBMODULE_VALUE.to_owned(), }]);
}
