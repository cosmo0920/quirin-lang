#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# Card MEASURE-CONTRACT-ONLY-SIDE-WIRING evidence harness: the command
# sequence `quirin-audit measure --reuse-contract-only --verify-reuse` runs for
# a pinned corpus route, executed by hand on a host whose AppArmor policy
# refuses unprivileged user namespaces (so `rev:` sides cannot run there).
# The mount namespace's one spelled tree and working directory are replaced
# by ONE real tree (checked out at each revision in turn) and ONE real
# working directory, which spell the same paths the namespace would.
#
# Usage: demo.sh DEMO_DIR INPUTS_ROOT LINK_MAP BASE TIP_A TIP_B
#   DEMO_DIR/src : a clone holding BASE, TIP_A, TIP_B, with
#                  target/release/quirin-audit built at BASE
#                  (QCORE_AUDIT_BUILD_COMMIT=BASE)
#   INPUTS_ROOT  : the pinned corpus inputs (c-gateway-wave-pin/...,
#                  provider-packages-wave20/qcore.chunkio)
# Writes DEMO_DIR/runs/<side>/corpus/ in the measure tool's side layout, so
# `quirin-audit measure --side NAME=run:DEMO_DIR/runs/<side>` compares them.
set -u
DEMO=$1 INPUTS=$2 LINKS=$3 BASE=$4 TIP_A=$5 TIP_B=$6
S=$DEMO/src
W=$DEMO/work
R=$DEMO/runs
BIN=$S/target/release/quirin-audit
mkdir -p "$W" "$R"

# One pinned corpus run: the measure corpus route's environment and
# run_pinned_audit.py's (Clang at this host's path), from the one
# working directory.  Usage: corpus SIDE JOBS [pinned options...]
corpus() {
  local side=$1 jobs=$2
  shift 2
  local route=$R/$side/corpus
  mkdir -p "$route/corpus"
  date -u +%FT%TZ > "$route/started.txt"
  ( cd "$W" && /usr/bin/time -v -o "$route/time.txt" \
    env -u QCORE_SOURCE_CONTRACTS \
      QCORE_CLANG=/usr/lib/llvm-21/bin/clang \
      QCORE_CHUNKIO_PACKAGE="$INPUTS/provider-packages-wave20/qcore.chunkio" \
      QCORE_PROFILES="$S/contract/profiles" \
      QCORE_CROSS_UNIT_LINK_MAP="$LINKS" \
      QCORE_NULL_PROPERTY="$route/corpus-null.jsonl" \
      QCORE_OUTER_PROJECTION="$route/corpus-outer.json" \
      SOURCE_DATE_EPOCH=1767225600 \
      "$BIN" pinned --jobs "$jobs" "$@" \
      "$INPUTS/c-gateway-wave-pin/fluent-bit-clang-source" \
      "$INPUTS/c-gateway-wave-pin/monkey/monkey-build/compile_commands.json" \
      "$route/corpus/monkey-source.json" "$route/corpus/resource-diagnostics.json" \
      > "$route/console.log" 2> "$route/stderr.log" )
  echo $? > "$route/corpus.exit"
  date -u +%FT%TZ > "$route/ended.txt"
}

classify() {
  "$BIN" pinned --classify-only --accept-base-analyzer "$BASE" --repository "$S" \
    > "$R/$1.classification.json" 2> "$R/$1.classification.stderr"
  echo $? > "$R/$1.classification.exit"
}

# Base side: seal every group.
git -C "$S" checkout -q --detach "$BASE"
corpus base 12 --checkpoint-dir "$DEMO/checkpoints"
cp "$DEMO/checkpoints/pinned-checkpoint-run.json" "$R/base.checkpoint-run.json"
for tip in a b; do
  rm -rf "$DEMO/checkpoints-$tip"
  cp -r "$DEMO/checkpoints" "$DEMO/checkpoints-$tip"
done

# Each tip: classified by the base binary, then resumed from the base side's
# checkpoints under the acceptance and, beside it, run in full.
for tip in a b; do
  if [ "$tip" = a ]; then rev=$TIP_A; else rev=$TIP_B; fi
  git -C "$S" checkout -q --detach "$rev"
  classify "tip-$tip"
  corpus "tip-$tip" 8 --checkpoint-dir "$DEMO/checkpoints-$tip" --resume \
    --accept-base-analyzer "$BASE" --repository "$S" &
  corpus "tip-$tip-full" 8 &
  wait
  cp "$DEMO/checkpoints-$tip/pinned-checkpoint-run.json" "$R/tip-$tip.checkpoint-run.json"
done
git -C "$S" checkout -q --detach "$BASE"
