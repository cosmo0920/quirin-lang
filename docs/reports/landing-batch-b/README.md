# Landing batch B (integrate/r75): formal-name readings and the Windows types notice

USER DECISION 2026-11-26: "Three batches". Batch B lands four changes in one
integrate branch with one full landing gate:

| Change | Report | Rule |
|---|---|---|
| WINDOWS-TYPES-NOTICE (documentation) | [report](../windows-types-notice/README.md) | none |
| NAME-KEYED-HOLDER-SHADOWING | [report](../name-keyed-holder-shadowing/README.md) | 86 |
| JUMPED-SHADOW-FORMAL-READINGS | [report](../jumped-shadow-formal-readings/README.md) | 86 (extended) |
| CALL-WITH-DEREF-ACTUAL-READ-AS-DECLARATION | [report](../call-with-deref-actual/README.md) | 89 |

Each card's own report holds its evidence; this page records the integration.
The landing gate record is [integration/gate.json](integration/gate.json).

## Integration

* JUMPED-SHADOW-FORMAL-READINGS was built on NAME-KEYED-HOLDER-SHADOWING's
  tip; its retire applies after that card's registration.
* JUMPED-SHADOW and CALL-WITH-DEREF both hook the shadow guard in
  `crates/qcore-c-gateway/src/legacy_classify.rs`; the merge kept rule 89's
  `paren_star_declarator` alternative and rule 86's `shadow_guard_refused`
  record.
* CALL-WITH-DEREF discharges CALL-WITH-DEREF-ACTUAL-READ-AS-DECLARATION,
  which OUT-SLOT-CONDITIONAL-DELIVERY registers in landing batch C; batch C
  retires it.
* BASE-GATEWAY-FRONTEND-LAYERING, registered by CALL-WITH-DEREF for main
  110a69c8e's layering and lock defects, is retired here: main 6821fddee
  ("Keep qcore-c-gateway off qcore-frontend") removed the dependency before
  this landing, and this batch's full gate runs the layering test and the
  provider gate on the merged tree.
