# Architecture

Prism is a Rust workspace containing one CLI package.

## Rendering pipeline

1. `input` reads text from stdin, a file, or the explicitly requested clipboard.
2. `normalize` removes terminal control sequences, handles progress output and backspaces, expands tabs, and normalizes UTF-8 text.
3. `theme` resolves a built-in or configured palette and applies color overrides.
4. `render` wraps text by Unicode display-cell width, builds an SVG scene, rasterizes it, and writes the PNG.

## Core modules

- `config`: application defaults, TOML configuration, and render-setting validation.
- `input`: bounded input from supported sources.
- `normalize`: terminal text normalization and Unicode-aware wrapping.
- `theme`: built-in palettes, custom theme inheritance, and color validation.
- `render`: SVG composition, PNG rasterization, and output writing.

## Rendering details

The renderer uses `resvg` and `tiny-skia`. It loads installed system fonts and uses a monospace font stack. Output dimensions, input size, normalized text, and rendered line count are bounded.

A successful render is written to a temporary file in the destination directory and then moved into place. CLI options take precedence over configuration values, which take precedence over built-in defaults.
