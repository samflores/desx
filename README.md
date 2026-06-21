# desx

[![tests](https://github.com/samflores/desx/actions/workflows/tests.yml/badge.svg)](https://github.com/samflores/desx/actions/workflows/tests.yml)

Dump installed desktop applications as `name,icon,exec` lines on stdout.
Designed to pipe into a menu like [drmenu](https://github.com/samflores/drmenu).

## What it does

- Walks the XDG application directories (`$XDG_DATA_HOME/applications`,
  `$XDG_DATA_DIRS/applications`).
- Filters out `NoDisplay=true`, `Hidden=true`, and non-`Application` entries.
- Strips `%u %U %f %F %i %c %k` field codes from each `Exec=` line,
  collapsing leftover whitespace and preserving the literal `%%` escape.
- Resolves each `Icon=` to an absolute path via the XDG icon theme. When
  no icon resolves, the icon field is left empty.
- Dedupes apps: entries with the same `Name` AND the same stripped `Exec`
  collapse to one (user-scoped entries win over system-scoped ones).
- Disambiguates same-named apps with different `Exec` values by suffixing
  the display name with `(<basename> <args>)` — e.g. `Firefox (firefox --beta)`.

## Dependencies

Rust stable (edition 2024). Runtime crates:

- [`freedesktop-desktop-entry`](https://crates.io/crates/freedesktop-desktop-entry) — parse `.desktop` files and walk the XDG search paths.
- [`freedesktop-icons`](https://crates.io/crates/freedesktop-icons) — resolve icon names to absolute paths.

Tests additionally use [`tempfile`](https://crates.io/crates/tempfile).

## Build

```sh
cargo build --release
```

Binary: `target/release/desx`.

## Usage

```sh
desx
```

### Filter by category

Use `--category` (alias `--categories`) to keep or drop apps by their
`.desktop` `Categories=` values. The value is comma-separated; prefix a
name with `-` to exclude it. Matching is case-insensitive. Include and
exclude values cannot be mixed in one invocation.

```sh
desx --category game            # only apps in the Game category
desx --category game,network    # apps in Game OR Network
desx --category -game           # everything except games
desx --categories -game,-audiovideo
```

Apps with no `Categories=` line are dropped by an include filter and kept
by an exclude filter.

Example output:

```
Firefox,/usr/share/icons/hicolor/48x48/apps/firefox.png,firefox
Google Chrome,/opt/google/chrome/product_logo_48.png,google-chrome-stable
Terminal (foot),/usr/share/icons/hicolor/scalable/apps/foot.svg,foot
Terminal (kitty),/usr/share/icons/hicolor/256x256/apps/kitty.png,kitty
```

### Pipe into drmenu

```sh
desx | drmenu | sh
```

Bind to a key in Sway (`~/.config/sway/config`):

```
bindsym $mod+d exec 'desx | drmenu | sh'
```

## Tests

```sh
cargo test
```

See `.cargo/config.toml` for `cargo cov`, `cargo cov-html`,
`cargo cov-missing`, and `cargo mut` aliases.
