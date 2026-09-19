#!/usr/bin/env python3
"""Register this card's residuals and its four retirements in the carried
`named_followups` block of `docs/reviews/r0_3/acceptance_matrix.json`
(card COMPOSED-WINDOW-REACH, wave 41).

Every retirement below was read in full before being taken, and each is met on
a disjunct the entry itself names — quoted in the card's report §9 beside the
evidence. Every registration carries a discharge condition and `recorded_by`.

Idempotent: re-running neither duplicates an entry nor re-retires one.
"""
import json
import sys
from pathlib import Path

CARD = "COMPOSED-WINDOW-REACH"
REPORT = "docs/reports/composed-window-reach/README.md"
MATRIX = Path("docs/reviews/r0_3/acceptance_matrix.json")

NEW = [
    {
        "id": "COMPOSED-BODY-BARE-RENDER-INCIDENTAL",
        "title": "A bare `print s;` of a view inside a composed for body is kept out by a rendering rule, not by a window rule",
        "why": (
            "MEASURED by card COMPOSED-WINDOW-REACH (wave 41). The composed string route's window "
            "function contributes nothing at a `for` (std_string::view_uses_at_any_depth's depth half "
            "is empty for every input, pinned by the_depth_contribution_is_empty_for_every_event), so "
            "its soundness rests entirely on no admitted composed body being able to read a view. That "
            "rests on THREE mechanisms, not the one the code claimed. (1) a body holds BufferEvents and "
            "no BufferEvent constructor names a view - a type-level fact. (2) a body statement that "
            "CONSTRUCTS a view (a quoted literal, a len, a concat, an inline-slice render, a "
            "borrow_utf8) is refused at Q077 by name "
            "(LOOP-ON-COMPOSED-ROUTE-BODY-VOCABULARY / LOOP-PER-ITERATION-LOAN), measured over six "
            "spellings. (3) a BARE name at a rendering operand - `print s;`, `log info s;` - is "
            "DELIBERATELY NOT claimed by StringUse::Render (source_std_string.rs:270-284: a bare name "
            "must not pull a module that never mentions a string onto this route, so `print bogus;` "
            "keeps refusing at Q030). It is refused instead by source.buffer.v1's own line reader, "
            "whose message is '`print` names a `std.fmt` rendering: `dec` or `hex`' - measured, "
            "probes/verdicts.log rows P1/P1b/P1c/P4. That third mechanism is a RENDERING rule doing a "
            "window's work. It is incidental: a card that admitted a selector-free `print NAME;` on the "
            "buffer route, or that gave the buffer route a view-shaped scalar, would remove it without "
            "reading constraint 15h. Pinned by "
            "a_bare_render_of_a_view_inside_a_composed_body_is_refused, which asserts BOTH that the "
            "message is the rendering one and that it is NOT the vocabulary one, so making the closure "
            "deliberate turns the row red rather than passing silently. NOT A DEFECT TODAY: no program "
            "is wrongly admitted (21 probes, section 2 of the report). DISCHARGE: claim the bare-name "
            "render at a composed `for` body in body_string_use so mechanism 3 becomes a window rule "
            "with its own name and message, and rewrite this test's second assertion; OR make the "
            "window's depth recursion live by admitting a view read in a composed body, which retires "
            "the question; OR record the incidental closure as a permanent measured boundary in "
            "constraint 15h, which already names it."
        ),
        "source": REPORT,
        "recorded_by": CARD,
    },
    {
        "id": "WINDOW-FIELD-PATH-REFUSED-NOT-WINDOWED",
        "title": "The window function refuses a field path rather than giving it a window, and the card that lifts the owned-field refusals must do the latter",
        "why": (
            "Card COMPOSED-WINDOW-REACH (wave 41) discharged WINDOW-FIELD-ARM-UNREACHED on the second "
            "of its two disjuncts - 'the arms must be made an explicit refusal rather than an empty "
            "vector' - and this is what that choice leaves open. std_buffer::custody_names_at_any_depth "
            "now classifies names as WindowName::Binding or WindowName::FieldPath, and "
            "window_unnameable_events turns a field path into a refusal by name at Q051 (a condition "
            "extension; no code allocated), reached from lend_windows. So the day "
            "OWNED-FIELD-LEND-WINDOW or OWNED-FIELD-LOOP-BODY is lifted, the window function REFUSES "
            "rather than silently forgetting - fail-closed, the direction invariant 1 wants - but it "
            "still does not WINDOW a field path. What is missing is the key space: a lend binding is "
            "one identifier, and a window keyed by `RECORD.FIELD` needs the map, the LendWindow "
            "binding field, the custody projection's segment walk (which addresses owners by plain "
            "name, OWNED-FIELD-LEND-WINDOW's own stated ground) and crossed_lend_windows to agree "
            "about paths. Measured beside it: the gate is unreachable from source today - a "
            "535-program emission census moved no verdict in either direction - so this is a tripwire "
            "and not a live refusal. DISCHARGE: the card that lifts either owned-field refusal must, "
            "in the same change, give the four Field* arms real windows (a key space that holds a "
            "path, through the projection and crossed_lend_windows), land a test that shows the window "
            "LENGTHENING across a field-path use in a body, and retire this gate deliberately; OR "
            "record the refusal as permanent, which means an entry that lends never composes an owned "
            "field."
        ),
        "source": REPORT,
        "recorded_by": CARD,
    },
    {
        "id": "LOOPWINDOWS-BINDING-IS-FINITE",
        "title": "The Lean-to-Rust window binding is eight shared vectors, not an extraction, and says nothing about a ninth program",
        "why": (
            "Card COMPOSED-WINDOW-REACH (wave 41) discharged LOOPWINDOWS-LEAN-IS-A-TRANSCRIPTION on "
            "two of its three disjuncts: the unrolling is instantiated at a lend-carrying alphabet "
            "(QCore.LoopWindows.LendEvent, the_lend_alphabets_unrolling_is_loop_v1s_recursion beside "
            "loop_v1s_unrolling_is_the_same_flattening), and the doc-comment link is replaced by a "
            "differential test over shared vectors - compiler/tests/composed_window_reach.rs reads "
            "proof/QCore/LoopWindows.lean AS TEXT, extracts all eight rows of its `agreement` table, "
            "rebuilds each spelling as a Vec<BufferEvent> and asserts last_uses returns the Lean "
            "number, while the Lean side proves the_agreement_table_holds (each row's spelling is its "
            "own program's rendering AND each row's number is lastUse's answer). Non-vacuity is "
            "measured in both directions: W41-M6 moves one row's number in the LEAN file and the Rust "
            "test goes red; W41-M7 moves one row's SPELLING and the Lean build fails. WHAT IS NOT "
            "PROVED: that last_uses and lastUse agree on any program OUTSIDE those eight. The binding "
            "is finite by construction - eight vectors chosen to exercise a flat last use, a body-only "
            "use, a nested-body use, a loop naming another binding, a use before and inside a loop, "
            "and a program with no use - and a divergence on a ninth shape would be invisible to it. "
            "The Rust test guards the table's size (>= 8 rows) and pins the body-only row, so shrinking "
            "the vector set is a red row rather than a silent loss. PROVED: properties of the Lean "
            "model, at an alphabet the implementation produces. OBSERVED, on eight points: that the "
            "Rust function is that model. DISCHARGE: an extraction, a generated digest over the two "
            "definitions, or a property-based/exhaustive differential over generated programs up to a "
            "stated depth and alphabet - any of which replaces 'eight points' with a quantifier; OR "
            "this sentence carried as the binding's stated boundary."
        ),
        "source": REPORT,
        "recorded_by": CARD,
    },
]

# Each retirement's own discharge condition, quoted in the report, met by this
# card. Read in full before being taken.
RETIRE = {
    # "amend 15h to attribute the composed route's repair to
    # check_string_program's Repeat arm and record view_uses_at_any_depth as
    # written-ahead-of-its-grammar" — done, in constraint 15h and in the
    # function's own documentation, with the residual's mistaken title
    # corrected and the closure's three mechanisms named.
    "COMPOSED-VIEW-WINDOW-INERT",
    # "the arms must be made an explicit refusal rather than an empty vector"
    # — done for the four field-path arms; the two the residual miscounted
    # (Construct, Field) are measured reachable and correctly empty.
    "WINDOW-FIELD-ARM-UNREACHED",
    # "a correspondence mechanism (… a differential test over shared vectors …)
    # binding last_uses to lastUse; OR an instantiation of the LOOP-V1 theorem
    # at an alphabet carrying Lend" — both.
    "LOOPWINDOWS-LEAN-IS-A-TRANSCRIPTION",
    # "restate the theorem's entry … OR relate AdmittedInBody to the Rust
    # admitted set by a checked correspondence" — the card took the stronger
    # route and superseded the theorem outright; the stated concern ("so the
    # theorem is not later cited as coverage of the route's admitted set") is
    # gone because the theorem is.
    "LOOPWINDOWS-ADMITTED-SHAPES-TAUTOLOGY",
}


def main() -> int:
    matrix = json.loads(MATRIX.read_text())
    followups = matrix["named_followups"]
    ids = {entry["id"] for entry in followups}
    missing = sorted(name for name in RETIRE if name not in ids)
    if missing:
        print(f"retirement targets not registered (already retired?): {missing}")
    before = len(followups)
    followups = [entry for entry in followups if entry["id"] not in RETIRE]
    retired = before - len(followups)
    added = 0
    for entry in NEW:
        if entry["id"] in ids:
            continue
        followups.append(entry)
        added += 1
    matrix["named_followups"] = followups
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(f"named_followups: {before} -> {len(followups)} (retired {retired}, added {added})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
