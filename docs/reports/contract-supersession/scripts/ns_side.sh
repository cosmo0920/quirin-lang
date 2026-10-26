#!/bin/sh
# Run a command with one side's snapshot bind-mounted at the common path
# private/cs/side-src, so BASE and TIP are built and run at one spelling (the
# audit embeds its own source path in contract-module paths and digests).
# Usage: unshare -rm ns_side.sh WORKTREE SIDE COMMAND [ARG...]   (SIDE: base|tip)
set -eu
W=$1
side=$2
shift 2
mount --bind "$W/private/cs/$side-src" "$W/private/cs/side-src"
exec "$@"
