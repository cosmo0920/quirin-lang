#!/usr/bin/env python3
"""Add card C-GATEWAY-RETURN-POSITION's untracked residuals to the follow-on ledger.

`named_followups` is `hand_maintained` in the matrix's own `section_ownership`
("waves add entries to the committed matrix and this script carries them"), so
this appends rather than regenerates — constraint 25 forbids whole-file
regeneration clobbering later reconciliations.

Only the three the sweep reported as reaching no tracker are added here; the
other three residuals this card introduces are already tracked, two in standing
constraint 5b and one in `AGENTS.md`'s merged-gate row.
"""
import json
import collections

PATH = 'docs/reviews/r0_3/acceptance_matrix.json'

NEW = [
    {
        "id": "RETURN-POSITION-SUBSHAPE-B",
        "title": "`return CALL(args)` with a non-neutral contract or summary is withdrawn, with its measurement published",
        "why": "Card C-GATEWAY-RETURN-POSITION (wave 33) offered two sub-shapes "
               "and landed one. Sub-shape B - apply a callee's declared "
               "contract or validated W4 summary at a return position - is "
               "WITHDRAWN, and the grounds are measured over the real "
               "population rather than sampled. Of the 335 `return CALL(...)` "
               "leaves among the 511 functions the baseline blocks on the "
               "return position alone, only 12 (3.6%) name a callee with a "
               "declared contract; 94 name a callee the corpus already reports "
               "as uncontracted elsewhere and 227 name one declared nowhere, "
               "so for 323 of 335 clearing the return refusal would merely "
               "EXCHANGE it for a `call to uncontracted function` leaf and "
               "decide nothing. A cross-cut by whether W4 could derive a "
               "summary at all puts 236 of 335 (70.5%) in the unit's own .c "
               "and 99 outside it. Note also that a call whose contract is "
               "all-Borrowed with result None is ALREADY admitted today "
               "through admitted_neutral_call, so B's real content is "
               "narrower than it sounds. The soundness cost is decisive: "
               "applying a callee's Consumed/Released parameter mode at a "
               "return position would DISCHARGE a live obligation of the "
               "caller at that exit - invariant 1's forbidden direction - on a "
               "route whose observe_exit has no arm for a call result at all, "
               "constructing ReturnedValue only from EscapeIfOwned/Escape. "
               "Discharge: a card that lands a return-position effectful "
               "instruction together with its ReturnedValue construction, the "
               "matching summary validation and the certificate recheck, and "
               "that states up front how many functions it DECIDES rather than "
               "how many refusals it converts. Its decisive population today "
               "is 12.",
        "source": "docs/reports/c-gateway-return-position/README.md",
        "recorded_by": "C-GATEWAY-RETURN-POSITION",
    },
    {
        "id": "RETURN-POSITION-TYPED-MEMBER-REACH",
        "title": "Most pure field-chain returns are refused by member typing, and the cost is unmeasured per premise",
        "why": "Card C-GATEWAY-RETURN-POSITION (wave 33) admitted a "
               "pointer-valued bounded typed access chain in a `return` and "
               "moved 33 of the 511 return-only-blocked functions (6.5%). The "
               "natural candidate population is the 75 pure field-chain "
               "returns the card's whole-population census measured, of which "
               "only 31 are admitted. The remainder fail on F4 premises the "
               "card must not weaken: the member is not typed EXACTLY "
               "`struct TAG *` (a `char *`, a `cfl_sds_t`, a typedef the "
               "reader will not resolve, a `void *`), the record is "
               "incomplete, a union or has more than one definition in the "
               "unit, or the base is bound because it is named in some call's "
               "argument list elsewhere in the body. Discharge: measure, PER "
               "REFUSED PREMISE, how many functions a widened member typing "
               "would decide - PREDICTED BEFORE IT IS ATTEMPTED, as "
               "AOM-POISON-CONDITION-FRAME's discharge condition requires - "
               "and only then decide whether any premise can be widened "
               "without losing the base-premise argument the wave-33 "
               "admission rests on. That argument is that an admitted chain "
               "names no place the projection binds; a premise that weakens "
               "the base is not a typing change but a soundness change, and "
               "must land RETURN-POSITION-ESCAPE-UNREACHABLE's test in the "
               "same card.",
        "source": "docs/reports/c-gateway-return-position/README.md",
        "recorded_by": "C-GATEWAY-RETURN-POSITION",
    },
    {
        "id": "RETURN-POSITION-CONDITIONAL",
        "title": "Conditional-expression returns are unmeasured as a population",
        "why": "Card C-GATEWAY-RETURN-POSITION (wave 33) measured the whole "
               "511-function return-only-blocked population by returned-"
               "expression shape and found 28 leaves (5.2%) are conditional "
               "expressions over field reads - `return error ? error->code : "
               "RD_KAFKA_RESP_ERR_NO_ERROR;` and the like. These refuse today "
               "for their OPERANDS rather than for the conditional operator "
               "itself: `?` and `:` are admitted operand punctuation, so a "
               "conditional all of whose operands are F4 chains or scalars is "
               "already admitted by the wave-33 rule, and the card's own "
               "re-pinned test drives exactly that shape. Discharge: measure "
               "how many of the 28 would be admitted if every operand were an "
               "F4 chain, which separates 'refused for the conditional' from "
               "'refused for an untyped member' and tells a later card "
               "whether there is a population here at all. This is a "
               "measurement request, not a proposed widening.",
        "source": "docs/reports/c-gateway-return-position/README.md",
        "recorded_by": "C-GATEWAY-RETURN-POSITION",
    },
]


def main():
    with open(PATH) as fh:
        doc = json.load(fh, object_pairs_hook=collections.OrderedDict)
    existing = {e['id'] for e in doc['named_followups']}
    added = [e for e in NEW if e['id'] not in existing]
    doc['named_followups'].extend(added)
    doc['dimensions']['named_followups'] = len(doc['named_followups'])
    with open(PATH, 'w') as fh:
        json.dump(doc, fh, indent=1, ensure_ascii=False)
        fh.write('\n')
    print('added %d; named_followups now %d'
          % (len(added), len(doc['named_followups'])))


if __name__ == '__main__':
    main()
