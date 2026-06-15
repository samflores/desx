use std::process::ExitCode;

use freedesktop_desktop_entry::get_languages_from_env;

use desx::args;
use desx::category_filter::apply;
use desx::dedupe::dedupe_and_disambiguate;
use desx::xdg::load_apps;

fn main() -> ExitCode {
    let parsed = match args::parse(std::env::args().skip(1)) {
        Ok(parsed) => parsed,
        Err(err) => {
            eprintln!("desx: {err}");
            eprintln!("usage: desx [--category[=]<list>]");
            eprintln!("  <list> is comma-separated; prefix a name with '-' to exclude.");
            eprintln!("  e.g. --category game,network   --category -game");
            return ExitCode::FAILURE;
        }
    };

    let locales = get_languages_from_env();
    let apps = dedupe_and_disambiguate(load_apps(&locales));
    let apps = apply(apps, parsed.category_filter.as_ref());

    for app in apps {
        let icon = app.icon.unwrap_or_default();
        println!("{},{},{}", app.name, icon, app.exec);
    }

    ExitCode::SUCCESS
}
