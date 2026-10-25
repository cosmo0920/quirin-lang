#!/bin/sh
# Run a command where /home/cosmo/Documents/GitHub/qcore-c is a read-only bind
# mount of the real checkout, not the symlink it is on this host, so the
# pinned inputs' old spelling resolves to itself (AUDIT-CANONICAL-PATHS
# namespace control). Usage: unshare -rm ns_run.sh COMMAND [ARG...]
# Private to the calling process tree: nothing outside the namespace changes.
set -eu
parent=/home/cosmo/Documents/GitHub
mount -t tmpfs -o mode=0755 none "$parent"
mkdir "$parent/qcore-c"
mount --bind /media/Data/GitHub/quirin-lang "$parent/qcore-c"
mount -o remount,bind,ro "$parent/qcore-c"
exec "$@"
