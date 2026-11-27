#!/usr/bin/env bash
# Which of card LOOPS-ON-COMPOSED-ROUTES' (wave 39) own tests does the window
# rule actually anchor? (card COMPOSED-WINDOW-REACH, wave 41)
#
# `LOOPWINDOWS-ADMITTED-SHAPES-TAUTOLOGY` says "some of the card's
# admitted-shape tests are true by construction rather than by the rule". This
# measures it rather than arguing it: apply that card's own M1 (the window
# function's `Repeat` arm emptied, so a loop contributes no name and a loan
# used only inside a loop closes before it opens), run the whole wave-39 file
# per test, and record which rows notice. A row that stays green under the
# mutation is not thereby a tautology — it may be about something else — but a
# row the card presents as evidence *for the window* and that stays green is.
#
#   bash docs/reports/composed-window-reach/wave39_anchoring.sh
set -u
cd "$(dirname "$0")/../../.."
HERE=docs/reports/composed-window-reach
LOG="$HERE/wave39-anchoring.log"
: > "$LOG"
say() { echo "$*" | tee -a "$LOG"; }

BUFFER=crates/qcore-core/src/std_buffer.rs
TESTS="cargo test -q -p qcore-compiler --test loops_on_composed_routes"

say "wave39_anchoring.sh — $(date -u +%Y-%m-%dT%H:%M:%SZ) — HEAD $(git rev-parse --short HEAD)"
before=$(sha256sum "$BUFFER" | cut -c1-64)

names=$(grep -oP '^fn \K[a-z0-9_]+' compiler/tests/loops_on_composed_routes.rs \
        | grep -v '^\(options\|compile\|codes\|messages\|admitted\|buffer_module\|string_module\|split_lines\|observation\|run_native\)$')
# The `#[test]` rows are the ones after a `#[test]` attribute; take them by name.
names=$(awk '/^#\[test\]/{getline; if ($0 ~ /^fn /) {sub(/^fn /,""); sub(/\(.*$/,""); print}}' \
        compiler/tests/loops_on_composed_routes.rs)

say "wave-39 test rows: $(echo "$names" | wc -l)"

cp "$BUFFER" "$BUFFER.anchor-backup"
python3 - "$BUFFER" <<'PY'
import sys
path = sys.argv[1]
old = """        BufferEvent::Repeat { body, .. } => {
            for event in body {
                custody_names_at_any_depth(event, into);
            }
        }"""
new = "        BufferEvent::Repeat { .. } => {}"
text = open(path).read()
assert text.count(old) == 1, f"the M1 site must occur exactly once ({text.count(old)})"
open(path, "w").write(text.replace(old, new))
PY
say "applied wave-39 M1 (the window function's Repeat arm emptied)"

red=0
green=0
for name in $names; do
  if $TESTS "$name" -- --exact > "$HERE/anchor-$name.out" 2>&1; then
    say "GREEN under M1  $name"
    green=$((green + 1))
  else
    say "RED under M1    $name"
    red=$((red + 1))
  fi
  rm -f "$HERE/anchor-$name.out"
done

mv "$BUFFER.anchor-backup" "$BUFFER"
touch "$BUFFER"
after=$(sha256sum "$BUFFER" | cut -c1-64)
if [ "$before" = "$after" ]; then
  say "restored sha256-equal ($(echo "$after" | cut -c1-16))"
else
  say "NOT restored ($before != $after) — FAIL"
fi
say "SUMMARY: $red row(s) anchored by the window rule, $green row(s) not."
