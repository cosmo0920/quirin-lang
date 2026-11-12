# W45-FB-ENGINE-PROGRESS-MEASURE: a checked measure for counted loops

Card W45-FB-ENGINE-PROGRESS-MEASURE, the P2 implementation card of the
[Fluent Bit system-coverage plan](../../plans/fluent-bit-system-coverage/README.md#fb-engine-progress),
base `74105dd0e` (`main`). It implements the property the measurement card
[W45-FB-ENGINE-PROGRESS](../w45-fb-engine-progress/README.md) registered (§6
there) as **gateway rule 44** ([rule](../../agents/gateway.md#constraint-44)).

**What it is.** A bound fact recorded beside the resource verdict: for an
admitted counted `for` loop, the checked measure `B - c` and the most
iterations the loop can run; for every other loop, a named refusal. **What it
is not.** No verdict moves, nothing reads the fact, and there is no progress,
termination-of-the-body, latency or throughput claim. A bounded number of
iterations bounds work per call only (§7 of the measurement report).

## 1. Prediction

[PREDICTION.md](PREDICTION.md), committed at `2cf7eb15b` before any
implementation code: no custody verdict moves on `eight-home`,
`fluent-bit-system` or `fluent-bit-tail-pause` (`route corpus none`, `route
system none`, `route slice none`), and one predicted deviation from the brief
(§4 below).

## 2. Mechanism

* **The rule** (`crates/qcore-c-gateway/src/loop_measure.rs`). It reads Clang's
  typed JSON AST of the unit's preprocessed text, one top-level declaration at
  a time, tracking Clang's de-duplicated locations in document order. Macro
  constants are literals after Clang's own preprocessing, and an array extent
  is read from Clang's type of the array, so nothing is read lexically. The
  admitted shape and the fourteen named refusals are rule 44's text; the
  decision is a precedence-ordered function over an abstract `Shape`, shared
  with Lean.
* **Soundness choices** the brief asked to be fail-closed:
  * `c0 >= B` records a measure of 0 iterations.
  * Every conversion Clang recorded on either side of `c < B` must preserve
    values (`int i < size_t n` is refused, `comparison-converts`).
  * `B`'s value (a constant) or type range must lie within the counter's
    type; otherwise `c++` may wrap (`unsigned char i < 300`,
    `uint8_t i < uint16_t n`: `counter-may-wrap`).
  * `break`, `return`, a `goto` out and `continue` are admitted. A label in
    the loop, or a `case` of a switch outside it (Duff's device), is refused
    (`label-in-loop`). A returns-twice call (`setjmp`, `vfork`, …), inline
    assembly or a block literal anywhere in the function is refused
    (`opaque-control`).
  * The counter's address may be taken nowhere in the function. A stable
    local bound's address may be taken nowhere either; a bound of an unsigned
    type at most 16 bits wide is admitted even when it changes or is
    address-taken, because its type alone bounds it.
  * The counter must be written by nothing but its one `c++`/`++c` step
    (a second increment in the body: `counter-written-in-loop`; `i += 2`:
    `step-not-increment`). Only `<` with the counter on the left is admitted
    (`<=`, `!=`, `>`: `test-not-less-than`).
* **The selection** (`qcore-audit`). `QCORE_LOOP_MEASURE_PROPERTY=<rows.jsonl>`
  selects the property exactly as `QCORE_BOUNDS_PROPERTY` and
  `QCORE_NULL_PROPERTY` do. Unset, nothing runs. Set, each projected function
  gets one row; the custody status is joined from the report's own unit row
  after every fold. Both checkpointed routes refuse the selection before any
  work, and a checkpointed fragment holding rows is refused.
* **The schema.** The fact is a **new optional component**, written beside the
  report: rows `{library, file, function, custody, loops, identity}`, each loop
  `{line, origin, kind, measure, refusal, detail}`, each measure `{counter,
  counter_type, init, bound_kind, bound, measure, max_iterations,
  admitted_conjuncts}`, with `<rows>.summary.json`. It is versioned by its own
  identifiers (`loop-measure.v1`, model `qcore-loop-measure/1`). The custody
  report's schema, its `schema_version` and its components sidecar are
  unchanged; **no schema version is bumped**.

## 3. Lean

`QCore.CountedLoopMeasure` with [`proof/AuditCountedLoopMeasure.lean`](../../../proof/AuditCountedLoopMeasure.lean)
(33 audited theorems, no placeholders, no project axioms), registered through
`scripts/gen_lean_roots.py`:

* the measure is positive while the test holds and strictly decreases, by
  exactly one, per iteration (`measure_positive_while_running`,
  `measure_strictly_decreases`, `measure_decreases_by_one`,
  `measure_is_the_distance`, `measure_along_a_run`);
* the step after a true test stays in the counter's range when the bound is
  within it (`step_stays_in_counter_range`);
* every run against bounds at most `bmax` has at most `(bmax - c0).toNat`
  iterations, a prefix of a run is a run, and `c0 >= b` runs no iteration
  (`run_length_bounded`, `fixed_bound_runs`, `a_prefix_of_a_run_is_a_run`,
  `no_iteration_when_init_reaches_bound`);
* the decision admits only shapes meeting every premise those theorems use
  (`an_admitted_shape_meets_the_premises`,
  `an_admitted_conjunct_meets_its_premises`), and refuses each refusal shape
  (`a_while_loop_is_refused`, `an_address_taken_counter_is_refused`,
  `a_double_increment_is_refused`, `a_non_less_than_test_is_refused`,
  `a_wrapping_bound_is_refused`, `a_written_bound_is_refused`,
  `an_address_taken_wide_bound_is_refused`,
  `a_converting_comparison_is_refused`, …);
* each refusal is necessary: a wrapping counter, a bound written in the loop,
  a counter reset through an alias, a double increment to the type's maximum
  and a `<=` test against the type's maximum each have a run that never ends
  (`a_wrapping_counter_never_exits`, `a_written_bound_never_exits`,
  `an_aliased_counter_reset_never_exits`, `a_double_increment_never_exits`,
  `a_less_or_equal_test_never_exits`);
* recording a fact keeps the verdict (`recording_a_fact_keeps_the_verdict`,
  `recording_facts_keeps_the_verdict`);
* `decideVectors` equals the Rust `DECIDE_VECTORS`
  (`decide_vectors_hold`), checked from both sides
  (`the_decide_vectors_hold_and_match_the_lean_table`).

## 4. The slice's loops

**The slice run with the selection was not measured on this host** (§6). The
table is the rule's decision on fixtures that reproduce each loop's text from
Fluent Bit `046accf6` (`crates/qcore-c-gateway/tests/loop_measure.rs`, run
through Clang 21.1.8), not the slice itself.

| Loop | Fixture | Fact |
|---|---|---|
| `src/flb_engine.c` `handle_output_events` :909 | `the_output_event_batch_is_bounded_by_its_array_count` | **checked**, constant: `sizeof(values) / sizeof(values[0])` over `uint64_t values[FLB_ENGINE_OUTPUT_EVENT_BATCH_SIZE]`, extent 1 from Clang's type; `1 - index`, **at most 1** iteration (the `index < limit` conjunct is admitted too, the smaller bound kept) |
| `src/flb_input_chunk.c` `flb_input_chunk_get_direct_routes` :2286 | `the_stored_route_count_is_a_stable_uint16_local` | **checked**, stable local: `stored_count - index`, `uint16_t`, **at most 65,535** |
| `src/flb_task.c` `flb_task_create` :652 | `an_address_taken_int_bound_is_refused` | **refused**, `bound-address-taken` (deviation, below) |
| `plugins/in_storage_backlog/sb.c` `cb_queue_chunks` :611 | `the_slice_refusing_controls_have_no_counter` | refused, `no-counter` |
| `src/flb_scheduler.c` `ipow` :237 | same | refused, `no-counter` (a shift measure needs `exp >= 0`, out of scope) |
| `src/flb_scheduler.c` `random_uniform` :106 | same | refused, `no-counter` |

**Deviation: `flb_task_create` :652 is refused, not accepted.** Its bound
`direct_output_count` is an `int` local whose address is passed to
`flb_input_chunk_get_direct_routes` at :648, before the loop, and the loop body
calls `task_collect_output_references`, `flb_input_chunk_get_name`, `strlen`
and `flb_free`. A callee that retained the address could let a later call in
the body change the bound, and `int` is not a type that bounds the loop by
itself. The brief's own soundness clause ("B unwritten and not address-taken";
"any doubt is refused") therefore refuses it. The census's 65,535 rests on the
callee writing a `uint16_t` value through the pointer and retaining nothing,
which is a cross-unit fact this rule does not derive. Admitting it needs an
out-parameter non-retention premise (`W45-FB-MEASURE-ESCAPED-BOUND-PREMISE`).
This was predicted before implementation (PREDICTION.md §4).

**Fixtures for each refusal shape** (the same test file): a wrapping counter
(`unsigned char i < 300`, `uint8_t i < uint16_t n`), a written bound, an
address-taken counter, a double increment and `i += 2`, non-`<` tests (`<=`,
`!=`, `>`, a call), a converting comparison, a static counter, a wide field
bound, a non-constant init, a label in the loop, `vfork`, and Duff's device.
Accepted extras: a macro constant, an enumerator, `c0 >= B` (0), a narrow
field, an address-taken narrow bound, early exits with `continue`, and a macro
loop placed at its use (line mapped through the gateway's line map).
`qcore-audit/tests/loop_measure_selection.rs` drives the selection through
`quirin-audit current`: the facts are written with the custody status and
original lines, every custody artifact is byte-identical with and without the
selection, and the checkpointed run refuses the selection.

## 5. Arbiter comparison

**Not run on this host.** `quirin-audit measure --declared PREDICTION.md`
needs the prepared inputs of the three live profiles (`build/cycle19`,
`private/w45-fb/w46-external-callee-contracts/attempt-1`) and a published
baseline (`private/baselines/<sha>`). None exists on this machine, and
preparing a Fluent Bit checkout means cloning the user's checkout, which this
card's worktree guard refuses (it refuses git operations on paths outside the
worktree). The host also had no `clang` binary or Lean toolchain installed;
the card used Ubuntu's `clang-21` 21.1.8-6ubuntu1 package and the Lean 4.35.0
release, both unpacked under the worktree's ignored `private/toolchains/`.

**The byte-identity argument, by construction.** With
`QCORE_LOOP_MEASURE_PROPERTY` unset, every new code path is guarded by
`std::env::var_os(LOOP_MEASURE_SELECTION).is_none()` and returns before doing
anything; no custody, bounds, null or numeric code path is changed. Set, the
rule only appends rows to a vector no report writer reads, and the selection
test shows every custody artifact byte-identical with and without it. So the
prediction (`none` on every route) should hold, and no report byte is
expected to change.

The integrator's arbiter, on a host with the prepared inputs:

```sh
target/release/quirin-audit --plugin-path target/release --output measure/w45-measure \
  --timeout-ms 36000000 measure --profile eight-home --profile fluent-bit-system \
  --profile fluent-bit-tail-pause --side base=baseline:<main> --side tip=rev:<card tip> \
  --declared docs/reports/w45-fb-engine-progress-measure/PREDICTION.md
QCORE_LOOP_MEASURE_PROPERTY=<out>/loops.jsonl quirin-audit current <the fluent-bit-system route's arguments>
```

The second command lists the bound facts on the slice
(`W45-FB-ENGINE-PROGRESS-MEASURE-SLICE-RUN`).

## 6. Validation

The scoped card gate, `python3 scripts/gate_scope.py --base 74105dd0e`, on
tip `c283beaaa` ([gate.json](gate.json), private run
`private/gate-scope/20261112T030826Z-1364545/`), with `QCORE_CLANG` set to the
unpacked `clang-21` ([clang-version.txt](clang-version.txt)) and the unpacked
Lean 4.35.0 first on `PATH`. Mode **full** (the diff reaches
`qcore-c-gateway`). Gate exit **1**:

| Step | Exit | Counts |
|---|---|---|
| submodules | 0 | 1 submodule |
| fmt | 0 | |
| clippy | 0 | |
| Rust suite | **101** | 629 units, 670 summaries; 9,207 passed, **198 failed**, 17 ignored |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| Lean (`check_lean_audits.py`, all) | 0 | 427 audits |
| Python | 0 | 756 tests |

**The 198 failures are this host's, not the card's.** They are 187 distinct
test names in 79 units ([gate/tip-failed-tests.txt](gate/tip-failed-tests.txt)).
The same 79 units were run at the base `74105dd0e` (a `git archive` of the base
with the same Clang, `cargo test --no-fail-fast`, private log
`private/base-failing-units.log`): 192 distinct names failed
([gate/base-failed-tests.txt](gate/base-failed-tests.txt)), and every tip
failure but one is among them. The six base-only names read the repository's
`.git` or index, which the archive lacks. The one tip-only failure,
`qcore-core` `translation_validation::tests::exhaustive_emitted_c_corpus_passes_available_native_sanitizers`,
comes from the Lean toolchain's own `clang` shadowing the system compiler
on the gate's `PATH`; run at the tip without that `PATH` it passes
(`private/qcore-core-sanitizer-no-lean-path.log`), and `qcore-core` is not
touched by this card. The causes seen in the failure logs are host
toolchain gaps: the minimal Clang install has no sanitizer runtimes
(`libclang_rt.asan*.a`), tests that require the configured compiler to be
named `clang` reject `clang-21`, and the base's contract modules (licence
headers at `74105dd0e`) fail `hash_contracts`'s "module header" check at the
base too. The card's own targets pass: `qcore-c-gateway` lib (1,280 tests,
including the four rule-44 unit tests), `qcore-c-gateway::test::loop_measure`
(15) and `qcore-audit::test::loop_measure_selection` (2). The integrator's
full gate on a provisioned host is the arbiter.

An earlier run on `61a41d426` (`private/gate-scope/20261112T025326Z-987629/`)
had the same Rust counts and one Python failure: the selection test's
directory walk was not in the reviewed walk list. `c283beaaa` replaced it
with a fixed file list.

## 7. Follow-ups

* `W45-FB-ENGINE-PROGRESS-MEASURE` is retired (its discharge is this card's
  rule, Lean and fixtures); its slice and arbiter evidence is rehomed to
  `W45-FB-ENGINE-PROGRESS-MEASURE-SLICE-RUN`.
* `W45-FB-MEASURE-ESCAPED-BOUND-PREMISE`: `flb_task_create` :652 and :688.
* `W45-FB-LIST-FINITENESS-PREMISE`: the 87 list-length loops (premise PL).
* `W45-FB-MEASURE-SHIFT-PRECONDITION`: `ipow`'s right-shift measure under an
  `exp >= 0` precondition contract.
* Unchanged and not discharged here: `W45-FB-ENGINE-REVISITS-UNEXERCISED`,
  `W45-FB-RETRY-UNSPENT-RESCHEDULE`, `W45-FB-BACKLOG-TRANSIENT-STICKY-ERROR`.
