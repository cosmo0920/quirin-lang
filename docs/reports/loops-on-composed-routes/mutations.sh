#!/usr/bin/env bash
# Mutation harness for card LOOPS-ON-COMPOSED-ROUTES (wave 39).
#
# Each mutation edits one arm the card added or rewrote, runs the named test
# that is supposed to guard it, expects that test RED, restores the file and
# checks it is sha256-equal to what it was. A mutation whose test stays green
# is a vacuous arm and is reported as such (FAIL). Run from the repository
# root; writes mutations.log beside this script.
#
#   bash docs/reports/loops-on-composed-routes/mutations.sh
set -u
cd "$(dirname "$0")/../../.."
LOG="docs/reports/loops-on-composed-routes/mutations.log"
: > "$LOG"
say() { echo "$*" | tee -a "$LOG"; }

BUFFER=crates/qcore-core/src/std_buffer.rs
STRING=crates/qcore-core/src/std_string.rs
FRONT=crates/qcore-frontend/src/source_std_string.rs
TESTS="cargo test -q -p qcore-compiler --test loops_on_composed_routes"

say "mutations.sh — $(date -u +%Y-%m-%dT%H:%M:%SZ) — HEAD $(git rev-parse --short HEAD)"
say "sha256 before: $(sha256sum $BUFFER $STRING $FRONT | awk '{print substr($1,1,16)"  "$2}' | tr '\n' ';')"

status=0

# A red row under a mutation means nothing unless the row is green without
# it: the whole test file is run green before the first mutation and again
# after the last restore, so a stale artifact or a red baseline is reported
# here rather than mistaken for non-vacuity.
baseline() {
  local label="$1"
  if $TESTS > "docs/reports/loops-on-composed-routes/baseline-$label.out" 2>&1; then
    say "baseline ($label): loops_on_composed_routes GREEN"
  else
    say "baseline ($label): loops_on_composed_routes RED — FAIL"
    status=1
  fi
  rm -f "docs/reports/loops-on-composed-routes/baseline-$label.out"
}
baseline before

# Apply a python replacement to a file, run the test, expect red, restore.
#   mutate NAME FILE OLD NEW TEST
mutate() {
  local name="$1" file="$2" old="$3" new="$4" test="$5"
  local before after
  before=$(sha256sum "$file" | cut -c1-64)
  cp "$file" "$file.mutation-backup"
  python3 - "$file" "$old" "$new" <<'PY'
import sys
path, old, new = sys.argv[1], sys.argv[2], sys.argv[3]
text = open(path).read()
assert text.count(old) == 1, f"{path}: the mutation site must occur exactly once ({text.count(old)})"
open(path, "w").write(text.replace(old, new))
PY
  if $TESTS "$test" > "docs/reports/loops-on-composed-routes/mutation-$name.out" 2>&1; then
    say "$name: test '$test' stayed GREEN under the mutation — VACUOUS (FAIL)"
    status=1
  else
    say "$name: test '$test' went RED under the mutation — non-vacuous (ok)"
  fi
  mv "$file.mutation-backup" "$file"
  # The backup's mtime predates the mutated build, so without this cargo's
  # fingerprint would read the restored file as unchanged and keep the rlib
  # compiled from the mutation — measured in this card's first run, where the
  # full gate then failed three rows against artifacts of M5 and M6.
  touch "$file"
  after=$(sha256sum "$file" | cut -c1-64)
  if [ "$before" = "$after" ]; then
    say "$name: $file restored sha256-equal ($(echo "$after" | cut -c1-16))"
  else
    say "$name: $file NOT restored ($before != $after) — FAIL"
    status=1
  fi
  rm -f "docs/reports/loops-on-composed-routes/mutation-$name.out"
}

# M1 — the window function put back to top-level only: a `Repeat` contributes
# no name, so a loan used only inside the loop closes before it opens (the
# STABILITY-REEVAL-3 admitting scenario).
mutate M1-top-level-only-window "$BUFFER" \
'        BufferEvent::Repeat { body, .. } => {
            for event in body {
                custody_names_at_any_depth(event, into);
            }
        }' \
'        BufferEvent::Repeat { .. } => {}' \
  a_loan_used_only_inside_the_loop_is_live_across_it_and_the_body_write_refuses

# M2 — the composed route's body-versus-live-loans judgment removed: the
# `Repeat` arm runs the body without judging its stores against the loans live
# at the loop.
mutate M2-composed-body-not-judged "$STRING" \
'                    if let Some((loan, action)) = loop_body_conflict(
                        &live,' \
'                    if let Some((loan, action)) = loop_body_conflict(
                        &[],' \
  a_body_write_into_a_borrowed_range_refuses_on_the_composed_route

# M3 — the per-iteration lend refusal removed on the buffer route.
mutate M3-per-iteration-lend-admitted "$BUFFER" \
'            if event.any(&|inner| matches!(inner, BufferEvent::Lend { .. })) {
                return Err(vec![BufferError::Lend {
                    event: position,
                    refusal: LendRefusal::Shape(
                        "this `for` body opens a lend, which is a per-iteration loan' \
'            if false && event.any(&|inner| matches!(inner, BufferEvent::Lend { .. })) {
                return Err(vec![BufferError::Lend {
                    event: position,
                    refusal: LendRefusal::Shape(
                        "this `for` body opens a lend, which is a per-iteration loan' \
  a_lend_inside_a_body_is_a_per_iteration_loan_and_refuses_by_name

# M4 — the body-custody refusal removed on the buffer route: the depth-aware
# nested-acquire scan then refuses the acquiring fixture under a different
# name, so this mutation shows the NAME is load-bearing, not the admission.
mutate M4-body-custody-unnamed "$BUFFER" \
'            if event.any(&|inner| {
                matches!(
                    inner,
                    BufferEvent::Acquire { .. } | BufferEvent::Release { .. }
                )
            }) {
                return Err(vec![BufferError::Lend {
                    event: position,' \
'            if false && event.any(&|inner| {
                matches!(
                    inner,
                    BufferEvent::Acquire { .. } | BufferEvent::Release { .. }
                )
            }) {
                return Err(vec![BufferError::Lend {
                    event: position,' \
  a_body_that_acquires_or_releases_refuses_by_name_where_a_loan_is_held

# M5 — the composed route's custody list no longer unrolls a body: a body
# store after the buffer's release is then unseen by the inherited checker.
mutate M5-composed-custody-not-unrolled "$STRING" \
'            StringEvent::Buffer(BufferEvent::Repeat { count, body, .. }) => {
                unrolled(body, *count, &names, &mut operations);
            }' \
'            StringEvent::Buffer(BufferEvent::Repeat { .. }) => {}' \
  a_body_store_after_the_release_is_the_inherited_use_after_drop

# M6 — the composed lowering's per-iteration-borrow refusal removed: the buffer
# reader then refuses `let s = borrow_utf8(b)` inside the body under its own
# message, so this mutation shows the NAME is load-bearing, not the admission.
mutate M6-per-iteration-borrow-unnamed "$FRONT" \
'            if refused {
                continue;
            }
            let mut loop_diagnostics = Vec::new();' \
'            if refused && false {
                continue;
            }
            let mut loop_diagnostics = Vec::new();' \
  a_borrow_inside_a_body_is_a_per_iteration_loan_and_refuses_by_name

say "sha256 after:  $(sha256sum $BUFFER $STRING $FRONT | awk '{print substr($1,1,16)"  "$2}' | tr '\n' ';')"
baseline after
if [ "$status" = 0 ]; then say "RESULT: every mutation went red and every file was restored sha256-equal"; else say "RESULT: FAIL (see above)"; fi
exit "$status"
