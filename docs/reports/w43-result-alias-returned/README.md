# W43-RESULT-ALIAS-RETURNED: inferred derived-result summaries

Card W43-RESULT-ALIAS-RETURNED, 2026-09-25, on base `f8b41209` (main after the
CONTRACT-INHERIT-ADOPTION integration).

Card W43-LIBC-RESULT-ALIAS gave the resource vocabulary a derived result. It
refused every escape of a derived pointer. A wrapper that returns `memcpy(…)`
or `strcasestr(…)` therefore stayed Unknown, and so did every call to it.

This card adds a W4 route that infers the summary such a declaration would
state. The route validates the summary before publishing it, and callers apply
it through the unchanged derived-result rules. The card also decides
`W43-RESULT-ALIAS-EXACT-RELEASE`: the refusal is kept, on a recorded ground.

It discharges `W43-RESULT-ALIAS-RETURNED`, retires
`W43-RESULT-ALIAS-EXACT-RELEASE` on its recorded ground, and registers one
residual.

## Headline

* **Rule** ([`legacy_derived_summary.rs`](../../../crates/qcore-c-gateway/src/legacy_derived_summary.rs)).
  A body whose every exit returns NULL or a pointer derived from **one**
  borrowed formal gets `ResourceReturnMode::Derived { parameter, extent,
  nullable, const_dropped }`. That is the record a `resource_return_alias` or
  `resource_return_interior` declaration loads to. The summary is published only
  after the ordinary W4 gate re-checks it. Contract spec
  [§C04.3.2](../../QCORE_CONTRACT_SPEC_r0_3.md) states the rule.
* **Lean** ([`QCore.DerivedResultInference`](../../../proof/QCore/DerivedResultInference.lean),
  [audit](../../../proof/AuditDerivedResultInference.lean), 23 theorems). It
  proves four things:
  * the gate is sound against the contract meaning;
  * every refused shape is refused;
  * the name-only base stays name-only under borrowing;
  * a caller's application is the declared `derive` instruction.

  The module is placeholder-free and uses no project axioms.
* **Prediction held exactly** ([PREDICTION.md](PREDICTION.md), frozen in
  `5644b955` before any analyzer change). **2 functions move Unknown → Safe**:
  `rdxxhash.c` `XXH_memcpy` and `mk_core/mk_string.c` `mk_string_casestr`. The
  delta is 2 return-expression leaves removed, 0 added, 0 unit moves, 0
  transitive movers and 0 violation candidates. Leak stays **272**.
* **Census** ([derived-census.jsonl](derived-census.jsonl)). The route examined
  **236** functions: 234 call a derived-result callee and return, but no exit
  returns such a call's result, and 2 were admitted. Every refused shape counts
  **0** in the corpus. Each shape is pinned by unit and end-to-end tests
  ([§4](#4-refused-shapes-with-their-measured-counts)).
* **Exact-alias release:** kept refused, on a recorded ground
  ([§5](#5-exact-alias-release-kept-refused)).

| Measure | BEFORE (W43-LIBC-RESULT-ALIAS baseline) | AFTER | Delta |
|---|---:|---:|---:|
| Functions Safe / provisional Unsafe / Unknown | 257 / 84 / 4,493 | **259 / 84 / 4,491** | +2 / 0 / −2 |
| Units Safe / provisional Unsafe / Unknown / data-only | 2 / 42 / 137 / 4 | 2 / 42 / 137 / 4 | 0 |
| `unsupported` | 48,930 | **48,928** | −2 |
| `unknown_alias` / `leak` / `summary_conflict` | 4,657 / 272 / 1 | 4,657 / 272 / 1 | 0 |
| `invalid_release` / `use_after_release` | 0 / 0 | 0 / 0 | 0 |
| return-expression leaves / functions / sole | 499 / 422 / 146 | **497 / 420 / 144** | −2 / −2 / −2 |

The census rows count Unknown functions, as [blocker-census.txt](blocker-census.txt)
and [sole-callees.txt](sole-callees.txt) do.

## 1. The rule

### 1.1 How a borrowed formal is tracked without being owned

An ordinary W4 entry does not bind a borrowed formal. A pointer derived from it
therefore has no target, and nothing records which formal it came from.
`EntryCustody::seed_derived_base` binds the formal to a **name-only site**
instead: a target that is not a resource. The site is never in
`state.resources`, which has three consequences:

* **Custody-bearing uses refuse.** Every custody-bearing use of the formal fails
  the live-identity check the transfer already applies, and refuses. That covers
  a release, a consumption, an escape (including `return k;`), an `Owned`
  position, an aggregate owner and a bound read.
* **Nothing is exempted** from leak reporting, because nothing is owned.
* **No path is deleted.** The binding may be null, so a null test keeps both
  edges.

One transfer path gives a status-free site a status without a diagnostic: a
nullable release (`free(k)` under `resource_released(p, nullable)`). A cascade
through an aggregate that held the formal would do the same. The exit
observation therefore gains `derived_base_touched`, which the gate requires to
be `false` at every exit. The Lean theorem
`a_release_reaching_the_base_is_seen` states the model's version: a release that
reaches the base either adds a diagnostic or gives the site a status. It is
never silent.

### 1.2 The derived CFG

`return f(…)` with a derived result is refused on every CFG the analyzer
decides a function on, exactly as before. The single exception is the CFG that
`build_derived_return_cfg` builds. Its parser flag `admit_derived_returns`
changes two things:

* it desugars a whole-call derived return, `PLACE = f(…); return PLACE;`, the
  way C-GATEWAY-RETURN-CALL-W4 desugars an owned one;
* it gives a null-constant `return` an `EscapeIfOwned` of the never-bindable
  place `NULL` (`return_call::NULL_RETURN_PLACE`), so `observe_exit` can report
  `ReturnedValue::Null` rather than an indistinguishable `None`.

That CFG is built only in two cases:

* for a candidate of the route;
* for a defined function whose entry contract in force is itself `Derived`.

Every other CFG is unchanged. The census hooks of the indirect-census feature
are silent on that parse, so a feature build records nothing twice.

### 1.3 The transfer and the observation

Under an entry with a derived base, `EscapeIfOwned` of a derived binding over
exactly that name-only site **escapes nothing**. The returned pointer owns
nothing, and the summary hands the caller the same non-owning relation the
callee's contract stated.

`observe_exit` reports such a binding as `ReturnedValue::Derived { site, extent,
nullable, const_dropped }`. Before this card it read `Ambiguous`. Every other
consumer treats the new variant as `Ambiguous`, and treats `Null` as `None`
(`propose_result`, `reallocation_exit`, and the `None`/`Owned` arms of
`result_matches`).

**Relative nullability.** `Derivation::Derived` gains a ghost `nullable` flag:
whether some derivation step may return NULL for a non-null base. It is kept
apart from `may_be_null`, which also carries the base's own nullness. It is
joined by disjunction and never narrowed, and no custody rule reads it. Without
it, every result derived from a nullable-seeded formal would read nullable, and
`XXH_memcpy` would lose its `nonnull`.

### 1.4 The route

The route is consulted only after the live-entry and removed-wrapper routes
publish nothing (`infer_validated_summary`). It runs these steps in order:

1. **Token pre-filter.** The body calls a callee whose contract in force has a
   derived result, and it contains `return`.
2. **CFG filter.** Some exit escapes a place that a derived-result call
   assigned. Failing this step is `no_derived_exit`.
3. **Base selection.** One run seeds every named, non-out formal with its own
   name-only site and reads which formals the derived exits target.
   * none → `no_formal_base`;
   * more than one → `several_formals`.

   This run only chooses the candidate formal. Nothing it observes is
   published.
4. **Least-binding search.** This is the ordinary search, with the chosen
   formal `k` seeded name-only and a permissive derived probe result. If the
   search wants to bind `k`, or any exit shows `derived_base_touched`, the
   outcome is `base_custody`.
5. **Clean run.** A run that is not clean is `mixed_exits` if some exit returns
   something other than NULL or a pointer derived from `k`, and
   `body_not_modeled` otherwise.
6. **Proposal.**
   * `extent` is `Exact` only if every derived exit is exact.
   * `nullable` is set if some exit returns NULL or some step may.
   * `const_dropped` is set if some step dropped it.
   * `k` is `Borrowed`, and every other formal's mode is read off as the
     ordinary search reads it.
7. **Fresh validation run.** It uses `observations_match_contract`, whose
   `result_matches` gains a derived arm. That arm accepts, at each exit:
   * NULL, when the contract is nullable;
   * a derived binding over `k`'s name-only site, whose extent, relative
     nullability and `const_dropped` the contract covers.

   It accepts nothing else, and no touched base.

`analyze_defined_function` uses the same entry and the same gate for a function
whose contract in force is `Derived`, whether declared or inferred. A body that
fails falls back to the ordinary CFG's contract-free analysis, exactly as any
other failed entry does.

### 1.5 The offset base

`derived_base` has always admitted `place ± one offset token` as a derived
base. `memcpy(p + 1, …)` returns `p + 1`, which is not `p`, so this card reads
such a base at the **interior** extent (`interior_extent` in the classifier).
Without that, an inferred summary could publish a false `Exact`.

Over a *tracked* resource, the pre-existing borrowed-expression gate refuses the
call first ("call has an unmodeled formal address or borrowed expression"), so
this reading is reachable only over an untracked base. That is exactly the
inference case. Its corpus effect is nil: all five artifacts are byte-identical
outside the two movers' leaves.

### 1.6 What callers see

The published record is a `ResourceReturnMode::Derived`, so a call to the
wrapper is classified and applied exactly as a call to a declared derived
callee. `tests/derived_result_returned.rs` checks this by **parity**. For each
of the following caller bodies, the vector through the wrapper equals the
vector with the declared callee substituted:

* use after the base is released: `unknown_alias` + `use_after_release` for the
  exact wrapper, `use_after_release` for the interior one;
* releasing an interior result: `invalid_release`;
* releasing an exact result: refused, `unknown_alias`;
* releasing the base once: `Safe`;
* forgetting the base: `leak`;
* null-testing the result: the base still leaks on the null edge.

## 2. Lean parity

[`QCore.DerivedResultInference`](../../../proof/QCore/DerivedResultInference.lean)
builds on `QCore.DerivedResult`.

**Model.**
* *Observations and proposal:* the exit observation (`Returned`, `Exit` with
  `baseTouched`), the summary (`Spec`), `result_matches`' derived arm
  (`exitMatches`), the gate (`validates`), and `propose` as a fold.
* *Concrete meaning:* pointer values are null or (object, offset). A spec
  *admits* `r` over base `b` when one of these holds:
  * `r` is null and either the base is null or the spec is nullable;
  * the spec is exact and `r = b`;
  * the spec is interior and `r` points into `b`'s object.

  An observation *denotes* the concrete results the analysis claims for it.

**Theorems** (23, audited by
[`AuditDerivedResultInference.lean`](../../../proof/AuditDerivedResultInference.lean)):

* **The gate is sound:** `an_accepted_exit_is_admitted`,
  `validation_is_sound`.
* **Every refused shape is refused, for every spec:** `a_mixed_exit_refuses`
  (no pointer, an identity site, an ambiguous binding), `another_base_refuses`,
  `a_touched_base_refuses`, `an_unclean_or_exitless_run_refuses`.
* **The proposal:**
  * it validates: `the_proposal_accepts_every_exit`,
    `the_proposal_validates`;
  * it is tight: `the_proposal_is_exact_iff_every_exit_is`;
  * it declines the refused shapes: `the_proposal_declines_a_mixed_exit`,
    `an_all_null_body_proposes_nothing`;
  * fold lemmas: `step_none`, `fold_some`.
* **The name-only base:** `borrowing_keeps_the_base_name_only` (derive, use and
  null test leave a status-free site status-free) and
  `a_release_reaching_the_base_is_seen`.
* **The offset base:** `an_offset_result_is_not_an_exact_alias`,
  `an_offset_result_is_interior`.
* **Callers apply the summary as a declaration:**
  `applying_the_summary_changes_no_status`, `a_wrapper_result_is_invalidated`
  (composing `QCore.DerivedResult.releasing_the_base_invalidates_every_derived_pointer`),
  `releasing_a_wrapper_result_never_releases_its_base`.
* **Witnesses:** `the_exact_wrapper_witness` (`XXH_memcpy`),
  `the_nullable_interior_wrapper_witness` (`mk_string_casestr`, which also
  rejects the two tighter specs), `the_mixed_exit_witness`.

The audit prints only `propext`, `Classical.choice` and `Quot.sound`. As with
every gateway module, the Rust–Lean correspondence is maintained by review, not
proved (trust boundary 28).

## 3. Arbiter

The runs use [`../w43-return-expression/scripts/run_audit.py`](../w43-return-expression/scripts/run_audit.py),
with the command shape, Clang and ChunkIO package of the baseline. Each run is
a release build of `cycle19_fluent_bit_libraries_audit`.

| Run | Commit | Binary (sha256) | Exit | Window (UTC) | Record |
|---|---|---|---:|---|---|
| BEFORE | `f8b41209` (base) | `1598eafb…` | 0 | 16:02:11–16:11:27 | [before-run.json](before-run.json) |
| AFTER | `f70e4be9` (implementation) | `eeb41791…` | 0 | 16:28:29–16:36:53 | [after-run.json](after-run.json) |
| CENSUS | `c7470e42` + `indirect-census` | `4a37105f…` | 0 | 16:49:09–16:57:11 | [census-run.json](census-run.json) |
| CONFIRM | `c7470e42` (final implementation) | `a497facf…` | 0 | 16:57:50–17:04:26 | [confirm-run.json](confirm-run.json), [artifacts/run.json](artifacts/run.json) |

**BEFORE vs the W43-LIBC-RESULT-ALIAS baseline**
([before-identity.json](before-identity.json)):

* diagnostics, chunk-contracts and source are **byte-identical**;
* `contracts` and `source-effects` differ in the checkout-path digests the brief
  lists, and in the CONTRACT-INHERIT-ADOPTION inclusion record that main already
  carries (confirmed verdict-neutral at that card's integration).

**AFTER vs BEFORE** ([delta.json](delta.json), [`scripts/delta.py`](scripts/delta.py)):

* chunk-contracts, contracts, source-effects and source are byte-identical;
* the diagnostics differ in exactly the following paths and nothing else:
  * the two functions' status and diagnostics;
  * their units' `diagnostic_counts.unsupported`, which is 326 → 325 for
    `rdxxhash.c` and 87 → 86 for `mk_core/mk_string.c`;
  * the `rdxxhash.c` unit's `reason` text, which carries that count;
  * the corpus `unsupported` total.

**CENSUS vs AFTER:** all five artifacts are byte-identical, so the feature's
sink perturbs no decision.

**CONFIRM vs AFTER:** all five artifacts are byte-identical. Between the two
binaries the card only split the census's not-a-candidate class, which
changes no decision. CONFIRM is the archived baseline
([artifacts/](artifacts/), [manifest.json](artifacts/manifest.json)).
`docs/reports/w43-libc-result-alias/artifacts/` is its frozen predecessor and
is not edited.

### 3.1 Attribution, leaf by leaf

| Removed leaf | Function | Unit | Its summary |
|---|---|---|---|
| `unmodeled call, aggregate or indirect access in return expression` at `rdxxhash.c:112` | `XXH_memcpy` | `rdxxhash.c` | `Derived { parameter: 0, extent: Exact, nullable: false, const_dropped: false }` |
| the same, at `mk_core/mk_string.c:316` | `mk_string_casestr` | `mk_core/mk_string.c` | `Derived { parameter: 0, extent: Interior, nullable: true, const_dropped: true }` |

No leaf was added. Each function carried exactly one leaf, so each moves
Unknown → Safe. Each is Safe **relative to the declared premises** (invariant 2's
reading, not a proof of the C): `memcpy` and `strcasestr` in
`providers/quirin-provider-libc`. No unit status moves: both units keep other
Unknown functions.

`mk_string_casestr` exercises the null-constant exit. Its
`if (!heystack || !needle) { return NULL; }` path contributes `Null`, which
makes the summary nullable, beside the derived exit.

### 3.2 Transitive movers

* **`XXH_memcpy`'s callers**, `XXH32_update_endian` and `XXH64_update_endian`
  (six call sites), do not move and none of their leaves changes. Every call's
  first argument is a member or a cast of one
  (`(BYTE*)(state->mem32) + state->memsize`, `state->mem32`), and those
  statements are refused by the indirect-access gate before any callee lookup.
  The functions carry 34 and 29 leaves. This matches the prediction, including
  the stated risk that did not materialize.
* **`mk_string_casestr`** has no caller in the corpus.

### 3.3 Prediction vs outcome

| Predicted ([PREDICTION.md](PREDICTION.md)) | Measured |
|---|---|
| `XXH_memcpy` → `Derived{0, Exact, nonnull, !const_dropped}`, Unknown → Safe | exactly |
| `mk_string_casestr` → `Derived{0, Interior, nullable, const_dropped}`, Unknown → Safe | exactly |
| 259 / 84 / 4,491 functions, units unchanged | exactly |
| `unsupported` −2; `unknown_alias`, `leak`, `summary_conflict` unchanged | exactly |
| no transitive movement; possible re-attribution on six lines | no movement and no re-attribution |
| 0 further movers through a name (`return p;`) | 0: the census found no such candidate |
| the offset-base interior reading moves nothing | nothing |
| exact-alias release kept refused | kept ([§5](#5-exact-alias-release-kept-refused)) |

## 4. Refused shapes, with their measured counts

The census comes from a `--features qcore-c-gateway/indirect-census` build,
using `QCORE_DERIVED_CENSUS` with one row per route decision:
[derived-census.jsonl](derived-census.jsonl). The route runs once per
(unit, function), and every one of the 236 rows has a distinct key.

| Shape | Census label | Corpus count | Pinned by |
|---|---|---:|---|
| admitted | `admitted` | **2** | both ledger shapes, in unit and end-to-end tests |
| calls a derived callee and returns, but no exit returns its result | `no_derived_exit` | **234** | `a_body_with_no_derived_return_is_not_a_candidate` |
| mixed exits: a fresh resource, an untracked pointer, an offset of the formal, or no pointer on some path | `mixed_exits` | 0 | `mixed_exits_are_named`, `mixed_exits_are_refused` |
| derived from a formal the body releases, consumes or returns itself, including a nullable release on a path that returns NULL | `base_custody` | 0 | `a_released_or_consumed_base_is_named`, `a_result_derived_from_a_formal_the_body_releases_consumes_or_returns_is_refused` |
| derived from a non-formal base: a local resource, a string literal, a global | `no_formal_base` | 0 | `a_non_formal_base_is_named`, `a_result_derived_from_a_non_formal_base_is_refused` |
| derivation through a local copy the body rebinds or increments (`q = q + 1`, `q++`) | `no_formal_base` | 0 | `a_mutated_local_copy_is_named`, `a_derivation_through_a_mutated_local_copy_is_refused` |
| different formals on different exits | `several_formals` | 0 | `several_formals_are_named` |
| otherwise not fully modeled | `body_not_modeled` | 0 | `a_body_the_ordinary_analysis_refuses_is_named` |
| proposal fails its fresh validation | `validation_failed` | 0 | none: no test body reaches it |

A mutated copy lands in `no_formal_base` because the classifier refuses the
rebinding, and the refusal's poison drops the copy's binding. An unmutated copy
(`q = d; return memcpy(q, …)`) is admitted.

Every refused wrapper keeps exactly its pre-card diagnostic vector, because
the ordinary CFG is unchanged. Its caller sees `call to uncontracted function`
(or `nested call … inside a modeled call`) naming it.

The zero counts record that the corpus contains no such bodies. They do not
verify the refusals on real code. The refusals are fail-closed, and every one is
exercised by the unit and end-to-end tests above.

## 5. Exact-alias release: kept refused

`free(memcpy(b, …))` frees `b` in C. Crediting it to `b` would be faithful.
This card keeps the refusal instead, and retires
`W43-RESULT-ALIAS-EXACT-RELEASE` on its "recorded ground" arm, for three
reasons:

1. **No measured reach.** With both new wrapper summaries in force, AFTER adds
   no leaf at all, so there is no release of an exact derived result, declared
   or inferred, anywhere in the corpus. `XXH_memcpy`'s callers never reach its
   contract (§3.2).
2. **It is the discharging direction.** Crediting would add an arm that removes
   obligations, and no corpus function would exercise it. That is the
   RETURN-POSITION-ACCUSING-UNEXERCISED shape, in the more dangerous direction.
3. **The premise is now faithful, so a later card can admit it cleanly.**
   Crediting needs "the alias's value equals the base's". Before this card,
   `memcpy(p + 1, …)` was bound as an exact alias of `p`. It is now interior
   (§1.5; `an_offset_result_is_not_an_exact_alias`). A card that meets a corpus
   function with this shape can admit crediting under its own attribution class.

The refusal is pinned for a wrapper
(`releasing_an_exact_wrapper_result_is_refused_not_credited`, by parity with
`derived_result.rs`'s declared case).

## 6. Validation

[`scripts/run_gate.py`](scripts/run_gate.py) is copied unchanged from
W43-LIBC-RESULT-ALIAS. It sets `CARGO_BUILD_JOBS=6` and `RUST_TEST_THREADS=2`,
turns debug info off, and captures every exit to a file, never through a pipe.
The logs are worktree-private under `measure/`.

The core and provider gates ran on the final implementation commit `c7470e42`.
Later commits change documents, the ledger, report scripts and evidence only.
The generator input is [gate.json](gate.json), with the
[Clang recording](clang-version.txt) taken in this checkout beside it.

| Command | Exit | Result | Log |
|---|---:|---|---|
| `git submodule update --init --recursive` | 0 | libevent at `d82464a2…` | `measure/gate/submodules.log` |
| `clang --version` | 0 | Ubuntu clang 21.1.8 | `measure/gate/clang.log` |
| `cargo test --workspace --no-fail-fast` | 0 | **5,231 passed / 0 failed / 11 ignored**, 279 summaries | `measure/gate/test.log` |
| `cargo fmt --all -- --check` | 0 | clean | `measure/gate/fmt.log` |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | clean | `measure/gate/clippy.log` |
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | | `measure/gate/probe.log` |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | | `measure/gate/followup.log` |
| `python3 scripts/check_lean_audits.py --jobs 4` | 0 | **171** audits | `measure/gate/lean.log` |
| `python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/w43-result-alias-returned/gate.json` | 0 | 298 follow-ups, 108 contract modules, 171 Lean audits | `measure/build-matrix.log` |
| `python3 scripts/build_docs_index.py`, then `--check` | 0 / 0 | index regenerated and matching | `measure/docs-index-check.log` |
| `python3 -m unittest discover -s scripts/tests` (after the final regeneration) | 0 | **330** tests | `measure/python-final.log` |
| `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline --no-fail-fast` | 0 | **117 passed / 0 failed / 0 ignored**, 39 summaries | `measure/provider-gate/ptest.log` |
| `… -p qcore-provider-conformance --test packages real_packages_preserve_unknown_and_reject_mutations … -- --exact --test-threads=1` | 0 | 1 passed (isolated, constraint 16) | `measure/provider-gate/pisolated.log` |
| `cargo fmt --manifest-path providers/Cargo.toml --all -- --check` | 0 | clean | `measure/provider-gate/pfmt.log` |
| `cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings` | 0 | clean | `measure/provider-gate/pclippy.log` |
| `python3 providers/contracts.py check` | 0 | inventories unchanged | `measure/provider-gate/pcontracts.log` |
| `python3 providers/package.py build --output measure/provider-gate/packages` | 0 | 5 packages | `measure/provider-gate/pbuild.log` |
| `python3 providers/package.py verify …/<id>` (×5: chunkio, cmetrics, ctraces, libevent, librdkafka) | 0 ×5 | | `measure/provider-gate/pverify.log` |
| `python3 -m unittest discover -s providers/tests` | 0 | 12 tests | `measure/provider-gate/ppython.log` |

**Counts.** This card adds 23 Rust tests and one test binary:

* `crates/qcore-c-gateway/tests/derived_result_returned.rs`: 15 tests;
* the `legacy_summary::derived` unit module
  (`crates/qcore-c-gateway/tests/unit/legacy_derived_summary.rs`): 8 tests.

It also adds one Lean audit. The base's own gate figures at `f8b41209` were not
remeasured here, so the delta is stated by test, not by subtraction. No
`P02-FLAKE` row failed.

**Attempts that are not the gate, disclosed.**

* **First core gate, stopped by hand** after its submodule and Clang steps,
  while `cargo test` was running. A census refinement (`c7470e42`, which splits
  the not-a-candidate class) was about to change Rust source. Its partial logs
  are in `measure/gate-aborted-1/`.
* **First provider gate, died of host disk exhaustion** (`No space left on
  device` in `providers/target`, with the shared disk at 100%). It measured the
  disk, not the program. It is retained at `measure/provider-gate-aborted-disk/`
  and is not evidence. Only this worktree's own `target/debug` (21 GB) and
  provider conformance artifacts were freed, after the core gate had completed.
  The provider gate was then rerun from scratch; that run is the one above.
* **First Python run, before the final regeneration:** 330 tests, exit 0
  (`measure/python1.log`). It is superseded by the final run above.

## 7. Ledger and status

* **Retired**, each verbatim with its ground in
  [retired-followups.json](retired-followups.json):
  * `W43-RESULT-ALIAS-RETURNED`: the inferred route, Lean and re-arbiter;
  * `W43-RESULT-ALIAS-EXACT-RELEASE`: refusal kept on the ground in §5.
* **Registered** ([followups.json](followups.json)):
  `W43-DERIVED-SUMMARY-CONST-DROPPED-FROM-CHAIN`. An inferred summary reads
  `const_dropped` off the derivation chain. `mk_string_casestr`'s summary says
  `true` because of `strcasestr`, although the wrapper's own formal is not
  `const`. No check reads the flag today.
* **Follow-ups:** 299 → **298**. `corpus.cycle19_raw_units` points at this
  report's [artifacts/](artifacts/). Its previous contents are in
  [previous-corpus-row.json](previous-corpus-row.json).
* **Untouched:** `W43-RESULT-ALIAS-ACCUSING-UNEXERCISED-IN-CORPUS` stays open.
  The wrappers' results can now reach both accusing arms, but no corpus caller
  does.

## 8. For the integrator

* **Constraint 7.** The leak pin chain can record W43-RESULT-ALIAS-RETURNED as
  leaving the pin at 272: 2 functions moved, and no leak leaf was added or
  withdrawn.
* **Files likely to conflict with main and the concurrent cards:**
  * `crates/qcore-c-gateway/src/legacy_c.rs`. W43-INDIRECT-ACCESS-GENERAL also
    edits it. This card's edits are localized to these places:
    * `Derivation::Derived` gains a `nullable` field, touching `derive_pointer`
      and `Derivation::join`;
    * `Statement::Return` gains `returns_null_constant`, and the `Parser` gains
      `admit_derived_returns`, set in `build_function_cfg_with`;
    * `return_call_instruction`'s admission;
    * `EntryCustody` gains `derived_bases` and `seed_derived_base`;
    * `ExitObservation` gains `derived_base_touched`, and `ReturnedValue` gains
      `Null` and `Derived`;
    * `observe_exit`, the `EscapeIfOwned` transfer arm, and `TransferContext`;
    * `analyze_defined_function`;
    * `derived_base` / `interior_extent` in the classifier.
  * `crates/qcore-c-gateway/src/legacy_summary.rs` and `legacy_return_call.rs`.
  * `crates/qcore-c-gateway/src/indirect_census.rs`: an appended sink.
  * `docs/QCORE_CONTRACT_SPEC_r0_3.md`: §C04.3.1 is amended in two sentences,
    and §C04.3.2 is new.
  * `proof/QCore.lean`: one import line.
  * `docs/reviews/r0_3/acceptance_matrix.json`, `docs/STATUS.md`,
    `docs/reports/INDEX.md` and `docs/development/increments/`. This card opens
    `wave-43.md`, since `wave-41-continuation.md` is over its cap. A concurrent
    card may open the same file.
* The `contracts` artifact embeds this worktree's path in its source keys, as
  earlier isolated cards' artifacts did.

## Reproduction

```sh
python3 docs/reports/w43-return-expression/scripts/run_audit.py <before> f8b41209          # base binary
python3 docs/reports/w43-return-expression/scripts/baseline_identity.py <before> docs/reports/w43-libc-result-alias/artifacts \
    --root-alias <this worktree>=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a4d52a22505f866ac
python3 docs/reports/w43-return-expression/scripts/run_audit.py <after> f70e4be9          # card binary
python3 docs/reports/w43-result-alias-returned/scripts/delta.py <before> <after> delta.json
cargo build --release -p qcore-compiler --example cycle19_fluent_bit_libraries_audit --features qcore-c-gateway/indirect-census
QCORE_DERIVED_CENSUS=<dir>/derived-census.jsonl python3 docs/reports/w43-return-expression/scripts/run_audit.py <census> c7470e42+indirect-census
python3 docs/reports/w43-return-expression/scripts/run_audit.py <confirm> c7470e42         # final binary
python3 docs/reports/w43-libc-contracts/scripts/archive.py <confirm> docs/reports/w43-result-alias-returned/artifacts
python3 docs/plans/cycle19-custody-cards/blocker_census.py docs/reports/w43-result-alias-returned/artifacts
python3 docs/reports/w43-return-expression/scripts/sole_callees.py docs/reports/w43-result-alias-returned/artifacts 30
python3 docs/reports/w43-result-alias-returned/scripts/update_ledger.py
python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/w43-result-alias-returned/gate.json
```
