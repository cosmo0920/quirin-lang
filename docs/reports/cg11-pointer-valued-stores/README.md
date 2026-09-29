# CG11-POINTER-VALUED-STORES

Card CG11 of the [Clang gap queue](../../plans/clang-gap-cards/README.md), wave
48 round 3. It works on route G (the custody gateway) and covers entries M02,
M03 and M04. It is the [unsupported coverage queue](../../plans/unsupported-coverage/README.md)'s
FIELD-CUSTODY (P5), with evidence Q18 and Q36 (example C).

- **Rule.** Gateway rule [10v](../../agents/gateway.md#constraint-10v).
- **Lean.** `QCore.PointerStore`
  ([`proof/QCore/PointerStore.lean`](../../../proof/QCore/PointerStore.lean)),
  audited in `proof/AuditPointerStore.lean`.
- **Base.** `f19204fa`, which is local `main` and `origin/main` (round 2
  landed).
- **Prediction.** [PREDICTION.md](PREDICTION.md), frozen at `96469510` before
  any analyzer change was committed and before any AFTER run.

## 1. What changed

- **The rule** ([`pointer_store.rs`](../../../crates/qcore-c-gateway/src/pointer_store.rs),
  flow in [`legacy_pointer_store.rs`](../../../crates/qcore-c-gateway/src/legacy_pointer_store.rs)).
  - **Where it reads.** The typed statement route reads a refused `P = V`
    whose place `P` has pointer type (object or function pointer). It is
    asked only after every older route declined (hard invariant 3). Elaborator
    pieces, `*slot` stores (the out-slot refusal is not consulted) and element
    stores an older route decides are not read.
  - **The destination** is a CG10 place (`typed_place::read_place`). Its
    per-hop obligations are the place's own: `Reader::path` reads it as it
    reads a scalar store, through a `pointer_store_target` flag that lifts
    only the scalar-type requirement.
  - **The value** is classified from the AST. The six classes that carry no
    identity are admitted:
    - the null constant;
    - a string literal;
    - a function designator or `&f`;
    - a name of function-pointer type, whose declaration id must be the one
      every spelling in the function refers to;
    - the destination's own object: the root pointer itself (`list->next = list`)
      or `&` of a record root;
    - the address of a place in that object read through no pointer but the
      root's (`&(&q->head)->tqh_first`).

    Everything else refuses by name: a formal's pointer, a local's pointer, a
    pointer read, a call, arithmetic, another object's address, and a cast
    other than a qualification conversion.
- **The four events, kept apart** (flow, `LegacyInstruction::PointerStore`).
  Every failing premise makes the node exactly its fallback, the gate's
  refusal, so a store that stays refused behaves byte for byte as before.
  1. **Value store.** The destination's access is applied by the ordinary
     transfer with its chain obligations.
  2. **Alias and overwrite.** Every binding of a place rooted at the
     destination's root is forgotten afterwards. The value adds none.
  3. **Old obligation.** `chain_owner`, a behaviour-preserving split of
     `chain_resolves` that returns the final owner, must read both the root and
     the member's object as untracked. No binding of a place rooted there may
     carry a target or be unknown. A tracked owner refuses (`owned-field`),
     because only O04-B08's field-link store rewrites a field an owner may
     hold.
  4. **Escape.** The admitted value carries no identity, so nothing escapes.
     A function-pointer name must have no binding with a target on any
     reaching path.
- **Summaries.** `observations_match_contract` refuses a body with an admitted
  pointer store (`has_pointer_store`), beside the foreign-unlink gate. A caller
  would read `Borrowed` for the formal whose object the body overwrote: the
  `mk_list_init`/`cfl_list_add` hazard of rule 5i.
  - While a summary is derived the formal is bound to its caller's site, so
    its object is a tracked owner and the store refuses anyway. The corpus
    shows this: 13 statements are admitted in the reported analysis and
    refused in an entry analysis.
  - The gate states the decision rather than relying on that. Bodies that
    were Unknown before had no summary, so no caller loses one.
- **Hooks.** These are thin and additive:
  - `typed_access_path.rs`: `Reader` fields `pointer_store_target` and
    `pointer_stores`, set only by `statement_with`; accessors; the
    `StatementPlan.pointer_store` field; the hook in `Reader::statement`.
  - `legacy_c.rs`: the `PointerStore` variant, the transfer arm,
    `writes_scalar`, the fallback carried by `typed_path_statement`, and
    parity.
  - `legacy_summary.rs`: the gate.
  - `legacy_joined_provenance.rs` and `legacy_descriptor_custody.rs`: the new
    variant.
  - `indirect_chain_access.rs`: `chain_owner`, with `Owner` made
    `pub(crate)`.

  **Shared items added**, for the integrator composing CG03 and CG13:
  - the `LegacyInstruction::PointerStore` variant;
  - `StatementPlan.pointer_store`, and `Reader.pointer_store_target` /
    `pointer_stores`;
  - `indirect_chain_access::chain_owner`.

  `elaboration.rs` and `typed_place.rs` are not changed: no `FORM_TABLE` row
  and no place decision moved.
- **Parity.** `position: "pointer-store"` rows record each classification
  (place, value class, admitted). `position: "pointer-store-flow"` rows record
  each flow decision (value, owner, decision).

## 2. Cited claims

| Claim | Outcome |
|---|---|
| M02 (Q36): `s->p = q`, `*slot = p`, `slots[i] = p` are refused | **reproduced** at base (CG00's `m02_…` control). It stays refused after the card, since each is a formal's value, an out slot, or an element route decision. |
| CG00's M02 correction: a fresh acquisition stored through `*slot` is an Out hand-off | **reproduced** (the same control's third row, `Safe`) |
| M03: field stores through formals and local owners are refused | **reproduced**, and stays refused: an owned identity into a caller's record, and a tracked owner |
| M04: nested, array and function-pointer stores are refused | **reproduced**. Its function-pointer row `s->cb = f` is **flipped** (§5). The two tracked-owner rows stay refused. |
| Q36's fixture `pointer_store_and_union_still_refuse` (`s->p = q`) | **reproduced**, unchanged: a formal's pointer (example C) |
| Q18 (`aggregate_store.rs`): O00-B08 links a declared record pointer of the field's type, never a `void *` value | **reproduced** by the probe ([probes/](probes/zz_cg11_probe.rs.txt)). `t->p = c` with `struct C *c` links; with `void *c` it refuses. |
| CG00's 37 freed-alone functions, measured on `ae17152b` | **all 37 still Unknown** on this base (`artifacts/before/cg00-list.txt`); none had moved |
| CG00 attributes `rd_kafka_message_timestamp` to M02 (store through a slot) | **correction**: `*tstype = …` stores an enumeration (a scalar) through an out slot, not a pointer value. It is not this card's shape. |
| CG00's per-entry sizes (M02 1,441 / 590, M03 531 / 297, M04 700 / 314) and the absorbed entries' registered sizes | **not re-run** by their own scripts. Rule 10v's own typed partition (§4) is recorded beside them. |

## 3. Example C

`void set(struct S *s, int *q) { s->p = q; }` **stays Unknown** with the gate's
refusal. It moves neither by source semantics nor by a trusted contract on
`set`, because `q` is a formal's object pointer. The missing premise is one of
these:

- an owner identity for `*s`, from a contract or tracked state, that bounds
  `*q`'s lifetime by its owner's;
- a derived summary mode that states a formal stored into another formal's
  record (a derived Stored mode, the W45-SUB-OBJECT-STORE-FORMAL-VALUE
  discharge).

Admitting the store on the body alone would make a derived summary call `q`
borrowed, the `cfl_list_add` hazard W45 measured. With the gate that summary
would be withheld, but the lifetime of `*q` against its owner would still rest
on nothing stated.

## 4. Measurements

Commands: [scripts/measure.sh](scripts/measure.sh) runs the corpus arbiter
`run_pinned_audit.py … 66910c10 --jobs 8`, the system route and the
tail-pause slice, with `QCORE_TYPED_PATH_PARITY`, each into a fresh private
directory. Every exit is 0. The records are in [artifacts/](artifacts/), and
[scripts/archive.py](scripts/archive.py) writes them.

| Route | BEFORE (`919508e3…`, base) | AFTER (`0c1fd4b9…`, `9d7c88d1`) | Moves |
|---|---|---|---|
| corpus | 543 / 10 / 4,281, `963d7c9f…` (CG02's AFTER, byte-identical) | **548 / 10 / 4,276**, `a6dc500b…` | +5 Unknown → Safe |
| system | 32 / 2 / 200 (CG02's AFTER totals) | **33 / 2 / 199** | +1 Unknown → Safe |
| tail-pause slice | 26 / 1 / 242 / 1 Pending (no diagnostic differs from CG02's AFTER run `cg02-after-main`) | 26 / 1 / 242 / 1 Pending | 0 |

- **Sidecars.** They are byte-identical on every route: `monkey-source.json`
  `57acfc58…`, `…chunk-contracts.json` `cbbcfd9c…`, and the contracts and
  source-effects files (`artifacts/after/identity.json`).
- **Tip binary.** It adds only the named value's declaration-id check and a
  doc comment. It is re-measured in §7.

**Every moving function, attributed by reading:**

- `chunkio.c::cio_options_init`: five null stores into members of the caller's
  `struct cio_options`. W45's lexical reader refuses the whole record because
  of its `log_cb` function-pointer member (W45-SUB-OBJECT-STORE-MEMBER-UNRESOLVED).
  Leaves −5 IND.
- `rdkafka_buf.c::rd_kafka_bufq_init`, `rdkafka_coord.c::rd_kafka_coord_cache_init`,
  `rd_kafka_coord_reqs_init` and `rdkafka_mock_sharegrp.c::rd_kafka_mock_sharegrps_init`:
  `TAILQ_INIT(&X->head)`, which expands to
  `((&X->head)->tqh_first) = ((void *) 0); (&X->head)->tqh_last = &((&X->head)->tqh_first);`.
  A null and an interior address go into the caller's embedded head
  (W45-SUB-OBJECT-STORE-PREFIX, a member-address root). Leaves −1 IND each.
- System: `src/flb_task.c::map_free_task_id`, `config->task_map[id].task = NULL`:
  a null store into an element of an embedded array of the caller's object.
  Leaves −1 IND.

**Other effects.**

- 45 more Unknown functions in the corpus lose leaves (IND −74 and DEST −1 in
  total).
- The slice has 2 such functions (IND −2).
- There is no Safe → non-Safe move, **no correction**, and no Unknown → Unsafe.
- No leak or violation is added, and `unknown_alias` is unchanged on every
  route.
- **`CG10-BOUND-UNTRACKED-ROOTS`:** rule 10v touches **0** of its 2,550
  `unknown_alias` leaves. A store whose root is bound but untracked is an
  unresolved owner and keeps its gate refusal, which adds no alias leaf. 470
  identity-free stores wait on that entry's premise
  (`CG11-UNRESOLVED-OWNER-POINTER-STORES`).
- **Callers.** No summary is published, so the corpus callers of the five
  newly Safe bodies keep their `call to uncontracted function` leaf: 5 leaves
  on 4 Unknown callers (`artifacts/after/callers.txt`).

**Rule 10v's typed census** ([scripts/census.py](scripts/census.py),
`artifacts/after/corpus-census.json`, deduplicated per statement). In the
corpus, 2,836 pointer-typed stores reach the classifier.

794 of them carry no identity:

| Value class | Statements |
|---|---:|
| null | 557 |
| function | 100 |
| interior address | 70 |
| the destination's own object | 30 |
| function-pointer name | 22 |
| string | 15 |

Of those 794, 133 are admitted in some analysis and 120 in every analysis,
on 58 functions. The flow refuses 470 of them as unresolved owners, and 3 as
tracked owners in every analysis.

2,042 are refused at classification:

| Refused class | Statements | Functions |
|---|---:|---:|
| destination not a CG10 place | 519 | 162 |
| pointer read | 421 | 187 |
| formal's pointer | 376 | 180 |
| local's pointer | 263 | 169 |
| another object's address | 217 | 73 |
| other | 186 | 133 |
| arithmetic | 53 | 30 |
| call | 7 | 4 |

The system census admits 12 statements, and the slice census admits 11 in
every analysis (12 in some).

## 5. Prediction against outcome

| Prediction | Outcome |
|---|---|
| BEFORE reproduces the baseline | **Held** (`963d7c9f…`, 543 / 10 / 4,281; system and slice as CG02's AFTER) |
| 1. No Safe → non-Safe | **Held**, on every route |
| 2. At most 5 token-visible Unknown → Safe, plus at most 5 more from functions and function-pointer names | **Held: exactly the 5 named** (`cio_options_init` and the four `TAILQ_INIT` bodies). The functions and function-pointer names moved none: of their 122 classified stores, 108 have an unresolved owner. |
| 3. The other 33 of CG00's 37 stay Unknown for the named reasons | **Held** (`artifacts/after/cg00-list.txt`) |
| 4. No Unknown → Unsafe; no leak or violation added | **Held** |
| 5. Store refusals fall; `unknown_alias` does not rise; CG10-BOUND-UNTRACKED-ROOTS' leaves untouched | **Held** (IND −74, DEST −1, alias ±0, 0 of its leaves) |
| 6. No cascade | **Held** |
| 7. System and slice: 0 status moves | **Missed on the system route: +1** (`map_free_task_id`, a direct admission attributed above). Slice held. |
| 8. Sidecars byte-identical | **Held** |
| 9. Tests: M04's `s->cb = f` flips; M02, M03 and Q36 keep their refusals | **Held**, and **missed two flips**: W45's pins `r->next = r` and `h->rec->name = 0` also flip (§6) |
| 10. Ledger: CG11 amended, the absorbed entries amended | **Held** (§8) |

## 6. Tests and flips

New tests:

- [`tests/cg11_pointer_stores.rs`](../../../crates/qcore-c-gateway/tests/cg11_pointer_stores.rs),
  12 tests;
- [`tests/unit/pointer_store.rs`](../../../crates/qcore-c-gateway/tests/unit/pointer_store.rs),
  6 tests: the table, the admission exactly-when, the Lean vectors bound row
  by row, every vector spelling classified by Clang, same-object addresses,
  and function-pointer types;
- [`tests/unit/legacy_pointer_store.rs`](../../../crates/qcore-c-gateway/tests/unit/legacy_pointer_store.rs).

The decision vectors are bound three ways: to Lean (text), to Clang's
classification (unit test), and to the route's outcome end to end
(`every_decision_vector_is_the_route_outcome`, which reads the vectors from
the source text).

**Flips, each named in its test:**

- CG00's `m04_nested_array_and_function_pointer_stores_are_refused`: the row
  `s->cb = f` goes from refused (IND) to **Safe**.
- W45's `a_formal_pointer_stored_into_a_member_keeps_its_refusal`: the row
  `r->next = r` goes from Unknown to **Safe**. The destination's own object
  moves no custody, and the body publishes no summary, so the `mk_list_init`
  hazard the pin guarded cannot recur. The row `h->name = (char *) s` keeps
  its refusal.
- W45's `a_member_chain_keeps_its_refusal`: `h->rec->name = 0` goes from
  Unknown to **Safe**, a null into a chain's member through a never-bound root.

**Refusal controls the brief names:**

- **An overwrite of an owning field.** `an_overwrite_of_an_owning_field_is_refused`
  covers `s->p = 0`, `s->in.p = 0` and `s->self = s` on an acquired `s`: each
  is refused (IND) and Unknown.
- **A double release through a stored alias.**
  `a_double_release_through_a_stored_alias_is_never_safe` covers
  `n->p = b; q_free(n->p); q_free(b);` and STABILITY-REEVAL-5's
  member-release shape with the member stored first. Neither is Safe.
- **A store into a formal's object without an owner premise.**
  `example_c_…_is_refused` covers example C, an owned identity into the
  caller's record, and a local pointer. All are Unknown.

**Other controls:**

- a released or bound owner;
- values that are reads, calls, arithmetic, another object's or a local's
  address, or conditionals;
- the summary gate: `init` is Safe; its caller keeps `call to uncontracted
  function`; a scalar-only `init` still serves its caller.

## 7. Tip identity

The tip binary (`scripts/measure.sh tip`) reran all three routes. The results
are in [artifacts/tip/](artifacts/tip/); see the gate record.

## 8. Lean

`QCore.PointerStore`, 18 theorems. It is registered in
`proof/AuditPointerStore.lean` and `QCore.lean`, with no `sorry` and no project
axiom. The theorems, by the brief's four requirements:

- **Frame preservation for non-owning stores.** `frame_preserved`: an admitted
  store changes no obligation and no field link, and stores no identity.
- **No obligation lost on overwrite.**
  - `no_obligation_lost_on_overwrite`: an admitted store's old value carried
    nothing tracked, its owner is untracked, and every outstanding obligation
    is still outstanding.
  - `an_owning_field_overwrite_refuses`, `a_tracked_old_value_refuses` and
    `an_owned_field_is_never_overwritten`.
- **No double release through a stored alias.**
  - `no_double_release_through_a_stored_alias`: after an admitted store, a
    release through the stored place names no identity.
  - `the_member_release_shape_is_refused`: a local's or formal's pointer, or
    a value carrying an identity, is never admitted into any owner, which is
    the shape `MEMBER-RELEASE-BEFORE-SUMMARY-RELEASE` needs.
- **Refusal of unlisted destinations.**
  - `an_unlisted_destination_refuses`, through `TypedPlace.readPlace`;
  - `an_unresolved_owner_refuses`;
  - `a_live_root_is_a_tracked_owner`: a formal bound to its caller's site
    while a summary is derived refuses.
- **Also:**
  - `decision_vectors_hold` and `admitted_exactly_when`;
  - `example_c_is_refused`;
  - `the_final_owner_decides_resolution` (`chain_owner` agrees with
    `resolves`);
  - `a_never_bound_root_owns_untracked`;
  - the same-object theorems `another_root_is_another_object`,
    `a_deeper_value_place_is_another_object` and
    `a_direct_member_place_is_not_the_root_object`.

Stated, not proved:

- **P-CG11-CALLER-SLOT:** the old value of a slot in an untracked object is
  not this body's obligation.
- Faithful Clang extraction (trust boundary 28).
- That the Rust transfer computes the modelled `Facts`, which the
  shared-vector tests bind at the decision.

## 9. Follow-ups

**Amended, not retired.** `CG11-POINTER-VALUED-STORES` is amended, not
retired. Its scope includes formal-value, owned-value and named-value stores,
and they stay refused. The ten absorbed entries are amended with what rule
10v measured for each ([scripts/update_ledger.py](scripts/update_ledger.py)).
None closes, and their registered sizes are carried, not re-run:

- `W43-INDIRECT-POINTER-VALUE-STORE`;
- the seven `W45-SUB-OBJECT-STORE-*` entries;
- `W44-CALL-RESULT-INDIRECT-DESTINATION` and `W43-COLLECTION-NAME-STORE`,
  which are unchanged: a call never reaches the typed statement route, and
  the older element route decides first.

**Registered, 623 → 627:**

- `CG11-UNRESOLVED-OWNER-POINTER-STORES`: 470 statements on 210 functions,
  which need CG10-BOUND-UNTRACKED-ROOTS' premise.
- `CG11-POINTER-STORE-SUMMARY-MODE`: 5 leaves on 4 callers.
- `CG11-FUNCTION-POINTER-PARAMETER-CLOSURE`: found by the probe.
  `void f(struct S *s, void (*g)(void)) { s->a = 1; }` is Unknown with
  `unknown_alias s`. 92 corpus functions declare a function-pointer
  parameter, all Unknown, and 23 carry an alias leaf on another parameter.
  This is fail-closed.
- `CG11-UNREAD-POINTER-PLACES`: 519 statements on 162 functions.

The queue's P5 status line and CG11's row are marked in the two plans.

## 10. Gate

See [gate.json](gate.json) and [gate/](gate/). The commands are those of
[scripts/gate.sh](scripts/gate.sh). The provider gate is not due, because
`providers/` is unchanged.
