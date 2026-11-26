# Prism

Prism turns terminal text you explicitly provide into a polished, shareable PNG.

The content is the input; the theme is the presentation. Prism removes source styling and terminal control sequences, normalizes text, and applies its own palette, typography, spacing, and window frame.

**Prism never monitors your terminal or captures it in the background.** It only processes text supplied through stdin, a file, or the clipboard when you explicitly request it.

## Quick start

Install the CLI from a checked-out source tree with a stable Rust toolchain:

```sh
cargo install --path crates/prism
```

Then use it with supplied output:

```sh
# Pipe command output
cargo test 2>&1 | prism --theme catppuccin-mocha -o tests.png

# Read a text file
prism --input output.txt --theme nord --output output.png

# Explicitly read text from your clipboard
prism --clipboard --theme dracula -o clipboard.png

# Customize a palette for one render
prism --theme github-dark --bg '#101820' --fg '#e6edf3' --accent '#7aa2f7'
```

When no output path is supplied, Prism writes `prism.png`. When input is not piped and `--input` is absent, it returns an actionable error instead of silently reading the clipboard.

## Themes

Six built-in palettes are included:

- `github-dark`
- `github-light`
- `catppuccin-mocha`
- `nord`
- `dracula`
- `rose-pine`

```sh
prism themes
```

Theme colors are presentation-only. Original ANSI colors are intentionally discarded; the same normalized input renders consistently under every theme.

### Custom themes

Create the starter config in your platform's application config directory:

```sh
prism config init
prism config show
```

The config location is the standard config directory returned by the operating system (for example, `~/.config/prism/config.toml` on many Linux systems).

Add a theme to `config.toml`:

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

Then render with `prism --theme ocean`. A custom theme can extend any built-in palette and override only the values it needs. Custom themes extend built-ins directly; nested custom-theme inheritance is intentionally not supported.

CLI values override config defaults, which override the built-in defaults.

## Options

```text
prism [OPTIONS]

Options:
  -i, --input <PATH>        Read input from a file
  -o, --output <PATH>       Output PNG path [default: prism.png]
      --clipboard           Read text from the system clipboard explicitly
      --theme <THEME>       Theme name
      --bg <HEX>            Override canvas background (sets both gradient stops)
      --fg <HEX>            Override text color
      --accent <HEX>        Override accent color
      --width <PIXELS>      Canvas width, 640–4096 [default: 1200]
      --font-size <PIXELS>  Monospace font size, 10–32 [default: 16]
      --padding <PIXELS>    Content padding, 8–128 [default: 30]
      --radius <PIXELS>     Window corner radius, 4–48 [default: 18]
      --chrome <STYLE>      macos, minimal, or none
      --tab-width <COLUMNS> Tab expansion width, 1–16 [default: 4]
      --wrap                Wrap long lines (overrides config)
      --no-wrap             Clip long lines instead of wrapping them
      --max-input-bytes <N> Input limit, 1 byte–64 MiB [default: 8 MiB]
  -h, --help                Print help
  -V, --version             Print version
```

Additional commands:

```sh
prism themes
prism config show
prism config init [--force]
```

## Normalization behavior

- ANSI SGR colors and styles are removed; Prism always applies its own colors.
- OSC titles and hyperlinks, CSI cursor/erase sequences, and other supported escape-string controls are consumed, never executed.
- Newlines and indentation are preserved. A single final newline is removed, so ordinary command output does not gain an extra blank row.
- Tabs expand to fixed columns. Carriage-return progress updates replace the current line where possible, and backspace removes the previous Unicode scalar.
- Input is decoded as UTF-8; invalid byte sequences become the standard replacement character.
- Unicode display width is considered when wrapping, including wide CJK characters.
- Rendering is bounded by the input limit, a 16 MiB normalized-text limit, a 10,000-line limit, canvas dimensions, and total pixel budget.

Prism is not a full VT emulator. If supplied text came from a full-screen TUI or contains cursor-addressed drawing, Prism strips the control operations and renders the remaining visible text stream; it cannot reconstruct terminal state that was not provided. For best results, pipe command output or copy the relevant output as plain text.

## Rendering

Prism builds an SVG scene and rasterizes it to PNG with `resvg` and `tiny-skia`. It looks for installed system fonts, preferring JetBrains Mono and common platform monospace fonts. Exact glyph shapes can therefore differ across machines if the preferred fonts are unavailable; dimensions, palette, and layout rules remain deterministic for the selected settings.

## Development

Requires a stable Rust toolchain with `rustfmt` and `clippy`.

```sh
./scripts/check.sh
./scripts/example.sh
```

CI runs formatting, Clippy with warnings denied, and tests on Linux, macOS, and Windows. The example script writes `example.png` in the workspace root.

## Privacy

Prism has no telemetry, network client, background service, or terminal-monitoring integration. Clipboard access occurs only when `--clipboard` is passed. On Linux, Wayland clipboard mode depends on compositor support for the standard data-control protocol; stdin and file input remain available when the clipboard backend is unavailable.

## License

MIT. See [LICENSE](LICENSE).
