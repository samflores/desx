//! Minimal command-line argument parsing for desx.
//!
//! The only option is `--category`/`--categories`, which takes a
//! comma-separated filter value (see [`crate::category_filter`]). The value
//! may be attached with `=` (`--category=game`) or given as the next
//! argument (`--category game`). Repeated flags are rejected to keep the
//! include/exclude semantics unambiguous.

use crate::category_filter::CategoryFilter;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Args {
    pub category_filter: Option<CategoryFilter>,
}

/// Parse an iterator of argument strings (excluding the program name).
pub fn parse<I, S>(args: I) -> Result<Args, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut iter = args.into_iter().peekable();
    let mut raw_filter: Option<String> = None;

    while let Some(arg) = iter.next() {
        let arg = arg.as_ref();
        let value = match arg {
            "--category" | "--categories" => match iter.next() {
                Some(v) => v.as_ref().to_string(),
                None => return Err(format!("{arg} requires a value")),
            },
            _ if arg.starts_with("--category=") => {
                arg.trim_start_matches("--category=").to_string()
            }
            _ if arg.starts_with("--categories=") => {
                arg.trim_start_matches("--categories=").to_string()
            }
            _ => return Err(format!("unknown argument: {arg}")),
        };

        if raw_filter.is_some() {
            return Err("--category may only be given once".to_string());
        }
        raw_filter = Some(value);
    }

    let category_filter = match raw_filter {
        Some(raw) => CategoryFilter::parse(&raw)?,
        None => None,
    };

    Ok(Args { category_filter })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(args: &[&str]) -> Args {
        parse(args.iter().copied()).unwrap()
    }

    #[test]
    fn no_args_is_empty() {
        assert_eq!(parse_ok(&[]), Args::default());
    }

    #[test]
    fn category_separate_value() {
        assert_eq!(
            parse_ok(&["--category", "game"]).category_filter,
            Some(CategoryFilter::Include(vec!["game".to_string()]))
        );
    }

    #[test]
    fn category_equals_value() {
        assert_eq!(
            parse_ok(&["--category=game"]).category_filter,
            Some(CategoryFilter::Include(vec!["game".to_string()]))
        );
    }

    #[test]
    fn categories_alias() {
        assert_eq!(
            parse_ok(&["--categories", "game"]).category_filter,
            Some(CategoryFilter::Include(vec!["game".to_string()]))
        );
        assert_eq!(
            parse_ok(&["--categories=game"]).category_filter,
            Some(CategoryFilter::Include(vec!["game".to_string()]))
        );
    }

    #[test]
    fn exclude_value() {
        assert_eq!(
            parse_ok(&["--category", "-game"]).category_filter,
            Some(CategoryFilter::Exclude(vec!["game".to_string()]))
        );
    }

    #[test]
    fn exclude_value_with_equals() {
        // `--category=-game` keeps the `-game` as the value, not a new flag.
        assert_eq!(
            parse_ok(&["--category=-game"]).category_filter,
            Some(CategoryFilter::Exclude(vec!["game".to_string()]))
        );
    }

    #[test]
    fn missing_value_is_error() {
        assert!(parse(["--category"].iter().copied()).is_err());
    }

    #[test]
    fn unknown_arg_is_error() {
        assert!(parse(["--bogus"].iter().copied()).is_err());
    }

    #[test]
    fn repeated_flag_is_error() {
        assert!(parse(["--category", "game", "--category", "network"].iter().copied()).is_err());
    }

    #[test]
    fn mixed_include_exclude_is_error() {
        assert!(parse(["--category", "game,-network"].iter().copied()).is_err());
    }
}
