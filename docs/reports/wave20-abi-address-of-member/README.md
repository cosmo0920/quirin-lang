# ABI-TYPEDEF-ADDRESS-OF-MEMBER — the addressed-member reader's unread typedef shape, widened and measured — 2026-09-11

The named continuation of card CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF
([`docs/reports/wave19-abi-typedef/README.md`](../wave19-abi-typedef/README.md)
§3.3, §9), which measured production `flb_input_chunk_create` down to three
statements, found that the refusal binding at each of them was **not** the
declared-ABI reader it had just widened, and renamed the residual
**`ABI-TYPEDEF-ADDRESS-OF-MEMBER`**: the importer refuses
`msgpack_packer_init(&ic->mp_pck, …)` and
`pthread_mutex_lock(&in->metrics_chunk_lock)` on the *address-of-a-member
actual*, before any contract is consulted.

**Base tree:** main `e01cac2` ("Close wave-19 remainder: ledger, gate numbers,
increment record"), submodules initialised, `third_party/libevent` at
`d82464a2`. Every measurement below was taken on that tree with only this card's
own diff applied, on the pinned corpus (Fluent Bit
`66910c10a4d7eafa810b84229cf2dcf7b0f26f97`, explicit `/usr/bin/clang`, Ubuntu
clang 21.1.8), through the canonical pinned route.

---

## Headline

**Production `flb_input_chunk_create` reaches function-level `Safe`.** That is
the first time the pinned Fluent Bit constructor this family has been measuring
since card O00 discharges every exit on byte-verbatim production source, and it
closes the production-reach item card O04-ARMS-O recorded as
`O04-TRACE-REACH` ("zero production admitted statements"). It is a promotion,
it is argued by name in §5, and **nothing is promoted at unit level or above**.

| | BEFORE (base `e01cac2`) | AFTER |
|---|---|---|
| `flb_input_chunk_create` diagnostics | **6** | **0** |
| its exits that `Discharge` | 5 of 6 | **6 of 6** |
| its function status | `Unknown` | **`Safe`** |
| `fluent-bit-chunk/flb_input_chunk.c` | provisional `Unsafe` | provisional `Unsafe` — **unchanged** |
| 185-unit headline | 2 / 42 / 137 / 4 | **2 / 42 / 137 / 4 — unchanged** |
| unit status moves | — | **0** |
| unit `reason_code` moves | — | **0** |
| newly surfaced provisional `Unsafe` **units** | — | **0** |
| function status moves (4,834 projected) | — | **2**, one each way (§6.3) |
| function statuses | 149 `Safe` / 84 `Unsafe` / 4,601 `Unknown` | **150 / 85 / 4,599** |
| leak diagnostics | 272 | **273** — §6.4, named prominently |

The second function move is in the **accusing** direction and is the one that
moves the constraint-7 pin: `input_chunk_get` goes `Unknown` → provisional
`Unsafe`, because the constructor call it makes is now modelled and the release
on one arm goes through an **uncontracted** callee. It is a finding, attributed
in §6.4, not absorbed.

What the diff actually is:

* **One typedef shape moves from unread to read**, in a *second* reader. Card
  CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF widened `legacy_declared_abi`'s typedef
  reader for the ABI comparison. The resource projection has its **own**
  declaration reader, `typed_field_access`, with the same defect: a
  `struct`/`union`/`enum` a typedef *defines* was abandoned at its own opening
  brace, so `pthread_mutex_t metrics_chunk_lock;` and `msgpack_packer mp_pck;`
  were members whose *type* the reader would not commit to (§1).
* **One premise is widened, and only one.** The addressed-member premise moves
  from "the specifier list classifies `struct TAG` or arithmetic" to "the
  specifier list names an **object type that is not a pointer type**", which is
  all the disjointness argument card O04-B09B10 rests on ever consumed (§2).
  The classifier itself answers exactly what it answered before, so every other
  premise keyed on it is untouched — proved in Lean and measured by the
  unchanged gateway suite (§2.4, §4).
* **`msgpack/packer.qcore` is landed**, on PROD-CONTRACTS' rule applied to a
  measurement rather than to an argument: card
  CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF wrote it, measured its payoff at **zero**
  and deliberately did not land it, naming the blocker
  `ABI-TYPEDEF-MSGPACK-UNDECLARED`. This card repairs that blocker and measures
  the payoff at **four diagnostic leaves and one function verdict** (§5.2), so
  the declaration lands, and that follow-on closes.
* **The whole corpus movement is 12 diagnostic leaves removed and 3 added**,
  across 2 units and 4 functions, every one attributed individually (§6.3), and
  the candidate set was written down before the runs (§3, [`prediction.md`](prediction.md)).

---

## 1. The refusal that was really binding, measured rather than inferred

Card CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF's §3.3 is exact about what it found and
careful about what it did not: the three statements lower to an `Unknown`
**instruction** rather than to a `Call` node, "and that is decided by the
importer before any contract lookup happens". What it did not do — being scoped
to the ABI reader — is say *which* conjunct of the importer's own member-address
rule fails. This card measured that first, on the production unit, before
writing any code.

The rule is card O04-B09B10's, in
`crates/qcore-c-gateway/src/aggregate_store.rs::member_address_actuals`: a whole
actual spelled exactly `& BASE - > FIELD`, at a position the contract in force
declares `Borrowed`, is admitted when the base is a declared `struct TAG *` over
a complete non-`union` record, the base is one of the two accepted routes, and
`TypedAccessModel::addressable_field(TAG, FIELD)` holds. Three of those four
conjuncts already held at both production sites, and the measurement says so:

| conjunct | `&in->metrics_chunk_lock` | `&ic->mp_pck` |
|---|---|---|
| base declared `struct TAG *` | ✅ `struct flb_input_instance *in` | ✅ `struct flb_input_chunk *ic` |
| `TAG` complete, single, non-`union` | ✅ — measured: the preprocessed body of `struct flb_input_instance` (138 lines) carries no `(`, `{`, `:` or `=` at all, so the all-or-nothing record reader reads it whole. Its array members (`int channel[2]`, `char name[32]`) the reader tolerates and skips; the function-pointer members one might expect belong to the neighbouring `struct flb_test_in_formatter`, not to this record | ✅ |
| base route accepted | ✅ untracked-never-bound | ✅ an aggregate-store base this body bound |
| **`addressable_field`** | ❌ | ❌ |

The proof that the first three hold is already in the base tree's own probe,
and it needs no new code to see: the neighbouring statement
`mk_list_add(&ic->_head, &in->chunks)` is **modelled** there
([`logs/probe-constructor-before.txt`](logs/probe-constructor-before.txt)
L19629, `Call mk_list_add(_, _) -> _ addresses["ic"]`). Both of *its* actuals
are member addresses on the same two bases; the only difference is the member's
declared type — `struct mk_list`, which the reader does classify. A statement
carrying an unadmitted `->` span is refused outright by the lexical gate, so
`mk_list_add` being modelled at all is a measurement that both bases and the
record completeness are fine.

So the binding conjunct is exactly `addressable_field`, and inside it exactly
the specifier premise. The two members are

```c
pthread_mutex_t metrics_chunk_lock;   /* typedef union { … } pthread_mutex_t;      */
msgpack_packer  mp_pck;               /* typedef struct msgpack_packer { … } msgpack_packer; */
```

and `typed_field_access::read_typedefs` recorded **nothing** for either typedef
name: its reading is the specifier tokens between `typedef` and the terminating
`;`, and it refuses outright when that run contains `{`, `}`, `(`, `)`, `[`,
`]`, `*` or `,`. An aggregate the typedef defines contains braces, so the name
resolved to nothing, classified `Opaque`, and the member fell out of
`non_pointer_fields`.

**This is the same defect card CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF repaired, in a
second reader that card did not touch.** `legacy_declared_abi::read_typedefs`
and `typed_field_access::read_typedefs` are independent readers over the same
preprocessed token stream, feeding two independent decisions (the declared/C ABI
comparison, and the resource projection's typed premises). Widening one left the
other exactly as it was — which is why removing the outer refusal revealed the
inner one and moved nothing.

### 1.1 A fixture that had been proxying the shape all along

Card O04-B09B10's fixture prelude
(`crates/qcore-c-gateway/tests/fixtures/o04/prelude.c`) declares

```c
typedef int pthread_mutex_t;          /* production: typedef union { … } pthread_mutex_t; */
struct msgpack_packer { … };          /* production: typedef struct msgpack_packer { … } msgpack_packer; */
```

Both spellings are ones the reader *does* classify, so the fixture exercised the
member-address admission at both members and production did not. That is
`NEIGHBOURS-FIXTURE-PROXY` again, on the very statements card O04-B09B10 was
named for, and it is why the gap survived four cards. The fixture is **not**
changed here: it is a valid test of the rule it tests, and rewriting it would
retire a pin rather than add one. This card adds a **held-out generic family**
that carries the production spellings instead (§4).

---

## 2. What the widened reader commits to, and on what argument

`typed_field_access::read_typedefs` now recognises one further shape, and the
member premise reads one further predicate. Everything else keeps the behaviour
it had.

### 2.1 The reading

`TypedefTarget` becomes an enum so the new reading can be represented without
pretending to be specifier words:

| shape | before | after |
|---|---|---|
| `typedef T t;` (no declarator at all) | the specifier tokens `T` | `Specifiers(T)` — **unchanged** |
| `typedef union { … } t;` / `typedef struct tag { … } t;` / `typedef enum { … } t;` | nothing recorded | `Aggregate(head)`, where `head` is `"union"` / `"struct tag"` / `"enum"` |
| `typedef struct s *p;`, `typedef int (*f)(void *);`, `typedef int a[4];`, `typedef struct { … } a, *b;`, `typedef struct { … } *p;`, `typedef struct { … } a[4];` | nothing recorded | **nothing recorded** — unchanged |

The recogniser is deliberately exact: the specifier part must begin with
`struct`, `union` or `enum`, the aggregate body's matching `}` must be the token
immediately before the declared name, and the declared name must be the last
token of the declaration. Nothing — no `*`, no `[`, no second declarator —
stands between the aggregate and the name. The **body is not read**: the reading
commits only to "this name denotes an object type with no pointer level", which
is all its one consumer needs.

### 2.2 The premise

`non_pointer_fields` — the member set `addressable_field` publishes, and the
member-address rule's own premise — moves its specifier side from

> the classifier answers `Record(_)` or `Scalar`

to

> `is_non_pointer_object`: the specifier list names an **object type that is not
> a pointer type**.

The declarator side is untouched: still exactly zero `*`, no array modifier, no
`volatile`/`_Atomic`.

**Why that is the right premise, and not a weakening.** Card O04-B09B10's
argument is quoted verbatim in its own source: "a member declared with **no**
`*` is a sub-object of the record. Under the C effective types premise this
whole family inherits, its storage is disjoint from every *other* member of the
same `struct`, and this projection links only single-`->` members of the base.
So a callee holding `&BASE->FIELD` can reach no field link at all." That
argument reads exactly one fact about the member — **that it is not itself a
pointer** — and nothing whatever about its shape, size or contents. An
aggregate, a `union` and an `enum` satisfy it precisely as an `int` does. The
classifier was answering a *stronger* question than the premise was asking, and
the gap between the two was the refusal.

Rows this widening therefore adds:

| admission | argument |
|---|---|
| a member whose typedef **defines** a `struct`/`union`/`enum` | the sub-object argument above, unchanged. Proved non-vacuous on the measured production member (`QCore.MemberAddressReader.an_aggregate_typedef_member_was_refused_and_is_now_addressable`) |
| the same reading behind a chain of plain typedefs | typedef transparency, exactly as for every other target |
| a `union TAG`-typed member of a `struct` | its storage is disjoint from its siblings exactly as any other member's is. The `union` exclusion elsewhere in this model is about *reading through* a union, whose members overlap; taking the address of a union-typed member of a **`struct`** crosses no overlap. The base being a `union` still refuses, untouched |

And the refusals this card **keeps**, each narrowing rather than deleting the
refused class:

| refusal | ground |
|---|---|
| a member with a declarator `*` | the laundering shape the rule exists to refuse: the address of a pointer member is a handle with which the callee could overwrite the very link this projection maintains. Pinned in both directions, and proved to hold under **both** readers |
| a member declared through a **pointer typedef** (`typedef struct s *p; … p m;`) | the same shape spelled without a star. `read_typedefs` records such a typedef unresolved, and an unresolved name is refused by `is_non_pointer_object` — fail-closed by construction, and pinned by a test whose C is byte-identical except for the typedef |
| a **function-pointer typedef** member, an **array typedef** member | unresolved for the same reason |
| an **array** member, a `volatile`/`_Atomic` member | the declarator side, unchanged |
| `void` | not an object type |
| a name typedef'd twice to disagreeing readings | unresolved; §2.3 |
| a non-`Borrowed` position, an uncontracted callee, a `union` base, an ambiguous or incomplete record tag, a chain, a `.` form, a parenthesised or subscripted actual, a base this body may bind | card O04-B09B10's and card C02-P's own conjuncts, none of them touched |

### 2.3 One regression avoided by construction

A unit carrying **both** `typedef struct tag { … } tag;` and
`typedef struct tag tag;` now records two readings for one name — `Aggregate`
and `Specifiers` — which the table's existing duplicate rule retires to the
absent reading. That is exactly the refusal the name had before this card, so
the pair cannot be admitted silently. It is deliberately **not** resolved the
way card CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF resolved the same pair in the *other*
reader (recording a tagged definition as the specifier pair it equals): there,
composing was needed to avoid *withdrawing* a declaration that agreed before
that card; here, refusing costs nothing measured — no member of the pinned
corpus is declared over such a pair — and refusing is the fail-closed direction.
Recorded as the follow-on `ADDRESS-OF-MEMBER-TAGGED-PAIR`, proved in Lean
(`an_aggregate_definition_and_a_plain_typedef_of_one_name_retire_it`).

### 2.4 What did **not** change

* **The classifier answers exactly what it answered before.** An aggregate a
  typedef defines classifies `Opaque`, as an unrecorded name did. Every premise
  keyed on `classify_specifiers` — the scalar field no-op rule, the
  `borrowable_fields`, `single_pointer_fields`, `void_pointer_fields` and
  `record_pointer_fields` member sets, the bounded access-chain walker, the
  declared types of a function's own formals and locals — therefore decides
  byte-identically. This is the containment statement, and it is proved
  (`QCore.MemberAddressReader.the_classifier_is_unchanged`) as well as measured:
  the whole `qcore-c-gateway` suite passes unchanged apart from two manifest
  **count** pins (§7.2).
* **No new refusal class and no new diagnostic code.** The card moves statements
  *out* of `indirect memory access is outside the resource projection`; §L21's
  per-code ownership table is untouched (constraint 1).
* **`record_definition_is_complete` is untouched.** The record body reader stays
  all-or-nothing; a record with a member declaration it cannot read still
  refuses whole.
* **Nothing moved onto the typed Clang AST** (constraint 8) — both readers still
  read only the preprocessed token stream, so W4 derivation, W4 validation and
  `check_legacy_safety_certificate` reproduce the widened decision by
  construction. Nothing touched W4 summary validation (constraint 4), the typed
  scalar-field whitelist (constraint 5), the `report_leaks` pre-pass
  (constraint 6), the publication refusals (constraint 10) or the
  `#line`/identity transport (constraints 11–12).

### 2.5 Where the widening can and cannot reach

`read_typedefs`, `is_non_pointer_object` and `non_pointer_fields` are private to
`crates/qcore-c-gateway/src/typed_field_access.rs`. `non_pointer_fields`'s only
reader is `addressable_field`, and `addressable_field` has exactly two callers:

* `aggregate_store::member_address_actuals` — the admission itself;
* `aggregate_store::insulated_bases` — card O04-B09B10's premise **F**, where
  the same shape (`& BASE - > FIELD` on a member with no `*`) is one of the
  three argument shapes that leave a base *insulated*. Widening the member class
  therefore also lets a base stay insulated across such a call. That is the same
  argument in the same direction (the callee receives an interior pointer to a
  sub-object whose storage is disjoint from every linkable member), and it is
  in the admitting direction; §6 measures what it did, which is nothing beyond
  the leaves listed there.

The entire blast radius is those two decisions. §6 measures it exhaustively.

---

## 3. The prediction, written before the corpus runs

[`prediction.md`](prediction.md) was written after the single-unit probe and
before either 185-unit run. Its candidate census
([`candidates.py`](candidates.py), transcript
[`logs/candidates.txt`](logs/candidates.txt)) enumerates, over the pinned Fluent
Bit and Monkey sources, every call statement of a **contracted** callee carrying
an `& BASE -> FIELD` actual — a strict over-approximation of what the widening
can reach:

| callee | statements | can it move? |
|---|---|---|
| `mk_list_add` | 444 | **no** — the member is `struct mk_list`, already admitted |
| `pthread_mutex_lock` | 149 | **yes** — the member is `pthread_mutex_t`, the new shape |
| `pthread_mutex_unlock` | 218 | **yes**, likewise |
| `malloc` | 1 | **no** — not a `Borrowed` position |

The prediction was: leaves removed at `pthread_mutex_{lock,unlock}` sites inside
the 185 audited units and at the two `msgpack_packer_init` statements; leaves
added only in the accusing direction, from unpoisoning; the constructor moving
`Unknown` → `Safe` as a **headline**; the unit `flb_input_chunk.c` **not**
moving; and — stated explicitly rather than hoped — the leak pin **not**
predicted to hold, because unpoisoning a statement can surface a leak the
`Unknown` had been hiding.

§6 reports what happened. The prediction is met on every point, including the
one it declined to promise.

---

## 4. Tests

### 4.1 The held-out generic family

`crates/qcore-c-gateway/tests/address_of_member.rs` (new, 15 tests) is an `a_*`
API deliberately unrelated to ChunkIO, Fluent Bit, msgpack, pthread and to the
`b_*`/`g_*`/`h_*`/`k_*`/`m_*`/`r_*` families of the earlier cards. Unlike card
O04-B09B10's prelude it carries the **production spellings**:
`typedef union { … } a_lock_t;`, `typedef struct a_packer { … } a_packer;`, a
plain alias to the first, a pointer typedef, a function-pointer typedef, an
array typedef, a `union` tag, a plain pointer member, an array member and a
`volatile` member — eleven members in one record, so that every row of §2.2's
two tables is decided over one declaration set.

Five admitted rows (the two new typedef shapes, the alias, the `union`-typed
member, and the scalar control that already worked), ten refused rows, plus:

* the base premise (constraint 5) — a base occupying an argument of an
  uncontracted call is never admitted;
* the position premise — `Consumed` and `Released` positions refuse, quantified
  over both custody sinks in scope;
* the containment rows — a whole aggregate member is still not a readable value,
  and an aggregate-typedef member is still not a store destination;
* the doubly-declared typedef (§2.3);
* **independent rederivation**, twice: substituting only the member's typedef
  (to a pointer typedef), only its declarator (to an array) or only its
  qualifier (to `volatile`) in the certificate's own preprocessed source is
  refused by `check_legacy_safety_certificate`; and a certificate whose carried
  contract no longer declares the position `Borrowed` is refused too.

### 4.2 The existing families are unchanged

The whole `qcore-c-gateway` suite — including `o04_b09b10.rs`, `o04_arms.rs`,
`o04_b08.rs`, `o04_read_bound.rs`, `o04_trace.rs`, `borrowed_field_actuals.rs`,
`publication_custody.rs`, `alias_aggregate.rs`, `chained_tail.rs`,
`constructor_residuals.rs`, `loader_abi.rs`, `loader_abi_modes.rs` — passes with
**no assertion changed except two manifest module counts** (§7.2). The only pins
that moved anywhere in the workspace are manifest and corpus **counts**. That is the containment
statement measured at test level.

---

## 5. The constructor, and the two declarations

### 5.1 `msgpack/packer.qcore`, landed on a measurement

Card CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF §3.1 read `msgpack_packer_init` against
its body at the pin —

```c
inline void msgpack_packer_init(msgpack_packer* pk, void* data,
                                msgpack_packer_write callback)
{ pk->data = data; pk->callback = callback; }
```

— and declared all three parameters `resource_borrowed` with
`resource_return_none()`, on card CONSTRUCTOR-RESIDUALS §4.1's ground for
`mk_list_add`: the body allocates nothing, frees nothing and reallocates
nothing, so a caller that owed a release before the call owes exactly the same
release after it. It then measured the declaration's payoff at **zero** and
deliberately did not land it, naming `ABI-TYPEDEF-MSGPACK-UNDECLARED`.

That reading is unchanged and is carried verbatim into
[`contract/qcore/msgpack/packer.qcore`](../../../contract/qcore/msgpack/packer.qcore).
What changed is the measurement, and only the measurement:

| | with the reader widened, declaration **not** landed | declaration landed |
|---|---|---|
| `flb_input_chunk_create` diagnostics | **4** ([`logs/probe-constructor-typedef-only.txt`](logs/probe-constructor-typedef-only.txt)) | **0** |
| its function status | `Unknown` | **`Safe`** |

`msgpack_packer_new` and `msgpack_packer_free`, which **do** acquire and
release, remain deliberately undeclared: no residual this card or its
predecessors measured reaches them, and PROD-CONTRACTS' rule is that a premise
with no measured payoff is not landed.

The gates the declaration had to survive are card
CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF's own, unchanged and re-measured here: the
declared/C ABI comparison agrees with it in both units that mention it (that
card's §4.1 census, run on this card's tree as part of the corpus run — zero
withdrawals of any class appear in either 185-unit log); and the
LOADER-ABI-MODES duty gate is not reached, because the definition is `inline` in
`pack.h` and `LegacyFunctionScope::MainSourceDefinitions` projects no body for
it, so W4 derives no summary and constraint 8's "absence of evidence is not
evidence" governs. `resource_borrowed(callback)` on the `fnptr` parameter is not
a choice among modes: it is the only one the loader admits there, and it is also
the true one.

### 5.2 The constructor's `Safe`, argued by name

[`logs/probe-constructor-after.txt`](logs/probe-constructor-after.txt): status
`Safe`, **0 diagnostics**, six exits, all `Discharged`.

```
exits:
  L19647  node 1 custody Discharged sites ["site0@L19578 Escaped", "site1@L19598 Escaped", "site2@L19615 Escaped"]
  L19622  node 20 custody Discharged sites ["site0@L19578 Released", "site1@L19598 Released"]
  L19602  node 37 custody Discharged sites ["site0@L19578 Released"]
  L19596  node 42 custody Discharged sites ["site0@L19578 Released"]
  L19589  node 47 custody Discharged sites ["site0@L19578 Released"]
  L19582  node 53 custody Discharged sites []
```

Read plainly: the body acquires three identities — the `cio_chunk_open` result
at L19578, the `flb_calloc` result at L19598 and the routes-mask allocation at
L19615 — releases exactly the ones it must on each of the five failure arms, and
on the success arm hands all three to the caller through the returned `ic`,
which the aggregate-store links carry.

**This is `Safe` for a declared custody projection under a named premise set,
and for nothing more.** The premises, in full:

1. **Seventeen declared contracts**, each a *trusted assumption* read against a
   pinned body, none of them proved: `cio_chunk_open`, `cio_chunk_close`,
   `cio_chunk_is_up`, `cio_chunk_up_force`, `cio_chunk_down`, `flb_calloc`,
   `flb_free`, `flb_log_print`, `flb_errno_print`, `flb_time_now`,
   `flb_routes_mask_get_size`, `flb_routes_mask_set_by_tag`,
   `flb_hash_table_add`, `generate_chunk_name`, `input_chunk_write_header`,
   `mk_list_add` and — added here — `msgpack_packer_init`, plus
   `pthread_mutex_lock`/`pthread_mutex_unlock` from card
   CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF. `flb_hash_table_add`'s and `mk_list_add`'s
   **retention is unmodeled**, stated as such in their own modules (`B03`/`R13`);
   `flb_errno_print`'s admission rests on card CONSTRUCTOR-RESIDUALS' refined
   dereferenced-result sentence.
2. **Nine checker admissions**, each with its own report and its own named
   boundaries: C02-P borrowed pointer-field actuals, O04-ARMS bounded typed
   chains, O04-B08 aggregate stores, O04-READ-BOUND bound reads, O04-B09B10
   member addresses and premise **F**, NEIGHBOURS-CHAINED-TAIL's clause T5,
   CONSTRUCTOR-RESIDUALS' dereferenced nested call result, LOADER-ABI's
   declared/C ABI agreement, and this card's widened addressed-member reader.
3. **Two premises this whole family inherits and does not prove**: that the
   declarations read are the ones in force at each access, and that the program
   respects C effective types.
4. **The audited scope** is `LegacyFunctionScope::MainSourceDefinitions`, and
   the ABI comparison's scalar projection assumes the LP64 model of the audited
   targets (`LOADER-ABI-LP64`).

Under invariant 2 the verdict this card publishes is therefore a *function*
status inside a unit whose own verdict is unchanged. **The unit
`fluent-bit-chunk/flb_input_chunk.c` remains provisional `Unsafe`, no raw unit is
promoted, and the Cycle 19 gates are untouched.** A function-level `Safe` inside
a provisionally-`Unsafe` unit is coherent and is exactly what is claimed.

---

## 6. Corpus — before, after, every leaf attributed

### 6.1 The route

```sh
python3 -B scripts/prepare_cycle19_clang_audit.py \
  --fluent-bit /home/cosmo/Documents/GitHub/fluent-bit \
  --revision 66910c10a4d7eafa810b84229cf2dcf7b0f26f97 \
  --output <worktree>/build/cycle19/clang-pin --clang /usr/bin/clang --clangxx /usr/bin/clang++
python3 -B providers/package.py build --output build/cycle19/provider-packages-aom
sh docs/reports/wave20-abi-address-of-member/run-audit.sh <before|after> <binary>
```

Card CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF's three recorded route details are
reproduced and hold: the pinned route reads
`<source>/build/compile_commands.json`, which the preparation script does not
create, so the `fluent-bit-clang-production` database is copied there unchanged
and both runs read the same bytes; `git submodule update --init --recursive` was
run **first**, because the `libevent` pin gate fires at the *end* of a run;
memory was checked before each run (the audit peaked at ~112 MB RSS, so the
OOM hazard that card recorded did not arise, and no run was discarded).

Both runs: **exit 0, zero gateway failures, all six scopes.**

### 6.2 The BEFORE run reproduces the base tree exactly

The BEFORE measurement was taken on the **detached base tree `e01cac2`**, with a
binary built from that tree and executed directly, so no part of this card's diff
— source, contract module or manifest — was on disk while it ran.

| measure | wave-17 frozen archive | BEFORE run |
|---|---|---|
| units | 185 | **185** |
| unit status | 2 `Safe` / 42 `Unsafe` / 137 `Unknown` / 4 data-only | **identical** |
| projected functions | 4,834 | **4,834** |
| function status | 149 / 84 / 4,601 | **identical** |
| `leak` | **272** | **272** |
| `unknown_alias` | 4,405 | **4,405** |
| `unsupported` | 49,835 | **49,833** |

Per library, against the baseline's own table: cmetrics `0/16/15`, ctraces
`1/2/13` (+1 data-only), librdkafka `0/10/81` (+3), chunkio `0/2/11`,
fluent-bit-chunk `0/1/0`, monkey-libevent `1/11/17` — **six for six**.

The one number that differs from the frozen archive is `unsupported`, by exactly
**−2**, and it is not this card's: it is card CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF's
own published movement, the two `call to uncontracted function
pthread_mutex_{lock,unlock}` leaves in `mk_sched_worker_free`, merged into main
between the wave-17 re-pin and this card's base. `diff_artifacts.py` against the
frozen archive
([`artifacts/diff-wave17-to-before.json.gz`](artifacts/diff-wave17-to-before.json.gz))
itemises exactly that and the one contract module that card added, and nothing
else:

```
--- chunk-contracts: NO DIFFERENCES
--- diagnostics:     {'leak': 272, 'unknown_alias': 4405, 'unsupported': 49835}
                  -> {'leak': 272, 'unknown_alias': 4405, 'unsupported': 49833};  units +0 -0 ~1
--- contracts:       sources 14 -> 15 (+1 qcore/libc/pthread_mutex.qcore, 0 changed, 0 removed)
                     manifest modules 14 -> 15 (+1, same module); audit_selection 5 -> 5; keys {}
--- source:          envelope key `compile_entry`; 4 wrapper rows changed
--- source-effects:  3 result rows changed
```

`source` and `source-effects` carry only the **path-literal provenance** and
**identity digest** residue classes that waves 9, 10, 15, 16, 17 and 19 already
record: the artifacts embed this worktree's absolute path and card G01's
compile-task identity binds the digests to the checkout path.

### 6.3 The AFTER run: every moved leaf, attributed

[`census.py`](census.py), transcript
[`logs/census-before-after.txt`](logs/census-before-after.txt):

```
units: 185 -> 185;  added: [];  removed: []
unit status before/after: {'Safe': 2, 'Unknown': 137, 'Unsafe': 42, 'data_only': 4}
unit status moves: 0
NEWLY SURFACED PROVISIONAL UNSAFE UNITS: 0
unit reason_code moves: 0
projected functions: 4834 -> 4834;  function status moves: 2
  FUNCTION (fluent-bit-chunk/flb_input_chunk.c, flb_input_chunk_create): Unknown -> Safe
  FUNCTION (fluent-bit-chunk/flb_input_chunk.c, input_chunk_get):       Unknown -> Unsafe
leak: 272 -> 273 (+1);  unknown_alias: 4405 -> 4403 (-2);  unsupported: 49833 -> 49825 (-8)
units with diagnostic-count changes: 2
leaves added: 3; leaves removed: 12
```

Fifteen leaves moved, in two units and four functions. Every one:

**`fluent-bit-chunk/flb_input_chunk.c` — `flb_input_chunk_create` (6 removed, 0 added; `Unknown` → `Safe`)**

| leaf | attribution |
|---|---|
| L19628 `unsupported: indirect memory access …` — **removed** | `msgpack_packer_init(&ic->mp_pck, ic, flb_input_chunk_write);` — `mp_pck` is now an addressable member (its typedef defines its own `struct`), the position is declared `Borrowed` by the landed `msgpack/packer.qcore`, and the base `ic` is an aggregate-store base this body bound, so `transfer` additionally requires it live here and it is. Modelled as `Call msgpack_packer_init(_, ic, flb_input_chunk_write) -> _ addresses["ic"]` |
| L19637, L19639 `unsupported: indirect memory access …` — **removed** (2) | `pthread_mutex_lock/unlock(&in->metrics_chunk_lock);` — `metrics_chunk_lock` is now addressable (untagged-`union` typedef), the position is `Borrowed`, and the base `in` is untracked-never-bound so no liveness is required |
| L19629 `unknown_alias: unresolved resource identity \`ic\`` — **removed** | consequential: the `Unknown` at L19628 used to poison every pointer on the path, so `ic` could not resolve at the `mk_list_add(&ic->_head, …)` member-address liveness check one line later. With L19628 modelled, it resolves |
| L19647 `unknown_alias: unresolved resource identity \`ic\`` — **removed** | consequential, same cause: `return ic;` now resolves to one live identity |
| L19647 `unsupported: nullable resource \`ic\` used by a non-null contract` — **removed** | consequential, same cause: with the poisoning gone the `if (!ic) { … return NULL; }` guard's narrowing survives to the return, so the nullable acquisition site is discharged rather than carried |

**`fluent-bit-chunk/flb_input_chunk.c` — `flb_input_chunk_map` (1 removed, 1 added; status `Unknown` → `Unknown`, unchanged)**

| leaf | attribution |
|---|---|
| L18755 `unsupported: indirect memory access …` — **removed**, and L18755 `unknown_alias: unresolved resource identity \`ic\`` — **added** | the same `msgpack_packer_init(&ic->mp_pck, ic, flb_input_chunk_write);` statement, in the *other* function that carries it. Here the base `ic` does **not** resolve at that point — the preceding `ic->chunk = chunk;` (L18753) is itself refused and poisons the path — so the modelled call's own liveness premise fires. **One `Unknown`-class refusal is replaced by another at the same statement**, reporting the reaching state instead of the syntactic shape; net zero leaves, no verdict move. This is exactly the pattern card CONSTRUCTOR-RESIDUALS recorded for `mk_list_add` ("row 7's line reports the reaching state instead") |

**`fluent-bit-chunk/flb_input_chunk.c` — `input_chunk_get` (2 removed, 2 added; `Unknown` → provisional `Unsafe`)** — §6.4.

**`monkey-libevent/mk_server/mk_scheduler.c` — `mk_sched_launch_thread` (3 removed, 0 added; status `Unsafe` → `Unsafe`, unchanged)**

| leaf | attribution |
|---|---|
| L6693, L6701, L6708 `unsupported: indirect memory access …` — **removed** (3) | `pthread_mutex_lock(&server->pth_mutex);` and its two `unlock`s. `pth_mutex` is a `pthread_mutex_t` member of a complete `struct mk_server`, `server` is untracked-never-bound, and the positions are `Borrowed`. The function keeps its two `leak from L6694` claims and its `Unsafe`, unchanged: the `mk_mem_alloc_z` obligation is not released on either exit, and every other refusal in the body (`pthread_attr_init`, `pthread_attr_setdetachstate`, `pthread_create`, `mk_utils_libc_error`, `server->pth_init`, `pthread_cond_wait`) is one this card does not touch |

`pthread_cond_wait(&server->pth_cond, &server->pth_mutex)` at L6706 keeps its
refusal, and that is the rule working: `pthread_cond_wait` has no contract in
force, and an uncontracted callee never admits an actual.

**No other unit, no other function and no other leaf moved anywhere in the
185-unit corpus.** The prediction's candidate set bounded this correctly: every
removed `indirect memory access` leaf is a `pthread_mutex_{lock,unlock}` or
`msgpack_packer_init` statement with an `& BASE -> FIELD` actual, and the
`indirect memory access` description inventory moves by exactly **−7**, which is
3 + 1 + 3.

### 6.4 The leak pin moves 271→272→**273**, deliberately, in the accusing direction

`fluent-bit-chunk/flb_input_chunk.c`, `input_chunk_get`, preprocessed line
**19849**: `leak: live obligation acquired at preprocessed line 19835`.
**This is a pin movement and a finding, not a promotion**, and `prediction.md`
declined in advance to promise the pin would hold.

The four leaves, and what happened
([`logs/probe-input-chunk-get-after.txt`](logs/probe-input-chunk-get-after.txt)):

| leaf | attribution |
|---|---|
| L19835 `unsupported: call to uncontracted function \`flb_input_chunk_create\`` — **removed** | this is the W4 custody-inference route, working as designed. `flb_input_chunk_create` is defined in this same translation unit; before this card its body carried six `Unknown` instructions, so no summary could be derived for it and every call of it was an uncontracted call. With the body fully modelled, W4 derives a summary, `observations_match_contract` validates it against a fresh analyzer run, and the call becomes `Call flb_input_chunk_create(in, event_type, _, tag_len) -> ic` |
| L19837 `unknown_alias: unresolved resource identity \`ic\`` — **removed** | consequential: `if (!ic)` one line after the modelled acquisition now resolves |
| **L19849 `leak: live obligation acquired at preprocessed line 19835` — added** | with the acquisition modelled, the arm `if (new_chunk \|\| …) { flb_input_chunk_destroy(ic, 1); return NULL; }` exits carrying it: `flb_input_chunk_destroy` is **uncontracted**, so its statement is refused and the release it performs is invisible to the projection. Exit `L19849 custody Unknown sites ["site0@L19835 Live"]` |
| L19851 `unsupported: nullable resource \`ic\` used by a non-null contract` — added | the success return, on the same acquisition: the constructor's declared result is nullable and this body's guard does not narrow it on that path |

**It moves in the refusing direction, and it stays provisional in the documented
sense.** The function was `Unknown` and is now `Unsafe`; the unit
`fluent-bit-chunk/flb_input_chunk.c` was already provisional `Unsafe` and is
unchanged; nothing became `Safe` at unit level anywhere in the corpus. The claim
rests entirely on `flb_input_chunk_destroy` being uncontracted — a callee whose
name says it frees `ic` and which the projection cannot see — so invariant 2
governs it exactly as it governs the 272 leaves that already stood: **it is not
an upstream defect claim.** Deciding it needs a declaration for
`flb_input_chunk_destroy` read against its body, which is a contract-text
question, not a checker one, and no card in this wave is scoped for it. Recorded
as the follow-on `ADDRESS-OF-MEMBER-DESTROY-UNDECLARED`.

The relation to `CONSTRUCTOR-RESIDUALS-LEAK-271` is worth stating: that finding
is about a *different* leaf in a *different* function
(`flb_input_chunk_get_direct_routes` L19399) resting on a *different*
uncontracted callee (`flb_input_chunk_destroy_direct_routes`). Both are the same
**shape** — a release the projection cannot see because its callee is
undeclared — and both stay provisional. This card withdraws neither.

### 6.5 Against the frozen baseline

`diff_artifacts.py --archived docs/reports/wave17-corpus/artifacts`
([`artifacts/diff-wave17-to-after.json.gz`](artifacts/diff-wave17-to-after.json.gz)):

```
--- chunk-contracts: NO DIFFERENCES (results 96/96, backend_teardown 21/21, +0 -0 ~0)
--- diagnostics:     {'leak': 272, 'unknown_alias': 4405, 'unsupported': 49835}
                  -> {'leak': 273, 'unknown_alias': 4403, 'unsupported': 49825};  units +0 -0 ~2
--- contracts:       sources 14 -> 16 (+2, 0 changed, 0 removed); manifest modules 14 -> 16 (+2)
                     audit_selection 5 -> 5; keys {}
--- source:          envelope key `compile_entry`; 4 wrapper rows changed
--- source-effects:  3 result rows changed
```

Reading the **normalised and raw module keys together**, as constraint 22
requires ([`raw_contract_keys.py`](raw_contract_keys.py)):

```
archived raw keys: 14      measured raw keys: 16
normalised added   : ['…/qcore/libc/pthread_mutex.qcore', '…/qcore/msgpack/packer.qcore']
normalised removed : []
normalised collision (before): False      (after): False
archived checkout roots: ['…/worktrees/agent-aa9f079fdd5560ab5/compiler/../']
measured checkout roots: ['…/worktrees/agent-a10f07ebf60e2e780/compiler/../']
```

There is **no normalisation collision on either side**, so no two modules were
merged into one row and the section was not demoted to raw keys; under
normalisation `contracts.json` gains exactly two modules (one wave-19's, one
this card's), removes none and changes none; and the raw keys differ
additionally only by the checkout root, which is the residue §6.2 already
identified. `source` and `source-effects` carry only the path-literal and
identity-digest classes, unchanged from §6.2 and unrelated to this card's diff.

**The baseline is not re-pinned here.** `docs/reports/wave17-corpus/` remains the
corpus baseline exactly as it stands (constraint 21 — published beside, never
edited). The headline row is unchanged; the leak pin and the function-status
histogram are not, and both are the integrator's to consolidate (§9).

---

## 7. What was touched, and the pins moved deliberately

### 7.1 The diff

| file | what |
|---|---|
| `crates/qcore-c-gateway/src/typed_field_access.rs` | the widening: `TypedefTarget` becomes an enum, `read_aggregate_definition` is new, `read_typedefs` records the aggregate shape, `classify_specifiers` maps it to `Opaque` (behaviour preserved), `is_non_pointer_object` is new, and `read_record_body`'s `non_pointer_fields` insertion reads it |
| `contract/qcore/msgpack/packer.qcore` | new: the declaration card CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF wrote, read against its body, now landed (§5.1) |
| `contract/cycle19-resource-contracts.json` | selected |
| `contract/qcore/README.md` | the manifest paragraph: fifteen modules / 36 declarations → sixteen / 37, with the provenance stated |
| `proof/QCore/MemberAddressReader.lean`, `proof/AuditMemberAddressReader.lean`, `proof/QCore.lean` | §8 |
| `crates/qcore-c-gateway/tests/address_of_member.rs`, `.../fixtures/contracts/address_of_member_generic.qcore` | new: §4.1 |
| four test files | §7.2 |

No provider module, manifest template or `qcore-schemas` major version was
touched, so constraint 18's rebuild-and-verify obligation is not triggered by
this diff. Packages were nonetheless rebuilt for this card's corpus runs
(`build/cycle19/provider-packages-aom`), because the pinned route binds one.

### 7.2 Pins moved deliberately, each keeping its load-bearing half

Editing `contract/qcore` requires updating the pinned count tables
**deliberately** (invariant 5). Six pins moved, and every one moved because the
manifest grew, not because a measured quantity did:

| pin | before → after |
|---|---|
| `compiler/tests/qcore_contract_corpus.rs` corpus size | 70 → **71** modules |
| its `EXPECTED_CONTRACTS` table | one row added: `msgpack_packer_init` (5 clauses) |
| `compiler/tests/contract_loader.rs` corpus module count | 70 → **71** |
| its manifest-selected module count (twice) | 15 → **16** |
| its `(selected, v1_declarations, rejected_declarations)` triple | `(15, 36, 145)` → **`(16, 37, 145)`** — the rejected count is unchanged, because the added module is entirely v1 |
| its explicit borrowed-only parity list | `msgpack_packer_init` added, `Borrowed` at every parameter with no result custody |
| `crates/qcore-c-gateway/tests/neighbours.rs`, `.../prod_contracts.rs` | 15 → **16**, each keeping the by-name assertions that are what those tests actually pin |

Two comment corrections travel with them: `contract_loader.rs` said "14 -> 16"
against a live 15 and claimed three declarations where two were landed — both
slips of card CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF's own text, corrected here
without moving any assertion.

New coverage, not just moved pins: 15 new tests in a new held-out family (§4.1),
and 36 new audited Lean theorems (§8).

---

## 8. Lean

`proof/QCore/MemberAddressReader.lean`, audited by
`proof/AuditMemberAddressReader.lean`: **36 theorems**, depending only on
`propext`, `Classical.choice` and `Quot.sound`, **no new axiom**, no
placeholder. `check_lean_audits.py` moves **85 → 86**. `QCore.MemberAddress`
(card O04-B09B10) is imported unchanged and no theorem of it, or of any other
namespace, was retired, renamed or weakened: this module supplies the `member`
classification that module takes as given, which is exactly the relation this
card changes.

| group | what it fixes |
|---|---|
| `the_classifier_is_unchanged`, `an_aggregate_definition_is_still_opaque_to_the_classifier`, `an_aggregate_definition_behind_an_alias_is_still_opaque` | **containment**: the new reading is recorded where `classify` does not look, so every premise keyed on it decides what it decided before |
| `a_classified_specifier_is_a_non_pointer_object`, `the_premise_only_adds`, `the_widening_only_adds`, `the_widening_moves_no_member_into_the_pointer_class` | **conservativity**, pointwise over every member shape: a member classified `nonPointer` before is `nonPointer` now, and none moved into the `pointer` class. Since `admitsAddress` reads the classification and nothing else about the member, no address refused for another reason can have moved |
| `an_aggregate_typedef_member_was_refused_and_is_now_addressable`, `an_aliased_aggregate_typedef_member_is_addressable`, `a_union_typed_member_was_refused_and_is_now_addressable` | **non-vacuity**, one witness per new shape, the first being the measured production member |
| `an_addressable_member_has_no_declarator_star`, `a_declared_pointer_member_is_never_addressable`, `a_pointer_typedef_member_is_never_addressable`, `a_function_pointer_typedef_member_is_never_addressable`, `an_array_typedef_member_is_never_addressable`, `a_pointer_typedef_behind_an_alias_is_never_addressable`, `an_array_member_is_never_addressable`, `a_qualified_member_is_never_addressable`, `an_unresolved_name_is_never_addressable`, `a_void_member_is_never_addressable` | the laundering shape, refused in **every spelling** — with a star, behind a pointer typedef, behind a function-pointer typedef, behind an array typedef, behind an alias to any of them |
| `an_addressable_member_is_never_linkable`, `a_linkable_member_is_never_addressable` | the **disjointness half** card O04-B09B10 argues in prose, as a theorem: the addressable and linkable member sets are disjoint by construction |
| `no_custody_position_admits_a_read_member`, `a_refused_base_never_admits_a_read_member`, `an_incomplete_record_never_admits_a_read_member`, `a_pointer_member_refuses_under_both_readers` | the admission relation inherits every refusal unchanged, and the last one proves the pointer refusal holds under **both** readers |
| `only_a_bare_name_after_the_body_is_read`, `a_pointer_declarator_after_the_body_is_not_read`, `a_second_declarator_after_the_body_is_not_read`, `an_array_declarator_after_the_body_is_not_read`, `the_aggregate_definition_is_read`, `a_plain_typedef_keeps_its_reading` | the shape recogniser is exact in both directions |
| `a_name_recorded_twice_alike_keeps_its_reading`, `a_name_recorded_twice_differently_is_unresolved`, `an_aggregate_definition_and_a_plain_typedef_of_one_name_retire_it`, `a_retired_name_is_never_addressable` | recording a name twice is fail-closed, and §2.3's pair retires rather than being admitted silently |

`AddressedMember`, `Address`, `Position`, `BaseRoute` and `admitsAddress` are
`QCore.MemberAddress`'s own abstractions, imported unchanged, so the
classification this module produces is literally the one that card's decision
consumes.

---

## 9. Gate

The full standing per-change gate from `AGENTS.md`, at the branch tip:

| step | result |
|---|---|
| `git submodule update --init --recursive` | `third_party/libevent` at `d82464a2` |
| `cargo test --workspace` | **exit 0** — 183 suites, **3,149 passed**, 8 ignored, 0 failed ([`logs/workspace-final.txt.gz`](logs/workspace-final.txt.gz)) |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | **exit 0**, zero warnings |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **exit 0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **exit 0** |
| `python3 scripts/check_lean_audits.py` | **86 audits pass** (85 → 86) |
| `python3 -m unittest discover -s tests` (from `scripts/`) | **192 of 193 pass**; the one failure is the derived Lean-audit count, §9.2 |

Clang is `/usr/bin/clang`, Ubuntu clang 21.1.8, selected explicitly through
`QCORE_CLANG` for every existing-C test and both corpus runs.

### 9.1 Load flakes (constraint 16)

The workspace total is **3,149** rows, 8 ignored — the merged base's 3,134 plus
exactly this card's 15 new tests (§4.1).

The final run at the committed tip is **fully green**. Two earlier runs, taken
while sibling agents and this card's own 185-unit audit were occupying the
machine, each tripped one row, and both are recorded rather than dropped.

One row tripped under the sibling load this card ran against:
`qcore-verifier-host`'s
`enforced_process_limits::memory_growth_beyond_the_declared_limit_is_bounded`
failed with `assertion left == right failed: Broken pipe (os error 32); left:
Io, right: Terminated` at load average ~19. That is the exact `Io(Broken pipe)`
shape constraint 16 records for the confined-fixture write test — a sibling row
of the same `enforced_process_limits` family, whose fixture is killed by the
enforced limit while the host is still writing to its stdin, so the host
observes the broken pipe before the kill. Re-run in isolation at load average
12.8 the **whole file passes, 12 of 12**
([`logs/flake-retry.txt`](logs/flake-retry.txt)), and it passes again in the
green whole-workspace run above. Nothing was weakened; the row is load-sensitive
in exactly the documented way, and it is a **fifth** row of that family behaving
so — recorded for the integrator as a candidate addition to constraint 16's
named list.

An earlier, aborted workspace run (taken while the 185-unit audit was still
occupying the machine, load average ~40) tripped
`vp04_vp07_vp08_process_lifecycle_and_payload_budgets` instead, which **is** one
of constraint 16's four named rows and which passed in the clean re-run above.
The other three named rows did not trip in either run.

**Providers.** No provider module, manifest template or `qcore-schemas` major
version was touched, so constraint 18's rebuild-and-verify obligation is not
triggered. Packages were rebuilt for this card's corpus runs because the pinned
route binds one.

### 9.2 One expected failure, in a file this card may not edit

`scripts/tests/test_build_matrix_sections.py::test_the_derived_facts_are_recomputed_rather_than_carried`
fails with `['14.0.0', 85] != ['14.0.0', 86]`. That is the acceptance matrix's
**derived** `validation.lean_audits` field, carried in
`docs/reviews/r0_3/acceptance_matrix.json`, which this card is not permitted to
edit and which is regenerated by the integrator. The test is doing exactly its
job — it is a tripwire that fires when a derived count moves — and the movement
is this card's one new audit. **Integrator reconciliation item**, §11.

---

## 10. The increment paragraph (draft — for the integrator, not applied here)

> **Card ABI-TYPEDEF-ADDRESS-OF-MEMBER (wave 20).** Card
> CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF widened the *declared-ABI* reader's typedef
> handling and then measured that the refusal actually binding at production
> `flb_input_chunk_create`'s three residual statements was the importer's
> address-of-a-member rule, naming it `ABI-TYPEDEF-ADDRESS-OF-MEMBER`. This card
> measured *which* conjunct of that rule failed — not the base, not the record
> completeness, not the route, but the member's own declared type — and found
> the same typedef defect in a **second, independent** reader,
> `qcore-c-gateway::typed_field_access`, whose `read_typedefs` abandoned a
> `struct`/`union`/`enum` a typedef *defines* at its own opening brace. That
> shape is now read, exactly (`struct|union|enum [TAG] { … } NAME`, nothing
> between the body and the name), and card O04-B09B10's addressed-member premise
> moves from "the classifier answers `struct TAG` or arithmetic" to "the
> specifier list names an object type that is not a pointer type" — which is all
> that rule's disjointness argument ever consumed. A `union`-typed member of a
> `struct` joins the class on the same ground. The classifier itself answers
> exactly what it answered before, so every other premise keyed on it decides
> byte-identically; the pointer-member refusal is kept in **every** spelling,
> including behind a pointer, function-pointer or array typedef, and a name
> typedef'd twice to disagreeing readings retires fail-closed.
> `msgpack/packer.qcore` — written, read against its body and measured at payoff
> **zero** by the previous card — lands here at a measured payoff of four leaves
> and one function verdict, closing `ABI-TYPEDEF-MSGPACK-UNDECLARED` on
> PROD-CONTRACTS' own rule. **Production `flb_input_chunk_create` reaches
> function-level `Safe`**, six diagnostics to zero and six exits of six
> `Discharged`, closing the production-reach item `O04-TRACE-REACH`; the premise
> set is named in full and the unit stays provisional `Unsafe`, so no raw unit is
> promoted. Two 185-unit runs: BEFORE at base `e01cac2` reproduces the frozen
> wave-17 archive in every number except the two leaves card
> CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF itself published. Corpus movement is **12
> diagnostic leaves removed and 3 added**, in two units and four functions, every
> one attributed: **zero unit status moves, zero unit `reason_code` moves, zero
> newly surfaced provisional `Unsafe` units**, and **two** function status moves
> — `flb_input_chunk_create` `Unknown` → `Safe`, and `input_chunk_get` `Unknown`
> → provisional `Unsafe`, the latter because the constructor call it makes is
> now modelled through a validated W4 summary while the release on one arm goes
> through the uncontracted `flb_input_chunk_destroy`. **The leak pin moves 272 →
> 273**, deliberately, in the accusing direction, on that one leaf, and stays
> provisional in the documented sense (`ADDRESS-OF-MEMBER-DESTROY-UNDECLARED`).
> Contract corpus 70 → 71 modules, manifest 15 → 16, declarations 36 → 37. Lean
> `QCore.MemberAddressReader`, 36 theorems, no project axiom; audits 85 → 86.
> Gate: 3,149 workspace tests (8 ignored), fmt/clippy clean, both probes exit 0,
> 86 Lean audits, 192 of 193 Python — the one failure being the acceptance
> matrix's own derived Lean-audit count, which the integrator regenerates.

Headline numbers for `AGENTS.md`: contract corpus **71 modules**; Lean audits
**86**; the 185-unit row is **unchanged** (2 / 42 / 137 / 4); the **leak pin
moves 272 → 273** and the function-status histogram moves 149/84/4,601 →
150/85/4,599.

---

## 11. Named follow-ons

* **`ADDRESS-OF-MEMBER-DESTROY-UNDECLARED`** — the leaf that moves the leak pin
  (§6.4). `input_chunk_get` releases through `flb_input_chunk_destroy`, which is
  uncontracted, so the release is invisible and the arm reports a leak. Deciding
  it is a contract-text question: read `flb_input_chunk_destroy` against its body
  and declare it, exactly as card CONSTRUCTOR-RESIDUALS did for
  `flb_errno_print` once its blocker moved. Until then the leaf stays
  provisional under invariant 2 and is never an upstream defect claim. It is the
  same **shape** as `CONSTRUCTOR-RESIDUALS-LEAK-271`, on a different function and
  a different callee; this card withdraws neither.
* **`ADDRESS-OF-MEMBER-MAP-LIVENESS`** — `flb_input_chunk_map` L18755 now reports
  `unresolved resource identity ic` where it reported `indirect memory access`
  (§6.3). The statement is admitted and the *aggregate's* liveness premise is
  what refuses, because the preceding `ic->chunk = chunk;` is itself refused —
  `chunk` there is a `void *` member whose store rule card ALIAS-AGGREGATE
  admits only for a declared record pointer. Sizing this is an aggregate-store
  question, not an address-of-member one.
* **`ADDRESS-OF-MEMBER-TAGGED-PAIR`** — a unit carrying both
  `typedef struct tag { … } tag;` and `typedef struct tag tag;` retires the name
  (§2.3), which is the refusal it already had. Composing the pair the way card
  CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF composed it in the *other* reader is
  possible and costs nothing measured today; it needs a unit that needs it.
* **`ADDRESS-OF-MEMBER-INLINE-AGGREGATE`** — a member declared over an aggregate
  written **inline** at the member position (`struct { int x; } inner;`) still
  refuses the whole record, because the record body reader is all-or-nothing and
  a brace in the body refuses it. That is the record reader's boundary, not this
  card's, and widening it is a much larger change than this one.
* **`ADDRESS-OF-MEMBER-DOT-FORM`** — `& BASE . FIELD` on a record-valued local is
  still refused: the admitted spelling is exactly `& BASE - > FIELD`. The `.`
  form has the same disjointness argument and no measured site.
* **`ABI-TYPEDEF-MSGPACK-UNDECLARED` is CLOSED** by §5.1: the declaration is
  landed at a measured payoff.
* **`O04-TRACE-REACH` is CLOSED** by §5.2 for `flb_input_chunk_create`: the
  production constructor now has admitted statements and a function-level
  verdict on byte-verbatim production source. The *unit* reach item is not
  closed and is not claimed.
* **`ABI-FNPTR-OPAQUE`, `ABI-FNPTR-DEPTH1`, `ABI-TYPEDEF-INLINE-DECLARATOR`,
  `LOADER-ABI-UNDECLARED`, `LOADER-ABI-LP64`, `NEIGHBOURS-FIXTURE-PROXY`** are
  carried unchanged from cards CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF, LOADER-ABI and
  NEIGHBOURS. This card exercises `fnptr` on the corpus for the first time (the
  landed `msgpack_packer_init`), which is the production exercise
  `ABI-TYPEDEF-MSGPACK-UNDECLARED` said it was waiting for; the opacity itself is
  unchanged.
* **`DIFF-TOOL-CARD-DIFFS`** applies to this card's artifact diffs, which live
  under [`artifacts/`](artifacts) and are outside the archived consolidation
  chain (constraint 22).
