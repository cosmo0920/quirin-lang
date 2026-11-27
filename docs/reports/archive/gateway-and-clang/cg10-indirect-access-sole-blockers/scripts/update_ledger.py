#!/usr/bin/env python3
"""CG10-INDIRECT-ACCESS-SOLE-BLOCKERS: the acceptance-matrix ledger changes.

Registers CG10-INDIRECT-ACCESS-SOLE-RESIDUALS (the ranked residual sub-class
table, from artifacts/classify-after.json) and amends the touched follow-ups
(CG10-TYPED-PLACE-RESIDUALS, CG12-VIEW-RESIDUAL-FORMS,
CG13-UNION-LEAVES-OTHER-CAUSES, W45-CALLEE-CASCADE-RESIDUAL). Idempotent: an
earlier amendment by this card is replaced. Writes with indent=1,
ensure_ascii=False and keeps dimensions.named_followups equal to the list
length.
"""
import json
import pathlib
import sys

CARD = "CG10-INDIRECT-ACCESS-SOLE-BLOCKERS"
REPORT = "docs/reports/cg10-indirect-access-sole-blockers/README.md"
MARK = f" AMENDED ({CARD}"
MEASURED = ("corpus 66910c10, BEFORE main cdc2fa878 d13ed4c1 654/9/4,171, AFTER the card d2f5a3f7 "
            "695/9/4,130")


def residual_entry(after):
    alone = after["functions_freed_alone_by_class"]
    leaves = after["leaves_by_class"]
    touched = after["functions_touched_by_class"]
    ranked = "; ".join(f"{cls} {alone[cls]} alone ({leaves.get(cls, 0)} leaves, {touched.get(cls, 0)} touched)"
                       for cls in list(alone)[:12])
    cumulative = ", ".join(str(freed) for _, freed in after["fixed_point_in_rank_order"][:10])
    return {
        "id": "CG10-INDIRECT-ACCESS-SOLE-RESIDUALS",
        "title": (f"Unknown functions blocked by the indirect-access refusal alone after rules 12i/12j: "
                  f"{after['sole_functions']} functions, {after['sole_leaves']} leaves, ranked by the "
                  "readers' decisions"),
        "why": (f"Measured by {CARD} ({MEASURED}; scripts/classify.py on the AFTER census run, the "
                "measurement-only indirect-census streams and the parity rows): of the "
                f"{after['sole_functions']} Unknown functions whose only refusal family is `indirect "
                "memory access is outside the resource projection`, ranked by functions freed alone: "
                f"{ranked}. Fixed point in rank order: {cumulative}, …. The two largest classes are "
                "not access questions: the lexical gate refuses the statement before its call's "
                "contract is looked up, so an uncontracted callee hides behind the access refusal "
                "(the P4 callee closure, W45-CALLEE-CASCADE-RESIDUAL). custody-position:* is "
                "W44-CALL-ARGUMENT-CUSTODY-POSITION by design; pointer-store:* are rule 10v's refused "
                "values (CG11, Q36's missing owner premise); probe-declined and call-result-store are "
                "call results stored into members (an owned result is the field-link model's, a scalar "
                "one through a pointer root, e.g. `vh->documentroot.len = strlen(v)` in "
                "mk_vhost_set_property, is unread); record-object places on a macro line holding "
                "another statement stay Unknown on that statement. DISCHARGE: each class's owner "
                "re-measures against this table; the entry closes when no class is left that an "
                "access reading alone blocks."),
        "source": REPORT,
        "recorded_by": CARD,
    }


AMEND = {
    "CG10-TYPED-PLACE-RESIDUALS": (
        f"{MARK}, gateway rule 12j; {MEASURED}): a `.`-only scalar place on a record object of the "
        "function's automatic storage, which the typed reader refused before rule 10t's decision "
        "whenever the closure did not prove the root never bound, is read with no root premise (544 "
        "place decisions admitted on the AFTER run, deduplicated per statement and place; 15 functions "
        "Unknown → Safe directly). The array-root, element-place, deeper-place and identity refusals "
        "are unchanged (125 / 92 / 71 / 26 decisions on the card's AFTER, by its deduplication, not the "
        "registered census's method)."),
    "CG12-VIEW-RESIDUAL-FORMS": (
        f"{MARK}, gateway rule 12i; {MEASURED}): the return-position forms this entry names for "
        "`rd_strcmp3` (`return f(*p, …)`), `XXH_readLE32_align` and `XXH_readLE64_align` are read: a "
        "refused `return`'s one call is now read with rule W44's call-argument access, and all three "
        "are Safe on the card's AFTER (with `XXH_readLE32`/`XXH_readLE64` through their summaries). "
        "The other forms stand; the M05 leaf count was not re-run."),
    "CG13-UNION-LEAVES-OTHER-CAUSES": (
        f"{MARK}, gateway rule 12j; {MEASURED}): scalar reads and stores through a union member of "
        "a record object of the function's automatic storage are read with rule 10w's decision and "
        "frame and no root premise (the librdkafka telemetry builders over "
        "`rd_kafka_telemetry_metric_value_t`, twelve now Safe). The M06 leaf count was not re-run."),
    "W45-CALLEE-CASCADE-RESIDUAL": (
        f"{MARK}; {MEASURED}): a further population blocked by uncontracted callees hides behind the "
        "access refusal: of the 513 Unknown functions blocked by `indirect memory access is outside "
        "the resource projection` alone on the BEFORE, 218 are call statements and 103 whole-call "
        "returns whose callee has no contract or summary in force (W44's own record: position "
        "`uncontracted`); the gate refuses before the contract lookup, so they are counted in the "
        "access family, not in this entry's. Lifting the access frees none of them "
        "(CG10-INDIRECT-ACCESS-SOLE-RESIDUALS ranks them on the AFTER)."),
}


def main() -> int:
    root = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else pathlib.Path(".")
    after = json.loads((root / "docs/reports/cg10-indirect-access-sole-blockers/artifacts/"
                        "classify-after.json").read_text())
    path = root / "docs/reviews/r0_3/acceptance_matrix.json"
    matrix = json.loads(path.read_text())
    entries = matrix["named_followups"]
    by_id = {entry["id"]: entry for entry in entries}
    for name, text in AMEND.items():
        entry = by_id[name]
        why = entry["why"]
        if MARK in why:
            why = why[:why.index(MARK)]
        entry["why"] = why.rstrip() + text
    new = residual_entry(after)
    if new["id"] in by_id:
        by_id[new["id"]].update(new)
    else:
        entries.append(new)
    matrix["dimensions"]["named_followups"] = len(entries)
    path.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(len(entries))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
