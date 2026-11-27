#!/usr/bin/env python3
"""FB-LOOP-CHAIN-ACCESS-IDENTITY-BLOCKERS: the ledger update (idempotent).

* Narrows FB-LOOP-CHAIN-ACCESS-IDENTITY-BLOCKERS to the residual blockers,
  sized by class and kind from artifacts/after-classes-{system,slice}.json
  (class (a) is cleared; every remaining leaf is named with the rule that keeps
  it). The entry stays open: its purpose, a lifetime model showing movement on
  these rows, is not yet met.
* Amends RECORD-BODY-TOTAL-OR-REJECT and W44-CHAIN-INCOMPLETE-RECORD: the
  function-pointer part of the whole-record refusal is repaired; the `{`, `:`,
  `=` and other parenthesised shapes remain.
* Registers LEXICAL-ROUTES-PREEMPT-TYPED-FALLBACKS: a lexical route whose flow
  obligation refuses an unsettled base (rule 5g's store, the W44 chain reading
  of a condition) runs before the Clang-typed fallback (rule 10k) that decides
  the same statement on its own premise, so a reader repair that widens the
  lexical facts can hand a decided statement to a refusing route.

Writes the matrix with indent=1, ensure_ascii=False and
dimensions.named_followups = the list length."""
import collections
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
REPORT = os.path.dirname(HERE)
REPO = os.path.abspath(os.path.join(HERE, "../../../.."))
MATRIX = os.path.join(REPO, "docs/reviews/r0_3/acceptance_matrix.json")
SOURCE = "docs/reports/fb-loop-chain-blockers/README.md"
CARD = "FB-LOOP-CHAIN-ACCESS-IDENTITY-BLOCKERS"
MARK = "AMENDED (" + CARD


def residual():
    parts = []
    for route in ("system", "slice"):
        with open(os.path.join(REPORT, "artifacts", f"after-classes-{route}.json"), encoding="utf-8") as f:
            data = json.load(f)
        kinds = collections.defaultdict(lambda: [set(), 0, 0])
        for leaf in data["leaves"]:
            kind = leaf["kind"].replace("cascade:", "")
            entry = kinds[(data["kinds"][kind]["class"], kind)]
            entry[0].add((leaf["unit_index"], leaf["function"]))
            entry[1 if leaf["leaf"] == "access" else 2] += 1
        by_class = collections.Counter()
        for (cls, _), (_, a, i) in kinds.items():
            by_class[(cls, "access")] += a
            by_class[(cls, "identity")] += i
        classes = ", ".join(f"({c}) {by_class[(c, 'access')]}/{by_class[(c, 'identity')]}"
                            for c in "abcd")
        detail = "; ".join(f"{kind} ({cls}) {len(rows)} rows {a}/{i}"
                           for (cls, kind), (rows, a, i) in sorted(kinds.items()))
        parts.append(f"{route}: access/identity by class {classes}; by kind: {detail}")
    return " | ".join(parts)


def main():
    with open(MATRIX, encoding="utf-8") as f:
        matrix = json.load(f)
    entries = matrix["named_followups"]
    by_id = {e["id"]: e for e in entries}

    entry = by_id[CARD]
    if MARK not in entry["why"]:
        entry["title"] = ("Residual access-gate and identity refusals on Fluent Bit's event-loop "
                          "registration and teardown chains, each named with the rule that keeps it")
        entry["why"] = entry["why"].rstrip() + " " + MARK + ", report " + SOURCE + "): " + (
            "census of every access and identity leaf of the chain rows by class (README section 2; "
            "artifacts/before-classes-*.json, after-classes-*.json). Class (a), an access an existing "
            "reading handles elsewhere, was entirely the lexical declaration reader's total-or-reject "
            "refusal of a function-pointer declarator (records flb_input_collector, flb_sched_timer, "
            "mk_event, flb_input_plugin; formals of collector_create and flb_sched_timer_cb_create) and is "
            "cleared: read_record_body and read_parameters read past exactly that shape, rule 5g's "
            "flow-refusing store keeps its domain of fully read declarations, QCore.FunctionPointerDeclarators. "
            "Measured at main b157c9cba against the tip (README section 4): corpus 939 -> 941 Safe "
            "(mk_sched_conn_pending_output, mk_sched_worker_cb_free), system 95 -> 97 "
            "(flb_sched_timer_cb_disable, both configurations), slice unchanged, null property unchanged on "
            "every route, no Safe -> non-Safe. collector_start now refuses only at rule 12t "
            "(retention-undecided at both registrations). Residual at the tip, every leaf named: "
            + residual() + ". Classes (b) (declarations of Unknown Fluent Bit functions or the pthread "
            "mutex pair, none mechanical), (c) (pointer stores into members, member releases and member "
            "descriptors, logging macros, callback-member calls, formals the body may bind, the loop "
            "destroy at a path: RETENTION-PATH-LOOP-DESTROY-ADMISSION) and (d) (identities from list "
            "iteration, get_collector and the loop's ready list) stay. DISCHARGE: the residual kinds on "
            "these rows cleared by their own cards, re-measured on the system and slice routes with every "
            "moved row read, or a recorded decision that the lifetime model proceeds on the rows that "
            "reach rule 12t.")
        entry["recorded_by"] = entry.get("recorded_by", CARD)

    record = by_id["RECORD-BODY-TOTAL-OR-REJECT"]
    if MARK not in record["why"]:
        record["why"] = record["why"].rstrip() + " " + MARK + ", report " + SOURCE + "): the "
        record["why"] += (
            "function-pointer part is repaired. read_record_body reads a body whose only unparsed member "
            "declarations are exactly SPECIFIERS ( * NAME ) ( PARAMETERS ); that member joins no class and "
            "every other member reads as without it (QCore.FunctionPointerDeclarators). Measured on the "
            "pinned corpus at main b157c9cba: +2 Safe, 104 Unknown rows trade leaves, no Safe -> non-Safe "
            "once rule 5g's store keeps its domain of fully read records (without it cio_options_init lost "
            "its Safe). Still refused whole: '{' (nested aggregates), ':' (bitfields), '=' and every other "
            "parenthesised declarator (arrays of function pointers, attributes). DISCHARGE (narrowed): a "
            "per-member reader for those shapes, measured with its own re-pin, or a recorded decision.")

    chain = by_id["W44-CHAIN-INCOMPLETE-RECORD"]
    if MARK not in chain["why"]:
        chain["why"] = chain["why"].rstrip() + " " + MARK + ", report " + SOURCE + "): a chain " + (
            "through a record whose only unparsed members are function-pointer declarators now types its "
            "links (flb_input_collector, flb_sched_timer, mk_event, flb_input_plugin on the Fluent Bit "
            "routes); bit-field, nested-aggregate and two-definition records are unchanged and keep this "
            "entry's DISCHARGE.")

    new_id = "LEXICAL-ROUTES-PREEMPT-TYPED-FALLBACKS"
    if new_id not in by_id:
        entries.append({
            "id": new_id,
            "title": "A lexical route whose flow check refuses an unsettled base runs before the Clang-typed fallback that decides the same statement",
            "why": (
                "Found by " + CARD + " (README section 4.3). Rule 5g lowers `B->F = scalar` with "
                "may_bind(B) to ScalarMemberStore, whose flow check refuses unless B resolves to one settled "
                "live identity; W43's typed scalar statement and the W44 chain reading of a condition (such "
                "as `ins->p->flags & X`) carry per-base obligations of the same kind; all run before the "
                "statement's or operand's refusal reaches the "
                "Clang-typed reading (rule 10k), which decides the same text on its own premise (the root "
                "never bound by the custody closure). Where a lexical reader cannot type the record the "
                "fallback decides; where it can, the lexical route refuses. Measured when the card widened the "
                "reader: cio_options_init (memset(options, ...) then scalar stores) went Safe -> Unknown "
                "until rule 5g kept its domain of fully read declarations; on the slice "
                "flb_input_net_property_check gains two `unresolved resource identity ins` leaves at its "
                "conditions (W44) and flb_input_set_test, flb_output_set_http_test and "
                "flb_output_set_test_with_ctx_callback one each at a scalar store through a looked-up "
                "instance (W43, reachable once their function-pointer formals no longer refuse the "
                "function); all four rows Unknown before and after. The same precedence holds today on every fully read "
                "record (e.g. flb_engine_shutdown's `config->is_running = FLB_FALSE`). DISCHARGE: a decided "
                "precedence (the lexical route's flow refusal falls back to the typed reading where that "
                "reading's own premise holds, or a stated reason it must not), with Lean parity and a measured "
                "re-pin, every moved row read."),
            "source": SOURCE,
            "recorded_by": CARD,
        })
    matrix["dimensions"]["named_followups"] = len(entries)
    with open(MATRIX, "w", encoding="utf-8") as f:
        json.dump(matrix, f, indent=1, ensure_ascii=False)
        f.write("\n")
    print(len(entries))


if __name__ == "__main__":
    main()
