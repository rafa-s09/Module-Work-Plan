#!/bin/sh
# Installs the mwp CLI from this checkout and, if a folder is given, sets up MWP in it.
# Usage: ./src/install.sh [project-dir]
set -eu

here=$(cd "$(dirname "$0")" && pwd)

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo not found. Install Rust first: https://rustup.rs" >&2
  exit 1
fi

# --force reinstalls, so template and skill changes reach the binary.
cargo install --path "$here/client" --locked --force

bin="${CARGO_INSTALL_ROOT:-${CARGO_HOME:-$HOME/.cargo}}/bin"
if [ $# -gt 0 ]; then
  "$bin/mwp" init "$1"
fi

case ":$PATH:" in
  *":$bin:"*) ;;
  *) echo "Add $bin to your PATH to run mwp from anywhere." ;;
esac
