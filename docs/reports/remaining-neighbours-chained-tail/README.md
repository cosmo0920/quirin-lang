# NEIGHBOURS-CHAINED-TAIL — a bounded typed access chain at a declared variadic data tail — 2026-09-10

**Card:** `NEIGHBOURS-CHAINED-TAIL`, the follow-on card
[`docs/reports/remaining-neighbours/README.md`](../remaining-neighbours/README.md)
§8 opened and sized at one actual.
**Base:** main `8069a99` (wave-15 consolidation).
**Branch:** `worktree-agent-a1233907e18fdc1ad`. Nothing is pushed or merged.

---

## Headline

Wave 15 isolated the ONE surviving fixture-route refusal on the byte-verbatim
upstream `flb_input_chunk_create`: a tail actual `storage->stream->name`, on the
`flb_error` arm of the `!chunk` exit. The same refusal is the pinned corpus's
`flb_input_chunk.c` **L19581** — the one blocker the two routes share.

This card admits it, fail-closed, as clause **T5** of the variadic-tail walker:
*a tail actual, at a position the callee's declaration covers with
`resource_variadic_data_tail()`, that is **exactly** one admitted bounded typed
access chain, is data.* The chain half is
[`borrowed_actual::is_admitted_access_chain`] — **the same predicate, over the
same declaration facts, that already decides a declared `Borrowed` actual**
(cards C02-P and O04-ARMS). The only thing the tail position contributes is the
declaration, and the declaration is read from the callee, never from the actual.

| | before | after |
|---|---|---|
| **185-unit corpus** | 2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only | **unchanged, every library, every unit** |
| corpus diagnostics | leak 271, unknown_alias 4403, unsupported 49896 | leak **271**, unknown_alias **4403**, unsupported **49895** |
| corpus leaf diff | — | **exactly one leaf removed, zero added**: `flb_input_chunk_create` L19581 |
| unit / function status moves | — | **0 / 0** |
| fixture leg, constructor | 1 refusal | **0** |
| Lean audits | 67 | **68** |

**Nothing is promoted.** No raw unit moves in either direction, no newly
surfaced provisional `Unsafe`, no function status move, and `flb_input_chunk.c`
keeps its provisional `Unsafe`. `flb_input_chunk_create` stays **Unknown** in
production on its twenty other residuals (§4.3).

---

## Three corrections this card measured, stated before anything else

### C1. The cause was **not** a pointer-typed final field

NEIGHBOURS §3.4 wrote that "Card O04-ARMS' chained-access admission requires
every link to be declared exactly `struct P *`, and this chain's **final field is
pointer-typed**, so the chain rule does not reach it."

**That is not the mechanism.** `TypedAccessModel::borrowable_field` has admitted
a **pointer** member since card C02-P — `read_record_body` inserts every
unqualified member with `stars >= 1` into `borrowable_fields`, beside the scalar
ones — so `admitted_access` accepted `storage->stream->name` all along. The
measurement that settles it: the depth-one **scalar** access `storage->flag`,
squarely inside card C02's original whitelist, refused in a tail position
**identically**, with the same diagnostic. So did `storage->leaf` and
`storage->leaf->n`. Every member access refused, at every depth, with any final
type.

The real blocker was premise **T2** of `variadic_nested::is_data_only_actual`,
which refused `&`, `*`, `[`, `]`, `.` and `->` in a tail actual outright — and,
because a tail actual's span was never claimed by any whitelist, the **lexical
indirect-access gate in `legacy_c` fired first**, reporting `indirect memory
access` before the tail rule was ever consulted. Both halves had to move.

The correction is pinned as `a_depth_one_scalar_field_was_refused_in_a_tail_too`
and modelled in Lean as `a_depth_one_scalar_chain_is_widened_the_same_way`, so
it cannot be quietly re-acquired.

### C2. The brief's closure requirement was measured unnecessary **on this route**, and the reason is structural

The brief asked that "a name derived from the admitted read joins
`untracked_bases` or the equivalent, so any later tracked use refuses."

**No name derives from an admitted tail read.** A tail actual is an *argument*,
never a destination: it is consumed by the statement and bound to nothing. The
shape that *does* bind — `x = base->f1->f2;` — is a **pre-existing, unmodified
route**: `borrowed_actual::is_opaque_field_read` produces
`LegacyInstruction::OpaqueValue`, and releasing an opaque-bound name still
reports `UnknownAlias`. This card did not touch that route; it retained it and
re-pinned its containment
(`an_opaque_bound_name_is_still_never_released_silently`).

What the closure *does* do here is refuse the **base**: `untracked_bases` is
consulted unchanged inside `admitted_access`, so a released base, a body-acquired
aggregate, a laundered name and every protected formal each refuse the chain at
its root, at every depth (§3.3, rows R11–R13).

### C3. The pinned NEIGHBOURS headline test moved in the admitting direction

`the_constructor_keeps_exactly_one_refusal_under_production_declarations` pinned
**exactly one** surviving refusal. That refusal is what this card admits, so the
fixture leg now reaches **zero**. The pin is updated **deliberately and loudly**,
with all three of its load-bearing negative halves kept and one added (§5).

---

## 1. What is admitted

### 1.1 The rule

`crates/qcore-c-gateway/src/variadic_nested.rs`, clause **T5**. At a call whose
callee has a contract in force that declares `resource_variadic_data_tail()`, an
actual at an index **beyond** the declared parameter vector is data when it is
**exactly** one admitted bounded typed access chain:

* `base -> f1`, `base . f1`, `base -> f1 -> f2`, up to
  `borrowed_actual::ACCESS_CHAIN_LIMIT` = **4** member selections;
* every selection before the last written `->` on a member declared *exactly*
  `struct POINTEE *`, so the walk re-enters the declaration model at `POINTEE`;
* the last selection a scalar or pointer member with no array modifier and no
  `volatile`/`_Atomic`, of a `struct` defined exactly once in the unit whose
  every member declaration was read;
* the access not continuing into `->`, `.`, `[` or `(`;
* the base a name `untracked_bases` says this body can **never** bind to a
  tracked resource identity.

That is `borrowed_actual::is_admitted_access_chain` — the C02-P premises (1)–(4)
as O04-ARMS widened them, called from a second place. Not one premise is new,
relaxed, or re-implemented.

`tail_access_spans` returns the spans of those actuals so the **lexical gate**
exempts them, exactly as it already exempts a declared `Borrowed` actual. The
spans are whole arguments by construction, so exempting them exempts exactly the
arguments the premises were checked against and nothing else.

### 1.2 The soundness argument

The claim is that evaluating an admitted tail actual neither changes nor depends
on the abstract state, so modelling the call under its declared parameter vector
alone is sound.

1. **The read premises are identical to the declared-`Borrowed` case.** The same
   function, over the same `TypedAccessModel`, `FunctionTypes` and
   `UntrackedBases` this body already computed. Whatever makes C02-P's admission
   sound at a `Borrowed` position makes this sound at a tail position, and
   nothing more is asserted.
   Proved in the model as
   `the_chain_half_is_exactly_the_borrowed_admission_at_depth_one`.
2. **The position is at least as safe as `Borrowed`.**
   `resource_variadic_data_tail()` is the trusted assumption that the callee
   takes **no custody of, and retains no pointer to**, any argument beyond the
   ones the declaration names. `resource_borrowed` says the caller's obligations
   are unchanged across the call. For custody purposes the tail statement is not
   weaker. This is the same premise card O04-TRACE already used for an admitted
   nested call's *result* (premise N8: "a declared `Borrowed` parameter, **or a
   declared data tail**") and for every scalar tail actual since.
3. **The value carries no identity this projection tracks.** The base is refused
   if it is in `untracked_bases`; no place along the chain is a place this
   projection binds, because the *destination* side of the lexical gate refuses
   every `->`/`.`/`[` destination except card O04-B08's admitted aggregate store,
   whose base is risky by the acquisition that bound it. Proved in the model as
   `an_admitted_chain_names_no_bound_base`.
4. **Nothing is bound.** See correction C2.
5. **The declaration is read from the callee.** `tail_access_spans` takes
   `variadic_tail` from `declared_variadic_tails(contracts)` and `named` from the
   contract's own parameter vector; no property of the actual can produce either.
   Proved as `the_declaration_is_read_from_the_callee_never_from_the_actual`.
6. **Exempting a span is not admitting the call.** The tail rule still requires
   **every** tail actual of the call to be data, so a sibling actual naming a
   tracked identity refuses the whole statement even when the chain beside it was
   exempted. Pinned in Rust
   (`a_sibling_tail_actual_carrying_an_identity_still_refuses`) and in Lean
   (`an_admitted_chain_beside_a_risky_sibling_still_refuses`).

### 1.3 What the argument does **not** establish

Recorded plainly, because the residual risk is real and inherited.

* **CHAINED-TAIL-NO-LIVENESS.** An admitted chain denotes no place the projection
  tracks, so it gets **no liveness precondition**. Card BORROW-LIVENESS refuses a
  borrow of a *settled denotation* that was released; it has nothing to settle
  here. A caller that passes, in a tail position, a pointer read out of an
  aggregate this body does not track, whose pointee was freed through some
  other name, is a use-after-free this route neither creates nor detects. That is
  **exactly** the non-claim `contract/qcore/fluent_bit/logging.qcore` already
  records in its own words, and it is unchanged by this card: the same read is
  already admitted at a declared `Borrowed` position with the same silence.
* **CHAINED-TAIL-ALIAS.** The name-based projection relates no two distinct
  declared names that may alias — card C02-P's recorded **C02-P-ALIAS**,
  inherited unchanged and not widened.
* **CHAINED-TAIL-READER.** The reader's type extraction and C effective types
  remain premises (card C02), assumed, not proved.
* **CHAINED-TAIL-BOUND-BASE.** Card O04-READ-BOUND admits a member read through a
  base *this body bound*, with the linked children's liveness required
  flow-sensitively. `bound_read_actuals` reads `contract.parameters.get(position)`
  and therefore covers **declared** positions only; a tail index yields `None` and
  is skipped. This card deliberately did **not** extend it, so the one shape where
  an admitted field could hold an identity this body stored there simply does not
  arise on the tail route: a body-acquired aggregate base is risky and refuses at
  premise (4). Pinned as `a_body_acquired_aggregate_base_is_refused_in_a_tail`.
* **CHAINED-TAIL-N9.** Premise **N9** — a nested call's own inner actuals — is
  deliberately **not** widened. `y_id(storage->leaf)` in a tail keeps its refusal.

---

## 2. The change, file by file

| Path | Change |
|---|---|
| `crates/qcore-c-gateway/src/borrowed_actual.rs` | `is_admitted_access_chain` is named as the shared primitive (the whole slice is one admitted chain); `is_opaque_field_read` now delegates to it, semantics byte-for-byte unchanged |
| `crates/qcore-c-gateway/src/variadic_nested.rs` | clause **T5**: `is_admitted_tail_access` and `tail_access_spans`; `tail_arguments_are_data` gains the disjunct; module header records what stays refused. **Premise N9 and `is_data_only_actual` itself are unchanged** |
| `crates/qcore-c-gateway/src/legacy_c.rs` | the admitted tail spans join `borrowed_actuals` before the lexical `unsupported_access` gate, the same way a C02-P borrowed actual, an O04-READ-BOUND bound read and an O04-B09B10 member address already do |
| `crates/qcore-c-gateway/tests/chained_tail.rs` | **new** — 12 assertions over a held-out generic `y_*` family |
| `.../tests/fixtures/contracts/chained_tail_generic.qcore` | **new** — the held-out `y_*` declarations |
| `crates/qcore-c-gateway/tests/neighbours.rs` | the headline pin re-pinned deliberately, with an added narrowness half; the superseded probe's doc corrected (§5) |
| `proof/QCore/ChainedTailActual.lean` | **new** — 28 theorems |
| `proof/AuditChainedTailActual.lean`, `proof/QCore.lean` | **new** audit; module registered |

Three production functions, 32 added lines in `legacy_c.rs`, and **no** existing
premise deleted, relaxed or rewritten.

---

## 3. Evidence

### 3.1 The admission — 5 assertions

| Test | What it decides |
|---|---|
| `a_two_link_pointer_tail_chain_is_admitted` | the subject shape, generically: zero refusals, zero diagnostics |
| `a_depth_one_scalar_field_was_refused_in_a_tail_too` | correction **C1**: `storage->flag`, `storage->leaf` and `storage->leaf->n` are all admitted now, and the module doc records that none of them was before |
| `a_record_valued_base_and_two_chains_are_admitted` | a single `.` on a record-valued base; two admitted chains in one argument list |
| `the_depth_bound_is_inherited_unchanged` | four selections admitted, five **refused, not truncated** |
| `a_certificate_resting_on_the_admission_is_rederived` | `check_legacy_safety_certificate` independently rederives a `Safe` that rests on this admission from the certificate's own content |

### 3.2 The refusal inventory — 12 shapes, one substitution each

`every_non_conforming_tail_shape_keeps_its_refusal` drives each row into the
*same* statement, so nothing but the actual differs from the admitted case.

| # | Actual | Ground |
|---|---|---|
| R1 | `&storage->leaf->label` | not a whole chain — address escape |
| R2 | `storage->leaf->label[0]` | subscript |
| R3 | `*storage->leaf->label` | dereference |
| R4 | `storage->flag + 1` | chain mixed with arithmetic |
| R5 | `storage->flag = 1` | **mutation through the tail** |
| R6 | `uh->view->leaf` | `union` intermediate link |
| R7 | `bh->bits->label` | a record the reader refuses (bitfield member) |
| R8 | `td->leaf->label` | intermediate link is a typedef the reader will not resolve |
| R9 | `hold->v.k` | continued with `.` across a nested aggregate |
| R10 | `top->deeper->deep->mid->leaf->label` | five selections, past the inherited bound |
| R11 | `storage->self->flag` **with `&storage` elsewhere in the body** | premise (4): the base is laundered risky. The same row **without** the address-of is asserted admitted, so the launder is load-bearing |
| R12 | `y_id(storage->leaf)` | premise **N9** unchanged. *Measured*: the lexical gate reports first, so the recorded diagnostic is the indirect-access one rather than O04-B09B10's nested-call wording. Both are the same `Unknown`; the load-bearing fact is that the shape is refused |

Plus four separately driven refusals:

| Test | What it decides |
|---|---|
| `a_released_base_is_never_read_through_in_a_tail` | the use-after-free shape the brief names: a base handed to a declared `Released` position is seeded risky, so reading through it in a tail refuses and never reaches `Safe` |
| `a_body_acquired_aggregate_base_is_refused_in_a_tail` | **CHAINED-TAIL-BOUND-BASE**, asserted rather than assumed |
| `the_position_is_decided_by_the_declaration_not_the_actual` | the identical actual at a `Consumed` position, and at a tail of a callee with **no** declared tail, both keep their refusals |
| `a_sibling_tail_actual_carrying_an_identity_still_refuses` | exempting a span is not admitting the call |

And two containment assertions:

| Test | What it decides |
|---|---|
| `an_opaque_bound_name_is_still_never_released_silently` | correction **C2**: the binding shape keeps its own route and its own containment |
| `the_borrowed_route_is_unchanged` | the declared `Borrowed` route this card mirrors still admits what it admitted |

Three further unit tests in `variadic_nested.rs` decide T5 without Clang:
`a_typed_chain_on_an_unbound_base_is_tail_data`,
`only_a_whole_admitted_chain_is_tail_data`, and
`tail_spans_are_claimed_only_under_a_declared_tail` (the claimed span set is
exactly the one chain argument, whole; empty with no declared tail; empty at a
named position).

The pre-existing
`an_address_a_dereference_a_subscript_or_a_member_is_never_data` is **unchanged
and still passes** — its `b -> flag` and `b . flag` rows refuse because `b` is
bound by an opaque call in that fixture, and its doc now records that reading
rather than leaving it to be misread.

### 3.3 Lean — 28 theorems, `QCore.ChainedTailActual`

`AuditChainedTailActual.lean` brings the audit count **67 → 68**. No new axiom,
no placeholder; every theorem depends only on `propext` / `Classical.choice` /
`Quot.sound`.

The model is `QCore.VariadicNested`'s own `TailActual`/`admitsTail` composed with
`QCore.ChainedAccess`'s `admitsChain`, so the widening is expressed as a
**disjunct on the existing rule** rather than a new rule:

* **The declaration half is untouched** —
  `the_declaration_is_read_from_the_callee_never_from_the_actual`,
  `unread_declarations_never_admit`.
* **Widening, stated as such** —
  `every_previously_admitted_actual_is_still_data` and
  `every_previously_admitted_site_is_still_admitted` (lifted over the
  un-widened model's own admission), `the_widening_is_strict`,
  `the_subject_site_is_admitted`,
  `a_depth_one_scalar_chain_is_widened_the_same_way` (correction C1).
* **Fail-closed** — eleven refusal-lifting theorems: bound base, continued
  access, past the bound, `union` / incomplete / ambiguous / qualified /
  aggregate link, non-continuable intermediate, partial chain, risky name.
* **The soundness sentence** — `an_admitted_chain_names_no_bound_base`;
  `the_chain_half_is_exactly_the_borrowed_admission_at_depth_one`.
* **Bridging** — `a_refused_site_refuses_the_statement`,
  `a_refused_site_is_never_safe`,
  `an_admitted_chain_does_not_rescue_the_statement`,
  `an_admitted_chain_beside_a_risky_sibling_still_refuses`.
* **Non-vacuity** — `each_premise_is_load_bearing_on_the_subject`, each premise
  separately flipped on the same witness.

This is a model of the **decision**, not of C semantics.

---

## 4. Corpus impact — every leaf attributed

### 4.1 Provenance

Canonical pinned compatibility route, on a clone this card prepared at Fluent Bit
`66910c10a4d7eafa810b84229cf2dcf7b0f26f97` with explicit `/usr/bin/clang`;
provider packages rebuilt for the run. Both runs use the same clone, the same
databases and the same packages. Full detail, including three itemised
deviations inherited from card CORPUS-REPIN, in
[`provenance.json`](provenance.json); transcripts in [`logs/`](logs/).

**The BEFORE run was executed with the worktree checked out at base main
`8069a99`** — `git checkout 8069a99`, run, then the branch restored — so not one
byte of this card's diff existed while it ran.

Both runs exit **0** with **zero gateway failures**.

### 4.2 The BEFORE run reproduces the pinned baseline

Against [`docs/reports/wave15-corpus/artifacts/`](../wave15-corpus/artifacts/),
the current 185-unit baseline:

| Artifact | wave-15 archived | BEFORE | AFTER |
|---|---|---|---|
| `diagnostics.json` | `1da0555bd5e3c5a6…` | **identical** | `fa45aeb52517662e…` |
| `chunk-contracts.json` | `c4f438e4ed3dab3c…` | **identical** | **identical to BEFORE** |
| `source.json` | `53d55b7e1fb174de…` | `dd637bcba41f4d30…` | **identical to BEFORE** |
| `source-effects.json` | `4001c2d8b40bc9df…` | `91e6477cb64044f3…` | **identical to BEFORE** |
| `contracts.json` | `ee5f8bcf24528a7c…` | `cc41d3aef0a36d76…` | **identical to BEFORE** |

**The verdict surface reproduces byte-identically.** The three artifacts that
differ from the wave-15 archive differ from it in the BEFORE run too — before any
of this card's code existed — and each difference was diffed leaf by leaf rather
than assumed:

* `source.json` — 51 changed lines, all of them the worktree path prefix in the
  four recorded Monkey wrapper commands, plus three `__FILE__` expansions whose
  `c_type` moves `char[147]` → `char[189]` because the path string is longer.
  **Location provenance, not analysis.**
* `contracts.json` — 27 changed lines, all of them the same path prefix on the
  twelve loaded contract-module keys. The module **texts** are unchanged.
* `source-effects.json` — 92 changed lines, all of them this run's own provider
  package identity (`package_digest`, `build_digest`, `configuration_digest` and
  the obligation key derived from them). This is the package residue waves 9 and
  13 already recorded.

**Only `diagnostics.json` moves between BEFORE and AFTER.** The other four
artifacts are byte-identical across the change.

### 4.3 The census — one leaf, and it is the predicted one

`census.py` (card NEIGHBOURS', retargeted) compares the two runs exhaustively.
Full output in [`logs/census.txt`](logs/census.txt):

```
unit status before: {'Safe': 2, 'Unknown': 137, 'Unsafe': 42, 'data_only': 4}
unit status after : {'Safe': 2, 'Unknown': 137, 'Unsafe': 42, 'data_only': 4}
unit status moves: 0
NEWLY SURFACED PROVISIONAL UNSAFE UNITS: 0
unit reason_code moves: 0
projected functions: 4834 -> 4834
function status moves: 0
  leak: 271 -> 271 (+0)
  unknown_alias: 4403 -> 4403 (+0)
  unsupported: 49896 -> 49895 (-1)
units with diagnostic-count changes: 1
  fluent-bit-chunk/flb_input_chunk.c: {'unsupported': -1}
leaf-level diff, per changed unit:
  UNIT fluent-bit-chunk/flb_input_chunk.c
    -1 flb_input_chunk_create L19581 unsupported: indirect memory access …
leaves added: 0; leaves removed: 1
```

**One leaf removed, zero added, in one function, in one unit** — and it is
exactly row 1 of NEIGHBOURS §4.3's residual table, the line that card named as
the one blocker the fixture leg and production share. Nothing else in the
185-unit corpus moved in any direction.

`flb_input_chunk_create` in production goes **21 → 20** diagnostics and stays
**Unknown**. Its twenty remaining residuals are unchanged and belong to families
this card does not touch: seven `indirect memory access` in the chained/aggregate
read family, twelve `unresolved resource identity` in the aggregate-aliasing
family, and one nullable-handback refusal. `flb_input_chunk.c` keeps its
provisional `Unsafe`.

### 4.4 The fixture leg

Measured in one run by the re-pinned headline test, so the movement is not
asserted from memory:

| state | diagnostics |
|---|---|
| the six NEIGHBOURS declarations withdrawn | **31** |
| the six in force, before this card | 1 |
| the six in force, after this card | **0** |
| the six in force, the same actual subscripted (`…->name[0]`) | **1**, the original refusal verbatim |

The fixture's zero is a **function-level** projection resting on twelve named
declared contracts, each a trusted assumption about a body read by hand, and is
never a proof of the C. `the_real_program_is_not_promoted_by_this_card` is
unchanged and still keeps the fixture *program* out of `Safe`, and §4.3 remains
this card's production authority, not §4.4 (card NEIGHBOURS'
**NEIGHBOURS-FIXTURE-PROXY**, inherited).

### 4.5 No re-pin

The global 185-unit baseline stays at `docs/reports/wave15-corpus/artifacts/`.
Both of this card's runs are archived here — `artifacts-before/` and
`artifacts/`, each with a `manifest.json` carrying every SHA-256 — so the single
changed leaf is independently reproducible. **The integrator consolidates.**

---

## 5. Pins moved deliberately — stated loudly

| Pin | before | after | Why |
|---|---|---|---|
| `neighbours.rs::the_constructor_keeps_exactly_one_refusal_under_production_declarations` | `["indirect memory access…"]`, 1 diagnostic | `[]`, **0** diagnostics | that refusal is what this card admits |

This is the only pin that moved, and it moved in the **admitting** direction, so
it is stated with its whole reasoning:

* **All three load-bearing negative halves are kept, byte-unchanged in
  substance**: the before-state is still measured in the same run at **31**
  diagnostics; all six callee-naming refusals are still asserted present in it;
  the C6 tail refusal is still asserted present before and absent after; and the
  constructor must still not leak.
* **A fourth negative half is added.** The same statement, with the same actual
  made non-conforming by a subscript the chain rule refuses
  (`storage->stream->name[0]`), must restore the **exact** measurement this test
  used to pin: one `indirect memory access`, nothing else. So the zero is this
  admission's and not a widened gate's.
* `withdrawing_the_chained_tail_actual_decides_the_constructor` is **retained
  with its assertion byte-unchanged** and marked superseded in its doc: the probe
  still applies and still decides; what it no longer isolates is a *difference*.
  Its doc also carries correction **C1**, so the mis-attribution is corrected at
  the site that made it.

**No other pin moved, in either direction.** The contract corpus stays **67**
modules; `contract/`, `compiler/` and every other crate are untouched; the
`third_party/libevent` submodule pin is unchanged; every earlier card's fixture
premise set is unchanged; no diagnostic code was added, removed, renamed or
downgraded; no schema, protocol, evidence or package version moved.

### 5.1 One consequence handed to the integrator

`scripts/tests/test_build_matrix_sections.py::test_the_derived_facts_are_recomputed_rather_than_carried`
fails on this branch, **by its own design**, with

```
AssertionError: Lists differ: ['12.0.0', 67] != ['12.0.0', 68]
```

`validation.lean_audits` is a **derived** field of
`docs/reviews/r0_3/acceptance_matrix.json` (its own `section_ownership.fields`
says so: *"derived — proof/Audit\*.lean on disk"*), and this card adds one audit
file. The tripwire is doing exactly its job: it reports that the committed matrix
is now stale by one. **The matrix is integrator-owned and this card does not edit
it.** At consolidation, `validation.lean_audits` moves **67 → 68** and the test
passes again. The other 145 Python tests pass.

---

## 6. Gate

Run on this branch, at the merged state of the card:

| Gate | Result |
|---|---|
| `cargo test --workspace` | **2653 passed, 0 failed, 8 ignored** |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **exit 0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **exit 0** |
| `python3 scripts/check_lean_audits.py` | **68 audits pass** (67 → 68), no new axiom |
| `providers/` workspace tests | **108 passed, 0 failed** |
| `providers/package.py build` | all five packages built and verified for the audit runs |
| Python `scripts/tests` | 146 tests, **145 pass**, 1 fails by design (§5.1) |
| 185-unit corpus, both runs | exit 0, zero gateway failures |

15 new Rust tests (12 integration, 3 unit) and 28 new Lean theorems.

---

## 7. Scope and non-claims

* **No raw unit is promoted**, at any level, in either direction. 2 Safe /
  42 provisional Unsafe / 137 Unknown / 4 data-only, unchanged.
* **A decided statement is not a `Safe` function, and a `Safe` function is not a
  `Safe` unit.** `flb_input_chunk_create` stays **Unknown** in production;
  `flb_input_chunk.c` stays provisional `Unsafe`; the fixture program stays
  `Unknown`.
* **A provisional `Unsafe` is never an upstream defect claim**, here or anywhere.
  The `y_*` family is synthetic and its `Safe` is `Safe` for a synthetic declared
  custody projection.
* **`resource_variadic_data_tail()` is a trusted declared assumption** about a
  callee, and this card verifies nothing about any C body. It says nothing about
  the callee being correct, nothing about the tail being type-correct against a
  format string, and nothing about the tail being read at all.
* **An admitted tail read carries no liveness guarantee** — see
  **CHAINED-TAIL-NO-LIVENESS** in §1.3. That risk is inherited from the declared
  `Borrowed` route unchanged, not created here, and it is recorded rather than
  hidden.
* **Nothing about the upstream code is alleged.** The upstream bodies are
  byte-unchanged everywhere in this card, and the one probe that derives a source
  asserts its substitution applied.
* **Unknown means Unknown.** A statement whose path carried a refusal alleges
  nothing in either direction. Projection coverage is not safety coverage;
  unknowns stay in the denominator.
* CT coverage is unchanged at **32/50**; the contract corpus stays **67**
  modules; `qcore-schemas` stays **12.0.0**; `qcore-compiler` stays **0.4.0**; no
  plugin, protocol or evidence version moved; no diagnostic code changed.

---

## 8. Named follow-ons

| # | Boundary | Status |
|---|---|---|
| **CHAINED-TAIL-NO-LIVENESS** | an admitted chain denotes no place the projection tracks, so it gets no liveness precondition; a released pointee reached through an untracked aggregate in a tail is a use-after-free this route neither creates nor detects. Inherited unchanged from the declared `Borrowed` route and from `logging.qcore`'s own recorded non-claim | **open, recorded, not created here** |
| **CHAINED-TAIL-BOUND-BASE** | card O04-READ-BOUND's liveness-checked bound read covers **declared** positions only; extending it to tails would need the flow-sensitive liveness half, which this card did not take. Measured consequence: a body-acquired aggregate base is refused in a tail, asserted | **open, deliberately not taken** |
| **CHAINED-TAIL-N9** | premise N9 — a nested call's own inner actuals — is deliberately not widened; `f(g(base->field))` in a tail keeps its refusal, and widening it would also need the lexical gate's view of nested arguments to move | **open, deliberately not taken** |
| **CHAINED-TAIL-ALIAS** | C02-P-ALIAS, inherited unchanged: two distinct declared names that may alias are unrelated by this name-based projection | **open, inherited** |
| **CHAINED-TAIL-READER** | the reader's type extraction and C effective types remain premises (card C02) | **premise, not proved** |
| **CHAINED-TAIL-MATRIX** | `validation.lean_audits` in `docs/reviews/r0_3/acceptance_matrix.json` must move 67 → 68 at consolidation (§5.1) | **handed to the integrator** |
| — | the twenty other `flb_input_chunk_create` production residuals — the chained/aggregate read family and the aggregate-aliasing family — are untouched and named in §4.3 | **open, unchanged** |
| — | a partial chain, a chain past the bound, a laundered base, a released base, a body-acquired base, a nested inner actual, a mutation through the tail, a chain at a custody position, a chain at a callee with no declared tail | **refused by construction, and asserted** |

---

## 9. DEVELOPMENT.md increment paragraph (draft — integrator-owned file, not edited here)

> - Chained tail actuals (NEIGHBOURS-CHAINED-TAIL): the one refusal wave 15
>   isolated falls. A tail actual at a **declared** `resource_variadic_data_tail()`
>   position that is exactly one admitted bounded typed access chain is data
>   (clause T5) — the same predicate, over the same declaration facts, that
>   already decides a declared `Borrowed` actual, so the position is the only
>   difference and the position comes from the callee. Three measured corrections
>   to the card premise: the cause was **not** a pointer-typed final field
>   (`borrowable_field` has admitted pointer members since C02-P, and a
>   depth-one **scalar** tail actual refused identically) but premise T2 plus the
>   lexical gate firing ahead of it; the requested `untracked_bases` closure over
>   the derived value is structurally unnecessary because a tail actual binds no
>   name, while the closure over the **base** is what refuses a released or
>   acquired root; and the pinned NEIGHBOURS headline moved in the admitting
>   direction, deliberately re-pinned with all three load-bearing negative halves
>   kept and a fourth added (the same actual subscripted restores the original
>   refusal verbatim). The 185-unit corpus was measured before and after through
>   the canonical pinned route, with the BEFORE run executed at base: the headline
>   is unchanged at 2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only,
>   zero unit and zero function status moves, leak unchanged at 271, and the whole
>   diff is **one leaf removed and zero added** — `flb_input_chunk_create` L19581,
>   exactly the line NEIGHBOURS predicted. `flb_input_chunk_create` stays Unknown
>   on twenty untouched residuals and nothing is promoted. 28 audited theorems
>   (`QCore.ChainedTailActual`, audits 67 → 68); CHAINED-TAIL-NO-LIVENESS is
>   recorded as inherited, not created. Evidence:
>   `docs/reports/remaining-neighbours-chained-tail/README.md`.
