#!/bin/sh
# CONTRACT-SUPERSESSION one-time re-pin (user decision 9a): stamp every entry
# of every provider pin lock (resource, bounds, outer) with one reviewed
# `pinned_at` and tie each lock to the latest Fluent Bit revision, changing no
# identity, then verify every lock and create the lock-set window.
# Usage: repin.sh BINARY LOG_DIR STAMP SOURCE_ROOT
#   SOURCE_ROOT: the prepared Fluent Bit checkout at the latest revision
#   (046accf6aded7906f66a1baf0202da349e521736, Fluent Bit 5.1.3).
set -u
binary=$1
logs=$2
stamp=$3
root=$4
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../../../.." && pwd)
mkdir -p "$logs"
cd "$repo" || exit 2
step() {
  name=$1
  shift
  "$@" >"$logs/$name.log" 2>&1
  echo $? >"$logs/$name.exit"
  echo "$name $(cat "$logs/$name.exit")"
}
step stamp-resource "$binary" pin --source-root "$root" --stamp-only --pinned-at "$stamp"
step stamp-bounds "$binary" pin --source-root "$root" --stamp-only --pinned-at "$stamp" --bounds
step stamp-outer "$binary" pin --source-root "$root" --stamp-only --pinned-at "$stamp" --outer
revision=$(cat "$root/.git/HEAD")
step window-create "$binary" pin --window-create "$revision"
step verify "$binary" pin --verify-contracts
step window-check "$binary" pin --window-check
