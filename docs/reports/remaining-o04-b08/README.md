# O04-B08 — aggregate stores and the aggregate branch condition — 2026-09-09

The named continuation of card O04-ARMS
([`docs/reports/remaining-o04-arms/README.md`](../remaining-o04-arms/README.md)
§3.2), which settled four of the six exits of byte-verbatim
`flb_input_chunk_create` and recorded the routes-mask arm **C5**'s residual as
exactly three families: **aggregate stores** (O00 blocker **B08**, including
`ic->chunk = chunk`), the **aggregate branch condition**
`if (ic->routes_mask == NULL)`, and one deliberately uncontracted success-path
logging call.

This card takes the first two. Both were destination-side and condition-side
refusals that fire *before* any declared contract is consulted.

| Arm | O04-ARMS | This card |
|---|---|---|
| **C1** — logging arm, `!chunk` | Discharged | **Discharged** |
| **C2** — `up_force == -1` | Discharged | **Discharged** |
| **C3** — header write `== -1` | Discharged | **Discharged** |
| **C4** — `ic` calloc NULL | Discharged | **Discharged** |
| **C5** — routes-mask arm | `Unknown`, blockers named | **Discharged** — chunk **and** `ic` released, routes-mask refuted |
| **C6** — success + publication | `Unknown` (out of scope) | **Unknown** (out of scope, unchanged) |

**Five of six exits now settle, and every refusal left in the caller is on the
out-of-scope success exit's path.** B08 is closed at the boundary this card
states.

**Nothing is promoted.** Both real functions keep function-level status
`Unknown`; the corpus headline is unchanged; the contract corpus stays at 60
modules; CT coverage stays 28/50; `qcore-schemas` is unchanged at 7.0.0.

---

## 1. What the card admits

One analyzer capability, in a new
[`crates/qcore-c-gateway/src/aggregate_store.rs`](../../../crates/qcore-c-gateway/src/aggregate_store.rs).

### 1.1 The destination

Cards C02, C02-P and O04-ARMS admitted typed member *reads* and each recorded
that the **destination** side of the lexical gate was deliberately untouched.
This card lifts it for exactly one shape — a **single** `->` selection on a
local aggregate pointer this body acquired — under a conjunctive whitelist whose
every premise is typed and read from the same preprocessed token stream the
importer projects.

| # | Premise | Refused otherwise |
|---|---|---|
| 1 | the declaration reader accepted the function (`FunctionTypes::is_readable`) | nothing at all is admitted in that function |
| 2 | `BASE`'s declared type is exactly `struct TAG *`, `TAG` has one complete non-`union` definition whose every member declaration was read, and `BASE` is **not a formal parameter** | a formal, a record value, a multi-star base, an incomplete/ambiguous/`union` record |
| 3 | `BASE` is the destination of exactly one top-level assignment, and that assignment's right-hand side **calls** something | a rebound base, and — crucially — an aggregate *read out of another object's memory* (`storage = in->storage;`) |
| 4 | `BASE` is never address-taken whole, incremented, compound-assigned, dereferenced (`* BASE`), subscripted or `.`-selected | every one of those |
| 5 | `BASE` is never the entire right-hand side of an assignment, and never a whole argument at an `Out`, `Stored` or `Removed` position of a contracted call | a second declared name for the same aggregate; a callee that could rebind or attach through it invisibly |
| 6 | the destination tokens are exactly `BASE - > FIELD` | `BASE->F->G`, `BASE.f`, `BASE[i].f`, a continued access |
| 7 | `FIELD` is either a scalar member (`TypedAccessModel::scalar_field`) or a member declared with **exactly one** `*` over a specifier list the reader resolved (`single_pointer_field`, new) | `void *`, multi-star, array, function-pointer, bit-field, aggregate sub-object, `volatile`/`_Atomic`, unresolved typedef |
| 8 | the value is an admitted scalar expression into a scalar member; or, into a pointer member, the null constant, a declared pointer variable of **exactly** the member's declared record-pointer type, or the result of a contracted call | everything else, including a pointer into a scalar member and a partial expression |

Premise 3's second half is not decoration. Without it, `storage = in->storage;`
would make `storage` an admissible store base, and card C02-P's fail-closed pin
`a_store_through_the_field_between_the_read_and_the_call_is_never_admitted`
would stop reporting its own diagnostic. **That pin caught the first
implementation and was not weakened**; the premise was added instead. This is
the same shape of finding C02 records for its own `untracked_bases` premise.

### 1.2 The flow-sensitive aggregate premise

Declarations are not enough. At the store itself, `legacy_c::transfer` requires
the aggregate to resolve to **exactly one live identity this body acquired**:

* the projection binds the base here, to exactly one site, not `unknown`;
* neither the pointer nor its site is possibly null;
* the site's status is exactly `LIVE` and carries no unresolved effect;
* the site is **not** entry custody the caller holds — a transfer into a
  caller-owned aggregate is invisible at the call site and is refused rather
  than modeled.

A store failing any of these reports `unknown_alias` on the aggregate's own name
and poisons the path. This is **card C02's fail-closed premise in its store
direction**: the read direction refuses a base the projection may bind, and the
store direction refuses a base that is not, right here, a live identity. A store
through a released aggregate can therefore never reach `Safe`.

### 1.3 The containment edge

An admitted store records a **field link** `(owner site, field) → child sites`
in the abstract state. It is deliberately **not** the `CONTAINED` relation a
declared `Stored` contract parameter creates, and the difference is the whole
modeling decision of this card:

| Event on the aggregate | `Stored` containment | B08 field link |
|---|---|---|
| escape (returned, consumed) | children escape | live children escape; any other state is recorded **unresolved** |
| release (`resource_released`) | children are released with it | **nothing happens** — premise **P-B08-SHALLOW** |
| a second store into the member | n/a | the link is replaced; the previously stored identity keeps its disposition, so a live one is reported as the residual it is |
| direct release of the child | `CustodyConflict` | permitted — custody never left this function |

**P-B08-SHALLOW**, stated plainly: *a declared `resource_released` release names
exactly one identity, and a plain C field store carries no declared destructor
obligation, so releasing an aggregate does not discharge what its members point
at.* It is a premise about what a declared contract means. It is **assumed, not
proved**, and it is the reason the routes-mask arm settles only because the body
releases the chunk itself before freeing `ic`.

If the field link used the `Stored` cascade instead, the real C5 arm —
`cio_chunk_close(chunk, CIO_TRUE); flb_free(ic);` — would report a
`CustodyConflict` on a correct program, because it releases a contained child
directly. That is the measured reason for the separate relation.

### 1.4 The branch condition

`Parser::null_condition` tries the existing `pointer_null_condition` first and
falls back to one bounded aggregate form: a null test (`!x`, `x == NULL`,
`x != NULL`, `NULL == x`) whose **entire** operand is an admitted single
selection on an admitted base naming a **pointer** member, in a condition with
no call, no braces and no other unmodeled access. Everything else — a scalar
member, a chained member, a member of a base this rule never admitted, an
arithmetic comparison — keeps the existing
`unmodeled call, aggregate or indirect access in branch condition` refusal.
Branch conditions in general stay out of scope (C02 §7).

### 1.5 Two refinements scoped to admitted stores

`ic->routes_mask = (flb_route_mask_element *) flb_calloc(...)` needed two more
things, and both fire **only** when `aggregate_store` admitted the destination,
so no other statement in the corpus can move:

1. The aggregate's own name is the modeled **destination**, not "a formal
   embedded in an unmodeled expression". Its liveness and single identity are
   required flow-sensitively by §1.2, which is strictly stronger than the
   syntactic gate. A protected name in the *right-hand side* still refuses.
2. One leading parenthesized cast between `=` and the call does not break the
   direct flow of the call's result into the member: it converts the result's
   declared type, and this projection tracks identities, not types. The call
   must still be the whole remainder of the statement (`is_leading_cast`).

---

## 2. Every entry premise, named

O04's nine named premises and O04-ARMS's two carry over unchanged and are still
**assumed, not derived** — including that the constructor's own body does not
discharge its declared contract. This card adds two:

| Premise | Status |
|---|---|
| **P-B08-SHALLOW** — a declared `resource_released` release names exactly one identity; a plain C field store carries no declared destructor obligation | **assumed** (§1.3) |
| `flb_time_now` takes no argument and returns no custody | **assumed**, declared in [`o04_b08_callees.qr`](../../../crates/qcore-c-gateway/tests/fixtures/contracts/o04_b08_callees.qr) |

Two premises are inherited unchanged from C02 and are **not** proved:
declaration completeness, and C effective types. C02-P's recorded may-alias
boundary (**C02-P-ALIAS**) is inherited and **not widened**: the field link is
keyed by the aggregate's *site*, not its name, so two names denoting the same
aggregate update the same link; and premise 5 refuses a base that could acquire
a second declared name at all.

### 2.1 A correction to O04-ARMS, by measurement

O04-ARMS §1.3 measured `flb_time_now` **inert** and deliberately left it
undeclared, because `ic->create_time = flb_time_now();` "is an aggregate *store*
that refuses before any contract is consulted, so declaring it would buy
nothing". That was true then and is false now: with the store admitted, the call
is no longer inert and becomes C5's blocker. The measurement moved with the
capability, and the O04-ARMS pin is updated to record exactly that (§4.3).

---

## 3. The measured per-arm disposition

Raw ledger: [`logs/arm_ledger.txt`](logs/arm_ledger.txt), regenerable with the
`print_the_measured_arm_ledger` reporting aid in
`crates/qcore-c-gateway/tests/o04_b08.rs`.

### 3.1 `flb_input_chunk_create` — six exits

| O00 exit | Upstream line | Arm | O04-ARMS | This card | Residual |
|---|---|---|---|---|---|
| C1 | 2530 | constructor returned NULL | Discharged | **Discharged** | none acquired |
| C2 | 2541 | `up_force == -1` | Discharged | **Discharged** | chunk **Released** |
| C3 | 2550 | header write `== -1` | Discharged | **Discharged** | chunk **Released** |
| C4 | 2558 | `ic` calloc NULL | Discharged | **Discharged** | chunk **Released** |
| C5 | 2587 | `routes_mask` calloc NULL | Unknown | **Discharged** | chunk **Released**, `ic` **Released**, routes-mask refuted absent |
| C6 | 2620 | success + publication | Unknown | **Unknown** (out of scope) | all three unresolved |

All four cleanup arms name the **same** acquisition site — the constructor call
at upstream 2525 — and C1 crosses no acquisition at all. **No residual is
alleged on this path in either direction:** no exit is a `Residual`, no `Leak`
is reported, and C6 carries its three identities as `Mixed`, never as a claim.

### 3.2 C5's residual, before and after

| Family | O04-ARMS | This card |
|---|---|---|
| aggregate **stores** into `ic` (upstream 2561–2586, twelve statements) | **open** — B08 | **admitted** |
| the aggregate **branch condition** `if (ic->routes_mask == NULL)` (upstream 2587) | **open** | **admitted** |
| the deliberately uncontracted success-path `flb_trace(...)` | open by decision | unchanged — it is on C6's path only |
| `flb_time_now` (upstream 2569) | measured inert | **declared** as a named premise (§2.1) |

C5 has no residual left. The refusals that remain in the caller are all on C6's
path: `flb_routes_mask_set_by_tag(ic->routes_mask, ...)` (a *read* of a member
through a base the projection binds — refused, and out of scope for this card),
`flb_trace`, `msgpack_packer_init`, `mk_list_add`, the four
`flb_hash_table_add` calls, the two `pthread_mutex_*` calls, and `ic` reaching
the success exit possibly-null. All are B09/B10 territory or deliberate.

### 3.3 `cio_chunk_open` — a finer projection, the same boundary

Four guard/allocation-failure exits stay **Discharged**; the rollback and
success exits stay **Unknown**. Its blockers are the two uncontracted backend
constructors, `ch->backend` (a `void *` member the reader will not type, so
premise 7 refuses it), and the list operations through `&`.

One thing did move: `ch->name = strdup(name);` is an admitted store, so the
`strdup` allocation this projection previously erased is now a **tracked
identity**. Both unresolved exits therefore carry two `Mixed` sites instead of
one. The projection got finer, not more permissive: nothing is discharged that
was not, and no leak is alleged. The constructor's declared contract remains a
**named premise**, not a fact its body discharges.

---

## 4. Evidence

### 4.1 Held-out generic family

[`crates/qcore-c-gateway/tests/o04_b08.rs`](../../../crates/qcore-c-gateway/tests/o04_b08.rs),
a `k_*` API deliberately unrelated to ChunkIO, to Fluent Bit **and** to the
`g_*` (O04) and `h_*` (O04-ARMS) families. Every rule is stated over declared C
types and contract modes, so no name in it can matter.

| Test | Result |
|---|---|
| `an_aggregate_store_and_its_branch_condition_discharge_every_exit` | the real C5 shape, synthetically: acquire, store, acquire into a member, branch on it, clean up — **Safe**, every exit `Discharged`, no refusal left |
| `publishing_the_aggregate_carries_its_linked_child` | the escape half of the containment edge: the success exit reports no leak |
| `a_double_store_reports_the_first_resource_as_a_leak` | **Unsafe**, exactly one `Leak` — a second store never discharges the first resource |
| `a_store_through_a_released_aggregate_is_never_admitted` | never `Safe`; reports `unknown_alias` on the aggregate (§1.2) |
| `a_store_into_a_borrowed_in_aggregate_is_never_admitted` | a caller-owned aggregate keeps its existing `Unknown` **and** its existing diagnostic |
| `every_refused_base_keeps_its_unknown_and_its_diagnostic` | six base shapes — read out of another object, copied to a second name, address-taken, bound twice, dereferenced, subscripted — each keeps `Unknown` and the existing `indirect memory access…` diagnostic |
| `every_refused_member_keeps_its_unknown_and_its_diagnostic` | six member shapes — `void *`, double star, array, `volatile`, chained destination, a member read as the value — same discipline |
| `a_branch_on_an_unrelated_field_keeps_its_existing_refusal` | a scalar member, a member of an unadmitted base and a chained member each keep the existing **branch** refusal |
| `a_store_never_manufactures_a_discharge` | P-B08-SHALLOW's reporting half: releasing the aggregate without releasing its member is **Unsafe** with a `Residual` |
| `a_borrowed_destructor_turns_the_store_path_arm_into_a_residual` | weakening the declared release mode reports, never silently discharges |
| `a_certificate_resting_on_an_aggregate_store_is_rederived_from_source` | five declaration substitutions the **independent recheck** refuses; the unmodified certificate still rechecks |

Plus eleven unit tests in `aggregate_store`'s own module covering the base scan
and the value classification directly.

Two directions are **not expressible** and are recorded rather than skipped: a
pointer stored into a scalar member, and a duplicated aggregate tag, are both
rejected by Clang before the analyzer sees them. Both are covered by the
module's unit tests and by the generic certificate substitutions instead.

### 4.2 Real-source recheck

| Test | Result |
|---|---|
| `the_routes_mask_arm_discharges_under_the_declared_premises` | 5 Discharged / 1 Unknown; site counts `[0,1,1,1,2]`, every site `Released`, all four cleanup arms on one acquisition site; the function stays **Unknown** |
| `every_remaining_refusal_belongs_to_the_out_of_scope_success_exit` | every remaining refusal is `INDIRECT`, `flb_trace`, or `ic` possibly-null; **no leak is alleged** |
| `the_constructor_body_still_does_not_discharge_its_own_contract` | 4 Discharged / 2 Unknown, both carrying two `Mixed` sites; no leak alleged |
| `substituting_a_member_declaration_withdraws_the_arm` | four substitutions on the real fixture — double pointer, `volatile`, an unresolved pointee, a scalar member that stops being one — each withdraws the discharge |
| `weakening_the_destructor_premise_withdraws_the_arms` | making `cio_chunk_close` borrow its argument reports leaks instead of discharging |

### 4.3 Deliberate pin updates

Three pinned expectations were updated, each keeping its load-bearing negative
half. No diagnostic code was added, removed or downgraded, and no gate was
weakened.

1. **`o04_arms::the_routes_mask_arm_records_its_named_residual`.** The store and
   branch-condition families it named as open are now admitted, so they no
   longer appear; `call to uncontracted function flb_time_now` does (§2.1). The
   row's structural half is untouched and still asserted: under *O04-ARMS's
   byte-unchanged premise set* the arm is still not discharged, both Unknown
   exits carry both identities unresolved, and no exit is a `Residual`.
2. **`o04_arms::the_constructor_body_still_does_not_discharge_its_own_contract`**
   and
3. **`o04_constructor_caller::the_constructor_body_does_not_discharge_its_own_declared_contract`.**
   Both pinned `sites == vec![Mixed]` on the constructor's two unresolved
   exits. `ch->name = strdup(name)` is now tracked, so each row asserts that
   **every** site is `Mixed` (the load-bearing half) and pins the arity of two
   separately, so a silent change in either direction still fails.

`o04_path.qcore` and `o04_arms_callees.qcore` are **byte-unchanged**, so cards
O04 and O04-ARMS keep their exact premise sets. This card's one added
declaration lives in its own fixture file, deliberately outside
`contract/qcore`, so the pinned corpus count is untouched.

---

## 5. Corpus impact

### 5.1 The 185-unit artifacts — 38 units move, no verdict does

Re-measured through the canonical pinned route on the prepared pinned clone
(Fluent Bit `66910c10a4d7eafa810b84229cf2dcf7b0f26f97`, explicit
`/usr/bin/clang` 21.1.8), following
[`docs/reports/corpus-repin/provenance.json`](../corpus-repin/provenance.json):

```sh
python3 -B providers/package.py build --output build/cycle19/provider-packages-b08
QCORE_CLANG=/usr/bin/clang \
QCORE_CHUNKIO_PACKAGE=build/cycle19/provider-packages-b08/qcore.chunkio \
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  --pinned-evidence <clone> <monkey>/compile_commands.json \
  <reports>/monkey-source.json <reports>/resource-diagnostics.json
```

Run **twice from this same worktree**, with only `crates/qcore-c-gateway/src/`
reverted to the branch base `05c51a5` between runs, so the comparison carries no
path provenance at all. Both exit 0 with zero gateway failures. Driver:
[`build/cycle19/run_audit.sh`](../../../build/cycle19/run_audit.sh) (not
committed; the command above is complete).

**Harness validation.** The *before* run reproduces the current archived
baseline [`docs/reports/remaining-e01-f1/artifacts/after/`](../remaining-e01-f1/artifacts/after)
exactly where analysis is concerned:

| Artifact | before vs archived baseline |
|---|---|
| `diagnostics.json` | **byte-identical** (SHA-256 `f4431c2cfcaae37c…`, matching that manifest's `uncompressed_sha256`) |
| `chunk-contracts.json` | **byte-identical** (SHA-256 `29122661dfe34b04…`) |
| `contracts.json` | identical after collapsing the checkout path — embedded contract source paths only |
| `source.json` | 4 wrapper rows differ in one leaf: `…expression.value.c_type` reads `char[147]` here and `char[189]` in the archive — a string-literal **array length** that follows the absolute path length of the Monkey source tree |
| `source-effects.json` | 3 rows differ in `evidence.reproducibility.package_digest`, `provider_version` and the `resource_contracts:<digest>` bound-key — the provider packages were rebuilt for this run, and the contract-bundle digest folds absolute contract paths |

The exhaustive `diff_artifacts.py` document reports **0 added / 0 removed /
0 changed** rows in `diagnostics.units`, `chunk-contracts.results` and
`chunk-contracts.backend_teardown`
([`artifacts/diff-baseline-to-before.json.gz`](artifacts/diff-baseline-to-before.json.gz)).
Every residue is the same location- and identity-provenance class O04-ARMS,
CORPUS-REPIN and E01-F1 each recorded, present *before* any change of mine. That
is what makes the before/after diff below an isolation of this card.

**Headline, pinned route, 185 units: 2 Safe / 42 provisional Unsafe /
137 Unknown / 4 data-only.** Identical before and after, and identical to the
archived pin.

| | before | after |
|---|---|---|
| units | 185 | 185 |
| projected functions | 4,834 | 4,834 |
| `Safe` / provisional `Unsafe` / `Unknown` / data-only units | 2 / 42 / 137 / 4 | 2 / 42 / 137 / 4 |
| unit status moves | — | **0** |
| unit `reason_code` moves | — | **0** |
| function status moves | — | **0** |
| `leak` diagnostics | 271 | **271** |
| `unknown_alias` diagnostics | 4,286 | 4,386 (**+100**) |
| `unsupported` diagnostics | 50,089 | 49,936 (**−153**) |

| Artifact | before → after |
|---|---|
| `chunk-contracts.json` | **byte-identical** (96 results, 3 checked / 93 unknown; 21 backend-teardown rows unchanged) |
| `source.json` | **byte-identical** (3 callbacks, 15 wrappers) |
| `source-effects.json` | **byte-identical** (96 results) |
| `contracts.json` | **byte-identical** |
| `diagnostics.json` | 38 of 185 units changed; **no unit verdict change** |

Four of the five artifacts are byte-identical, which is the tightest isolation
this route can give: the whole effect of this card is a diagnostic-level change
inside `diagnostics.json`.

**Three facts the integrator needs, each measured exhaustively rather than
argued** ([`census.py`](census.py), transcript at
[`logs/census.txt`](logs/census.txt)):

* **No newly surfaced provisional `Unsafe` unit.** The unit status histogram is
  identical, no unit row changed `status`, and the corpus `leak` count is
  unchanged at **271**. This card *can* surface a leak in principle — an
  allocation stored into an admitted member (`ch->name = strdup(name)`) becomes
  a tracked identity where it was previously erased — so this was the direction
  to check first. Measurement: none did.
* **No function-level `Unknown` → `Safe` movement.** All 4,834 projected
  functions were compared pairwise: `function status moves: 0`. The routes-mask
  arm settles at *exit* level; `flb_input_chunk_create` itself stays `Unknown`
  because its success exit does.
* **Every diagnostic movement is in the fail-closed direction.** 153 refusals
  disappear and 100 unresolved-identity reports appear. The 53-row net
  difference is the genuinely custody-neutral stores — a scalar member written
  from a scalar expression, or a pointer member written from a value this
  function never binds. The 100 additions are stores whose aggregate or value
  the projection cannot resolve here: previously the whole statement was refused
  wholesale, now the unresolved identity is *named*. Neither direction can
  produce a `Safe` that was not there before.

Per-unit deltas, all 38, are in [`logs/census.txt`](logs/census.txt). The
largest single mover is the card's own subject,
`fluent-bit-chunk/flb_input_chunk.c` (`unsupported −22`, `unknown_alias +12`),
followed by `cmetrics/cmt_exp_histogram.c` (`−9 / +8`) and
`monkey-libevent/mk_server/mk_http_thread.c` (`−7 / +7`).

Both artifact sets, their manifests and two machine-readable diff documents
(baseline→before, before→after, both generated with
`docs/reports/corpus-repin/diff_artifacts.py --full`) are archived under
[`artifacts/`](artifacts/). **The re-pin decision is the integrator's**: this
card archives its measurement and does not overwrite
`docs/reports/remaining-e01-f1/artifacts/`.

### 5.2 The three configured audits — verdicts unchanged, diagnostics move

Run in this same worktree, before and after, with only
`crates/qcore-c-gateway/src/` reverted between runs. Transcripts in
[`logs/`](logs/).

| Audit | before | after | delta |
|---|---|---|---|
| `contract/profiles/cmetrics-counter` | 1 unit: 0 Safe / 1 provisional Unsafe / 0 Unknown; 1 checked function contract / 6 Unknown | **identical verdicts** | `cmt_counter.c`: `leak` 2 → 2, `unknown_alias` 2 → 4, `unsupported` 35 → 32 |
| `contract/profiles/chunkio-core` | 13 units: 0 Safe / 2 provisional Unsafe / 11 Unknown | **identical verdicts** | `chunkio.c` `−4/+4`, `cio_chunk.c` `−6/+6`, `cio_memfs.c` `−4/+3`; `leak` unchanged |
| `contract/profiles/libevent-core` | 21 units: 0 Safe / 4 provisional Unsafe / 17 Unknown | **identical verdicts** | `buffer.c` `−8/+8`, `bufferevent_pair.c` `−7/+2`, `evmap.c` `−10/+10`, `bufferevent.c` `−5/+1`; `leak` unchanged |

No unit changed verdict in any of the three, no `leak` count moved anywhere, and
the CMetrics run still reports one checked function contract with six Unknown
functions and no whole-unit promotion.


---

## 6. Lean parity

[`proof/QCore/AggregateStore.lean`](../../../proof/QCore/AggregateStore.lean),
**42 theorems**, registered as the new
[`proof/AuditAggregateStore.lean`](../../../proof/AuditAggregateStore.lean) and
checked by `check_lean_audits.py` (44 audits → **45**). Only `propext` and
`Quot.sound`; no new axiom, no placeholder, no `sorry`.

* **Every premise of the destination whitelist refuses on its own** — thirteen
  theorems, one per conjunct, including
  `an_unacquired_base_never_admits_a_store` (the premise that keeps C02-P's pin
  refusing) and `an_aliased_base_never_admits_a_store`.
* **The flow-sensitive aggregate premise** — eight theorems, including
  `a_released_aggregate_never_resolves`, which is card C02's fail-closed rule in
  its store direction, and `a_caller_held_aggregate_never_resolves`.
* **Fail-closed transfer, both directions** — `a_refused_store_never_links`,
  `an_unresolved_owner_never_links`, and `only_a_doubly_admitted_store_links`,
  which shows the two gates are *jointly* necessary and that only a tracked
  value ever links.
* **The containment edge** — `an_escaping_aggregate_carries_its_live_child`,
  `the_escape_cascade_never_releases` (the cascade cannot manufacture a
  discharge), `a_released_aggregate_changes_no_child` (P-B08-SHALLOW as a
  theorem about the model), and
  `a_double_store_never_discharges_the_first_identity`.
* **Bridging into the exit ledger** — a refused store settles no exit; a live
  linked child on a modeled path is a `residual`, not a discharge.
* **Non-vacuity in both directions** — an admitted store exists and links, and
  five separately flipped premises each refuse it.

The model is of the **decisions**, not of C semantics. Declared types, record
facts, base-scan flags and the owner's abstract state are premises the reader
and the transfer function supply; nothing proves either extracts them
faithfully, and nothing here is a claim about arbitrary C memory safety.
P-B08-SHALLOW is modeled and its consequences are proved; **that it is true of
the real contracts is not proved anywhere.**

---

## 7. Open boundaries

Carried forward unchanged: O04's blockers **B02–B07**, **B09–B11**, and
O04-ARMS's **O04-ARMS-VARIADIC**, **O04-ARMS-SCALAR-CHAIN** and
**C02-P-ALIAS**.

Closed by this card: **B08**, at the boundary §1 states.

| # | Boundary | Status |
|---|---|---|
| **P-B08-SHALLOW** | a declared release names exactly one identity, so releasing an aggregate does not discharge what its members link | **assumed**, modeled and proved-about, never proved |
| **O04-B08-BORROWED-AGG** | a store into an aggregate the *caller* owns is refused, not modeled; the card models publication, not interface change | **refused by construction** |
| **O04-B08-READ-BOUND-BASE** | a member *read* through a base the projection binds (`flb_routes_mask_set_by_tag(ic->routes_mask, …)`) keeps C02-P premise 4's refusal; the store side is admitted, the read side is not | **open**, and the last blocker between C6 and B09/B10 |
| **O04-B08-DOT** | a record-valued base (`agg.field = …`) is refused; only `->` on a single-star record pointer is admitted | **refused by construction** |
| **O04-B08-CAST** | the leading-cast refinement (§1.5) is scoped to admitted stores; a cast anywhere else keeps its existing treatment | **recorded** |
| — | chained destinations, `void *` members, multi-star members, array members, bit-fields, `union` bases, qualified members, aggregate sub-objects | **refused by construction** |
| **E01-F1-RAW** | another wave-9 card touches the gateway's raw tokenizer; this card touches `legacy_c.rs`'s statement classifier, the typed reader and a new module. **Overlap recorded for the integrator, not resolved here** (§9) | **recorded** |

---

## 8. Scope and non-claims

- **No raw unit is promoted.** Both functions on the path keep status
  **Unknown**; the corpus aggregate is unchanged.
- A per-exit **Discharged** is a claim about *this projection under the named
  premises of §2*, not a proof that the C is correct. Every declared contract it
  rests on is a trusted assumption, and P-B08-SHALLOW is a new one.
- A provisional `Unsafe` is never presented as an upstream defect; the `k_*`
  family is synthetic.
- **Unknown means Unknown.** An exit whose path carried a refusal alleges
  nothing in either direction.
- Projection coverage is not safety coverage; unknowns stay in the denominator.
- CT coverage is unchanged at 28/50; the contract corpus remains 60 modules;
  `qcore-schemas` is unchanged at 7.0.0; no plugin, protocol or evidence version
  moved.

---

## 9. Cross-card overlap, recorded for the integrator

Card **E01-F1-RAW** (wave 9) is named as touching the gateway's *raw tokenizer*
— the `#`-on-a-continuation-line defect E01-F1 recorded. This card touches:

* `crates/qcore-c-gateway/src/aggregate_store.rs` (new);
* `crates/qcore-c-gateway/src/legacy_c.rs` — the statement classifier, the
  branch-condition entry, `AbstractState`, `transfer`, and the four cascade /
  refinement helpers;
* `crates/qcore-c-gateway/src/typed_field_access.rs` — one additive member
  classification (`single_pointer_fields`) and one visibility change;
* `crates/qcore-c-gateway/src/borrowed_actual.rs` — one additive accessor.

The two cards therefore both edit `crates/qcore-c-gateway/`, but at different
layers: E01-F1-RAW's subject is the *scanner that enumerates definitions*, and
this card's is the *statement classifier and the abstract transfer*. If the raw
tokenizer's repair changes which text `legacy_c` sees, this card's corpus
measurement (§5) must be re-run after that merge. **Recorded, not resolved.**

---

## 10. Files changed

| Path | Change |
|---|---|
| `crates/qcore-c-gateway/src/aggregate_store.rs` | new — the destination whitelist, the base scan, the value classification, 12 unit tests |
| `crates/qcore-c-gateway/src/typed_field_access.rs` | additive `single_pointer_fields` member map and `record_definition_is_complete`; `scalar_field` made crate-visible |
| `crates/qcore-c-gateway/src/borrowed_actual.rs` | additive `UntrackedBases::never_bound` |
| `crates/qcore-c-gateway/src/legacy_c.rs` | `AggregateStore` instruction, `AbstractState::field_links` and its join, the store admission in `read_statement`, `Parser::null_condition`, the transfer pre/post steps, `cascade_field_links_on_escape`, `caller_visible_containment`, `is_leading_cast`, `TransferContext::caller_sites` |
| `crates/qcore-c-gateway/src/lib.rs` | module registration |
| `crates/qcore-c-gateway/tests/o04_b08.rs` | new — 16 assertions plus one `#[ignore]`d reporting aid |
| `crates/qcore-c-gateway/tests/fixtures/contracts/o04_b08_{callees,generic}.qcore` | new — the one added premise and the held-out `k_*` API |
| `crates/qcore-c-gateway/tests/o04_arms.rs` | two pinned rows updated deliberately (§4.3) |
| `crates/qcore-c-gateway/tests/o04_constructor_caller.rs` | one pinned row updated deliberately (§4.3) |
| `proof/QCore/AggregateStore.lean`, `proof/QCore.lean`, `proof/AuditAggregateStore.lean` | new module and its axiom audit |
| `docs/reports/remaining-o04-b08/` | this record, the arm ledger, the configured-audit transcripts and the re-measured artifacts |

No file owned by the integrator was touched: `AGENTS.md`, `CLAUDE.md`,
`README.md`, `DEVELOPMENT.md`, `docs/reevaluation/ct_manifest.json`,
`docs/reviews/r0_3/acceptance.json` and
`docs/reviews/r0_3/acceptance_matrix.json` are unchanged, as is
`contract/qcore/` and every archived predecessor report.

---

## 11. Gate results

Run in this worktree at branch `claude/o04-b08`, base `05c51a5`, with the
Libevent submodule initialised.

| Gate | Command | Observed | Exit |
|---|---|---|---|
| Workspace tests | `cargo test --workspace --no-fail-fast` | **1,814 passed / 0 failed / 4 ignored** (base was 1,787 / 3) | 0 |
| Format | `cargo fmt --all -- --check` | clean | 0 |
| Clippy | `cargo clippy --all-targets -- -D warnings` | clean | 0 |
| Probe | `cargo run -p qcore-compiler --example reevaluation_probe` | ran | 0 |
| Follow-up | `cargo run -p qcore-compiler --example reevaluation_followup` | ran | 0 |
| Python | `python3 -B -m unittest discover -s scripts/tests` | **108 ran, OK** | 0 |
| Lean build | `cd proof && lake build` | completed | 0 |
| Lean audits | `python3 -B scripts/check_lean_audits.py` | **45 audits passed** (44 → 45) | 0 |
| Provider tests | `cargo test --locked --offline --manifest-path providers/Cargo.toml --workspace` | **67 passed / 0 failed** | 0 |
| Provider packages | `python3 -B providers/package.py build` | five packages built and verified | 0 |
| 185-unit corpus | `cycle19_fluent_bit_libraries_audit --pinned-evidence`, before and after | §5.1 | 0 / 0 |
| Configured audits | `qcore-audit` over cmetrics-counter, chunkio-core, libevent-core, before and after | §5.2 | 0 / 0 |

The card contributes **27 new assertions** (16 integration, 11 unit) plus one
`#[ignore]`d reporting aid, and **42 new Lean theorems**. The workspace tally
moves 1,787 → 1,814 passed and 3 → 4 ignored, which is exactly this card's
contribution and nothing else.

