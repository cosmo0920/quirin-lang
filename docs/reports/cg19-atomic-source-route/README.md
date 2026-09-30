# CG19-ATOMIC-SOURCE-ROUTE

Wave 48, round 7, High. Card CG19 of the
[Clang gap queue](../../plans/clang-gap-cards/README.md), gap entries W01 and
W09 (evidence Q10, Q34, Q35, Q39, Q46); routes G, T and the atomic-publication
route. Gateway rule [11i](../../agents/gateway.md#constraint-11i);
`QCore.AtomicSourceRoute`
([`proof/QCore/AtomicSourceRoute.lean`](../../../proof/QCore/AtomicSourceRoute.lean),
audited in `proof/AuditAtomicSourceRoute.lean`). Base `c374de96`; `main`
`b59aaae2` (CG07-GENERIC-AND-STATEMENT-EXPRESSIONS, W48-QRLIB-MACOS) merged
at `5b8cd34c`, `main` `60cb0895` (W48-CALLEE-CLOSURE-R3) at `4fd89b6f`,
`main` `fe9c5b48` (CG14) at `0d846040` and the final `main` `e93dbe4e` (CG15)
at `e8cc26bd`; every figure below is re-measured on `b59aaae2`, `60cb0895`
and `e93dbe4e` (§4; `fe9c5b48` is contained in `e93dbe4e` and was not
measured separately).

## 1. What changed

- **The route** ([`atomic_source_route.rs`](../../../crates/qcore-c-gateway/src/atomic_source_route.rs)).
  `ATOMIC_FORM_TABLE` classifies 51 builtins: `__c11_atomic_load`/`store`
  and `__atomic_load_n`/`store_n` (plain), `__c11_atomic_init`
  (initialisation), the generic by-pointer `__atomic_load`/`store`, every
  exchange, fetch-and-op, op-and-fetch, test-and-set and listed `__sync`
  read-modify-write, the compare-exchanges and the fences. `<stdatomic.h>`
  arrives through Clang's lowering to `__c11_atomic_*`. `classify_qualified`
  reads the `_Atomic` operator forms: the lvalue conversion (`AtomicToNonAtomic`,
  a seq_cst load), the simple assignment (a `NonAtomicToAtomic` value, a
  seq_cst store) and the compound assignment and increment (read-modify-writes).
  These are the atomic casts the gap entry names; the elaborator's cast rows
  for them stay refused (§8). A plain operation becomes the core model's own
  event (`AtomicLoad`/`AtomicStore`/`AtomicInit`) with Clang's order decoded
  (0–5) and admitted per C11 7.17.7; the model's own validity test is exposed
  additively as `qcore_core::event_order_represented`, and every routed order
  satisfies it. Everything else refuses **by name**: read-modify-write,
  compare-exchange and fence (CG28), a consume load (by value 1 or by the
  enumerator `memory_order_consume`), an order invalid for the operation, an
  unevaluated order, a by-pointer form, a pointer or aggregate value, and an
  unlisted builtin of the three families.
- **Route G** ([`legacy_atomic_route.rs`](../../../crates/qcore-c-gateway/src/legacy_atomic_route.rs);
  hooks at the end of `expression_statement` and of the `return` chain in
  `legacy_c.rs`, after CG07's). Asked only where the classifier refused: a
  statement or `return` refused as a call to an uncontracted atomic builtin
  (for a `return`, W43's whole-call re-attribution), and a statement whose top
  node is an `_Atomic` store or read-modify-write. A routed plain access is
  lowered to **the plain access's custody effect**: the typed reader's
  dereference of the pointer object, read with the value type (the reader and
  `is_pointer` refuse every `_Atomic` spelling, including an `_Atomic(T)`
  pointee, which is this route's case), as `IndirectScalarAccess` with its
  base obligation (checked by `transfer`) and, for a store, the refused-write
  frame; `&x` reads nothing; a loaded value's scalar destination forgets its
  fact; a returned scalar escapes nothing. The order goes to the parity row
  only (`position: "atomic-route-…"`): G's verdict is custody, and it claims
  no ordering. Every other answer keeps the refusal's writes and replaces its
  text with the route's named refusal.
- **Route T** (`typed_source.rs`). An `_Atomic` type has its own arm, asked
  first, with its own message; the `volatile`/`restrict` arm is unchanged
  (CG15, CG18). An atomic builtin is named by the route's answer instead of
  `unmodeled callback expression AtomicExpr`. No T consumer reads a routed
  access, so T admits none.
- **Publication (W09)** ([`atomic_publication_source.rs`](../../../crates/qcore-c-gateway/src/atomic_publication_source.rs)).
  The extractor reads the flag's store and load through the route: any
  routed plain store/load of the flag (`__c11_atomic_*`, the generics, the
  `_Atomic` forms, and GNU `__atomic_*` builtins on a non-`_Atomic` `int` flag
  both of whose accesses are one, with that assumption stated in the
  certificate), a literal order or a canonical enumerator (the canonical
  enumeration is now required only where an enumerator is used), and a
  `seen == READY` guarded use beside `seen != READY` early return. It records
  the stages it derived (`AtomicPublicationStages`: initialise, publish,
  observe, use) and says where it stops: `reclaim: not derived:
  reclamation is outside this route (CG29)`. A consume load or a
  read-modify-write of the flag is `NamedAtomicRefusal` with the route's text.
- **Production caller** (absorbs CT41-ROUTE-HAS-NO-PRODUCTION-CALLER).
  `qcore_verification::atomic_publication_source::atomic_publication_verdict`
  is `Safe` exactly when extraction and the core model both accept and
  `Unknown` on any refusal; both carry the same assumptions, now including
  `SINGLE_GRAPH_ASSUMPTION` (one candidate execution; not race freedom over
  every execution, W07/CG29; no liveness, W11).
  `quirin-audit atomic-publication CONFIG.json`
  ([`atomic_publication.rs`](../../../crates/qcore-audit/src/atomic_publication.rs))
  selects a unit and a publication profile, writes a report with the verdict
  contract's fields (artifact, property, profile, scope, bounds, assumptions,
  evidence version, source digest, result, refusal, stages) and exits
  non-zero on `Unknown`.

Shared foundations: `elaboration.rs`, `typed_place.rs` and
`call_instantiation.rs` are **not** touched. Additive items: the pub module
`atomic_source_route`, `qcore_core::event_order_represented` (and its
re-export), `AtomicPublicationSourceRefusal::NamedAtomicRefusal`,
`AtomicPublicationStages` and the certificate's `stages` field,
`AtomicPublicationVerdict`/`atomic_publication_verdict`, two hooks in
`legacy_c.rs` and two arms in `typed_source.rs`.

## 2. Cited claims

- **Q10** (T refuses `_Atomic`): reproduced; the one arm also refused
  `volatile` and `restrict` with the same message (the CT41 entry).
- **Q34** (weak_memory: plain loads/stores, no RMW/fence/consume, one supplied
  graph): reproduced by reading and by CG00's core controls.
- **Q35** (the publication route is a dedicated shape): reproduced: before
  this card the extractor accepted exactly `__c11_atomic_store`/`load` with
  canonical enumerators and a `!=` gate.
- **Q39** (`typed_root_declarations.rs` reads `AtomicExpr` operands as
  operands of a builtin that is not a call): reproduced by reading; unchanged.
- **Q46** (concurrency.rs's scope): reproduced by reading; unchanged.
- **W01 correction (partial).** The entry says G has "specialized routes such
  as atomic publication". G never called the publication route; it read an
  atomic builtin as an uncontracted call and an `_Atomic` load as a neutral
  scalar, and it refused `_Atomic` stores and read-modify-writes under
  generic texts (`unsupported assignment expression`, `unrecognized
  assignment destination`, a formal-mutation refusal). The publication route
  was reachable only from tests (the CT41 entry).

## 3. Prediction and outcome

[PREDICTION.md](PREDICTION.md), frozen before any AFTER run (with the
disclosure that the route's code preceded it, and an amendment for the
`_Atomic` statement hook, also before any AFTER run).

| Prediction | Outcome |
|---|---|
| `cmt_atomic_load` Unknown → Safe | **Unknown → Safe** (its one leaf, the uncontracted `__atomic_load_n`, gone) |
| `cmt_atomic_store` Unknown → Safe | **Unknown → Safe** (the uncontracted `__atomic_store_n` gone) |
| `cmt_atomic_compare_exchange` stays Unknown, leaf renamed | **stays Unknown**, its one leaf now ``atomic compare-exchange `__atomic_compare_exchange` is not represented by the weak-memory model (CG28)`` |
| nothing else moves; 610 / 10 / 4,214 | **610 / 10 / 4,214** on `c374de96`+`b59aaae2` (and on `60cb0895`); **625 / 10 / 4,199** on the final `e93dbe4e` (623 / 10 / 4,201 before); each time 2 moves, 1 same-status function with a changed leaf, no other leaf changed |
| no Safe → non-Safe, no added leak or violation | none |
| `_Atomic` statement hook: no corpus effect | none (no arbiter unit spells `_Atomic`) |
| slices unreachable | not re-run (0 atomic-builtin leaves in either) |

The prediction held exactly. No correction: no function moved Safe → non-Safe.
CG00's measured upper bound for CG19 (2 functions freed alone, 2 with no alias
leaf) is met.

## 4. Measurement

Corpus arbiter (`scripts/run_pinned_audit.py … 66910c10 --jobs 4`, release
`quirin-audit`, `QCORE_TYPED_PATH_PARITY` on), each into a fresh
`private/cg19/<run>/arbiter`:

| Run | Tree | Binary | `resource-diagnostics.json` | Safe / Unsafe / Unknown |
|---|---|---|---|---|
| BEFORE (base) | `c374de96` | built at `c374de96` | `3a25c4e9…` (the brief's baseline) | 608 / 10 / 4,216 |
| BEFORE (merged main) | `b59aaae2` | `b8bcc25c…` | `8b9e4047…` (the integrator's figure for `b59aaae2`) | 608 / 10 / 4,216 |
| AFTER (on `b59aaae2`) | merge `5b8cd34c`, code as built at `95b7f93e` | `220c2b92…` | `c7d4dfa4…` | **610 / 10 / 4,214** |
| BEFORE (merged main) | `60cb0895` | built at `60cb0895` | `c2cc1db902f24bfb131f6f8453f040a1f535a8923ff03f5f8c58ef1ac4fdb49a` | 608 / 10 / 4,216 |
| AFTER (on `60cb0895`) | merge `4fd89b6f` | `a16e1630…` | `31503cba…` | **610 / 10 / 4,214** |
| BEFORE (final main) | `e93dbe4e` | built at `e93dbe4e` | `d42a99e5e677a44584e883b984085a8e246190fe39bb7987fc4e67b508ba2f33` (623 / 10 / 4,201, the integrator's figure) | 623 / 10 / 4,201 |
| AFTER (tip) | merge `e8cc26bd` | `ea5a3ce5…` | `ac2d4e19cfc465b9a8ce1eade5b83f4e670865192429d2d0cc821c7cad76c97f` | **625 / 10 / 4,199** |

Attribution ([`artifacts/corpus-comparison.json`](artifacts/corpus-comparison.json),
`scripts/compare.py`, BEFORE `e93dbe4e` → AFTER tip; the `b59aaae2` and
`60cb0895` pairs gave the identical record): the two moves above, each by removing its only leaf;
`cmt_atomic_compare_exchange`'s one leaf renamed; leaf delta otherwise empty.
Atomic-builtin census ([before](artifacts/before-census.json),
[after](artifacts/after-census.json), `scripts/census.py`): 3 leaves on 3
functions → 1 leaf on 1 function. The AFTER parity stream holds 7
`atomic-route-*` rows on those 3 sites (repeats are the rebuilds of the W4
re-check): the load and store routed with `SequentiallyConsistent`, the
compare-exchange refused by name. The elaborator's `atomic access (CG19)` row
and the atomic cast rows appear in no parity row of either run (0).

**Slices.** The system slice and the tail-pause slice are not re-run: neither
carries a leaf naming an atomic builtin (CG15's BEFORE system run on main
`f9deddc3…` and the shared tail-pause run `w48cc-after`: 0 each), none of
their 21 source files spells an atomic builtin, and every hook of this card
fires only on an atomic builtin's refusal or an `_Atomic` statement. System
42 / 2 / 190 and tail-pause 29 / 1 / 239 / 1 Pending (the integrator's figures
at `e93dbe4e`) are carried.

**CT stages.** `python3 scripts/ct_stage_summary.py . --check`: exit 0,
mapped 50 / unmapped 0, no stage moved.

## 5. Tests and pins

- **Flipped BEFORE controls** (CG00): `clang_gap_phase3.rs`
  `w01_atomic_accesses_do_not_reach_the_atomic_model` (T's two refusal texts;
  G's `__c11_atomic_load` now `Safe`) and
  `w09_general_publication_is_not_derived_from_source` (G's function-local
  release store now custody-`Safe`, the publication judgment being the
  publication route's); `clang_gap_phase4.rs`
  `w02_w03_w04_atomic_rmw_cas_and_fences_are_uncontracted_calls` (still
  `Unknown`, now by the route's names). `tests/atomic_publication_source.rs`
  `atomic_spelling_extracts_orders_without_claiming_synchronization`: the
  `_Atomic` simple assignment `done = 1` is now a seq_cst publisher store
  (BEFORE `UnsupportedPublisher`).
- **New** (`crates/qcore-c-gateway/tests/atomic_source_route.rs`): the
  cmetrics shapes (load and store `Safe`, compare-exchange refused by name);
  `<stdatomic.h>` plain forms and destinations; the dereference obligation
  kept (a store through a released pointer is not `Safe`); refusal controls
  for `atomic_fetch_add` (statement, initializer, return),
  `atomic_compare_exchange_strong`, `atomic_thread_fence`, a consume load (by
  enumerator and by `__ATOMIC_CONSUME`), a release load and a pointer-valued
  load; a `volatile` that is not atomic (G unchanged, T's volatile and
  restrict arm unchanged); T's discrimination; `_Atomic` stores and
  read-modify-writes in G. `tests/atomic_publication_source.rs`
  `cg19_publication_forms_through_the_atomic_route`; the verification test
  `cg19_publication_verdict_gates_on_the_composed_check`; the audit unit test
  `cg19_atomic_publication_route_reports_safe_and_unknown`; the unit tests of
  the route (`tests/unit/atomic_source_route.rs`, 7).
- **CT stage summary**: `scripts/ct_stage_summary.py . --check` exit 0,
  mapped 50 / unmapped 0; no CT stage moved (CT41's controls, run by
  `compiler/tests/support/ct_atomic_publication.rs`, answer as before).

## 6. Lean

`QCore.AtomicSourceRoute`, 23 audited theorems, no axioms beyond `propext`:
`names_unique`; `routed_order_is_represented` and
`routed_plain_agrees_with_model` (every admission is a plain operation whose
order the model represents); `route_class_is_plain`;
`read_modify_write_refused`, `compare_exchange_refused`, `fence_refused`,
`by_pointer_refused`, `consume_load_refused`, `order_one_never_routed`,
`consume_never_routed`, `unlisted_refused`, `unevaluated_order_refused`;
`routed_store_matches_publication`, `routed_load_matches_publication`,
`releases_iff`, `acquires_iff` (agreement with `QCore.AtomicPublication` on
Clang's numbering); `publication_verdict_carries_single_graph`,
`failed_publication_is_unknown`, `undecided_publication_is_unknown`,
`safe_publication_only_when_checked`; `route_vectors_hold` and
`order_vectors_hold`. Bound to Rust by `tests/unit/atomic_source_route.rs`:
`formTable` = `ATOMIC_FORM_TABLE` row for row, `routeVectors` run through
`classify_builtin` on synthetic nodes, `orderVectors` through `decode_order`
and `route_order`, `modelRepresents` = `event_order_represented` for every
load/store order, and routed events run through `check_c11_execution`.
Premises by review: faithful Clang extraction (trust boundary 28) and the
Rust-to-Lean correspondence of the verdict gate.

## 7. Scope stated

A routed access is one event; the publication route checks one candidate
execution of one closed shape. No verdict here is race freedom over every
execution (W07, CG29), liveness or eventual observation (W11), or a
reclamation proof (CG29). Route G's `Safe` is custody under its declared
contracts, with no ordering claim.

## 8. Follow-ups

- **Closed:** `CG19-ATOMIC-SOURCE-ROUTE` (every DISCHARGE clause holds;
  reclamation is CG29's by the brief), `CT41-ROUTE-HAS-NO-PRODUCTION-CALLER`
  (the `quirin-audit atomic-publication` route), and
  `CT41-TYPED-SOURCE-QUALIFIER-ARM-STILL-UNDISCRIMINATING` (T's `_Atomic` arm
  with its own message and controls; `volatile` and `restrict` still share
  the old arm, owned by CG15 and CG18).
- **Registered:** `CG19-ENUMERATOR-ORDER-UNEVALUATED-IN-G-AND-T` (0 corpus
  leaves), `CG19-ATOMIC-FORMAL-VALUE-STORE-OUT-PARAMETER` (0),
  `CG19-PUBLICATION-ONE-FLAG-ONE-PAYLOAD` (the route is not in the arbiter).
- **Amended:** `CG28-WEAK-MEMORY-EVENTS` (the source side is routed and
  refused by name; 1 corpus leaf), `CG29-EXECUTION-COVERAGE-AND-RECLAMATION`
  (the single-graph assumption and the stated reclamation stop).
- **Not absorbed, unchanged:** `CT41-PROOF-NOT-BOUND-TO-THE-CASE-ARTIFACT`,
  `CT41-BOUNDS-ARE-CALLER-DECLARED-PREMISES`,
  `CT41-VOLATILE-COMPLEMENT-UNMEASURED` (CG15's). The elaborator's
  `atomic access` row and the `AtomicToNonAtomic`/`NonAtomicToAtomic` cast
  rows stay `Refused("CG19")`: the corpus parity stream carries no refusal of
  them (0 rows), so no follow-up is registered for them; the route reads
  those casts itself.

## 9. Gate

Script [`scripts/gate.sh`](scripts/gate.sh), logs `private/cg19/gate2` of
the card's worktree, record [`gate.json`](gate.json), exits in [`gate/`](gate/).
GATE_TIP_LINE

| Step | Command | Exit | Count |
|---|---|---:|---|
| 01 | `git submodule update --init --recursive` | 0 | |
| 03 | `python3 scripts/run_test_binaries.py` | 101 | 402 units, 415 summaries, 7,060 passed, 1 failed, 11 ignored |
| 03b | `cargo test -p qcore-audit --test checkpoint_cli` (alone) | 0 | 7 / 7 |
| 03c | `python3 scripts/run_test_binaries.py` (re-run) | 101 | 7,059 passed, 2 failed, 11 ignored |
| 03d | `cargo test -p qcore-compiler --test cycle20_verifier_plugins` (alone) | 0 | 20 / 20 |
| 03e | `cargo test -p qcore-audit --test checkpoint_cli` (alone) | 0 | 7 / 7 |
| 02 | `cargo fmt --all -- --check` | 0 | |
| 05 | `cargo clippy --all-targets -- -D warnings` | 0 | |
| 06 | `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | |
| 07 | `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | |
| 08 | `python3 scripts/check_lean_audits.py` | 0 | 257 audits (256 + `AuditAtomicSourceRoute.lean`) |
| 09 | `python3 -m unittest discover -s scripts/tests` | 1 | 365; the derived Lean audit count 256 → 257 |
| 09a | `build_matrix.py . --gate gate.json` | 0 | |
| 09b | `python3 -m unittest discover -s scripts/tests` | 0 | 365 |
| 10 | `python3 scripts/build_docs_index.py` | 0 | |

The failures of 03 and 03c are timing assertions under a shared host at load
45–72 (`checkpoint_cli`'s `no unit checkpoint before timeout`,
`cycle20_verifier_plugins`' `elapsed < 500ms`); each passes alone
(constraint 16: verified in isolation, not weakened), and every one of the
7,061 tests passed in at least one run. A first gate run (`private/cg19/gate`)
found one real failure, fixed at `095c0c63`: the new `atomic-publication`
subcommand was missing from `subcommand_plugins::BUILTINS`, so an external
plugin of that name could have shadowed it. The provider gate is not due
(`providers/` unchanged). Clang 21.1.8.
