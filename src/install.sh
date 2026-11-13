#!/bin/sh
# Installs the mwp CLI from this checkout and, if a folder is given, sets up MWP in it.
# --gui also installs mwp-gui, the desktop editor.
# Usage: ./src/install.sh [--gui] [project-dir]
set -eu

here=$(cd "$(dirname "$0")" && pwd)

gui=false
project=""
for arg in "$@"; do
  case "$arg" in
    --gui) gui=true ;;
    -*) echo "unknown option: $arg" >&2; exit 2 ;;
    *) project="$arg" ;;
  esac
done

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo not found. Install Rust first: https://rustup.rs" >&2
  exit 1
fi

# --force reinstalls, so template and skill changes reach the binary.
cargo install --path "$here/client" --locked --force
if [ "$gui" = true ]; then
  cargo install --path "$here/gui" --locked --force
fi

bin="${CARGO_INSTALL_ROOT:-${CARGO_HOME:-$HOME/.cargo}}/bin"
if [ -n "$project" ]; then
  "$bin/mwp" init "$project"
fi

case ":$PATH:" in
  *":$bin:"*) ;;
  *) if [ "$gui" = true ]; then what="mwp and mwp-gui"; else what="mwp"; fi
     echo "Add $bin to your PATH to run $what from anywhere." ;;
esac
