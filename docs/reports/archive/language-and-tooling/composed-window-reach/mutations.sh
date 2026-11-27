#!/usr/bin/env bash
# Mutation harness for card COMPOSED-WINDOW-REACH (wave 41).
#
# Each mutation edits one arm this card added, rewrote or claims is
# load-bearing, runs the named test that is supposed to guard it, expects that
# test RED, restores the file and checks it is sha256-equal to what it was. A
# mutation whose test stays green is a vacuous arm and is reported as such
# (FAIL). Run from the repository root; writes mutations.log beside this
# script.
#
#   bash docs/reports/composed-window-reach/mutations.sh
#
# Restores are by `mv` + `touch`, never mtime-preserving: a backup whose mtime
# predates the mutated build makes cargo's fingerprint read the restored file
# as unchanged and keep the rlib compiled from the mutation
# (`MUTATION-HARNESS-STALE-ARTIFACTS`, measured by card
# LOOPS-ON-COMPOSED-ROUTES' own first run). The Lean half additionally removes
# the module's `.olean` so `lake` cannot serve a stale one.
set -u
cd "$(dirname "$0")/../../.."
HERE=docs/reports/composed-window-reach
LOG="$HERE/mutations.log"
: > "$LOG"
say() { echo "$*" | tee -a "$LOG"; }

BUFFER=crates/qcore-core/src/std_buffer.rs
STRING=crates/qcore-core/src/std_string.rs
FRONT=crates/qcore-frontend/src/source_std_string.rs
LEAN=proof/QCore/LoopWindows.lean
TESTS="cargo test -q -p qcore-compiler --test composed_window_reach"

say "mutations.sh — $(date -u +%Y-%m-%dT%H:%M:%SZ) — HEAD $(git rev-parse --short HEAD)"
say "sha256 before: $(sha256sum $BUFFER $STRING $FRONT $LEAN | awk '{print substr($1,1,16)"  "$2}' | tr '\n' ';')"

status=0

# A red row under a mutation means nothing unless the row is green without it.
baseline() {
  local label="$1"
  if $TESTS > "$HERE/baseline-$label.out" 2>&1; then
    say "baseline ($label): composed_window_reach GREEN"
  else
    say "baseline ($label): composed_window_reach RED — FAIL"
    status=1
  fi
  rm -f "$HERE/baseline-$label.out"
}

lean_baseline() {
  local label="$1"
  if (cd proof && lake build QCore.LoopWindows) > "$HERE/lean-baseline-$label.out" 2>&1; then
    say "lean baseline ($label): QCore.LoopWindows BUILDS"
  else
    say "lean baseline ($label): QCore.LoopWindows FAILS TO BUILD — FAIL"
    status=1
  fi
  rm -f "$HERE/lean-baseline-$label.out"
}

replace() {
  python3 - "$1" "$2" "$3" <<'PY'
import sys
path, old, new = sys.argv[1], sys.argv[2], sys.argv[3]
text = open(path).read()
assert text.count(old) == 1, f"{path}: the mutation site must occur exactly once ({text.count(old)})"
open(path, "w").write(text.replace(old, new))
PY
}

restore() {
  local file="$1" before="$2"
  mv "$file.mutation-backup" "$file"
  touch "$file"
  local after
  after=$(sha256sum "$file" | cut -c1-64)
  if [ "$before" = "$after" ]; then
    say "  $file restored sha256-equal ($(echo "$after" | cut -c1-16))"
  else
    say "  $file NOT restored ($before != $after) — FAIL"
    status=1
  fi
}

# mutate NAME FILE OLD NEW TEST — expects the Rust test RED.
mutate() {
  local name="$1" file="$2" old="$3" new="$4" test="$5"
  local before
  before=$(sha256sum "$file" | cut -c1-64)
  cp "$file" "$file.mutation-backup"
  replace "$file" "$old" "$new"
  if $TESTS "$test" > "$HERE/mutation-$name.out" 2>&1; then
    say "$name: test '$test' stayed GREEN under the mutation — VACUOUS (FAIL)"
    status=1
  else
    say "$name: test '$test' went RED under the mutation — non-vacuous (ok)"
  fi
  restore "$file" "$before"
  rm -f "$HERE/mutation-$name.out"
}

# mutate_lean NAME OLD NEW — expects `lake build QCore.LoopWindows` to FAIL.
mutate_lean() {
  local name="$1" old="$2" new="$3"
  local before
  before=$(sha256sum "$LEAN" | cut -c1-64)
  cp "$LEAN" "$LEAN.mutation-backup"
  replace "$LEAN" "$old" "$new"
  rm -f proof/.lake/build/lib/lean/QCore/LoopWindows.olean
  if (cd proof && lake build QCore.LoopWindows) > "$HERE/mutation-$name.out" 2>&1; then
    say "$name: QCore.LoopWindows STILL BUILDS under the mutation — VACUOUS (FAIL)"
    status=1
  else
    say "$name: QCore.LoopWindows FAILED TO BUILD under the mutation — non-vacuous (ok)"
  fi
  restore "$LEAN" "$before"
  rm -f proof/.lake/build/lib/lean/QCore/LoopWindows.olean
  rm -f "$HERE/mutation-$name.out"
}

baseline before
lean_baseline before

# --- W41-M1: the four field-path arms put back in the empty arm, which is
# exactly the state WINDOW-FIELD-ARM-UNREACHED measured: a name the window
# cannot express, silently contributing nothing.
mutate W41-M1-field-arms-silent "$BUFFER" \
'        BufferEvent::FieldStore { record, field, .. }
        | BufferEvent::FieldLoad { record, field, .. }
        | BufferEvent::FieldExtent { record, field, .. }
        | BufferEvent::FieldRelease { record, field } => into.push(WindowName::FieldPath {
            record: record.as_str(),
            field: field.as_str(),
        }),' \
'        BufferEvent::FieldStore { .. }
        | BufferEvent::FieldLoad { .. }
        | BufferEvent::FieldExtent { .. }
        | BufferEvent::FieldRelease { .. } => {}' \
  a_field_path_is_refused_by_the_window_functions_own_gate

# --- W41-M2: the gate itself removed from the window computation, leaving the
# classification in place but nobody acting on it.
mutate W41-M2-gate-not-consulted "$BUFFER" \
'    for (index, path) in window_unnameable_events(events) {' \
'    for (index, path) in Vec::<(usize, String)>::new() {' \
  a_field_path_is_refused_by_the_window_functions_own_gate

# --- W41-M3: the composed route's shallow window reader emptied at its two
# binding positions. The window then closes at the borrow and an owner write
# before a later read is admitted.
mutate W41-M3-shallow-window-emptied "$STRING" \
'        StringEvent::Slice { name, .. } | StringEvent::Len { name, .. } => vec![name.as_str()],' \
'        StringEvent::Slice { .. } | StringEvent::Len { .. } => Vec::new(),' \
  every_view_reading_position_keeps_the_owner_out

# --- W41-M3b: the same arm against the constructor-by-constructor pin, so the
# section-1 test is shown to depend on the rule and not only on its own shape.
mutate W41-M3b-shallow-pin "$STRING" \
'        StringEvent::Text { operand, .. } => operand_use(operand).into_iter().collect(),
        StringEvent::Concat { pieces, .. } => pieces
            .iter()
            .filter_map(|piece| match piece {
                StringPiece::Text(operand) => operand_use(operand),
                StringPiece::Scalar { .. } => None,
            })
            .collect(),
        StringEvent::Bind { .. }' \
'        StringEvent::Text { .. } => Vec::new(),
        StringEvent::Concat { .. } => Vec::new(),
        StringEvent::Bind { .. }' \
  the_depth_contribution_is_empty_for_every_event

# --- W41-M4: the depth recursion made to contribute a name. The section-1 pin
# claims the contribution is empty for every input; if that claim were
# untestable this mutation would stay green.
mutate W41-M4-depth-contributes "$STRING" \
'            BufferEvent::Repeat { body, .. } => body.iter().flat_map(buffer_view_uses).collect(),' \
'            BufferEvent::Repeat { variable, .. } => vec![variable.as_str()],' \
  the_depth_contribution_is_empty_for_every_event

# --- W41-M9: the composed route's window map emptied entirely. Its *depth*
# half contributes nothing (W41-M4 measures that from the other side), but its
# top level decides: with no entry, `unwrap_or(index)` closes every loan at
# its own borrow and the owner may write immediately. This is the mutation
# that separates "the window is inert" (false) from "the window's recursion is
# inert" (true).
mutate W41-M9-composed-window-emptied "$STRING" \
'    let mut last: BTreeMap<String, usize> = BTreeMap::new();
    for (index, event) in program.events.iter().enumerate() {
        for name in view_uses_at_any_depth(event) {
            last.insert(name.to_owned(), index);
        }
    }
    last' \
'    let _ = program;
    BTreeMap::new()' \
  every_view_reading_position_keeps_the_owner_out

# --- W41-M5: mechanism 2 removed — the composed body's view-construct refusal
# no longer claims a rendering operand.
mutate W41-M5-body-vocabulary-skips-render "$FRONT" \
'                        StringUse::Literal
                        | StringUse::Length
                        | StringUse::Concat
                        | StringUse::Render
                        | StringUse::Slice => {' \
'                        StringUse::Literal
                        | StringUse::Length
                        | StringUse::Concat
                        | StringUse::Slice => {' \
  a_view_constructing_statement_in_a_composed_body_is_refused_by_name

# --- W41-M6: one shared vector's number moved in the LEAN source. The Rust
# differential reads that number out of the Lean file, so this is the mutation
# that shows the binding is mechanical rather than a transcription.
mutate W41-M6-lean-vector-number "$LEAN" \
'{ spelling := "z[2l]z", expected := 1,' \
'{ spelling := "z[2l]z", expected := 2,' \
  the_lean_window_model_and_the_rust_window_function_agree_on_shared_vectors

# --- W41-M7: one shared vector's SPELLING moved, so the Lean side's own
# `the_agreement_table_holds` must fail. This is the other half of the
# binding: neither side can be edited alone.
mutate_lean W41-M7-lean-vector-spelling \
'{ spelling := "z[2[3l]]z", expected := 1,' \
'{ spelling := "z[2[9l]]z", expected := 1,'

# --- W41-M8: the body-shape decision made to admit an event the window
# cannot name — the shape `every_admitted_body_event_is_windowed` refuses, and
# the theorem the tautology it replaced could not have caught.
mutate_lean W41-M8-body-admits-field-path \
'def admittedInBody : LendEvent → Bool
  | .names _ => true
  | .quiet => true
  | .fieldPath => false' \
'def admittedInBody : LendEvent → Bool
  | .names _ => true
  | .quiet => true
  | .fieldPath => true'

baseline after
lean_baseline after

say "sha256 after:  $(sha256sum $BUFFER $STRING $FRONT $LEAN | awk '{print substr($1,1,16)"  "$2}' | tr '\n' ';')"
if [ "$status" -eq 0 ]; then
  say "RESULT: every mutation reddened its named test; every file restored sha256-equal."
else
  say "RESULT: FAIL — see the rows above."
fi
exit "$status"
