use freedesktop_desktop_entry::get_languages_from_env;

use desx::dedupe::dedupe_and_disambiguate;
use desx::xdg::load_apps;

fn main() {
    let locales = get_languages_from_env();
    let apps = dedupe_and_disambiguate(load_apps(&locales));

    for app in apps {
        let icon = app.icon.unwrap_or_default();
        println!("{},{},{}", app.name, icon, app.exec);
    }
}
