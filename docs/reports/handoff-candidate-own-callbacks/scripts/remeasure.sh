#!/usr/bin/env bash
# remeasure.sh -- HANDOFF-CANDIDATE-OWN-CALLBACKS: after the rebase onto
# 711287771, measure main's binary (m711, built from its export by
# build-main.sh, reading its own contracts) and the rebased tip's (b2) on every
# route with the null property, one after the other (after.sh), so the shared
# target/release/quirin-audit the corpus arbiter runs is never written while
# the other run reads it.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
bash "$here/after.sh" m711 "$repo/private/hcoc/bin/m711"
bash "$here/after.sh" b2 "$repo/private/hcoc/bin/b2"
cat "$repo/private/hcoc/m711.step.exit" "$repo/private/hcoc/b2.step.exit"
