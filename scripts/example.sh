#!/usr/bin/env sh
set -eu

printf 'Prism\n\n$ cargo test\n   Compiling prism v0.1.0\n    Finished test profile\n     Running tests\n\nrunning 30 tests\n\ntest result: ok. 30 passed; 0 failed\n' \
  | cargo run --quiet -- --theme monochrome --output example.png

printf 'Wrote example.png\n'
