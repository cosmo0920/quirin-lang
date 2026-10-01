#!/usr/bin/env bash
# targeted.sh NAME -- CG29-EXECUTION-COVERAGE-AND-RECLAMATION's own test
# targets (core units, CG00 controls, the publication route in gateway,
# verification and audit), each exit written to private/cg29/NAME-*.exit.
# Nothing is piped.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cg29"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
step() {
  label="$1"; shift
  "$@" > "$out/$name-$label.log" 2>&1
  echo $? > "$out/$name-$label.exit"
}
step core cargo test -p qcore-core --lib -- execution_coverage reclamation weak_memory atomic_publication
step clang-gap-core cargo test -p qcore-core --test clang_gap_core
step gateway-route cargo test -p qcore-c-gateway --lib -- atomic_source_route
step gateway-events cargo test -p qcore-c-gateway --test atomic_weak_memory_events
step verification cargo test -p qcore-verification --test atomic_publication_source
step audit cargo test -p qcore-audit --lib -- atomic_publication
step compiler-ct41 cargo test -p qcore-compiler --test ct_corpus -- atomic_publication
