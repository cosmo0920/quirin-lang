#!/usr/bin/env python3
"""FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE: retire FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE
(its DISCHARGE holds), amend NULL-MEMBER-SCOPE-ENTRY-INFERENCE and
NULL-MEMBER-STABLE-CENSUS with the Fluent Bit sizes, register the residuals
with sizes. Idempotent. indent=1, ensure_ascii=False;
`dimensions.named_followups` = list length."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
CARD = "FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE"
SOURCE = "docs/reports/fb-plugin-descriptor-nonnull/README.md"
MEASURED = (
    "Measured by FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE (binary 91e1d574, tip a642e71f3; custody unchanged on "
    "every route; null corpus 1,114 / 1,435 / 2,354 / 18, system 86 / 125 / 243, slice 69 / 121 / 216; "
    "stable flb_{input,output,filter,processor}_instance.p and ten nonnull_member_scope(flb_input_instance, "
    "p, ins) entries)"
)

NEW = [
    {
        "id": "NULL-MEMBER-COPY-TEST-PATH",
        "title": "A tested copy of a member does not discharge the member path at a later scope-entry call",
        "why": MEASURED + ": flb_input_pre_run_all and flb_input_exit_all (slice, both configurations) read "
        "`p = ins->p; if (!p) continue;` and then call a scope function with `ins`; rule 14z's caller check "
        "reads the path `ins->p`, which the walk does not relate to the tested copy `p`, so each call is a "
        "memory-field candidate (4 rows, already Unknown). DISCHARGE: the walk records that a local assigned "
        "from a tracked path `V->FIELD` equals it until either is assigned (or, without a stable member, "
        "until a call or pointer store), so a test of the copy decides the path; Lean beside "
        "QCore.NullMemberScope; re-measured.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "FB-PLUGIN-DESCRIPTOR-RESIDUAL-READERS",
        "title": "Fluent Bit plugin-descriptor reads that the stable member and the ten scope entries leave candidates",
        "why": MEASURED + ": candidates sourced `...->p` fell system 90 -> 80 (19 functions) and slice "
        "250 -> 224 (71 functions; in_tail's `ctx->ins->p` member bases included). Not declared: two static "
        "functions with only `ins->p` candidates (input_instance_channel_events_init, handle_input_event; a "
        "declaration names a function run-wide and rule 14v exports nothing from a static definition); every "
        "output reader (flb_output_pre_run / flb_output_exit / flb_output_init_all read a list element "
        "`ins` from config->outputs, the property checks and flb_output_set_property keep other candidates); "
        "member bases (`ctx->ins->p` in sb.c and in_tail, which rule 14y does not keep); and 4 caller rows "
        "the entries moved Safe -> Unknown (flb_input_rate_gate_protect, flb_input_test_pause_resume, both "
        "configurations) plus new caller candidates in flb_input_pause_all, flb_input_rate_gate_resume, "
        "flb_input_chunk_protect and flb_input_chunk_set_limits. DISCHARGE: scope entries on the callers "
        "with an instance formal (or NULL-MEMBER-SCOPE-ENTRY-INFERENCE), a reading for list elements of a "
        "container whose every element was stored by a constructor that sets p (e.g. a container-element "
        "stable fact), and a member-base reading; each measured with every moved row read.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
]

AMEND = {
    "NULL-MEMBER-SCOPE-ENTRY-INFERENCE": (
        " FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE declared ten Fluent Bit entries chosen mechanically from a "
        "stable-only run (scripts/scope_candidates.py): 20 slice rows Unknown -> Safe conditional; their "
        "caller checks moved 4 rows Safe -> Unknown and added 12 slice, 6 system and 3 corpus candidates."
    ),
    "NULL-MEMBER-STABLE-CENSUS": (
        " FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE declared flb_{input,output,filter,processor}_instance.p stable "
        "(hosts the four constructors; every analysed store proved, none withdrawn)."
    ),
}

# This card's observation that flb_processor_instance_create allocates
# `sizeof(struct flb_filter_instance)` is not registered. Its DISCHARGE was met
# on main by card FB-PROCESSOR-INSTANCE-ALLOCATION-SIZE
# (docs/reports/fb-processor-instance-allocation-size/): benign but fragile;
# the processor instance is 152 bytes and the filter instance 216 in every
# configuration, and none of the 550 accesses goes past byte 152. That card
# registered the sibling FB-PROCESSOR-INSTANCE-ALLOCATION-SIZE-MEASURED
# because the original entry was not on main. The sibling resolves the
# observation and is retired here.
RETIRE = {CARD, "FB-PROCESSOR-INSTANCE-ALLOCATION-SIZE-MEASURED"}


def main():
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    entries[:] = [e for e in entries if e["id"] not in RETIRE]
    for entry in entries:
        if entry["id"] in AMEND and AMEND[entry["id"]] not in entry["why"]:
            entry["why"] += AMEND[entry["id"]]
    known = {e["id"] for e in entries}
    for entry in NEW:
        if entry["id"] not in known:
            entries.append(entry)
    matrix["dimensions"]["named_followups"] = len(entries)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(len(entries))


main()
