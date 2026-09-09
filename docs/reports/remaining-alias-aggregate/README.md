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
shape as R4. L19581 is the sibling card's.

### 1.2 The attribution, checked by withdrawal rather than argued

The table above is a reading of the transfer function. It is also **measured**,
by three runs of the same probe over the same production unit, each withdrawing
one root and nothing else. A withdrawal replaces the named statement in the
*preprocessed* text with `(void) 0;` and re-analyses; it is an attribution
experiment on a modified program, not a claim about the real one.

| Run | What is in force | Diagnostics | Identity residuals left |
|---|---|---|---|
| [`logs/probe-before.txt`](logs/probe-before.txt) | base tree | **21** | 12 + the nullable handback |
| [`logs/probe-after.txt`](logs/probe-after.txt) | **R1 admitted** (this card) | **12** | 4 + the nullable handback |
| [`logs/probe-withdraw-errno.txt`](logs/probe-withdraw-errno.txt) | R1 admitted, **R2/R3 withdrawn** | **7** | 1 + the nullable handback (rows 12, 13) |
| [`logs/probe-withdraw-all.txt`](logs/probe-withdraw-all.txt) | R1 admitted, **R2/R3/R4 and the two mutex calls withdrawn** | **1** | **none** |

The last row is the whole census in one line. With this card's admission in
force and the three roots it does not take removed, `flb_input_chunk_create`
reports **one** diagnostic — L19581, the sibling card CHAINED-TAIL's — and
**five of its six exits are `Discharged`**, the success exit escaping all three
identities it holds:

```
L19647  custody Discharged  sites [site0@L19578 Escaped, site1@L19598 Escaped, site2@L19615 Escaped]
L19622  custody Discharged  sites [site0@L19578 Released, site1@L19598 Released]
L19602  custody Discharged  sites [site0@L19578 Released]
L19596  custody Discharged  sites [site0@L19578 Released]
L19589  custody Discharged  sites [site0@L19578 Released]
L19582  custody Unknown     sites []          ← the `!chunk` arm, L19581
```

`site2@L19615` is the routes-mask allocation, which the base tree never tracked
at all because its store refused. So the twelve residuals are not merely
*attributed* to four roots — the four roots are jointly **sufficient**, and
nothing else in this body stands between the projection and a complete custody
account of it. What that account would then *rest on* is a different question:
the six NEIGHBOURS declarations, `P-B08-SHALLOW`, and everything else those
cards named as premises.

---

## 2. R1 — the opaque member, and the fixture that hid it

### 2.1 What was measured

`struct flb_input_chunk`'s member is `void *chunk;`
(preprocessed line 10707 of the production unit, from
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
retained unchanged — **no theorem was retired, renamed or weakened**.
`QCore.MemberAddress` and `QCore.AggregateComposition`, which also import this
namespace, are unaffected. The audit-file count stays at 67; the two existing
audits grow by fourteen `#print axioms` lines.

---

## 5. The three roots this card does **not** take, with the capability each needs

Three roots, two capabilities. Both are reasoned refusals with the ground
measured, not deferrals by silence, and each names what it would take.

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

### 6.1 The route, and the BEFORE reproduction

A pinned Fluent Bit `66910c10…` clone and a forced-Libevent Monkey were
prepared fresh in this worktree with
`scripts/prepare_cycle19_clang_audit.py`; provider packages were rebuilt at this
tree. Both runs used the canonical pinned route
([`logs/run-audit.sh`](logs/run-audit.sh)), both exit 0 with zero gateway
failures:

```sh
python3 -B providers/package.py build --output build/cycle19/provider-packages-alias-aggregate
QCORE_CLANG=/usr/bin/clang \
QCORE_CHUNKIO_PACKAGE=<worktree>/build/cycle19/provider-packages-alias-aggregate/qcore.chunkio \
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  --pinned-evidence <pin>/fluent-bit-clang-source \
  <pin>/monkey/monkey-build/compile_commands.json \
  <out>/monkey-source.json <out>/resource-diagnostics.json
```

The BEFORE run was started from the **base tree**: this card's two source files
were checked out at `8069a99` for the whole of it, so it carries no trace of the
diff. Against the wave-15 baseline
([`docs/reports/wave15-corpus/artifacts/`](../wave15-corpus/artifacts/)):

| Artifact | BEFORE vs wave-15 baseline |
|---|---|
| `resource-diagnostics.json` | **byte-identical** (`1da0555b…`) |
| `resource-diagnostics.chunk-contracts.json` | **byte-identical** (`c4f438e4…`) |
| `resource-diagnostics.contracts.json` | **0** differing lines after path normalisation — the contract map is keyed by absolute path and this worktree's differs |
| `monkey-source.json` | 14 differing lines after path normalisation, all `"c_type": "char[147]"` → `"char[150]"`: a `__FILE__`-derived array whose length follows the clone path, three characters longer here |
| `resource-diagnostics.source-effects.json` | 60 differing lines, all package/build/configuration digests: this card rebuilt the provider packages into its own directory |

Both residues are the classes the wave-9 and wave-10 consolidations already
recorded — **path-literal provenance** and **package identity** — and neither
touches a verdict. The verdict artifact reproduces exactly, so the BEFORE
measurement is the baseline.

### 6.2 The census

[`census.py`](census.py) (card NEIGHBOURS', reused apart from its header and
example paths), transcript at [`logs/census.txt`](logs/census.txt):

```
units: 185 -> 185;  added: [];  removed: []
unit status before/after: {'Safe': 2, 'Unknown': 137, 'Unsafe': 42, 'data_only': 4}
unit status moves: 0
NEWLY SURFACED PROVISIONAL UNSAFE UNITS: 0
unit reason_code moves: 0
projected functions: 4834 -> 4834;  function status moves: 0
leak: 271 -> 271 (+0);  unknown_alias: 4403 -> 4399 (-4);  unsupported: 49896 -> 49891 (-5)
units with diagnostic-count changes: 4
leaves added: 4; leaves removed: 13
```

**The 185-unit headline is `2 Safe / 42 provisional Unsafe / 137 Unknown /
4 data-only`, identical before and after, with zero unit status moves in either
direction, zero unit `reason_code` moves, zero function status moves across all
4,834 projected functions, no newly surfaced provisional `Unsafe` unit, and the
leak count unchanged at 271** — the card brief's hard constraint, measured
rather than argued. Four of the five artifacts are byte-identical between the
two runs; only `resource-diagnostics.json` moves.

### 6.3 Every leaf, by rule

Seventeen leaves moved across four units. All seventeen are attributed:

| Δ | Unit / function | Leaf | Rule |
|---|---|---|---|
| **−1** | `fluent-bit-chunk/flb_input_chunk.c`, `flb_input_chunk_create` L19607 | `indirect memory access` | **R1 admitted**: `ic->chunk = chunk;` is now an opaque-member store |
| **−8** | same function, L19608-19613, L19615, L19624 | `unresolved resource identity \`ic\`` | the cascade R1 was poisoning — rows 2-8 and 11 of §1 |
| **+1 / −1** | `chunkio/cio_chunk.c`, `cio_chunk_open` L2041 | `unresolved resource identity \`ch\`` replaces `indirect memory access` | `ch->backend = NULL;` — an opaque member taking the null constant. The destination is admitted now, so the statement is examined and the *aggregate's* identity is what refuses, poisoned by the earlier `ch->name = strdup(name);` |
| **+2 / −2** | `chunkio/chunkio.c`, `cio_create` L2337, L2347 | `unresolved resource identity \`ctx\`` replaces `indirect memory access` | same shape, on `struct cio_ctx`'s opaque members |
| **+1 / −1** | `monkey-libevent/mk_server/mk_fifo.c`, `mk_fifo_worker_create` L5130 | `unresolved resource identity \`fw\`` replaces `indirect memory access` | same shape |

**All four added leaves are refusals replacing refusals, in the fail-closed
direction.** No statement moved from refused to admitted at any of them: what
changed is that an admitted *destination shape* now exposes the aggregate's
unresolved identity instead of hiding behind the lexical gate — the same
phenomenon card NEIGHBOURS recorded for its own two added leaves. Nothing became
`Safe`, at any level, anywhere in the corpus.

The corpus reach of the admission is therefore **five statements in four units**:
one that clears a whole cascade (`flb_input_chunk_create`) and four whose
aggregates are refused on other grounds. That is a small number, and it is the
measured one.

### 6.4 The baseline is not re-pinned

[`artifacts/`](artifacts/) is this card's after-measurement, archived with a
manifest carrying each SHA-256. The wave-15 baseline is untouched and stays
current until an integrator consolidation moves it; §6.3 is the input to that
decision.

---

## 7. What was touched, and the pins moved deliberately

### 7.1 The diff

| File | What |
|---|---|
| `crates/qcore-c-gateway/src/typed_field_access.rs` | the `void_pointer_fields` member class and its accessor; two new unit tests |
| `crates/qcore-c-gateway/src/aggregate_store.rs` | `AdmittedStore::opaque_pointer`; the three-way exclusivity check in `admitted_field`; the opaque branch of `classify_value`; the explicit refusal in `bound_read_actuals`; three new unit tests and one deliberate pin update |
| `crates/qcore-c-gateway/tests/alias_aggregate.rs` | new: the held-out `v_*` family and the two-declaration comparison (12 tests) |
| `crates/qcore-c-gateway/tests/fixtures/contracts/alias_aggregate_generic.qcore` | new: the `v_*` declared premises |
| `crates/qcore-c-gateway/tests/o04_b08.rs` | three deliberate pin updates (§7.2) |
| `proof/QCore/AggregateStore.lean`, `proof/QCore/BoundFieldRead.lean` and their audits | the third member class and the store/read asymmetry |
| `compiler/examples/alias_aggregate_probe.rs` | new: the read-only per-unit probe this card's census was measured with |

**`crates/qcore-c-gateway/src/legacy_c.rs` is byte-unchanged.** No transfer
rule, no flow-sensitive premise, no diagnostic, no work-list behaviour and
nothing in `report_leaks`, `alias_facts` or `AbstractState::join` moved, so
`C02-P-ALIAS-LEAK-ORDER` is undisturbed by construction as well as by
measurement. `borrowed_actual.rs` (C02-P) is untouched: a member read through a
base the projection *never binds* keeps its existing admission, including for a
`void *` member, because such a base can carry no tracked identity for a type
premise to be about. This card's read refusal is about **bound** bases only.

### 7.2 Pins moved deliberately, each keeping its load-bearing half

| Pin | Was | Now | Why |
|---|---|---|---|
| `aggregate_store.rs::a_scalar_and_a_single_pointer_field_are_admitted_and_nothing_else` | `void * any` refused | renamed `…_a_scalar_a_single_pointer_and_an_opaque_pointer_field_…`; `any` admitted **and its class asserted** | the third class. `struct child ** deep` and an undeclared member stay refused, and every field's class is now asserted rather than only its admission |
| `o04_b08.rs::every_refused_member_keeps_its_unknown_and_its_diagnostic` | 6 rows, one of them `b->any = it;` | 5 rows | the `void *` row moved to `alias_aggregate.rs`, where it is asserted **admitted as a store and refused as a read**. The other five rows are unchanged |
| `o04_b08.rs::a_certificate_resting_on_an_aggregate_store_is_rederived_from_source`, row "the acquired member's pointee stops resolving" | `k_word *raw;` → `void *raw;` | → `k_hidden *raw;` | the substitution must violate the premise; `void *` no longer does. An undeclared typedef is still exactly "a pointee the reader will not resolve at all", which is what that row's own comment says it is testing |
| `o04_b08.rs::substituting_a_member_declaration_withdraws_the_arm`, rows "the branched member's pointee stops resolving" and "an admitted scalar member stops being one" | both substituted `void *` | `flb_route_mask_element **` and `size_t **` | same reason; a double-star member is refused by every class |

No other pin moved in either direction. No diagnostic code was added, changed or
downgraded, and no fail-closed gate was weakened: the change is a *widening of
one member premise* with an explicit refusal added on the read side.

---

## 8. Gate

Run in this worktree at the card's final tree:

| Gate | Result |
|---|---|
| `cargo test --workspace` | **2,654 passed, 0 failed, 8 ignored** across 152 targets — [`logs/workspace-tests.txt`](logs/workspace-tests.txt) |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py` | **67 Lean audits pass**, no new axiom |
| 185-unit pinned audit, twice | both exit 0, zero gateway failures (§6) |

`crates/qcore-c-gateway` alone runs 352 unit tests plus its integration suites,
including this card's 12 new tests in
[`tests/alias_aggregate.rs`](../../../crates/qcore-c-gateway/tests/alias_aggregate.rs)
and 4 new unit tests in the two changed modules.

**One flake observed and not hidden.** The first full-workspace run failed one
test — `qcore-verifier-host`'s
`a_confined_fixture_cannot_write_outside_the_admitted_paths`, with
`Broken pipe (os error 32)` where it expected `Terminated`. The machine was
carrying a load average above 40 from concurrent work at the time. It passes on
its own and passes in the clean rerun archived above; nothing this card touches
is reachable from `qcore-verifier-host`, whose dependency set contains no
gateway crate. Recorded here rather than smoothed over, in the same spirit as
`P02-FLAKE`; the assertion was not weakened.

---

## 9. The increment paragraph (draft — for the integrator, not applied here)

> **Opaque aggregate members (ALIAS-AGGREGATE):** the twelve `unresolved
> resource identity` residuals card NEIGHBOURS left on production
> `flb_input_chunk_create` are measured to have **no aliasing cause at all** —
> each is the reaching state of an earlier refusal, and the twelve have four
> roots. The root of eight of them corrects card O04-B08's own record:
> production declares `void *chunk`, not the fixture prelude's
> `struct cio_chunk *chunk`, so that card's flagship store never reached
> production (`NEIGHBOURS-FIXTURE-PROXY`, caught on the statement it was named
> for). A third member class lands — `void *`, admitted as a store destination
> and refused as a bound read in every position, with the exact-tag value
> premise replaced by "a declared record pointer" — and the constructor moves
> 21 → 12 diagnostics with `ic` residuals 10 → 2. A withdrawal experiment shows
> the four roots are jointly sufficient: with this admission and the three
> named follow-ons removed, the body reports one diagnostic (the sibling card's
> L19581) and five of six exits Discharge. Corpus: headline unchanged at
> 2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only, zero unit and
> zero function status moves, **leak unchanged at 271**, four units'
> diagnostics moved with all four added leaves refusals replacing refusals.
> Fourteen new audited Lean theorems across two existing namespaces, no new
> axiom; five pins updated deliberately, each keeping its load-bearing negative
> half. Evidence: `docs/reports/remaining-alias-aggregate/README.md`.

---

## 10. Named follow-ons

| Name | What |
|---|---|
| **`ALIAS-AGGREGATE-ERRNO`** | §5.1 — a neutral-expression family for a dereference of a call result, plus contract text for `__errno_location`. Accounts for three of the four remaining identity residuals |
| **`ALIAS-AGGREGATE-PUBLICATION`** | §5.2 — contract text for `mk_list_add`/`msgpack_packer_init`, which requires the unmodeled publication relation (O00 B03/R13) |
| **`ALIAS-AGGREGATE-OPAQUE-READ`** | the opaque member is deliberately store-only. Reading one back would need a way to relate an untyped member to a callee's declared parameter type; no such premise exists here and none is claimed |
