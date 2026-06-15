//! Parse and apply `--category`/`--categories` filters.
//!
//! Filter values are comma-separated. Each value is either an include
//! (`game`) or an exclude (`-game`, prefixed with `-`). Include and
//! exclude values cannot be mixed in a single invocation. Matching against
//! a `.desktop` entry's `Categories=` field is case-insensitive.

use crate::desktop_entry::DesktopApp;

/// A parsed category filter. Either keeps only apps in the listed
/// categories (`Include`) or drops apps in the listed categories
/// (`Exclude`). Category names are stored lowercased for case-insensitive
/// comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CategoryFilter {
    Include(Vec<String>),
    Exclude(Vec<String>),
}

impl CategoryFilter {
    /// Parse a raw filter string (the value of `--category`).
    ///
    /// Returns `Ok(None)` when the value carries no categories (e.g. an
    /// empty string), `Err` when include and exclude values are mixed or
    /// a bare `-` appears with no category name.
    pub fn parse(raw: &str) -> Result<Option<CategoryFilter>, String> {
        let mut includes = Vec::new();
        let mut excludes = Vec::new();

        for token in raw.split(',') {
            let token = token.trim();
            if token.is_empty() {
                continue;
            }
            if let Some(name) = token.strip_prefix('-') {
                let name = name.trim();
                if name.is_empty() {
                    return Err("category exclusion is missing a name (got bare '-')".to_string());
                }
                excludes.push(name.to_lowercase());
            } else {
                includes.push(token.to_lowercase());
            }
        }

        match (includes.is_empty(), excludes.is_empty()) {
            (true, true) => Ok(None),
            (false, true) => Ok(Some(CategoryFilter::Include(includes))),
            (true, false) => Ok(Some(CategoryFilter::Exclude(excludes))),
            (false, false) => Err(
                "cannot mix included and excluded categories in the same filter".to_string(),
            ),
        }
    }

    /// True if `app` passes this filter.
    pub fn matches(&self, app: &DesktopApp) -> bool {
        let has = |wanted: &[String]| {
            app.categories
                .iter()
                .any(|c| wanted.iter().any(|w| w == &c.to_lowercase()))
        };
        match self {
            CategoryFilter::Include(cats) => has(cats),
            CategoryFilter::Exclude(cats) => !has(cats),
        }
    }
}

/// Apply an optional filter to a list of apps. `None` keeps everything.
pub fn apply(apps: Vec<DesktopApp>, filter: Option<&CategoryFilter>) -> Vec<DesktopApp> {
    match filter {
        None => apps,
        Some(f) => apps.into_iter().filter(|a| f.matches(a)).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop_entry::Scope;

    fn app(name: &str, categories: &[&str]) -> DesktopApp {
        DesktopApp {
            appid: format!("{name}.desktop"),
            name: name.to_string(),
            icon: None,
            exec: name.to_lowercase(),
            scope: Scope::System,
            categories: categories.iter().map(|c| c.to_string()).collect(),
        }
    }

    #[test]
    fn parse_empty_is_none() {
        assert_eq!(CategoryFilter::parse("").unwrap(), None);
        assert_eq!(CategoryFilter::parse("  ").unwrap(), None);
        assert_eq!(CategoryFilter::parse(",,").unwrap(), None);
    }

    #[test]
    fn parse_single_include() {
        assert_eq!(
            CategoryFilter::parse("game").unwrap(),
            Some(CategoryFilter::Include(vec!["game".to_string()]))
        );
    }

    #[test]
    fn parse_multiple_includes() {
        assert_eq!(
            CategoryFilter::parse("game,network").unwrap(),
            Some(CategoryFilter::Include(vec![
                "game".to_string(),
                "network".to_string()
            ]))
        );
    }

    #[test]
    fn parse_single_exclude() {
        assert_eq!(
            CategoryFilter::parse("-game").unwrap(),
            Some(CategoryFilter::Exclude(vec!["game".to_string()]))
        );
    }

    #[test]
    fn parse_multiple_excludes() {
        assert_eq!(
            CategoryFilter::parse("-game,-network").unwrap(),
            Some(CategoryFilter::Exclude(vec![
                "game".to_string(),
                "network".to_string()
            ]))
        );
    }

    #[test]
    fn parse_lowercases() {
        assert_eq!(
            CategoryFilter::parse("Game").unwrap(),
            Some(CategoryFilter::Include(vec!["game".to_string()]))
        );
    }

    #[test]
    fn parse_trims_whitespace() {
        assert_eq!(
            CategoryFilter::parse(" game , network ").unwrap(),
            Some(CategoryFilter::Include(vec![
                "game".to_string(),
                "network".to_string()
            ]))
        );
    }

    #[test]
    fn parse_mixed_is_error() {
        assert!(CategoryFilter::parse("game,-network").is_err());
    }

    #[test]
    fn parse_bare_dash_is_error() {
        assert!(CategoryFilter::parse("-").is_err());
        assert!(CategoryFilter::parse("- ").is_err());
    }

    #[test]
    fn include_keeps_only_matching() {
        let filter = CategoryFilter::Include(vec!["game".to_string()]);
        assert!(filter.matches(&app("Quake", &["Game"])));
        assert!(!filter.matches(&app("Firefox", &["Network"])));
    }

    #[test]
    fn include_matches_case_insensitively() {
        let filter = CategoryFilter::Include(vec!["game".to_string()]);
        assert!(filter.matches(&app("Quake", &["Game"])));
        assert!(filter.matches(&app("Doom", &["GAME"])));
    }

    #[test]
    fn include_matches_any_listed_category() {
        let filter =
            CategoryFilter::Include(vec!["game".to_string(), "network".to_string()]);
        assert!(filter.matches(&app("Quake", &["Game"])));
        assert!(filter.matches(&app("Firefox", &["Network", "WebBrowser"])));
        assert!(!filter.matches(&app("Gimp", &["Graphics"])));
    }

    #[test]
    fn exclude_drops_matching() {
        let filter = CategoryFilter::Exclude(vec!["game".to_string()]);
        assert!(!filter.matches(&app("Quake", &["Game"])));
        assert!(filter.matches(&app("Firefox", &["Network"])));
    }

    #[test]
    fn exclude_drops_if_any_category_matches() {
        let filter = CategoryFilter::Exclude(vec!["game".to_string()]);
        assert!(!filter.matches(&app("Hybrid", &["Utility", "Game"])));
    }

    #[test]
    fn app_with_no_categories_excluded_by_include() {
        let filter = CategoryFilter::Include(vec!["game".to_string()]);
        assert!(!filter.matches(&app("Bare", &[])));
    }

    #[test]
    fn app_with_no_categories_kept_by_exclude() {
        let filter = CategoryFilter::Exclude(vec!["game".to_string()]);
        assert!(filter.matches(&app("Bare", &[])));
    }

    #[test]
    fn apply_none_keeps_all() {
        let apps = vec![app("A", &["Game"]), app("B", &["Network"])];
        let out = apply(apps.clone(), None);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn apply_filters() {
        let apps = vec![app("A", &["Game"]), app("B", &["Network"])];
        let filter = CategoryFilter::Include(vec!["game".to_string()]);
        let out = apply(apps, Some(&filter));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "A");
    }
}
