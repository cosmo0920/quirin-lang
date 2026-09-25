# W45-SUB-OBJECT-STORES: stores into a sub-object through a resolved base

Card W45-SUB-OBJECT-STORES, 2026-09-25/26, on base `b160d765` (local `main`,
"Open the wave-45 increment record").

Gateway constraint 5i (W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE) refuses every
store whose left-hand side is a sub-object at the lexical gate, and W43's typed
scalar route re-admits only `p->f = e` / `s.f = e` with scalar sides and no
call. This card censuses the refused stores by shape and stored value, and
lowers them through a resolved typed base as the store they are, admitting
exactly those that move no custody. 5i and the refused hand-on rule are intact.

The forecast was frozen before the rule was wired ([PREDICTION.md](PREDICTION.md),
commit `5096a620`). Commits: census stream `b6261263`; reader, instruction and
transfer, unwired, `bf51aea2`, `86d436b8`, `6bf77d1a`; wiring `e31926ec`; two
repairs after the first AFTER run `e91bf3e6`; the final custody decision
`a175403b` (§3).

## Headline

| Measure | BEFORE (`b160d765`) | AFTER (`a175403b`) | Delta |
|---|---:|---:|---:|
| Functions Safe / provisional Unsafe / Unknown | 306 / 12 / 4,516 | **307 / 12 / 4,515** | +1 / 0 / −1 |
| Units Safe / provisional Unsafe / Unknown / data-only | 3 / 10 / 168 / 4 | 3 / 10 / 168 / 4 | 0 |
| `leak` / `summary_conflict` / violations | 37 / 1 / 0 | 37 / 1 / 0 | 0 (every leaf identical) |
| `unknown_alias` | 3,507 | 3,507 | 0 |
| `unsupported` | 43,590 | **43,360** | −230 |
| indirect memory access: leaves / Unknown functions / sole | 17,710 / 3,324 / 388 | **17,487 / 3,307 / 387** | −223 / −17 / −1 |

* **Census** ([§1](#1-census)): 10,490 distinct sub-object store statements,
  8,391 refused by the gate (6,912 in Unknown functions). Admitting every one
  would free at most 64 of the 388 sole-blocked functions. The declaration
  reader, not the store, is the binding constraint: incomplete records (2,141),
  chains and nested aggregates (1,101), unreadable functions (1,101) and
  unresolved roots (521) refuse most of them before any custody question.
* **Rule** ([§2](#2-the-rule)): `LegacyInstruction::SubObjectStore`. 1,054
  statements are lowered; a store is admitted only when its value carries no
  tracked identity. The sole-blocked indirect family moves by one function.
* **The custody decision** ([§3](#3-three-arbiter-runs-and-the-custody-decision)):
  the brief's escape of an owned identity into a caller-reachable record was
  implemented and measured, and withdrawn: two standing pins
  (`a_store_into_a_borrowed_in_aggregate_is_never_admitted`,
  `a_refused_store_of_the_identity_withdraws_the_leak`) encode that no summary
  mode states that transfer, so a caller's `Safe` would rest on it unseen. Such
  a store keeps the refusal, which hands the identity on.
* **Moves**: `destroy_id` Unknown → Safe; 0 provisional Unsafe; 0 unit moves;
  every leak and violation leaf identical.
* **Lean**: `QCore.SubObjectStore`, 18 theorems, audited by
  `AuditSubObjectStore.lean`, no placeholders, no project axioms.
* **Ledger**: 393 → 399. `W44-SUB-OBJECT-STORE-REFUSED` retired against its
  discharge condition; seven measured residuals registered ([§6](#6-ledger)).

## 1. Census

### 1.1 Instrumentation (`b6261263`, reader answers `bf51aea2`)

The `indirect-census` feature gains `QCORE_SUB_OBJECT_STORE_CENSUS`
(`crates/qcore-c-gateway/src/sub_object_store_census.rs`, listed in
`census_sink_variables()`). One record per ordinary statement whose left-hand
side the gate reads as a sub-object store (a named destination containing
`.`, `[` or `->`, or `names_a_sub_object`), written after the classifier
decided: the outcome, the declaration labels, `may_bind`/`never_bound`, the
body's owned and derived result names, the tracked descriptor names, the
destination chains, every value-side callee with its contract result, and the
rule's own reader answer. The statement census (`QCORE_STATEMENT_CENSUS`) is
collected alongside, to know every gate refusal on a line.

Every census run reproduces its tree's five artifacts
([census-identity.json](census-identity.json) for the unwired tree,
[census-after-identity.json](census-after-identity.json) for AFTER).

### 1.2 By stored value and by shape

[census/store-census-rejoined.txt](census/store-census-rejoined.txt) (the
census of the unwired tree `6bf77d1a`) splits the 6,912 gate-refused records in
Unknown functions ("freed alone": sole-blocked functions whose every gate
refusal is in the class):

| Stored value | Records | Functions | Freed alone |
|---|---:|---:|---:|
| scalar | 2,248 | 619 | 12 |
| null | 872 | 422 | 1 |
| a read through memory (`q->x`) | 861 | 379 | 13 |
| uncontracted call | 707 | 391 | 3 |
| pointer the function may bind | 617 | 391 | 5 |
| string literal | 337 | 81 | 0 |
| owned call result | 302 | 194 | 0 |
| call with a `None` result | 213 | 97 | 0 |
| pointer never bound | 187 | 75 | 0 |
| owned or derived name | 175 | 99 | 6 |
| tracked descriptor | 0 | 0 | 0 |

No refused sub-object store stores a tracked descriptor name: the descriptor
guard already refuses such a statement under its own description (W43's typed
route reads `h->fd = fd` as data and the guard refuses it).

By shape: `p->f` 2,706, `(p)->f` 880, `(&s)->f` 712, `p->a.b` 511,
`p->a.b.c` 383, `s.f` 326, `==` misread as assignment 231
(`W44-FIRST-EQUAL-IS-A-COMPARISON`), `a[i]` 134, and a long tail.

The rule's reader answers: member unresolved (an incomplete record) 2,141;
chain or nested aggregate 1,101; function declarations unreadable 1,101; root
resolves to no struct 521; cast or `(*p)` prefix 335; value shape not read 405;
read 1,077 (scalar 348, untracked pointer 384, named pointer 157, call 188).

### 1.3 The frozen forecast

The frozen forecast (`census/forecast.json`) used a unit join that dropped one
unit sequence whose functions include a header helper published by another unit
(`store_census.join_units`, fixed before the attribution; rejoined figures in
`census/forecast-rejoined.json`). It forecast the rule as first designed, with
the escape.

| Forecast | Frozen | Rejoined | Measured (final rule) |
|---|---:|---:|---:|
| gate leaves leaving the family | 106 – 437 | 112 – 448 | **223** |
| Unknown → Safe | 6 – 12, named | same | **1** (`destroy_id`, named) |
| unit moves | 0 | 0 | 0 |
| leak | 37 | 37 | 37 |
| violations | 0 | 0 | 0 |

The misses are the two decisions of §3: five named movers store a formal's
pointer into a record and six store an owned instance into the caller's
record, and both now keep the refusal. The first AFTER run hit the forecast's
movers closely (16 Safe) and the second (escape, no formals) moved 8; both
are described in §3.

## 2. The rule

### 2.1 Reader (`crates/qcore-c-gateway/src/legacy_sub_object_store.rs`)

`read_destination` accepts exactly one member selection:

* `R -> F`, `( R ) -> F`, `R [ I ] . F`, `( R ) [ I ] . F` through a pointer
  root `R` (`T *`, or a formal `T R[N]`), which keeps W43's dereference
  obligation;
* `S . F`, `( S ) . F`, `( & S ) -> F` and a local `A [ I ] . F`: a store into
  the function's own record, which dereferences nothing.

`T` must resolve to one `struct` tag (`RecordRoots`, over the same
total-or-reject declarator reader as `FunctionTypes::read`, through typedef
chains and a typedef that defines its own tagged struct; the side reading
`TypedAccessModel::record_tag` changes no `DeclaredType` answer). `F` must be a
scalar or single-pointer member of a complete, unambiguous, non-union record.
The index must be a scalar that writes nothing.

`read_value` reads, into a scalar member, W43's scalar walker (its accesses
carry their own obligation); into a pointer member, the null constant, a string
literal, a declared **non-formal** name (optionally cast), or one whole call
reclassified as `__qcore_sub_object_value__ = CALL` and read only when its
contract borrows every parameter and states a `None` result.

The classifier consults the rule only for the gate's own refusal, after W43's
typed route and W44's chain route declined, never for a statement that reads a
tracked descriptor (it keeps its refusal byte for byte).

### 2.2 Transfer

Every premise is decided on the state at the node; a failing premise makes the
node exactly its **fallback**, the gate's refusal, so the refused hand-on rule
still withdraws the leak claim of whatever it names:

1. the base resolves as W43 requires, a pointer member's base carries no
   tracked target (only O04-B08 rewrites a field link), and every scalar value
   base resolves; a base an out-parameter contract sanctions refuses;
2. a scalar, null or literal value is custody-neutral, and the member's place
   binding is forgotten;
3. a named or call value (the call applied first) is admitted only when it is
   unbound on every reaching path, or bound with no target and not unknown;
   anything with a target — owned, derived, a caller's site, a descriptor — or
   an unknown binding is the fallback (a refused stored call also marks its
   temporary's identities unresolved, since the hand-on reader never sees the
   temporary).

The settled-state passes examine a stored call through `element_call_view`
(`call_runs`), so its borrowed actuals keep their liveness precondition.

### 2.3 Composition

* **Constraint 5i.** The destination is the member's place; no name is bound
  (Lean `an_admitted_store_rebinds_no_other_name`). The brief's counterexample
  `recs[i].name = x_dup(s); x_free(name);` stays Unknown with
  `[GATE, unknown_alias:name]`, through a local array and through a caller's.
* **Descriptors** are custody: a statement reading one never reaches the rule;
  `admits_descriptor_read` refuses the variant explicitly
  (`a_descriptor_stored_into_a_member_stays_custody`,
  `a_descriptor_stored_through_a_parenthesised_base_keeps_the_gate`).
* **Refused hand-on.** The fallback is the gate refusal byte for byte.
* **Derivations, `B[X++]`, collections**: a derived value is never stored, a
  subscript that writes refuses, a collection owner is a tracked target.
* **Publication.** A file-scope root never resolves; CTWIN-PUBLISH stands.

### 2.4 Lean (`proof/QCore/SubObjectStore.lean`)

Built on `QCore.IndirectScalarAccess`'s state and obligation: an admitted store
changes no status, marker or custody map (`an_admitted_store_moves_no_custody`),
forgets the member and rebinds no other name; an unresolved base or a tracked
base under a pointer member refuses; any value bound to a target keeps the
refusal (`a_stored_identity_keeps_the_refusal`), an unknown one too, an unbound
one is neutral; the classifier reads no formal's pointer and no call result
carrying custody; the fallback changes no status and marks the stored identity;
non-vacuity on a concrete state. Premises, not theorems: the Rust readers
transcribe the definitions; a member store writes that member only (the C
object model under constraint 10a's no-undefined-behaviour reading).

## 3. Three arbiter runs and the custody decision

The rule was first implemented as the brief sketched it: an owned identity
stored through a formal the caller reaches **escapes**.

1. **First AFTER run** (`e31926ec`, not archived; `measure/after1/`): 16
   Unknown → Safe and **3 Unknown → provisional Unsafe**, adding 3 leaks, 4
   `release_after_escape` and 2 `custody_conflict`. Read one by one, every
   added accusation was an artifact:
   * `cmt_labels_add_kv` (leak of `l`) and `mk_server_listen_init` (leak of
     `listeners`): the rule admitted `next->prev = new` inside `__cfl_list_add`
     and `list->next = list` inside `mk_list_init` (an unbound formal is
     custody-neutral for the body). Their derived W4 summaries then called the
     stored formal `Borrowed`, and the callers' objects, linked into a list,
     were reported leaked. **Repair**: a formal's pointer is never a stored
     value.
   * `cio_file_open` (`ch->backend = cf;`, then `free(cf)` on error paths):
     `release_after_escape` and `custody_conflict` on C that may clear the
     member first.
2. **Second AFTER run** (`e91bf3e6`; escape kept, every name of an escaped
   identity made unknown): 8 Unknown → Safe, 0 provisional Unsafe, leak
   unchanged, every delta attributed. But the standing suite failed two pins:
   O04-B08's `a_store_into_a_borrowed_in_aggregate_is_never_admitted` ("a
   transfer into it is invisible at the call site") and W44-LEAK-PIN-AUDIT's
   `a_refused_store_of_the_identity_withdraws_the_leak`. The pins are right:
   storing an owned identity into the caller's record moves an obligation to
   the caller that no summary mode states. The six `create_*_instance`
   movers of that run would have given their callers validated summaries
   (`map` borrowed, result none) that drop the new `map->parent` child; a
   caller's `Safe` would rest on a transfer it cannot see.
3. **Final rule** (`a175403b`): no store moves custody. A value bound to any
   tracked identity keeps the gate's refusal, which hands it on: no leak is
   claimed and the function stays Unknown. The brief's counterexample "an
   owned pointer stored into a caller-visible record escapes" holds in that
   sense — it escapes the leak claim — and is pinned
   (`an_owned_pointer_stored_into_a_caller_visible_record_escapes_the_leak_claim_only`).
   Admitting it needs a summary mode that states the transfer
   (`W45-SUB-OBJECT-STORE-OWNED-VALUE`).

## 4. Arbiter

The runner is [scripts/run_audit.py](scripts/run_audit.py): PIN
`/home/cosmo/Documents/GitHub/qcore-c/build/cycle19`, `QCORE_CLANG=/usr/bin/clang`,
the wave-20 ChunkIO package, default jobs.

* **BEFORE** ([before-run.json](before-run.json)) reproduces the baseline
  `docs/reports/w44-collection-allocation/integration/artifacts/`: diagnostics,
  chunk-contracts and source byte for byte, contracts after the checkout-root
  alias, source-effects differing only in checkout-path digests
  ([before-identity.json](before-identity.json)).
* **AFTER** ([after-run.json](after-run.json)) at `a175403b`; the census run
  of the same code (`efca9100`, a documentation commit later;
  [census-after-run.json](census-after-run.json)) reproduces its five
  artifacts byte for byte. Archived as the new baseline under
  [artifacts/](artifacts/).

### 4.1 Attribution

[scripts/attribute.py](scripts/attribute.py) →
[attribution-summary.json](attribution-summary.json), leaf rows in
[attribution-leaves.json.gz](attribution-leaves.json.gz):
`attribution_complete: true`, 0 unattributed.

| Class | Leaves | What it is |
|---|---:|---|
| `store-admitted` | −223 unsupported | the gate refusal at a line whose every gate statement is now an admitted sub-object store |
| `callee-summary` | −7 unsupported | `call to uncontracted function rd_tmpabuf_new` in 7 librdkafka functions: the static helper's stores are admitted, so it gains a validated W4 summary |

Nothing is added. 1,054 statements are lowered (489 untracked pointer, 457
scalar, 94 named pointer, 14 call); the ones still on a line with a gate
refusal (at most 70 named-pointer, 314 untracked and 215 scalar, on 52, 71 and
75 Unknown functions) either fall back in the flow or share their line with
another refused statement.

### 4.2 Verdict changes

| Function | Unit | Move | Reading |
|---|---|---|---|
| `destroy_id` | `ctr_encode_opentelemetry.c` | Unknown → Safe | `id.len = 0; if (id.data) { id.data = NULL; }` on a by-value record parameter: custody-neutral |

No function moves to provisional Unsafe; no unit moves. Every leak, violation,
custody-conflict and summary-conflict leaf is identical before and after: no
previously correct leak is withdrawn. No `Safe` rests on a store that could
move custody unseen: an admitted store changes no status (Lean
`an_admitted_store_moves_no_custody`), and every value with a target, and every
formal's pointer, keeps the refusal.

## 5. Validation

In this worktree (`agent-ad637c4665a18b9ca`) with `CARGO_INCREMENTAL=0` and
`QCORE_CLANG=/usr/bin/clang`; logs in the private `measure/gate/`. Figures in
[gate.json](gate.json).

| Step | Exit | Result |
|---|---:|---|
| `git submodule update --init --recursive` | 0 | |
| `python3 scripts/run_test_binaries.py` | 0 | **5,669 passed / 0 failed / 11 ignored**, 290 units, 300 summaries (`private/test-binaries/20260925T162153Z-3854141`) |
| `cargo fmt --all -- --check` | 0 | clean |
| `cargo clippy --all-targets -- -D warnings` | 0 | clean |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `python3 scripts/check_lean_audits.py --jobs 4` | 0 | **190** audits (adds `AuditSubObjectStore`) |
| census feature: `cargo clippy -p qcore-c-gateway --features indirect-census --all-targets -D warnings`, `cargo test … --features indirect-census --lib` | 0, 0 | 537 passed |
| `build_matrix.py . --gate gate.json`, then `python3 -m unittest discover -s scripts/tests` | 0, 0 | **351** tests |
| `python3 scripts/build_docs_index.py --check` | 0 | after regenerating `INDEX.md` |

Kept, not validation evidence: two earlier runner attempts, both exit 101 —
`20260925T154711Z-3311261` (a build failure: `alias_aggregate_probe`'s
exhaustive match lacked the variant) and `20260925T155544Z-3446957` (5,668
passed / 2 failed: the two pins of §3). The census feature's guard test
`every_declared_sink_forces_one_job` fails on the base itself
(`QCORE_CALL_ARGUMENT_CENSUS` unlisted); the card lists the sink. `providers/`
is untouched, so the provider gate was not run; `provider_tests` in
[gate.json](gate.json) is carried. Clang is recorded in this checkout
([clang-version.txt](clang-version.txt)).

## 6. Ledger

* **Retired** ([retired-followups.json](retired-followups.json)):
  `W44-SUB-OBJECT-STORE-REFUSED`, against its condition (the stores lowered as
  the store they are, Lean parity, measured re-pin, no name bound but the
  destination); its unadmitted part is re-registered narrower.
* **Registered** ([followups.json](followups.json)), sizes from the AFTER
  census ([census/residual-after.txt](census/residual-after.txt); Unknown
  functions; "freed" as in §1.2):
  * `W45-SUB-OBJECT-STORE-MEMBER-UNRESOLVED` 2,141 / 605 / 7;
  * `W45-SUB-OBJECT-STORE-CHAIN` 1,101 / 443 / 8;
  * `W45-SUB-OBJECT-STORE-ROOT-UNRESOLVED` 521 / 171 / 24;
  * `W45-SUB-OBJECT-STORE-FORMAL-VALUE` 68 / 50 / 5;
  * `W45-SUB-OBJECT-STORE-OWNED-VALUE` at most 70 / 52 lowered stores that
    fall back, plus every owned call result;
  * `W45-SUB-OBJECT-STORE-VALUE-SHAPES` 590 / 285;
  * `W45-SUB-OBJECT-STORE-PREFIX` 335 / 148 / 4.
* **Unchanged**: `W44-CALL-RESULT-INDIRECT-DESTINATION` (14 call-result stores
  with a `None` result are lowered; the rest are its own),
  `W44-FIRST-EQUAL-IS-A-COMPARISON` (231 records), and the 1,101 records in
  functions whose declarations the reader refuses (the reader's own residuals).
* `corpus.cycle19_raw_units` points at this report's artifacts; its previous
  contents are in [previous-corpus-row.json](previous-corpus-row.json)
  ([scripts/update_ledger.py](scripts/update_ledger.py)).

## 7. For the integrator

* **Likely conflicts**: `crates/qcore-c-gateway/src/legacy_c.rs` (the variant,
  the transfer arm, two classifier hooks in `classify_simple_unguarded`, the
  census hook at the statement walker, one `Parser` field and its
  construction, one-line arms in `instruction_produced_sites`,
  `offset_produced_sites`, `element_call_view` and `transfer_base`);
  `crates/qcore-c-gateway/src/lib.rs` (a module and two census sink entries);
  `typed_field_access.rs` (`record_tag`, added to the model's impl);
  `indirect_census.rs` (two helpers made `pub(crate)`);
  `legacy_descriptor_custody.rs`, `legacy_joined_provenance.rs` (one arm each);
  `compiler/examples/alias_aggregate_probe.rs` (one arm); `proof/QCore.lean`
  (one import); `docs/agents/gateway.md` (a paragraph under 5i);
  `docs/STATUS.md`, `docs/reviews/r0_3/acceptance_matrix.json`,
  `docs/development/increments/wave-45.md`, `docs/reports/INDEX.md`.
* **Base guard fix**: base `b160d765` fails the census feature's own guard test
  (`every_declared_sink_forces_one_job`: W44-INDIRECT-CALL-ARGUMENT-ACCESS's
  `QCORE_CALL_ARGUMENT_CENSUS` was not listed). This card lists it
  (`9168566b`); if another card lists it too, keep one.
* **Composition**: a card that widens the declaration reader (typedef or
  record-member resolution) widens this rule's reach; a composed arbiter run is
  needed. A card that adds a summary mode for a formal's record receiving a
  child discharges `W45-SUB-OBJECT-STORE-OWNED-VALUE` and
  `-FORMAL-VALUE` together.
