# W44-INDIRECT-CALL-ARGUMENT-ACCESS: typed indirect reads in call actuals

Card W44-INDIRECT-CALL-ARGUMENT-ACCESS, 2026-09-25, on base `c1e69bb5` (main,
"Open the wave-44 increment record"), then merged with main `70ce6b44`
(merge `9c8b7809`, [§8](#8-the-merge-of-main-70ce6b44)). It narrows
`W43-INDIRECT-CALL-ARGUMENT-ACCESS`, the largest indirect-access subfamily:
`f(p->x)`, `f(a[i])`, `f(*p)`, `strdup(src[i])`, `f(s.f)`. The census came
first; the forecast was frozen before the admission was committed or measured
([PREDICTION.md](PREDICTION.md), commit `83d05af6`); the admission is
`9ccd035d`, corrected by `634bd6bd` (§4.1).

**The current measurement is the combined tree's (§8).** Sections 1–4 are the
card-base measurement on `c1e69bb5`; their evidence is under
[pre-merge/](pre-merge/), and §4's figures are superseded by §8's.

## Headline

* **Census** ([§1](#1-census)). Of the family's 11,069 leaves at the card's
  base (2,675 Unknown functions, 288 sole-blocked; W43's classifier window
  9,522 / 2,407 / 250), **7,953 have a callee with no contract in force**:
  admitting the actual would only exchange the leaf for `call to uncontracted
  function`. Most of the contracted rest are actuals the declaration reader
  cannot type. The probe admits 173 leaves in 103 functions.
* **The rule** ([§2](#2-the-rule)). A simple statement the lexical gate refused
  for an actual is classified again with every *typed read* at a declared
  `Borrowed` position or in a declared variadic data tail admitted: a scalar
  value over typed scalar accesses, a pointer read (`B[I]`/`*B` over `T **B`,
  `B->F`/`S.F` over a pointer member), or a non-pointer sub-object address
  (`&B->F`, `&S.F`, `&B[I]`). When that is a contracted call whose destination
  is exactly its left-hand side (or an element store around one), it lowers to
  the typed read of W43-INDIRECT-ACCESS-GENERAL over every base, a second
  obligation (`ArgumentReads`), then the call. Custody-taking positions,
  uncontracted callees and descriptor reads keep the refusal byte for byte.
* **Lean.** `QCore.CallArgumentAccess`, 26 theorems, audited by
  `AuditCallArgumentAccess.lean`; no placeholders, no project axioms.
* **Arbiter on the combined tree** ([§8](#8-the-merge-of-main-70ce6b44)).
  BEFORE (main `70ce6b44`) reproduces the W44-LIBC-RESIDUALS integration
  baseline's diagnostics byte for byte. AFTER removes **195** leaves and adds
  **38**, every one attributed.
  * **1 function Unknown → Safe** (`cio_file_hash_print`, as on the card's
    base).
  * **1 function and 1 unit Unknown → provisional Unsafe**
    (`mk_rconf_read`, `mk_core/mk_rconf.c`). Its one leak is class (c), a
    premise gap already registered as `MK-LIST-ADD-BORROWED-PUBLICATION`: the
    rule now models `mk_list_add(&file->_head, &conf->includes)`, so the refused
    hand-on rule no longer withdraws it.
  * **Leak pin 19 → 20.** The card base's eleven exposed false leaks and its
    7 function and 3 unit moves to provisional Unsafe **do not occur**:
    W44-LEAK-PIN-AUDIT's refused hand-on rule withdraws all eleven.
  * **0** `use_after_release`, `invalid_release` or `double_release`.
* **The suffix-destination hole** this card found on its base is repaired on
  main by W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE; this card's fixture is
  re-pinned to that answer, and its own guard (`destination_is_exact`) is kept
  and measured redundant (a corpus run with it disabled is byte-identical).
* **Ledger 377 → 384** on the combined tree. `W43-INDIRECT-CALL-ARGUMENT-ACCESS`
  retired against its discharge condition; eight residuals registered, each
  sized; one measured sentence appended to `MK-LIST-ADD-BORROWED-PUBLICATION`
  ([§6](#6-ledger)).

| Measure | BEFORE (main `70ce6b44`) | AFTER (merge `9c8b7809`) | Delta |
|---|---:|---:|---:|
| Functions Safe / provisional Unsafe / Unknown | 300 / 9 / 4,525 | **301 / 10 / 4,523** | +1 / +1 / −2 |
| Units Safe / provisional Unsafe / Unknown / data-only | 3 / 8 / 170 / 4 | **3 / 9 / 169 / 4** | 0 / +1 / −1 / 0 |
| `leak` / `unknown_alias` / `unsupported` | 19 / 3,357 / 43,885 | **20 / 3,379 / 43,705** | +1 / +22 / −180 |
| `double_release` / `invalid_release` / `use_after_release` | 0 / 0 / 0 | 0 / 0 / 0 | 0 |
| `indirect memory access`: leaves / Unknown functions / sole | 18,101 / 3,357 / 428 | 17,916 / 3,346 / 426 | −185 / −11 / −2 |
| the family at AFTER (W43 window / widened by the call stream) | — | 9,776 / 2,442 / 252; 11,356 / 2,721 / 295 | — |

The `indirect memory access` row is [blocker-census.txt](blocker-census.txt)
and the same census of the combined baseline; the family rows count leaves in
Unknown functions only. The card-base figures (294 / 85 / 4,455 → 295 / 92 /
4,447, leak 271 → 282) are in [pre-merge/](pre-merge/) and §4.

## 1. Census

### 1.1 Instrumentation (commits `d359d3fd`, `43dd4933`, `ba3a22fa`)

The recogniser, the probe and a new census stream
(`QCORE_CALL_ARGUMENT_CENSUS`, in
[`legacy_call_argument_access.rs`](../../../crates/qcore-c-gateway/src/legacy_call_argument_access.rs))
were committed before any rule. For every simple statement the lexical gate
refused whose first token run carries a call, the stream records every actual's
position (from the callee's declaration: `Borrowed`, tail, each custody mode,
beyond, uncontracted), the recogniser's answer or refusal reason, each base's
declaration facts, and — where a candidate exists — the classifier's own
answer when the statement is classified again with the candidates admitted.
At those commits the probe's answer was recorded and discarded, every counter
it advanced restored. Every census run reproduced all five BEFORE artifacts
byte for byte ([pre-merge/run-identities.json](pre-merge/run-identities.json)).

The recogniser went through three census runs, each set out in [census/](census/):
the first ([census-1-initial-recogniser.txt](census/census-1-initial-recogniser.txt))
found member addresses and type sizes dominating the contracted positions, the
second ([census-2-addresses-and-sizes.txt](census/census-2-addresses-and-sizes.txt))
added them and found them arriving macro-expanded as `&(rkb)->lock`, the third
([census.txt](census/census.txt), commit `ba3a22fa`) reads a parenthesised
declared variable as the variable. The third is the forecast's basis.

### 1.2 The split

[census/census.txt](census/census.txt) (W43 window) and
[census/family-residuals-before.txt](census/family-residuals-before.txt)
(widened), leaves / Unknown functions / sole-blocked, widened family:

| Class | Size |
|---|---:|
| uncontracted callee | 7,953 / 2,139 / 179 |
| destination is the access (`p->f = g()`) | 704 / 437 / 35 |
| member address the reader cannot type | 503 / 200 / 28 |
| custody-taking position (typed 144, untyped 311) | 455 / 230 / 4 |
| function declarations unreadable | 290 / 108 / 3 |
| other, mostly `sizeof (*v)` | 244 / 214 / 53 |
| member chain | 189 / 111 / 3 |
| **admitted by the probe** | **173 / 103 / 7** |
| untyped base / untyped member / nested call / cast | 149 / 112 / 85 / 51 leaves |
| not a simple statement | 118 / 107 / 38 |
| probe refused (another actual, excess actuals, nested call, discarded result, formal expression) | 43 leaves |

The shape of the family: **it is mostly the call routes' and the reader's**,
not this rule's. The uncontracted class is 72% of it, and even a perfect typed
recogniser would leave those calls refused. The largest contracted refusal is
`mtx_lock(&(rkbuf->rkbuf_rkb)->rkb_logname_lock)`: a member address reached
through a chain (`rkbuf->rkbuf_rkb`), which this rule does not type; the
single-selection addresses that remain refused sit over records the declaration
reader did not complete (`&(rkb)->rkb_lock`).

**Classifier window.** W43's `residuals.py` files some call statements under
`chain`: a typed declaration `T * x = f(s.f)` above all, as
W44-INDIRECT-CHAIN-ACCESS measured. This card's call stream records 1,547 such
leaves at the base. The frozen forecast used W43's window; the residuals below
use the widened family, and both are reported (`W44-CALL-ARGUMENT-FAMILY-WINDOW`).

### 1.3 Prediction, frozen

[PREDICTION.md](PREDICTION.md) (commit `83d05af6`, on the W43 window): 144
leaves removed, at most 117 exchanges, at most 1 function Unknown → Safe
(`cio_file_hash_print`), 0 unit moves, leak 271, 0 violation candidates.

## 2. The rule

### 2.1 Admission

[`legacy_call_argument_access.rs`](../../../crates/qcore-c-gateway/src/legacy_call_argument_access.rs),
a child module of `legacy_c`, holds the whole rule. `legacy_c.rs` carries only
hooks marked with the card's name: the module declaration, one `Parser` field,
the statement walker's call, three hooks in `classify_simple_base` (the
admitted spans leave the gate, an admitted actual is not an unmodeled formal
expression, an admitted tail actual is data to the tail rule), the census
suppression during the probe, the `ArgumentReads` variant and its transfer arm.

1. **Where.** A simple statement (the statement walker only) the classifier
   refused with the lexical gate's `indirect memory access` refusal, whose
   first call's callee has a contract in force, and that reads no tracked
   descriptor name (`statement_reads_descriptor`).
2. **Which actuals.** Each whole actual of the first call that reads memory
   and that the recogniser types, at a declared `Borrowed` position or beyond
   the declared parameters of a callee with a declared variadic data tail:
   * a scalar value carrying at least one typed scalar access — the value
     walker of `typed_scalar_statement`, unchanged
     (`typed_field_access::typed_scalar_value`); a `sizeof ( TYPE )` whose
     operand names no declared variable and carries no `[` is a constant, so
     `sizeof(T) * p->n` is admitted and `sizeof(*v)` is not;
   * a pointer read: `B [ I ]` or `* B` with `B` declared `T ** B` at every
     declaration (the collections card's `pointer_element`) and `I` a scalar
     value with no access or write; `B -> F` with `B` declared `struct T *`,
     `S . F` with `S` declared `struct T`, `F` a pointer member of a complete,
     single-definition, non-union record;
   * a non-pointer sub-object address: `& B -> F`, `& S . F` with `F`
     `addressable_field` (O04-B09B10's premise 4: no field link is reachable
     through it), `& B [ I ]` over a scalar pointer or array;
   * `( NAME )` with `NAME` a declared variable reads as `NAME`.
3. **The probe.** The statement is classified again by the ordinary
   classifier with those actuals leaving the gate exactly as a C02-P borrowed
   actual does (keyed by callee, index and tokens, so the element-store probe's
   renamed statement finds them). An actual an existing route already exempts
   keeps that route's semantics and carries no new obligation. The rule
   applies only when the answer is a contracted call whose destination is
   exactly the left-hand side (a plain name or declaration, `*name`, or the
   whole spelled place), or an element store around one. Otherwise the
   original refusal is kept byte for byte and every counter restored.
4. **The lowering**, at the statement's line, in C's order (every actual is
   evaluated before the call, C11 6.5.2.2p10): the typed read
   `IndirectScalarAccess { bases, alias_reads, destination: None, frame: None }`
   over every base the used actuals dereference; `ArgumentReads { names,
   pointer_bases, pointer_places }`; the call.

### 2.2 Transfer

* The typed read is W43-INDIRECT-ACCESS-GENERAL's, unchanged: each base must
  be unbound on every reaching path, a non-null binding with no tracked
  target, or exactly one known, non-null, live identity, else `UnknownAlias`;
  a base settled on a released identity is a `use_after_release` candidate
  from the settled states (constraint 6).
* `ArgumentReads` (new, [`refused_reads`](../../../crates/qcore-c-gateway/src/legacy_call_argument_access.rs))
  records `UnknownAlias` for a name bound to an identity carried by an integer
  (`carries_integer_identity`: a tracked descriptor is custody, never data), a
  pointer-valued read through a base whose target owns a field link, a
  contained child or an indexed collection (the value read may be a tracked
  object the read does not name), and a pointer read whose own place (`p->f`)
  is bound. It changes no map.
* Both read nodes are custody-neutral, so the call sees exactly the state it
  saw before this card, and every obligation of the call is its own.

`admits_descriptor_read` refuses `ArgumentReads` explicitly (the rule never
lowers a statement reading a descriptor). No `Derivation`, stepped-pointer or
`B[X++]` rule is touched.

### 2.3 Scope (invariant 2)

The report's scope string and spec L18 gain: *a typed read handed to a
borrowed or data-tail call actual (a scalar, a pointer read, a sub-object
address) decides its base's custody only, never the liveness of an object a
read pointer names.* No bounds claim.

### 2.4 Tests and pins

[`tests/call_argument_access.rs`](../../../crates/qcore-c-gateway/tests/call_argument_access.rs),
25 tests, each asserting the complete diagnostic vector and rechecking every
`Safe` certificate: the brief's `strdup(src[i])` and its collection example
`v[i] = x_dup(src[i])` with partial cleanup (both Safe), scalar members,
reads through a live and a guarded nullable identity, tail reads, sub-object
addresses including `&(r)->lock`, a type size; the two accusations (a read and
a member address after release); the refusals (unguarded nullable base,
releasing and consuming positions, uncontracted callee, chain, pointer-member
address, `sizeof(*r)`, a linked owner, a misnamed destination); a leak the call
does not touch; the suffix-destination fixture (re-pinned at the merge to
main's repaired answer, §8); and the three leak semantics of §4.3 (re-pinned at
the merge to `Unknown` with no leak, which W44-LEAK-PIN-AUDIT's rule gives them). The module has 9 unit tests (recogniser, positions,
the second obligation over constructed states).

Nine existing pins move deliberately, each explained where it stands:

| Pin | Before | After, and why |
|---|---|---|
| `address_of_member.rs` `a_base_this_body_may_bind_is_never_admitted` | `indirect memory access` | only the uncontracted call's refusal: `&hold->lock` is admitted under `hold`'s obligation; never Safe |
| `alias_aggregate.rs` `an_opaque_member_is_never_read_back` (borrowed row) | gate refusal | `UnknownAlias b->any` (second obligation) and an unmasked leak of `it`; never Safe |
| `borrowed_field_actuals.rs` (5 rows) | gate refusal | `UnknownAlias storage` from the base's obligation; never Safe |
| `chained_tail.rs` `a_body_acquired_aggregate_base_is_refused_in_a_tail`, `storage->flag + 1` row | refused | Safe: a scalar read through a live base; the same read after release is accused (new assertion) |
| `o04_trace.rs` adversarial row 4 | refused | the live-base row admitted; the row now reads after the release, refused and accused |
| `o04_read_bound.rs` void and double pointer members; scalar member | gate refusal | `UnknownAlias b` (b owns a field link); the scalar member (`O04-READ-BOUND-SCALAR`) is admitted |
| `o04_constructor_caller.rs` consumed base | gate refusal | `UnknownAlias conf` |
| `o04_b09b10.rs` constructor, 6 indirect refusals | 6 | 3: `mk_list_add(&ch->…, &st->…)` admitted at L167, refused by `ch`'s obligation at L185/L188 |
| `neighbours.rs` before-state count | 31 | 20 (measured off and on in one tree) |

## 3. Lean parity

[`QCore.CallArgumentAccess`](../../../proof/QCore/CallArgumentAccess.lean),
audited by [`AuditCallArgumentAccess.lean`](../../../proof/AuditCallArgumentAccess.lean),
26 theorems over `QCore.IndirectScalarAccess`'s state and obligation:

* **positions:** `a_custody_position_never_admits`,
  `an_uncontracted_callee_never_admits`,
  `an_excess_actual_without_a_tail_never_admits`;
* **the classifier:** `a_statement_the_gate_did_not_refuse_keeps_its_lowering`,
  `a_descriptor_read_keeps_the_refusal`, `an_uncontracted_call_keeps_the_refusal`,
  `a_declined_probe_keeps_the_refusal`, `a_used_actual_is_at_an_admitting_position`,
  `a_custody_position_is_never_used`, `the_reads_precede_the_call`,
  `every_used_base_is_obligated`;
* **the second obligation:** `the_call_sees_the_state_before_the_statement`,
  `the_typed_read_changes_no_binding`, `a_descriptor_value_refuses`,
  `a_pointer_read_through_an_owner_of_sub_custody_refuses`,
  `a_pointer_read_of_a_bound_place_refuses`,
  `an_unbound_name_is_never_refused_by_the_second_obligation`;
* **verdicts:** `a_released_base_refuses_and_is_accused`,
  `resolved_reads_add_no_diagnostic`, `a_refused_read_is_never_safe`;
* **non-vacuity:** `the_brief_example_is_admitted`,
  `the_brief_example_adds_no_diagnostic`, `a_releasing_position_keeps_the_refusal`,
  `a_linked_owner_refuses_the_pointer_read`,
  `an_unlinked_live_owner_admits_the_pointer_read`,
  `a_scalar_read_of_a_descriptor_carrier_refuses`.

Premises stated, not proved: C's evaluation of every actual before the call,
the declared `Borrowed` and data-tail modes (that the callee takes no custody
and keeps no pointer), O04-B09B10's sub-object disjointness, the no-undefined-
behaviour reading, and the Rust–Lean correspondence and C extraction premises
of trust boundary 28. The destination check and the `sizeof` fold are syntax
the model receives as the probe's answer.

## 4. Arbiter on the card's base (superseded by §8)

The evidence of this section is under [pre-merge/](pre-merge/); every link
below that names a file resolves there.

### 4.1 Runs

Runner [`scripts/run_audit.py`](scripts/run_audit.py) (W43-INDIRECT-ACCESS-GENERAL's,
plus the call-argument stream), same pin, Clang 21.1.8 and ChunkIO package as
the baseline.

| Run | Binary | Tree | Exit | Record |
|---|---|---|---:|---|
| BEFORE | `f8dbef9c…` | base `c1e69bb5` | 0 | [before-run.json](pre-merge/before-run.json), [before-identity.txt](pre-merge/before-identity.txt) |
| CENSUS (3 runs) | `d2b321db…`, `5423c5ec…`, `c5472627…` | `d359d3fd`, `43dd4933`, `ba3a22fa` + feature | 0 | [census-run.json](pre-merge/census-run.json) (the third) |
| AFTER (first) | — | `9ccd035d` | 0 | private, superseded (§4.4) |
| AFTER | — | `634bd6bd` | 0 | [after-run-634bd6bd.json](pre-merge/after-run-634bd6bd.json) |
| CENSUS (after) | — | `634bd6bd` + feature | 0 | [census-after-run.json](pre-merge/census-after-run.json) |
| AFTER (archived) | — | `6e8a41b8` (the scope string) | 0 | [after-run.json](pre-merge/after-run.json), [artifacts manifest](pre-merge/artifacts-manifest.json) |

Binary digests are in each `run.json`.

* **BEFORE against the combined baseline**
  (`docs/reports/w43-indexed-resource-collections/integration/artifacts/`,
  `baseline_identity.py --root-alias <this worktree>=…/hungry-thompson-1740da`):
  diagnostics, chunk-contracts and source byte-identical; contracts identical
  after the root alias; source-effects differ in the checkout-path digests
  the brief lists.
* **Every census run against its plain run:** all five artifacts
  byte-identical ([run-identities.json](pre-merge/run-identities.json)).
* **The archived AFTER against the `634bd6bd` AFTER:** equal except the
  `scope` string; the other four artifacts byte-identical. AFTER against
  BEFORE: only the diagnostics move.

### 4.2 Attribution

[`scripts/attribute.py`](scripts/attribute.py) →
[attribution.txt](pre-merge/attribution.txt), [attribution.json.gz](pre-merge/attribution.json.gz);
`attribution_complete: true`.

| Class | Leaves | What it is |
|---|---:|---|
| removed: rule | **143** | forecast leaves (librdkafka 90, chunkio 15, cmetrics 15, monkey-libevent 13, fluent-bit-chunk 10) |
| removed: rule-unforecast | 29 | leaves the probe admitted that W43's classifier filed under `chain` (§1.2): `data = calloc(st.st_size, 1)`, `upper_bounds = calloc(1, sizeof(double) * (count + 1))`, … |
| removed: rule, outside Unknown | 5 | probe-admitted leaves in functions already provisional Unsafe or Safe (the forecast counted Unknown functions) |
| removed: downstream | 9 | binding precision: an acquisition now modeled binds its name, so a later release or null test resolves (`data`, `upper_bounds` ×3, `exemplars` ×2, `buffer`, and two branch refusals of `initialize_exemplars_from_metadata`) |
| added: exchange | **21** | `UnknownAlias` at a removed leaf's coordinate: the base's obligation refusing (`ic` ×3, `ctx` ×3, `ret` ×3, `ch` ×2, `metric` ×2, …) |
| added: exchange, call refusal | 5 | the now-modeled call's own refusal at the same line: an unsanctioned out-slot destination reported as `discarded owned result` (`*outbuf = rd_malloc(*outlenp)`, …) |
| added: downstream | 10 | `UnknownAlias` of a newly tracked name later (`upper_bounds` ×5, `exemplars[dest_index]` ×2) and `nullable resource … used by a non-null contract` ×3 |
| added: violation | **11** | leak leaves, §4.3 |

Separated per `POISON-UNMASK-ON-ADMISSION`: 9 removals come from binding
precision and 11 leak additions from exposure; no removal came from an edge
becoming infeasible.

One forecast leaf stays (`check_oauthbearer_extension_value` L20297): one of
the analyzer's passes reads the `snprintf` there in an environment without its
declared tail, and the probe refuses there (`W44-CALL-ARGUMENT-PROBE-REFUSED`).

### 4.3 The leak pin and the violation candidates

**271 → 282 (constraint 7), in the accusing direction.** Every added leaf is
in a function the rule reached by admitting an acquiring call's actuals, so the
acquisition is now tracked; each is reported by a leak semantics the base
already has, and each is **false by source reading** (witness status: not a
leak):

| Function | Leaves | Why it is not a leak | Base semantics |
|---|---:|---|---|
| `cio_memfs_content_copy`, `cio_utils_read_file`, `initialize_exemplars_from_metadata` | 3 | the buffer escapes through `*out_buf = buf` with no out-parameter contract | an unsanctioned out-slot store keeps the site owed |
| `cmt_histogram_buckets_create_size`, `_exponential_create`, `_linear_create` | 3 | `upper_bounds` is linked into `buckets->upper_bounds` and `buckets` is returned | an object linked into a returned owner, after a subscripted fill, is reported |
| `rd_kafka_telemetry_encode_metrics` | 5 | released by `free_metrics`/`free_resource_attributes` | a release through an uncontracted helper discharges nothing |

Each base semantics is reproduced at this tree by a fixture with **no**
call-argument access (`call_argument_access.rs`,
`an_unsanctioned_out_slot_store_is_reported_as_a_leak_at_the_base`,
`a_release_through_an_uncontracted_helper_is_reported_as_a_leak_at_the_base`,
`an_object_linked_into_a_returned_owner_is_reported_as_a_leak_at_the_base`).
They move seven functions and three units (`cio_memfs.c`, `cio_utils.c`,
`rdkafka_telemetry_encode.c`) Unknown → provisional Unsafe. **Provisional
Unsafe is not an upstream defect claim**, and these eleven are registered as
`W44-CALL-ARGUMENT-EXPOSED-LEAKS` for W44-LEAK-PIN-AUDIT. No
`use_after_release`, `invalid_release` or `double_release` leaf was added.

### 4.4 The suffix-destination hole

The first AFTER run (`9ccd035d`, kept privately as
`aborted-after-9ccd035d-superseded`) had leak 293: eleven more leaves in
`flb_input_chunk_get_direct_routes`. There `result[index].label =
flb_malloc((size_t) label_lengths[index] + 1)` was admitted once its actual
was, because `direct_assignment_destination` names a destination by the
**first suffix** of the left-hand side that spells a place: `label`. The lexical
gate checks the destination only through that name, so the statement modeled a
binding of an unrelated local. `634bd6bd` makes the probe decline unless the
named destination is the whole left-hand side (`destination_is_exact`), which
removed those eleven.

The hole predates this card: with an actual no rule refuses,
`recs[i].name = x_dup(s); x_free(name);` **certifies Safe** at this tree while
releasing a different object (a fail-closed violation, hard invariant 1). It is
pinned (`the_pre_existing_suffix_destination_hole_is_pinned`) so its repair is
a deliberate re-pin, and registered as
`W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE`; its corpus reach is unmeasured.

### 4.5 Movers

| Function | Status | Why |
|---|---|---|
| `cio_file_hash_print` (`cio_file.c`) | Unknown → **Safe** | its two leaves were `printf` tail reads of `(long unsigned int) cf->crc_cur`; Safe relative to the declared premises, no bounds claim |
| the seven functions of §4.3 | Unknown → provisional Unsafe | the exposed false leaks |

### 4.6 Prediction against outcome

| Forecast | Measured | |
|---|---|---|
| 144 leaves removed | 143 of them, plus 29 outside the W43 window and 5 outside Unknown functions | hit; the window miss is registered |
| ≤ 117 exchanges | 21 + 5 | hit |
| ≤ 1 Unknown → Safe, `cio_file_hash_print` | that one | hit |
| 0 unit moves | 3 Unknown → provisional Unsafe | **miss**: exposed leaks |
| leak 271 | 282 | **miss**: eleven exposures, §4.3 |
| 0 violation candidates | 0 `use_after_release`/`invalid_release`/`double_release`; 11 leak candidates | miss on leaks |

The frozen file is not edited; these corrections live here.

## 5. Validation

Measured in this worktree (`agent-a833cb615fbcb8088`) by
[`scripts/run_gate.py`](scripts/run_gate.py) (`CARGO_INCREMENTAL=0`) on the
merge `9c8b7809`; later commits change documents, evidence, report scripts and
the ledger only. Every exit is captured to a file, never through a pipe. Logs
are worktree-private under `private/gate/` and `private/provider-gate/`; the
card-base gate on `dbef7720` is kept as `private/gate-premerge-dbef7720/` and
`private/provider-gate-premerge-dbef7720/` (its figures, 5,425 / 178 / 124, are
in [pre-merge/gate.json](pre-merge/gate.json)) and is superseded. The generator
input is [gate.json](gate.json), with the [Clang recording](clang-version.txt)
taken in this checkout. The dev profile was reclaimed after the test step and
again before the provider gate (the coordinator's disk rule).

| Command | Exit | Result |
|---|---:|---|
| `git submodule update --init --recursive` | 0 | libevent `d82464a2…` |
| `clang --version` | 0 | Ubuntu clang 21.1.8 |
| `cargo test --workspace --no-fail-fast` | 0 | **5,540 passed / 0 failed / 11 ignored**, 293 summaries |
| `cargo clean --profile dev` | 0 | after the test step |
| `cargo fmt --all -- --check` | 0 | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `python3 scripts/check_lean_audits.py --jobs 4` | 0 | **186** audits |
| census feature: clippy `-D warnings`, `cargo test -p qcore-c-gateway --features indirect-census --lib` | 0 | clean; 513 passed |
| `python3 docs/reports/remaining-d02/build_matrix.py . --gate …/gate.json` | 0 | 384 follow-ups, 111 contract modules, 186 Lean audits |
| `python3 -m unittest discover -s scripts/tests` (after the regeneration) | 0 | **330** tests |
| `python3 scripts/build_docs_index.py`, then `--check` | 0 / 0 | regenerated and matching |
| `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline --no-fail-fast` | 0 | **124 passed / 0 failed / 0 ignored**, 39 summaries |
| isolated package mutation control (`--exact --test-threads=1`) | 0 | 1 passed |
| provider fmt / clippy `-D warnings` | 0 / 0 | clean |
| `python3 providers/contracts.py check` | 0 | |
| `python3 providers/package.py build`, then `verify` ×5 | 0, 0 ×5 | chunkio, cmetrics, ctraces, libevent, librdkafka |
| `python3 -m unittest discover -s providers/tests` | 0 | 13 tests |

**Counts.** The card adds the 25-test `call_argument_access.rs` binary, 9 unit
tests of its module and one Lean audit, moves nine pins on its base (§2.4) and
re-pins four of its own fixtures at the merge (§8.2). Main's own figures at
`70ce6b44` were not remeasured here, so the delta is stated by test.

## 6. Ledger

On the combined tree (main's 377 at `70ce6b44`), **377 → 384**:

**Retired** ([retired-followups.json](retired-followups.json)):
`W43-INDIRECT-CALL-ARGUMENT-ACCESS`, against its own discharge condition ("a
card that admits a typed scalar access in a Borrowed data argument … with the
call's existing obligations retained, and a measured re-pin; or a recorded
decision that the class belongs to the call routes"). The uncontracted class is
recorded as the call routes'; the call-result destination and every other
refused shape are re-registered, measured.

**Registered on this card's branch only, and not registered on the combined
tree** (recorded in the same file with `registered_on_main: false`; nothing is
removed for them, their entries are kept verbatim in
[branch-only-followups.json](branch-only-followups.json)):

* `W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE`, discharged by the card of the same
  name on main (constraint 5i), with this card's fixture re-pinned to its
  answer;
* `W44-CALL-ARGUMENT-EXPOSED-LEAKS`, discharged on the combined tree by
  W44-LEAK-PIN-AUDIT's refused hand-on rule: all eleven leaves are withdrawn,
  measured (§8.3).

**`W44-REFUSED-HANDOFF-LEAK-CLAIMS` is not retired here**: it is registered
neither on main nor on this branch (W44-LEAK-PIN-AUDIT's report says it lives
on another card's branch), so removing it would be a no-op. This card measured
that the rule it names withdraws all eleven of this card's base exposures
(§8.3).

**Amended** ([amended-followups.json](amended-followups.json)):
`MK-LIST-ADD-BORROWED-PUBLICATION` gains one measured sentence: this card
models `mk_rconf_read`'s `mk_list_add`, so that leaf is accused again, and 4 of
its 8 now stay.

**Registered** ([followups.json](followups.json)), sized at the combined-tree
AFTER census ([residuals.txt](residuals.txt); leaves / Unknown functions / sole):

| Id | Size | What stays open |
|---|---|---|
| `W44-CALL-ARGUMENT-UNCONTRACTED` | 7,946 / 2,177 / 180 | actuals of uncontracted callees; the call routes' |
| `W44-CALL-ARGUMENT-UNTYPED-ACTUAL` | 1,762 / 795 / 97 | actuals the recogniser refuses (addresses into incomplete records, unreadable functions, `sizeof (*v)`, chains, untyped bases and members, nested calls, casts) |
| `W44-CALL-RESULT-INDIRECT-DESTINATION` | 948 / 516 / 37 | `p->f = g()`; larger on the combined tree because constraint 5i refuses every sub-object store |
| `W44-CALL-ARGUMENT-CUSTODY-POSITION` | 505 / 251 / 4 | indirect actuals at custody-taking positions, by design |
| `W44-CALL-ARGUMENT-NON-STATEMENT` | 142 / 125 / 39 | `for` clauses, conditions, returns |
| `W44-CALL-ARGUMENT-PROBE-REFUSED` | 52 / 41 / 1 | statements still refused after the probe |
| `W44-CALL-ARGUMENT-ACCUSING-UNEXERCISED-IN-CORPUS` | — | the `use_after_release` arm reaches no corpus function |
| `W44-CALL-ARGUMENT-FAMILY-WINDOW` | 1,580 leaves | W43's classifier window |

**Noted, not retired:** `POISON-UNMASK-ON-ADMISSION` (measured: 10 removals
by binding precision, 1 leak exposure); `W43-INDIRECT-ACCUSING-UNEXERCISED-IN-CORPUS`.

`corpus.cycle19_raw_units` points at this report's [artifacts/](artifacts/);
the previous (W44-LIBC-RESIDUALS integration) row is in
[previous-corpus-row.json](previous-corpus-row.json).

## 7. For the integrator

* **Proposed gateway amendment**, a constraint beside 5g–5i, in these words:

  > W44-INDIRECT-CALL-ARGUMENT-ACCESS: a simple statement the lexical gate
  > refused for an actual of a contracted call, reading no tracked descriptor,
  > is classified again with every typed read at a declared `Borrowed`
  > position or in a declared variadic data tail admitted (a scalar value over
  > typed scalar accesses, `sizeof` of a type name a constant; `B[I]`/`*B` over
  > `T **B`; `B->F`/`S.F` over a pointer member; `&B->F`/`&S.F` over a
  > non-pointer member; `&B[I]` over a scalar pointer or array; no chain).
  > Only a contracted call whose named destination is its whole left-hand side,
  > or an element store around one, is admitted; it lowers to the typed read of
  > every base (carrying no chain), `ArgumentReads` (integer-carried
  > identities, pointer reads through an owner of a field link, contained child
  > or collection, and bound pointer places refuse), then the call.
  > Custody-taking positions and uncontracted callees keep the refusal byte for
  > byte. The read decides its base's custody only.

* **Constraint 7:** record 19 → 20 on the combined tree: one class (c) leaf,
  `mk_rconf_read` (`MK-LIST-ADD-BORROWED-PUBLICATION`), re-exposed because its
  `mk_list_add` is now modeled rather than refused (§8.3).
* **Composition.** Chains: the rule's typed read carries an empty `chains`, and
  `typed_scalar_value` uses the unextended walker. W44-COLLECTION-ALLOCATION:
  `sizeof (*v)` is left to it. W44-BRANCH-CONDITION-CALLS: this rule is
  consulted only by the statement walker; condition calls are its residual
  `W44-CALL-ARGUMENT-NON-STATEMENT`.
* **Files likely to conflict:**
  * `crates/qcore-c-gateway/src/legacy_c.rs`: the module declaration, the
    `Parser` field and constructor (beside `desugar`), the three hooks in
    `classify_simple_base` (after `borrowed_actuals.extend(dereferenced)`, the
    formal-expression check, the tail-data call), `census_indirect`'s early
    return, the statement walker after `stepped_access`, the `ArgumentReads`
    variant (after `PointerStep`) and its transfer arm, and one arm in the
    `transfer_base` match;
  * `crates/qcore-c-gateway/src/legacy_descriptor_custody.rs` (one arm),
    `typed_field_access.rs` (`typed_scalar_value`), and
    `compiler/examples/alias_aggregate_probe.rs`;
  * `crates/qcore-audit/src/report.rs` (the scope string),
    `docs/QCORE_SPEC_r0_3.md` L18, `proof/QCore.lean`;
  * nine gateway test files with deliberate pin moves (§2.4);
  * the shared records: `docs/reviews/r0_3/acceptance_matrix.json`,
    `docs/STATUS.md`, `docs/development/increments/wave-44.md`,
    `docs/reports/INDEX.md`.
* The `contracts` and `source-effects` artifacts embed this worktree's path.

## 8. The merge of main `70ce6b44`

Main advanced from `c1e69bb5` with W44-INDIRECT-CHAIN-ACCESS,
W44-CORPUS-CALLEE-SUMMARIES, W44-BRANCH-CONDITION-CALLS, W44-LEAK-PIN-AUDIT,
W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE and W44-LIBC-RESIDUALS. The merge is
`9c8b7809` (`git merge`, no rebase).

### 8.1 Resolutions

* **`legacy_c.rs`**, two hunks, both sides kept: the `Parser` field and its
  constructor, beside W44-BRANCH-CONDITION-CALLS' `desugar`. The
  `classify_simple_base` gate arm merged without a conflict: main's
  `names_a_sub_object` refusal is the destination half of the same lexical-gate
  disjunction, and this card's admitted spans only exempt value-side tokens
  (`outside_spans(…, equal + 1)`), so a sub-object destination is refused
  whatever the actuals are.
* **`legacy_call_argument_access.rs`**: the typed read now carries
  W44-INDIRECT-CHAIN-ACCESS's `chains` field, **empty**. The recogniser refuses
  every chain, and `typed_scalar_value` calls the walker with `no_extension`,
  so no admitted actual carries a chain whose link obligations would be dropped.
* **`QCore.lean`**: both sides' imports kept. `QCore.IndirectScalarAccess`,
  which `QCore.CallArgumentAccess` imports, is unchanged on main; the module
  builds on the merged tree.
* **Records** took main's version at the merge and were re-derived from this
  re-measurement (the ledger by [`scripts/update_ledger.py`](scripts/update_ledger.py)).

### 8.2 Pins re-pinned at the merge

* `call_argument_access.rs` `the_pre_existing_suffix_destination_hole_is_pinned`:
  `Safe` → `Unknown`, `[GATE, "unknown_alias:name"]`, exactly main's
  `a_member_store_no_longer_binds_the_same_named_local`.
* The three leak-mechanism fixtures (renamed `…_is_withdrawn_by_the_hand_on_rule`):
  `Unsafe` + `leak` → `Unknown` with the refusal kept (the linked-owner one
  also `unknown_alias:ub`). W44-LEAK-PIN-AUDIT's own fixtures for the same
  mechanisms are `tests/exposed_leak_mechanisms.rs`.

**`destination_is_exact` is kept.** On the combined tree it is redundant:
constraint 5i refuses a sub-object store at the lexical gate before any call is
classified, so the probe never sees a misnamed destination. Measured: a private
build with the guard disabled produced all five corpus artifacts byte-identical
to AFTER ([run-identities.json](run-identities.json),
[guard-disabled-run.json](guard-disabled-run.json)). It is kept because it can
only refuse, costs one comparison, and states in this route the premise the
admission relies on, rather than resting silently on another module's
invariant. Its doc comment says so.

### 8.3 Arbiter

| Run | Binary (sha256) | Tree | Exit | Record |
|---|---|---|---:|---|
| BEFORE | `817319d2…` | main `70ce6b44` | 0 | [before-run.json](before-run.json), [before-identity.txt](before-identity.txt) |
| AFTER | `9dc8ee15…` | merge `9c8b7809` | 0 | [after-run.json](after-run.json), [artifacts/](artifacts/) |
| CENSUS (after) | `492b1d3f…` | `9c8b7809` + feature | 0 | [census-after-run.json](census-after-run.json), [census/census-combined-after.txt](census/census-combined-after.txt) |
| AFTER, guard disabled | private | `9c8b7809` with `destination_is_exact` off | 0 | [guard-disabled-run.json](guard-disabled-run.json) |

* **BEFORE against the combined baseline**
  (`docs/reports/w44-libc-residuals/integration/artifacts/`, root alias
  `…/hungry-thompson-1740da`): diagnostics, chunk-contracts and source
  byte-identical; contracts identical after the root alias; source-effects
  differ in the checkout-path digests.
* **The census run and the guard-disabled run against AFTER:** all five
  artifacts byte-identical. **AFTER against BEFORE:** only the diagnostics move.

[`scripts/attribute.py`](scripts/attribute.py) (census = the combined census
run) → [attribution.txt](attribution.txt),
[attribution.json.gz](attribution.json.gz); `attribution_complete: true`.

| Class | Leaves | What it is |
|---|---:|---|
| removed: rule | **143** | the frozen forecast leaves, the same 143 as on the card's base |
| removed: rule-unforecast | 41 | probe-admitted leaves outside the forecast: the card base's 29; 4 more whose functions are Unknown here (they were provisional Unsafe on the base: `cmt_summary_create`, `append_to_ring_buffer`, `mk_rconf_read`, `mk_sched_init`); and 8 **complementary** leaves, whose callees gained contracts on main (`cio_file_st_get_meta`/`_get_hash`/`_set_meta_len` ×6, `rd_strlcpy`, `vsnprintf`) |
| removed: rule, outside Unknown | 1 | a probe-admitted leaf in a function not Unknown at BEFORE |
| removed: downstream | 10 | the card base's 9 (bindings that now resolve) and 1 branch refusal in `rd_kafka_oauthbearer_set_token0` exchanged for the `token_value` alias below |
| added: exchange | **21** | the base's obligation refusing (`UnknownAlias`), the same 21 as on the card's base |
| added: exchange, call refusal | 5 | the modeled call's own `discarded owned result` at the same line, the same 5 |
| added: downstream | 11 | the card base's 10 and `token_value` in `rd_kafka_oauthbearer_set_token0` L20330 |
| added: violation | **1** | `mk_rconf_read`'s leak, below |

The forecast leaf still kept is the same one as on the base
(`check_oauthbearer_extension_value` L20297).

**What became of the card-base moves.**

| Card base (`c1e69bb5`) | Combined tree |
|---|---|
| `cio_file_hash_print` Unknown → Safe | Unknown → **Safe**, unchanged |
| 11 exposed leaks: `cio_memfs_content_copy`, `cio_utils_read_file`, `initialize_exemplars_from_metadata`, `cmt_histogram_buckets_{create_size,exponential_create,linear_create}`, `rd_kafka_telemetry_encode_metrics` ×5 | **all withdrawn** by the refused hand-on rule; all seven functions Unknown with 0 leak leaves |
| 7 functions and 3 units Unknown → provisional Unsafe | **none**; `cio_memfs.c`, `cio_utils.c`, `rdkafka_telemetry_encode.c` stay Unknown |

**Leak classification** (constraint 7, 19 → 20), by reading, in the leak
audit's classes ([`../w44-leak-pin-audit/scripts/`](../w44-leak-pin-audit/scripts/)):
the 19 baseline leaves are main's, classified by W44-LEAK-PIN-AUDIT and
unchanged here. The one added leaf:

| Function | Exit / acquired | Class | Mechanism | Reason |
|---|---|---|---|---|
| `mk_rconf_read` | `mk_core/mk_rconf.c:465` / `:456` | **(c) premise gap** | borrowed-list-insertion | `mk_list_add(&file->_head, &conf->includes)` links the new file record into the caller's configuration before `return 0`; `mk_list_add` is declared `resource_borrowed` on both positions, which does not state the publication (`MK-LIST-ADD-BORROWED-PUBLICATION`). The leak audit classified this exact leaf (its row 238) the same way; its hand-on rule withdrew it only because the call was refused. |

It is not a real leak and not a projection artifact; no leaf is withdrawn in
the discharging direction. It moves `mk_rconf_read` and `mk_core/mk_rconf.c`
Unknown → provisional Unsafe. **Provisional Unsafe is not an upstream defect
claim.**

**Residual census on the combined tree**: [residuals.txt](residuals.txt)
(widened family 11,356 / 2,721 / 295; W43 window 9,776 / 2,442 / 252).

The new baseline is [artifacts/](artifacts/), with its manifest; the card-base
archive's manifest is [pre-merge/artifacts-manifest.json](pre-merge/artifacts-manifest.json).

## Reproduction

```sh
# combined tree
python3 docs/reports/w44-indirect-call-argument-access/scripts/run_audit.py <before> 70ce6b44 --binary <main binary>
docs/reports/w43-return-expression/scripts/baseline_identity.py <before> docs/reports/w44-libc-residuals/integration/artifacts --root-alias <worktree>=<integrator worktree>
python3 docs/reports/w44-indirect-call-argument-access/scripts/run_audit.py <after> 9c8b7809 --binary <merge binary>
python3 docs/reports/w44-indirect-call-argument-access/scripts/run_audit.py <census-after> 9c8b7809+census --census --binary <merge census binary>
python3 docs/reports/w44-indirect-call-argument-access/scripts/attribute.py <before> <after> <census-after> census/census.json attribution.json attribution.txt
python3 docs/reports/w44-indirect-call-argument-access/scripts/family_residuals.py <census-after> residuals.json residuals.txt
python3 docs/reports/w43-libc-contracts/scripts/archive.py <after> docs/reports/w44-indirect-call-argument-access/artifacts
python3 docs/plans/cycle19-custody-cards/blocker_census.py docs/reports/w44-indirect-call-argument-access/artifacts
python3 docs/reports/w44-indirect-call-argument-access/scripts/update_ledger.py
python3 docs/reports/w44-indirect-call-argument-access/scripts/run_gate.py <gate-dir> submodules clang test fmt clippy probe followup lean census
# card base (c1e69bb5): the census and forecast
python3 docs/reports/w44-indirect-call-argument-access/scripts/run_audit.py <census> ba3a22fa+census --census --binary <census binary>
python3 docs/reports/w44-indirect-call-argument-access/scripts/call_argument_census.py <census> census.json census.txt [--union]
```

The census streams are not committed; the base census's call-argument stream
is `6f2e7225…` (SHA-256 prefix).
