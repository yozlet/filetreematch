use filetreematch::config::ignore::IgnoreRules;
use std::path::Path;

#[test]
fn ignores_ds_store_by_name() {
    let rules = IgnoreRules::defaults();
    assert!(rules.should_ignore(Path::new("/archive/photos/.DS_Store")));
}

#[test]
fn ignores_git_directory_glob() {
    let rules = IgnoreRules::defaults();
    assert!(rules.should_ignore(Path::new("/archive/project/.git/config")));
}

#[test]
fn does_not_ignore_bashrc() {
    let rules = IgnoreRules::defaults();
    assert!(!rules.should_ignore(Path::new("/archive/home/.bashrc")));
}

#[test]
fn ignore_add_extends_rules() {
    let rules = IgnoreRules::defaults()
        .with_extra_globs(&["**/*.tmp"])
        .unwrap();
    assert!(rules.should_ignore(Path::new("/archive/file.tmp")));
}
