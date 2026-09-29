#!/usr/bin/env python3
"""CG04-CONDITION-ACQUISITION: the ledger edits, applied once.

usage: update_ledger.py MATRIX_JSON

Retires CG04-CONDITION-ACQUISITION (every DISCHARGE clause holds), the
absorbed W44-CONDITION-CALL-TYPE-OPERATOR (its discharge holds) and
CONDITION-DESUGAR-REFUSAL-ARMS-UNPINNED (one test per arm; its six unexplained
W44 leaves are not identifiable from W44's archive, and the current analog is
registered); resizes the other absorbed follow-ups with re-measured sizes;
registers the card's two residuals; and keeps `dimensions.named_followups`
equal to the list length. Writes the matrix back in its own form (indent=1,
ensure_ascii=False, trailing newline).

Sizes are the AFTER corpus arbiter's (run private/cg04/after1, base main
27dc7c9a), joined with its parity stream by scripts/followups.py,
scripts/census.py and scripts/unconsulted.py.
"""

import json
import sys

REPORT = "docs/reports/cg04-condition-acquisition/README.md"
BY = "CG04-CONDITION-ACQUISITION"

RETIRE = [
    "CG04-CONDITION-ACQUISITION",
    "W44-CONDITION-CALL-TYPE-OPERATOR",
    "CONDITION-DESUGAR-REFUSAL-ARMS-UNPINNED",
]

REGISTER = [
    {
        "id": "CG04-LOOP-SITE-GENERATIONS",
        "title": "Soundness: a re-acquired site on a CFG cycle merges a live earlier generation, and a refuted failure edge erases it (statement form wrongly Safe; condition form refused)",
        "why": (
            "Found by CG04-CONDITION-ACQUISITION (gateway rule 10x). The flow keeps one abstract site per acquisition "
            "node: `create_resource` (legacy_c.rs) re-inserts the site LIVE when a loop re-executes the acquisition, "
            "without distinguishing a live earlier generation, and a refuted nullable acquisition on the failure edge "
            "removes the site. So `for (;;) { p = malloc(n); if (p == NULL) break; use(p); }` and `do { p = malloc(n); "
            "if (p) use(p); } while (p != NULL);` are reported Safe while every earlier generation leaks; with another "
            "exit (`while (flag) { ... }`) the leak is reported. Pinned as observed on the card's base by "
            "crates/qcore-c-gateway/tests/condition_acquisition.rs "
            "(`the_statement_form_loop_site_defect_is_pinned`). Rule 10x keeps the condition form clear of it: a "
            "custody split on a node a CFG cycle may pass through is refused (`placement`, `acquisition-on-cycle`: 2 "
            "consultations over 2 functions on the corpus at main 27dc7c9a, `rd_gz_decompress` and "
            "`rd_kafka_lz4_decompress`). The statement form's corpus exposure is not measured. DISCHARGE: the flow "
            "tells a live earlier generation of a site at its re-acquisition apart (reported or refused, with Lean "
            "parity), a measured corpus re-pin names every Safe -> non-Safe move as a correction, the pinned test "
            "flips, and rule 10x's placement guard is lifted with its own re-pin."
        ),
    },
    {
        "id": "CG04-UNCONSULTED-CONDITION-LEAVES",
        "title": "Condition refusals no desugaring or elaborator consultation answers: 129 leaves on 86 Unknown functions",
        "why": (
            "Found by CG04-CONDITION-ACQUISITION, the current analog of the six 'unexplained' leaves of "
            "docs/reports/w44-branch-condition-calls/residuals.txt (which W44's archive does not identify: "
            "residuals.json holds counts and no examples). On the corpus at main 27dc7c9a, 129 controlling-expression "
            "refusals in 86 Unknown functions carry no `condition-desugar` and no `elaboration-condition` parity row at "
            "their (function, line) (scripts/followups.py class `no-consultation`). scripts/unconsulted.py groups them "
            "by the other rows at that line: 123 have no parity row there, read by sampling as mostly call-free "
            "conditions holding a type query the fixed scalar query does not fold (`len > sizeof(redacted)`, "
            "`i < RD_ARRAYSIZE(exp)`, `(size_t)r > sizeof(ret[reti]) - of`: CG08), which the elaborator declines as "
            "call-bearing (`sizeof (`) before it records and whose desugared plan keeps no typed or elaborated "
            "operand; 6 share their line with statement or typed-condition rows only. Each keeps its refusal "
            "(fail-closed). "
            "DISCHARGE: a consultation row for every refused controlling expression (or a census that keys them by "
            "their own tokens), and each group explained or registered with its size."
        ),
    },
]

AMEND = {
    "W44-CONDITION-CALL-ASSIGNMENT": (
        " RESIZED by CG04-CONDITION-ACQUISITION (absorbed; rule 10x): a destination is now any automatic "
        "non-formal local the lexical or Clang reader types (rule 10l's declared local: `void *`, typedef'd "
        "pointers), and an owned result assigned in a condition is admitted when the leaf splits it on its own "
        "carrier's failure value off every cycle. Re-measured on the corpus at main 27dc7c9a (AFTER run, "
        "scripts/followups.py, leaves joined to the card's `condition-desugar` parity rows): 231 leaves / 95 "
        "Unknown functions, 1 sole-blocked: `AssignmentShape` 219 (a store through a member or pointer, "
        "`(&x)->str = rd_slice_ensure_contig(...)`, `md->brokers = rd_tmpabuf_alloc0(...)`: CG11's pointer-valued "
        "stores) and `AssignmentDestination` 12 (a formal, or a local neither reader types). DISCHARGE unchanged "
        "for the rest."
    ),
    "W44-CONDITION-CALL-FREE-OPERAND": (
        " RESIZED by CG04-CONDITION-ACQUISITION (absorbed): a call-free leaf the older routes refuse is now asked "
        "of the elaborator (rule 10s) last, and one holding an increment goes to it directly. Re-measured on the "
        "corpus at main 27dc7c9a (AFTER run, scripts/followups.py): 63 leaves / 44 Unknown functions, 0 "
        "sole-blocked (`CallFreeLeafRefused`, 68 consultations over 47 functions; e.g. "
        "`elen >= sizeof(buf) - of`, a `sizeof` of an array the fixed scalar query does not fold (CG08), and "
        "`__builtin_expect((!(!*\"...\")), 0)`). DISCHARGE unchanged."
    ),
    "W44-CONDITION-CALL-REFUSED-SHAPES": (
        " RESIZED by CG04-CONDITION-ACQUISITION (absorbed; rule 10x): the owned-result class is discharged (the "
        "NULL split), and increments are decided per leaf (a call-free leaf's is the elaborator's). Re-measured "
        "on the corpus at main 27dc7c9a (AFTER run, scripts/followups.py): 72 leaves / 62 Unknown functions, 3 "
        "sole-blocked: a call through a member or expression 29 (CG22), braces and statement expressions 22 "
        "(CG07), other classifier refusals 15 (withdrawn contracts, literal-argument premises, shadowed formals, "
        "nested calls (CG05)), an increment beside a call in one leaf 4 (unsequenced, CG05), an acquisition on a "
        "CFG cycle 2 (CG04-LOOP-SITE-GENERATIONS). DISCHARGE unchanged for the rest."
    ),
    "W44-CONDITION-DISCARDED-DERIVED-RESULT": (
        " NOTE by CG04-CONDITION-ACQUISITION (absorbed): re-read and kept. Rule 10x binds no synthetic temporary; "
        "a discarded derived result stays a discarded `None`, and a derived result bound to the program's own "
        "pointer and split on NULL is the use `TestedSplit`, accepted as `Tested` accepted it, with identical "
        "instructions. The integrator's decision that the reading stands is still the discharge."
    ),
    "W43-INDEX-TRAVERSAL-CONDITION-WRITE": (
        " RESIZED by CG04-CONDITION-ACQUISITION (absorbed; rule 10x): a write in a call-free leaf of a desugared "
        "condition (`n-- > 0 && p`, `i++ < 3 && f(x)`) is the elaborator's events (rule 10s, sequenced by the "
        "connective), then a branch that refines nothing. Re-measured by scripts/followups.py as call-free "
        "controlling expressions answered only by the elaborator whose text writes (`++`, `--`, `=`) in Unknown "
        "functions: 172 leaves / 103 functions on the BEFORE run, 71 / 62 on the AFTER run (main 27dc7c9a); "
        "W43's own census definition was not re-run. The rest are pointer steps (`*p++`, CG16) and "
        "pointer-valued conditions the elaborator refuses."
    ),
    "W44-CONDITION-CALL-INDIRECT-ARGUMENT": (
        " NOTE by CG04-CONDITION-ACQUISITION: re-measured on the corpus at main 27dc7c9a (AFTER run, "
        "scripts/followups.py): 1,768 leaves / 708 Unknown functions, 21 sole-blocked, the largest condition "
        "class and most of CG00's E13 function list (`strcmp(p->shortname, x)`, `cmt_opts_compare(&c->opts, o)`): "
        "the typed call actual needs a never-bound root (rule 10k) the loop variable is not, or an address of a "
        "place (CG11), both outside this card."
    ),
}


def main():
    path = sys.argv[1]
    with open(path, encoding="utf-8") as stream:
        matrix = json.load(stream)
    followups = matrix["named_followups"]
    ids = [entry["id"] for entry in followups]
    for retired in RETIRE:
        assert retired in ids, retired
    for amended in AMEND:
        assert amended in ids, amended
    for new in REGISTER:
        assert new["id"] not in ids, new["id"]
    followups = [entry for entry in followups if entry["id"] not in RETIRE]
    for entry in followups:
        if entry["id"] in AMEND:
            entry["why"] += AMEND[entry["id"]]
    for new in REGISTER:
        followups.append({**new, "source": REPORT, "recorded_by": BY})
    matrix["named_followups"] = followups
    matrix["dimensions"]["named_followups"] = len(followups)
    with open(path, "w", encoding="utf-8") as stream:
        json.dump(matrix, stream, indent=1, ensure_ascii=False)
        stream.write("\n")
    print(len(ids), "->", len(followups))


if __name__ == "__main__":
    main()
