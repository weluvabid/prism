#!/usr/bin/env sh
set -eu

ROOT=$(git rev-parse --show-toplevel 2>/dev/null) || {
    printf '%s\n' "Run this script from inside the Prism Git repository." >&2
    exit 1
}

cd "$ROOT"
git config core.hooksPath .githooks
printf '%s\n' "Installed Prism Git hooks from .githooks."
