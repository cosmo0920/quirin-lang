#!/usr/bin/env bash
# rebase-measure.sh MAIN BRANCH -- GATEWAY-ANALYSIS-PATH-OPTIMIZATION: after the
# rebase onto MAIN, the new BEFORE and AFTER: cargo check of the rebased tip,
# then the release binary of MAIN (checked out detached for the build only)
# as private/gapo/bin/main2 and of the rebased tip (BRANCH) as bin/rebased,
# each measured on the three routes with measure-atomic.sh (main2, then
# rebased), and identity.py main2 rebased. Exits to files; nothing piped.
set -u
main="$1"; branch="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/gapo"
cd "$repo" || exit 2
# the scripts are read from a private copy: MAIN's tree does not have them
tools="$out/tools/scripts"   # four levels under the checkout, as the scripts expect
rm -rf "$out/tools" && mkdir -p "$out/tools" && cp -r "$here" "$tools" || exit 2
"$tools/cargo-step.sh" check-rebased check --workspace --all-targets || exit 3
git checkout -q "$main" || exit 4
"$tools/build.sh" main2; built=$?
git checkout -q "$branch" || exit 5
[ "$built" = 0 ] || exit 6
"$here/build.sh" rebased || exit 7
for name in main2 rebased; do
  BINARY="$out/bin/$name" "$here/measure-atomic.sh" "$name"
done
python3 "$here/identity.py" main2 rebased "$out/identity-rebased.json" > "$out/identity-rebased.txt" 2>&1
echo $? > "$out/identity-rebased.exit"
tail -n 1 "$out/identity-rebased.txt"
