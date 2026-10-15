#!/usr/bin/env python3
"""edit_records.py -- LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER: STATUS rows
(projected functions, contract dependencies, null property, Lean toolchain,
registered follow-ups) from the report's primary evidence, and the increment
paragraph, newest first, in wave-48-part8.md. Idempotent."""
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
STATUS = REPO / "docs/STATUS.md"
PART = REPO / "docs/development/increments/wave-48-part8.md"
TAG = "LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER"
LINK = "[report](reports/librdkafka-plugin-premise-remainder/README.md)"

PROJECTED = ("| Projected functions | **938 Safe / 9 provisional Unsafe / 3,887 Unknown**, 4,834 total "
             f"({TAG}, the plugin premise on the scoped member facts, the crash path and the model contract, "
             "rules 14x, 14z and 13h amended: combined arbiter main `b2fbde2c5` against the rebased tip, corpus "
             "`fc645e9b…` → `fc645e9b…` **byte-identical** (on base `711287771` likewise `82214412…`); system and "
             "slice identical apart from the contract-source digests; null rows identical apart from the premise "
             f"fields {LINK}); before it: ")
DEPENDENCIES = ("| Contract dependencies per verdict (rules 13f–13k) | Since " + TAG + " (corpus binary "
                "`b11509e5…`): **all 69 corpus rows resting on `rd_kafka_crash` or `rd_kafka_dump0` list "
                "`rk_no_plugin_library`** (2 Safe and 43 Unknown gain it); every row's dependency field, the "
                "counts and the Safe roll-up identical. Before it: ")
NULLROW = ("**Premised member facts** (" + TAG + ", rules 14x/14z): the 67 librdkafka "
           "`rd_kafka_broker_s.rkb_rk` stable/scope declarations name `rk_no_plugin_library`; all 339 corpus "
           "null rows resting on them (75 Safe, 262 Unknown, 2 provisional Unsafe) list it, no other row "
           "does, and every null verdict is unchanged " + LINK + ". ")
LEAN = ("| Lean toolchain | **`leanprover/lean4:v4.35.0`** (from 4.34.0, 2026-10-15): build and all **324** "
        "audits pass")
LEAN_NEW = ("| Lean toolchain | **`leanprover/lean4:v4.35.0`** (from 4.34.0, 2026-10-15): **328** audits since "
            + TAG + " (`QCore.NullMemberPremise`, on the tip rebased onto `b2fbde2c5`); at the pin move build and all **324** audits pass")
FOLLOWUPS = ("| Registered follow-ups | **785** (wave 48: " + TAG + " retires "
             "`LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED` and `LIBRDKAFKA-CRASH-DUMP-PREMISE-UNSTATED` and registers "
             "`LIBRDKAFKA-NONNULL-MEMBER-PREMISE-UNADMITTED`); before it ")

PARAGRAPH = (
    f"**`{TAG}` (2026-10-16, base `711287771`, rebased onto `b2fbde2c5`; census and prediction `7472690aa`, committed before any "
    "analyser, loader, contract or Lean change).** The user's decisions of 2026-10-15 (\"Only "
    "nonnull_member* need it\", \"Yes, list it\", \"Add premise support\") close the plugin premise's "
    "remainder with the existing `resource_premise(NAME, \"TEXT\")` spelling and no new word. The premise "
    "clause's shape is now checked by the contract format at parse in every module (`frontend.rs` "
    "`premise_clause`), so a Quirin model contract can state one (`contract_loader::module_premises`), "
    "total-or-reject. Bounds v1 admits it only beside `nonnull_member_stable` / `nonnull_member_scope` "
    "(`premise-beside-member` elsewhere). Every null row whose dependency set names a premised fact "
    "(`stable:TAG.FIELD`, or a scope function) lists it in a `premises` field and as `premise:NAME`, after "
    "the fold's travel too; the fold's records list it; nothing decides on it. A census over the "
    "preprocessed units at both revisions (`member_reach.py`, rule: a store to the member reachable from a "
    "function a plugin's callback may call, or the body reaching an rk_conf or interceptor call) premises "
    "all 67 `rd_kafka_broker_s.rkb_rk` declarations (1 stable, 66 scope); `rd_kafka_crash`, "
    "`rd_kafka_dump0` and the model contract `rd_kafka_q_destroy_final` name it too; re-pinned at both "
    "revisions, 70 clause pins +1. Measured (BEFORE `352276f2` from an exported base tree, AFTER "
    "`b11509e5`): corpus custody report byte-identical (`82214412…`), system 93/4/351 and slice "
    "52/1/347/4 Pending unchanged apart from contract-source digests, every null row identical apart from "
    "the premise fields and the bounds digest; 339 corpus null rows (75 Safe, 262 Unknown, 2 provisional "
    "Unsafe) list the premise and no other row does; all 69 custody rows on the crash path list it (2 Safe "
    "and 43 Unknown gain it). Lean `QCore.NullMemberPremise` (328 audits on the rebased tip). Rules 14x, 14z and 13h "
    "amended in place. Ledger 786 → 785: both premise entries retired, "
    "`LIBRDKAFKA-NONNULL-MEMBER-PREMISE-UNADMITTED` registered (rule 14t's five plain `nonnull_member` "
    "clauses meet the rule but the decision admits the premise only beside the scoped forms). "
    "Gate g3 (on the tip rebased onto `635669fdb`): standing and provider gate 0, 8,112 passed, 326 Lean "
    "audits, 374 Python OK after the matrix regeneration (g1 discarded for disk exhaustion; g2 found the "
    "memory.qv parity fixture, now updated). After the rebase onto `b2fbde2c5` (HANDOFF-CANDIDATE-OWN-CALLBACKS): "
    "a targeted check and the combined arbiter (main against the tip): corpus 938/9/3,887 byte-identical "
    "(`fc645e9b…`), null tallies unchanged on all three routes, the two new Safe rows (`rd_kafka_event_destroy`, "
    "`ut_op_destroy_free`) still list their premises, the 339 null rows and 69 crash-path rows list "
    "`rk_no_plugin_library`. "
    "[Report](../../reports/librdkafka-plugin-premise-remainder/README.md).\n\n"
)


def prepend_in_row(text, row_start, prefix):
    start = text.index(row_start)
    if prefix in text:
        return text
    return text[:start] + prefix + text[start + len(row_start):]


def main():
    text = STATUS.read_text()
    if TAG not in text:
        text = text.replace("| Projected functions | ", PROJECTED, 1)
        text = text.replace("| Contract dependencies per verdict (rules 13f–13k) | Since ", DEPENDENCIES + "Since ", 1)
        null_start = "| Null-dereference property (separate from custody; rules 14k–14z, 15q–15u, C09.3) | "
        assert text.count(null_start) == 1
        text = text.replace(null_start, null_start + NULLROW, 1)
        assert text.count(LEAN) == 1
        text = text.replace(LEAN, LEAN_NEW, 1)
        assert text.count("| Registered follow-ups | ") == 1
        text = text.replace("| Registered follow-ups | ", FOLLOWUPS, 1)
        STATUS.write_text(text)
        print("STATUS updated")
    part = PART.read_text()
    if f"`{TAG}`" not in part:
        marker = "**`HANDOFF-CANDIDATE-OWN-CALLBACKS` (2026-10-15"
        assert part.count(marker) == 1
        part = part.replace(marker, PARAGRAPH + marker, 1)
        PART.write_text(part)
        print("increment written")


main()
