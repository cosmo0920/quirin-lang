# O04-ARMS — per-arm discharge on the remaining early-return arms — 2026-09-09

> Historical checkpoint: source filenames, runnable references or discovery scripts in this archive were updated by the [extension migration](../extension-physical-migration/README.md). Recorded results, hashes and gate counts describe the original revision, not a new run of the maintained files. Use [current status](../../STATUS.md) for present measurements.


> **Superseded as the current baseline, 2026-09-09 (later):** card E01-F1
> re-measured through the same pinned route; its before-run reproduces
> `artifacts/` here byte-identically (0 added / 0 removed / 0 changed on the
> exhaustive diff), and its after-run — denominator 4,775 → 4,921, zero unit
> verdict movement, four of five artifacts byte-identical beyond the additive
> enumeration block — is the current baseline at
> `docs/reports/remaining-e01-f1/artifacts/`. This directory remains the
> O04-ARMS measurement record.

The named continuation of card O04
([`docs/reports/remaining-o04/README.md`](../remaining-o04/README.md) §7), which
discharged the caller's three modeled cleanup arms of `flb_input_chunk_create`
and recorded two reasons the rest stayed `Unknown`: **chained accesses are
refused by construction** (arm C1) and **aggregate stores and branch conditions
are out of scope** (arm C5).

This card takes both remaining early-return arms and, for each, either
discharges the `cio_chunk_open` custody exactly once or records its named
residual.

| Arm | Before this card | After |
|---|---|---|
| **C1** — logging arm, `!chunk` | `Unknown` (chained read `storage->stream->name`, uncontracted `flb_error`) | **Discharged** — no identity acquired on the path |
| **C5** — routes-mask arm | `Unknown` | **Unknown**, with its blockers narrowed to two named families (§3.2) |

The success exit **C6**, the unenumerated exit **C8**, the descriptor and
mapping identities **ID-D/ID-M** and `cio_scan_stream_files` were out of scope
for O00 §8 and remain out of scope here.

**Nothing is promoted.** Both real functions keep function-level status
`Unknown`; across all 4,688 projected functions of the pinned 185-unit corpus
**no function changed status in either direction**, no unit changed verdict, no
newly surfaced provisional `Unsafe` unit exists, the canonical CMetrics audit is
byte-identical, the contract corpus stays at 60 modules, CT coverage stays
24/50, and `qcore-schemas` is unchanged.

---

## 1. What the card admits

One analyzer capability, in `crates/qcore-c-gateway`:

### 1.1 The bounded, fully typed access chain

C02-P admitted exactly one member selection and refused every chain by
construction. `borrowed_actual::admitted_access` now walks **one to four**
selections, and every premise is conjunctive, typed, and read from the same
preprocessed token stream the importer projects:

| # | Premise | Refused otherwise |
|---|---|---|
| 1 | the base is a direct identifier whose declared type matches the first operator | computed bases |
| 2 | every selection *before the last* is written `->` and its member is declared **exactly** `struct POINTEE *` (`TypedAccessModel::record_pointer_field`) | `union` pointee, multi-star member, record-valued member, array member, qualified member, a typedef the reader will not resolve |
| 3 | the walk re-enters the declaration model at `POINTEE`, so **every** link's record must be a `struct` defined exactly once whose every member declaration was read | incomplete, ambiguous or `union` link at any depth |
| 4 | the last selection satisfies C02-P's own member premise (scalar or pointer, no `volatile`/`_Atomic`) and does not continue into `->`, `.`, `[` or `(` | aggregate final member, continued access |
| 5 | at most `ACCESS_CHAIN_LIMIT = 4` selections | a longer chain is **refused, never truncated** to an admitted prefix |
| 6 | C02-P premises 1, 4 and 5 unchanged: the root carries no tracked identity (`untracked_bases`), and for the actual form the contract in force declares that index `Borrowed` | bound roots, consuming positions, uncontracted callees, variadic tails |

A `.` may appear only as the *first* selection, on a record-valued base;
continuing with `.` would cross a nested aggregate sub-object and is refused.

**The destination side of the lexical gate is untouched.** Every store through
`->`, `.` or `[` keeps its existing rejection and its existing diagnostic, which
is what keeps an admitted place from ever being bound.

### 1.2 The alias premise, and why the chain does not widen it

A chain reads through intermediate *places* this function never names, so the
`untracked_bases` closure cannot be consulted for them. That is the same
exposure premise (4) already accepts at depth one, and that C02-P recorded as
**C02-P-ALIAS**: two distinct declared names that may alias are unrelated by
this name-based projection.

Nothing new is assumed, and the argument is measured rather than asserted: an
intermediate place can only be released *inside this function* through a local
bound by an **opaque** field read, and releasing an opaque-bound name reports
`UnknownAlias`, so no such body can reach `Safe`. That containment is pinned by
`releasing_an_opaque_bound_name_is_never_silently_admitted`.

### 1.3 What the card's hypothesis got wrong, by measurement

The task named three prerequisites from O00 §8. Two of the three were measured
**not** to be the blocker:

* **Array declarators (O00 §8 feature P2) were already satisfied.** O04's
  constant-array-declaration admission covers `char name[64];`, and
  re-measurement of the real path finds **no array-shaped refusal at all** on
  either remaining arm — neither in a declaration, nor for `name` as an actual,
  nor for a member of the 239-member upstream `struct flb_input_instance`.
  Array *members* stay excluded from both `borrowable_fields` and
  `record_pointer_fields`, so an array can never carry a chain; that is pinned,
  not newly built.
* **The three borrowed callee declarations are two.** O00 §8 feature P3
  predicted `flb_log_check`, `flb_log_print` and `flb_errno_print`; the
  byte-verbatim upstream region writes `flb_error(...)`, the macro those expand
  from, so the prediction collapses to **one** declaration. A second declaration
  that P3 did not name — `flb_routes_mask_get_size` — was found by measurement.
  The third uncontracted callee on the arms, `flb_time_now`, is measured
  **inert**: its statement `ic->create_time = flb_time_now();` is an aggregate
  *store* that refuses before any contract is consulted, so declaring it would
  buy nothing and it is deliberately left undeclared.

Both declarations live in
[`o04_arms_callees.qr`](../../../crates/qcore-c-gateway/tests/fixtures/contracts/o04_arms_callees.qr),
concatenated onto O04's premise set, which stays **byte-unchanged** so O04's own
measurement keeps its exact premises. The file sits with the gateway test
fixtures, deliberately outside `contract/qcore`, so the pinned corpus count is
untouched and nothing here is offered as a production declaration.

**One limitation recorded rather than hidden.** `flb_error` is declared
`void flb_error(const char *format, ...)` in C, and the resource contract
vocabulary has no variadic-tail form. The declaration therefore covers exactly
the three-argument shape this call site uses; a call at any other arity is
**refused**, not silently covered, because an actual beyond the declared
parameter vector is never admitted. Generalising to a declared variadic tail is
the named follow-on **O04-ARMS-VARIADIC**. No `qcore-schemas` change was made
for it.

---

## 2. Every entry premise, named

O04 §3's nine named premises carry over unchanged and are still **assumed, not
derived** — including that the constructor's own body does not discharge its
declared contract. This card adds two, both trusted declarations:

| Premise | Status |
|---|---|
| `flb_error` borrows all three of its arguments and returns no custody | **assumed** (§1.3 limitation applies) |
| `flb_routes_mask_get_size` borrows the router and returns a scalar | **assumed** |

Two premises are inherited unchanged from C02 and are **not** proved:
declaration completeness, and C effective types.

### 2.1 The chain's intermediate links, measured against upstream bytes

C1's discharge rests on `storage->stream->name`, whose second link is
`struct cio_stream`. That record is a *reduction* in O04's prelude, so this card
pins the **upstream** declaration and measures the premise directly rather than
assuming the reduction is faithful:

| Region | Upstream | Lines | Bytes |
|---|---|---|---|
| `struct cio_stream` | `lib/chunkio/include/chunkio/cio_stream.h` | 25–33 | 479 |
| `struct flb_config` | `include/fluent-bit/flb_config.h` | 60–365 | 10,180 |

Both at Fluent Bit `90a010910e5b3328f496cc20fadc937547133439`, the revision O04
pinned; digests in
[`fixtures/o04_arms/provenance.json`](../../../crates/qcore-c-gateway/tests/fixtures/o04_arms/provenance.json)
and asserted by `the_intermediate_link_declarations_are_the_upstream_bytes`.
`the_upstream_link_declarations_admit_both_chains` then feeds the **full**
upstream `struct flb_input_instance` (239 members), `struct flb_storage_input`,
`struct cio_stream` and the 306-line `struct flb_config` through the pure-token
entry point and checks that both real chains — `storage->stream->name` and
`in->config->router` — are admitted there too. O04's own fixture directory is
byte-unchanged.

---

## 3. The measured per-arm disposition

Raw ledger: [`logs/arm_ledger.txt`](logs/arm_ledger.txt), regenerable with the
`print_the_measured_arm_ledger` reporting aid.

### 3.1 `flb_input_chunk_create` — six exits

| O00 exit | Upstream line | Arm | O04 | O04-ARMS | Residual |
|---|---|---|---|---|---|
| C1 | 2530 | constructor returned NULL | Unknown | **Discharged** | none acquired |
| C2 | 2541 | `up_force == -1` | Discharged | **Discharged** | chunk **Released** |
| C3 | 2550 | header write `== -1` | Discharged | **Discharged** | chunk **Released** |
| C4 | 2558 | `ic` calloc NULL | Discharged | **Discharged** | chunk **Released** |
| C5 | 2587 | `routes_mask` calloc NULL | Unknown | **Unknown** | chunk + `ic`, both unresolved |
| C6 | 2620 | success + publication | Unknown | **Unknown** (out of scope) | chunk + `ic`, both unresolved |

Four of six exits now settle. The three cleanup arms still name the **same**
acquisition site — the constructor call at upstream 2525 — and C1 crosses no
acquisition at all, because the `!chunk` guard refines the identity to absent
before the logging call.

**No residual is alleged on this path in either direction:** no exit is a
`Residual`, and the two `Unknown` exits carry both identities as `Mixed`
(unresolved), never as a leak claim.

### 3.2 C5's named residual

Every refusal left in the caller belongs to exactly three named families, pinned
by `the_routes_mask_arm_records_its_named_residual`:

| Family | Statements | Status |
|---|---|---|
| aggregate **stores** into `ic` (`ic->chunk = chunk`, `ic->routes_mask = flb_calloc(...)`, the scalar member stores) | upstream 2561–2586 | **open** — O00 blocker **B08**; the admission covers reads only, and the destination side of the gate is deliberately untouched |
| the aggregate **branch condition** `if (ic->routes_mask == NULL)` | upstream 2587 | **open** — out of scope for C02 §7 and for this card |
| the deliberately uncontracted success-path `flb_trace(...)` | upstream 2599 | **open** by decision — it is on C6's path only, and keeping it uncontracted keeps a real refusal in place |

C5 is therefore not blocked by anything this card's admission family could have
reached: it needs a pointer-field custody *transfer* model, which is B08 and
stays open.

### 3.3 `cio_chunk_open` — unchanged

Four guard/allocation-failure exits stay **Discharged**; the rollback and
success exits stay **Unknown**, because its blockers are aggregate stores
(`ch->name = strdup(...)`, `mk_list_add(&ch->_head, &st->chunks)`) and the two
uncontracted backend constructors. The constructor's declared contract remains a
**named premise**, not a fact its body discharges — exactly as O04 recorded.

---

## 4. Evidence

### 4.1 Held-out generic family

`crates/qcore-c-gateway/tests/o04_arms.rs`, an `h_*` API deliberately unrelated
to ChunkIO, to Fluent Bit **and** to O04's own `g_*` family. Every rule is
stated over declared C types and contract modes, so no name in it can matter.

| Test | Result |
|---|---|
| `a_fully_typed_chain_discharges_every_exit` | a three-link chain at two borrowed positions; every exit **Discharged**, `Safe`, no refusal remains |
| `every_untyped_link_keeps_its_unknown_and_its_diagnostic` | seven shapes — a `union` intermediate, an intermediate whose record body the reader refuses, a chain past the bound, a subscripted link, a chain at a consuming position, a chain beyond the declared vector, a chain whose root the projection may bind — each keeps **Unknown** *and* its existing `indirect memory access…` diagnostic |
| `a_chain_never_manufactures_a_discharge` | an arm that returns without releasing is **Unsafe**, `Leak`, exactly one **Residual** with a Live identity |
| `releasing_an_opaque_bound_name_is_never_silently_admitted` | the §1.2 containment: releasing an opaque-bound intermediate never reaches `Safe` and is reported as `UnknownAlias` |
| `a_certificate_resting_on_a_chain_is_rederived_from_source` | six declaration substitutions on an *intermediate or final link* — union, unreadable member, array member, second star, duplicated tag, `volatile` final member — each refused by the independent recheck; the unmodified certificate still rechecks at the end |
| `a_borrowed_destructor_turns_the_chain_path_arm_into_a_residual` | weakening the declared release mode turns the arm into a reported **Residual**, never a silent discharge |

Plus eight new unit tests in `borrowed_actual`'s own module covering the walk
directly (admitted three-link chain, over-bound, union link, ambiguous tag,
double-pointer link, `.`-continuation, bound root, chain as an opaque read).

### 4.2 Real-source recheck

| Test | Result |
|---|---|
| `the_intermediate_link_declarations_are_the_upstream_bytes` | two SHA-256 pins, no Clang needed |
| `the_upstream_link_declarations_admit_both_chains` | the full upstream `flb_input_instance`, `flb_storage_input`, `cio_stream` and `flb_config` admit both real chains |
| `the_logging_arm_discharges_under_the_declared_callee_premises` | 4 Discharged / 3 cleanup arms on one acquisition site / 1 acquiring nothing; the function stays **Unknown** |
| `the_routes_mask_arm_records_its_named_residual` | 2 Unknown exits, both identities `Mixed`, **no** `Residual`, and every remaining refusal in the three named families of §3.2 |
| `substituting_a_chain_link_withdraws_the_logging_arm` | four substitutions on the real fixture — an unreadable member in `cio_stream`, `_Atomic` on `stream`, `volatile` and array on the chain's final member — each withdraws C1's discharge |
| `weakening_the_logging_premise_withdraws_the_arm` | making one `flb_error` argument `Consumed` withdraws the arm: the discharge follows a *declared* contract, not the statement's shape |
| `the_constructor_body_still_does_not_discharge_its_own_contract` | 4 discharged guard exits, 2 unresolved, unchanged |

Retagging `struct cio_stream` as a `union` is **not expressible** on the real
fixture — five other declarations in the same unit name the tag and Clang
rejects the unit — so that direction is covered by the unit tests and by the
generic certificate substitutions instead. This is recorded, not skipped.

### 4.3 Deliberate pin updates

Two pinned expectations were updated, each keeping its load-bearing half:

1. `borrowed_field_actuals::every_other_refused_shape_keeps_its_unknown_and_its_diagnostic`
   (C02-P) and
   `o04_constructor_caller::ambiguous_actual_shapes_keep_their_unknown` (O04)
   each pinned a row reading *"a chained access is not a direct member read"*.
   The chain rule deliberately admits `in->storage->cio` / `env->conf->ctx`, so
   each row now pins the same property on a chain the reader did **not** fully
   type — a `union` intermediate link — and keeps its `Unknown` plus its exact
   existing diagnostic. Both are pinned positively in `o04_arms.rs`.
2. `borrowed_actual::a_chained_access_is_never_admitted` is renamed
   `a_chain_through_an_undefined_record_is_never_admitted`, with the identical
   assertion and its real reason recorded.

**O04's own exit-count pins are unchanged** (3 Discharged / 3 Unknown), because
`flb_error` stays uncontracted in O04's premise set and its statement simply
moves from one refusal to another. No diagnostic code was added, removed or
downgraded, and no gate was weakened.

---

## 5. Corpus impact

### 5.1 The 185-unit artifacts — four units move, no verdict does

Re-measured through the canonical pinned route on freshly prepared pinned clones
(Fluent Bit `66910c10a4d7eafa810b84229cf2dcf7b0f26f97`, explicit
`/usr/bin/clang` 21.1.8), following
[`docs/reports/corpus-repin/provenance.json`](../corpus-repin/provenance.json):

```sh
python3 -B scripts/prepare_cycle19_clang_audit.py \
  --fluent-bit /home/cosmo/Documents/GitHub/fluent-bit \
  --revision 66910c10a4d7eafa810b84229cf2dcf7b0f26f97 --clang clang --clangxx clang++
python3 -B providers/package.py build --output build/cycle19/provider-packages-o04arms
QCORE_CLANG=/usr/bin/clang \
QCORE_CHUNKIO_PACKAGE=build/cycle19/provider-packages-o04arms/qcore.chunkio \
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  --pinned-evidence <clone> <monkey>/compile_commands.json \
  <reports>/monkey-source.json <reports>/resource-diagnostics.json
```

Run **twice**: once from a clean worktree at this branch's base `5589839`
(before), once from this worktree (after). Both exit 0 with zero gateway
failures.

**Harness validation.** The *before* run reproduces the CORPUS-REPIN artifacts
exactly where analysis is concerned: `diagnostics.json` and
`chunk-contracts.json` are **byte-identical** to
`docs/reports/corpus-repin/artifacts/`, and `monkey-source.json` is identical
after normalising the absolute checkout path. Only `source-effects.json` and
`contracts.json` differ, in the `build_digest`-derived identity fields and the
embedded contract source paths — the same location-provenance class CORPUS-REPIN
itself recorded, present *before* any change of mine. That is what makes the
before/after diff below an isolation of this card and not of the environment.

**Headline, pinned route, 185 units: 2 Safe / 42 provisional Unsafe /
137 Unknown / 4 data-only.** Identical to the archived pin, in both directions.

| Artifact | before → after |
|---|---|
| `chunk-contracts.json` | **byte-identical** (96 results, 3 checked / 93 unknown; 21 backend-teardown rows unchanged) |
| `monkey-source.json` | **byte-identical** (3 callbacks, 15 wrappers unchanged) |
| `diagnostics.json` | **4 of 185 units changed**, no unit verdict change |
| `source-effects.json` | 3 of 96 rows changed in `build_digest`-derived identity fields only; status unchanged at 3 checked / 93 unknown — **location provenance, measured identical in class to the before-vs-archived delta** |
| `contracts.json` | embedded contract source paths only |

Every changed leaf class in `diagnostics.json`:

| Library / unit | Function | Status | Change |
|---|---|---|---|
| `chunkio/cio_chunk.c` | `cio_chunk_is_file` | Unknown → **Unknown** | `type = ch->st->type;` is now modeled, so one `unsupported: indirect memory access…` (line 2370) becomes one `unknown_alias: unresolved resource identity \`type\`` on the following branch (line 2371) |
| `cmetrics/cmt_cat.c` | `cat_histogram_values` | Unknown → **Unknown** | two `unsupported: indirect memory access…` removed |
| `cmetrics/cmt_cat.c` | `cmt_cat_histogram` | Unknown → **Unknown** | one removed |
| `cmetrics/cmt_decode_msgpack.c` | `unpack_meta_opts` | Unknown → **Unknown** | one removed |
| `cmetrics/cmt_encode_opentelemetry_batch.c` | `batch_add_scope` | Unsafe → **Unsafe** | one removed |

Corpus-wide diagnostic totals: `leak` **271 → 271** (unchanged),
`unknown_alias` 4,166 → 4,167 (**+1**), `unsupported` 48,318 → 48,312 (**−6**).

Three facts the integrator needs, each measured exhaustively rather than argued:

* **No newly surfaced provisional `Unsafe` unit.** The unit status histogram is
  identical and no unit row changed `status`.
* **No function-level `Unknown` → `Safe` movement.** All 4,688 projected
  functions were compared pairwise: `status moves: none`. This card removes six
  refusals but promotes nothing, because every affected function has other
  blockers — which is the expected shape for a *read* admission.
* **The one added diagnostic is in the fail-closed direction.** `cio_chunk_is_file`
  reads `type = ch->st->type;`, whose final member is a *scalar*. The chain rule
  admits it as an opaque field read, so the local `type` binds
  `LegacyInstruction::OpaqueValue` — an unknown, possibly-null value — and the
  branch that follows now *reports* the unresolved identity instead of the
  statement being refused wholesale. The function stays `Unknown` either way.
  This is conservative rather than precise: for a scalar final member the
  opaque binding is stronger than C02's scalar no-op would be, and it can only
  add `Unknown`, never remove one. Deferring a scalar-terminated chain to C02's
  no-op classification is a precision follow-on, recorded in §7, and was
  deliberately not taken here because it would widen the corpus delta beyond
  this card's property.

The four produced artifacts, their digests, and three machine-readable diff
documents (before→after, CORPUS-REPIN→before, CORPUS-REPIN→after, all generated
with `docs/reports/corpus-repin/diff_artifacts.py --full`) are archived under
[`artifacts/`](artifacts/) with per-file digests in
[`artifacts/manifest.json`](artifacts/manifest.json). **The re-pin decision is
the integrator's**: this card archives its measurement and does not overwrite
`docs/reports/corpus-repin/artifacts/`.

### 5.2 The canonical CMetrics audit — byte-identical

Run in **this** worktree twice, with only the two production files reverted to
`5589839` between runs, so the comparison carries no path provenance at all:

```
cmetrics/lib/cmetrics/src/cmt_counter.c: Unsafe (7/7)
  resource diagnostics: leak=2, unknown_alias=2, unsupported=35
source contracts: 1 checked function contract(s) / 6 Unknown; no whole-unit promotion
1 units: 0 Safe / 1 provisional Unsafe / 0 Unknown / 0 data-only / 0 gateway failures
```

[`cmetrics-baseline.json`](cmetrics-baseline.json) and
[`cmetrics-after.json`](cmetrics-after.json) are **byte-identical**
(`82d6bbc9ae087450ecfe259f89de95b19c959aba3b01a9bda4474faf500f6c2d`). The raw
unit stays **provisional Unsafe**, unpromoted, with no per-function delta to
explain.

---

## 6. Lean parity

`proof/QCore/ChainedAccess.lean`, **23 theorems**, registered as the new
`proof/AuditChainedAccess.lean` and checked by `check_lean_audits.py`
(38 audits → **39**). Only `propext` and `Quot.sound`; no new axiom, no
placeholder, no `sorry`.

* **Structural refusal** — `an_empty_chain_never_admits`,
  `unread_declarations_never_admit_a_chain`,
  `a_mismatched_base_never_admits_a_chain`,
  `a_bound_root_never_admits_a_chain` (C02-P premise 4 unchanged at every
  depth), `a_continued_access_never_admits`,
  `a_chain_beyond_the_bound_never_admits` (refused, not truncated).
* **One untyped link refuses the whole chain, wherever it sits** —
  `linksOk_false_of_bad_link` by induction over the link list, with
  `a_union_link_never_admits`, `an_incomplete_link_never_admits`,
  `an_ambiguous_link_never_admits`, `a_qualified_link_never_admits` and
  `an_aggregate_link_never_admits` as its corollaries.
* **The clause the chain adds** — `linksOk_false_of_unContinuable`,
  `a_non_continuable_intermediate_never_admits` and
  `a_scalar_intermediate_never_admits`: a member that would be a perfectly good
  *final* selection still refuses as an intermediate one.
* **Extension, not replacement** — `a_depth_one_chain_is_the_direct_admission`
  proves the depth-one rule is *exactly* `ExitResiduals.admitsActual`, so
  nothing C02-P refused becomes admitted by accident. Non-vacuity in both
  directions: `the_chain_rule_is_not_vacuous`,
  `one_union_link_flips_the_admitted_chain`, and `the_bound_is_exactly_four`
  (four links admitted, five refused).
* **Fail-closed bridging** — `a_refused_chain_refuses_the_statement`,
  `a_refused_chain_is_never_safe`, and
  `an_admitted_chain_does_not_rescue_a_statement`: any other indirect access in
  the same statement still refuses it, however many chains were admitted.

The model is of the **decisions**, not of C semantics. Declared types, record
facts, parameter modes and base-binding flags are premises the reader and the
transfer function supply; nothing proves either extracts them faithfully, and
nothing here is a claim about arbitrary C memory safety.

---

## 7. Open boundaries

Carried forward from O04 unchanged: blockers **B02–B11**, and the recorded
precision warning about `cio_scan_stream_files` — which this card's
re-measurement now settles negatively: that unit's status is unchanged and no
newly surfaced provisional `Unsafe` unit exists.

New or narrowed by this card:

| # | Boundary | Status |
|---|---|---|
| **B08** | pointer-field custody **transfer** (`ic->chunk = chunk`) | **open** — the binding constraint on C5; the admission covers reads only |
| — | aggregate **branch conditions** (`if (ic->routes_mask == NULL)`) | **open**, C02 §7's own increment |
| **O04-ARMS-VARIADIC** | no variadic-tail form in the contract vocabulary; `flb_error` is declared at exactly the arity of its one call site, and any other arity refuses | **open**, deliberately not solved with a `qcore-schemas` change |
| **C02-P-ALIAS** (widened scope, unchanged premise) | the chain's intermediate places are unrelated to any local this function may have released; contained because releasing an opaque-bound name reports `UnknownAlias`, pinned by test | **recorded**, not repaired |
| **O04-ARMS-SCALAR-CHAIN** | a chain whose final member is a scalar binds an `OpaqueValue` rather than deferring to C02's scalar no-op; conservative, and measured to cost one added `unknown_alias` corpus-wide | **open**, precision only |
| — | `untracked_bases` stays flow-insensitive | **recorded** |
| — | chains deeper than four selections; `.`-continuation into nested aggregates; typedef'd link types the reader will not resolve to a `struct` tag | **refused by construction** |

---

## 8. Scope and non-claims

- **No raw unit is promoted.** Both functions on the path keep status
  **Unknown**; the CMetrics unit remains **provisional Unsafe**, unpromoted; the
  corpus aggregate is unchanged at 2 Safe / 42 provisional Unsafe /
  137 Unknown / 4 data-only.
- A per-exit **Discharged** is a claim about *this projection under the named
  premises of §2*, not a proof that the C is correct. Every declared contract it
  rests on is a trusted assumption.
- A provisional `Unsafe` is never presented as an upstream defect; the `h_*`
  family is synthetic.
- **Unknown means Unknown.** An exit whose path carried a refusal alleges
  nothing in either direction.
- Projection coverage is not safety coverage; unknowns stay in the denominator.
- CT coverage is unchanged at 24/50; the contract corpus remains 60 modules;
  `qcore-schemas` is unchanged at 6.0.0; no plugin, protocol or evidence version
  moved.

---

## 9. Files changed

| Path | Change |
|---|---|
| `crates/qcore-c-gateway/src/typed_field_access.rs` | `record_pointer_fields` member map and `record_pointer_field()`; `read_record_body` returns a `RecordMembers` record |
| `crates/qcore-c-gateway/src/borrowed_actual.rs` | the bounded chain walk in `admitted_access`, `ACCESS_CHAIN_LIMIT`, restated premise (3), eight new unit tests, one renamed pin |
| `crates/qcore-c-gateway/tests/o04_arms.rs` | new — 13 assertions plus one `#[ignore]`d reporting aid |
| `crates/qcore-c-gateway/tests/support/digest.rs` | new — the local SHA-256, factored out of `o04_constructor_caller.rs` and shared |
| `crates/qcore-c-gateway/tests/fixtures/o04_arms/` | new — two verbatim upstream regions, `provenance.json`, `.gitattributes` |
| `crates/qcore-c-gateway/tests/fixtures/contracts/o04_arms_{callees,generic}.qcore` | new — the two added premises and the held-out `h_*` API |
| `crates/qcore-c-gateway/tests/borrowed_field_actuals.rs` | one pinned row updated deliberately (§4.3) |
| `crates/qcore-c-gateway/tests/o04_constructor_caller.rs` | one pinned row updated deliberately (§4.3); digest helper shared |
| `proof/QCore/ChainedAccess.lean`, `proof/QCore.lean`, `proof/AuditChainedAccess.lean` | new module and its axiom audit |
| `docs/reports/remaining-o04-arms/` | this record, the arm ledger, the two CMetrics reports, the re-measured artifacts and three diff documents |

No file owned by the integrator was touched: `AGENTS.md`, `CLAUDE.md`,
`README.md`, `DEVELOPMENT.md`, `docs/reevaluation/ct_manifest.json` and
`docs/reviews/r0_3/acceptance_matrix.json` are unchanged, as is
`contract/qcore/` and every archived predecessor report.

---

## 10. Gate results

Run in this worktree at branch `worktree-agent-aa36c4a06706419d2`, base
`5589839`, with the Libevent submodule initialised.

| Gate | Command | Observed | Exit |
|---|---|---|---|
| Workspace tests | `cargo test --workspace --no-fail-fast` | **1,673 passed / 0 failed / 2 ignored** | 0 |
| Format | `cargo fmt --all -- --check` | clean | 0 |
| Clippy | `cargo clippy --all-targets -- -D warnings` | clean | 0 |
| Probe | `cargo run -p qcore-compiler --example reevaluation_probe` | ran | 0 |
| Follow-up | `cargo run -p qcore-compiler --example reevaluation_followup` | ran | 0 |
| Python | `python3 -B -m unittest discover -s scripts/tests` | **108 ran, OK** | 0 |
| Lean build | `cd proof && lake build` | 85 jobs, completed | 0 |
| Lean audits | `python3 -B scripts/check_lean_audits.py` | **39 audits passed** | 0 |
| Provider tests | `cargo test --locked --offline --manifest-path providers/Cargo.toml --workspace` | **58 passed / 0 failed** | 0 |
| Provider clippy | `cargo clippy --locked --offline --manifest-path providers/Cargo.toml --all-targets -- -D warnings` | clean | 0 |
| Provider fmt | `cargo fmt --manifest-path providers/Cargo.toml --all -- --check` | **pre-existing diff** (§10.1) | 1 |
| 185-unit corpus | `cycle19_fluent_bit_libraries_audit --pinned-evidence`, before and after | §5.1 | 0 / 0 |
| CMetrics audit | `cargo run -p qcore-audit -- contract/profiles/cmetrics-counter/audit.json`, before and after | byte-identical (§5.2) | 0 / 0 |

The card contributes 21 new assertions (13 integration, 8 unit) plus one
`#[ignore]`d reporting aid.

### 10.1 One pre-existing gate finding, not repaired

`cargo fmt --manifest-path providers/Cargo.toml --all -- --check` reports a
diff in `providers/conformance/tests/packages.rs:239` — a line-length wrap in a
`verify_executable_package` call. `git status` shows that file **unmodified** on
this branch, so the finding predates this card and belongs to whichever change
last touched that line. It is reported rather than fixed: `providers/` is
outside this card's scope, and an unrelated edit there risks colliding with
another lane. The provider *tests* and *clippy* both pass.

### 10.2 One measurement not repeated

The baseline workspace test tally was not re-measured: the second checkout's
`target/` exhausted the scratch filesystem quota mid-run and was deleted to
recover. The recorded wave-6 baseline is 1,651 passed / 1 ignored at a
checkpoint three commits earlier than this branch's base, so the +22 observed
here is this card's 21 assertions and one ignored aid plus at least one test
landed by the intervening commits; the exact split is not claimed.
