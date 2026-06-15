//! Dedupe and disambiguate a list of `DesktopApp`s.
//!
//! Two entries with the same `Name` and `Exec` are treated as one record;
//! User-scoped entries win over System-scoped ones.
//!
//! Two entries with the same `Name` but different `Exec` are both kept,
//! and their display names get a disambiguation suffix of
//! "<basename> <args>" so the user can tell them apart.

use std::collections::HashMap;

use crate::desktop_entry::DesktopApp;
use crate::exec_cleanup::exec_display;

pub fn dedupe_and_disambiguate(apps: Vec<DesktopApp>) -> Vec<DesktopApp> {
    let merged = merge_identical_launches(apps);
    disambiguate_duplicate_names(merged)
}

/// Collapse entries with the same (name, exec) into a single entry,
/// preferring User scope over System scope when both exist.
fn merge_identical_launches(apps: Vec<DesktopApp>) -> Vec<DesktopApp> {
    let mut by_key: HashMap<(String, String), DesktopApp> = HashMap::new();
    let mut order: Vec<(String, String)> = Vec::new();

    for app in apps {
        let key = (app.name.clone(), app.exec.clone());
        match by_key.get(&key) {
            None => {
                order.push(key.clone());
                by_key.insert(key, app);
            }
            Some(existing) => {
                // Keep the entry from the higher-priority scope. On a tie,
                // the first one wins (stable order).
                if app.scope > existing.scope {
                    by_key.insert(key, app);
                }
            }
        }
    }

    order
        .into_iter()
        .filter_map(|k| by_key.remove(&k))
        .collect()
}

/// Any remaining entries that share a display `Name` must have different
/// `Exec` lines (already checked above). Rewrite their `name` to
/// "<original name> (<exec basename + args>)" so the user can distinguish
/// them in the launcher.
fn disambiguate_duplicate_names(apps: Vec<DesktopApp>) -> Vec<DesktopApp> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for app in &apps {
        *counts.entry(app.name.clone()).or_insert(0) += 1;
    }

    apps.into_iter()
        .map(|app| {
            if counts.get(&app.name).copied().unwrap_or(0) > 1 {
                let suffix = exec_display(&app.exec);
                DesktopApp {
                    name: format!("{} ({})", app.name, suffix),
                    ..app
                }
            } else {
                app
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop_entry::Scope;

    fn app(appid: &str, name: &str, exec: &str, scope: Scope) -> DesktopApp {
        DesktopApp {
            appid: appid.to_string(),
            name: name.to_string(),
            icon: None,
            exec: exec.to_string(),
            scope,
            categories: Vec::new(),
        }
    }

    #[test]
    fn identical_name_and_exec_collapses_to_one() {
        let input = vec![
            app(
                "com.google.Chrome",
                "Google Chrome",
                "chrome",
                Scope::System,
            ),
            app("google-chrome", "Google Chrome", "chrome", Scope::System),
        ];
        let out = dedupe_and_disambiguate(input);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "Google Chrome");
    }

    #[test]
    fn user_scope_wins_against_system() {
        let input = vec![
            app("chrome", "Chrome", "chrome", Scope::System),
            app("chrome", "Chrome", "chrome", Scope::User),
        ];
        let out = dedupe_and_disambiguate(input);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].scope, Scope::User);
    }

    #[test]
    fn user_scope_wins_regardless_of_input_order() {
        let input = vec![
            app("chrome", "Chrome", "chrome", Scope::User),
            app("chrome", "Chrome", "chrome", Scope::System),
        ];
        let out = dedupe_and_disambiguate(input);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].scope, Scope::User);
    }

    #[test]
    fn same_name_different_exec_both_kept_and_disambiguated() {
        let input = vec![
            app("firefox", "Firefox", "firefox", Scope::System),
            app("firefox-beta", "Firefox", "firefox --beta", Scope::System),
        ];
        let out = dedupe_and_disambiguate(input);
        assert_eq!(out.len(), 2);
        let names: Vec<&str> = out.iter().map(|a| a.name.as_str()).collect();
        assert!(names.contains(&"Firefox (firefox)"));
        assert!(names.contains(&"Firefox (firefox --beta)"));
    }

    #[test]
    fn unique_names_untouched() {
        let input = vec![
            app("firefox", "Firefox", "firefox", Scope::System),
            app("chrome", "Chrome", "chrome", Scope::System),
        ];
        let out = dedupe_and_disambiguate(input);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].name, "Firefox");
        assert_eq!(out[1].name, "Chrome");
    }

    #[test]
    fn preserves_input_order() {
        let input = vec![
            app("zed", "Zed", "zed", Scope::System),
            app("alacritty", "Alacritty", "alacritty", Scope::System),
            app("firefox", "Firefox", "firefox", Scope::System),
        ];
        let out = dedupe_and_disambiguate(input);
        let names: Vec<&str> = out.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, vec!["Zed", "Alacritty", "Firefox"]);
    }

    #[test]
    fn empty_input() {
        assert!(dedupe_and_disambiguate(vec![]).is_empty());
    }

    #[test]
    fn same_scope_tie_keeps_first_seen() {
        // Two identical entries in the same scope: the first one in input
        // order must win so behavior is stable. This guards against
        // mutating the strict `>` priority check into `>=`, which would
        // flip the winner to the last entry.
        let first = DesktopApp {
            appid: "first-pkg".to_string(),
            ..app("chrome", "Chrome", "chrome", Scope::System)
        };
        let second = DesktopApp {
            appid: "second-pkg".to_string(),
            ..app("chrome", "Chrome", "chrome", Scope::System)
        };
        let out = dedupe_and_disambiguate(vec![first.clone(), second]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].appid, "first-pkg");
    }

    #[test]
    fn three_way_collision_user_wins() {
        // Same (name, exec) across /usr/share, /usr/local/share, and ~/.local.
        // User scope must beat all System entries.
        let input = vec![
            app("chrome", "Chrome", "chrome", Scope::System),
            app("chrome", "Chrome", "chrome", Scope::System),
            app("chrome", "Chrome", "chrome", Scope::User),
        ];
        let out = dedupe_and_disambiguate(input);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].scope, Scope::User);
    }

    #[test]
    fn chrome_bug_scenario() {
        // The exact scenario from the Chrome duplicate report:
        // two system .desktop files with identical Name=Google Chrome
        // and identical exec collapse to one.
        let input = vec![
            app(
                "com.google.Chrome",
                "Google Chrome",
                "google-chrome-stable",
                Scope::System,
            ),
            app(
                "google-chrome",
                "Google Chrome",
                "google-chrome-stable",
                Scope::System,
            ),
        ];
        let out = dedupe_and_disambiguate(input);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "Google Chrome");
    }
}
