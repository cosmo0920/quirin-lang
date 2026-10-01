# CG17-LAYOUT-DERIVED-EXTENTS: VLA, flexible-array, alignment and container_of facts from Clang's layout

Card `CG17-LAYOUT-DERIVED-EXTENTS` of the
[Clang gap card queue](../../plans/clang-gap-cards/README.md) (wave 48, round 10;
entries M12 M13 M19 E18; absorbs `W45-FB-OR6-ELEMENT-IDENTITY`). Base: local
`main` at `b6a3dc42` (= origin/main), unchanged during the card.

- **Rule:** [gateway 11u](../../agents/gateway.md#constraint-11u).
- **Lean:** `QCore.LayoutExtents`, `proof/AuditLayoutExtents.lean` (24 theorems).
- **Prediction:** frozen in [PREDICTION.md](PREDICTION.md) (commit `ba84ff28`),
  before any AFTER measurement and before the route-G hooks ran on any input
  (the pure module was drafted first; nothing measured it).
- **Published property** (Phase 4's choice, by the user's instruction to launch
  every remaining card): `layout.alignment.v1`, scope one function of a C11
  unit read by Clang through `layout_extents::layout_report`, obligations from
  derived-address conversions (C11 6.3.2.3p7), `Safe` only when every
  obligation is aligned, assumption `P-CG17-POINTER-ALIGNED` when used,
  evidence version `cg17-layout-extents.v1`. It is a library report, not a
  `quirin-audit` verdict, and it is reported apart from custody (projection
  coverage is not safety coverage). The flexible-array extent and the VLA
  facts are published as facts for CG16's bounds and CG20's numbers, not as
  properties.

## 1. What changed

**`layout_extents.rs`** (new, `pub`): four pure decisions with their Clang
readers, each bound to Lean by shared vectors.

- `vla_decision` (M12): a declaration of one automatic VLA is admitted only for
  a builtin scalar element, a bound Clang prints as `ELEM[BOUND]` whose tokens
  only read (integer literals, `+ - * ( )`, names declared once as
  non-`volatile` integer parameters or automatics), and uses that are element
  accesses or `sizeof` operands. The decision carries three numeric
  obligations (`bound-defined`, `bound-positive`, `size-representable`),
  never discharged here (CG20). Any other use (a decay into an actual, a
  return, a pointer store, `&a`, `&a[i]`) refuses as `address-escapes`: the
  lifetime ends at the block's exit (`the_address_is_dead_after_the_block`).
- `flexible_extent` (M13): `(size − offsetof(data)) / sizeof(T)` with Clang's
  offset and width, only for an established allocation size; no current
  contract establishes one (CG16's vocabulary), so every access reads
  `size-not-established`.
- `alignment` / `alignment_verdict` (M19): every `BitCast` of a derived address
  (pointer arithmetic or `&x`, through value-preserving pointer conversions)
  to a non-character, non-`void` referent carries the obligation; Clang's
  `_Alignof`/`sizeof` (one probe per function, `clang_constants`) decide it
  aligned, misaligned or undecided; misaligned joins to `Unknown`.
- `container_of` / `member_provenance` / `container_shape` (E18):
  `(T *)((char *)P - OFF)` is the enclosing object `x` only when `P` is
  `&x->m…` (or a pointer local whose only write is that initializer, neither
  written nor addressed, `x` declared once), with the records and member
  paths of `P`, the result and `OFF` equal. `OFF` is an `offsetof(T, m…)`
  read from its own tokens or the null-pointer form `(integer) &((T *) 0)->m…`.

**Route G** (`legacy_layout_extents.rs`, thin hooks in `legacy_c.rs`, asked
last, only for a refusal):

- an admitted VLA declaration is lowered to `ForgetScalar` of its name (a new
  object each time it runs, no custody event);
- a declaration, assignment or `return` whose whole value is an admitted
  `container_of` is lowered as the respelled `… = x;` / `return x;` by the
  older routes, and only if they admit it;
- parity runs record every consultation and a per-function census
  (`layout-extents-census`).

**Shared items added (for the integrator):** `typed_access_path::analysed_unit_constants`
(Clang constants over the analysed unit); `legacy_c.rs`: the module
declaration `layout_extents_route`, two hooks at the end of
`expression_statement`, one at the end of the `return` chain, one census call
beside `record_callee_cast_census`. No shared enum gained a variant. `lib.rs`:
`pub mod layout_extents`. The CG00 control `m12_variable_length_arrays_are_refused`
is flipped deliberately (with an escaping-address refusal kept);
`m13`/`m19`/`e18` gain notes (custody unchanged).

## 2. The cited claims, reproduced at the base

Every claim the card relies on was reproduced by behaviour at `b6a3dc42`
(the CG00 controls in `crates/qcore-c-gateway/tests/clang_gap_phase3.rs` and a
route-G probe run before the hooks existed). The supplied evidence's line
ranges (Q11, Q12, Q16, Q18, Q33, Q36, Q38) cite file digests of earlier
revisions; the claims were checked by what the code does now, not by line.

| Entry | Claim | At the base |
|---|---|---|
| M12 | T supports fixed storage only | reproduced: T refuses `int a[n];` (`local outside scalar/pointer callback profile`) |
| M12 | G has an Unknown fixture for VLA `sizeof` side effects | **correction (CG00's, re-confirmed)**: the G declaration refuses for another reason, `local declaration may shadow a formal resource identity` (the bound names a formal), or `indirect memory access …` when the bound is a local; CG08 already admits an evaluated VLA `sizeof` |
| M12/M13 | the core footprint model excludes VLAs and flexible members | reproduced: `UnsupportedFootprint::{VariableLengthArray, FlexibleArrayMember}` (`crates/qcore-core/tests/clang_gap_core.rs`), unchanged |
| M13 | no gateway connection derives a flexible extent | reproduced: `f->data[i]` custody-Safe with no extent |
| M19 | no source connection compares alignment | reproduced: `*(int *)(c + 1)` custody-Safe with no obligation |
| E18 | T has no `OffsetOfExpr` | reproduced (`unmodeled callback expression OffsetOfExpr`) |
| E18 | G's `container_of` boundaries are shapes, not a model | reproduced: rule 5l's statement expression over a cursor is the only reading; `return (T *)((char *)m - offsetof(T, b))` refuses (`unmodeled … in return expression`), and so does `&x->b`'s round trip |

One measurement correction of this card's own: the first AFTER corpus attempt
ran the BEFORE binary (`scripts/run_pinned_audit.py` always runs
`target/release/quirin-audit`, and the AFTER binary had been built into a
separate target directory); it was discarded and `scripts/measure.sh` now
installs the measured binary first. The BEFORE system step's exit record was
overwritten by that edit while `measure.sh` ran; the step was re-run alone
(`scripts/system-only.sh before2`) and is byte-identical (`9d3309b7…`).

## 3. Measurements

Release binaries: BEFORE `e2f76113…` (built at `b6a3dc42`), AFTER `7b62706f…`
(built at `b66124d4`, `scripts/build-after.sh`); digests in
`artifacts/binaries.sha256`. Arbiter `--jobs 4`, pin `66910c10`.

| Route | BEFORE | AFTER | Moves |
|---|---|---|---|
| Corpus | 629 / 10 / 4,195, `3d53a5d0…` | 629 / 10 / 4,195, `3d53a5d0…` | none; no leaf changed |
| System | 44 / 2 / 188, `9d3309b7…` | 44 / 2 / 188, `9d3309b7…` | none |
| Tail-pause slice | 29 / 1 / 239 / 1 Pending, `c9e3ee81…` | the same, `c9e3ee81…` | none |

The corpus totals and digest equal the brief's baseline. The system and slice
totals equal it; their digests differ from the baseline's (`79404fed…`,
`93cc660c…`) because those reports carry this worktree's absolute paths (the
profile directory); BEFORE and AFTER here are byte-identical.

**Prediction against outcome.** Predicted: no function moves on any route,
0–5 same-status functions losing a VLA leaf, every `container_of`
unestablished, system and slice byte-identical. Outcome: no function moves,
**0** leaf changes (the corpus has no VLA at all), every `container_of`
unestablished, all three reports byte-identical. No Safe → non-Safe move, no
leak or violation added, no correction to a published verdict.

**Synthetic movement** (`tests/layout_extents.rs`, `clang_gap_phase3.rs`):
`void f(int n) { int a[n]; a[0] = 1; }` Unknown → Safe; `x = q_alloc(); y =
container_of(&x->b, …); q_free(y);` Unknown → Safe (and `q_free(x)` after it
is the double release it is); `return container_of(&x->b, …)` Unknown → Safe.
Refusals kept: a VLA address returned, passed or element-addressed; a bound
with a call; pointer elements; `container_of` of an unrelated pointer, of
another member, of another record, of a list lookup.

## 4. Census

Parity rows `position: "layout-extents-census"`, de-duplicated
(`scripts/census.py`; the join to BEFORE leaves is by function and line).

| Form | Corpus | System | Slice |
|---|---|---|---|
| VLA declarations | 0 | 0 | 0 |
| `container_of` shapes (all `unestablished: not-a-member-address`) | 355 / 189 functions | 99 / 66 | 66 / 52 |
| `offsetof` expressions | 380 / 192 | 100 / 66 | 68 / 52 |
| derived-address conversions | 441 / 232 (41 aligned, 0 misaligned, 400 undecided) | 105 / 69 (5 aligned) | 70 / 55 (4 aligned) |
| flexible-array accesses (all `size-not-established`) | 16 / 12 | 0 | 0 |

The route-G hooks were never consulted on the corpus: no VLA, and no
`container_of` value reached as a whole statement value (every one stands in
a statement expression whose inner statements the older routes refuse first:
46 indirect-access, 38 `__builtin_offsetof`, 19 braced leaves on those lines
at BEFORE). Undecided alignments are mostly the `container_of` subtraction
itself (a `(char *)__mptr` base whose alignment the reader does not name)
and `char *` cursors. Per-route records: `artifacts/census-*.json`.

## 5. Absorbed follow-up and residuals

- **`W45-FB-OR6-ELEMENT-IDENTITY`** (absorbed): **amended**, not discharged.
  OR-6's `queued_task = mk_list_entry_first(&queue->pending, …)` is a list
  lookup; rule 11u refuses it as `not-a-member-address` by design (its
  identity is rule 10f's container premise). Re-measured on the system route:
  `flb_output_task_queue_flush_one` and `flb_output_task_singleplex_flush_next`
  stay Unknown with the same leaves (counted-user operation, a refuted
  contract, unresolved `queued_task`, an indirect access, and the mutual
  uncontracted calls); size unchanged (1 obligation, 2 functions); the
  blocker stays `W45-FOREIGN-UNLINK-SUMMARY`.
- **`CG17-LAYOUT-DERIVED-EXTENTS`**: retired (its DISCHARGE clauses hold).
- **Registered:** `CG17-FLEXIBLE-EXTENT-UNESTABLISHED` (16 accesses / 12
  functions; needs CG16's allocation-size vocabulary),
  `CG17-ALIGNMENT-UNDECIDED` (400 of 441 conversions / 232 functions; no
  `quirin-audit` consumer selects `layout.alignment.v1`),
  `CG17-CONTAINER-OF-LIST-LOOKUPS` (355 shapes / 189 functions).
  Ledger edits: `scripts/update_ledger.py` (666 → 668).
- **Not done here:** VLA numeric obligations stay CG20's; no extent
  vocabulary is defined (CG16's, rule 11s), so nothing is consumed from it
  yet; when CG16 lands, `flexible_extent` takes its established size.

## 6. Gate

⟨gate⟩
