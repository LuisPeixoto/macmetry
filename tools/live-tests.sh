#!/usr/bin/env bash
# Runs the checks that need a real Mac: every collector against the hardware it is running on, plus the command line
# and the MCP server end to end. Safe to run: it reads, and writes only to a throwaway data folder.
#
#   tools/live-tests.sh
set -euo pipefail
cd "$(dirname "$0")/.."

home="$(mktemp -d)"
trap 'rm -rf "$home"' EXIT
export MACMETRY_HOME="$home"

echo "== Mac: $(sysctl -n hw.model), $(uname -m), macOS $(sw_vers -productVersion)"

echo "== Unit and integration tests"
cargo test --workspace --exclude macmetry-desktop

echo "== Live collector checks (run on this hardware)"
cargo test -p macmetry-sys -- --ignored --test-threads=1

echo "== Command line"
cargo build -q -p macmetry-cli
bin=target/debug/macmetry
"$bin" info
"$bin" snapshot --wait-ms 1000 --processes 3
"$bin" sensors | head -5 || true
"$bin" battery

echo "== Probes (what each collector sees, as JSON)"
cargo run -q -p macmetry-sys --example probe | head -40

echo
echo "Done. To contribute a new Mac's fixtures, run tools/capture-fixtures.sh"
