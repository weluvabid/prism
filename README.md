# Prism

Prism turns terminal text you provide into a polished PNG.

The default theme is Monochrome. Six additional built-in themes and custom TOML themes are available. Prism normalizes the supplied text, then applies its own typography, spacing, palette, and terminal window.

## Install

Install from a checkout with a stable Rust toolchain:

```sh
cargo install --path crates/prism
```

## Use

```sh
# Render command output
cargo test 2>&1 | prism --theme monochrome -o tests.png

# Render a text file
prism --input output.txt --theme nord --output output.png

# Read the clipboard explicitly
prism --clipboard --theme dracula -o clipboard.png

# List themes
prism themes
```

If `--output` is omitted, Prism writes `prism.png`. Input can come from stdin, a file, or the clipboard when `--clipboard` is passed.

## Themes

Seven built-in themes are included:

- `monochrome` (default)
- `github-dark`
- `github-light`
- `catppuccin-mocha`
- `nord`
- `dracula`
- `rose-pine`

Create and inspect the user configuration:

```sh
prism config init
prism config show
```

The configuration file is stored in the platform's application config directory, under `prism/config.toml`.

Custom themes can extend a built-in theme and override individual colors:

```toml
[themes.ocean]
extends = "nord"
name = "Ocean"
background = "#101820"
background_end = "#192a37"
surface = "#101820"
foreground = "#e6edf3"
muted = "#9aa9b8"
accent = "#7aa2f7"
border = "#314253"
```

Use it with `prism --theme ocean`.

## Rendering options

```text
-i, --input <PATH>        Read input from a file
-o, --output <PATH>       Output PNG path [default: prism.png]
    --clipboard           Read clipboard text explicitly
    --theme <THEME>       Select a built-in or configured theme
    --bg <HEX>            Override canvas background
    --fg <HEX>            Override text color
    --accent <HEX>        Override accent color
    --width <PIXELS>      Canvas width: 640–4096
    --font-size <PIXELS>  Font size: 10–32
    --padding <PIXELS>    Content padding: 8–128
    --radius <PIXELS>     Window corner radius: 4–48
    --chrome <STYLE>      macos, minimal, or none
    --tab-width <COLUMNS> Tab expansion width: 1–16
    --wrap                Wrap long lines
    --no-wrap             Clip long lines
    --max-input-bytes <N> Input limit: 1 byte–64 MiB
```

CLI options override values in `config.toml`, which override the built-in defaults.

## Text handling

Prism removes terminal styling and supported control sequences, expands tabs, handles carriage-return progress output and backspaces, and preserves text and indentation. Wrapping uses Unicode display-cell width and keeps grapheme clusters together. Invalid UTF-8 input is replaced with the Unicode replacement character.

## Development

Requires stable Rust with `rustfmt` and `clippy`.

Run the project checks:

```sh
./scripts/check.sh
```

Run the rendering example:

```sh
./scripts/example.sh
```

Install the repository's pre-commit hook once:

```sh
./scripts/install-hooks.sh
```

The hook checks staged whitespace, formatting, Clippy, and the workspace tests before each commit. CI runs the same Rust checks on Linux, macOS, and Windows.

## License

MIT. See [LICENSE](LICENSE).
