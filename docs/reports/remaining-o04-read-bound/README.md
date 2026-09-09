# O04-READ-BOUND — reading a member back through a base this body bound — 2026-09-09

The named continuation of card O04-B08
([`docs/reports/remaining-o04-b08/README.md`](../remaining-o04-b08/README.md)
§7), whose last open boundary was **`O04-B08-READ-BOUND-BASE`**:

> a member *read* through a base the projection binds
> (`flb_routes_mask_set_by_tag(ic->routes_mask, …)`) keeps C02-P premise 4's
> refusal; the store side is admitted, the read side is not — **open**, and the
> last blocker between C6 and B09/B10.

This card supplies the missing typed premise. **The named blocker is gone, and
the success exit C6 is still `Unknown`.** That is the honest headline in both
halves: the refusal this card removes was one of ten on C6's path, and the other
nine are B09/B10 territory or deliberate. **No exit that was not settled before
is settled now.**

| Arm | O04-ARMS | O04-B08 | This card |
|---|---|---|---|
| **C1** — logging arm, `!chunk` | Discharged | Discharged | **Discharged** |
| **C2** — `up_force == -1` | Discharged | Discharged | **Discharged** |
| **C3** — header write `== -1` | Discharged | Discharged | **Discharged** |
| **C4** — `ic` calloc NULL | Discharged | Discharged | **Discharged** |
| **C5** — routes-mask arm | `Unknown` | Discharged | **Discharged** |
| **C6** — success + publication | `Unknown` | `Unknown` | **`Unknown`** — one fewer named blocker |

**Nothing is promoted.** Both real functions keep function-level status
`Unknown`; the corpus headline is unchanged; **all five 185-unit artifacts are
byte-identical before and after**; the contract corpus stays at 60 modules; CT
coverage stays 30/50; `qcore-schemas` is unchanged at 8.0.0.

---

## 1. What the card admits

One analyzer capability, added to the existing
[`crates/qcore-c-gateway/src/aggregate_store.rs`](../../../crates/qcore-c-gateway/src/aggregate_store.rs)
(`bound_read_actuals`) plus one flow-sensitive revalidation in
[`legacy_c.rs`](../../../crates/qcore-c-gateway/src/legacy_c.rs)
(`bound_read_resolves`).

### 1.1 The shape

Card C02-P admits a member read only through a base its `untracked_bases`
closure did **not** refuse — a base the body may never bind to a resource
identity. Card O04-B08 admits a *store* into a member of a base the body **did**
bind. The read through such a base fell between the two and was refused. It is
now admitted for exactly one shape.

| # | Premise | Refused otherwise |
|---|---|---|
| 1-5 | **the identical base premises O04-B08's `store_bases` checks** — readable declarations; a non-formal local declared exactly `struct TAG *` over a complete non-`union` record; bound by exactly one top-level assignment whose right-hand side *calls* something; never address-taken, dereferenced, subscripted, `.`-selected, incremented or compound-assigned; never the whole right-hand side of an assignment and never a whole argument at an `Out`/`Stored`/`Removed` position of a contracted call | every shape O04-B08 §1.1 lists, unchanged |
| 6 | the actual is exactly `BASE - > FIELD` and is the **whole** argument | a cast, parentheses, arithmetic, a chained access, a partial argument, `&BASE->FIELD` |
| 7 | `FIELD` is a member declared with **exactly one** `*` over a specifier list the reader resolved (`single_pointer_field`) | `void *`, multi-star, array, function-pointer, bit-field, aggregate sub-object, `volatile`/`_Atomic`, unresolved typedef — **and a scalar member**, which can never hold an identity (`O04-READ-BOUND-SCALAR`) |
| 8 | the actual occupies a declared **`Borrowed`** parameter position of a callee with a contract in force | every custody-bearing position, and every uncontracted callee (`O04-READ-BOUND-CUSTODY`, §1.3) |

The call whose arguments are read is **`legacy_c`'s own `find_call` result**,
not one this module scans for independently. The two scans differ on reserved
words, and an independently derived index could have excluded a span belonging
to a *different* call from the lexical gate. That was found and closed during
implementation, before any measurement; it is pinned by
`only_the_supplied_call_admits_bound_reads`.

### 1.2 The flow-sensitive premise

Declarations are not enough, exactly as in the store direction. At the read
itself, `legacy_c::transfer` requires four conjuncts, **before any effect of the
call applies**:

1. the **owner** resolves through the unchanged `resolved_aggregate_owner` — one
   live identity this body acquired, bound here, unique, not `unknown`, not
   possibly null, not caller-held, not carrying an unresolved effect;
2. the **place** `BASE->FIELD` is bound to a definite, non-empty target set — a
   member this body never stored into carries no identity to denote;
3. that target set is **exactly** the field link an admitted store recorded
   under the resolved owner. A binding that has drifted from its link is refused
   rather than believed;
4. every linked identity is itself **live** and carries no unresolved effect.

Conjunct 4 was added during implementation after conjunct 1-3 alone admitted a
read of a member left pointing at an identity the body had already released. It
is **card C00's lesson in the read direction**, and it is why
`a_read_of_a_released_linked_child_is_never_admitted` passes.

A read failing any conjunct reports `unknown_alias` on the aggregate's own name
and poisons the path — the same failure mode, the same diagnostic and the same
code as the store direction. **No new diagnostic code was added, removed or
downgraded.**

### 1.3 Why `Borrowed` only — `O04-READ-BOUND-CUSTODY`

A borrowed actual discharges nothing (`apply_contract_call` gives
`ResourceParameterMode::Borrowed` the identity effect), so admitting one can
only withdraw a *coverage* refusal. It can never manufacture a release.

Every custody-bearing position is refused, and the reason is measured rather
than stylistic: a contracted callee that merely **borrows** the aggregate could
rewrite the member invisibly, and this projection maintains no frame condition
for that. Admitting `free(ch->name)` — a real statement in `cio_chunk_open`,
whose `ch->name = strdup(name)` store O04-B08 already admits — would therefore
rest on a premise nothing here establishes. It stays refused, and that is why
the constructor's boundary is byte-for-byte what O04-B08 measured (§3.2).

This is recorded as an open boundary, **not** as a claim that the restriction is
unnecessary.

---

## 2. Every entry premise, named

O04's nine premises, O04-ARMS's two and O04-B08's two (**P-B08-SHALLOW** and the
`flb_time_now` declaration) carry over unchanged and are still **assumed, not
derived** — including that the constructor's own body does not discharge its
declared contract. **This card adds no new declared premise and no new contract
text:** `flb_routes_mask_set_by_tag` was already declared in the byte-unchanged
[`o04_path.qcore`](../../../crates/qcore-c-gateway/tests/fixtures/contracts/o04_path.qcore),
with its first parameter `Borrowed`, by card O04.

Two premises are inherited from C02 and are **not** proved: declaration
completeness, and C effective types. C02-P's recorded may-alias boundary
(**C02-P-ALIAS**) is inherited and **not widened**: the field link is keyed by
the aggregate's *site*, and base premise 5 refuses a base that could acquire a
second declared name at all.

---

## 3. The measured per-arm disposition

Raw ledgers: [`logs/arm_ledger_before.txt`](logs/arm_ledger_before.txt) and
[`logs/arm_ledger_after.txt`](logs/arm_ledger_after.txt), regenerable with the
`print_the_measured_arm_ledger` reporting aid in
`crates/qcore-c-gateway/tests/o04_b08.rs`. **The two files differ by exactly one
line** — the removed refusal — which is the tightest isolation this subject can
give.

### 3.1 `flb_input_chunk_create` — six exits, unchanged

| O00 exit | Upstream line | Arm | O04-B08 | This card | Residual |
|---|---|---|---|---|---|
| C1 | 2530 | constructor returned NULL | Discharged | **Discharged** | none acquired |
| C2 | 2541 | `up_force == -1` | Discharged | **Discharged** | chunk Released |
| C3 | 2550 | header write `== -1` | Discharged | **Discharged** | chunk Released |
| C4 | 2558 | `ic` calloc NULL | Discharged | **Discharged** | chunk Released |
| C5 | 2587 | `routes_mask` calloc NULL | Discharged | **Discharged** | chunk + `ic` Released |
| C6 | 2620 | success + publication | Unknown | **Unknown** | all three `Mixed` |

Site counts `[0,1,1,1,2]` on the five settled exits, every site `Released`, all
four cleanup arms naming the same acquisition site: unchanged. No exit is a
`Residual`, no `Leak` is reported, and C6 carries its three identities as
`Mixed`, never as a claim.

### 3.2 C6's residual, before and after — the honest table

| # | Statement (preprocessed line) | O04-B08 | This card |
|---|---|---|---|
| 1 | `has_routes = flb_routes_mask_set_by_tag(ic->routes_mask, tag, tag_len, in);` (253) | `INDIRECT` — **`O04-B08-READ-BOUND-BASE`** | **admitted** |
| 2 | `flb_trace(…)` (255) | uncontracted callee, deliberate | unchanged |
| 3 | `msgpack_packer_init(&ic->mp_pck, ic, flb_input_chunk_write);` (258) | `INDIRECT` | unchanged |
| 4 | `mk_list_add(&ic->_head, &in->chunks);` (259) | `INDIRECT` | unchanged |
| 5-8 | four `flb_hash_table_add(in->ht_*_chunks, tag, tag_len, ic, 0);` (264, 268, 272, 275) | `INDIRECT` | unchanged |
| 9-10 | `pthread_mutex_lock/unlock(&in->metrics_chunk_lock);` (267, 269) | `INDIRECT` | unchanged |
| 11 | `return ic;` (277) | `ic` possibly-null under a non-null contract | unchanged |

**Nine `INDIRECT` refusals become eight.** Rows 3, 4, 9 and 10 take the address
of a member (`&ic->mp_pck`, `&ic->_head`, `&in->metrics_chunk_lock`) — refused by
base premise 4 and by the whole-argument premise, and out of scope for every
card in this family. Rows 5-8 call an **uncontracted** `flb_hash_table_add`.
Row 11 is C6's own nullability. **C6 is not discharged and this card does not
claim it is.** Both halves are pinned by
`the_success_exit_keeps_its_named_residual`, which asserts the exact count of
eight so a silent change in either direction fails.

### 3.3 `cio_chunk_open` — byte-for-byte unchanged

Four guard/allocation-failure exits stay Discharged; the rollback and success
exits stay Unknown, each carrying two `Mixed` sites. Its `ch->name = strdup(name)`
store stays admitted and `free(ch->name)` stays refused, because a releasing
position is refused by `O04-READ-BOUND-CUSTODY` (§1.3). Pinned by
`the_constructor_boundary_is_unchanged`.

---

## 4. Evidence

### 4.1 Held-out generic family

[`crates/qcore-c-gateway/tests/o04_read_bound.rs`](../../../crates/qcore-c-gateway/tests/o04_read_bound.rs),
an `r_*` API deliberately unrelated to ChunkIO, to Fluent Bit **and** to the
`g_*` (O04), `h_*` (O04-ARMS) and `k_*` (O04-B08) families. Every rule is stated
over declared C types and contract modes, so no name in it can matter.
18 assertions.

| Test | Result |
|---|---|
| `a_bound_member_read_at_a_borrowed_position_discharges_every_exit` | the real shape, synthetically: acquire, store, acquire into a second member, read **both** back at borrowed positions — **Safe**, every exit `Discharged`, no refusal, no leak |
| `an_admitted_read_does_not_hide_a_leak` | the same body with one cleanup arm removed is **Unsafe** with a leak: a precision gain in the defect direction too |
| `a_read_after_an_overwrite_denotes_the_current_link_and_reports_the_first` | **adversarial row 1a** — the read denotes the *current* link, the overwritten resource still leaks (exactly one), and the read itself is admitted |
| `a_read_after_the_member_is_cleared_is_never_admitted` | **row 1b** — storing null unlinks the member, so a later read is refused rather than resolved to the stale identity; `unknown_alias` on the aggregate |
| `a_read_of_a_released_linked_child_is_never_admitted` | **adversarial row 2** — card C00's lesson: never `Safe`, `unknown_alias` reported |
| `a_read_through_a_released_aggregate_is_never_admitted` | the same lesson one level up |
| `a_read_through_an_aliased_base_is_never_admitted` | **adversarial row 3** — a second declared name refuses the base outright; the existing `INDIRECT` diagnostic is retained |
| `a_read_of_an_unlinked_member_is_never_admitted` | **adversarial row 4** — a sibling member this body never stored into is refused although the aggregate resolves perfectly |
| `a_custody_bearing_position_keeps_its_existing_refusal` | `O04-READ-BOUND-CUSTODY`: a releasing and a consuming position each keep `Unknown` and `INDIRECT` |
| `every_refused_member_shape_keeps_its_unknown_and_its_diagnostic` | seven shapes — `void *`, double star, array, `volatile`, chained, **scalar**, partial argument — same discipline |
| `a_read_through_an_aggregate_read_out_of_memory_is_never_admitted` | base premise 3 |
| `the_two_read_routes_are_disjoint` | measured, not argued: a caller-owned aggregate stays C02-P's `Safe` and records **no** bound read; `R_CREATE` records exactly `["b->item", "b->raw"]` |
| `a_certificate_resting_on_a_bound_read_is_rederived_from_source` | five declaration substitutions the **independent recheck** refuses; the unmodified certificate still rechecks |
| `a_certificate_resting_on_a_bound_read_rests_on_the_declared_mode` | turning the borrowing sink into a releasing one withdraws the projection |

Plus **nine** new unit tests in `aggregate_store`'s own module (11 → 20)
covering the argument scan, the position filter, the member filter, the
whole-argument rule and the supplied-call-index rule directly, with no Clang.

### 4.2 Real-source recheck

| Test | Result |
|---|---|
| `the_named_read_blocker_is_gone_and_the_success_exit_is_still_unknown` | no `Unsupported` diagnostic remains on the routes-mask read's own preprocessed line; **exactly five** exits stay Discharged, the function stays `Unknown`, C6 stays `Unknown` with every identity `Mixed`, no `Residual`, no leak |
| `the_success_exit_keeps_its_named_residual` | every remaining refusal is `INDIRECT`, `flb_trace` or `ic` possibly-null, and the `INDIRECT` count is exactly **8** |
| `substituting_the_read_member_declaration_restores_the_refusal` | two substitutions on the real fixture — double pointer, `volatile` — each restore the refusal |
| `the_constructor_boundary_is_unchanged` | 4 Discharged / 2 Unknown with two `Mixed` sites each; `free(ch->name)` keeps its refusal; no leak alleged |

### 4.3 Pinned expectations

**No pinned expectation was updated, weakened or deleted by this card.** Card
O04-B08's `every_remaining_refusal_belongs_to_the_out_of_scope_success_exit`,
card O04-ARMS's rows, card C02-P's
`a_store_through_the_field_between_the_read_and_the_call_is_never_admitted` and
card C02's `unmodeled_memory_reads_do_not_become_neutral_effects` all pass
unchanged. `o04_path.qcore`, `o04_arms_callees.qcore` and
`o04_b08_callees.qcore` are byte-unchanged. The one new fixture contract lives
outside `contract/qcore`, so the pinned corpus count of 60 is untouched.

---

## 5. Corpus impact — zero movement, measured exhaustively

### 5.1 The 185-unit artifacts

Re-measured through the canonical pinned route on the shared prepared pinned
clone (Fluent Bit `66910c10a4d7eafa810b84229cf2dcf7b0f26f97`, explicit
`/usr/bin/clang` 21.1.8), following
[`docs/reports/corpus-repin/provenance.json`](../corpus-repin/provenance.json):

```sh
python3 -B providers/package.py build --output build/cycle19/provider-packages-readbound
sh build/cycle19/run_audit.sh build/cycle19/reports-{before,after}
#   = QCORE_CLANG=/usr/bin/clang \
#     QCORE_CHUNKIO_PACKAGE=build/cycle19/provider-packages-readbound/qcore.chunkio \
#     cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
#       --pinned-evidence <clone> <monkey>/compile_commands.json \
#       <out>/monkey-source.json <out>/resource-diagnostics.json
```

Run **twice from this same worktree**, with only `crates/qcore-c-gateway/src/`
reverted to the branch base `0f6d66c` between runs, so the comparison carries no
path provenance at all. Both exit 0 with zero gateway failures.
Transcripts: [`logs/corpus-before.log.gz`](logs/corpus-before.log.gz),
[`logs/corpus-after.log.gz`](logs/corpus-after.log.gz). `build/cycle19/` is
gitignored, so `run_audit.sh` is not committed; the command it runs is complete
above. The prepared clone is the shared read-only tree under the main checkout's
`build/cycle19/clang/66910c10…/`; nothing in it was written to.

#### Harness validation — the BEFORE run against the archived baseline

| Artifact | before vs `docs/reports/wave9-corpus/artifacts/` |
|---|---|
| `diagnostics.json` | **byte-identical** (SHA-256 `cc6d8dc087f20922…`) |
| `chunk-contracts.json` | **byte-identical** (SHA-256 `ba171e4f93712f0a…`) |
| `source.json` | **byte-identical** (SHA-256 `53d55b7e1fb174de…`) |
| `contracts.json` | differs — **path-literal residue only** (§5.2) |
| `source-effects.json` | differs — **package-identity residue only**, 3 of 96 rows (§5.2) |

The exhaustive `diff_artifacts.py` document
([`artifacts/diff-baseline-to-before.json.gz`](artifacts/diff-baseline-to-before.json.gz))
reports **0 added / 0 removed / 0 changed** in `diagnostics.units`,
`chunk-contracts.results`, `chunk-contracts.backend_teardown`,
`source.callbacks` and `source.wrappers`, and **3 changed** in
`source-effects.results` — the itemised residue. Status histograms agree
exactly: **2 Safe / 42 Unsafe / 137 Unknown / 4 data-only**, `checked` 3 /
`unknown` 93 on both sidecars, and reason codes `48 null` /
`1 incomplete-obligation-coverage` / `136 unsupported-syntax`.

That is what makes the before/after comparison below an isolation of this card.

#### Before → after

**All five artifacts are byte-identical**, SHA-256 for SHA-256:

| Artifact | before | after |
|---|---|---|
| `diagnostics.json` | `cc6d8dc087f20922…` | **same** |
| `chunk-contracts.json` | `ba171e4f93712f0a…` | **same** |
| `source.json` | `53d55b7e1fb174de…` | **same** |
| `source-effects.json` | `9c7bad356bc1b866…` | **same** |
| `contracts.json` | `6d288d92a5404615…` | **same** |

The exhaustive diff
([`artifacts/diff-before-to-after.json.gz`](artifacts/diff-before-to-after.json.gz))
reports **0 added / 0 removed / 0 changed** in every collection, including
`source-effects.results`.

The independent census ([`census.py`](census.py), transcript at
[`logs/census.txt`](logs/census.txt)) compares the two runs directly rather than
trusting the digests:

```
units: 185 -> 185;  added: [];  removed: []
unit status before/after: {'Safe': 2, 'Unknown': 137, 'Unsafe': 42, 'data_only': 4}
unit status moves: 0
NEWLY SURFACED PROVISIONAL UNSAFE UNITS: 0
unit reason_code moves: 0
projected functions: 4834 -> 4834;  function status moves: 0
leak: 271 -> 271 (+0);  unknown_alias: 4386 -> 4386 (+0);  unsupported: 49936 -> 49936 (+0)
units with diagnostic-count changes: 0
description inventory deltas: (none)
```

**No newly surfaced provisional `Unsafe` unit**, at any level: the unit status
histogram is identical, no unit row changed `status` or `reason_code`, all 4,834
projected functions were compared pairwise with zero moves, and the corpus
`leak` count is unchanged at 271. This was the direction to check first, and the
measurement is exhaustive rather than sampled.

### 5.2 The two exempt residue classes, itemised

Both are present *before* any change of mine and were recorded by CORPUS-REPIN,
O04-ARMS, O04-B08, E01-F1 and the wave-9 consolidation.

1. **Path-literal provenance — `contracts.json`.** The report embeds the
   absolute path of each loaded `.qcore` module
   (`…/agent-a634d3860d9395e84/compiler/../contract/…`). Only the *keys* of the
   `sources` map differ; the contract text, the manifest, the audit selection
   and the source-profile digest `5507e9f605522ac8…` are identical. This class
   reappears in any measurement from a different directory, exactly as every
   prior re-pin recorded.
2. **Package identity — `source-effects.json`, 3 of 96 rows.** The provider
   packages were rebuilt into `build/cycle19/provider-packages-readbound` for
   this run, so `cio_chunk_is_locked`, `cio_chunk_unlock` and
   `flb_input_chunk_release_lock` move in `evidence.reproducibility.package_digest`,
   `witness.task_digest`, `task.unit.build_digest`, `evidence.unit.build_digest`,
   `import_identity.configuration_digest`, the per-obligation identifiers, the
   `evidence.coverage` entries and the `resource_contracts:<digest>` bound-key
   name. **44 differing leaves in 3 rows, and no verdict-bearing field among
   them**: `status`, `function`, `unit`, `reason` and `reason_code` are equal on
   all 96 rows, and both sides report `checked_function_contract` 3 /
   `unknown` 93.

### 5.3 Why the reach is zero, measured rather than argued

Zero corpus movement is a result, not an absence of measurement, and it is
consistent with card C03-NFR's precedent. Two measurements bound it from above
([`reach.py`](reach.py), transcript at [`logs/reach.txt`](logs/reach.txt)):

* **Contract side.** Of the **19** contracted functions the pinned corpus
  manifest puts in force, only **4** declare a `Borrowed` parameter whose
  declared type is a *pointer* at all: `cio_chunk_open` (indices 0, 2, 4, 8),
  `flb_sds_create_len`, `rd_strdup` and `rd_strndup` (index 0). No other
  position in the corpus can ever receive an admitted bound read.
* **Source side.** Across the **151** `.c` files the 185-unit audit covers,
  **31** call sites pass a single `->` selection as one of those whole
  arguments. That count is a strict over-count: it ignores the per-function
  scope of the base premises, the exactly-once binding rule, the member's
  declared type, whether an admitted store linked a *tracked* identity into that
  member on every incoming path, and the liveness gate.

The 31 split **27** `rd_strdup(x->field)`, **2** `rd_strndup(x->field)` and
**2** `cio_chunk_open(x->field, …)`. One worked example of why the largest
family cannot fire: at
`lib/librdkafka-2.15.0/src/rdkafka_broker.c:5290`,
`rkb->rkb_logname = rd_strdup(rkb->rkb_name);` reads `rkb_name`, declared
`char rkb_name[RD_KAFKA_NODENAME_SIZE]` — an **array** member, refused by
premise 7 — and the surrounding statements pass `sizeof(rkb->rkb_name)` to
uncontracted callees, which poisons the path before the read is reached anyway.

**The admission's payoff in the pinned corpus is therefore the O04 subject
alone.** It gates, as O04-B08's did, on how much declared contract text exists:
a wider corpus contract set would widen it.

### 5.4 The three configured audits — byte-identical

Run in this same worktree, before and after, with only
`crates/qcore-c-gateway/src/` reverted between runs. Every transcript in
[`logs/`](logs/).

| Audit | Result | before vs after |
|---|---|---|
| `contract/profiles/cmetrics-counter` | 1 unit: 0 Safe / 1 provisional Unsafe / 0 Unknown; 1 checked function contract / 6 Unknown | **byte-identical transcript** |
| `contract/profiles/chunkio-core` | 13 units: 0 Safe / 2 provisional Unsafe / 11 Unknown | **byte-identical transcript** |
| `contract/profiles/libevent-core` | 21 units: 0 Safe / 4 provisional Unsafe / 17 Unknown | **byte-identical transcript** |

---

## 6. Lean parity

[`proof/QCore/BoundFieldRead.lean`](../../../proof/QCore/BoundFieldRead.lean),
**44 theorems**, registered as the new
[`proof/AuditBoundFieldRead.lean`](../../../proof/AuditBoundFieldRead.lean) and
checked by `check_lean_audits.py` (48 audits → **49**). Every theorem is audited.
Only `propext`; twelve depend on **no axiom at all**. No new axiom, no
placeholder, no `sorry`.

It reuses `QCore.AggregateStore`'s `MemberKind`, `Disposition` and `Owner`, so
the two decisions are stated over the same vocabulary rather than two copies.

* **Every premise of the syntactic whitelist refuses on its own** — thirteen
  theorems, one per conjunct, including `a_scalar_member_never_admits_a_read`
  (`O04-READ-BOUND-SCALAR`), `an_unacquired_base_never_admits_a_read` and
  `an_aliased_base_never_admits_a_read`.
* **`O04-READ-BOUND-CUSTODY`** — `a_custody_bearing_position_never_admits_a_read`
  is quantified over the *whole* position vocabulary, so a later widening cannot
  slip a mode past it, with a per-mode corollary and the contrapositive
  `an_admitted_read_is_borrowed`.
* **The flow-sensitive premise** — ten theorems, including
  `a_read_through_a_released_aggregate_never_resolves`,
  `a_released_linked_child_never_resolves_a_read` (card C00's lesson in the read
  direction), `an_unlinked_member_never_resolves_a_read` and
  `a_drifted_binding_never_resolves_a_read`.
* **Fail-closed transfer, both directions** — `a_refused_read_never_denotes`,
  `an_unresolved_read_never_denotes`, and `only_a_doubly_admitted_read_denotes`,
  which shows the two gates are *jointly* necessary and that a denoting read's
  position is `borrowed` and its child live.
* **What an admitted read does, and does not, do** —
  `an_admitted_read_changes_no_disposition`, `an_admitted_read_never_discharges`
  and `an_admitted_read_never_releases`: the borrowed effect is the identity on
  every disposition, which is what makes the admission structurally incapable of
  manufacturing a discharge.
* **Disjointness** — `the_two_read_routes_are_disjoint` proves an admitted bound
  read's base is one card C02-P's route refuses, with
  `the_borrowed_actual_route_is_not_vacuous` showing the two are complements
  rather than both empty.
* **Non-vacuity in both directions** — an admitted read exists and denotes, four
  separately flipped syntactic premises refuse it, and three separately flipped
  flow-sensitive facts refuse it.

The model is of the **decisions**, not of C semantics. Declared types, record
facts, base-scan flags, the owner's abstract state and the member's field link
are premises the reader and the transfer function supply; nothing proves either
extracts them faithfully, that a declared `Borrowed` mode really borrows, or
that P-B08-SHALLOW is true of the real contracts. Nothing here is a claim about
arbitrary C memory safety.

---

## 7. Open boundaries

Carried forward unchanged: O04's blockers **B02-B07**, **B09-B11**; O04-ARMS's
**O04-ARMS-VARIADIC** and **O04-ARMS-SCALAR-CHAIN**; C02-P's **C02-P-ALIAS**;
O04-B08's **P-B08-SHALLOW**, **O04-B08-BORROWED-AGG**, **O04-B08-DOT** and
**O04-B08-CAST**.

Closed by this card: **O04-B08-READ-BOUND-BASE**, at the boundary §1 states.

| # | Boundary | Status |
|---|---|---|
| **O04-READ-BOUND-CUSTODY** | a bound read may occupy only a `Borrowed` position. Every position that discharges — `Owned`, `Consumed`, `Released`, `Stored`, `Removed`, `Out` — keeps its refusal, because a contracted callee that merely borrows the aggregate could rewrite the member invisibly and this projection has no frame condition for that. `free(ch->name)` in `cio_chunk_open` is the concrete statement this refuses | **open**, refused by construction, proved to be load-bearing |
| **O04-READ-BOUND-SCALAR** | a scalar member read through a *bound* base keeps its refusal: it can never hold an identity, and card C02's scalar route independently refuses a base this body binds | **refused by construction** |
| **O04-READ-BOUND-REACH** | measured corpus reach is **0**; the payoff gates on how much declared contract text exists (only 4 of 19 contracted functions have a `Borrowed` pointer parameter at all) | **recorded, §5.3** |
| **C6** | the constructor's success exit is **not** discharged. Eight `INDIRECT` refusals remain on its path (member address-of for `msgpack_packer_init`/`mk_list_add`/`pthread_mutex_*`, four uncontracted `flb_hash_table_add` calls), plus the deliberate `flb_trace` and `ic`'s own nullability | **open** — B09/B10 territory |
| — | chained reads, `void *`, multi-star, array, bit-field, `union`, qualified and aggregate members; casts, parentheses and partial arguments; record-valued (`.`) bases | **refused by construction** |

### 7.1 Cross-card overlap, recorded for the integrator, not resolved here

Two wave-10 cards were named as running in parallel and also touching
`crates/qcore-c-gateway/`:

* **C02-P-ALIAS** — subject `borrowed_actual.rs`. **This card does not touch
  `borrowed_actual.rs` at all.** It was deliberately left alone: an earlier
  draft would have changed its call scan for consistency, and that edit was
  reverted in favour of passing `legacy_c`'s own call index into the new
  function (§1.1). The two routes are *proved* disjoint
  (`the_two_read_routes_are_disjoint`, plus the measured Rust test), but if
  C02-P-ALIAS widens `untracked_bases` so that a call-bound base becomes
  admissible there, that disjointness theorem's premise moves and both this
  card's route and its corpus measurement (§5) must be re-run after the merge.
* **E01-RAW-COND** — subject the raw scan / conditional-arm brace imbalance.
  This card touches the *statement classifier* and the *abstract transfer*, not
  the scanner that enumerates definitions. If that repair changes which text
  `legacy_c` sees, §5's measurement must be re-run after the merge.

**Recorded, not resolved.**

---

## 8. Scope and non-claims

- **No raw unit is promoted.** Both functions on the path keep status
  **Unknown**; the corpus aggregate is unchanged and every artifact is
  byte-identical.
- **C6 is not discharged.** This card removes one named blocker from a residual
  of ten; nine remain.
- A per-exit **Discharged** is a claim about *this projection under the named
  premises of §2*, not a proof that the C is correct. Every declared contract it
  rests on is a trusted assumption.
- A provisional `Unsafe` is never presented as an upstream defect; the `r_*`
  family is synthetic and its `Safe` is `Safe` for a synthetic declared custody
  projection.
- **Unknown means Unknown.** An exit whose path carried a refusal alleges
  nothing in either direction.
- Projection coverage is not safety coverage; unknowns stay in the denominator.
- CT coverage is unchanged at 30/50; the contract corpus remains 60 modules;
  `qcore-schemas` is unchanged at 8.0.0; no plugin, protocol or evidence version
  moved.

---

## 9. Files changed

| Path | Change |
|---|---|
| `crates/qcore-c-gateway/src/aggregate_store.rs` | additive `BoundFieldRead` and `bound_read_actuals`; module header records the read half; 9 new unit tests |
| `crates/qcore-c-gateway/src/legacy_c.rs` | `ContractCall::aggregate_reads`; the admission in `classify_simple` (merged into the existing span exclusion, with `find_call`'s own index); `bound_read_resolves` and its pre-effect check in `transfer` |
| `crates/qcore-c-gateway/tests/o04_read_bound.rs` | new — 18 held-out assertions |
| `crates/qcore-c-gateway/tests/fixtures/contracts/o04_read_bound_generic.qcore` | new — the held-out `r_*` API, deliberately outside `contract/qcore` |
| `proof/QCore/BoundFieldRead.lean`, `proof/QCore.lean`, `proof/AuditBoundFieldRead.lean` | new module (44 theorems) and its axiom audit |
| `docs/reports/remaining-o04-read-bound/` | this record, the census/reach/archive tooling, the transcripts and the re-measured artifacts |

No file owned by the integrator was touched: `AGENTS.md`, `CLAUDE.md`,
`README.md`, `DEVELOPMENT.md`, `docs/reevaluation/ct_manifest.json`,
`docs/reviews/r0_3/acceptance.json` and
`docs/reviews/r0_3/acceptance_matrix.json` are unchanged, as is
`contract/qcore/`, `crates/qcore-c-gateway/src/borrowed_actual.rs`, every
archived predecessor report and every existing test fixture.

---

## 10. Gate results

Run in this worktree at branch `worktree-agent-a634d3860d9395e84`, base
`0f6d66c`, with the Libevent submodule initialised.

| Gate | Command | Observed | Exit |
|---|---|---|---|
| Workspace tests | `cargo test --workspace --no-fail-fast` | **1,915 passed / 0 failed / 4 ignored** (base 1,888 / 4) | 0 |
| Format | `cargo fmt --all -- --check` | clean | 0 |
| Clippy | `cargo clippy --all-targets -- -D warnings` | clean | 0 |
| Probe | `cargo run -p qcore-compiler --example reevaluation_probe` | ran | 0 |
| Follow-up | `cargo run -p qcore-compiler --example reevaluation_followup` | ran | 0 |
| Lean audits | `python3 -B scripts/check_lean_audits.py` | **49 audits passed** (48 → 49) | 0 |
| Provider tests | `cargo test --locked --offline --manifest-path providers/Cargo.toml --workspace` | **75 passed / 0 failed** | 0 |
| Provider packages | `python3 -B providers/package.py build` | five packages built and verified | 0 |
| Python | `python3 -B -m unittest discover -s scripts/tests` | **125 ran, 1 failure** — see §10.1 | 1 |
| 185-unit corpus | pinned route, before and after | §5.1 | 0 / 0 |
| Configured audits | `qcore-audit` over cmetrics-counter, chunkio-core, libevent-core, before and after | §5.4 | 0 / 0 |

The card contributes **27 new assertions** (18 integration, 9 unit) and **44 new
Lean theorems**. The workspace tally moves 1,888 → 1,915, which is exactly this
card's contribution and nothing else.

### 10.1 One integrator-owned pin must move — `validation.lean_audits`

`scripts/tests/test_build_matrix_sections.py::test_the_derived_facts_are_recomputed_rather_than_carried`
fails:

```
AssertionError: Lists differ: ['8.0.0', 48] != ['8.0.0', 49]
```

`docs/reviews/r0_3/acceptance_matrix.json`'s `validation` block is the
**integrator-supplied merged gate** from wave 9 (`lean_audits: 48`,
`workspace_tests: 1888`, …). This card adds one `Audit*.lean` file, so the
derived count is now 49 and the committed value is stale. **That is card
GEN-REASON's tripwire working exactly as designed**, not a defect and not a
weakened gate.

`docs/reviews/r0_3/acceptance_matrix.json` is explicitly integrator-owned and
this card is forbidden from editing it, so the pin is **deliberately left for the
integrator** to move when regenerating the matrix with `--gate`:

* `validation.lean_audits`: 48 → **49**
* `validation.lean_audit_files`: add `AuditBoundFieldRead.lean`
* `validation.workspace_tests`: 1888 → this card's contribution plus the rest of
  the wave

No other Python test fails; the other 124 pass.
