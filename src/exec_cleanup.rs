//! Strip field codes from `.desktop` `Exec=` values.
//!
//! Per the freedesktop spec, field codes are `%f %F %u %U %i %c %k` (plus
//! a literal `%%` escape for a single `%`). Launchers that don't expand
//! them must remove them before exec.

use std::sync::LazyLock;

use regex::Regex;

static FIELD_CODE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"%[uUfFick]").expect("valid field-code regex"));

pub fn strip_field_codes(exec: &str) -> String {
    // Handle the `%%` escape by swapping it to a placeholder, stripping
    // the single-char codes, then restoring. This way `echo 100%%` stays
    // `echo 100%` instead of being mangled.
    let escaped = exec.replace("%%", "\u{FEFF}");
    let stripped = FIELD_CODE.replace_all(&escaped, "");
    collapse_internal_whitespace(&stripped.replace('\u{FEFF}', "%"))
        .trim()
        .to_string()
}

fn collapse_internal_whitespace(s: &str) -> String {
    // Multiple spaces left behind by stripped codes should collapse to one.
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Return `(basename, rest)` of an `Exec=` line so callers can render
/// "firefox --new-window" as a disambiguator without the full path.
pub fn split_basename_and_args(exec: &str) -> (String, String) {
    let trimmed = exec.trim();
    let (head, tail) = match trimmed.split_once(char::is_whitespace) {
        Some((h, t)) => (h, t.trim()),
        None => (trimmed, ""),
    };
    let basename = std::path::Path::new(head)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(head)
        .to_string();
    (basename, tail.to_string())
}

/// Render `basename + args` as a disambiguation suffix, e.g. `firefox --new-window`.
pub fn exec_display(exec: &str) -> String {
    let (base, args) = split_basename_and_args(exec);
    if args.is_empty() {
        base
    } else {
        format!("{} {}", base, args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_single_char_field_codes() {
        assert_eq!(strip_field_codes("firefox %u"), "firefox");
        assert_eq!(strip_field_codes("chromium %U"), "chromium");
        assert_eq!(strip_field_codes("editor %f"), "editor");
        assert_eq!(strip_field_codes("thunar %F"), "thunar");
        assert_eq!(strip_field_codes("app %i"), "app");
        assert_eq!(strip_field_codes("app %c"), "app");
        assert_eq!(strip_field_codes("app %k"), "app");
    }

    #[test]
    fn preserves_literal_percent_escape() {
        assert_eq!(strip_field_codes("echo 100%%"), "echo 100%");
        assert_eq!(strip_field_codes("progress %% %u"), "progress %");
    }

    #[test]
    fn collapses_whitespace_from_stripped_codes() {
        assert_eq!(
            strip_field_codes("env FOO=1 app  %u  --flag"),
            "env FOO=1 app --flag"
        );
    }

    #[test]
    fn empty_and_unaffected_inputs() {
        assert_eq!(strip_field_codes(""), "");
        assert_eq!(strip_field_codes("plain command"), "plain command");
    }

    #[test]
    fn split_basename_handles_plain_command() {
        assert_eq!(
            split_basename_and_args("firefox"),
            ("firefox".to_string(), String::new())
        );
    }

    #[test]
    fn split_basename_strips_path() {
        assert_eq!(
            split_basename_and_args("/usr/bin/firefox --new-window"),
            ("firefox".to_string(), "--new-window".to_string())
        );
    }

    #[test]
    fn split_basename_preserves_multi_arg() {
        assert_eq!(
            split_basename_and_args("/opt/google/chrome/chrome --profile-directory=Default"),
            (
                "chrome".to_string(),
                "--profile-directory=Default".to_string()
            )
        );
    }

    #[test]
    fn exec_display_no_args() {
        assert_eq!(exec_display("/usr/bin/firefox"), "firefox");
    }

    #[test]
    fn exec_display_with_args() {
        assert_eq!(
            exec_display("/usr/bin/firefox --new-window"),
            "firefox --new-window"
        );
    }

    #[test]
    fn exec_display_trims_input() {
        assert_eq!(exec_display("   firefox   "), "firefox");
    }
}
