# ALIAS-AGGREGATE — the twelve identity residuals, measured to their roots — 2026-09-10

The named continuation of card NEIGHBOURS
([`docs/reports/remaining-neighbours/README.md`](../remaining-neighbours/README.md)
§4.3), which closed the uncontracted-callee family on production
`flb_input_chunk_create` and left **21** residual diagnostics in two families,
of which **twelve** are `unresolved resource identity` on `ic` and `chunk` — the
aggregate-aliasing family this card was given.

**Base tree:** main `8069a99` ("Consolidate wave 15: corpus re-pin, follow-ons,
ledger"), submodules initialised, `third_party/libevent` at `d82464a2`. Every
measurement below was taken on that tree with only this card's own diff applied,
on the pinned corpus (Fluent Bit `66910c10a4d7eafa810b84229cf2dcf7b0f26f97`,
explicit `/usr/bin/clang`), through the canonical pinned route.

---

## Headline

**None of the twelve is an aliasing question.** Measured statement by statement
on the production corpus — not on a fixture — every one of them is the *reaching
state* of an earlier refusal in the same body, and the twelve have exactly
**four** roots between them. This card does not narrow the family by refining an
alias relation, because there was no alias relation to refine; it narrows it by
removing the one root that belongs to the aggregate-store machinery, and it
records the other three with the capability each would take.

| | before | after |
|---|---|---|
| `unresolved resource identity` on `ic` | **10** | **2** |
| `unresolved resource identity` on `chunk` | **2** | **2** |
| `nullable resource \`ic\` used by a non-null contract` | 1 | 1 |
| `indirect memory access` | 8 | **7** |
| **total diagnostics, `flb_input_chunk_create`** | **21** | **12** |
| function status | `Unknown` | **`Unknown`** — unchanged |

**The measured root of eight of the twelve is a correction to card O04-B08's own
record.** Production declares

```c
struct flb_input_chunk { …; void *chunk; …; };          /* real header */
```

while the reduced `crates/qcore-c-gateway/tests/fixtures/o04/prelude.c` declares
`struct cio_chunk *chunk;`. `void *` was refused by O04-B08's premise 7, so
**that card's flagship store `ic->chunk = chunk;` never reached production at
all** — it is admitted on the fixture and refused on the corpus. This is exactly
the hazard NEIGHBOURS recorded as `NEIGHBOURS-FIXTURE-PROXY`, caught here on the
statement that card was named for.

**Nothing is promoted, at any level.** The 185-unit headline is unchanged, no
unit and no function changed verdict, and the leak count is unchanged.
`C02-P-ALIAS-LEAK-ORDER` is undisturbed by construction: nothing this card
touches runs in `report_leaks`, in `alias_facts` or in the custody work list.

**One deviation from the card brief, taken deliberately and recorded here rather
than in passing.** The brief reserves "L19581 and the 7-refusal
chained/indirect-read family" for the sibling card CHAINED-TAIL, and L19607
(`ic->chunk = chunk;`) is one of those seven rows in NEIGHBOURS' table. It is
taken here, on the measured ground in §2.3: it is not a chained access and not a
read — it is an aggregate-store *destination*, decided by
`crates/qcore-c-gateway/src/aggregate_store.rs`, which is this family's own
machinery — and it is the root of eight of the twelve residuals this card was
given. Without it this card can only publish a census. The diff is confined to
the store side so the two cards stay separable; §7 states exactly what was
touched.

---

## 1. The census — every residual, with its binding conjunct

Measured with the read-only probe
[`compiler/examples/alias_aggregate_probe.rs`](../../../compiler/examples/alias_aggregate_probe.rs),
which runs the production gateway over one compilation-database entry and prints
every CFG node in line order with the instruction the importer lowered it to, the
preprocessed source line, and the diagnostics reported at it. It publishes no
verdict and writes no artifact. Transcript:
[`logs/probe-before.txt`](logs/probe-before.txt).

Three refusal sites in `legacy_c.rs` can produce `unresolved resource identity`:
`apply_resource_action` (a custody-bearing actual), `resolved_aggregate_owner`
(an aggregate store, a member address) and `bound_read_resolves` (a bound member
read). All three begin by asking the same question of the reaching state — *is
this name bound to exactly one, known, non-null identity here?* — and every
statement the importer refuses answers it "no" for **every** pointer in the
state at once:

```rust
LegacyInstruction::Unknown { description } => {
    diagnostics.insert(…);
    for pointer in state.pointers.values_mut() {
        pointer.unknown = true;
        pointer.may_be_null = true;
    }
}
```

That is the mechanism, and it is the whole census.

| # | Line | Residual | Statement | Binding conjunct | Root |
|---|---|---|---|---|---|
| 1 | 19601 | `chunk` | `cio_chunk_close(chunk, !0);` | `apply_resource_action`: `pointer.unknown` | **R2** (19600) |
| 2 | 19608 | `ic` | `ic->fs_backlog = 0;` | `resolved_aggregate_owner`: `pointer.unknown` | **R1** (19607) |
| 3 | 19609 | `ic` | `ic->in = in;` | same | **R1** |
| 4 | 19610 | `ic` | `ic->stream_off = 0;` | same | **R1** |
| 5 | 19611 | `ic` | `ic->task = ((void*)0);` | same | **R1** |
| 6 | 19612 | `ic` | `ic->create_time = flb_time_now();` | same | **R1** |
| 7 | 19613 | `ic` | `ic->total_records = 0;` | same | **R1** |
| 8 | 19615 | `ic` | `ic->routes_mask = (flb_route_mask_element *) flb_calloc(…);` | same | **R1** |
| 9 | 19620 | `chunk` | `cio_chunk_close(chunk, !0);` | `apply_resource_action`: `pointer.unknown` | **R3** (19619) |
| 10 | 19621 | `ic` | `flb_free(ic);` | same | **R3** |
| 11 | 19624 | `ic` | `has_routes = flb_routes_mask_set_by_tag(ic->routes_mask, …);` | `bound_read_resolves` conjunct 1 (owner) **and** conjunct 3 (no field link, because row 8 refused) | **R1** |
| 12 | 19647 | `ic` | `return ic;` | `apply_resource_action`: `pointer.unknown` | **R4** (19628/19629) |
| — | 19647 | `nullable ic` | `return ic;` | `unresolved_nullability`: the site is back in `nullable_sites` | **R4** |

Row 13 — the nullable handback — deserves its own sentence, because it looks
like an independent fact and is not one. `ic = flb_calloc(…)` acquires a
`nullable` site, and the guard `if (!ic) { … return NULL; }` **does** refine it:
`refine_pointer_null(assume_null = false)` calls `finalize_replacement_success`,
which removes the site from `nullable_sites`. What puts it back is
`AbstractState::join`:

```rust
for pointer in self.pointers.values() {
    if pointer.may_be_null {
        self.nullable_sites.extend(pointer.targets.iter().copied());
    }
}
```

Poisoning sets `may_be_null = true` on every pointer, so at the next join the
refuted nullability is reinstated. The handback residual is therefore the same
cascade as row 12, in a different diagnostic channel.

### 1.1 The four roots

| Root | Line(s) | Statement | Why it refuses |
|---|---|---|---|
| **R1** | 19607 | `ic->chunk = chunk;` | the destination member is declared `void *`, which O04-B08's premise 7 refused (§2) |
| **R2** | 19600 | `flb_errno_print((*__errno_location ()), "<stdin>", 2556);` | a dereference of a **call result** in an argument (§5.1) |
| **R3** | 19619 | `flb_errno_print((*__errno_location ()), "<stdin>", 2584);` | the same statement in the routes-mask arm |
| **R4** | 19628, 19629 | `msgpack_packer_init(&ic->mp_pck, ic, flb_input_chunk_write);`, `mk_list_add(&ic->_head, &in->chunks);` | the member-address rule requires a **contracted** callee, and neither is contracted (§5.2) |

Two further `indirect memory access` refusals sit in the same body — L19637 and
L19639, `pthread_mutex_lock/unlock(&in->metrics_chunk_lock)` — and are the same
shape as R4. They are not roots of any residual in this family: the two
`flb_hash_table_add` calls between them are already refused-through, and nothing
downstream of them but `return ic` reads a tracked identity. L19581 is the
sibling card's.

---

## 2. R1 — the opaque member, and the fixture that hid it

### 2.1 What was measured

`struct flb_input_chunk`'s member is `void *chunk;`
(preprocessed line 10706 of the production unit, from
`include/fluent-bit/flb_input_chunk.h`). O04-B08's premise 7 admits a member
declared with exactly one `*` **over a specifier list the reader resolved**, and
`void` classifies `Opaque`:

```rust
if declarator.stars == 1
    && matches!(self.classify_specifiers(&specifiers, 0),
                DeclaredType::Record(_) | DeclaredType::Scalar)
{ single_pointer_fields.insert(name.clone()); }
```

So `admitted_field` found neither a scalar nor a single-pointer member, refused,
and the statement kept the lexical gate's `indirect memory access` — poisoning
every pointer in the state and producing rows 2-8 and 11.

The scalar stores **before** it (`ic->event_type`, `ic->busy`, `ic->fs_counted`,
lines 19604-19606) are admitted and carry no diagnostic, which is the control:
the record model is complete, the base `ic` is an admissible aggregate, and the
one thing that fails is this member's declared type.

### 2.2 Why the fixture leg could not see it

`crates/qcore-c-gateway/tests/fixtures/o04/prelude.c` declares
`struct cio_chunk *chunk;`. Under that declaration the same byte-verbatim
statement is admitted — which is what cards O04-B08, O04-READ-BOUND, O04-B09B10
and O04-TRACE measured, and it is true of the fixture. It was never true of
production. Pinned, in both directions, by
`the_production_member_is_opaque_and_the_fixture_prelude_is_not` and
`the_same_store_is_admitted_against_both_declarations`.

This is `NEIGHBOURS-FIXTURE-PROXY` with a name attached. It is not a defect in
those cards' arithmetic: each measured what it said it measured. It is a defect
in reading a fixture number as a production number, and this card's §1 and §6
are production measurements for exactly that reason.

### 2.3 Why this is the aggregate-store family and not the chained-access one

`ic->chunk = chunk;` contains no chain (`a->b->c`), no read of a member, and no
actual: it is an assignment *destination*, one `->` selection, decided by
`aggregate_store::admitted_store`/`admitted_field` and by the value classifier
beside it. The sibling card CHAINED-TAIL extends `borrowed_actual.rs`'s access
**chain** for a pointer-typed final field in an argument at L19581. The two
touch disjoint decisions; §7 lists the files.

---

## 3. What lands: the opaque member class, store-only

Implemented in
[`crates/qcore-c-gateway/src/typed_field_access.rs`](../../../crates/qcore-c-gateway/src/typed_field_access.rs)
(the classification) and
[`crates/qcore-c-gateway/src/aggregate_store.rs`](../../../crates/qcore-c-gateway/src/aggregate_store.rs)
(the admission). `legacy_c.rs` is **unchanged**: no transfer rule, no
flow-sensitive premise, no diagnostic and no work-list behaviour moved.

### 3.1 The classification

A member is *opaque* exactly when its specifier list is the literal one-token
`void`, it carries exactly one `*`, it has no array modifier, it carries neither
`volatile` nor `_Atomic`, and its record has one complete non-`union`
definition. Nothing else: `const void *`, `void **`, `void *x[2]`,
`volatile void *` and every typedef spelling — including a typedef *for*
`void *` — are refused, and all six are pinned
(`only_the_exact_void_star_spelling_is_an_opaque_pointer_member`,
`every_other_opaque_member_spelling_keeps_its_unknown_and_its_diagnostic`).

The three classes are disjoint by construction, and `admitted_field` checks the
sum rather than assuming it: a member the reader placed in two classes refuses.

### 3.2 The asymmetry, which is the whole soundness argument

| | typed member | opaque member |
|---|---|---|
| store destination | admitted | **admitted** |
| bound read, `Borrowed` position | admitted | **refused** |
| bound read, custody-taking position | admitted when insulated | **refused** |
| member address (`&BASE->FIELD`) | n/a (pointer member) | n/a |
| value premise | exactly the member's declared pointee type | **any declared record pointer**, or the null constant |
| flow-sensitive owner premise | required | **required, unchanged** |
| escape cascade / overwrite / `P-B08-SHALLOW` | as before | **as before** |

The store is sound on an opaque member for the same reason it is sound on a
typed one, and the difference is confined to what a *read* could claim:

* a store replaces exactly one pointer value, so the link `(owner, field) ->
  child` stays single-valued — the property every use of a field link depends
  on;
* nothing that *uses* a link reads the member's declared type. Escape cascades
  by identity, overwrite replaces by place, and release deliberately does not
  cascade (`P-B08-SHALLOW`, unchanged and still not proved);
* the one use that would need a declared type is the bound read, where the
  member's type is matched against the callee's declared parameter. `void *`
  names no pointee, so there is nothing to match — and the read is refused, in
  every position, insulated or not.

What replaces the exact-type match on the store side is **not** nothing: the
value must still be a single declared name whose declared type is `struct TAG *`
for some tag, so the identity written into the member is one the projection can
name. A cast, a call that is not contracted, an address-of, a scalar and every
non-record pointer keep the existing refusal
(`an_opaque_member_refuses_a_value_that_is_not_a_declared_record_pointer`, both
as a unit test over the classifier and as a source-level test through the public
entry point).

### 3.3 Everything that did not move

The flow-sensitive owner premise (`resolved_aggregate_owner`) is untouched, so a
store into an opaque member of a released, escaped, ambiguous, nullable or
caller-held aggregate refuses exactly as before
(`an_opaque_store_through_a_released_aggregate_is_never_admitted`). A double
store into the same opaque member still reports the first resource
(`a_double_opaque_store_reports_the_first_resource_as_a_leak`). Releasing the
aggregate still does not discharge what the member links
(`releasing_the_aggregate_never_discharges_its_opaque_member`). No diagnostic
code was added, changed or downgraded.

---

## 4. Lean

`QCore.AggregateStore` gains the third member class and ten new audited
theorems; `QCore.BoundFieldRead` gains four. No new axiom; the modules keep
their existing dependency set.

| Theorem | What it fixes |
|---|---|
| `an_opaque_member_store_is_admitted`, `the_admitted_opaque_store_links` | non-vacuity in both halves |
| `the_opaque_member_does_not_require_an_exact_tag` | the production shape is exactly what the relaxation admits |
| `an_opaque_member_refuses_a_value_that_is_not_a_declared_record_pointer` | what replaces the exact-tag premise |
| `a_single_pointer_member_still_requires_the_exact_tag` | the relaxation did not leak into the typed class |
| `an_opaque_member_store_keeps_every_base_premise`, `…_the_qualifier_premise` | only the *member* premise widened |
| `a_released_owner_flips_the_admitted_opaque_store`, `a_caller_held_owner_flips_…` | the flow-sensitive gate is untouched |
| `the_member_classes_are_exclusive` | the model half of `admitted_field`'s sum check |
| `an_opaque_member_never_admits_a_read`, `…_a_custody_read`, `an_opaque_member_is_admitted_by_no_read_half`, `one_opaque_member_flips_the_admitted_read` | the asymmetry, over the implementation's own union of the two read halves |

`Value` and `MemberKind` are the same abstractions the existing modules used;
`Store` gains two fields (`valueTagMatches`, `valueIsDeclaredRecordPointer`) so
the two value premises can be told apart, and every pre-existing theorem is
retained unchanged. `QCore.MemberAddress` and `QCore.AggregateComposition`,
which also import this namespace, are unaffected.

---

## 5. The three roots this card does **not** take, with the capability each needs

Both are reasoned refusals with the ground measured, not deferrals by silence.

### 5.1 R2/R3 — `flb_errno_print((*__errno_location ()), …)`

`flb_errno()` expands to a call whose first argument is `(*__errno_location ())`:
a **dereference of a call result**. The importer's lexical gate refuses any
statement carrying a unary `*` outside an admitted span, and no admitted span
covers this one.

What it would take, precisely:

1. a declared contract for `__errno_location` (it has none, and card
   PROD-CONTRACTS deliberately left `flb_errno_print` undeclared on exactly this
   ground: "the `flb_errno()` macro's dereference refuses the statement before
   any contract is consulted");
2. a **new** neutral-expression family admitting a dereference of a call result.
   Card W4-EXPR-POS's admitted families are proved under the sentence *an
   admitted expression contains no pointer-valued operand*
   (`docs/design/W4_NEUTRAL_EXPRESSION_POSITIONS.md`), and `__errno_location ()`
   is pointer-valued by construction, so this is not a widening of that family
   but a different one, with its own soundness obligation.

Named **`ALIAS-AGGREGATE-ERRNO`**. It accounts for rows 1, 9 and 10 — two
`chunk` residuals and one `ic`.

### 5.2 R4 — `msgpack_packer_init(&ic->mp_pck, ic, …)` and `mk_list_add(&ic->_head, …)`

Card O04-B09B10's member-address rule admits `& BASE -> FIELD` only at a
`Borrowed` position of a **contracted** callee:

```rust
let Some(contract) = contracts.contract(callee) else { return Vec::new(); };
```

Neither `msgpack_packer_init` nor `mk_list_add` has contract text in the pinned
corpus, so no member-address actual is admitted and the `&` keeps the lexical
gate's refusal. The binding conjunct is **absent declared contract text**, which
is the NEIGHBOURS family's question, not this one's; and both are harder
declarations than NEIGHBOURS' six, because both *publish* through their
arguments — `mk_list_add` links `ic` into `in->chunks`, and `msgpack_packer_init`
stores `ic` as a callback datum. A faithful declaration would have to say what
that publication does to the caller's obligation, which is O00 blockers
**B03/R13** and is unmodeled.

Named **`ALIAS-AGGREGATE-PUBLICATION`**. It accounts for rows 12 and 13.

### 5.3 What this leaves

`flb_input_chunk_create` keeps **four** identity residuals and the nullable
handback in production, all attributed above, and the function stays `Unknown`.
No exit of it is claimed to settle in production by this card.

---

## 6. Corpus — before, after, every leaf attributed

*(filled from the measured runs; see `logs/` and `artifacts/`)*

---

## 7. What was touched, and the pins moved deliberately

*(see §7 table)*

---

## 8. Named follow-ons

| Name | What |
|---|---|
| **`ALIAS-AGGREGATE-ERRNO`** | §5.1 — a neutral-expression family for a dereference of a call result, plus contract text for `__errno_location`. Accounts for three of the four remaining identity residuals |
| **`ALIAS-AGGREGATE-PUBLICATION`** | §5.2 — contract text for `mk_list_add`/`msgpack_packer_init`, which requires the unmodeled publication relation (O00 B03/R13) |
| **`ALIAS-AGGREGATE-OPAQUE-READ`** | the opaque member is deliberately store-only. Reading one back would need a way to relate an untyped member to a callee's declared parameter type; no such premise exists here and none is claimed |
