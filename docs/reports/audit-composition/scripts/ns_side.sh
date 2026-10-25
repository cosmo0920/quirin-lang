#!/bin/sh
# Run a command with one side's snapshot bind-mounted at the common path
# private/ac/side-src, so BASE and TIP are built and run at the same spelling
# (the audit embeds the analyzer's source path in contract-module paths and in
# the digests taken over them). Private to the calling process tree.
# Usage: unshare -rm ns_side.sh WORKTREE SIDE COMMAND [ARG...]   (SIDE: base|tip)
set -eu
W=$1
side=$2
shift 2
mount --bind "$W/private/ac/$side-src" "$W/private/ac/side-src"
exec "$@"
