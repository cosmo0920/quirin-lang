#!/bin/sh
# Run a command in a private mount namespace with fixed paths, so two sides
# publish byte-comparable output (AUDIT-CANONICAL-PATHS).
# Usage: unshare -rm sh pin_run.sh SNAPSHOT SNAP_MOUNT OUT_ROOT OUT_MOUNT OLD COMMAND [ARG...]
#   SNAPSHOT is bound read-write at SNAP_MOUNT and OUT_ROOT at OUT_MOUNT (both
#   empty directories), so every side's analyzer, profiles, contracts and
#   output have one spelling: the contract-source paths a report digests
#   (`resource_contract_sources`, `assumptions`, `build_identity`) and the
#   report path the invocation sidecar records then differ only by input
#   spelling.
#   OLD=yes also makes /home/cosmo/Documents/GitHub/qcore-c a read-only bind
#   mount of the real checkout instead of the symlink it is on this host, so
#   the pinned inputs' old spelling resolves to itself (the namespace control).
#   Every sibling of qcore-c stays visible: the parent is replaced by a tmpfs
#   holding a bind mount of each original entry (the libevent provider's
#   contracts.lock resolves a tip identity in /home/cosmo/Documents/GitHub/monkey;
#   the corpus run of attempt 2 hid it, see the report).
# Nothing outside the calling process tree changes.
set -eu
snapshot=$1
snap_mount=$2
out_root=$3
out_mount=$4
old=$5
shift 5
mount --bind "$snapshot" "$snap_mount"
mount --bind "$out_root" "$out_mount"
if [ "$old" = yes ]; then
  parent=/home/cosmo/Documents/GitHub
  original=$(dirname "$snap_mount")/ns-parent
  mount --bind "$parent" "$original"
  mount -t tmpfs -o mode=0755 none "$parent"
  for entry in "$original"/* "$original"/.[!.]*; do
    [ -e "$entry" ] || [ -L "$entry" ] || continue
    name=$(basename "$entry")
    [ "$name" = qcore-c ] && continue
    if [ -L "$entry" ]; then
      cp -P "$entry" "$parent/$name"
    elif [ -d "$entry" ]; then
      mkdir "$parent/$name"
      mount --bind "$entry" "$parent/$name"
    else
      touch "$parent/$name"
      mount --bind "$entry" "$parent/$name"
    fi
  done
  mkdir "$parent/qcore-c"
  mount --bind /media/Data/GitHub/quirin-lang "$parent/qcore-c"
  mount -o remount,bind,ro "$parent/qcore-c"
fi
cd "$snap_mount"
exec "$@"
