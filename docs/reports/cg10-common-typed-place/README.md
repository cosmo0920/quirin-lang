# CG10-COMMON-TYPED-PLACE

Card CG10 of the [Clang gap queue](../../plans/clang-gap-cards/README.md), wave
48 round 2, route G (the custody gateway). Entries M01 and I04 and the place
side of K03; evidence Q16, Q17, Q43. Gateway rule
[10t](../../agents/gateway.md#constraint-10t); `QCore.TypedPlace`
([`proof/QCore/TypedPlace.lean`](../../../proof/QCore/TypedPlace.lean), audited in
`proof/AuditTypedPlace.lean`). Base `e182d403` (= local and origin `main`).

## 1. What changed

- **The common typed place** ([`typed_place.rs`](../../../crates/qcore-c-gateway/src/typed_place.rs)).
  A place is read from the Clang AST only: a root `DeclRefExpr` naming a
  `ParmVarDecl` or `VarDecl` (its DeclID), then `->F` / `.F` (FieldDecl id),
  `[I]` and `*`, each with Clang's type and view. A subscript or indirection is
  a *hop* only through an `LValueToRValue` pointer read, never through
  `ArrayToPointerDecay`. A missing fact, a union or bit-field member, a
  `volatile`/`_Atomic` base, any other cast, a call, pointer arithmetic and
  every unlisted form refuse.
- **Tracked roots.** Where W46's mixed-path premise (a never-bound root) fails,
  the reader's thin hook in `Reader::path` asks the place layer. It admits a
  pointer root declared once in the function (no shadow, not `static`, not
  file scope) whose hops are the root alone (*root only*: `R->in.x`,
  `R->arr[i]`, `R[i].x`) or the root and the direct member place `R->F`
  (*direct member*: `R->items[i].v`, `*R->p`, `R->F->in.x`). It lowers the place
  to the existing `AccessChain` with `root_never_bound: false`, so
  `chain_resolves` decides at the flow: the root must be exactly one known,
  non-null, live identity (a released or reallocated root refuses or is
  accused), and `R->F` must carry the admitted store's evidence. A store
  through a subscript or indirection keeps W43's refused-write frame; a
  non-pointer read or store changes no custody; a pointer value read is P2's
  target-free borrowed alias. A deeper place (`R->in.p->x`), an element or
  pointee of the root (`R[i]->x`, `(*R)->x`), a record-object or array root and
  a pointer-valued store (CG11) stay refused. Pointer-valued returns and
  actuals keep rule 10k's premises; a custody-taking actual stays refused.
- **I04 identity.** `StatementIndex::statement_with`: several Clang candidates
  with one statement's line and tokens are told apart only when the statement
  is the run the parser consumed (`typed_place_origin`, its stream index and
  length), the line's tokens equal the analysed source's, every candidate lies
  on that line at a distinct offset, and the candidates number exactly the
  run's occurrences on the line. A condition, return or call-actual operand is
  a copy of the stream with no occurrence and keeps the unique-match refusal.
- **Premise 3** (`W44-AGGREGATE-PREMISE3-MEMBER-NAME`): `aggregate_store::acquired_bindings`
  skips a name after `.` or `->`. Refusing direction only.
- **Parity.** Every place decision is a `position: "typed-place"` row (root
  DeclID, steps with types and views, admitted or the refusal code) recorded
  by the consultation's own hook. The layer is consulted only after every
  older route of the position refused (hard invariant 3); no route is retired.
  "The conservative result wins" is read as rule 10k reads it: an older route's
  admission stands, and the new layer only answers where every older route
  refused.

## 2. Cited claims

| Claim | Outcome |
|---|---|
| M01 (Q16): a mixed path is admitted only on a never-bound root | **reproduced** at base (`m01_…` control, IND refusal on `struct S *s = q_alloc(); s->in.x = 1; s->arr[1] = 2;`) |
| I04 (Q17, Q43): two candidates on one line refuse | **reproduced** at base (`i04_…` control) |
| K03 place side (Q16–Q18): complex actual places are not substituted | reproduced, **unchanged**: every K03 row is a custody-taking actual or an `Out` slot, which rule 10t keeps refused |
| CG00: M01 is the root premise ("expressible-path-premise") | **correction**: of the 7,669 M01 leaves that stay, 6,354 are statements the typed statement reader refuses before any root premise (no parity row; rule 10t never saw them); 919 are positions whose refusal every root premise granted would lift; 275 are positions refused for another construct; 121 more have no row |

## 3. Prediction and outcome

[PREDICTION.md](PREDICTION.md) was frozen at `05e183d9`, before any code.

| Measure | Predicted | Outcome |
|---|---|---|
| BEFORE reproduces `ae17152b…` | 97% | **byte-identical**; system and slice 0 moved functions, 0 moved units ([identity](artifacts/)) |
| Unknown → Safe, corpus | 60 (band 15–200) | **1**, below the band: `rd_kafka_anyconf_is_modified` (a scalar return `!!(confhdr->modified[bkt] & bit)` on a root W4 binds) |
| of those, in CG00's 293 list | ≥ 70% | 0 of 1 (not in the list) |
| Unknown functions whose unsupported leaves drop | 150–1,200 | **796** Unknown functions move leaves only (and 3 provisional Unsafe) |
| Safe → non-Safe | 0–3 | **0** (the premise-3 repair moves nothing) |
| provisional Unsafe added | 0 | **0**; leaks 36 → 36 |
| `unknown_alias` leaves added | hundreds | **2,550** (and 5 removed) |
| I04 leaves removed | 30–60% | **144** of 1,168 (12%): only consumed statements have an occurrence |
| system slice (`046accf6`) | 0–5 functions move | **0** status; 19 functions' leaves move |
| tail-pause slice (`90978ac6`) | 0–3 | **0** status; 20 functions' leaves move |

The miss is the prediction's amendment 2 at full strength: the roots M01
names are overwhelmingly **bound but untracked** (formals handed to a
non-`Borrowed` position, locals assigned from a call or an opaque read), and
`chain_resolves` refuses such a root exactly as it refuses a direct `R->F`
chain on it. Rule 10t turns 1,454 refusals into flow obligations that fail
(`unknown_alias`) rather than into Safe functions. Registered as
`CG10-BOUND-UNTRACKED-ROOTS`.

## 4. Corpus arbiter

`scripts/measure.sh PHASE [ROUTES]` runs the brief's commands into
`private/cg10/PHASE` (BEFORE with the base binary, AFTER with the change);
`scripts/compare.py` diffs one route with W47-FLB-LIB-STRICT-PAIRING's
`compare`, unchanged; `scripts/attribute.py` attributes every moving function;
`scripts/entries.py` follows every CG00 M01/I04 leaf.

| Route | BEFORE | AFTER |
|---|---|---|
| corpus (185 units, `66910c10`) | 526 / 10 / 4,298, `ae17152b…` | **527 / 10 / 4,297**, `b63c1b57…` |
| system (`046accf6`) | 31 / 2 / 201 | 31 / 2 / 201 |
| slice (`90978ac6`) | 24 / 1 / 244 / 1 Pending | 24 / 1 / 244 / 1 Pending |

**Attribution** ([attribution](artifacts/after-corpus.attribution.json.gz)): 800
corpus functions move, every one attributed:

- **tracked-place** 792: an admitted `typed-place` row names the function (788
  Unknown → Unknown, 3 provisional Unsafe → Unsafe with added `unknown_alias`
  only, 1 Unknown → Safe);
- **occurrence** 3: one-line duplicate statements (macro-expanded
  `__tmpstr[0] = …`, `rkbuf` reads) now read, their root refused at the flow;
- **callee-summary** 5: `rd_kafka_anyconf_is_modified` now has a validated
  summary; two callers lose its `uncontracted` leaf and three conditions that
  call it (`rdkafka_conf.c:2863, 3460, 4815`, by reading) now reach their
  operand reads, whose roots the flow refuses.

**New findings, by reading.** No leak or violation is added on any route; the
three provisional Unsafe functions (`mk_http_error`, `mk_net_conn_create`,
`mk_sched_add_connection`) keep their existing leaks and gain only
`unknown_alias` leaves. Two `unsupported` leaves are added, in
`rd_kafka_oauthbearer_set_token0` (`handle->wts_md_lifetime = …`,
`handle->errstr = NULL`): the declaration `handle = rk->rk_sasl.handle`, refused
before, is now read as a borrowed alias, and W45's sub-object store rule
refuses a store through a borrowed-alias base at the flow where it passed a
base the refused statement had poisoned. Both are refusals (Unknown); the
function was and stays Unknown.

**CG00 entry leaves** ([entries](artifacts/after-corpus.entries.json)): of
10,186, 1,458 gone (M01 1,314, I04 144), 1,454 of them with an
`unknown_alias` at the line; 8,728 stay. The BEFORE control keeps all
10,186. Place decisions (parity rows, repeated by W4 re-derivation): 8,753 admitted (top shapes `->.` 4,614, `->->.`
1,504, `->..` 649, `[].` 413); refused: array root 366, element place 245,
deeper place 177, identity 12.

## 5. Tests and pins

- **Flipped BEFORE controls** (`tests/clang_gap_phase2.rs`):
  `m01_mixed_paths_need_a_never_bound_root` → `m01_mixed_paths_on_a_tracked_root_carry_per_hop_obligations`
  (the tracked-root program is now Safe; controls: released root, nullable
  root, null-refined root, borrowed pointer read, pointer store still refused,
  opaque-read root refused at the flow);
  `i04_one_line_identity_refuses_duplicate_candidates` → `i04_one_line_identity_is_the_gateway_occurrence`
  (Safe; duplicate condition operands keep the BRANCH refusal).
- **Deliberate pin updates** (each the tracked-root read of a formerly refused
  shape):
  `indirect_chain_access::a_mixed_path_under_a_tracked_root_still_refuses` →
  `…_carries_the_link_evidence` (Safe through the stored live child; refused
  after its release, `unknown_alias:b->item` with no store);
  `access_path_positions::a_condition_keeps_every_path_refusal` (tracked row:
  BRANCH → `unknown_alias:b->items`);
  `typed_const_pointer_value::a_const_pointer_middle_hop_keeps_its_premises`
  (BRANCH → `unknown_alias:b->pp`); `chained_tail` (`*storage->leaf->label`:
  IND → `UnknownAlias:storage`); `reach_gate_positions` (`*view->count`:
  refusal → `UnknownAlias:view`); unit
  `mixed_embedded_and_indexed_path_carries_per_hop_obligations`. Every flip
  stays non-Safe except the two with a live tracked root and link evidence.
- **New** (`tests/unit/typed_place.rs`, 8): the decision table; every vector
  spelling read by Clang as its root and steps; the Lean binding
  (`cg10_lean_vectors_match_the_rust_tables`); root-only / direct-member
  lowering; deeper, element, record, array, shadowed, global and static roots
  refused; casts, calls, unions, bit-fields and `volatile` bases refused;
  occurrence counting; duplicate statements told apart only with the consumed
  origin and an unrewritten line. `aggregate_store`:
  `a_member_name_is_not_an_acquired_binding`.

## 6. Lean

`QCore.TypedPlace`, **25** audited theorems, no placeholders, standard axioms
only. The place grammar (`readPlace`; `every_unlisted_form_refuses`), the
decision (`decidePlace`, `decision_vectors_hold`, bound to Rust
`DECISION_VECTORS` spelling by spelling), the lowering to
`IndirectChainAccess.Obligation` with the never-bound premise off
(`an_unbound_root_is_refused`, `a_target_free_root_is_refused`,
`a_root_that_is_not_live_is_refused`, `a_nullable_root_is_refused`;
per-hop sufficiency `a_live_root_discharges_a_root_only_place`,
`a_live_linked_member_discharges_a_direct_member_place`,
`every_hop_discharged_defines_the_access`), the shape rule as W46's with the
mixed conjunct widened (`every_w46_admission_stands`,
`the_untracked_case_is_w46s` — the soundness interlock),
`a_nonpointer_access_preserves_the_frame`,
`a_pointer_read_is_a_borrow_without_custody`, `a_pointer_store_stays_refused`,
and non-vacuity on `IndirectChainAccess.example_state`. Premise 3's repair
narrows an extracted fact `AggregateStore` already models
(`an_unacquired_base_never_admits_a_store`).

## 7. Follow-ups

- **Retired:** `W46-ACCESS-PATH-TRACKED-MIXED-PLACES` (its DISCHARGE: tracked
  place, per-hop obligations, Rust refusal controls, audited Lean, measured
  corpus and system effects) and `W44-AGGREGATE-PREMISE3-MEMBER-NAME`
  (implemented in the refusing direction, measured: no corpus move).
- **Amended:** `CG10-COMMON-TYPED-PLACE` (not retired: the two census classes
  were not re-measured by their own scripts), `W44-CHAIN-NESTED-AGGREGATE`,
  `W43-INDIRECT-UNTYPED-SINGLE-ACCESS` (their registering scripts read census
  streams of the retired compiler-example runner; not re-run),
  `REFUSED-HANDON-LINE-GRANULARITY` (unchanged; the approximation stands).
- **Registered:** `CG10-BOUND-UNTRACKED-ROOTS` (2,550 `unknown_alias` leaves on
  800 functions) and `CG10-TYPED-PLACE-RESIDUALS` (refused shapes, I04 outside
  statements, the 6,354 uncaused M01 statement leaves).

## 8. Gate

[`scripts/gate.sh`](scripts/gate.sh), exits in [gate/](gate/), figures in
[gate.json](gate.json); logs in `private/cg10/gate`. Submodule 0; fmt 0;
`run_test_binaries.py` 6,569 passed, 1 failed, 11 ignored (367 units): the
failure is `cycle20_verifier_plugins::vp04_vp07_vp08_…`, a wall-clock
`elapsed < 500 ms` assertion under a load average of 28–36 (three cards
running), unrelated to the gateway; it failed again at load 28 (`03b`, 101)
and passed 20/20 at load 20 (`03c`, 0). Clippy 0; both reevaluation probes 0;
`check_lean_audits.py` 228 passed; Python tests 365 OK (and again after the
matrix regeneration, `13-python.exit`); docs index 0; `cargo clean --profile
dev` 0. `providers/` is unchanged: no provider gate. The tip binary was
rebuilt and the corpus arbiter re-run at the tip (`tip`); see §4.
