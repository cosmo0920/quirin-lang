# CG16-INDEX-AND-OFFSET-EXTENTS: reviewed index and extent contracts

Status: **partly implemented** (wave 48 round 9; the
[report](../../reports/cg16-index-and-offset-extents/README.md) lists what
landed and the registered residuals). Originally a candidate card, not
dispatched.
Expanded on 2026-10-01 from the user's decision to express existing-C access
bounds through user-authored, human-reviewed contracts. This is the detailed
brief for the existing CG16 tracker, not a second card or a completed feature.
Authority: [contract specification C09.1](../../QCORE_CONTRACT_SPEC_r0_3.md#c091-reviewed-index-and-extent-contracts-planned).
Queue: [phase 3](README.md#phase-3-connections-to-existing-core-checkers-r).
Effort: High. Route: G with an explicitly selected bounds consumer; a core
model alone or route T admission alone does not complete this card.

## 1. Objective and baseline

Add a separately reported index/access-bounds property to existing-C analysis.
Derive fixed-array bounds from source, accept external extent/index relations
only through the reviewed-declaration boundary, check each use, and compose
the result with custody only when both properties are requested. The flagship
control is a reachable `arr[7]` for `int arr[4]`: it remains custody-only Safe
where it is today, but its bounds result is a validated violation. A user's
larger extent assertion must not override the source-established extent.

Brief baseline: `d4df096fd310019628cd7f21a5b68e8f1bd58daf`. At dispatch, record
the actual base revision and generate/read its task-context union; initialize
submodules on a fresh checkout. Reproduce the existing controls in
`crates/qcore-c-gateway/tests/clang_gap_phase3.rs` (M14–M17) before changing them.
These are **custody limits**, not necessarily Unknown refusals. CG00's
historical attribution/ranking is not a prediction of a new property's
coverage. Read current STATUS and the latest integration artifacts before
freezing `docs/reports/cg16-index-and-offset-extents/PREDICTION.md`.

## 2. Dependencies and ownership

- CG10's typed places and CG03's common call instantiation are inputs; use the
  current landed implementations and their remaining identity restrictions.
- CG20 owns general C numeric/range semantics. This card can implement checked
  literals, source-derived fixed extents and a precisely bounded range subset
  first, but unsupported symbolic ranges must remain Unknown. Any prerequisite
  subset brought into CG16 needs its own Rust/Lean semantics and explicit
  reconciliation with CG20, not an assumed general range solver.
- CG16 owns the **minimal single-base extent/index contract vocabulary and
  review lifecycle** needed by C09.1. It does not wait on CG24 to define this
  minimum; CG24 continues to own multi-base, richer derived and frame relations.
  CG17 owns VLA/flexible-array layout, alignment and `container_of`; unsupported
  inputs of those forms remain named Unknowns here.
- `qcore-schemas`: property, typed relations, provenance and evidence carriers;
  `qcore-contract`: versioned consumer selection, complete loading and review
  state; `qcore-core`: pure index/extent decisions and checked arithmetic;
  `qcore-c-gateway`: typed source facts, access/call obligations and invalidation;
  `qcore-audit` / `qcore-verification`: explicit selection, certificates and
  property composition; `proof/`: matching models and registered audits.
  Keep implementation out of the compiler compatibility crate and keep
  providers outside the trusted decision kernel.

Shared records: contract/language specifications, implemented grammars when
syntax lands, STATUS, acceptance matrix, the current increment file and this
queue. No provider-owned contract module is changed merely to raise coverage.
If provider data is needed, follow its owner, byte pins and package gate.

## 3. Implementation work

1. Freeze a concrete, versioned contract grammar and consumer selector before
   implementation. `Idx<N>` in C09.1 is semantic shorthand, not an allocation
   of a keyword/type, a change to `Idx + Ordered`, or implemented syntax.
   Preserve resource-v1's total-or-reject behavior; no old consumer may erase
   a new clause and certify the full contract. Follow the diagnostic namespace
   policy and perform the source-spelling census for any new syntax.
2. Represent a single-base extent and an index relation with element/byte
   units, object/subobject identity, offset, target element width and validity
   state. Bind facts to source/build/target and contract identities. Never use
   an extent as evidence of liveness, custody or non-aliasing.
3. Implement the new consumer's review-held user declarations, concrete review
   reasons and explicit text-edit resolution. Reuse C06.1/C06.2 semantics with
   a checked integration; do not assume resource-v1's marker already reaches
   this consumer. Persist and recheck review state through loading, inheritance,
   packaging (if used), checkpoints, caching and certificate admission. A
   loader or plugin cannot approve a declaration on behalf of a person.
4. Derive fixed-array extents from the actual Clang type. Inventory every
   selected access, including accesses in conditions, returns, actuals and
   loop bodies, or record a named import failure. Check bounds at the access
   and preconditions at call sites. Use external postconditions only for their
   covered exits after the call's preconditions are established.
5. Connect element indices and byte offsets to the core checker; handle
   negative indices, conversions and checked `element_size * index` / access
   end arithmetic. Distinguish one-past formation from access. Preserve CG16's
   original same-object obligations for pointer subtraction and ordering.
   Refuse unresolved identities and unsupported numeric forms explicitly.
6. Invalidate dependent facts on writes, aliasing calls, resize, release or
   reallocation unless their preservation is established. A declaration
   inconsistent with source facts cannot replace them; an inconsistent set of
   premises cannot discharge obligations by vacuity. A checked body refutation
   withdraws dependent uses, including cross-unit uses under one bound context.
7. Publish separate custody and bounds rows, complete access coverage, all
   assumptions and reason sets. Implement C09.1's review-hold standing check:
   accepting a held clause hypothetically does not publish a second verdict,
   and another obstacle keeps the row Unknown. Validate reachability and the
   violation before publishing bounds-Unsafe. All-property composition must
   include a requested but unavailable bounds result as Unknown.
8. Register matching Lean models in `Audit*.lean`: range/unit arithmetic,
   source-bound non-widening, fact invalidation, review withholding, conditional
   call obligations, property composition and scope/evidence identity. Bind
   Rust decisions to the model with shared vectors or equivalent checked
   correspondence evidence. State extraction and Rust–Lean correspondence
   premises; no claim of end-to-end C correctness follows from model proofs.

## 4. Required acceptance controls

Run these through the source consumer and report, not only a core helper.
Each result is for the requested bounds property and its stated profile.

| Control | Required outcome |
|---|---|
| `int arr[4]`, reachable access at 0 or 3 | Bounds Safe when all other bounds obligations discharge |
| Same object, reachable access at 4, 7 or -1 | Validated bounds Unsafe; no user contract required |
| Source-derived count 4 plus a reviewed claim of count 8 | Claim cannot enlarge the object; the source-supported violation remains |
| Custody profile assumes no undefined behavior | That assumption cannot discharge the bounds property being checked |
| External base/count/index relation, review held, otherwise fully checkable | Pending with clause, reason, owner/disposition and digest |
| The same relation explicitly resolved after human review | Conditional Safe only after all access and call obligations discharge |
| Review held plus an unrelated unsupported access or unresolved alias | Unknown with both reasons; approval alone cannot hide the other obstacle |
| A hypothetical violation depending only on the held declaration | Unknown standing reason, never an accusation from an unaccepted premise |
| Missing extent, stale identity, unsupported range or exhausted bound | Unknown; a pin hold is Pending only when C06.2/standing rules permit it |
| Callee requires an in-range index but caller does not establish it | No unconditional caller Safe; unresolved obligation or validated violation |
| Guard establishes a range, then index/base/extent may change | Invalidate or re-prove; no stale range reused |
| Zero extent, count/bytes confused, scaling/addition overflow, signedness mismatch | No fabricated in-range fact; exact named refusal or validated violation |
| One-past formation versus dereference; unrelated objects in subtraction/order | Distinct formation/access and same-object checks |
| Unreachable out-of-bounds branch versus unresolved reachability | Checked exclusion only for proved unreachable; no unvalidated Unsafe |
| Custody Safe and bounds Unknown/Pending/Unsafe | Both rows visible; combined result is respectively Unknown/Pending/Unsafe |
| Missing selected access, omitted bounds row, changed contract/target, malformed evidence | Certificate/admission refuses; denominator is retained |
| Old resource-v1 consumer presented with the new clause | Total rejection or explicit narrower-property result, never full-contract Safe |

Include negative identity, inheritance/review preservation and cross-unit
refutation cases as well as happy paths. Cover pointer reads and writes and
each admitted expression position without changing C's evaluation order.

## 5. Measurement, completion and residuals

Run the [full standing gate](../../agents/validation.md#standing-per-change-gate)
and any conditional provider gate. Record commands, exits, counts, host/Clang
identity, failed attempts and private logs. Archive a fresh BEFORE/AFTER corpus
comparison under `docs/reports/cg16-index-and-offset-extents/`; preserve the
custody-only predecessor byte for byte unless a separately justified correction
is measured. Measure bounds and combined-property coverage separately, with
Unknown and Pending in each requested denominator. No promised Safe-count gain.

Close only with implemented source reach, negative controls, audited Lean,
version/migration documentation, attributed verdict movements, a report and
one increment paragraph. The report must distinguish model proof, source
binding and native observation. Preserve old routes until G4 parity; no merge
or push of main is part of this card.

Reconcile the existing absorbed trackers
`W43-INDEX-TRAVERSAL-STEP-COMPUTED`,
`W43-INDEX-TRAVERSAL-COMPUTED-POINTER-ACCESS` and
`W46-ROUTING-QUERY-EXTENTS`: close only discharged portions and re-register
remaining ones with measured scope. Unsupported symbolic ranges, richer
extents or cross-profile consumers remain named follow-ups under CG20, CG17
or CG24 as appropriate. Completing a bounded sub-increment does not silently
close this full card. Stop only for a concrete missing prerequisite or a
protected action; report the blocker and unfinished acceptance items.
