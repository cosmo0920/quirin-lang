# W44-CORPUS-CALLEE-SUMMARIES: why corpus callees get no summary, and the one route gap closed

Card W44-CORPUS-CALLEE-SUMMARIES, 2026-09-25, on base `c1e69bb5` ("Open the
wave-44 increment record"). `call to uncontracted function` is the largest sole
blocker at the baseline: **9,547 leaves / 2,591 Unknown functions / 407 blocked
only by uncontracted calls**. W43-LIBC-CONTRACTS left the 2,462 corpus-defined
callees uncontracted on purpose, because they should be verified, not assumed.
This card measures why each one gets no validated W4 summary, and fixes the
one cause that is a route gap rather than an unmodeled body.

## Headline

* **Census first** ([PREDICTION.md](PREDICTION.md), [census/](census/)). Every
  leaf is classified by where the callee's body is, seen from the calling unit.
  The dominant causes are **not** route gaps. The callee's own body does not
  validate for 3,594 + 58 leaves (same unit), 2,871 leaves (another unit) and
  1,257 leaves (a header `static` body). 950 leaves have no body in any pinned
  unit (the cfl library, builtins, OpenSSL), and 586 are the glibc `__bswap_*`
  bodies owned by W44-LIBC-RESIDUALS.
* **The route gap.** Under main-source selection, W4 derived summaries only for
  the definitions a unit *reports*. A `static inline` helper defined by a header
  in the translation unit is the body every call to it runs, yet it was never
  offered to the derivation. **41 leaves** (16 callees, 2 sole) sit on helpers
  whose bodies validate. They are the only population a summary route can
  reach today.
* **The fix.** The derivation scope (not the reported scope) now includes every
  unselected definition the selection reaches through direct calls that is
  `static` and not `extern` on its own definition, and whose name is not
  reserved in every context. The summary still passes the unchanged
  `observations_match_contract` gate; declared contracts still win.
  New module `crates/qcore-c-gateway/src/legacy_callee_scope.rs`, two thin
  hooks in `legacy_c.rs`. Lean: `QCore.CalleeSummaryScope` (11 audited
  theorems, `AuditCalleeSummaryScope.lean`). Rule: gateway constraint 4a.
* **Arbiter.** BEFORE reproduces the baseline diagnostics **byte for byte**.
  AFTER changes **70 leaves, every one attributed**. **2 functions move
  Unknown → Safe** (`mk_event_init`, `mk_event_backend` in
  `monkey-libevent/mk_core/mk_event.c`), as predicted. No other function or
  unit moves. The leak pin holds at **271**, and no violation leaf is added or
  removed.
* **Not admitted, and why.** A "releases its formal and nothing else"
  destructor summary cannot be proved of a body with an unmodeled statement.
  Only 8 of 230 destructor-shaped callees have a body blocked by calls alone. A
  cross-unit route would reach 104 leaves at most. Both are registered with
  their sizes, as are three other causes (six follow-ups, 345 → **351**).

| Measure | BEFORE (baseline) | AFTER | Delta |
|---|---:|---:|---:|
| Functions Safe / provisional Unsafe / Unknown | 294 / 85 / 4,455 | **296 / 85 / 4,453** | +2 / 0 / −2 |
| Units Safe / provisional Unsafe / Unknown / data-only | 3 / 43 / 135 / 4 | 3 / 43 / 135 / 4 | 0 |
| `unsupported` | 45,848 | **45,795** | −53 |
| `unknown_alias` | 4,358 | **4,371** | +13 |
| `leak` / `summary_conflict` | 271 / 1 | 271 / 1 | 0 |
| `double_release` / `invalid_release` / `use_after_release` | 0 / 0 / 0 | 0 / 0 / 0 | 0 |
| uncontracted-call leaves / Unknown functions / only-calls blocked | 9,547 / 2,591 / 407 | 9,505 / 2,579 / 406 | −42 / −12 / −1 |
| Unknown functions blocked by exactly one uncontracted callee | 302 | 300 | −2 |

## 1. Census: why no summary reaches each call

**Method.** A private run of the base binary through a transparent Clang
wrapper ([`scripts/clang-tee.sh`](scripts/clang-tee.sh),
[`scripts/capture_run.sh`](scripts/capture_run.sh)) saved every unit's
line-marked and `-P` preprocessing. Its diagnostics are byte-identical to BEFORE.
[`scripts/definition_sites.py`](scripts/definition_sites.py) joins each
analysed unit to its capture and records every file-scope definition in the
translation unit: its file, whether that is a system header, and its `static`,
`inline` and `extern` specifiers. 180 of 185 units join; the other 5 have no
analysed rows. [`scripts/callee_census.py`](scripts/callee_census.py) then
classifies each leaf by where the callee's body is.

Whether a header body *would* validate is not guessed. The census build
(`--features qcore-c-gateway/indirect-census`, `QCORE_CALLEE_SCOPE_CENSUS`, the
commit `ebf7115d` instrumentation) re-runs `ModeGate::apply`, and with it the
W4 derivation, over the widened scope on a private copy of the unit. For every
reached unselected definition it records the decision (`validated`,
`no_summary`, a declared outcome, or `refused_linkage`) and the body's own
status and blockers. Nothing it computes reaches the analysis, and its
diagnostics are byte-identical to BEFORE.

| Class (baseline) | Leaves | Callees | Unknown fns | Sole | Why no summary |
|---|---:|---:|---:|---:|---|
| same unit, selected definition, body `Unknown` | 3,594 | 1,566 | 1,633 | 219 | its own body does not validate (constraint 4) |
| other pinned unit, body `Unknown` | 2,841 | 695 | 1,005 | 36 | no cross-unit route, and the body would not validate |
| same unit, header `static` definition, body does not validate | 1,257 | 155 | 457 | 12 | route gap **and** unmodeled body |
| no body in any pinned unit | 950 | 217 | 361 | 23 | only a declaration can decide it |
| same unit, glibc header body (`__bswap_*`, `__uint64_identity`) | 586 | 4 | 97 | 3 | owned by W44-LIBC-RESIDUALS |
| other pinned unit, body `Safe` | 104 | 35 | 88 | 2 | no cross-unit route |
| same unit, header definition of external linkage | 86 | 16 | 39 | 0 | the call may run another body (C11 6.7.4p7) |
| same unit, selected definition, body provisional `Unsafe` | 58 | 38 | 30 | 4 | a defective body validates nothing |
| **same unit, header `static` definition, body validates** | **41** | **16** | **38** | **2** | **route gap only: this card** |
| other pinned unit, body provisional `Unsafe` | 30 | 19 | 17 | 1 | as above |

The brief's top ten, by this classification:

| Callee | Leaves | Class | What blocks it |
|---|---:|---|---|
| `__bswap_32` / `__bswap_16` | 279 / 247 | glibc header body | W44-LIBC-RESIDUALS (see §5) |
| `cfl_sds_cat_safe` | 170 | no pinned body | cfl is not a pinned unit; a declaration is the only route |
| `rd_kafka_buf_write_i32` | 143 | header `static`, no summary | its body calls `__bswap_32` and `rd_kafka_buf_write`, which in turn reaches `rd_buf_write` in `rdbuf.c` |
| `rd_list_destroy` | 129 | other unit (1 same-unit leaf) | its own body: `rd_list_destroy_elems` uncontracted, a branch condition, an unresolved identity |
| `rd_kafka_op_destroy` | 109 | other unit / same unit | 123 refusals of its own (59 indirect access, 49 conditions) |
| `rd_kafka_topic_partition_list_destroy` | 97 | other unit / same unit | indirect access, loop and branch conditions |
| `rd_kafka_buf_write_i8` | 96 | header `static`, no summary | `rd_kafka_buf_write` |
| `rd_kafka_op_new0` | 86 | other unit / same unit | indirect access, braced initializers |
| `ut_destroy_metadata` | 77 | other unit / same unit | indirect access, loop conditions, `rd_kafka_metadata_destroy` |

**The four suspected causes, measured.**

* *The callee's own body is `Unknown`.* This dominates: 7,780 leaves across
  the same-unit, other-unit and header classes (`Unknown` or provisional
  `Unsafe` bodies). Among the 1,395 callees called
  only from their own unit, the bodies carry indirect access (1,146), a
  branch or loop condition (884), further uncontracted calls (826) or an
  unresolved identity (444). Only 56 (205 leaves) are blocked by calls alone
  ([census/body-reasons.txt](census/body-reasons.txt)). Of the 1,257
  header-body leaves, 378 (20 callees) are blocked only by further calls
  ([census/header-reasons.json](census/header-reasons.json)).
* *A destructor shape the vocabulary cannot state.* 230 destructor-shaped
  callees (names reading `destroy`, `free`, `release`, `unref`, `clear`,
  `term`, `fini`, `cleanup`) own 1,175 leaves. Only 8 (53 leaves) have a body
  blocked by calls alone. The vocabulary already states a released formal; what
  is missing is a modeled body (§4).
* *Another translation unit.* 2,975 leaves, of which 104 have a `Safe` home
  body (§4).
* *A macro or inline.* Every glibc and project `static inline` body is in the
  unit and now in scope where reached. The `mpack_*` C99 `inline` definitions
  (86 leaves) are refused on linkage. Macros never reach the analysis as calls:
  `rd_kafka_buf_write_arraycnt_pos` expands to `rd_kafka_buf_write_i32`, and it
  is counted there.

## 2. The rule

[`legacy_callee_scope.rs`](../../../crates/qcore-c-gateway/src/legacy_callee_scope.rs)
adds `derivation_scope(tokens, selected, definitions, selection_applied)`.
Without a selection it returns the selection unchanged, because every
definition is then already in scope. With one, it returns the selection plus
every definition reached from it through `direct_calls`, transitively, that is:

1. **unselected**;
2. **of internal linkage on its own definition**: `static` present and `extern`
   absent among the tokens back to the previous `;`, `{` or `}`. The call then
   denotes this body and no other (C11 6.2.2p3). A definition whose `static` is
   spelled only on an earlier declaration is answered "not established", which
   is the refusing direction;
3. **not reserved in every context**: not `__x` and not `_X` (C11 7.1.3). This
   keeps glibc header bodies with the libc provider. `_mk_event_init` (one
   underscore, then lowercase) is reserved only at file scope, is corpus code,
   and is admitted.

Two hooks in `legacy_c.rs` use it: `analyze_legacy_c_translation_unit_selected_partial`
passes the scope to `ModeGate::apply`, and so to `derive_custody_summaries`,
and `recursive_component_census` does the same. The analysed and reported rows,
the effective entry and the whole-unit `Safe` scope remain `names`, the
selection. Everything else is unchanged:

* an admitted helper publishes a summary only through
  `observations_match_contract`, against a fresh analysis of its own body in
  this unit;
* a declared contract still wins, and a disagreement is still recorded;
* the declared-mode gate compares exactly as it does for a selected definition.
  A reached `static` definition that contradicts a declaration now withdraws it
  in that unit, the refusing direction. This is what
  `LOADER-ABI-MODES-SELECTION` anticipated. In the corpus every compared header
  row is the known `rd_free`/`mk_mem_free`/`flb_free` nullability refinement,
  and nothing is withdrawn;
* a `SummaryConflict` for an unreported helper emits no diagnostic, because
  diagnostics are per reported row; `summary_conflict` stays at 1;
* the certificate checker re-analyses through the same function with the same
  selection, so it re-derives this scope and these summaries. The three `Safe`
  units re-certify.

The composition invariants of the brief are untouched: no new
`LegacyInstruction` variant, no change to `Derivation::Derived`, the `B[X++]`
forget rule, or descriptor custody. A helper's summary is the one the ordinary
W4 inference derives, including its heap-only kind rule
(`W43-DESCRIPTOR-W4-SUMMARIES`).

**Tests** (`crates/qcore-c-gateway/tests/unit/legacy_callee_scope.rs`, 13):

* linkage-reader cases;
* scope reachability;
* the reserved-name split;
* end to end through the selected analysis:
  * a static helper is summarised and its caller certifies, with the
    helper's record in the unit's summaries;
  * a `Borrowed` helper exposes the caller's leak (never weaker than the body);
  * a helper whose body double-releases leaves the call uncontracted;
  * an external `inline` helper and a reserved helper are refused;
  * a contradicting static definition withdraws the declaration;
  * without a selection the helper is a reported row, as before.

## 3. Lean parity

`proof/QCore/CalleeSummaryScope.lean` models definitions (name, linkage,
selected, reserved, calls), the reachability step and its iteration, call
resolution (`MayRun`), and environment installation. The theorems are audited
by `proof/AuditCalleeSummaryScope.lean`, with no `sorry` and no project axiom:

* `every_scope_member_is_selected_or_internal`,
  `an_unselected_external_definition_is_never_in_scope`,
  `an_unselected_reserved_definition_is_never_in_scope`,
  `the_selection_is_in_scope`;
* `an_admitted_callee_runs_only_its_analysed_body`: for an unselected scope
  member, every body a call in the unit may run is the analysed one. Its
  necessity witness is `an_external_definition_may_run_another_body`;
* `a_declaration_is_never_overridden`,
  `an_installed_inference_is_exactly_the_derived_contract` (callers get exactly
  the contract semantics a declaration of it has), and
  `an_installed_inference_was_validated_in_scope`;
* `an_existing_summary_survives_the_widening`: validation that refuses an
  uncontracted callee and reads only its callees' contracts derives every
  earlier summary unchanged under an extended environment, so widening only
  adds summaries;
* `the_witness_scope` (decided).

**What the Lean does not cover.** The Rust readers (`has_internal_linkage`,
`is_reserved_identifier`, `direct_calls`) are transcriptions of `candidate` and
`step`, maintained by review (trust boundary 28). Validation is taken as an
abstract predicate; its own soundness is `QCore.CustodySummary`'s. The
selected definitions' call-resolution premise is the pre-existing W4 premise
and is not changed here.

## 4. Arbiter

One command shape for every run:
[`../w43-return-expression/scripts/run_audit.py`](../w43-return-expression/scripts/run_audit.py),
with pinned Fluent Bit and Monkey inputs, `QCORE_CLANG=/usr/bin/clang` and the
wave-20 ChunkIO package.

| Run | Tree | Binary (sha256) | Exit | Record |
|---|---|---|---:|---|
| BEFORE | `c1e69bb5` (base) | see record | 0 | [before-run.json](before-run.json) |
| CAPTURE | `c1e69bb5`, Clang tee wrapper | same binary | 0 | private; diagnostics identical to BEFORE |
| CENSUS (unreserved) | instrumentation without the reserved filter | see record | 0 | [census-unreserved-run.json](census-unreserved-run.json) |
| CENSUS | `ebf7115d` + `indirect-census` | see record | 0 | [census-run.json](census-run.json) |
| AFTER | `4006c7b6` (implementation) | see record | 0 | [after-run.json](after-run.json), [artifacts/run.json](artifacts/run.json) |

**BEFORE vs the baseline**
(`docs/reports/w43-indexed-resource-collections/integration/artifacts/`,
[before-identity.json](before-identity.json), with `--root-alias` onto the
integrator worktree):

* diagnostics, chunk-contracts and source are **byte-identical**;
* `contracts` is identical after the root alias;
* `source-effects` differs only in the brief's checkout-path digests:
  `resource_contracts:<digest>`, `build_digest`, `configuration_digest`,
  `task_digest`, `obligation_id` and the `boolean-field-import` digest.

**Both censuses vs BEFORE:** diagnostics byte-identical, so the sink perturbs
no decision.

**AFTER vs BEFORE:** chunk-contracts, contracts, source-effects and source are
byte-identical. The diagnostics differ in 70 leaves, and in the per-unit counts
and reason strings that carry them. AFTER is archived as the new baseline under
[artifacts/](artifacts/); the W43-INDEXED-RESOURCE-COLLECTIONS integration
artifacts are its frozen predecessor.

### 4.1 Attribution, leaf by leaf

[`scripts/attribute.py`](scripts/attribute.py) assigns every changed leaf to
the first class that explains it. It exits nonzero if any leaf is
unattributed, and it exits 0. The full list is in
[attribution.json](attribution.json).

| Class | Removed | Added | What it is |
|---|---:|---:|---|
| `decided-callee` | 42 | 0 | a `call to uncontracted function` leaf naming a callee that validated in that unit |
| `same-line` | 0 | 13 | the call is now modeled and checks its argument: `unresolved resource identity` at a `Released{nonnull}` (`rd_kafkap_str_destroy` 8, `rd_kafkap_bytes_destroy` 4) or `Owned{nonnull}` (`mk_sched_conn_timeout_add` 1) position whose actual has no resolved identity |
| `decided-statement` | 11 | 0 | a refusal on a statement that calls a newly validated callee, which BEFORE refused before reaching the call: 8 branch conditions (`rd_timeout_expired` 7, `rd_dbl_eq0` 1) and 3 indirect-access statements whose call now admits its member-address argument (`crc_finalize(cf->crc_cur)`, `mk_iov_init(&header->headers_iov, …)`, `mk_static_plugins(&server->plugins)`) |
| `decided-function` | 2 | 2 | same-line re-attribution in a function with a decided call: `rdkafka_cgrp.c:24902` (`call to uncontracted function` becomes `unmodeled expression may mutate a formal resource identity`) and `rdkafka_request.c:21799` (`indirect memory access` becomes the same). The decided `Released` call makes the name a resource-bearing one, so the earlier refused write is reported as such |
| `transitive` / `unattributed` | 0 | 0 | — |

The 16 callees whose validated summaries decide the 42 leaves are:

* `Borrowed`/`None`: `rd_kafka_message2msg` 10, `rd_kafka_q_keep` 3,
  `rd_crc32_finalize` 2, `rd_kafka_q_disable0` 2,
  `rd_kafka_q_can_contain_fetched_msgs` 2, `crc_finalize` 2, `rd_timeout_init0`,
  `rd_kafka_headers_serialized_size`, `crc_init`, `flb_task_acquire_lock`,
  `flb_task_release_lock`, `_mk_event_backend`, `_mk_event_init`, `mk_iov_init`;
* `Released{nonnull}`: `rd_kafkap_str_destroy` 8, `rd_kafkap_bytes_destroy` 4;
* `Owned{nonnull}, Borrowed`: `mk_sched_conn_timeout_add` 1, in the provisional
  `Unsafe` function `mk_sched_add_connection`, which stays `Unsafe` with its
  violation leaves unchanged.

### 4.2 Movers

* **Unknown → Safe:** `mk_event_init` and `mk_event_backend`
  (`monkey-libevent/mk_core/mk_event.c`). Each carried exactly one leaf, the
  call to `_mk_event_init` / `_mk_event_backend`, which are `static inline` bodies of
  the included libevent backend (`mk_event_libevent.c`) and validate as
  `() -> None`. Each is `Safe`
  **relative to the declared premises** the unit already rests on. It is not a
  proof of the C, and the unit stays Unknown with 12 other Unknown functions.
* **Transitive movers: 0.** No function's last blocker was a mover or a newly
  decided callee. Twelve Unknown functions lost every uncontracted-call leaf
  (2,591 → 2,579) but keep other refusals.
* **Units: 0 moves.** `cmt_time.c`, `ctr_version.c` and `tls/tls_stub.c`
  re-certify under the widened scope.
* **Violation candidates: none added or removed.** `leak` 271,
  `double_release`, `invalid_release` and `use_after_release` 0. The leak pin
  is re-verified, not re-derived, for constraint 7's chain.

### 4.3 Prediction vs outcome

| Predicted ([PREDICTION.md](PREDICTION.md)) | Measured |
|---|---|
| 16 validated callees decide 42 leaves (41 Unknown, 1 Unsafe fn) | exactly |
| the 13 effectful sites removed or replaced on the same line; `unknown_alias` +0 to +13 | all 13 replaced by `unresolved resource identity`: **+13** |
| 2 movers Unknown → Safe (`mk_event_init`, `mk_event_backend`); 0 transitive | exactly |
| units unchanged; leak 271; `summary_conflict` 1; no mode withdrawal; `__bswap_*` 586 unchanged | exactly |
| `unsupported` 45,806 "plus any same-line replacement" | **45,795**. The census predicted leaves by name only. 11 refusals on statements that call a decided callee before reaching it (§4.1, `decided-statement`) were not predicted, nor were the 2 same-function re-attributions |
| three `Safe` units re-certify | exactly |

## 5. Hand-offs to concurrent cards

* **W44-LIBC-RESIDUALS (`__bswap_*`).** The reserved filter is the only thing
  keeping those bodies out. The census run without it
  ([census/prediction-unreserved.json](census/prediction-unreserved.json),
  [census/scope-census-unreserved.jsonl.gz](census/scope-census-unreserved.jsonl.gz))
  measured that `__bswap_16/32/64` and `__uint64_identity` validate
  `Borrowed -> None`. Admitted, they would decide 599 leaves (586 in Unknown
  functions) and move `ut_put_be16/32/64` (`rdunittest_msgset_errors.c`) to
  `Safe`. This refines `W43-LIBC-HEADER-INLINE-BODIES`: its discharge asks for
  `__builtin_bswap*` modeling, but the bodies already validate. Registered as
  `W44-CALLEE-SCOPE-RESERVED-BODIES`.
* **W44-BRANCH-CONDITION-CALLS** consumes these summaries through the normal
  contract lookup. The 8 branch-condition refusals removed here (§4.1) are
  conditions calling a newly validated helper that the **existing** condition
  route admits once the callee is contracted. If that card widens the route,
  its arbiter sees this card's helpers as contracted callees.
* **W44-INDIRECT-CALL-ARGUMENT-ACCESS.** The 3 indirect-access removals are
  existing argument admissions for contracted calls, which the new summaries
  make reachable.

## 6. Validation

The core gate ran on the implementation commit `4006c7b6`. Later commits
change documents, the ledger, report scripts and evidence only. The generator
input is [gate.json](gate.json), with the [Clang recording](clang-version.txt)
taken in this checkout beside it. Logs are worktree-private under
`measure/gate/` and `measure/provider-gate/`.

| Command | Exit | Result | Log |
|---|---:|---|---|
| `git submodule update --init --recursive` | 0 | libevent at `d82464a2…` | session output (fresh worktree) |
| `clang --version` | 0 | Ubuntu clang 21.1.8 | [clang-version.txt](clang-version.txt) |
| `cargo test --workspace --no-fail-fast` (`CARGO_INCREMENTAL=0`) | 0 | **5,403 passed / 0 failed / 11 ignored**, 289 summaries | `measure/gate/cargo-test.log` |
| `cargo fmt --all -- --check` | 0 | clean | `measure/gate/fmt.log` |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | clean | `measure/gate/clippy.log` |
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | | `measure/gate/probe.log` |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | | `measure/gate/followup.log` |
| `cargo clippy -p qcore-c-gateway --all-targets --features indirect-census -- -D warnings` | 0 | clean (census code touched) | `measure/gate/census-clippy.log` |
| `cargo test -p qcore-c-gateway --features indirect-census --no-fail-fast` | 0 | 1,279 passed / 0 failed / 7 ignored, 58 summaries | `measure/gate/census-test.log` |
| `python3 scripts/check_lean_audits.py` | 0 | **178** audits | `measure/gate/lean.log` |
| `python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/w44-corpus-callee-summaries/gate.json` | 0 | 351 follow-ups, 111 contract modules, 178 Lean audits | `measure/build-matrix.log` |
| `python3 scripts/build_docs_index.py`, then `--check` | 0 / 0 | index regenerated and matching | `measure/docs-index-check.log` |
| `python3 -m unittest discover -s scripts/tests` (after the final regeneration) | 0 | **330** tests | `measure/python-final.log` |
| `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline --no-fail-fast` | 0 | **124 passed / 0 failed / 0 ignored**, 39 summaries | `measure/provider-gate/ptest.log` |
| `… -p qcore-provider-conformance --test packages real_packages_preserve_unknown_and_reject_mutations -- --exact --test-threads=1` | 0 | 1 passed (isolated) | `measure/provider-gate/pisolated.log` |
| `cargo fmt --manifest-path providers/Cargo.toml --all -- --check` | 0 | clean | `measure/provider-gate/pfmt.log` |
| `cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings` | 0 | clean | `measure/provider-gate/pclippy.log` |
| `python3 providers/contracts.py check` | 0 | unchanged | `measure/provider-gate/pcontracts.log` |
| `python3 providers/package.py build --output measure/provider-gate/packages` | 0 | 5 packages | `measure/provider-gate/pbuild.log` |
| `python3 providers/package.py verify …/<id>` (×5: chunkio, cmetrics, ctraces, libevent, librdkafka) | 0 ×5 | | `measure/provider-gate/pverify.log` |
| `python3 -m unittest discover -s providers/tests` | 0 | 13 tests | `measure/provider-gate/ppython.log` |

**Counts.** This card adds 13 Rust tests (the `legacy_c::callee_scope` unit
module) and one Lean audit. The base's own gate figures were not re-measured
here, so the delta is stated by test, not by subtraction.

**Runs that are not the gate, disclosed.** `cargo test` ran while the AFTER
corpus audit was running, which slowed both and changes neither. A first
Python run after the regeneration (`measure/python1.log`, 330 tests, exit 0)
fixed the count recorded in `gate.json`; the final run is the one above. The
unreserved census binary was built from the instrumentation before rule (3)
existed and is evidence only for §5.

## 7. Ledger and status

* **Retired: none.** No registered entry's discharge condition is met.
  * `W43-DESCRIPTOR-W4-SUMMARIES` asks for kinded inference and integer
    formal retention, which this card does not touch.
  * `W43-LIBC-HEADER-INLINE-BODIES` is W44-LIBC-RESIDUALS' decision (§5).
  * `CONTRACT-INHERIT-FLB-STRING-ASSUMPTIONS` is untriggered: no reached header
    row compares `flb_strdup`/`flb_strndup`.
  * `RETURN-CALL-W4-REACH-BLOCKED` asks for a body-refusal census of the wave-34
    `return CALL(...)` population, not of this family.
  * `LOADER-ABI-MODES-SELECTION` is a name mentioned inside
    `LOADER-ABI-MODES`' entry, never a registered id, so there is nothing to
    remove. §2 records its measured consequence.
* **Registered** ([followups.json](followups.json)), one per cause that stays
  unsummarizable:
  * `W44-CALLEE-BODY-DOES-NOT-VALIDATE`: 4,909 leaves;
  * `W44-CALLEE-CROSS-UNIT-SUMMARY`: 2,975 leaves, 104 reachable;
  * `W44-CALLEE-EXTERNAL-INLINE-DEFINITION`: 86 leaves;
  * `W44-CALLEE-NO-CORPUS-BODY`: 950 leaves;
  * `W44-CALLEE-DESTRUCTOR-SUMMARY`: 230 callees, 1,175 leaves;
  * `W44-CALLEE-SCOPE-RESERVED-BODIES`: 599 leaves, a hand-off.
* **Follow-ups 345 → 351.** `corpus.cycle19_raw_units` points at
  [artifacts/](artifacts/); its previous contents are in
  [previous-corpus-row.json](previous-corpus-row.json).
  [`scripts/update_ledger.py`](scripts/update_ledger.py) applied both edits.

## 8. For the integrator

* **Constraint 7.** The chain can record this card as leaving the leak pin at
  271: 2 functions moved, and no leak leaf was added or withdrawn.
* **Composition.** The widened scope adds contracts at call sites in other
  cards' territory: branch conditions, call arguments and indirect chains.
  Re-run the attribution
  (`scripts/attribute.py BEFORE AFTER census/scope-census.jsonl.gz OUT <capture>`)
  on the combined tree. A card that makes a header body validate, or a
  `__bswap_*` decision, moves this card's census.
* **Files likely to conflict:**
  * `crates/qcore-c-gateway/src/legacy_c.rs`: two commented hooks
    (`analyze_legacy_c_translation_unit_selected_partial` around the
    `ModeGate::apply` call, and `recursive_component_census`) and one
    `#[path]` module line just above the former;
  * `proof/QCore.lean`: one import line, after `DerivedResultInference`;
  * `docs/agents/gateway.md` and `docs/agents/README.md`: the new constraint
    4a and its router row;
  * `docs/QCORE_CONTRACT_SPEC_r0_3.md`: one paragraph after the W4
    invalidation paragraph;
  * `docs/reviews/r0_3/acceptance_matrix.json`, `docs/STATUS.md`,
    `docs/reports/INDEX.md` and `docs/development/increments/wave-44.md`.
* `contracts` artifacts embed the checkout path, as earlier isolated cards'
  artifacts did.

## Reproduction

```sh
python3 docs/reports/w43-return-expression/scripts/run_audit.py <before> c1e69bb5            # base binary
python3 docs/reports/w43-return-expression/scripts/baseline_identity.py <before> \
    docs/reports/w43-indexed-resource-collections/integration/artifacts \
    --root-alias <this worktree>=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/hungry-thompson-1740da
docs/reports/w44-corpus-callee-summaries/scripts/capture_run.sh <base binary> <capture>
python3 docs/reports/w44-corpus-callee-summaries/scripts/definition_sites.py <capture> <before>/resource-diagnostics.json <sites.json>
QCORE_CALLEE_SCOPE_CENSUS=<scope.jsonl> python3 docs/reports/w43-return-expression/scripts/run_audit.py <census> ebf7115d   # indirect-census build
python3 docs/reports/w44-corpus-callee-summaries/scripts/callee_census.py <before>/resource-diagnostics.json <sites.json> <census.json> <scope.jsonl>
python3 docs/reports/w44-corpus-callee-summaries/scripts/body_reasons.py <before>/resource-diagnostics.json <census.json> <reasons.json>
python3 docs/reports/w44-corpus-callee-summaries/scripts/header_reasons.py <census.json> <scope.jsonl>
python3 docs/reports/w44-corpus-callee-summaries/scripts/predict.py <before>/resource-diagnostics.json <scope.jsonl> <prediction.json>
python3 docs/reports/w43-return-expression/scripts/run_audit.py <after> 4006c7b6
python3 docs/reports/w44-corpus-callee-summaries/scripts/attribute.py <before>/resource-diagnostics.json \
    <after>/resource-diagnostics.json <scope.jsonl> <attribution.json> <capture>
```
