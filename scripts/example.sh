#!/usr/bin/env sh
set -eu

printf 'Prism\n\n$ cargo test\n   Compiling prism v0.1.0\n    Finished test profile\n     Running tests\n\nrunning 42 tests\n\ntest result: ok. 42 passed; 0 failed\n' \
  | cargo run --quiet -- --theme catppuccin-mocha --output example.png

printf 'Wrote example.png\n'
