#!/usr/bin/env python3
"""CG20-NUMERIC-DOMAIN's ledger edits to the acceptance matrix's
named_followups: retire the card and CG01-CYCLE18-EVALUATOR-WIDTH, amend
W45-FB-SCHED-RANDOM-RANGE, register the residuals. Idempotent; run from the
repository root."""

import json
import pathlib

PATH = pathlib.Path("docs/reviews/r0_3/acceptance_matrix.json")
REPORT = "docs/reports/cg20-numeric-domain/README.md"
CARD = "CG20-NUMERIC-DOMAIN"

RETIRE = {"CG20-NUMERIC-DOMAIN", "CG01-CYCLE18-EVALUATOR-WIDTH"}

SCHED_WHY = (
    "src/flb_scheduler.c:106-108 terminates with probability 1 only when range = "
    "min(cap, base * 2^n) - base + 1 lies in [1, RAND_MAX]; range = 0 (cap = base - 1) "
    "divides by zero at :102, before the loop, and limit = 0 (a condition that is always "
    "true) needs range > RAND_MAX, reachable only by int overflow. Amended by "
    "CG20-NUMERIC-DOMAIN (reading of Fluent Bit 046accf6, source only): scheduler.cap and "
    "scheduler.base are FLB_CONF_TYPE_INT entries (src/flb_config.c:233-238) written only "
    "by flb_config_set_property through atoi into the unsigned int fields via an int * "
    "(:913-915), defaults 2000 / 5 (:456-457), with no ordering or range check in src/; "
    "the only reader is flb_sched_request_create (src/flb_scheduler.c:297). The reading "
    "does not establish the range premise (scheduler.cap 4 with scheduler.base 5 gives "
    "range = 0 at :102). The C integer arithmetic property (gateway rule 11q) reports "
    "random_uniform Unknown with open SignedOverflow (:101 twice, :103), open "
    "DivisionByZero (:102, :110) and open SignedConversion (:96, :97); no native trace was "
    "made, so this is not a validated reachable violation and not an upstream defect claim. "
    "DISCHARGE: a native trace with scheduler.cap = scheduler.base - 1 recording the "
    "outcome, with a default-configuration control; or an upstream ordering check read at "
    "a later pinned revision."
)

NEW = [
    {
        "id": "CG20-SWITCH-AND-CORRELATION-PRECISION",
        "title": "Switch-value case pruning and branch correlation (C04, C05) stay unimplemented",
        "why": (
            "Registered by CG20-NUMERIC-DOMAIN, which implemented the C integer domain and "
            "the arithmetic property (gateway rule 11q) but not the queue's optional "
            "precision work. CG00's controls reproduce unchanged at base b6a3dc42: "
            "c04_switch_values_do_not_prune_cases (route G keeps every case edge, a "
            "provisional Unsafe Leak for a constant switch value) and "
            "c05_branch_correlation_is_not_tracked (if (flag) / if (!flag) reports Leak "
            "and DoubleRelease). Conservative non-Safe results, not soundness defects. "
            "Size: not measured on the corpus. DISCHARGE: a sound refinement of case edges "
            "by a switch value's interval (rule 11q's values) and of correlated conditions "
            "by path predicates, with Rust controls turning both CG00 controls into their "
            "AFTER and Lean parity; or a decision that the precision is not wanted."
        ),
        "source": REPORT,
        "recorded_by": CARD,
    },
    {
        "id": "CG20-ABI-READER-ARCH-FLAG",
        "title": "The lexical ABI reader's plain-long question ignores a unit's -arch flag",
        "why": (
            "Registered by CG20-NUMERIC-DOMAIN (gateway rule 11q (6)): legacy_declared_abi "
            "reads plain long as the 64-bit projection only when the analysing Clang's "
            "default target is LP64 (c_numeric::plain_long_is_lp64, one query per resolved "
            "Clang and process). The reader has no unit context, so an Apple -arch flag "
            "(CFrontendFlag::Architecture, admitted by compile_db) that selects a non-LP64 "
            "architecture on an LP64 host is not seen. No such unit exists in the corpus, "
            "system or slice inputs (all x86_64-pc-linux-gnu). DISCHARGE: the reader asks "
            "the unit's own target model (its frontend flags), with a test under a wrapper "
            "or -arch selecting a 32-bit long; or a refusal of plain long whenever a unit "
            "carries -arch."
        ),
        "source": REPORT,
        "recorded_by": CARD,
    },
    {
        "id": "CG20-RESULT-RANGE-TRANSFER-LEAN",
        "title": "The arithmetic property's result-range transfer is tested, not proved",
        "why": (
            "Registered by CG20-NUMERIC-DOMAIN. QCore.CNumeric proves the discharge rules "
            "(add, sub, mul, div/rem, shift count, negative shifts, shl overflow, signed "
            "conversion) sound for operands in their ranges, but that each operation's "
            "value lies in the interval binary_interval / unary_interval / "
            "convert_interval return (which later sites' discharges rely on) is checked "
            "only exhaustively at width 8 in Rust "
            "(cg20_discharge_is_sound_exhaustively_at_width_eight). DISCHARGE: Lean "
            "theorems for each transfer's result range, bound to the Rust kernel by "
            "vectors."
        ),
        "source": REPORT,
        "recorded_by": CARD,
    },
    {
        "id": "CG20-NUMERIC-CORPUS-CENSUS",
        "title": "The C integer arithmetic property has no corpus-wide census or pinned route",
        "why": (
            "Registered by CG20-NUMERIC-DOMAIN: the property (gateway rule 11q) runs per "
            "unit through c_numeric_obligations::analyze_numeric_obligations and per "
            "recorded compile command through examples/numeric_obligations.rs (measured on "
            "flb_scheduler.c's random_uniform and backoff_full_jitter only). No corpus, "
            "system or slice census of its verdicts or open obligations exists, and the "
            "pinned arbiter does not run it, so no coverage figure is published. "
            "DISCHARGE: a census over the corpus arbiter's units (functions Safe / Unknown "
            "under the property, open and violated obligations by fault) recorded beside "
            "the custody figures, never folded into them."
        ),
        "source": REPORT,
        "recorded_by": CARD,
    },
]


def main() -> None:
    matrix = json.loads(PATH.read_text())
    followups = [f for f in matrix["named_followups"] if f["id"] not in RETIRE]
    for entry in followups:
        if entry["id"] == "W45-FB-SCHED-RANDOM-RANGE":
            entry["why"] = SCHED_WHY
            entry["amended_by"] = CARD
    known = {f["id"] for f in followups}
    followups.extend(entry for entry in NEW if entry["id"] not in known)
    matrix["named_followups"] = followups
    matrix["dimensions"]["named_followups"] = len(followups)
    PATH.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(len(followups))


if __name__ == "__main__":
    main()
