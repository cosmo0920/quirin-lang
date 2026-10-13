#!/usr/bin/env bash
# isolated.sh LABEL CRATE-DIR BINARY [ARGS...] -- PROCESS-FIXTURES-FS-WRITE-OUTSIDE-HOST-CRATE:
# rerun one gate test binary alone, from its crate root, to attribute a gate
# failure (recorded in the report; the gate's own exit stands). Output to
# private/pfw/iso-LABEL.log, exit to private/pfw/iso-LABEL.exit. Nothing piped.
set -u
label="$1"; cwd="$2"; binary="$3"; shift 3
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
cd "$repo/$cwd" || exit 2
"$binary" "$@" > "$repo/private/pfw/iso-$label.log" 2>&1
echo $? > "$repo/private/pfw/iso-$label.exit"
