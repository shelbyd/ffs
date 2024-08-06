#! /bin/bash

set -euo pipefail

if ! command -v cargo &> /dev/null; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
fi

cargo install --git https://github.com/shelbyd/ffs
