# Landing batch C (integrate/r76): leak witness, keep-or-deliver, callback tables

USER DECISION 2026-11-26: "Three batches". Batch C lands three cards in one
integrate branch with one full landing gate:

| Card | Report | Rule |
|---|---|---|
| SETTLED-LEAK-JOINED-GENUINE-WITHDRAWN | [report](../settled-leak-joined-genuine-withdrawn/README.md) | 84 |
| OUT-SLOT-CONDITIONAL-DELIVERY | [report](../out-slot-conditional-delivery/README.md) | 85 |
| CALLBACK-TABLE-MEMBER-PATH-STATES | [report](../callback-table-member-path-states/README.md) | 13v, 74p amended |

Each card's own report holds its evidence; this page records the integration.
The landing gate record is [integration/gate.json](integration/gate.json).

## Integration

* Main reorganised the contract spec after CALLBACK-TABLE's base; its "At a
  table dispatch" paragraph is placed in C04.19.6's composition section.
* SETTLED-LEAK's `Replay::step` mirrors `analyze_cfg_with_entry_collected`'s
  work-list loop; the cards and main rounds merged since its base (rules 79,
  81, 38k and this batch's rule 85) are checked by its `debug_assertions`
  equality of the uncut replay and the settled states in this batch's gate.
* CALL-WITH-DEREF-ACTUAL-READ-AS-DECLARATION, registered here by
  OUT-SLOT-CONDITIONAL-DELIVERY, was discharged by the CALL-WITH-DEREF card
  landed in batch B (rule 89) and is retired here.
