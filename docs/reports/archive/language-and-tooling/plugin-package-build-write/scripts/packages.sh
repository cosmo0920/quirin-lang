#!/usr/bin/env bash
# packages.sh BASE TIP -- PLUGIN-PACKAGE-BUILD-WRITES-EXECUTABLE-IN-PROCESS:
# `providers/package.py build` at BASE and at TIP, each into a fresh output and
# target directory under private/ppb/, then a comparison of the two
# packages.json records and of every installed file. The two files this card
# changes are restored from BASE for the first build and from TIP for the
# second (the tree is otherwise TIP's); the working tree is left at TIP.
# Run it under the heavy lock. Exits: private/ppb/packages/*.exit.
set -u
base="$1"; tip="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/ppb/packages"
mkdir -p "$out" || exit 2
files="providers/quirin-provider-plugin/src/package.rs providers/quirin-provider-plugin/tests/sdk.rs"
cd "$repo" || exit 2
for label in base tip; do
  revision="$base"; [ "$label" = tip ] && revision="$tip"
  for file in $files; do
    git show "$revision:$file" > "$file" || exit 2
  done
  python3 providers/package.py build --output "$out/$label" \
    --target-dir "$repo/private/ppb/packaging-$label" > "$out/$label.log" 2>&1
  echo $? > "$out/$label.exit"
done
git diff --quiet -- $files
echo $? > "$out/tree-at-tip.exit"
python3 - "$out" > "$out/compare.txt" <<'EOF'
import hashlib, json, sys
from pathlib import Path
out = Path(sys.argv[1])
records = {label: json.loads((out / label / "packages.json").read_text()) for label in ("base", "tip")}
print("packages.json equal:", records["base"] == records["tip"])
for artifact in records["tip"]["artifacts"]:
    print(artifact["plugin_id"], artifact["executable_sha256"], artifact["archive_sha256"])
def tree(root):
    return {str(p.relative_to(root)): (hashlib.sha256(p.read_bytes()).hexdigest(), oct(p.stat().st_mode & 0o7777))
            for p in sorted(root.rglob("*")) if p.is_file()}
base, tip = tree(out / "base"), tree(out / "tip")
print("installed files:", len(tip), "identical (bytes and mode):", base == tip)
EOF
echo $? > "$out/compare.exit"
