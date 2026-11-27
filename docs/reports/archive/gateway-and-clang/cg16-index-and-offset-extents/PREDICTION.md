# CG16-INDEX-AND-OFFSET-EXTENTS: frozen prediction

Frozen before the route-G bounds consumer and the audit selection were written,
after the core decisions (`qcore_core::index_extent`), their Lean model
(`QCore.IndexExtent`) and the bounds contract loader
(`qcore_contract::bounds_contract`), none of which any custody route reads.
Base: main `b6a3dc42`. The BEFORE run was in flight when this was written; its
digests are recorded in the report, not here.

## Custody (the existing property)

The bounds property is selected explicitly (`QCORE_BOUNDS_PROPERTY` on the audit
routes; a separate API in the gateway). No custody reader consults it.

| Route | Baseline (main `b6a3dc42`) | Predicted AFTER |
|---|---|---|
| Corpus arbiter | 629 / 10 / 4,195, `3d53a5d0…` | **byte-identical** `resource-diagnostics.json` (0 moves; band 0) |
| System | 44 / 2 / 188, `79404fed…` | byte-identical |
| Tail-pause slice | 29 / 1 / 239 / 1 Pending, `93cc660c…` | byte-identical |

CG00's M14–M17 custody controls keep their custody verdicts; each gains a
bounds row in a new test instead of being edited.

## Bounds (the new property, reported separately)

Denominator: every function the corpus arbiter projects (Unknown and Pending
stay in it). Predicted on the corpus with no bounds contract selected:

| Bounds verdict | Predicted | Band |
|---|---|---|
| Safe | ~10% of projected functions (no subscript, dereference, arrow, pointer arithmetic or comparison, and no call passing a pointer) | 5–20% |
| Pending | 0 (no bounds declaration exists in the corpus) | 0 |
| Unsafe | 0 | 0–2; each read and classified |
| Unknown | the rest, dominated by `bounds-missing-extent` (pointer parameters, `->`) and calls passing a pointer to a callee with no bounds contract | — |

Combined (custody and bounds both requested): Safe only where both are Safe,
predicted 50–250 functions (≤ custody's 629); the combined row is never Safe
where bounds is not.

## Absorbed follow-ups

`W43-INDEX-TRAVERSAL-STEP-COMPUTED` and `W43-INDEX-TRAVERSAL-COMPUTED-POINTER-ACCESS`
are custody-route admissions (refusals of the custody gateway). This card adds
the bounds obligation for those forms, not their custody admission, so both are
predicted to stay registered (amended), with sizes re-measured from the BEFORE
custody artifacts. `W46-ROUTING-QUERY-EXTENTS` is predicted to stay registered:
the vocabulary can state a readable extent, but no Fluent Bit bounds
declaration is written or reviewed by this card.
