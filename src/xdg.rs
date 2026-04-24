//! Thin wrapper around `freedesktop-desktop-entry` that walks the XDG
//! search paths, filters out entries that shouldn't be shown, resolves
//! icons to absolute paths, and returns plain `DesktopApp` records.

use std::path::Path;

use freedesktop_desktop_entry::{DesktopEntry, Iter, PathSource, default_paths};
use freedesktop_icons::lookup;

use crate::desktop_entry::{DesktopApp, Scope};
use crate::exec_cleanup::strip_field_codes;

pub fn load_apps(locales: &[String]) -> Vec<DesktopApp> {
    let entries: Vec<DesktopEntry> = Iter::new(default_paths()).entries(Some(locales)).collect();
    entries
        .into_iter()
        .filter_map(|entry| to_desktop_app(&entry, locales))
        .collect()
}

fn to_desktop_app(entry: &DesktopEntry, locales: &[String]) -> Option<DesktopApp> {
    if entry.no_display() || entry.hidden() {
        return None;
    }
    if entry.type_() != Some("Application") {
        return None;
    }

    let name = entry.name(locales)?.to_string();
    let exec_raw = entry.exec()?;
    let exec = strip_field_codes(exec_raw);
    if exec.is_empty() {
        return None;
    }

    let icon = entry
        .icon()
        .and_then(|name| lookup(name).find())
        .and_then(|p| p.to_str().map(|s| s.to_string()));

    Some(DesktopApp {
        appid: entry.appid.clone(),
        name,
        icon,
        exec,
        scope: classify_scope(&entry.path),
    })
}

fn classify_scope(path: &Path) -> Scope {
    match PathSource::guess_from(path) {
        PathSource::Local | PathSource::LocalFlatpak | PathSource::LocalNix => Scope::User,
        _ => Scope::System,
    }
}

/// Pure variant used by the integration tests: takes pre-collected entries
/// from a fixture directory and emits `DesktopApp`s the same way `load_apps`
/// would. Kept `pub` so integration tests in `tests/` can reach it without
/// walking the real XDG paths.
pub fn entries_to_apps(
    entries: impl IntoIterator<Item = DesktopEntry>,
    locales: &[String],
) -> Vec<DesktopApp> {
    entries
        .into_iter()
        .filter_map(|entry| to_desktop_app(&entry, locales))
        .collect()
}
