#!/usr/bin/env python3
"""Register this card's residuals and its one retirement in the carried
`named_followups` block of `docs/reviews/r0_3/acceptance_matrix.json`
(card LOOPS-ON-COMPOSED-ROUTES, wave 39).

Retirement: `STRING-CONCAT-LOOP-WINDOW`, registered by the wave-38 close, met
word for word (its discharge: "revisit the window's computation for a loop
construct - a window over iterations ... and amend 15h's sentence deliberately
when it lands"). `SLICE-PERMISSIONS-LOOP-WINDOW` has no entry to retire: the
wave-38 close deliberately did not register it (a constraint-named rule under
CONSTRAINT-NAMED-BOUNDARIES-NOT-OPEN-WORK), and the amended constraint 15h
records its lifting. `WRITE-LEND-RELEND` is left open, deliberately.

Idempotent: re-running neither duplicates an entry nor re-retires one.
"""
import json
import sys
from pathlib import Path

CARD = "LOOPS-ON-COMPOSED-ROUTES"
REPORT = "docs/reports/loops-on-composed-routes/README.md"
MATRIX = Path("docs/reviews/r0_3/acceptance_matrix.json")

NEW = [
    {
        "id": "LOOP-PER-ITERATION-LOAN",
        "title": "A lend or a borrow opened inside a for body is a per-iteration loan, refused by name on both routes",
        "why": (
            "Card LOOPS-ON-COMPOSED-ROUTES (wave 39) computes a loan's window over iterations for a "
            "TOP-LEVEL loan: a loan a loop body names is live across the whole loop, proved sound "
            "relative to LOOP-V1's unrolling (QCore.LoopWindows). A loan OPENED inside a body is a "
            "different construct - its window opens and closes inside one iteration and is not an "
            "interval of top-level statement indices at all, so neither the theorem nor the "
            "lend-structured projection (whose borrow nodes are top-level windows) covers it. It is "
            "refused by name: a `lend(...)` inside a body at Q051 on the buffer route and at Q077 on "
            "the composed route (checker, before the window function runs), and a `borrow_utf8(...)` "
            "inside a body at Q077 at the composed lowering. Mutation M3 shows the buffer-route refusal "
            "load-bearing: without it the per-iteration lend's stores reach the inherited checker as "
            "no operation and the program is ADMITTED. DISCHARGE: a per-iteration window - a borrow "
            "node opened and closed inside the unrolled body at every iteration - with its own "
            "exclusivity statement against the top-level loans live across the loop, landed in the "
            "IR rather than in a route (constraint 15h), or the shape recorded as a permanent measured "
            "boundary."
        ),
        "source": REPORT,
        "recorded_by": CARD,
    },
    {
        "id": "LOOP-WINDOW-BODY-CUSTODY",
        "title": "A for body neither acquires nor releases where a loan is held: refused by name on the lending buffer route and on the composed route",
        "why": (
            "Card LOOPS-ON-COMPOSED-ROUTES (wave 39). On the plain buffer route (no lend) a loop body may "
            "acquire and release, decided by LOOP-V1's per-iteration identity gate (Q099) over the "
            "unrolled operation list with per-iteration scoped names. In an entry that LENDS the "
            "projection is the lend-structured one, which opens borrow nodes over top-level windows and "
            "produces no identity gates, and on the composed string route the custody list is a flat "
            "unrolling with no gates either - so a custody step inside an iteration would be decided by "
            "neither gate. Rather than admit it, an acquisition or a release inside a body is refused by "
            "name at Q051 (buffer route, entries that lend) and Q077 (composed route). The refusal is "
            "fail-closed and the direct-caller path cannot admit: a body release the projection never "
            "saw leaves the owner live at the normal exit (the inherited leak), a body acquisition it "
            "never saw makes every later use an unknown name. DISCHARGE: produce LOOP-V1's identity "
            "gates on both projections (the prefix inside a borrow node's body being the open question) "
            "and lift the refusal with a test per shape, or record it as a permanent measured boundary."
        ),
        "source": REPORT,
        "recorded_by": CARD,
    },
    {
        "id": "LOOP-ON-COMPOSED-ROUTE-BODY-VOCABULARY",
        "title": "A for body on the composed string route is source.buffer.v1's vocabulary; a string construct, a text line, a len-bound header or an unrolling licence inside one is refused",
        "why": (
            "Card LOOPS-ON-COMPOSED-ROUTES (wave 39) lowers a `for` in a composed module through "
            "source.buffer.v1's own loop reader, so the body is that route's statement grammar: a "
            "string view, `len`, `concat` or a text line (`print \"...\"`) inside a body is refused at "
            "Q077 by name at the lowering; a loop bound naming a `len(s)` binding is refused by the "
            "loop's own Q098 (the lowering-visible tier holds literal lets and extents, not view "
            "lengths); and a `[| #unroll, where #bound < N |]` clause list before a composed `for` "
            "refuses at Q068 as a statement the string route does not model, so a composed loop is "
            "held to the default budget. Each is fail-closed and measured (loops_on_composed_routes.rs "
            "section 5). DISCHARGE: extend the composed route's body grammar construct by construct, "
            "each with the window consequence stated (a view read inside a body lengthens the window "
            "through view_uses_at_any_depth, already written for it), or record the boundary as "
            "permanent."
        ),
        "source": REPORT,
        "recorded_by": CARD,
    },
]

RETIRE = {"STRING-CONCAT-LOOP-WINDOW"}


def main() -> int:
    matrix = json.loads(MATRIX.read_text())
    followups = matrix["named_followups"]
    ids = {entry["id"] for entry in followups}
    for name in RETIRE:
        if name not in ids:
            print(f"retirement target {name} is not registered (already retired?)")
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
