# W42-INDIRECT-ACCESS prediction — published before AFTER

Published from the clean B01–B04 integration base `f7b010de`, after the root
arbiter reproduced that baseline byte-for-byte and before this branch ran a
changed corpus audit.  The baseline is
`/home/cosmo/Documents/GitHub/qcore-c/private/w42-priority/baseline/`.

The implementation admits one exact destination shape:
`BASE -> FIELD = RHS`, where the declaration reader resolves `BASE` to a
single-star record pointer, `FIELD` to a scalar member of a complete non-union
record, and `RHS` to a scalar-only, access-free, mutation-free expression.  It
lowers an explicit `ScalarMemberStore`, never a `NoOp`.  Transfer requires one
known, non-null, live target with no nullable or unresolved marker and retains
the existing same-type may-alias obligation.  Pointer fields, chains,
subscripts, dereferences, calls, compound assignments and RHS mutations stay
refused.

## Prediction

* **Leaf movement: 349 indirect-access leaves removed, 0 added, 0 exchanged.**
  This is the pre-card census population
  `Formal · may-bind · -> · Scalar · AssignmentDestination` at
  `docs/reports/c-gateway-indirect-census-2/README.md` §6.2.  It is an
  intentionally testable forecast rather than a claim that the historical
  census proves every leaf meets the stricter exact-RHS and flow premises.
* **Function movement: 1 Unknown to Safe**, predicted as
  `ctraces/ctr_span.c::ctr_span_start_ts`, the census's sole cleared function
  for this shape.  **No unit movement** is predicted.
* **Retirements: none.**  The card admits a bounded scalar-store subfamily, not
  the whole 17,775-leaf indirect-access family, so it does not meet the broad
  W42 tracker row's complete discharge condition.

The AFTER may legitimately show fewer removals or new `unknown_alias` rows:
the historical census classified declarations and syntax, while the
implementation additionally rejects released, escaped, nullable, ambiguous or
unresolved bases at the instruction.  Any miss will remain recorded here and
will be attributed in the card report rather than rewriting this forecast.
