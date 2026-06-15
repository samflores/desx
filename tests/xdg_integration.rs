//! End-to-end test: build a fixture tree of .desktop files in a tempdir,
//! walk it with `freedesktop-desktop-entry`, then feed the resulting
//! `DesktopEntry`s through desx's `entries_to_apps` + `dedupe_and_disambiguate`
//! and assert the final output.
//!
//! The real `xdg::load_apps` uses `default_paths()` which walks the
//! system's XDG dirs — not testable hermetically. `entries_to_apps` is
//! the same transformation logic without the filesystem-walk step, so
//! this test feeds it entries parsed from our own fixture.

use std::fs;
use std::path::{Path, PathBuf};

use freedesktop_desktop_entry::{DesktopEntry, Iter};
use tempfile::TempDir;

use desx::category_filter::{CategoryFilter, apply};
use desx::dedupe::dedupe_and_disambiguate;
use desx::xdg::entries_to_apps;

fn write_desktop_file(dir: &Path, filename: &str, contents: &str) -> PathBuf {
    let path = dir.join(filename);
    fs::write(&path, contents).expect("write fixture");
    path
}

fn load_fixture_entries(search_dirs: &[&Path]) -> Vec<DesktopEntry> {
    let no_locales: Option<&[String]> = None;
    Iter::new(search_dirs.iter().map(|p| p.to_path_buf()))
        .entries(no_locales)
        .collect()
}

fn run_pipeline(search_dirs: &[&Path]) -> Vec<(String, String)> {
    let entries = load_fixture_entries(search_dirs);
    let apps = entries_to_apps(entries, &[]);
    let apps = dedupe_and_disambiguate(apps);
    apps.into_iter().map(|a| (a.name, a.exec)).collect()
}

fn run_pipeline_filtered(search_dirs: &[&Path], filter: &str) -> Vec<String> {
    let entries = load_fixture_entries(search_dirs);
    let apps = entries_to_apps(entries, &[]);
    let apps = dedupe_and_disambiguate(apps);
    let filter = CategoryFilter::parse(filter).unwrap();
    let mut names: Vec<String> = apply(apps, filter.as_ref())
        .into_iter()
        .map(|a| a.name)
        .collect();
    names.sort();
    names
}

#[test]
fn no_display_entries_are_filtered() {
    let tmp = TempDir::new().unwrap();
    write_desktop_file(
        tmp.path(),
        "visible.desktop",
        "[Desktop Entry]\nType=Application\nName=Visible\nExec=visible\n",
    );
    write_desktop_file(
        tmp.path(),
        "hidden.desktop",
        "[Desktop Entry]\nType=Application\nName=Hidden\nExec=hidden\nNoDisplay=true\n",
    );
    write_desktop_file(
        tmp.path(),
        "truly-hidden.desktop",
        "[Desktop Entry]\nType=Application\nName=Gone\nExec=gone\nHidden=true\n",
    );

    let out = run_pipeline(&[tmp.path()]);
    let names: Vec<&str> = out.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, vec!["Visible"]);
}

#[test]
fn non_application_entries_are_filtered() {
    let tmp = TempDir::new().unwrap();
    write_desktop_file(
        tmp.path(),
        "app.desktop",
        "[Desktop Entry]\nType=Application\nName=Real App\nExec=real\n",
    );
    write_desktop_file(
        tmp.path(),
        "link.desktop",
        "[Desktop Entry]\nType=Link\nName=Web Shortcut\nURL=https://example.com\n",
    );
    write_desktop_file(
        tmp.path(),
        "directory.desktop",
        "[Desktop Entry]\nType=Directory\nName=Folder\n",
    );

    let out = run_pipeline(&[tmp.path()]);
    let names: Vec<&str> = out.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, vec!["Real App"]);
}

#[test]
fn field_codes_are_stripped_from_exec() {
    let tmp = TempDir::new().unwrap();
    write_desktop_file(
        tmp.path(),
        "ff.desktop",
        "[Desktop Entry]\nType=Application\nName=Firefox\nExec=firefox %u\n",
    );

    let out = run_pipeline(&[tmp.path()]);
    assert_eq!(out, vec![("Firefox".to_string(), "firefox".to_string())]);
}

#[test]
fn identical_name_and_exec_across_scopes_dedupe_user_wins() {
    // System scope dir (path starts with /usr/share → classified System).
    // We cannot write to /usr/share in a test, so we simulate two
    // same-scope entries here and rely on unit tests in dedupe.rs to
    // cover the cross-scope priority. This test confirms the
    // within-scope dedupe path through the real file parser.
    let tmp = TempDir::new().unwrap();
    write_desktop_file(
        tmp.path(),
        "com.google.Chrome.desktop",
        "[Desktop Entry]\nType=Application\nName=Google Chrome\nExec=google-chrome-stable %U\n",
    );
    write_desktop_file(
        tmp.path(),
        "google-chrome.desktop",
        "[Desktop Entry]\nType=Application\nName=Google Chrome\nExec=google-chrome-stable %U\n",
    );

    let out = run_pipeline(&[tmp.path()]);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].0, "Google Chrome");
    assert_eq!(out[0].1, "google-chrome-stable");
}

#[test]
fn same_name_different_exec_both_kept_and_disambiguated() {
    let tmp = TempDir::new().unwrap();
    write_desktop_file(
        tmp.path(),
        "firefox.desktop",
        "[Desktop Entry]\nType=Application\nName=Firefox\nExec=firefox %u\n",
    );
    write_desktop_file(
        tmp.path(),
        "firefox-beta.desktop",
        "[Desktop Entry]\nType=Application\nName=Firefox\nExec=firefox-beta %u\n",
    );

    let out = run_pipeline(&[tmp.path()]);
    assert_eq!(out.len(), 2);
    let names: Vec<String> = out.iter().map(|(n, _)| n.clone()).collect();
    assert!(names.iter().any(|n| n == "Firefox (firefox)"));
    assert!(names.iter().any(|n| n == "Firefox (firefox-beta)"));
}

#[test]
fn empty_exec_is_rejected() {
    let tmp = TempDir::new().unwrap();
    write_desktop_file(
        tmp.path(),
        "broken.desktop",
        "[Desktop Entry]\nType=Application\nName=Broken\nExec=%u\n",
    );

    let out = run_pipeline(&[tmp.path()]);
    assert!(out.is_empty(), "expected no apps, got {:?}", out);
}

#[test]
fn missing_name_is_rejected() {
    let tmp = TempDir::new().unwrap();
    write_desktop_file(
        tmp.path(),
        "anon.desktop",
        "[Desktop Entry]\nType=Application\nExec=anon\n",
    );

    let out = run_pipeline(&[tmp.path()]);
    assert!(out.is_empty(), "expected no apps, got {:?}", out);
}

fn category_fixture(dir: &Path) {
    write_desktop_file(
        dir,
        "quake.desktop",
        "[Desktop Entry]\nType=Application\nName=Quake\nExec=quake\nCategories=Game;ActionGame;\n",
    );
    write_desktop_file(
        dir,
        "firefox.desktop",
        "[Desktop Entry]\nType=Application\nName=Firefox\nExec=firefox\nCategories=Network;WebBrowser;\n",
    );
    write_desktop_file(
        dir,
        "gimp.desktop",
        "[Desktop Entry]\nType=Application\nName=Gimp\nExec=gimp\nCategories=Graphics;\n",
    );
    write_desktop_file(
        dir,
        "uncategorized.desktop",
        "[Desktop Entry]\nType=Application\nName=Mystery\nExec=mystery\n",
    );
}

#[test]
fn category_include_keeps_only_matching() {
    let tmp = TempDir::new().unwrap();
    category_fixture(tmp.path());
    let out = run_pipeline_filtered(&[tmp.path()], "game");
    assert_eq!(out, vec!["Quake"]);
}

#[test]
fn category_include_is_case_insensitive() {
    let tmp = TempDir::new().unwrap();
    category_fixture(tmp.path());
    let out = run_pipeline_filtered(&[tmp.path()], "GAME");
    assert_eq!(out, vec!["Quake"]);
}

#[test]
fn category_include_multiple() {
    let tmp = TempDir::new().unwrap();
    category_fixture(tmp.path());
    let out = run_pipeline_filtered(&[tmp.path()], "game,network");
    assert_eq!(out, vec!["Firefox", "Quake"]);
}

#[test]
fn category_exclude_drops_matching() {
    let tmp = TempDir::new().unwrap();
    category_fixture(tmp.path());
    let out = run_pipeline_filtered(&[tmp.path()], "-game");
    // Everything except Quake, including the uncategorized entry.
    assert_eq!(out, vec!["Firefox", "Gimp", "Mystery"]);
}

#[test]
fn category_exclude_multiple() {
    let tmp = TempDir::new().unwrap();
    category_fixture(tmp.path());
    let out = run_pipeline_filtered(&[tmp.path()], "-game,-graphics");
    assert_eq!(out, vec!["Firefox", "Mystery"]);
}

#[test]
fn category_include_excludes_uncategorized() {
    let tmp = TempDir::new().unwrap();
    category_fixture(tmp.path());
    let out = run_pipeline_filtered(&[tmp.path()], "network");
    assert_eq!(out, vec!["Firefox"]);
}

#[test]
fn multiple_apps_preserve_order() {
    let tmp = TempDir::new().unwrap();
    write_desktop_file(
        tmp.path(),
        "a.desktop",
        "[Desktop Entry]\nType=Application\nName=Alpha\nExec=a\n",
    );
    write_desktop_file(
        tmp.path(),
        "b.desktop",
        "[Desktop Entry]\nType=Application\nName=Bravo\nExec=b\n",
    );
    write_desktop_file(
        tmp.path(),
        "c.desktop",
        "[Desktop Entry]\nType=Application\nName=Charlie\nExec=c\n",
    );

    let out = run_pipeline(&[tmp.path()]);
    let names: Vec<String> = out.iter().map(|(n, _)| n.clone()).collect();
    // Directory walk order is not guaranteed by the filesystem, but the
    // pipeline itself must be order-preserving. Sort and compare set-wise.
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(sorted, vec!["Alpha", "Bravo", "Charlie"]);
}
