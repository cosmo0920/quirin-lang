# SCALAR-ROUTE-EXIT-EDGE (wave 37, first arbiter)

**Status: in progress.** Skeleton committed first per the card's hard scope; every
section below is filled from measurement before this card reports.

## 1. The finding under investigation

`refine_pointer_null` (`crates/qcore-c-gateway/src/legacy_c.rs`) prunes a loop
header successor through `PointerRefinement::Infeasible` — the only place in the
file where a successor is not propagated. Wave 35's
`C-GATEWAY-CONSTANT-DO-WHILE` pinned it conservative on four probed shapes, but
every probed row spells the header positively, so only the **back edge** was
measured. Residuals: `SCALAR-ROUTE-EXIT-EDGE`,
`SCALAR-ROUTE-BODY-WRITE-UNEXERCISED`, `SCALAR-ROUTE-SECOND-ARM`,
`SCALAR-ROUTE-POISON-DEPENDENCE`, `SCALAR-ROUTE-ZERO-SPELLINGS`,
`CONSTANT-DO-WHILE-SCALAR-ROUTE-UNOWNED`.

## 2. Polarity/arm census over the pinned corpus

TBD.

## 3. The pruning-disabled verdict diff

TBD.

## 4. Defend or restrict, and why

TBD.

## 5. Lean

TBD.

## 6. Mutations

TBD.

## 7. The corpus re-pin

TBD.

## 8. Gate

TBD.

## 9. Named follow-ons

TBD.

## 10. For the integrator

TBD.
