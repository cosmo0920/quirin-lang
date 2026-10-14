#!/usr/bin/env bash
# identity_rebased.sh -- PLUGIN-PACKAGE-BUILD-WRITES-EXECUTABLE-IN-PROCESS: the
# byte-identity checks rerun on the rebased tip (run under the heavy lock).
#  1. SDK: build quirin-plugin at the current tree, package the saved checker
#     (private/ppb/checker) into private/ppb/pkg/rebased, verify it with the
#     base CLI, and compare with private/ppb/pkg/base (the base CLI's package).
#  2. First-party: providers/package.py build into private/ppb/packages/rebased
#     (fresh target dir, deleted afterwards), compared with
#     private/ppb/packages/base (built at b565f79ab by packages.sh).
# Exits and results: private/ppb/identity-rebased/.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
ppb="$repo/private/ppb"
out="$ppb/identity-rebased"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
git rev-parse HEAD > "$out/head"
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
cargo build --manifest-path providers/Cargo.toml -p quirin-provider-plugin --bins --locked --offline \
  > "$out/cli-build.log" 2>&1
echo $? > "$out/cli-build.exit"
mkdir -p "$ppb/bin/rebased" && cp providers/target/debug/quirin-plugin "$ppb/bin/rebased/quirin-plugin"
rm -rf "$ppb/pkg/rebased"
"$ppb/bin/rebased/quirin-plugin" package --executable "$ppb/checker" --output "$ppb/pkg/rebased" \
  > "$out/sdk-package.log" 2>&1
echo $? > "$out/sdk-package.exit"
"$ppb/bin/base/quirin-plugin" verify "$ppb/pkg/rebased" > "$out/sdk-verify-by-base.log" 2>&1
echo $? > "$out/sdk-verify-by-base.exit"
rm -rf "$ppb/packages/rebased" "$ppb/packaging-rebased"
python3 providers/package.py build --output "$ppb/packages/rebased" \
  --target-dir "$ppb/packaging-rebased" > "$out/packages-build.log" 2>&1
echo $? > "$out/packages-build.exit"
rm -rf "$ppb/packaging-rebased"
python3 - "$ppb" > "$out/compare.txt" <<'EOF'
import hashlib, json, sys
from pathlib import Path
ppb = Path(sys.argv[1])
def tree(root):
    return {str(p.relative_to(root)): (hashlib.sha256(p.read_bytes()).hexdigest(), oct(p.stat().st_mode & 0o7777))
            for p in sorted(root.rglob("*")) if p.is_file()}
sdk_base, sdk_rebased = tree(ppb / "pkg/base"), tree(ppb / "pkg/rebased")
print("sdk package files:", len(sdk_rebased), "identical to base (bytes and mode):", sdk_base == sdk_rebased)
for name, (digest, mode) in sorted(sdk_rebased.items()):
    print(" ", name, digest, mode)
records = {label: json.loads((ppb / "packages" / label / "packages.json").read_text())
           for label in ("base", "rebased")}
print("packages.json equal:", records["base"] == records["rebased"])
first_base, first_rebased = tree(ppb / "packages/base"), tree(ppb / "packages/rebased")
print("first-party installed files:", len(first_rebased), "identical to base (bytes and mode):",
      first_base == first_rebased)
EOF
echo $? > "$out/compare.exit"
grep -H . "$out"/*.exit
cat "$out/compare.txt"
