# W43-INDIRECT-ACCESS-GENERAL: typed scalar accesses through pointers

Card W43-INDIRECT-ACCESS-GENERAL, 2026-09-24, on base `d3cd8d81` (main after
the W43-LIBC-RESULT-ALIAS integration). It generalises W42-INDIRECT-ACCESS's
bounded `BASE->FIELD = scalar` route to one typed rule. A statement whose
every memory access is a typed scalar access (`*p`, `p[i]`, `p->f`, `s.f`),
and whose destination and value are scalar, is admitted if the existing
classifier refused it. It is admitted as custody-neutral, and it keeps the
dereference obligation of every base. The forecast was frozen before any rule
existed ([PREDICTION.md](PREDICTION.md), commit `eff8fac1`).

## Headline

* **Census first.** The brief's own example, `*p = '\0'`, is **not** in the
  `indirect memory access` family. A single-dereference destination is named
  `*p`. It lands in the `store through *p without an out-parameter contract`
  or `unsupported assignment expression` family. `p[i]++`, `(*p)++` and
  `p->f += e` land in the increment and unrecognized-destination families. The
  census therefore covers five families, statement by statement, from the
  analyzer's own token view ([§1](#1-the-shape-census)).
* **Shape ranking.** Admitting a *syntactic* shape wholesale would decide at
  most 81 functions (call arguments), 57 (chain reads), 22 (chain stores) or
  19 (single member stores). No typed rule reaches those figures, because the
  member or base reader refuses most of them. This reproduces the finding of
  C-GATEWAY-INDIRECT-CENSUS-2 and BASE-TYPE-READER-CENSUS.
* **The rule** ([§2](#2-the-rule)). Every base a statement dereferences must
  be one of three things:
  * unbound on every reaching path;
  * a non-null binding with no tracked target;
  * exactly one known, non-null, live identity. This includes derived
    `strchr`/`memcpy` results, which dereference their base's allocation.

  Anything else records `UnknownAlias`. A base released on a reaching path is
  a `use_after_release` candidate. A write through `*p`/`p[i]` keeps the
  binding frame of the refusal it replaces. Bounds are not decided.
* **Lean.** `QCore.IndirectScalarAccess` has 31 theorems, audited by
  `AuditIndirectScalarAccess.lean`. It has no placeholders and no project
  axioms.
* **Arbiter.** BEFORE reproduces the W43-LIBC-RESULT-ALIAS diagnostics byte for
  byte. AFTER removes **1,112** leaves and adds **83**, each attributed
  ([§4](#4-arbiter)):
  * **985** removals are exactly the forecast rule leaves;
  * **98** are rule leaves the forecast mirror missed, for one disclosed
    reason;
  * **1** is an unreached node;
  * **16** are call leaves decided by new validated summaries;
  * **12** are downstream: 7 by binding precision, 2 by bindings a new
    summary created, and 3 behind an edge that became infeasible;
  * **78** additions are flow exchanges and 5 are summary-side refusals.
* **Moves.**
  * **12 functions Unknown → Safe** (11 forecast). All 12 are Safe relative to
    the declared premises.
  * **1 unit Unknown → Safe** (`cmt_time.c`; 0 forecast).
  * **0 provisional Unsafe added.** 0 violation candidates.
* **Leak pin 272 → 271**, which the forecast missed. One false leak in
  `rd_kafka_snappy_java_uncompress` is withdrawn. The release before it is now
  credited, because a frame-bounded admitted store replaced a blanket-poisoning
  refusal ([§4.3](#43-the-leak-pin)).
* **Ledger: 297 → 306.** Three entries are retired:
  * `INDIRECT-CENSUS-FEATURE-UNBUILT` and
    `BASE-TYPE-READER-CENSUS-FEATURE-UNBUILT` (a CI step now builds and tests
    the census feature);
  * `INDIRECT-EXCHANGE-PROXY-UNVALIDATED` (measured, and narrowed).

  Twelve entries are registered, including one per refused shape with its
  measured size ([§6](#6-ledger)).

| Measure | BEFORE (W43-LIBC-RESULT-ALIAS) | AFTER | Delta |
|---|---:|---:|---:|
| Functions Safe / provisional Unsafe / Unknown | 257 / 84 / 4,493 | **269 / 84 / 4,481** | +12 / 0 / −12 |
| Units Safe / provisional Unsafe / Unknown / data-only | 2 / 42 / 137 / 4 | **3 / 42 / 136 / 4** | +1 / 0 / −1 / 0 |
| `unsupported` | 48,930 | **47,832** | −1,098 |
| `unknown_alias` | 4,657 | **4,727** | +70 |
| `leak` / `summary_conflict` / `double_release` | 272 / 1 / 0 | **271** / 1 / 0 | −1 / 0 / 0 |
| `invalid_release` / `use_after_release` | 0 / 0 | 0 / 0 | 0 |
| indirect memory access: leaves / Unknown functions / sole | 17,665 / 3,315 / 378 | 17,300 / 3,291 / 380 | −365 / −24 / +2 |
| store through `*p` without an out-parameter contract | 656 / 253 / 2 | 230 / 124 / 0 | −426 / −129 / −2 |
| unrecognized assignment destination | 901 / 434 / 3 | 772 / 385 / 2 | −129 / −49 / −1 |
| unmodeled increment or decrement | 734 / 378 / 5 | 678 / 345 / 1 | −56 / −33 / −4 |
| unsupported assignment expression | 1,683 / 589 / 4 | 1,645 / 569 / 5 | −38 / −20 / +1 |

The family rows count leaves in Unknown functions, as
[blocker-census.txt](blocker-census.txt) does. The two sole-blocked counts that
rise are functions whose other families were cleared.

## 1. The shape census

### 1.1 Instrumentation

The `indirect-census` feature (C-GATEWAY-INDIRECT-CENSUS-2,
BASE-TYPE-READER-CENSUS) now also streams the following. Every addition is
compiled out with the feature off, and is written after the classifier has
decided:

* for each refused `indirect memory access` statement:
  * its complete token text;
  * the declaration reader's label for each identifier (declared type, opaque
    cause, enumeration constant, scalar type name, and the new scalar-element
    answer below);
  * which of its names the body assigns a declared derived or owned result;
* a **statement stream** (`QCORE_STATEMENT_CENSUS`): every ordinary statement
  the classifier refused, or lowered to an unsanctioned `*p` store, with the
  refusal's own description, whole-statement chains and `may_bind` names.

Every census run reproduced the baseline diagnostics byte for byte
([census-identity.json](census-identity.json)). That is the inertness claim
with the feature on and streaming. The AFTER census run reproduces the AFTER
diagnostics byte for byte ([census-after-identity.json](census-after-identity.json)).

The declaration reader gained one **side table**, `FunctionTypes::scalar_element`.
A name declared exactly once as `SCALAR *name` or `SCALAR name[N]` has a
scalar element. No `DeclaredType` answer changes, and the table was read by
no rule until the admission commit.

The census is built in CI from now on: `.github/workflows/ci.yml` runs clippy
with `-D warnings` and the library tests of `qcore-c-gateway` with the feature
on.

### 1.2 What it measured

These figures come from `scripts/shape_census.py` over the census run of the
base tree ([census/shape-census.txt](census/shape-census.txt),
[census/shape-census.json.gz](census/shape-census.json.gz)), counted in
Unknown functions:

| Family | Leaves | Functions | Sole-blocked |
|---|---:|---:|---:|
| indirect memory access | 17,665 | 3,315 | 378 |
| unsupported assignment expression | 1,683 | 589 | 4 |
| unrecognized assignment destination | 901 | 434 | 3 |
| unmodeled increment or decrement | 734 | 378 | 5 |
| store through `*p` without an out-parameter contract | 656 | 253 | 2 |

The indirect family breaks down by the shape of its hardest statement. The
"decided" column is the number of functions decided if the shape alone were
admitted wholesale, before any type premise:

| Shape | Leaves | Functions | Decided | Pairs it completes |
|---|---:|---:|---:|---|
| access in a call statement's arguments | 8,128 | 2,131 | 81 | unresolved identity 49, uncontracted call 45, branch condition 33 |
| chain store `a->b->f = e` | 2,862 | 1,097 | 22 | uncontracted call 23, branch condition 13 |
| single member store `p->f = e` | 2,602 | 869 | 19 | uncontracted call 30 |
| chain read `x = a->b->f` | 2,262 | 1,410 | 57 | uncontracted call 32, branch condition 17 |
| other value-side expressions | 973 | 546 | 2 | |
| subscript store `p[i] = e` | 841 | 135 | 1 | |
| single member read `x = p->f` | 569 | 344 | 0 | |
| record-object store `s.f = e` | 373 | 132 | 13 | |
| deref store `*p = e` | 167 | 112 | 3 | |

`*p = e` is mostly in the store-through family (656 leaves), not here.

Three findings shape the admission:

* **Leaves and functions diverge again.** The largest shape, call arguments,
  decides at most 81 functions, and only if every access in every argument
  were admitted wholesale.
* **The member readers dominate.** Of the chains in refused statements, the
  large majority end at a member or base the reader does not classify
  (`Unresolved`). A typed rule cannot admit those.
* **The typed rule's forecast is small.** 985 leaves leave their families. 10
  functions are decided directly, and 1 more is decided through a W4 summary.
  The forecast mirror applies the planned rule to every census statement
  (`scripts/predict.py`, [census/predict-unknown.txt](census/predict-unknown.txt)).

### 1.3 Prediction, frozen

[PREDICTION.md](PREDICTION.md) (commit `eff8fac1`) forecast the following:

* 985 leaves leaving their families across all statuses:
  * store-through 446;
  * indirect 293;
  * unrecognized destination 144;
  * increment 64;
  * unsupported assignment 38;
* at most 70 exchanges;
* 11 Unknown → Safe, listed by name, with 1 of them transitive;
* 0 unit moves;
* leak pin unchanged at 272;
* 0 violation candidates.

## 2. The rule

### 2.1 Classifier

`classify_simple` wraps the existing classifier. It consults
`typed_field_access::typed_scalar_statement` **only** in two cases:

* the existing classifier refused the statement with one of:
  * `indirect memory access is outside the resource projection`;
  * `unsupported assignment expression`;
  * `unrecognized assignment destination`;
  * `unmodeled increment or decrement expression`;
* it lowered the statement to an `Alias`/`Clear` whose destination is `*p`
  (the out-slot naming, rejected in transfer unless sanctioned).

A statement the existing classifier admits keeps its exact lowering. This
includes W42's `ScalarMemberStore`, the unbound scalar-field no-op, aggregate
stores and opaque reads. That conservativity is proved as
`an_admitted_statement_keeps_its_lowering`.

The rule admits a statement with no call, no `{ } ; ,`, no unary `&`, no nested
assignment and no increment except its own. Every access must be one of:

| Access | Premise (declaration facts of this unit and function) |
|---|---|
| `* B` | `B` declared `SCALAR * B`: one `*` over a specifier list the classifier resolves to `Scalar` (so never `void *`, `volatile` or `_Atomic`) |
| `B [ I ]` | `B` declared `SCALAR * B` or `SCALAR B [ N ]` (one dimension); `I` scalar-only (declared scalars, enumeration constants, numeric literals) |
| `B -> F` | `B` declared `struct T *`; `F` a scalar member of a completely read, single-definition, non-union `struct T` |
| `S . F` | `S` declared `struct T` (an object); `F` a scalar member |

The statement itself must take one of these forms:

* `ACCESS = E` or `ACCESS op= E` (every compound operator, `<<=`/`>>=`
  included);
* `++ACCESS`, `--ACCESS`, `ACCESS++` or `ACCESS--` through a pointer. `*p++`
  is never admitted, because it increments the pointer;
* `X = E`, `X op= E` or `SCALAR_TYPE X = E`, where `X` is a declared scalar
  and `E` carries at least one access.

`E` is built from numeric and character literals, declared scalars,
enumeration constants, casts to a scalar type name, and accesses. The
statement lowers to `LegacyInstruction::IndirectScalarAccess { bases,
alias_reads, destination, frame }`.

### 2.2 Transfer

In `transfer`:

1. The C02-P-ALIAS may-alias obligation of each `->` base is checked, as for
   every modeled read.
2. Each base must pass `indirect_scalar_base_resolves`. It passes when it is:
   * **unbound on every reaching path.** No modeled statement bound it: a
     formal's caller-provided pointer, an automatic array, or a name whose
     only binding statement was itself refused, which already keeps the
     function Unknown. This is the reading an unbound name already gets at a
     `Borrowed` position.
   * **a non-null binding with no tracked target.** This is an untracked or
     derived-untracked pointer, or an opaque member value, after its null
     test. Every other construction that loses all targets (widening, a
     refusal's poison) is itself refused.
   * **exactly one known, non-null target whose status is exactly live**, with
     no nullable or unresolved-effect marker, identity or derived alike.

   A possibly-null binding (it must be guarded), a `Mixed` derivation, an
   ambiguous, released, escaped or unresolved identity, and an unknown binding
   that still carries a tracked target all refuse with `UnknownAlias`. A `*B`
   store to a slot an out-parameter contract sanctions also refuses. The
   out-slot machinery keeps that slot.
3. The statement writes only a scalar. The resource, nullable, unresolved,
   custody, contained and field-link maps are preserved whether the statement
   is admitted or refused. The destination scalar's old binding is forgotten,
   as `ForgetScalar` does.
4. **The frame.** A write through `*B` or `B[I]` may reach any byte of the
   object `B` points into. A character-typed write can rewrite a pointer
   object's representation without undefined behaviour (C11 6.5p7). So every
   binding outside the constraint-10a frame is poisoned, exactly as the
   refusal it replaces poisoned it. The frame spares a name whose slot no
   other lvalue designates, and is computed from the parsed statement (whose
   only writes are its typed destination), not from `may_write_names`, which
   reads `* p =` as a write of `p`. The joined-provenance pre-pass forgets
   non-preserved names at the same statements. A member store or a read
   writes one scalar and poisons nothing.
5. `report_indirect_scalar_violations` reads the **settled** states
   (constraint 6). It reports a `use_after_release` candidate where a base is
   settled on one identity whose status carries the released bit (the
   `DoubleRelease` reading), and never through an unresolved site. A
   candidate is provisional `Unsafe` under the existing policy. It is never a
   validated upstream defect.

### 2.3 Property scope (invariant 2)

The verdict's scope field and spec L18 now say that an admitted scalar access
decides **custody only**. It never decides bounds, alignment, or the liveness
or nullness of an object the projection does not track. A subscript's index is
read as a scalar, and nothing is claimed about the object's extent. Constness
is outside the property as well: a derived `const_dropped` pointer is written
through when its base resolves (`W43-INDIRECT-CONST-DROPPED-WRITE`).

### 2.4 Tests and pins

`crates/qcore-c-gateway/tests/indirect_scalar_access.rs` has 23 tests. Each
asserts the complete diagnostic vector with the status, and every `Safe`
certificate is rechecked. They cover:

* every admitted form through unbound bases;
* guarded allocation fill (the `mk_string_dup` shape);
* the unguarded-nullable refusal;
* store, read and member use after release, including a release on one
  branch;
* guarded and unguarded derived `strchr` stores over a caller's string and
  over a live allocation;
* the derived use after release;
* the refused shapes: `*p++`, a pointer value, a pointer member, a call, an
  access index, and a protected-name increment;
* the frame poisoning a binding whose name is address-taken, beside the same
  body without the store (Safe) and a member store (Safe);
* a leak the store does not touch.

Two existing pins move deliberately, and each is explained where it stands:

* `scalar_member_store.rs` no longer lists `p->state = p->state` among the
  refused right-hand sides. A new test pins it Safe on a live identity and
  refused after release.
* `typed_field_access.rs` moves `d->cursor = v[0]` out of the ambiguous table.
  A new test pins its certificate.

## 3. Lean parity

[`QCore.IndirectScalarAccess`](../../../proof/QCore/IndirectScalarAccess.lean)
models the dereference obligation, the custody-neutral transfer with the
frame, the accusing reading and the classifier wrapper. It is audited by
[`AuditIndirectScalarAccess.lean`](../../../proof/AuditIndirectScalarAccess.lean),
with 31 theorems:

* **custody neutrality:** `the_statuses_are_unchanged`,
  `the_markers_are_unchanged`, `custody_and_field_links_are_unchanged`,
  `no_binding_gains_a_target`;
* **the frame:** `a_spared_binding_is_untouched`,
  `without_a_frame_only_the_destination_changes`;
* **the obligation:**
  * `an_unbound_base_resolves`;
  * the refusals: `a_nullable_base_refuses`, `a_mixed_base_refuses`,
    `an_ambiguous_base_refuses`, `an_unknown_base_with_a_target_refuses`,
    `a_released_base_refuses`, `an_unresolved_base_refuses`;
  * `a_resolved_tracked_base_is_one_live_identity`;
* **accusation:** `a_resolved_base_is_never_accused`,
  `an_accusation_names_a_released_settled_identity`;
* **verdicts:** `an_admitted_statement_adds_no_diagnostic`,
  `a_refusal_is_never_safe`;
* **the classifier:** `an_admitted_statement_keeps_its_lowering` and six
  syntax-premise refusals;
* **non-vacuity:** a live identity, an unbound formal and a guarded
  derived-untracked pointer are admitted; a released identity is refused and
  accused; the full-syntax candidate is admitted.

The model's premises are the frame (the constraint-10a C object model under
the no-undefined-behaviour reading) and the Rust–Lean correspondence and C
extraction premises of trust boundary 28. They are stated, not proved.

## 4. Arbiter

### 4.1 Runs

Each run uses [`scripts/run_audit.py`](scripts/run_audit.py), W43-RETURN-EXPRESSION's
runner with a `--binary` option and the statement census variable. Each run
has the same pin, Clang and ChunkIO package as the baseline.

| Run | Binary (sha256) | Tree | Exit | Record |
|---|---|---|---:|---|
| BEFORE | `1cb57bab…` | base `d3cd8d81` | 0 | [before-run.json](before-run.json), [before-identity.json](before-identity.json) |
| CENSUS (base) | `9e8a8c84…` | census instrumentation only | 0 | [census-run.json](census-run.json), [census-identity.json](census-identity.json) |
| AFTER | `564f0263…` | `d975ffc6` | 0 | [after-run.json](after-run.json), [artifacts/](artifacts/) |
| CENSUS (after) | `8235cf9e…` | `d975ffc6` + feature | 0 | [census-after-run.json](census-after-run.json), [census-after-identity.json](census-after-identity.json) |

**BEFORE against the W43-LIBC-RESULT-ALIAS baseline:**

* diagnostics, chunk-contracts and source are byte-identical;
* `contracts` is identical once the checkout root is aliased;
* `source-effects` carries the checkout-path digests the brief lists.

**AFTER against BEFORE (same checkout):** chunk-contracts, contracts,
source-effects and source are byte-identical. Only the diagnostics move. They
move by the leaves attributed below and by the report's `scope` string
([§2.3](#23-property-scope-invariant-2)).

An earlier AFTER on `29aca41d` differs from this one only in that `scope`
string. It was measured before the scope text landed and is kept privately.

Four census runs were stopped or superseded while the instrumentation was
extended (see [§7](#7-attempts-that-are-not-the-evidence)). None is evidence.

### 4.2 Attribution

[`scripts/attribute.py`](scripts/attribute.py) puts every leaf in exactly one
class, with `attribution_complete: true`. The results are in
[attribution.txt](attribution.txt) and
[attribution.json.gz](attribution.json.gz).

| Class | Leaves | What it is |
|---|---:|---|
| removed: rule | **985** | an intercepted refusal whose every census statement the forecast mirror admits. Equal to the forecast, family by family: store-through 446, indirect 293, unrecognized destination 144, increment 64, unsupported assignment 38 |
| removed: rule-unforecast | 98 | intercepted refusals the mirror refused and the rule admitted. All are indirect-access two-base member copies (`metric_dst->x = metric_src->x`). The indirect census records only the destination window of a destination-refused statement, so the mirror had no member class for the value-side base ([unforecast.txt](unforecast.txt), `W43-INDIRECT-FORECAST-CENSUS-WINDOW`) |
| removed: unreached | 1 | `rd_kafka_handle_OffsetFetch` L21154, a call statement the rule never admits. Its node is no longer reached: `topic_id` is initialised `NULL` and never reassigned (the assignment is commented out in the source), and the frame-bounded stores now keep that fact, so the `if (topic_id)` edge is infeasible |
| removed: summary | 16 | `call to uncontracted function F` leaves where `F` moved to Safe and gained a validated W4 summary (constraint 4): `set_result` ×10, `cio_chunk_counter_total_add`/`_sub`, `empty_batches_create`, `flb_input_chunk_output_would_exceed_limit`, `mk_string_dup` ×2 |
| removed: downstream | 12 | 7 by binding precision, regained where a refusal that poisoned bindings became a framed admitted store: `outbuf` ×3 and 1 leak (snappy, [§4.3](#43-the-leak-pin)), `cf` ×2 (`cio_file_open`), `scope_json`. 2 by bindings a new summary created: `buf` and `out`, bound by `mk_string_dup` calls that are now modeled. 3 by the infeasible OffsetFetch `topic_id` edge: 2 alias leaves and 1 `rd_kafka_Uuid_destroy` call leaf |
| added: exchange | **78** | `UnknownAlias` at the coordinate of a removed rule leaf: the flow half refusing a base that is possibly null, poisoned, or bound to an unknown target. There are 45 functions (librdkafka 55, cmetrics 9, monkey 7, chunkio 5, ctraces 2), and none moves |
| added: downstream | 5 | summary-side refusals: `empty_batches_create` and `mk_string_dup` now carry validated summaries with nullable results, and their callers pass the result where a non-null contract requires it (`nullable resource … used by a non-null contract` ×3 and 2 companion `UnknownAlias`) |
| added: violation | **0** | |

**Separated per `POISON-UNMASK-ON-ADMISSION`.** Refusals that disappeared
through an edge becoming infeasible, rather than through the access being
modelled, number **4** leaves in one function (OffsetFetch: L21154, L21153,
and L21204 ×2). Removals by binding precision number 7, and removals by new
summaries number 18 (16 call leaves and 2 alias leaves). Removals by the rule
itself number 1,083.

### 4.3 The leak pin

**272 → 271 (constraint 7).** The withdrawn leaf is
`rd_kafka_snappy_java_uncompress`, `snappy.c:1694`:

```c
if (outbuf) rd_free(outbuf);
return NULL;
```

At BEFORE, `*outlenp = uof;` (snappy.c:1717, a `size_t *` formal) was refused
as an unsanctioned out-slot store. `reject_unsanctioned_store` poisons **every**
binding, so on the loop's second pass `outbuf` was unknown. The release at
1693 was then refused (`UnknownAlias outbuf`, removed too), and the site was
reported leaked at the `return NULL`. At AFTER the store is admitted through
an unbound base with the constraint-10a frame. `outbuf` is an automatic local
that is never address-taken, so it is spared, the release is credited, and no
leak is reported there.

The source confirms `outbuf` is freed on that path. This is a **withdrawal of
a false leak by the discharging arm**. It rests on the frame premise
constraint 10a already carries, applied to a statement that was blanket-poisoned
before. It is the `POISON-UNMASK-ON-ADMISSION` hazard moving in the precise
direction. The function keeps four other leaks and stays provisional
`Unsafe`, and no function or unit status moves because of it.

### 4.4 Movers

Every mover is **Safe relative to the declared premises**, which is
invariant 2's reading, not a proof of the C. The unit move (`cmt_time.c`, all
of whose projected functions are now Safe) is a function-level verdict
roll-up. It is not a conformance promotion.

| Function | Unit | BEFORE blocker | Forecast |
|---|---|---|---|
| `cio_chunk_counter_total_add` / `_sub` / `_up_add` / `_up_sub` | `cio_chunk.c` | `ctx->n++` (increment) | yes |
| `set_result` | `cmt_encode_opentelemetry_batch.c` | `*result = value` (store-through) | yes |
| `empty_batches_create` | same | calls to `set_result` (transitive summary) | yes |
| `cmt_time_from_ns` | `cmt_time.c` | `tm->tv_nsec = ns - (tm->tv_sec * 1000000000L)` | **no**: census window |
| `flb_input_chunk_output_would_exceed_limit` | `flb_input_chunk.c` | two `*available_space = …` | yes |
| `rd_kafka_ConsumerGroupDescription_authorized_operations`, `rd_kafka_TopicDescription_authorized_operations` | `rdkafka_admin.c` | `*cntp = RD_MAX(grpdesc->cnt, 0)` | yes |
| `rd_bytes_hash` | `rdmap.c` | `hash = … + bytes[i]` | yes |
| `mk_string_dup` | `mk_core/mk_string.c` | `p[len] = '\0'` after `p = mk_mem_alloc(…); if (!p) return NULL;` | yes |

`mk_string_dup` is the brief's buffer case end to end. The code allocates,
null-tests, fills through a subscript and returns. The case would be
provisional `Unsafe` had the store followed a release.

### 4.5 Prediction against outcome

| Forecast | Measured | |
|---|---|---|
| 985 leaves leave their families (family split above) | 985 in exactly that split, plus 98 the mirror missed | hit; the miss is the census window |
| ≤ 70 exchanges | 78 | miss by 8 |
| 11 Unknown → Safe, named | the 11 named, plus `cmt_time_from_ns` | hit plus one; census window |
| 0 unit moves | 1 (`cmt_time.c`) | miss; same cause |
| leak 272 | 271 | miss; §4.3 |
| 0 violation candidates | 0 | hit |

The frozen file is not edited, and these corrections live here.

### 4.6 The census-2 proxy against an admission

This is `INDIRECT-EXCHANGE-PROXY-UNVALIDATED`.
[`scripts/proxy_validation.py`](scripts/proxy_validation.py)
([proxy-validation.txt](proxy-validation.txt)) recomputes
C-GATEWAY-INDIRECT-CENSUS-2's three-way prediction on this baseline. It does
so for the 17 census shapes this card's rule touches, and compares the result
with the measured AFTER:

* **CLEAR predictions.** Of 11 functions the proxy calls CLEAR, **4 measured
  cleared**. The other 7 kept a leaf only because another statement of the
  same census shape lies outside the typed rule. The census shape is coarser
  than the rule. No CLEAR prediction was wrong where the rule covered the
  whole shape.
* **EXCH predictions.** The 4 EXCH predictions kept a leaf for the same
  reason, so the exchange arm stays unmeasured
  (`INDIRECT-EXCHANGE-PROXY-EXCH-ARM-UNTESTED`).

This card's own proxy (the forecast mirror) was validated leaf by leaf:
985/985 forecast leaves were removed, and the misses are listed in §4.5.

## 5. Validation

The gate was measured in this worktree (`agent-a8d24f7f717a987c1`) by
[`scripts/run_gate.py`](scripts/run_gate.py). It is W43-LIBC-RESULT-ALIAS's
runner plus a `census` step and a `docsindex` step. The runner sets
`CARGO_BUILD_JOBS=6` and `RUST_TEST_THREADS=2`, turns debug info off, and
captures every exit to a file, never through a pipe.

The core and provider gates ran on `d975ffc6`, the implementation commit.
Later commits change documents, the ledger, report scripts and evidence only.
Logs are worktree-private under `private/gate/` and `private/provider-gate/`.
The generator input is [gate.json](gate.json), with the
[Clang recording](clang-version.txt) taken in this checkout beside it.

| Command | Exit | Result |
|---|---:|---|
| `git submodule update --init --recursive` | 0 | libevent `d82464a2…` |
| `clang --version` | 0 | Ubuntu clang 21.1.8 |
| `cargo test --workspace --no-fail-fast` | 0 | **5,222 passed / 0 failed / 11 ignored**, 277 summaries |
| `cargo fmt --all -- --check` | 0 | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | |
| `python3 scripts/check_lean_audits.py --jobs 4` | 0 | **171** audits |
| `cargo clippy -p qcore-c-gateway --features indirect-census --all-targets -- -D warnings` and `cargo test -p qcore-c-gateway --features indirect-census --lib` | 0 | clean; 428 passed |
| `python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/w43-indirect-access-general/gate.json` | 0 | 306 follow-ups, 107 contract modules, 171 Lean audits |
| `python3 -m unittest discover -s scripts/tests` (after the final regeneration) | 0 | **327** tests |
| `python3 scripts/build_docs_index.py`, then `--check` | 0 / 0 | regenerated and matching |
| `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline --no-fail-fast` | 0 | **109 passed / 0 failed / 0 ignored**, 39 summaries |
| `… -p qcore-provider-conformance --test packages real_packages_preserve_unknown_and_reject_mutations … -- --exact --test-threads=1` | 0 | 1 passed (isolated, constraint 16) |
| `cargo fmt --manifest-path providers/Cargo.toml --all -- --check` | 0 | clean |
| `cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings` | 0 | clean |
| `python3 providers/contracts.py check` | 0 | 72 provider-owned |
| `python3 providers/package.py build --output private/provider-gate/packages` | 0 | 5 packages |
| `python3 providers/package.py verify …/<id>` (×5) | 0 ×5 | chunkio, cmetrics, ctraces, libevent, librdkafka |
| `python3 -m unittest discover -s providers/tests` | 0 | 5 tests |

**Counts.** The card adds these tests:

* the 23-test `indirect_scalar_access.rs` binary;
* one test each in `scalar_member_store.rs` and `typed_field_access.rs`;
* one Lean audit.

It narrows two pins (§2.4). The base's own gate figures at `d3cd8d81` were not
remeasured here, so the delta is stated by test, not by subtraction.

## 6. Ledger

**Retired**, each verbatim with its ground in
[retired-followups.json](retired-followups.json):

* `INDIRECT-CENSUS-FEATURE-UNBUILT` and
  `BASE-TYPE-READER-CENSUS-FEATURE-UNBUILT`. The CI step of §1.1 is exactly
  their discharge condition.
* `INDIRECT-EXCHANGE-PROXY-UNVALIDATED`, measured in §4.6 and narrowed.

**Registered** ([followups.json](followups.json)). Each refused shape carries
its size at the AFTER census ([residuals.txt](residuals.txt)), counted as
leaves / Unknown functions / sole-blocked:

| Id | Size | What stays refused |
|---|---|---|
| `W43-INDIRECT-CALL-ARGUMENT-ACCESS` | 9,539 / 2,408 / 242 | an access in a statement carrying a call |
| `W43-INDIRECT-CHAIN-ACCESS` | 5,616 / 2,057 / 188 | member chains of depth ≥ 2 |
| `W43-INDIRECT-UNTYPED-SINGLE-ACCESS` | 3,136 / 1,212 / 94 | single accesses whose base or member the reader does not classify |
| `W43-INDIRECT-POINTER-VALUE-STORE` | 306 / 207 / 18 | `p->f = q`, `*pp = q` |
| `W43-INDIRECT-POINTER-INCREMENT` | 335 / 172 / 0 | `*p++ = c`, `*(p + i) = c` |
| `W43-INDIRECT-OTHER-STATEMENTS` | 1,693 / 764 / 25 | everything else, uncensused `for` clauses included |
| `W43-INDIRECT-PROTECTED-FORMAL-INCREMENT` | 15 leaves / 9 functions | typed statements the older formal-mutation refusal blocks first |

The remaining registrations:

* `W43-INDIRECT-ACCUSING-UNEXERCISED-IN-CORPUS`;
* `W43-INDIRECT-BYTE-STORE-REPRESENTATION-PREMISE`;
* `W43-INDIRECT-CONST-DROPPED-WRITE`, which records that
  W43-LIBC-RESULT-ALIAS §1.3's sentence "a direct write through a
  const-dropped pointer is already refused" no longer holds;
* `INDIRECT-EXCHANGE-PROXY-EXCH-ARM-UNTESTED`;
* `W43-INDIRECT-FORECAST-CENSUS-WINDOW`.

**Noted, not retired:**

* `POISON-UNMASK-ON-ADMISSION` is measured for this card's admitted set (§4.2).
  It stays registered because the hazard applies to every later admission.
* `W43-RESULT-ALIAS-ACCUSING-UNEXERCISED-IN-CORPUS` is unchanged.

Follow-ups go from **297 to 306**. `corpus.cycle19_raw_units` points at this
report's artifacts. Its previous contents are in
[previous-corpus-row.json](previous-corpus-row.json).

## 7. Attempts that are not the evidence

* **Census runs.** `census-base` (texts only) and `census-base2` (first
  statement stream) were superseded as labels were added. `census-base4` and
  `census-base5` were stopped by their task handles, and both directories were
  set aside as `aborted-*`. `census-base3` and `census-base6` completed; the
  latter is the census. All kept privately under `private/runs/`.
* **AFTER on `29aca41d`.** It is identical to AFTER except for the `scope`
  string (§4.1).
* **The first test run of the new binary** failed 6 of 20 tests. Two causes
  were real:
  * the frame computed with `may_write_names` poisoned the dereferenced
    pointer itself (`* p =` reads as a write of `p`). The frame is now built
    from the parsed statement, and the fixed behaviour is pinned by
    `filling_a_guarded_allocation_certifies`;
  * the older formal-mutation refusal blocked increments of protected names
    (kept, and registered).

  The rest were fixture expectations corrected to the full vector.
* **The first gateway suite after the rule** failed the two pins of §2.4.
  They were narrowed deliberately.

## 8. For the integrator

* **Proposed gateway amendment**, for a new constraint 5h beside 5g, in these
  words:

  > W43-INDIRECT-ACCESS-GENERAL adds one route consulted only after the
  > existing classifier refused a statement for its access, destination or
  > increment, or named an unsanctioned `*p` store: a statement whose every
  > memory access is `*B`/`B[I]` over a declared `SCALAR *B` or
  > `SCALAR B[N]`, `B->F` over a declared record pointer and scalar member, or
  > `S.F` over a declared record object, whose destination and value are
  > scalar, with no call, unary `&`, comma, nested write or pointer increment,
  > lowers to `IndirectScalarAccess`. It is custody-neutral. Each dereferenced
  > base must be unbound on every reaching path, a non-null binding with no
  > tracked target, or exactly one known non-null live identity (derived
  > bindings included), else `UnknownAlias`; a base released on a reaching
  > path is a `use_after_release` candidate from the settled states. A write
  > through `*B`/`B[I]` keeps the refused statement's constraint-10a frame,
  > computed from the parsed statement. Bounds, alignment, constness and
  > untracked objects' liveness are outside the property. A statement the
  > older classifier admits keeps its lowering.

* **Constraint 7:** record the move 272 → 271 as a withdrawal by the
  discharging arm, attributed in §4.3.
* **Files likely to conflict with main:**
  * `crates/qcore-c-gateway/src/legacy_c.rs`: `LegacyInstruction`, the
    `classify_simple` wrapper, `transfer`, `rebound_names`, the post-passes.
    It conflicts if another card edits those, as CONTRACT-INHERIT-ADOPTION
    does not;
  * `crates/qcore-c-gateway/src/typed_field_access.rs`: the declarator reader
    signatures and `FunctionTypes`;
  * `crates/qcore-c-gateway/src/indirect_census.rs`;
  * `crates/qcore-audit/src/report.rs`: the `scope` string, which changes
    every future diagnostics artifact;
  * `compiler/examples/alias_aggregate_probe.rs`;
  * `.github/workflows/ci.yml`;
  * `proof/QCore.lean`;
  * `docs/QCORE_SPEC_r0_3.md` §L18;
  * the shared records: `docs/reviews/r0_3/acceptance_matrix.json`,
    `docs/STATUS.md`, `docs/development/increments/README.md` and the new
    `wave-43.md`, and `docs/reports/INDEX.md`.
* **The concurrent CONTRACT-INHERIT-ADOPTION card** changes contract manifests,
  not verdicts. The combined tree must be re-arbitrated against this card's
  artifacts. Its contracts artifact will differ.
* The `contracts` and `source-effects` artifacts embed this worktree's path, as
  earlier isolated cards' artifacts did.

## Reproduction

```sh
python3 docs/reports/w43-indirect-access-general/scripts/run_audit.py <before> d3cd8d81 --binary <base binary>
python3 docs/reports/w43-indirect-access-general/scripts/run_audit.py <census> d3cd8d81+census --census --binary <census binary>
python3 docs/reports/w43-indirect-access-general/scripts/shape_census.py <census> shape-census.json shape-census.txt
python3 docs/reports/w43-indirect-access-general/scripts/predict.py <census> predict.json predict.txt [--all-statuses]
python3 docs/reports/w43-indirect-access-general/scripts/run_audit.py <after> d975ffc6 --binary <after binary>
python3 docs/reports/w43-indirect-access-general/scripts/attribute.py <before> <after> <census> attribution.json attribution.txt
python3 docs/reports/w43-indirect-access-general/scripts/unforecast.py attribution.json <census> unforecast.txt
python3 docs/reports/w43-indirect-access-general/scripts/proxy_validation.py <census> <after> proxy-validation.json proxy-validation.txt
python3 docs/reports/w43-indirect-access-general/scripts/run_audit.py <census-after> d975ffc6+census --census --binary <after census binary>
python3 docs/reports/w43-indirect-access-general/scripts/residuals.py <census-after> residuals.json residuals.txt
python3 docs/reports/w43-indirect-access-general/scripts/formal_mutation.py <census-after>
python3 docs/reports/w43-libc-contracts/scripts/archive.py <after> docs/reports/w43-indirect-access-general/artifacts
python3 docs/plans/cycle19-custody-cards/blocker_census.py docs/reports/w43-indirect-access-general/artifacts
python3 docs/reports/w43-indirect-access-general/scripts/update_ledger.py
python3 docs/reports/w43-indirect-access-general/scripts/run_gate.py <gate-dir> submodules clang test fmt clippy probe followup lean census
```

The census streams are 5–14 MB each and are not committed. Their SHA-256
digests are as follows:

* base statement stream `34da9251…`;
* base indirect stream `1fea4329…`;
* AFTER statement stream `74a01b7e…`;
* AFTER indirect stream `9f01ed48…`.
