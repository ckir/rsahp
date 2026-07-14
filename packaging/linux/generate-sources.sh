#!/usr/bin/env bash
# Generates cargo-sources.json from Cargo.lock so flatpak-builder can build offline.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
repo_root="$(cd "$here/../.." && pwd)"

# Pin flatpak-cargo-generator to a specific commit (NOT master) for reproducibility.
GEN_REF="${FLATPAK_CARGO_GEN_REF:-f03a673abe6ce189cea1c2857e2b44af2dd79d1f}"
gen="$here/flatpak-cargo-generator.py"
if [[ ! -f "$gen" ]]; then
  curl -fsSL -o "$gen" \
    "https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/${GEN_REF}/cargo/flatpak-cargo-generator.py"
fi

python3 "$gen" "$repo_root/Cargo.lock" -o "$here/cargo-sources.json"
echo "wrote $here/cargo-sources.json (generator ref: $GEN_REF)"
