#!/usr/bin/env bash
# Captures what this Mac exposes, as fixtures for the tests: the SMC keys, every IOReport channel and HID
# temperature sensor, the GPU accelerator, the battery and the block storage statistics. Nothing is sent anywhere;
# the files are written to a folder you choose, and you decide what to commit.
#
#   tools/capture-fixtures.sh [folder]      (default: crates/macmetry-sys/tests/fixtures/<model>)
#
# Look through the files before you share them. Serial numbers, UUIDs and network addresses are removed, but a
# machine name or a user name can still appear in a registry dump.
set -euo pipefail
cd "$(dirname "$0")/.."

model="$(sysctl -n hw.model | tr ',' '-' | tr '[:upper:]' '[:lower:]')"
arch="$(uname -m)"
out="${1:-crates/macmetry-sys/tests/fixtures/${arch}-${model}}"
mkdir -p "$out"
echo "Capturing to $out"

cargo build -q -p macmetry-sys --examples

echo "- SMC keys"
cargo run -q -p macmetry-sys --example smc_dump > "$out/smc-keys.jsonl" 2>/dev/null || echo "  (this Mac has no readable SMC)"

echo "- IOReport channels and HID temperature sensors"
cargo run -q -p macmetry-sys --example ioreport_dump > "$out/ioreport.jsonl" 2>/dev/null || echo "  (IOReport is not available)"

echo "- What the collectors see"
cargo run -q -p macmetry-sys --example probe > "$out/probe.json" 2>/dev/null

echo "- GPU accelerator"
ioreg -arc IOAccelerator -d1 > "$out/ioaccelerator.plist" 2>/dev/null || true

echo "- Battery"
ioreg -arc AppleSmartBattery > "$out/applesmartbattery.plist" 2>/dev/null || true
# Registry dumps carry identifiers that say nothing about performance. Remove them.
for f in "$out"/*.plist; do
  [ -s "$f" ] || { rm -f "$f"; continue; }
  /usr/bin/python3 - "$f" <<'PY'
import plistlib, sys
DROP = {"Serial", "BatterySerialNumber", "DeviceName", "UUID", "SerialNumber", "IOPlatformSerialNumber", "IOPlatformUUID", "Manufacturer Name"}
def clean(node):
    if isinstance(node, dict):
        return {k: clean(v) for k, v in node.items() if k not in DROP}
    if isinstance(node, list):
        return [clean(v) for v in node]
    return node
path = sys.argv[1]
with open(path, "rb") as f:
    data = plistlib.load(f)
with open(path, "wb") as f:
    plistlib.dump(clean(data), f)
PY
done

echo "- Block storage statistics"
ioreg -arlc IOBlockStorageDriver | /usr/bin/python3 -c '
import plistlib, sys
tree = plistlib.loads(sys.stdin.buffer.read())
out = []
def walk(n):
    if not isinstance(n, dict): return
    if n.get("IOObjectClass") == "IOBlockStorageDriver" and "Statistics" in n:
        kids = [{"BSD Name": c["BSD Name"], "Whole": bool(c.get("Whole"))} for c in n.get("IORegistryEntryChildren", []) if isinstance(c, dict) and "BSD Name" in c]
        out.append({"IOClass": "IOBlockStorageDriver", "Statistics": n["Statistics"], "IORegistryEntryChildren": kids})
    for c in n.get("IORegistryEntryChildren", []): walk(c)
for n in tree: walk(n)
plistlib.dump(out, open(sys.argv[1], "wb"))
' "$out/iostorage.plist"

echo
ls -la "$out"
echo "Done. Review the files, then commit the ones that are useful."
