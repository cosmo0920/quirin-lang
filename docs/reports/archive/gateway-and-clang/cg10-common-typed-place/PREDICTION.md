# CG10-COMMON-TYPED-PLACE: frozen prediction

Frozen before any implementation, on base `e182d403` (= local `main`), while
the BEFORE arbiter was running. Figures are against the corpus arbiter
baseline `ae17152b…` (526 Safe / 10 provisional Unsafe / 4,298 Unknown).

## The inherited list

The function list, per-function blockers and predicted verdicts are CG00's,
unchanged: [`../cg00-clang-gap-baseline/predictions/CG10.md`](../cg00-clang-gap-baseline/predictions/CG10.md)
(sha256 `717600ee9f97e22de74f0a6229cbb6ab2920147dd3c9a49dd70ff70d36f48297`):
491 functions this card could free alone, 293 of them with no `unknown_alias`
leaf (predicted Safe) and 198 with one (predicted Unknown); 0 provisional
Unsafe. Of the 293, 18 carry an I04 leaf and 7 carry only I04 leaves.

## Amendments, with reasons

CG00's list is an upper bound: an M01 leaf is the *residue* of every typed
position route, an expressible Clang path with no other attributed cause,
not a measured refusal of the tracked-root premise. The card's scope, as
briefed, admits less than the residue:

1. **Tracked root, one or two pointer hops.** The new route admits a mixed
   `->`/`.`/`[]`/`*` path on a root the closure does not prove never bound
   only when the pointers it reads through are the root itself (case A:
   `R->in.x`, `R->arr[i]`, `R[i].x`, `(*R).in.x`) or the root and the direct
   member place `R->F` (case B: `R->items[i].v`, `*R->p`, `R->F->in.x`). Both
   carry the existing chain obligations (`chain_resolves`): the root must be
   exactly one known, non-null, live identity (or never bound), and `R->F`
   must carry the admitted store's evidence. A pointer loaded from any deeper
   place (`R->in.p->x`, `R->slots[i]->x`, `R[i]->x`, `(*R)->x`) has no
   evidence in the projection and stays refused. So do a record-object root
   (`s.in.x`) the closure does not prove never bound, an array root, a
   `static` or file-scope root and any root whose spelling names two
   declarations.
2. **A bound but untracked root stays refused at flow time.** A root bound to
   a target-free alias (an opaque read, a borrowed alias, a derived result)
   fails `chain_resolves`'s root reading exactly as a direct `R->F` chain
   does today; this card does not change that reading. Many M01 leaves are
   expected to be such roots.
3. **Pointer-valued returns and actuals keep their premise.** Rule 10k (2)
   forbids relaxing the pointer-return premise; a pointer-valued actual keeps
   its never-bound premise too (the brief's custody-position limit). Only
   scalar reads and non-pointer stores gain the tracked root, in statements,
   condition operands, scalar returns and scalar actuals.
4. **I04 by occurrence identity.** Two candidates with the same tokens on one
   line are told apart by the occurrence of the gateway's own token run in
   the function's token stream, matched to the Clang candidates in source
   order only when both counts are equal and every candidate begins at a
   distinct offset. A synthetic (desugared) token run has no occurrence and
   keeps the unique-match rule.
5. **W44-AGGREGATE-PREMISE3-MEMBER-NAME** is a refusing-direction repair
   (`acquired_bindings` skips a member name). It can only withdraw Safe.

## Predicted outcome

| Measure | Prediction |
|---|---|
| BEFORE reproduces `ae17152b…` byte-identically | 97% |
| Unknown → Safe, corpus | **60** (band 15–200); upper bound 293 strict / 491 with alias |
| of those, in CG00's 293 list | ≥ 70% |
| Unknown functions whose unsupported-leaf count drops but stay Unknown | several hundred (band 150–1,200) |
| Safe → non-Safe, corpus | 0–3, each a named correction (premise-3 repair or a tracked-root obligation now checked) |
| provisional Unsafe added | 0 (band 0–3); every added leak or violation classified by reading |
| `unknown_alias` leaves added | hundreds (a tracked-root place now admitted but its root or `R->F` fails the flow reading) |
| I04 leaves removed | 30–60% of the 873 IND + 292 COND on shared lines |
| system slice (`046accf6`) | 0–5 functions move, none Safe → non-Safe |
| tail-pause slice (`90978ac6`) | 0–3 functions move |

Confidence in the point estimate is low: the M01 residue has not been split
by actual refusal cause. The AFTER parity stream records each tracked-root
place decision (`position: "typed-place"`) so the split is measured, not
inferred.
