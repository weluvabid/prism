# Architecture

Prism is organized as a single Rust package inside a workspace. The module boundaries are intentionally simple; they are implementation boundaries, not separate published crates.

## Pipeline

1. `input`: reads stdin, a file, or explicitly requested clipboard text with a hard byte limit.
2. `normalize`: consumes ANSI/VT control sequences, resolves carriage-return progress lines, expands tabs, preserves text and indentation, and replaces invalid UTF-8 at the input boundary.
3. `theme`: resolves a built-in palette and applies validated TOML and command-line overrides. Colors are strictly six-digit hex internally.
4. `render`: wraps text by Unicode terminal-cell width, constructs a small SVG scene, rasterizes it, validates image size, and writes a PNG.

## Invariants

- No terminal buffer access, shell integration, PTY, monitoring, or network access.
- The canonical text has no source color/style annotations. Theme resolution is independent of parsing.
- User-provided text and custom theme values are data, never SVG markup. Text is XML-escaped and color values are validated before interpolation.
- Input bytes, number of lines, dimensions, and total pixels are bounded.
- A successful render writes to a temporary file in the output directory and then renames it into place.
- CLI options take precedence over config; config takes precedence over defaults.

## Deliberate limits

Prism is not a terminal emulator and does not interpret cursor-addressed screen updates into a full virtual screen. It discards those control operations and renders the supplied text stream. Custom themes can extend built-in palettes but cannot form chains of custom theme inheritance, preventing cyclic configuration graphs and keeping resolution non-recursive.
