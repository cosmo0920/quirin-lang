#!/usr/bin/env python3
"""Apply W45-SR5-SOUNDNESS's hand-maintained matrix edits once, on the merged tree.

Usage: update_ledger.py   (then regenerate with docs/reports/remaining-d02/build_matrix.py)

* retires the eleven entries this card discharged into `retired-followups.json`,
  each with its ground (the evidence named), refusing if one is not registered;
* amends W45-UNCHECKED-DECLARATIONS-TRUSTED by an appended sentence (skipped
  when its marker is already present);
* appends every entry of `followups.json` (refusing a duplicate id).
"""
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w45-sr5-soundness"
CARD = "W45-SR5-SOUNDNESS"
MARK = "AMENDED (W45-SR5-SOUNDNESS):"
TESTS = "crates/qcore-c-gateway/tests/sr5_soundness.rs"
PROBE = "docs/reports/w45-sr5-soundness/probes/ (probe-before.log, probe-after.log)"

RETIRED = {
    "MEMBER-RELEASE-BEFORE-SUMMARY-RELEASE": (
        f"Discharged: rule 5k reads every R(B) for what R does to B->F (legacy_member_release.rs "
        f"section 4); a unit-defined callee that releases the member again makes the release a "
        f"DoubleRelease candidate and withholds the place. The probe's shape, its null-tested "
        f"form, the wrapper, the declared-and-defined destructor and the same-member-next row are "
        f"pinned Unsafe with the double release in {TESTS} (the unit verdict too); the neighbours "
        f"stay Safe. Lean: QCore.MemberReleaseCallee (an_admitted_member_is_not_released_again, "
        f"a_reported_place_is_never_admitted). Residual premise: W45-SR5-DECLARED-RELEASE-DOES-NOT-CASCADE."
    ),
    "HELD-PREFIX-SOURCE-FORGES-PENDING": (
        f"Discharged: legacy_reserved_spelling.rs escapes every source identifier spelled with a "
        f"reserved rewrite prefix before any rewrite mints one, on every route; an escaped callee "
        f"is Unknown (reserved-prefix-in-source). Both texts corrected (legacy_pin_hold.rs, "
        f"legacy_review_hold.rs). The probe's rows are pinned Unknown in {TESTS} and a genuine "
        f"hold stays Pending. Lean: QCore.ReservedSpelling (pending_only_from_a_minted_hold). "
        f"Gateway rule 10i."
    ),
    "PUBLICATION-ROOT-RELEASED": (
        f"Discharged by the second arm: a publication into a container whose root the body hands "
        f"whole to a non-Borrowed position (a direct release, a summary's Released, through a local "
        f"initialised from the formal) refuses (legacy_container_publication.rs rule 7, gateway "
        f"10d); pinned not Safe in {TESTS} beside the Safe publication alone. Lean: "
        f"QCore.Sr5Soundness (a_released_root_discharges_nothing)."
    ),
    "COUNTED-USER-CONDITIONAL-OPERAND": (
        f"Discharged: the counted-user walker undecides a call or counter write behind ?:, && or "
        f"|| and treats f(a)(b) as opaque (legacy_counted_users.rs; the unit pin that read f(a)(b) "
        f"as named is inverted). The fixture `u_ref(t); c ? u_unref(t, 0) : (void)0; return 0;` "
        f"and the && and f(a)(b) forms are Undecided, neither body nor caller Safe, in {TESTS}. "
        f"Lean: QCore.Sr5Soundness (a_skippable_or_opaque_event_holds_no_clause)."
    ),
    "CONTAINER-OF-FORMAL-PUBLICATION-LAUNDER": (
        f"Discharged: a publication of an element rule 5l(2) bound over a formal takes the formal "
        f"entry's rule (ForeignOrigins::note_over_formal, legacy_container_publication.rs rule 8); "
        f"the link helper over `void *arg` and the caller that frees after linking are pinned not "
        f"Safe in {TESTS}. Lean: QCore.Sr5Soundness (an_element_over_a_formal_is_never_borrowed)."
    ),
    "DECLARED-NEUTRAL-UNVERIFIED-AT-CALL": (
        f"Discharged: call_effect routes a declared resource_user_neutral() of a body the unit "
        f"defines through verify_declared (legacy_counted_users.rs); a helper writing the counter "
        f"or operating in a loop leaves its caller's clause Undecided, and the verifying control "
        f"stays Safe, in {TESTS}. The Fluent Bit slice at 046accf6 is leaf-identical. Lean: "
        f"QCore.Sr5Soundness (an_unverified_neutral_body_is_opaque)."
    ),
    "SCALAR-READ-PARENTHESISED-DOUBLE-DEREF": (
        "Discharged: the reader counts a `*` through parentheses and ends the dereference only "
        "after closing ones (legacy_return_family.rs stars_before, ends_one_dereference); unit pins "
        "assert None for (*q)->len, (*q)[0], **q, *(*q), (q->next)->len, (q[0])->len, *(q->next), "
        "*(q[0]) and the cast shape `*(q *) p` (tests/unit/legacy_return_family.rs). Lean: "
        "QCore.Sr5Soundness (an_admitted_read_dereferences_once). Pre-existing neighbour: "
        "W45-SR5-SCALAR-READ-STAR-THROUGH-CAST."
    ),
    "GATEWAY-5L-ELEMENT-TRACKED-AFTER-FOREIGN-UNLINK": (
        f"Discharged: DOCS-COMPACT-9 revised 5l's text to the post-10f state; this card pins the "
        f"fixture that re-publishes a foreign-unlinked element into the iterated list under an "
        f"alias head spelling not Safe (a_foreign_element_republished_under_an_alias_head_is_not_safe "
        f"in {TESTS}; probe row 5l-republish-alias-head, Unknown at base and after, {PROBE})."
    ),
    "STATIC-ROOT-SHADOW-READER": (
        f"Discharged by the fixture arm: a global shadowed by a declaration containing `(` (a "
        f"parenthesised initialiser, a parenthesised declarator) is pinned not Safe beside the "
        f"unshadowed Safe control (a_shadowed_static_root_is_not_a_root in {TESTS}); the probe "
        f"measured all three shadowing rows Unknown at base ({PROBE})."
    ),
    "PENDING-BYPASSES-UNIT-SAFE-GATES": (
        "Discharged: the unread-definition gate and the certificate target scope apply to a "
        "Pending unit in legacy_c.rs, and projection.rs withholds a Pending unit the inventory "
        "withholds; any failure is Unknown (gateway 10i). Fixtures: a held call beside an `#if 0` "
        "or K&R definition is an Unknown unit on the gateway route "
        "(crates/qcore-c-gateway/tests/sr5_soundness.rs) and on the audit route "
        "(crates/qcore-audit/tests/sr5_pending_gates.rs), the function row keeping its Pending "
        "reason. Lean: QCore.Sr5Soundness (a_gated_unit_is_never_pending). Residual: "
        "W45-SR5-PENDING-INVENTORY-ARM-UNDRIVEN."
    ),
    "W45-REVIEW-PREFIX-COLLISION-UNPINNED": (
        "Discharged by its second arm, made true again: the review hold's comment is corrected, "
        "and a collision can no longer read as held at all, because every source-spelled reserved "
        "prefix is escaped before the rewrites run (legacy_reserved_spelling.rs, unit tests "
        "there; crates/qcore-c-gateway/tests/sr5_soundness.rs pins the collision with a withheld "
        "name Unknown). Since held calls became Pending the entry's 'so no verdict moves' was "
        "false (HELD-PREFIX-SOURCE-FORGES-PENDING); it is true again."
    ),
}

AMEND = {
    "W45-UNCHECKED-DECLARATIONS-TRUSTED": (
        " " + MARK + " inside the defining unit a declared resource_user_neutral() is no longer "
        "applied unverified: call_effect verifies it against the body (DECLARED-NEUTRAL-UNVERIFIED-AT-CALL, "
        "discharged); the slice at 046accf6 is leaf-identical. Open as before: every cross-unit "
        "application, and every other unchecked declaration."
    ),
}


def main() -> int:
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    entries = matrix["named_followups"]
    ids = [entry["id"] for entry in entries]
    before = len(entries)
    new = json.loads((REPORT / "followups.json").read_text(encoding="utf-8"))
    for entry in new:
        if entry["id"] in ids:
            raise SystemExit(f"duplicate follow-up id {entry['id']}")
    missing = sorted(set(RETIRED) - set(ids))
    if missing:
        raise SystemExit(f"not registered on this base: {missing}")
    retired = [entry for entry in entries if entry["id"] in RETIRED]
    (REPORT / "retired-followups.json").write_text(
        json.dumps(
            [{"entry": e, "discharged_by": CARD, "ground": RETIRED[e["id"]]} for e in retired],
            indent=2,
            ensure_ascii=False,
        )
        + "\n",
        encoding="utf-8",
    )
    entries = [entry for entry in entries if entry["id"] not in RETIRED]
    for entry in entries:
        text = AMEND.get(entry["id"])
        if text is not None and MARK not in entry["why"]:
            entry["why"] += text
    unamended = sorted(set(AMEND) - {entry["id"] for entry in entries})
    if unamended:
        raise SystemExit(f"cannot amend unregistered {unamended}")
    entries.extend(new)
    matrix["named_followups"] = entries
    MATRIX.write_text(json.dumps(matrix, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(
        f"named_followups {before} -> {len(entries)} (+{len(new)} -{len(retired)}); "
        f"amended {len(AMEND)}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
