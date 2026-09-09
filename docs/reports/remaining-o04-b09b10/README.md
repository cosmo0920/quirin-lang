# O04-B09B10 — member addresses and custody-taking bound reads — 2026-09-09

The named continuation of card O04-READ-BOUND
([`docs/reports/remaining-o04-read-bound/README.md`](../remaining-o04-read-bound/README.md)
§7), whose success exit C6 carried nine remaining refusals, and of O00's
blockers **B09** and **B10**
([`docs/reports/remaining-o00/README.md`](../remaining-o00/README.md) §7).

**C6 does not settle. It narrows from nine refusals to one**, and that one is
owned by two named follow-ons neither of which is this card. That is the honest
headline in both halves.

| Arm | O04-ARMS | O04-B08 | O04-READ-BOUND | This card |
|---|---|---|---|---|
| **C1** — logging arm, `!chunk` | Discharged | Discharged | Discharged | **Discharged** |
| **C2** — `up_force == -1` | Discharged | Discharged | Discharged | **Discharged** |
| **C3** — header write `== -1` | Discharged | Discharged | Discharged | **Discharged** |
| **C4** — `ic` calloc NULL | Discharged | Discharged | Discharged | **Discharged** |
| **C5** — routes-mask arm | `Unknown` | Discharged | Discharged | **Discharged** |
| **C6** — success + publication | `Unknown` (10 refusals) | `Unknown` (10) | `Unknown` (9) | **`Unknown` — 1 refusal** |

**Nothing is promoted.** Both real functions keep function-level status
`Unknown`; the 185-unit headline is **2 Safe / 42 provisional Unsafe /
137 Unknown / 4 data-only**, unchanged, with **zero unit and zero function
verdict movement** and **no newly surfaced provisional `Unsafe` unit**; the
contract corpus stays at 60 modules; CT coverage stays 31/50;
`qcore-schemas` is unchanged at 8.0.0.

---

## 1. The nine-refusal census, and what decides each row

Measured before any change, through the reporting aid
`print_the_measured_arm_ledger` in
[`crates/qcore-c-gateway/tests/o04_b08.rs`](../../../crates/qcore-c-gateway/tests/o04_b08.rs);
the after-measurement uses this card's own aid in
[`o04_b09b10.rs`](../../../crates/qcore-c-gateway/tests/o04_b09b10.rs). Both
transcripts are in [`logs/`](logs/).

| # | Statement (preprocessed line) | Refusal before | Admission family that decides it | Existed? | Now |
|---|---|---|---|---|---|
| 0 | `flb_trace("…", flb_input_chunk_get_name(ic), tag);` (255) | uncontracted callee — **deliberate** in O04-B08/READ-BOUND | (a) a declared **variadic tail** form and (b) **nested-call modeling** | **neither** | **unchanged** (§7) |
| 1 | `msgpack_packer_init(&ic->mp_pck, ic, flb_input_chunk_write);` (258) | `INDIRECT` | **member-address actual**, bound base | **no** | **admitted** |
| 2 | `mk_list_add(&ic->_head, &in->chunks);` (259) | `INDIRECT` | **member-address actual**, bound base *and* untracked base, twice in one statement | **no** | **admitted** |
| 3 | `pthread_mutex_lock(&in->metrics_chunk_lock);` (267) | `INDIRECT` | **member-address actual**, untracked base | **no** | **admitted** |
| 4 | `pthread_mutex_unlock(&in->metrics_chunk_lock);` (269) | `INDIRECT` | as row 3 | **no** | **admitted** |
| 5-8 | four `flb_hash_table_add(in->ht_*_chunks, tag, tag_len, ic, 0);` (264, 268, 272, 275) | `INDIRECT` | C02-P's **existing** member-read family, blocked only by **absent declared contract text** | read family **yes**, contract text **no** | **admitted** |
| 9 | `return ic;` — `UnknownAlias{ic}` + `nullable resource ic used by a non-null contract` (277) | the identity residue | **none**: measured to be a *consequence* of row 0 (§3.2) | — | **unchanged** |

Rows 1-8 are exactly the eight `INDIRECT` refusals card O04-READ-BOUND counted;
row 9 is its "identity residue"; row 0 was recorded there as deliberate. All
eight `INDIRECT` refusals are gone. Rows 0 and 9 remain, and §3.2 measures that
row 9 is nothing but row 0's shadow.

---

## 2. What the card admits

Two analyzer capabilities and one fail-closed repair, in
[`aggregate_store.rs`](../../../crates/qcore-c-gateway/src/aggregate_store.rs),
[`typed_field_access.rs`](../../../crates/qcore-c-gateway/src/typed_field_access.rs)
and [`legacy_c.rs`](../../../crates/qcore-c-gateway/src/legacy_c.rs).

### 2.1 Member-address actuals — `& BASE - > FIELD` (`member_address_actuals`)

| # | Premise | Refused otherwise |
|---|---|---|
| 1 | readable declarations (`FunctionTypes::is_readable`) | the whole function admits nothing |
| 2 | the argument is exactly `& BASE - > FIELD` and is the **whole** argument | a cast, parentheses, arithmetic, a chain, a subscript, `&BASE`, a partial argument |
| 3 | `BASE` is declared exactly `struct TAG *`, and `TAG` is a `struct` defined once whose every member declaration was read | typedefs the reader will not resolve, `union` tags, ambiguous or incomplete records |
| 4 | `FIELD` is declared with **no** `*`, no array modifier and no rejecting qualifier, over a specifier list the reader resolved (`TypedAccessModel::addressable_field`) | every pointer member, `void *`, multi-star, arrays, bit-fields, `volatile`/`_Atomic`, unresolved typedefs |
| 5 | the position is declared **`Borrowed`** by a contract in force | every custody-bearing position, every uncontracted callee, every variadic tail |
| 6 | `BASE` is either a `store_bases` base this body bound **or** a C02-P `untracked_bases` name | every other base |

**Why premise 4 is the whole rule.** A member with no `*` is a *sub-object*.
Under the C effective types premise this family inherits, its storage is
disjoint from every other member of the same `struct`, and this projection links
only single-`->` members of the base. So a callee holding `&BASE->FIELD` can
reach no field link at all. A **pointer** member's address is the opposite: a
handle with which the callee could overwrite the very link the projection
maintains. That asymmetry is what separates `&ic->mp_pck` (admitted) from
`&ic->chunk` (refused), and it is proved in Lean (§6).

**Flow-sensitively**, when the base is one this body bound, `legacy_c::transfer`
requires it to resolve *here* through the unchanged `resolved_aggregate_owner`:
one live identity this body acquired, unique, not `unknown`, not possibly null,
not caller-held, no unresolved effect. Handing out the address of a member of a
released aggregate is card C00's mistake in this direction, and it is refused.
An untracked base needs no such check: by premise it carries no identity at all.

One existing gate was narrowed, deliberately and only for arguments this rule
admitted: `call has an unmodeled formal address or borrowed expression` fired on
*any* `&` argument mentioning a protected name. Its purpose is exactly the frame
condition premise 4 supplies, so an admitted member address is exempt and every
other `&` argument keeps its refusal.

### 2.2 Custody-taking bound reads — premise **F** (`bound_read_actuals`)

Card O04-READ-BOUND admitted a bound member read only at a `Borrowed` position
and recorded the gap as **`O04-READ-BOUND-CUSTODY`**: "a contracted callee that
merely borrows the aggregate could rewrite the member invisibly and this
projection maintains no frame condition for that."

This card supplies that frame condition and admits `Released` and `Consumed`
positions under it. A base is **insulated** when no *contracted* call of the
body can leave the aggregate both live and rewritable; only three argument
shapes mentioning the base are permitted, each with its own reason:

* exactly `BASE - > FIELD` — a *read* of a member cannot write one;
* exactly `& BASE - > FIELD` on a member premise 4 admits — the callee receives
  storage disjoint from every linkable member. The premise that makes §2.1 safe
  is the same one that preserves F, which is why one member classification
  decides both rules;
* exactly `BASE` at a `Released` or `Consumed` position — afterwards the
  identity is not live, so no later read through it can resolve at all. This is
  what lets a body hand its aggregate to its own destructor without losing the
  premise; the first implementation forbade it and made the whole family
  unreachable for any realistic body.

**Uncontracted** calls need no premise, by the same argument `store_bases`
premise 5 already records: such a statement is refused, and a refused statement
sets every pointer `unknown`; `AbstractState::join` propagates `unknown` on
either incoming edge, so a poisoned path stays poisoned through a join. If the
uncontracted call can precede the read, the read fails its flow-sensitive owner
check; if it cannot, it cannot have changed what the read saw.

An admitted custody read applies the contract's own effect to the linked child
and then **drops both the place binding and the field link**. A second read of
the same member then fails `bound_read_resolves` instead of releasing the same
identity twice, and an escape of the aggregate no longer cascades to a child it
no longer holds.

`Owned`, `Stored`, `Removed` and `Out` still refuse in full — the first three
are additionally refused at the base by `store_bases` premise 5.

### 2.3 A pre-existing false `Safe`, repaired (`nested_call`)

`find_call` returns the **first** call of a statement, and everything downstream
models exactly that one. A call nested inside an argument therefore had its
acquisitions, releases and transfers dropped **without a diagnostic**. Measured
directly, through the production entry point, before any change of mine:

```
k_borrow(k_take(1));   ->   Safe, no leak, certificate rechecks
k_take(1);             ->   Unknown: discarded owned result of resource API `k_take`
```

The nested acquisition is an owned, non-null result that nothing discharges.
This is a false `Safe`, not a coverage gap, and this card's own subject makes it
reachable: declaring the five callees of §4 turns statements that used to refuse
lexically into modeled calls. It is repaired rather than recorded: a nested call
inside a **contracted** call now refuses, naming the callee. The C type
operators (`sizeof`, `_Alignof`, `alignof`, `typeof`, `__typeof`, `__typeof__`)
keep their exemption — they evaluate no operand and appear inside arguments the
existing measurements already admit, such as `flb_calloc(n, sizeof(T))`. An
*outer* call this analyzer does not model already refuses on its own, so its
existing diagnostic is untouched.

The repair fires on the pinned corpus (§5.2), and one of the nine statements it
names was previously dropped silently.

---

## 3. The measured per-arm disposition

Raw ledgers: [`logs/arm-ledger-before.txt`](logs/arm-ledger-before.txt) and
[`logs/arm-ledger-after.txt`](logs/arm-ledger-after.txt).

### 3.1 `flb_input_chunk_create` — six exits

| O00 exit | Line | Arm | O04-READ-BOUND | This card | Residual |
|---|---|---|---|---|---|
| C1 | 2530 | constructor returned NULL | Discharged | **Discharged** | none acquired |
| C2 | 2541 | `up_force == -1` | Discharged | **Discharged** | chunk Released |
| C3 | 2550 | header write `== -1` | Discharged | **Discharged** | chunk Released |
| C4 | 2558 | `ic` calloc NULL | Discharged | **Discharged** | chunk Released |
| C5 | 2587 | `routes_mask` calloc NULL | Discharged | **Discharged** | chunk + `ic` Released |
| C6 | 2620 | success + publication | Unknown (9 refusals) | **Unknown (1 refusal)** | all three `Mixed` |

Site counts `[0,1,1,1,2]` on the five settled exits, every site `Released`, all
four cleanup arms naming the same acquisition site: unchanged. No exit is a
`Residual`, no `Leak` is reported, and C6 carries its three identities as
`Mixed`, never as a claim. Pinned by
`the_success_exit_residual_narrows_to_one_statement`, which asserts the exact
remaining diagnostic vector so a silent change in either direction fails.

### 3.2 What the last refusal costs — one labelled control

The two diagnostics on `return ic;` could be an independent blocker or a
consequence of row 0. That is a question measurement can answer, so it was
measured rather than argued.

**Control** (clearly non-verbatim, and asserted as such): replace the one
refused statement with an already-contracted, argument-free call
(`flb_errno();`). Everything else is byte-verbatim upstream.

```
flb_input_chunk_create: Safe
  exit L276 Discharged sites=[(206, Escaped), (227, Escaped), (244, Escaped)]
  exit L251 Discharged sites=[(206, Released), (227, Released)]
  exit L231 Discharged sites=[(206, Released)]
  exit L225 Discharged sites=[(206, Released)]
  exit L218 Discharged sites=[(206, Released)]
  exit L211 Discharged sites=[]
```

Every exit settles; C6 escapes all three identities — the chunk and the routes
mask as live linked children of the escaping aggregate (premise
**P-B08-SHALLOW**), and `ic` itself as the returned value. So **row 9 is row 0's
shadow**, and C6's whole residual is one statement.

This is a control, not a claim about the upstream source. On the byte-verbatim
bodies C6 is `Unknown`, and the assertion in §3.1 is what governs. Pinned by
`the_logging_control_settles_every_exit`.

### 3.3 What row 0 would need

`flb_trace("…", flb_input_chunk_get_name(ic), tag)` needs **two** families this
card does not build, and neither is a variation of what it does build:

1. a **declared variadic tail** — `flb_trace` is
   `void flb_trace(const char *, ...)` and the resource-contract vocabulary has
   no variadic form. Card O04-ARMS met the same wall for `flb_error` and
   recorded it as **`O04-ARMS-VARIADIC`**; declaring it at this one call site's
   arity would cover exactly three arguments and refuse every other arity;
2. **nested-call modeling** — the second argument is
   `flb_input_chunk_get_name(ic)`, a call. §2.3 has just made that an explicit
   refusal *because nothing models it*; declaring `flb_trace` would therefore
   move row 0 from `call to uncontracted function` to
   `nested call flb_input_chunk_get_name inside a modeled call` and change
   nothing else. That was measured, not assumed, which is why `flb_trace` is
   deliberately still **not** declared.

Recorded as the named follow-on **`O04-B09B10-TRACE`** (§7).

### 3.4 `cio_chunk_open` — four settled exits, unchanged

Four guard/allocation-failure exits stay Discharged; the rollback and success
exits stay Unknown, each carrying two `Mixed` sites. Seven `INDIRECT` refusals
remain, `free(ch->name)` among them.

The reason it is refused is now **checked** rather than a blanket position rule:
under this fixture's premise set `cio_error_reset(ch)` and `cio_chunk_is_up(ch)`
hand the chunk whole to a *contracted* callee at a `Borrowed` position, so
premise F refuses `ch` and no frame condition for `ch->name` is available. In
the pinned 185-unit corpus neither of those callees is contracted, so F does not
refuse `ch` there and the *flow-sensitive* gate refuses the same statement
instead — both refusals, both fail-closed, and both measured (§5.2). Pinned by
`the_constructor_boundary_is_unchanged`.

---

## 4. Every entry premise, named

O04's nine premises, O04-ARMS's two, O04-B08's two (**P-B08-SHALLOW** and the
`flb_time_now` declaration) and O04-READ-BOUND's none carry over unchanged and
are still **assumed, not derived** — including that the constructor's own body
does not discharge its declared contract. Two premises are inherited from C02
and are **not** proved: declaration completeness, and C effective types. C02-P's
recorded may-alias boundary (**C02-P-ALIAS**) is inherited and not widened.

This card adds **five** declared callee premises, in the new fixture module
[`o04_b09b10_callees.qcore`](../../../crates/qcore-c-gateway/tests/fixtures/contracts/o04_b09b10_callees.qcore),
which lives with the gateway test fixtures so the pinned `contract/qcore` corpus
count of 60 is untouched. Each is a **trusted assumption**, and each was checked
against the pinned upstream sources rather than assumed:

| Callee | Declaration | Faithfulness, measured |
|---|---|---|
| `mk_list_add(entry, head)` | borrows both | faithful as to custody: it splices two list nodes. The **containment** it creates is O00 blockers B03/R13 and stays unmodeled |
| `msgpack_packer_init(pk, data, callback)` | borrows all three | faithful: it copies two pointers into `*pk` |
| `flb_hash_table_add(ht, key, key_len, val, val_size)` | borrows all five | faithful **at these call sites only**. `flb_hash_table_entry_free` (`src/flb_hash_table.c:93` at the pin) frees `entry->val` exactly when `entry->val_size > 0`, and all four call sites pass the literal `0`. The vocabulary has no value-dependent mode, so the declaration also covers positive-size call sites where it would be **unfaithful** — recorded as **`O04-B09B10-VALUE-DEPENDENT`** |
| `pthread_mutex_lock`/`_unlock(mutex)` | borrows the mutex | faithful as to *custody*. This projection models resource custody, not mutual exclusion: the declarations say nothing about locking, ordering or data races |

What the declarations assert is only that no call transfers custody of a tracked
identity out of this function's accounting — which is what a per-exit
disposition needs and all it needs, because the projection's per-exit claim is
that every identity is **accounted for**, never that it is **exclusively held**.

---

## 5. Corpus impact — zero verdict movement, attributed statement by statement

### 5.1 Provenance and harness validation

Re-measured through the canonical pinned route on the shared prepared pinned
clone (Fluent Bit `66910c10a4d7eafa810b84229cf2dcf7b0f26f97`, explicit
`/usr/bin/clang` 21.1.8), following
[`docs/reports/corpus-repin/provenance.json`](../corpus-repin/provenance.json):

```sh
python3 -B providers/package.py build --output build/cycle19/provider-packages-b09b10
sh build/cycle19/run_audit.sh build/cycle19/reports-{before,after}
#   = QCORE_CLANG=/usr/bin/clang \
#     QCORE_CHUNKIO_PACKAGE=build/cycle19/provider-packages-b09b10/qcore.chunkio \
#     cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
#       --pinned-evidence <clone> <monkey>/compile_commands.json \
#       <out>/monkey-source.json <out>/resource-diagnostics.json
```

Run **twice from this same worktree**, with only `crates/qcore-c-gateway/src/`
reverted to the branch base `67961f1` between runs, so the comparison carries no
path provenance at all. Both exit 0 with zero gateway failures. Transcripts:
[`logs/corpus-before.log.gz`](logs/corpus-before.log.gz),
[`logs/corpus-after.log.gz`](logs/corpus-after.log.gz). `build/cycle19/` is
gitignored, so `run_audit.sh` is not committed; the command it runs is complete
above. Nothing in the shared clone was written to.

**The BEFORE run against the archived wave-11 baseline**
([`docs/reports/wave11-corpus/artifacts/`](../wave11-corpus/artifacts/)):

| Artifact | before vs archived |
|---|---|
| `diagnostics.json` | **byte-identical** (SHA-256 `a606285bbeb11b2b…`) |
| `chunk-contracts.json` | **byte-identical** (SHA-256 `c4f438e4ed3dab3c…`) |
| `source.json` | **byte-identical** (SHA-256 `53d55b7e1fb174de…`) |
| `contracts.json` | differs — **path-literal provenance residue only** |
| `source-effects.json` | differs — **package-identity residue only** (packages rebuilt for this run) |

Both residue classes were recorded by CORPUS-REPIN, O04-ARMS, O04-B08, E01-F1,
O04-READ-BOUND and the wave-9/10/11 consolidations, and both are **identical
between before and after here**, so they do not touch the isolation.

### 5.2 Before → after: one artifact moves, exhaustively attributed

| Artifact | before | after |
|---|---|---|
| `diagnostics.json` | `a606285bbeb11b2b…` | `8a0301d667703801…` |
| `chunk-contracts.json` | `c4f438e4ed3dab3c…` | **same** |
| `source.json` | `53d55b7e1fb174de…` | **same** |
| `source-effects.json` | `13d03dd1295142f0…` | **same** |
| `contracts.json` | `3c113cadd6669ec8…` | **same** |

The independent census ([`census.py`](census.py), transcript at
[`logs/census.txt`](logs/census.txt)) compares the two runs directly rather than
trusting digests:

```
units: 185 -> 185;  added: [];  removed: []
unit status before/after: {'Safe': 2, 'Unknown': 137, 'Unsafe': 42, 'data_only': 4}
unit status moves: 0
NEWLY SURFACED PROVISIONAL UNSAFE UNITS: 0
unit reason_code moves: 0
projected functions: 4834 -> 4834;  function status moves: 0
leak: 271 -> 271 (+0);  unknown_alias: 4387 -> 4399 (+12);  unsupported: 49936 -> 49925 (-11)
```

**No newly surfaced provisional `Unsafe` unit**, at any level: identical status
histogram, no unit row changed `status` or `reason_code`, all 4,834 projected
functions compared pairwise with zero moves, and the corpus `leak` count
unchanged at 271. That was the direction to check first, and the measurement is
exhaustive rather than sampled.

Every changed diagnostic is attributed to a statement by
[`attribute.py`](attribute.py) (transcript at
[`logs/attribution.txt`](logs/attribution.txt)) — **nine units, 21 rows, all
one-for-one**:

| Class | Units | Rows | Movement |
|---|---|---|---|
| **A — custody-taking bound reads now modeled** | `chunkio/cio_chunk.c` (`ch`), `chunkio/cio_memfs.c` (`mf`), `ctraces/ctr_encode_opentelemetry.c` (`kv`), `monkey-libevent/mk_server/mk_fifo.c` (`fw`), `fluent-bit-chunk/flb_input_chunk.c` (`ic`, 8 rows in `flb_input_chunk_map`) | 12 | `indirect memory access …` → `unresolved resource identity …`. The statement is now *modeled*; the flow-sensitive gate refuses it because the aggregate does not resolve there, the paths having been poisoned by uncontracted calls. A refusal becomes a **more specific** refusal |
| **B — nested-call repair, renaming an existing refusal** | `mk_cache.c` (4), `mk_scheduler.c` (3), `mk_server.c` (1) | 8 | `arguments do not match resource contract for mk_mem_free` → `nested call pthread_getspecific inside a modeled call …`. Same statement, same refusal, accurate reason |
| **C — nested-call repair, a refusal that was previously dropped** | `librdkafka/rdkafka_request.c`, `rd_kafkap_share_leader_changes_apply` | 1 | **new** `nested call rd_list_cnt inside a modeled call …`. This is the §2.3 class occurring in real source: the nested call's effects used to be dropped without a diagnostic |

No row moves in the permissive direction. Class C is the only *added* refusal,
and it is exactly the repair working.

**Measured reach of the member-address family in the pinned corpus is zero
admitted statements**: the corpus contract set has 19 declarations, of which
only four have a `Borrowed` **pointer** parameter at all
(`cio_chunk_open` indices 0/2/4/8, `flb_sds_create_len`, `rd_strdup`,
`rd_strndup`), and none of the corpus's `& x -> f` arguments occupies one. As in
cards C03-NFR and O04-READ-BOUND, the payoff gates on how much declared contract
text exists; a wider corpus contract set would widen it.

### 5.3 The configured audits

Run in this same worktree, before and after, with only
`crates/qcore-c-gateway/src/` reverted between runs.

| Audit | Result | before vs after |
|---|---|---|
| `contract/profiles/cmetrics-counter` | 1 unit: 0 Safe / 1 provisional Unsafe / 0 Unknown; 1 checked function contract / 6 Unknown | **byte-identical transcript** |
| `contract/profiles/chunkio-core` | 13 units: 0 Safe / **2 provisional Unsafe** / 11 Unknown | verdicts identical; two diagnostic counts move |
| `contract/profiles/libevent-core` | **not run** | the audit needs a prepared `build/cycle19/libevent-clang-build` CMake database that does not exist in this environment; it rejects identically before and after |

`chunkio-core`'s movement is class A above and nothing else:
`cio_chunk.c` `unsupported 147 → 146` / `unknown_alias 19 → 20`, and
`cio_memfs.c` `unsupported 34 → 33` / `unknown_alias 7 → 8`. Transcripts in
[`logs/`](logs/).

---

## 6. Lean parity

**54 audits** (53 → 54). No new axiom, no placeholder, no `sorry`.

### 6.1 New — [`proof/QCore/MemberAddress.lean`](../../../proof/QCore/MemberAddress.lean), 35 theorems

Registered as [`proof/AuditMemberAddress.lean`](../../../proof/AuditMemberAddress.lean).
It reuses `QCore.AggregateStore`'s `Disposition`, `Owner` and `ownerResolves`
and `QCore.BoundFieldRead`'s `Position`, so the three decisions are stated over
one vocabulary rather than three copies.

* **Every premise refuses on its own** — eight theorems, including
  `a_pointer_member_address_is_never_admitted`, which is the rule the whole
  admission rests on.
* **Position** — `a_custody_bearing_position_never_admits_an_address` is
  quantified over the *whole* vocabulary so a later widening cannot slip a mode
  past it, with a per-mode corollary and the contrapositives
  `an_admitted_address_is_borrowed` and
  `an_admitted_address_has_a_non_pointer_member`.
* **The flow-sensitive gate** — five theorems, including
  `an_address_through_a_released_aggregate_never_resolves` (card C00's lesson in
  this direction) and `an_untracked_base_always_resolves`, which states exactly
  why the C02-P route needs no liveness check.
* **Fail-closed transfer, both directions** — `a_refused_address_is_never_handed`,
  `an_unresolved_address_is_never_handed`, and
  `only_a_doubly_admitted_address_is_handed`, which shows the two gates are
  *jointly* necessary.
* **What an admitted address does, and does not, do** —
  `an_admitted_address_changes_no_disposition`,
  `an_admitted_address_never_discharges`, and
  `an_admitted_address_names_no_linkable_member`.
* **The tie to premise F** — `an_admitted_address_preserves_insulation` and
  `a_pointer_member_address_breaks_insulation`: the premise that makes §2.1 safe
  is the one that preserves §2.2's frame condition.
* **Non-vacuity in both directions** — an admitted address exists on **both**
  base routes and is handed over, four separately flipped syntactic premises
  refuse it, and two separately flipped owner facts refuse it on the bound
  route.

### 6.2 Extended — [`proof/QCore/BoundFieldRead.lean`](../../../proof/QCore/BoundFieldRead.lean), 44 → 62 theorems

**Additively.** `admitsRead` and every theorem about it are byte-unchanged and
still model the *borrowed* half exactly — including
`a_custody_bearing_position_never_admits_a_read`, which remains true of that
half. `admitsCustodyRead` models the new half and `admitsAnyRead` their union.

* `premise_f_is_required` and `an_exposed_base_admits_only_the_borrowed_half` —
  without F the union collapses to exactly what card O04-READ-BOUND admitted;
* `the_two_read_halves_are_disjoint` (on position) and
  `no_refused_position_is_admitted_by_either_half` (quantified over `owned`,
  `stored`, `removed`, `out`);
* the base and member premises restated for the custody half;
* `a_second_read_of_a_transferred_member_never_resolves` — the double-release
  row, proved rather than tested;
* `only_the_custody_effect_can_change_a_disposition` — the asymmetry the card
  rests on, stated directly: the borrowed effect is the identity on every
  disposition and the custody effect is not, which is why the borrowed half
  needs no frame condition and this half does;
* non-vacuity in both directions.

The model is of the **decisions**, not of C semantics. Declared types, record
facts, base-scan flags, the owner's abstract state, the member's field link, C's
object model itself and premise **P-B08-SHALLOW** are premises supplied from
outside; nothing proves the reader extracts them faithfully or that a declared
`Borrowed` mode really borrows. Nothing here is a claim about arbitrary C memory
safety.

---

## 7. Deliberate pin updates, and what stayed load-bearing

Three, each keeping its load-bearing half. No gate was weakened and no
diagnostic code was added, removed or downgraded.

| # | Pin | Before | After | Load-bearing half kept |
|---|---|---|---|---|
| 1 | `aggregate_store::a_custody_bearing_position_is_never_an_admitted_bound_read` | every custody position refuses | split into `a_released_position_through_an_insulated_base_takes_custody` (the superseded half) plus **two** retained assertions: `a_custody_bearing_read_through_an_exposed_base_is_never_admitted` and `only_released_and_consumed_positions_take_custody_of_a_bound_read` | an **exposed** base still refuses, and the four other modes still refuse — now asserted separately and over the whole vocabulary |
| 2 | `o04_read_bound::a_custody_bearing_position_keeps_its_existing_refusal` | both custody sinks refuse | `a_custody_bearing_position_is_admitted_only_for_an_insulated_base`, asserting **both** directions on the same `r_*` family, with a locally declared aggregate-borrowing sink so the card's fixture contract file stays byte-unchanged | the exposed-base half asserts the identical `AnalysisStatus` and the identical `INDIRECT` diagnostic |
| 3 | `custody_summary_regression::a_changed_transitive_callee_body_invalidates_the_whole_certificate`, third row | `q_observe(q_alloc())` refused at `PROGRAM_MISMATCH` | refused at `SUMMARY_MISMATCH` — one gate **earlier**, because the nested call now refuses where it stands | the mutation is still **rejected**; and the `PROGRAM_MISMATCH` gate it used to reach is now covered *directly* by a forged per-function status, which is a sharper pin than the one it replaced |

`o04_read_bound::a_certificate_resting_on_a_bound_read_rests_on_the_declared_mode`
was re-pinned in the same spirit: it substituted `Borrowed` → `Released`, which
this card admits, so it now substitutes `Borrowed` → `Owned`, which it does not.
(`resource_out` was tried and the loader refuses it on a one-parameter
declaration; the whole mode vocabulary is pinned without Clang in
`aggregate_store`'s unit tests.)

`o04_path.qcore`, `o04_arms_callees.qcore`, `o04_b08_callees.qcore` and
`o04_read_bound_generic.qcore` are **byte-unchanged**, so cards O04, O04-ARMS,
O04-B08 and O04-READ-BOUND keep their exact premise sets. Every other pinned
expectation — O04-B08's
`every_remaining_refusal_belongs_to_the_out_of_scope_success_exit`, C02-P's
`a_store_through_the_field_between_the_read_and_the_call_is_never_admitted`,
C02's `unmodeled_memory_reads_do_not_become_neutral_effects`, the contract
corpus counts, the submodule revision — passes unchanged.

---

## 8. Evidence

### 8.1 Held-out generic family

[`crates/qcore-c-gateway/tests/o04_b09b10.rs`](../../../crates/qcore-c-gateway/tests/o04_b09b10.rs),
an `m_*` API deliberately unrelated to ChunkIO, to Fluent Bit **and** to the
`g_*` (O04), `h_*` (O04-ARMS), `k_*` (O04-B08) and `r_*` (O04-READ-BOUND)
families. Every rule is stated over declared C types and contract modes, so no
name in it can matter. 22 assertions.

| Test | Result |
|---|---|
| `a_member_address_at_a_borrowed_position_discharges_every_exit` | the real shape, synthetically: two member addresses on a bound aggregate — **Safe**, every exit `Discharged`, no refusal |
| `an_admitted_member_address_does_not_hide_a_leak` | the same body with the release removed is **Unsafe** with exactly one leak: precision in the defect direction too |
| `a_pointer_or_untyped_member_address_keeps_its_refusal` | six member shapes — pointer, second pointer, `void *`, double star, array, `volatile` — each keeps `INDIRECT` |
| `a_member_address_through_a_released_aggregate_is_never_admitted` | card C00's lesson in this direction |
| `a_member_address_through_an_aliased_base_is_never_admitted` | a second declared name refuses the base outright |
| `a_member_address_on_an_untracked_base_is_admitted` | the C02-P route, with no liveness check and no identity |
| `a_custody_bearing_position_never_takes_a_member_address` | a releasing position keeps `INDIRECT` |
| `a_custody_taking_bound_read_transfers_the_linked_child` | both `Released` and `Consumed` sinks settle every exit |
| `a_double_release_through_the_same_link_is_never_admitted` | **adversarial row 1** — the second read is refused, never a second release |
| `a_borrowed_read_after_the_transfer_is_never_admitted` | **adversarial row 2** — free then read |
| `a_custody_taking_read_through_an_exposed_base_is_never_admitted` | **adversarial row 3** — premise F, with the borrowed read through the *same* base staying admitted as the control |
| `a_link_surviving_the_aggregate_is_reported_not_discharged` | **adversarial row 4** — P-B08-SHALLOW: the still-linked child is a reported leak |
| `releasing_through_the_link_and_the_original_name_is_reported` | the transfer is not a licence to reuse the local |
| `a_custody_taking_read_of_an_unlinked_member_is_never_admitted` | a sibling member the body never stored into |
| `a_nested_call_inside_a_modeled_call_is_never_admitted` | the measured false `Safe`, repaired and named |
| `a_type_operator_is_not_a_nested_call` | `sizeof` keeps its exemption |
| `a_certificate_resting_on_a_member_address_is_rederived_from_source` | three declaration substitutions the **independent recheck** refuses (pointer member, qualified member, `union` record); the unmodified certificate still rechecks |
| `a_certificate_resting_on_a_member_address_rests_on_the_declared_mode` | turning the borrowing sink into a releasing one withdraws the projection |

Plus **9 new unit tests** in `aggregate_store`'s own module (20 → 29), covering
the argument scan, the position filter, the member filter, the whole-argument
rule, premise F's three permitted shapes and the whole custody-mode vocabulary
directly, with no Clang.

### 8.2 Real-source recheck

| Test | Result |
|---|---|
| `the_success_exit_residual_narrows_to_one_statement` | **zero** `INDIRECT` refusals remain; the exact remaining diagnostic vector is pinned; five exits stay Discharged; C6 stays `Unknown` with all three sites `Mixed`; no leak alleged |
| `the_logging_control_settles_every_exit` | the §3.2 control, with all three sites `Escaped` |
| `substituting_the_addressed_member_declarations_restores_the_refusal` | two substitutions on the real fixture — `struct msgpack_packer mp_pck` → a pointer, `pthread_mutex_t metrics_chunk_lock` → `volatile` — each restore the refusal |
| `the_constructor_boundary_is_unchanged` | 4 Discharged / 2 Unknown with two `Mixed` sites each; seven `INDIRECT` refusals retained; no leak alleged |

---

## 9. Open boundaries

Carried forward unchanged: O04's blockers **B02-B07** and **B11**; O04-ARMS's
**O04-ARMS-VARIADIC** and **O04-ARMS-SCALAR-CHAIN**; C02-P's **C02-P-ALIAS**;
O04-B08's **P-B08-SHALLOW**, **O04-B08-BORROWED-AGG**, **O04-B08-DOT** and
**O04-B08-CAST**; O04-READ-BOUND's **O04-READ-BOUND-SCALAR** and
**O04-READ-BOUND-REACH**.

Closed by this card: **B09** and **B10**, at the boundary §2 states.
**Superseded** by this card, at a narrower checked boundary:
**O04-READ-BOUND-CUSTODY**.

| # | Boundary | Status |
|---|---|---|
| **O04-B09B10-TRACE** | C6's whole residual is one statement: an uncontracted **variadic** callee whose second argument is a **nested call**. It needs a declared variadic-tail form (O04-ARMS-VARIADIC) *and* nested-call modeling, and §3.2 measures that nothing else stands between C6 and a settled exit | **open**, and it is the whole of C6 |
| **O04-B09B10-NESTED** | a nested call is now an explicit refusal, not a model. Modeling one needs an evaluation order and an intermediate-value identity this projection does not have | **open, refused by construction** |
| **O04-B09B10-VALUE-DEPENDENT** | `flb_hash_table_add`'s custody depends on the *value* of `val_size`, and the vocabulary has no value-dependent parameter mode. The declaration is faithful at this call site's literal `0` and would be unfaithful at a positive size | **recorded, §4** |
| **O04-B09B10-FRAME** | premise F is *syntactic and whole-body*. It refuses a base that any contracted call receives at a non-terminal position, even when that call provably cannot precede the read. A flow-sensitive frame condition would admit strictly more | **open**, deliberately conservative |
| **O04-B09B10-REACH** | measured member-address reach in the pinned corpus is **zero admitted statements** (§5.2); the payoff gates on how much declared contract text exists | **recorded, §5.2** |
| — | chained addresses, `&BASE.FIELD` on a record-valued base, pointer/array/bit-field/`union`/qualified members, casts, parentheses and partial arguments | **refused by construction** |

### 9.1 Cross-card overlap, recorded for the integrator, not resolved here

Two wave-12 cards run in parallel in `crates/qcore-c-gateway/`:

* **W4-EXPR-POS** — the expression-position families of
  `docs/design/W4_NEUTRAL_EXPRESSION_POSITIONS.md`. This card touches
  `classify_simple`'s contracted-call branch (the nested-call refusal, the
  member-address span, the `&`-gate exemption) and `transfer`. If W4-EXPR-POS
  changes which expression positions are neutral, §5's measurement must be
  re-run after the merge, and the interaction with `nested_call` needs one
  look: an expression position that admits a *call* would reach that gate.
* **ALIAS-LOCALS** — the C02-P-ALIAS successor. This card does not touch
  `borrowed_actual.rs` or `may_alias.rs` at all, and its `untracked_bases`
  consumption is read-only. But `member_address_actuals` consults
  `UntrackedBases::never_bound` for its second base route, so if ALIAS-LOCALS
  widens or narrows that closure, this card's untracked route moves with it and
  §5 must be re-run.

**Recorded, not resolved.**

---

## 10. Scope and non-claims

- **No raw unit is promoted.** Both functions on the path keep status
  **Unknown**; the corpus aggregate is unchanged and no verdict moved.
- **C6 is not discharged.** This card removes eight of nine refusals; one
  remains, and the control in §3.2 is explicitly non-verbatim.
- A per-exit **Discharged** is a claim about *this projection under the named
  premises of §4*, not a proof that the C is correct. Every declared contract it
  rests on is a trusted assumption, and one of them
  (`O04-B09B10-VALUE-DEPENDENT`) is faithful only at the call sites measured.
- **The containment publications are not modeled.** `mk_list_add` really does
  splice `ic` into `in->chunks` and `flb_hash_table_add` really does record `ic`
  in a table; those are O00 blockers B03/R13 and remain unmodeled. The per-exit
  claim is that every identity is accounted for, never that it is exclusively
  held.
- A provisional `Unsafe` is never presented as an upstream defect; the `m_*`
  family is synthetic and its `Safe` is `Safe` for a synthetic declared custody
  projection.
- **Unknown means Unknown.** An exit whose path carried a refusal alleges
  nothing in either direction.
- Projection coverage is not safety coverage; unknowns stay in the denominator.
- CT coverage is unchanged at 31/50; the contract corpus remains 60 modules;
  `qcore-schemas` is unchanged at 8.0.0; no plugin, protocol or evidence version
  moved.

---

## 11. Files changed

| Path | Change |
|---|---|
| `crates/qcore-c-gateway/src/typed_field_access.rs` | additive `non_pointer_fields` classification and `addressable_field` |
| `crates/qcore-c-gateway/src/aggregate_store.rs` | additive `MemberAddressActual`/`member_address_actuals`, `insulated_bases` and `AggregateStoreBases::is_insulated`, the custody flag on `BoundFieldRead`; 9 new unit tests |
| `crates/qcore-c-gateway/src/legacy_c.rs` | `ContractCall::{aggregate_consumed, aggregate_addresses}`; the member-address span and `&`-gate exemption in `classify_simple`; the `nested_call` refusal; the owner-liveness check and link-drop in `transfer` |
| `crates/qcore-c-gateway/tests/o04_b09b10.rs` | new — 22 held-out assertions |
| `crates/qcore-c-gateway/tests/fixtures/contracts/o04_b09b10_{callees,generic}.qcore` | new — the five declared callee premises and the held-out `m_*` API, both outside `contract/qcore` |
| `crates/qcore-c-gateway/tests/o04_read_bound.rs` | two pins re-pinned deliberately (§7); the fixture contract file is byte-unchanged |
| `crates/qcore-c-gateway/tests/custody_summary_regression.rs` | one row re-pinned deliberately, and the gate it used to reach now covered directly (§7) |
| `crates/qcore-c-gateway/tests/unit/legacy_c.rs` | two new struct fields in one constructed instruction |
| `proof/QCore/MemberAddress.lean`, `proof/QCore.lean`, `proof/AuditMemberAddress.lean` | new module (35 theorems) and its axiom audit |
| `proof/QCore/BoundFieldRead.lean` | additive custody section (44 → 62 theorems); everything above it is byte-unchanged |
| `docs/reports/remaining-o04-b09b10/` | this record, the census/attribution/archive tooling, the transcripts and the re-measured artifacts |

No file owned by the integrator was touched: `AGENTS.md`, `CLAUDE.md`,
`README.md`, `DEVELOPMENT.md`, `docs/reevaluation/ct_manifest.json`,
`docs/reviews/r0_3/acceptance.json` and
`docs/reviews/r0_3/acceptance_matrix.json` are unchanged, as are
`contract/qcore/`, `crates/qcore-c-gateway/src/borrowed_actual.rs`,
`crates/qcore-c-gateway/src/may_alias.rs`, every archived predecessor report and
every existing test fixture except the two files §7 names.

---

## 12. Gate results

Run in this worktree at branch `claude/o04-b09b10`, base `67961f1`, with the
Libevent submodule initialised.

| Gate | Command | Observed | Exit |
|---|---|---|---|
| Workspace tests | `cargo test --workspace --no-fail-fast` | **2,150 passed / 0 failed / 5 ignored** (base 2,119 / 4) | 0 |
| Format | `cargo fmt --all -- --check` | clean | 0 |
| Clippy | `cargo clippy --all-targets -- -D warnings` | clean | 0 |
| Probe | `cargo run -p qcore-compiler --example reevaluation_probe` | ran | 0 |
| Follow-up | `cargo run -p qcore-compiler --example reevaluation_followup` | ran | 0 |
| Lean audits | `python3 -B scripts/check_lean_audits.py` | **54 audits passed** (53 → 54) | 0 |
| Provider tests | `cargo test --locked --offline --manifest-path providers/Cargo.toml --workspace` | **76 passed / 0 failed** | 0 |
| Provider packages | `python3 -B providers/package.py build` | five packages built and verified | 0 |
| Python | `python3 -B -m unittest discover -s scripts/tests` | **125 ran, 1 failure** — see §12.1 | 1 |
| 185-unit corpus | pinned route, before and after | §5 | 0 / 0 |
| Configured audits | `qcore-audit` over cmetrics-counter and chunkio-core, before and after | §5.3 | 0 / 0 |

The card contributes **31 new assertions** (22 integration, 9 unit) and **53
new Lean theorems**, every one of them audited by name. The workspace tally moves 2,119 → 2,150, which is exactly
this card's contribution and nothing else.

Two known load-flaky tests failed once each in a full-workspace run and passed
in isolation: `qcore-verifier-host`'s
`a_confined_fixture_cannot_write_outside_the_admitted_paths` (P03) and
`cycle20_verifier_plugins`'s `vp04_vp07_vp08_…`. Neither assertion was touched.

### 12.1 One integrator-owned pin must move — `validation.lean_audits`

`scripts/tests/test_build_matrix_sections.py::test_the_derived_facts_are_recomputed_rather_than_carried`
fails:

```
AssertionError: Lists differ: ['8.0.0', 53] != ['8.0.0', 54]
```

`docs/reviews/r0_3/acceptance_matrix.json`'s `validation` block is the
**integrator-supplied merged gate**. This card adds one `Audit*.lean` file, so
the derived count is now 54 and the committed value is stale. **That is card
GEN-REASON's tripwire working exactly as designed**, not a defect and not a
weakened gate.

That file is explicitly integrator-owned and this card is forbidden from editing
it, so the pin is **deliberately left for the integrator** to move when
regenerating the matrix with `--gate`:

* `validation.lean_audits`: 53 → **54**
* `validation.lean_audit_files`: add `AuditMemberAddress.lean`
* `validation.workspace_tests`: 2119 → this card's contribution plus the rest of
  the wave

No other Python test fails; the other 124 pass.
