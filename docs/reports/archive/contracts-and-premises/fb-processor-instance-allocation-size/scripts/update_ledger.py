#!/usr/bin/env python3
"""update_ledger.py -- FB-PROCESSOR-INSTANCE-ALLOCATION-SIZE: register the
card's outcome and residuals in the acceptance matrix's named follow-ups
(indent=1, ensure_ascii=False; `dimensions.named_followups` = list length).

The card's own entry, FB-PROCESSOR-INSTANCE-ALLOCATION-SIZE, is registered by
FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE, which had not landed when these records
were written, so the outcome is a sibling entry for the integrator to
reconcile (retire both once the original is on main). Idempotent: an id
already present is left as it is."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
CARD = "FB-PROCESSOR-INSTANCE-ALLOCATION-SIZE"
SOURCE = "docs/reports/fb-processor-instance-allocation-size/README.md"

REGISTER = [
    {
        "id": "FB-PROCESSOR-INSTANCE-ALLOCATION-SIZE-MEASURED",
        "title": "flb_processor_instance_create's sizeof(struct flb_filter_instance) allocation: measured benign but fragile (type mismatch, no overflow)",
        "why": (
            f"Sibling of {CARD} (registered by FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE, not on main when "
            "this was written), carrying its outcome. src/flb_processor.c:2171 (identical at 66910c10, "
            "046accf6 and the slice export 34155955; unchanged in every local ref since it was added by "
            "f3a433c3, 2023-04-12) allocates sizeof(struct flb_filter_instance) for a struct "
            "flb_processor_instance. Clang's layout (-fdump-record-layouts-complete) and an "
            "offsetof/sizeof probe, each with the unit's own recorded command, agree in all eight "
            "recorded configurations (corpus 66910c10, system 046accf6, the pin tree 046accf6, slice "
            "34155955; static and shared objects each): processor 152 bytes, filter 216, every "
            "processor member ends at or before 152. Derived readings (not recorded builds): without "
            "FLB_HAVE_REGEX and FLB_HAVE_METRICS 152 vs 200; i686 96 vs 132 (124 without both); armv7 "
            "and aarch64 not measured (host headers refuse those targets). A Clang AST census of 48 "
            "units that can reach a processor instance (flb_processor.c, flb_help.c, flb_notification.c, "
            "flb_metrics_exporter.c, every processor plugin unit, the processor tests; fluent-bit.c "
            "read partially and by text) finds 550 member accesses over all 16 members, none past "
            "byte 152, and only void * / NULL / (void) conversions of the pointer, no view as another "
            "record type; pu->ctx is read as a filter instance only under unit_type "
            "FLB_PROCESSOR_UNIT_FILTER (set once in flb_processor_unit_create). No byte function and "
            "no sizeof(struct flb_processor_instance) anywhere. Margin 64 bytes on LP64 (48 without "
            "the two #ifdef filter members), 36 on i686 (28). Not a defect; no upstream note drafted. "
            "DISCHARGE: reconcile with FB-PROCESSOR-INSTANCE-ALLOCATION-SIZE once it is on main (retire "
            "both; the analysis residual is BOUNDS-ALLOCATION-SIZED-BY-ANOTHER-TYPE), or re-measure if "
            "either record changes at a later pinned revision."
        ),
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "BOUNDS-ALLOCATION-SIZED-BY-ANOTHER-TYPE",
        "title": "The bounds property cannot state an allocation's byte size, so an allocation sized by another type than its pointee is never checked",
        "why": (
            f"Registered by {CARD}. Over src/flb_processor.c alone (scripts/processor-selection.json, "
            "quirin-audit current at 046accf6 and 34155955, release binary 87c51c03 from main "
            "1c7333db1, QCORE_BOUNDS_PROPERTY and QCORE_NULL_PROPERTY): flb_processor_instance_create "
            "is bounds Unknown with 49 bounds-missing-extent accesses, 19 of them `no extent established "
            "for `instance``; no contract establishes the byte size of flb_calloc's result (rule 11u: "
            "\"no current contract establishes one\"; quirin-provider-fluent-bit memory.qv states "
            "forwarded_size / forwarded_n only), and nothing compares an established size with the "
            "pointee record's sizeof at the void * -> T * conversion (here 216 >= 152, measured by "
            "Clang, by this card only). Prerequisite: CG17-FLEXIBLE-EXTENT-UNESTABLISHED (an "
            "allocation's byte size at acquisition from malloc-family size actuals). DISCHARGE: that "
            "size established at flb_calloc / calloc / malloc results, an obligation per typed access "
            "(Clang's offset + width <= established size) at the conversion of the result to a record "
            "pointer, Lean parity, and re-measurement with flb_processor_instance_create's accesses "
            "decided (expected in bounds at every recorded configuration)."
        ),
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "FB-PROCESSOR-NULL-ROWS-OFF-ROUTE",
        "title": "src/flb_processor.c is on no audited route; its null rows include two unresolved post-destroy candidates and one provisional Unsafe",
        "why": (
            f"Registered by {CARD} (same runs as BOUNDS-ALLOCATION-SIZED-BY-ANOTHER-TYPE; identical at "
            "046accf6 and 34155955). The unit is in neither the profile's audit.json, the system nor the "
            "tail-pause selection. Null over its 72 functions (36 per object): 26 Safe unconditional, 16 "
            "Safe conditional, 28 Unknown, 2 Unsafe (10 / 60 / 2 joined with custody). (1) flb_processor_instance_create: after "
            "flb_processor_instance_destroy(instance); instance = NULL; (encoder create failed, :2198-2199) "
            "the function continues to `instance->log_decoder = ...` (:2203) and, after the decoder "
            "branch, `flb_log_event_decoder_read_groups(instance->log_decoder, ...)` (:2209); the null "
            "property reports checks 35 and 44 as candidates from the null literal, triage unresolved "
            "(the path runs through the strcasecmp-terminated plugin lookup). By reading, reachable only "
            "when flb_log_event_encoder_create or flb_log_event_decoder_create returns NULL (allocation "
            "or initialisation failure); not validated, not a defect claim. (2) flb_processor_run: a "
            "null-on-normal-path finding at :1379 (mk_list_foreach over `list`, NULL when `type` is none "
            "of the four FLB_PROCESSOR_* values), provisional: no caller passing another type was "
            "established. DISCHARGE: the unit placed on a measured route (or a selection of its own) "
            "with each row read: (1) its failure path validated against the encoder/decoder contracts "
            "or shown unreachable, (2) the callers' `type` values established, before anything is "
            "presented upstream."
        ),
        "source": SOURCE,
        "recorded_by": CARD,
    },
]


def main():
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    before = len(entries)
    ids = {e["id"] for e in entries}
    for entry in REGISTER:
        if entry["id"] not in ids:
            entries.append(entry)
    matrix["dimensions"]["named_followups"] = len(entries)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(f"named_followups {before} -> {len(entries)}")


if __name__ == "__main__":
    main()
