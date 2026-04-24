//! Plain data shape for a `.desktop` application entry after all the
//! XDG spec quirks have been handled. The loader (`xdg.rs`) produces
//! these; `dedupe.rs` consumes them.

/// Scope a `.desktop` file came from. User-scoped entries win ties
/// against system-scoped ones during dedupe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Scope {
    /// `/usr/share/applications`, `/usr/local/share/applications`, etc.
    System,
    /// `$XDG_DATA_HOME/applications`, defaulting to `~/.local/share/applications`.
    User,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopApp {
    /// Filename stem (e.g. `google-chrome` from `google-chrome.desktop`).
    pub appid: String,
    /// Display name shown to the user.
    pub name: String,
    /// Icon identifier (icon theme name or absolute path) if present.
    pub icon: Option<String>,
    /// `Exec=` line with field codes stripped and whitespace collapsed.
    pub exec: String,
    /// Origin directory scope.
    pub scope: Scope,
}

impl DesktopApp {
    /// Apps are equivalent for dedupe if both `name` and `exec` match.
    pub fn same_launch(&self, other: &DesktopApp) -> bool {
        self.name == other.name && self.exec == other.exec
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(name: &str, exec: &str, scope: Scope) -> DesktopApp {
        DesktopApp {
            appid: format!("{}.desktop", name),
            name: name.to_string(),
            icon: None,
            exec: exec.to_string(),
            scope,
        }
    }

    #[test]
    fn same_launch_matches_on_name_and_exec() {
        let a = app("Chrome", "chrome", Scope::System);
        let b = app("Chrome", "chrome", Scope::User);
        assert!(a.same_launch(&b));
    }

    #[test]
    fn same_launch_differs_when_exec_differs() {
        let a = app("Chrome", "chrome", Scope::System);
        let b = app("Chrome", "chrome --incognito", Scope::System);
        assert!(!a.same_launch(&b));
    }

    #[test]
    fn same_launch_differs_when_name_differs() {
        let a = app("Chrome", "chrome", Scope::System);
        let b = app("Chromium", "chrome", Scope::System);
        assert!(!a.same_launch(&b));
    }

    #[test]
    fn scope_user_greater_than_system() {
        assert!(Scope::User > Scope::System);
    }
}
