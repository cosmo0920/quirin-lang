# W44-INDIRECT-CHAIN-ACCESS: typed record-pointer member chains

Card W44-INDIRECT-CHAIN-ACCESS, 2026-09-25, on base `c1e69bb5` (local `main`,
"Open the wave-44 increment record").

W43's typed scalar access admits one member selection per access, and so does
W43-INDEX-TRAVERSAL's typed condition read. This card lets both routes read and
write through a **member chain** `R->L1->…->F`, `(*R).L1->…->F` or
`S.L1->…->F`. Every pointer the chain reads through gets its own obligation.

The forecast was frozen before the rule was wired into the classifier
([PREDICTION.md](PREDICTION.md), commit `cd1a08e9`). The census instrumentation
is commit `5365dd94` and the admission is commit `9243680e`.

## Headline

* **Census first** ([§1](#1-census)). W43's own classifiers, re-run on this
  base, reproduce both ledger sizes:
  * statements: 5,565 leaves / 2,049 functions / 212 sole;
  * conditions: 1,248 / 632 / 1.

  The chain census splits those 6,813 leaves by the rule's own recogniser.
  **3,564 of them carry no member chain at all.** The causes:
  * W43's place reader classes a typed declarator `T * x = e` as a chain;
  * its condition class counts two separate arrows as one chain.

  Of the 3,249 real chains, the dominant refusals are:
  * nested aggregate sub-objects (`a->b.c`), 1,287 leaves;
  * typedef'd roots, 580;
  * undeclared roots, 396;
  * calls, 383;
  * subscripts, 296;
  * incomplete records, 195.

  The rule the brief specifies admits **74 leaves**.
* **The rule** ([§2](#2-the-rule)). A chain is two to four selections. Every
  link is exactly `struct POINTEE *` over a complete, unambiguous, non-`union`
  record, and the final member is scalar. The chain rides on W43's
  `IndirectScalarAccess` as a `chains` field, so there is no new
  `LegacyInstruction` variant. Every pointer on the chain is read as an
  untracked or a tracked owner:
  * **root**: a never-bound name (C02-P premise 4) that is unbound or non-null
    is untracked. Exactly one known, non-null, live identity is tracked.
  * **link of an untracked owner**: untracked (C02-P premise 3).
  * **link of a tracked owner**: must be the member place an admitted aggregate
    store bound to one live child, equal to its field link. A place bound to no
    target and no link makes the link untracked.
  * **tracked owner past the first link**: refuses.

  Anything else records `UnknownAlias` on the spelled prefix. The chain changes
  no binding.
* **Lean.** `QCore.IndirectChainAccess` has 36 theorems, audited by
  `AuditIndirectChainAccess.lean`, with no placeholders and no project axioms.
  It builds on `QCore.IndirectScalarAccess` (the transfer) and
  `QCore.ChainedAccess` (O04-ARMS's chain typing).
* **Arbiter** ([§4](#4-arbiter)).
  * BEFORE reproduces the combined baseline's diagnostics byte for byte. The
    census run reproduces all five BEFORE artifacts.
  * AFTER removes **74** leaves and adds **49**, every one attributed
    (`attribution_complete: true`).
* **Prediction against outcome.** Every forecast figure is hit:
  * 74 leaves leave their families: 24 are removed, and 50 are exchanged for
    49 `unknown_alias` leaves;
  * 0 downstream;
  * **0 function moves, 0 unit moves**;
  * **leak 271** unchanged;
  * **0 violation candidates**.
* **Ledger: 345 → 352.** Both entries are retired against their own
  discharge conditions. Nine measured residuals are registered
  ([§6](#6-ledger)).

| Measure | BEFORE (combined baseline) | AFTER | Delta |
|---|---:|---:|---:|
| Functions Safe / provisional Unsafe / Unknown | 294 / 85 / 4,455 | **294 / 85 / 4,455** | 0 |
| Units Safe / provisional Unsafe / Unknown / data-only | 3 / 43 / 135 / 4 | 3 / 43 / 135 / 4 | 0 |
| `unsupported` | 45,848 | **45,774** | −74 |
| `unknown_alias` | 4,358 | **4,407** | +49 |
| `leak` / `summary_conflict` / violations | 271 / 1 / 0 | 271 / 1 / 0 | 0 |
| indirect memory access: leaves / Unknown functions / sole | 17,281 / 3,289 / 406 | 17,236 / 3,278 / 408 | −45 / −11 / +2 |
| branch condition | 8,287 / 2,320 / 49 | 8,266 / 2,316 / 50 | −21 / −4 / +1 |
| unresolved resource identity | 4,136 / 1,237 / 3 | 4,180 / 1,256 / 3 | +44 / +19 / 0 |
| W43 statement class `chain` | 5,565 / 2,049 / 212 | 5,522 / 2,028 / 213 | −43 / −21 / +1 |
| W43 condition class `chain` | 1,248 / 632 / 1 | 1,227 / 622 / 1 | −21 / −10 / 0 |

The family rows come from [blocker-census.txt](blocker-census.txt) and the
same script over the baseline. The class rows come from
[census/](census/). A sole-blocked count that rises belongs to a function
whose other families cleared.

## 1. Census

### 1.1 Instrumentation (commit `5365dd94`)

The `indirect-census` feature gains a **member-chain stream**,
`QCORE_CHAIN_CENSUS` (`crates/qcore-c-gateway/src/indirect_chain_census.rs`).
It records every refused statement, `for` clause and controlling expression that
selects a member, with:

* the answer of the recogniser the rule calls (`chain_statement` or
  `chain_condition`);
* whether the typed route is consulted for that refusal;
* every chain-shaped access the chain reader met, with its answer or refusal
  reason, its depth, and its root's kind, declared type, never-bound and
  may-bind bits and final member.

The reader, the walker extension and the inert `chains` field were committed
with the census, and no route called them. The census run reproduces **all
five** BEFORE artifacts byte for byte ([census-identity.json](census-identity.json)).
The census of the AFTER tree reproduces AFTER's five artifacts
([census-after-identity.json](census-after-identity.json)). The streams are not
committed; their SHA-256 prefixes are chain `c7db6faf…` (base) and `53c92ff0…`
(after).

### 1.2 The split

[`scripts/chain_census.py`](scripts/chain_census.py) selects the family with
W43's own classifiers: `residuals.py` class `chain` for statements, and
`condition_class` `chain` for conditions. It then splits each leaf by the
hardest chain-census record at its key ([census/chain-census.txt](census/chain-census.txt)).
Only an access path of two or more selections counts as a chain, and a
subscript counts as a selection, as W43's class definition does.

| Reader outcome | Leaves | Functions | Sole |
|---|---:|---:|---:|
| not a member chain (declarator, single access, separate arrows, no record) | 3,564 | 1,534 | 155 |
| `dot-link`: a nested aggregate `a->b.c` | 1,287 | 517 | 35 |
| `root-untyped`: root declared through a typedef (`rd_kafka_buf_t *`) | 580 | 300 | 17 |
| `root-undeclared` | 396 | 122 | 4 |
| `call` in the statement | 383 | 253 | 13 |
| `subscript` on the path | 296 | 136 | 6 |
| `incomplete-record` (incomplete, ambiguous or union) | 195 | 151 | 10 |
| **`admitted`** | **64** | **50** | 0 |
| pointer, array or aggregate final member; `->` after a non-pointer | 48 | 33 | 1 |

The real chains split by depth as follows:

* depth 2 (member selections): 1,883 leaves;
* depth 3: 825;
* depth 4: 123;
* depth 5 or more: 33;
* depth 1 (a subscript followed by one member): 385.

By statement kind, counting every family leaf with a census record, whether
or not it has a chain, the leaves are reads (1,891), `if` conditions (1,159),
stores (1,037), declarations (778), increments (161), `for` conditions (69)
and compound stores (33). The chain roots are mostly formals and locals the
body may bind (premise 4's syntactic reading). The same split, grouped into
the residuals of §6, is in
[census/residual-groups-at-base.txt](census/residual-groups-at-base.txt).

| Root | Leaves |
|---|---:|
| `formal:may-bind` | 1,245 |
| `local:may-bind` | 833 |
| `formal:never-bound` | 580 |
| `local:never-bound` | 157 |

### 1.3 Forecast

Across every status, the recogniser admits **74 leaves**
([census/admitted.txt](census/admitted.txt)):

* 64 in the family and 10 outside it;
* 69 in Unknown functions and 5 in provisional-Unsafe functions.

Their arms split as follows:

* **24 `clear`**: every root is never bound;
* **50 `flow`**: some root the body may bind. The flow must find one live
  identity with a bound member place, and none was expected to.

No Unknown function has every leaf `clear`, so **0 function moves** were
forecast. The frozen [PREDICTION.md](PREDICTION.md) has the complete forecast.

## 2. The rule

### 2.1 Reader (`crates/qcore-c-gateway/src/indirect_chain_access.rs`)

`parse_chain` reads a root and two to `CHAIN_SELECTION_LIMIT` (4, O04-ARMS's
bound) selections. It accepts three root forms:

* `R ->` or `( * R ) .` with `R` declared `struct T *`;
* `S .` with `S` declared `struct T`.

Every member selected and then dereferenced must satisfy
`TypedAccessModel::record_pointer_field` over a complete, unambiguous,
non-union `struct`, and the walk re-enters the model at the pointee. The final
member must satisfy `scalar_field`, and the access must not continue.

The reader refuses each of the following for a recorded reason:

* `.` after the first selection;
* a subscript;
* a call through a member;
* `void *`;
* a typedef'd or multi-star link;
* an incomplete record;
* an untyped or undeclared root;
* a chain past the bound.

`chain_statement` and `chain_condition` run W43's own walker with one opt-in
**access extension**. `typed_field_access::typed_scalar_statement_with` and
`typed_scalar_condition_with` take an `AccessExtension` that is consulted at
every operand position and for the destination. Every pre-existing entry point
passes `no_extension`, so each of their answers is unchanged. After the walker
admits, the admitted run is re-read for its chains, which can only add
obligations. Each dereferenced root joins the plan's `bases` and
`record_bases`, so it keeps:

* W43's dereference obligation;
* its C02-P-ALIAS may-alias obligation;
* its `use_after_release` accusation.

The classifier consults the route only after the typed single-selection route
returned `None`, at the same position and for the same refusals, and only for a
statement with no call and no brace (`Parser::classify_simple_unguarded`). The
condition read consults it only where W43-INDEX-TRAVERSAL's typed condition
returned `None` (`Parser::condition_access`). A statement any older route
admits keeps its exact lowering. For example, `x = a->b->c` over a never-bound
root stays O04-ARMS's opaque read, and REACH-GATE's F4 conditions stay NoOps.

### 2.2 Transfer

In `transfer`'s `IndirectScalarAccess` arm, after W43's base loop, each chain is
decided by `indirect_chain_access::chain_resolves` over a read-only `ChainView`
of the state. The state implements the view: bindings, exactly-live statuses
and field links. The obligation is the module header's four bullets.

A refused chain records `UnknownAlias`. Admitted or refused, nothing is
written: bindings, statuses, markers, custody, containment and field links are
all unchanged. The statement's scalar destination and its `*B`/`B[I]` frame are
the enclosing typed access's. A member store through a chain writes one scalar
and has no frame, as W43's member store does.

**Why an untracked link needs no nullness evidence.** Each dereferenced link
needs non-null evidence where the projection binds it: the root's binding, and
the member place under a tracked owner. A member of an object no reaching path
bound is a place this function never names. The projection binds `->` places
only under O04-B08's aggregate store, whose base is body-acquired and therefore
never a never-bound name. So such a link carries no binding at all. Its nullness
is outside the property exactly as an unbound base's is: spec L18, and W43's
scope string "never … the liveness or nullness of an object the projection does
not track". A tracked owner's member **can** be bound, so there the rule demands
the evidence, and refuses without it.

### 2.3 Composition with wave-43 machinery

* **Descriptors.** A chain adds no variant, so
  `descriptor_custody::admits_descriptor_read` decides it by W43's
  `IndirectScalarAccess` arm (refused), and a comment there says so. A tracked
  descriptor stored through a chain is refused by name (test
  `a_tracked_descriptor_stored_through_a_chain_is_refused_by_name`). An
  ordinary integer is admitted.
* **Derivations.** A tracked root requires an identity derivation. A derived or
  stepped pointer (`Stepped::Any` included) never roots a tracked chain, because
  a field of an interior pointer is not the field the link recorded.
* **`B[X++]`** is untouched: the extension parses chains only, and a subscript
  on a chain refuses.

### 2.4 Scope (invariant 2)

The scope string is unchanged, because a chain access is an admitted scalar
access through pointers, and the string already says that such an access
"decides custody only, never bounds, alignment, or the liveness or nullness of
an object the projection does not track". Spec L18 gains the rule paragraph.
The diagnostics artifact therefore moves only by its leaves.

### 2.5 Tests and pins

* **`crates/qcore-c-gateway/src/indirect_chain_access.rs`** has 12 module tests
  for the reader and the obligation.
* **`crates/qcore-c-gateway/tests/indirect_chain_access.rs`** has 14 tests,
  each asserting the complete diagnostic vector and rechecking every `Safe`
  certificate. They cover:
  * stores, compound stores, increments and depth-four chains through a
    formal;
  * reads beside `*p` and other chains;
  * conditions, and a record-object root;
  * a chain through a stored live child (Safe);
  * refusals: a released child, an unstored member of a tracked root, a
    tracked owner past the first link, a released root (refused and accused,
    Unsafe), and an unguarded nullable root;
  * a leak beside an admitted chain;
  * the refused shapes (pointer final member, nested aggregate, `void *` link,
    union pointee, past the bound);
  * the premise-4 over-approximation (`--b->…`, `(*b).…`);
  * descriptor composition.

Six existing pins move deliberately, each explained where it stands:

| Pin | Before | After, and why |
|---|---|---|
| `typed_field_access.rs` ambiguous table, `n->doc->cursor = 1` | Unknown, indirect | Safe; new test `a_typed_member_chain_store_is_admitted_by_the_chain_route` |
| `scalar_member_store.rs` `p->state = p->next->state` | Unknown, unsupported | Unknown, `unknown_alias p->next` (tracked owner, no bound member place); new test |
| `o04_b08.rs` chained destination `b->item->weight = 1` | indirect refusal | `unknown_alias b->item`; new test `a_chained_member_of_an_acquired_base_refuses_on_its_link` |
| `o04_b08.rs` chained branch `b->item->weight == 0` | branch refusal | same new test |
| `reach_gate_positions.rs` adversarial row "a base named in any call argument list" | Unknown, branch refusal | Safe through the chain route, which keeps the root's obligations; new test `a_borrowed_base_is_admitted_by_the_chain_route_with_its_obligations` with a released-root control |
| `reach_gate_positions.rs` `an_admitted_base_is_never_a_modeled_call_actual` borrowed half | Unknown | moved to that new test; REACH-GATE's own premise (`may_bind`) is unchanged |

`compiler/examples/alias_aggregate_probe.rs` matched the variant exhaustively.
It prints the chain count only when there is one, so every existing line reads
as before.

## 3. Lean parity

[`QCore.IndirectChainAccess`](../../../proof/QCore/IndirectChainAccess.lean) is
audited by [`AuditIndirectChainAccess.lean`](../../../proof/AuditIndirectChainAccess.lean).
It has 36 theorems:

* **syntax, over O04-ARMS's chain facts:**
  * `the_root_premise_is_left_to_the_flow`;
  * `a_never_bound_scalar_chain_is_an_o04_chain` (the reader is O04-ARMS's,
    narrowed);
  * `a_bad_link_refuses`, `a_non_continuable_intermediate_refuses`,
    `a_single_selection_is_not_this_rule`, `a_chain_beyond_the_bound_refuses`,
    `a_pointer_final_member_refuses`, `a_continued_access_refuses`;
* **custody neutrality and conservativity:**
  * `a_chain_changes_no_binding` (the state after is W43's transfer);
  * `field_links_are_unchanged`, `the_statuses_and_markers_are_unchanged`;
  * `with_no_chain_the_access_is_w43s`;
  * `admitted_chains_add_no_diagnostic`, `a_refused_chain_is_never_safe`;
* **the root:**
  * `an_unbound_never_bound_root_is_untracked`;
  * refusals: `an_unbound_root_the_body_may_bind_refuses`,
    `a_nullable_root_refuses`, `a_target_free_root_the_body_may_bind_refuses`,
    `a_record_object_root_the_body_may_bind_refuses`;
  * `a_tracked_root_is_one_live_identity`, `a_tracked_root_is_never_accused`;
* **the links:**
  * `an_untracked_owner_walks_untracked`,
    `every_link_of_an_untracked_root_resolves`;
  * `a_tracked_owner_past_the_first_link_refuses`,
    `a_tracked_owner_needs_a_bound_place`, `a_nullable_place_refuses`;
  * `a_tracked_link_is_the_stored_live_child`;
  * `a_drifted_link_refuses`, `a_released_child_refuses`;
* **non-vacuity:** a depth-four chain through an unbound formal and a linked
  live child are admitted. A linked released child, a tracked owner one link
  deeper and an unbound member place are refused. A full scalar chain passes
  the syntax.

The following premises are stated, not proved:

* C02-P premise 3: an untracked owner's member is untracked;
* the frame of constraint 10a;
* the Rust–Lean correspondence and C extraction premises of trust boundary 28.

A refused chain's diagnostic names its root in the model and its spelled
prefix in Rust. The abstraction is only the variable's name.

## 4. Arbiter

### 4.1 Runs

The runner is [`scripts/run_audit.py`](scripts/run_audit.py): W43-INDEX-TRAVERSAL's
runner, with `QCORE_CHAIN_CENSUS` added under `--census`. Every run uses the
same pin, Clang 21.1.8 and ChunkIO package as the baseline.

| Run | Binary (sha256) | Tree | Exit | Record |
|---|---|---|---:|---|
| BEFORE | `cdea036a…` | base `c1e69bb5` | 0 | [before-run.json](before-run.json), [before-identity.json](before-identity.json) |
| CENSUS | `a05e510a…` | `5365dd94` + feature | 0 | [census-run.json](census-run.json), [census-identity.json](census-identity.json) |
| AFTER | see [after-run.json](after-run.json) | `9243680e` | 0 | [after-run.json](after-run.json), [artifacts/](artifacts/) |
| CENSUS (after) | see [census-after-run.json](census-after-run.json) | `9243680e` + feature | 0 | [census-after-identity.json](census-after-identity.json) |

**BEFORE against the combined baseline** (`w43-indexed-resource-collections/integration/artifacts/`,
`baseline_identity.py --root-alias`):

* diagnostics, chunk-contracts and source are byte-identical;
* `contracts` is identical once the checkout root is aliased;
* `source-effects` carries the checkout-path digests the brief lists.

AFTER was built from `9243680e`. The later code commits do not touch the
analyzer's decisions:

* `21d76be9` changes test files only;
* `a23efc93` is rustfmt only;
* `6e0bced6` changes the aggregate probe example and adds a test.

The archived artifacts are therefore the tip's.

**AFTER against BEFORE** (same checkout, [after-vs-before.json](after-vs-before.json)):
chunk-contracts, contracts, source-effects and source are byte-identical. Only
the diagnostics move, by the leaves below. The scope string is unchanged.

### 4.2 Attribution

[`scripts/attribute.py`](scripts/attribute.py) produces
[attribution.txt](attribution.txt) and
[attribution.json.gz](attribution.json.gz), with `attribution_complete: true`.

| Class | Leaves | What it is |
|---|---:|---|
| removed: rule:clear | **24** | admitted chains whose roots are never bound: cmetrics decoders' `counter->map->metric_static_set = 0` family, `ch->ctx->page_size` in `cio_file_*`, `server->clock_context->…`, `storage->cio->…` |
| removed: rule:flow | **50** | admitted chains with a root the body may bind (`ch` ×22 in chunkio, `decode_context` ×15, `server` ×5, `in` ×2, `batch` ×2, `histogram` ×2, `chunk`, `state`) |
| added: exchange:flow | **49** | `unresolved resource identity` on the root at each `flow` coordinate; two leaves at `mk_lib_worker:6520` (branch and `for`) share one |
| removed/added: downstream | 0 | no binding-precision move |
| added: violation | **0** | |

Every `clear` leaf was removed, and no `clear` coordinate was exchanged.

**`POISON-UNMASK-ON-ADMISSION`.** It is measured for this admission and did
not fire. No removal came from binding precision or from an edge becoming
infeasible.

### 4.3 Movers

None. Of the 74 leaves, 69 sit in Unknown functions and 5 in provisional-Unsafe
ones. No function had these leaves as its last blockers. The census forecast 0,
and 0 moved.

### 4.4 Prediction against outcome

| Forecast (frozen) | Measured | |
|---|---|---|
| 74 leaves leave their families (indirect 46, conditions 26, increment 2) | 74, in exactly that split | hit |
| 24 `clear` removed, 50 `flow` exchanged | 24 removed, 50 exchanged | hit |
| exchanges ≤ 74, expected 50 | 49 (one shared line) | hit |
| 0 Unknown → Safe, 0 unit moves | 0, 0 | hit |
| leak 271 | 271 | hit |
| 0 violation candidates | 0 | hit |
| downstream ≤ 10 | 0 | hit |
| `unsupported` 45,774, `unknown_alias` ≤ 4,408 | 45,774, 4,407 | hit |

## 5. Validation

Gate figures are in [gate.json](gate.json) and [gate-facts.json](gate-facts.json),
measured by [`scripts/run_gate.py`](scripts/run_gate.py) in this worktree with
`CARGO_INCREMENTAL=0`. Logs are worktree-private under `private/gate2/` and
`private/provider-gate/`. The [Clang recording](clang-version.txt) was taken in
this checkout. The results table is in [§5.1](#51-results).

A first gate run (`private/gate/`) is not evidence. `cargo test` and
`cargo clippy` failed to compile `compiler/examples/alias_aggregate_probe.rs`,
which matched `IndirectScalarAccess` without the new field. The probe was
fixed in `6e0bced6` (§2.5), and the gate was rerun from that commit.

### 5.1 Results

| Command | Exit | Result |
|---|---:|---|
| `git submodule update --init --recursive` | 0 | libevent `d82464a2…` |
| `clang --version` | 0 | Ubuntu clang 21.1.8 |
| `cargo test --workspace --no-fail-fast` | 0 | **5,420 passed / 0 failed / 11 ignored**, 288 summaries |
| `cargo fmt --all -- --check` | 0 | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | |
| `python3 scripts/check_lean_audits.py --jobs 4` | 0 | **178** audits |
| `cargo clippy -p qcore-c-gateway --features indirect-census --all-targets -- -D warnings` and `cargo test -p qcore-c-gateway --features indirect-census --lib` | 0 / 0 | clean; 462 passed |
| `python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/w44-indirect-chain-access/gate.json` | 0 | 352 follow-ups, 111 contract modules, 178 Lean audits |
| `python3 -m unittest discover -s scripts/tests` (after the final regeneration) | 0 | **330** tests |
| `python3 scripts/build_docs_index.py`, then `--check` | 0 / 0 | regenerated and matching |
| `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline --no-fail-fast` | 0 | **124 passed / 0 failed / 0 ignored**, 39 summaries |
| `… -p qcore-provider-conformance --test packages real_packages_preserve_unknown_and_reject_mutations … -- --exact --test-threads=1` | 0 | 1 passed (isolated, constraint 16) |
| `cargo fmt --manifest-path providers/Cargo.toml --all -- --check` | 0 | clean |
| `cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings` | 0 | clean |
| `python3 providers/contracts.py check` | 0 | |
| `python3 providers/package.py build --output private/provider-gate/packages` | 0 | 5 packages |
| `python3 providers/package.py verify …/<id>` (×5) | 0 ×5 | chunkio, cmetrics, ctraces, libevent, librdkafka |
| `python3 -m unittest discover -s providers/tests` | 0 | 13 tests |

**Counts.** The integrator measured 5,390 core tests at the base's merge. This
run has 5,420, which is +30: exactly the tests this card adds.

* 12 module tests;
* the 14-test `indirect_chain_access.rs` binary;
* four new tests beside the six moved pins.

The base was not remeasured here. So the summary count (288, against the
integrator's 289) is not compared, and the delta is stated by test, not by
subtraction. After the test step was recorded, `cargo clean --profile dev`
removed 16.4 GiB.

## 6. Ledger

**Retired** ([retired-followups.json](retired-followups.json)), each against its
own discharge condition:

* `W43-INDIRECT-CHAIN-ACCESS`. The condition was "a card that extends the route
  to record-pointer member chains with each intermediate dereference checked
  against field links, Lean parity and a measured re-pin". This card is that
  card.
* `W43-INDEX-TRAVERSAL-CONDITION-CHAIN`. The condition was "the same card …
  extends the condition read, measured together". Both routes are extended
  and measured together.

**Registered** ([followups.json](followups.json)), sized at the AFTER census
([residuals.txt](residuals.txt)) as leaves / functions / sole. Together they
cover every leaf the two retired classes still hold.

| Id | Size | Statement / condition leaves | What stays refused |
|---|---|---|---|
| `W44-CHAIN-NESTED-AGGREGATE` | 1,287 / 517 / 35 | 776 / 511 | `a->b.c`, `s.a.b` (a record-valued member) |
| `W44-CHAIN-TYPEDEF-ROOT` | 580 / 300 / 17 | 482 / 98 | a root declared through a record typedef |
| `W44-CHAIN-UNDECLARED-ROOT` | 396 / 122 / 4 | 272 / 124 | file-scope roots; unread declarations |
| `W44-CHAIN-SUBSCRIPT` | 296 / 136 / 6 | 163 / 133 | `a[i].f`, `a->b[i]` |
| `W44-CHAIN-INCOMPLETE-RECORD` | 195 / 151 / 10 | 111 / 84 | incomplete, ambiguous or union records |
| `W44-CHAIN-CALL` | 383 / 253 / 13 | 380 / 3 | a chain in a call's arguments, or a call through a member |
| `W44-CHAIN-NON-SCALAR-MEMBER` | 48 / 33 / 1 | 23 / 25 | pointer, array or aggregate final member |
| `W44-CHAIN-BOUND-ROOT` | 49 leaves / 35 functions | — | admitted chains whose may-bind root the flow leaves unbound (the exchanges) |
| `W44-CHAIN-CLASS-NOT-A-CHAIN` | 3,564 / 1,534 / 156 | 3,315 / 249 | leaves W43's classes counted that carry no member chain: a measurement correction |

**Measured boundaries.** These are noted here and not registered:

* **The tracked-owner arm is reached by no corpus leaf.** Every admitted corpus
  chain's root was untracked or refused, so the arm (a bound member place with
  its live child) is exercised by fixtures only:
  `a_chain_through_a_stored_live_child_certifies` and its three refusals.
* **The root's accusation is W43's own arm.** It is unexercised in the corpus,
  as `W43-INDIRECT-ACCUSING-UNEXERCISED-IN-CORPUS` already records.
* **`POISON-UNMASK-ON-ADMISSION` stays registered.** It did not fire here.

Follow-ups go from **345 to 352**. `corpus.cycle19_raw_units` points at this
report's [artifacts/](artifacts/). Its previous contents are in
[previous-corpus-row.json](previous-corpus-row.json).

## 7. Attempts that are not the evidence

* The first gate run (§5).
* Earlier `chain_census.py` passes. They counted every non-`not-a-chain`
  reader answer as a chain, depth-1 accesses with an untyped root included,
  and so inflated the family. The script now reads access paths from the
  tokens and counts only two or more selections. [PREDICTION.md](PREDICTION.md)
  was frozen after that correction.
* The first run of the new test binary failed two bodies, both explained by
  C02-P premise 4's syntactic reading. It reads `--b->…` as a decrement of
  `b`, and a `(*b).` root as a protected-formal use. Both are now pinned as
  refusals (`a_prefix_decrement_marks_its_root_as_bound_and_refuses`).

## 8. For the integrator

* **Proposed gateway amendment**, a constraint beside the W43 proposals (5h,
  5i), in these words:

  > W44-INDIRECT-CHAIN-ACCESS: where the typed scalar statement route or the
  > typed condition read refuses, a member chain `R->L1->…->F`, `(*R).L1->…->F`
  > or `S.L1->…->F` of two to four selections, every link exactly
  > `struct POINTEE *` over a complete, unambiguous, non-union record and a
  > scalar final member, is admitted as an access of W43's `IndirectScalarAccess`
  > (a `chains` field, no new variant). A dereferenced root is also a base.
  > Each pointer the chain reads through keeps an obligation. The root must be
  > a never-bound name (C02-P premise 4) that is unbound or non-null (it is then
  > untracked), or one known, non-null, live identity (tracked). A link of an
  > untracked owner is untracked. A link of a tracked owner must be the member
  > place an admitted store bound to one live identity equal to its field link,
  > or to no target with no field link. A tracked owner past the first link,
  > and everything else, refuses with `UnknownAlias`. The chain changes no
  > binding, and a pointer-valued final member is not admitted. Custody only:
  > no bounds claim.

* **Constraint 7:** the leak pin is unchanged at 271.
* **Composition with W44-INDIRECT-CALL-ARGUMENT-ACCESS.** That card extends the
  chain reader for call arguments. This card's reader is `parse_chain`, and
  its walker hook is the opt-in `AccessExtension` of `typed_field_access`.
  A call-argument route that reuses `typed_scalar_statement` or
  `typed_scalar_condition` without the extension is unaffected. A route that
  wants chains must pass `chain_statement`'s extension **and** carry the
  plan's `chains`. Dropping them would admit a chain without its obligations.
* **Files likely to conflict:**
  * `crates/qcore-c-gateway/src/legacy_c.rs`: the `IndirectScalarAccess`
    variant (`chains`), its three constructors, `chain_access_instruction`,
    the hooks in `classify_simple_unguarded` and `condition_access`, the
    transfer arm, `impl ChainView for AbstractState`, and the two census hooks;
  * `crates/qcore-c-gateway/src/typed_field_access.rs`: the walker's
    `extension` parameter and the `_with` entry points;
  * `crates/qcore-c-gateway/src/legacy_descriptor_custody.rs` (a comment);
  * `crates/qcore-c-gateway/src/lib.rs`;
  * `compiler/examples/alias_aggregate_probe.rs`;
  * four test files with deliberate pin moves (§2.5);
  * `proof/QCore.lean`, `docs/QCORE_SPEC_r0_3.md` L18;
  * the shared records: `docs/reviews/r0_3/acceptance_matrix.json`,
    `docs/STATUS.md`, `docs/development/increments/wave-44.md` and
    `docs/reports/INDEX.md`.
* **Artifacts.** The `contracts` and `source-effects` artifacts embed this
  worktree's path, as earlier isolated cards' artifacts did.

## Reproduction

```sh
python3 docs/reports/w44-indirect-chain-access/scripts/run_audit.py <before> c1e69bb5 --binary <base binary>
python3 docs/reports/w44-indirect-chain-access/scripts/run_audit.py <census> 5365dd94+census --census --binary <census binary>
python3 docs/reports/w44-indirect-chain-access/scripts/same_checkout_identity.py <before> <census> census-identity.json
python3 docs/reports/w44-indirect-chain-access/scripts/chain_census.py <census> chain-census.json chain-census.txt
python3 docs/reports/w44-indirect-chain-access/scripts/admitted_records.py <census> chain-census.json admitted.txt
python3 docs/reports/w44-indirect-chain-access/scripts/run_audit.py <after> 9243680e --binary <after binary>
python3 docs/reports/w44-indirect-chain-access/scripts/attribute.py <before> <after> chain-census.json attribution.json attribution.txt
python3 docs/reports/w44-indirect-chain-access/scripts/run_audit.py <census-after> 9243680e+census --census --binary <after census binary>
python3 docs/reports/w44-indirect-chain-access/scripts/chain_census.py <census-after> chain-census-after.json chain-census-after.txt
python3 docs/reports/w44-indirect-chain-access/scripts/residuals.py chain-census-after.json attribution.json residuals.json residuals.txt
python3 docs/reports/w43-libc-contracts/scripts/archive.py <after> docs/reports/w44-indirect-chain-access/artifacts
python3 docs/plans/cycle19-custody-cards/blocker_census.py docs/reports/w44-indirect-chain-access/artifacts
python3 docs/reports/w44-indirect-chain-access/scripts/update_ledger.py
python3 docs/reports/w44-indirect-chain-access/scripts/run_gate.py <gate-dir> submodules clang test fmt clippy probe followup lean census
```
