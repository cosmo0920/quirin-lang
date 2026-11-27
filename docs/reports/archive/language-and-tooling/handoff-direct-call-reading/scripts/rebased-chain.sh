#!/usr/bin/env bash
# rebased-chain.sh MAIN-REV MAIN-NAME TIP-NAME -- HANDOFF-DIRECT-CALL-READING:
# after a rebase, build main's binary from an export of MAIN-REV
# (build-main.sh) and the rebased tip's (after-chain.sh builds, runs the census
# and measures it), then measure main's binary on every route with the null
# property (after.sh). Each step takes the shared heavy lock itself; exits to
# private/hdcr/*.exit.
set -u
rev="$1"; main="$2"; tip="$3"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/hdcr"
"$here/build-main.sh" "$rev" "$main" > "$out/$main-build.out" 2>&1
echo $? > "$out/$main-chain.build.exit"
[ "$(cat "$out/$main-chain.build.exit")" = 0 ] || exit 1
"$here/after.sh" "$main" "$out/bin/$main"
echo $? > "$out/$main-chain.after.exit"
"$here/after-chain.sh" "$tip"
echo $? > "$out/$tip-chain.exit"
