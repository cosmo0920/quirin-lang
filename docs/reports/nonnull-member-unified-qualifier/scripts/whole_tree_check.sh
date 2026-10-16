#!/usr/bin/env bash
# whole_tree_check.sh BINARY LOG_DIR -- NONNULL-MEMBER-UNIFIED-QUALIFIER: the
# whole-tree resource-lock check (`pin --check`, no `--bounds`) at both audited
# revisions with BINARY, to compare the not-in-force entries a binary reports
# (this card changes no resource lock). Output and exit to files.
set -u
binary="$1"; logs="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
old=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source
new=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source
mkdir -p "$logs"
cd "$repo" || exit 2
for root in "$old" "$new"; do
  tag=$(basename "$(dirname "$root")")
  "$binary" pin --source-root "$root" --check > "$logs/check-all-$tag.log" 2>&1
  echo $? > "$logs/check-all-$tag.exit"
  echo "check-all-$tag $(cat "$logs/check-all-$tag.exit")"
  grep '^  x' "$logs/check-all-$tag.log" | sed 's/:.*//' > "$logs/check-all-$tag.not-in-force"
done
