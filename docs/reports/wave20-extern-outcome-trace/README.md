# EXTERN-SCALAR-OUTCOME (trace half) — the observation names the returned number (wave 20)

Card brief: *"The extern observation records `value qcore_extern_sum v1 b:r1
d:8` — the shape of a scalar binding but not the number that came back. The
binding half closed in wave 19 with a declared `succeeds INTEGER` outcome the
emitted program verifies (compares the real result, exits 70 on disagreement).
The trace half remains: the observation should name the returned number where a
declared outcome makes it well-defined."*

Base commit: main `e01cac2`.

**What landed.** One field, appended to one line form, in one producer. A call
whose site declares `succeeds INTEGER` now observes

```
value qcore_extern_sum v1 b:r1 d:8 -> 56
```

in the reference execution and in the emitted C alike. A call with no declared
outcome observes exactly what card `PIPE-SCALAR-RESULT` shipped, byte for byte,
and that boundary is now pinned as a literal in four places rather than
described in a comment. The composed route and the pipe operator inherit the
extended line through machinery neither of which was touched: there is one
lowering of an extern call site and one desugaring of a stage.

The honesty argument is §2 and is the whole card. Four new audited Lean theorems
plus one reachability fixture in `QCore.ComposedStdIo`; **one** pin moved
deliberately (that model's payoff fixture, §4.2); **zero** digests exist over
this format, which the census establishes rather than assumes (§3.2).

---

## 1. The change

### 1.1 The one producer

`crates/qcore-core/src/extern_c_emit.rs` is the only module that renders an
extern observation line. `value_observation` gained a fourth parameter and one
conditional field:

```rust
fn value_observation(
    callee: &ExternCallee,
    arguments: &[ExternArgument],
    value: ValueId,
    outcome: Option<i32>,
) -> String {
    format!(
        "value {} {}{}{}",
        callee.name,
        value_name(value),
        argument_lines(arguments, 0),
        outcome.map_or_else(String::new, |outcome| format!(" -> {outcome}"))
    )
}
```

Both of its call sites pass `binding.outcome`: the reference execution in
`trace_extern_events`, and the emitted `QCORE_IO_WRITE` in
`emit_extern_statements`. Because both runs read the *same* declaration off the
*same* checked `ScalarBinding`, the two observations are equal by construction
rather than by comparison — which is the property the extern route has always
had for every other line form, and the one this card had to preserve.

Nothing else changed in the backend. No new statement, no new C, no new
diagnostic, no new keyword: `succeeds INTEGER` was parsed, checked, certified
and *runtime-verified* by card `STD-IO-COMPOSE`; this card only reads the field
that card already published.

### 1.2 Why the field is spelled ` -> N`

Two candidate spellings were considered against the file's own conventions.

A **bare trailing token** (`… d:8 56`) follows the acquisition line's
precedent, which appends a bare `ok` / `null`. It is unambiguous today — every
argument token carries an `x:` prefix — but it makes the number
positionally indistinguishable from an argument to a human reader, and it would
become genuinely ambiguous the moment any future argument form renders without
a prefix.

The **arrow** (`… d:8 -> 56`) is this backend's *own existing* rendering of a
call result: the declared-outcome violation it already writes to standard error
is `declared outcome violated: NAME -> %ld`. Choosing the arrow means the two
places this backend names a call's number name it the same way, and the
argument list stays exactly the space-separated `x:y` tokens it was. That is
what landed.

### 1.3 What is *not* observed, and why that is the honest remainder

A binding with no declared outcome appends nothing. This is not an omission
awaiting a follow-on — it is the only correct behaviour available, and it is
reachable rather than hypothetical:

* The reference interpreter cannot compute a C function's result. It has no
  callee body, no linkage, no target model.
* Without a declaration there is nothing for the emitted program to compare
  against either, so no `if (vN != …)` is emitted, and no run establishes
  anything about the number.
* Publishing a number in that state would be publishing fiction on both sides,
  and the two sides would then have to be kept equal by making one read the
  other — which is exactly the thing the route's design forbids
  (`docs/QCORE_SPEC_r0_3.md` §L15: the two backends are written against the
  model, not against each other's output).

The undeclared line is reachable in a checked, admitted program on the extern
route (`crates/qcore-core/tests/unit/extern_call.rs` ::
`a_binding_without_a_declared_outcome_observes_no_number`,
`compiler/tests/extern_calls.rs` ::
`a_scalar_result_is_bound_and_read_at_a_declared_scalar_parameter`) and on the
composed route (`proof/QCore/ComposedStdIo.lean` :: `undeclaredTrace`, §4.1).
Nothing about it is unreachable, so nothing about it is named as a follow-on.

---

## 2. The honesty argument

**Claim.** Printing the *declared* number is an **observation** of the returned
number, not a claim about it.

The argument is entirely about an ordering inside the emitted C. For a binding
with a declared outcome, `emit_extern_statements` writes three statements in
this order:

```c
    int32_t v1 = qcore_extern_sum(r1, 8);
    (void) v1;
    if (v1 != 56) {
        QCORE_IO_WRITE(stderr, "declared outcome violated: qcore_extern_sum -> %ld\n", (long) v1);
        return 70;
    }
    QCORE_IO_WRITE(stdout, "value qcore_extern_sum v1 b:r1 d:8 -> 56\n");
```

The comparison **dominates** the write. Control reaches the observation only on
a run in which the real result and the declared number are equal. Therefore, on
every run that publishes the line, the number on the line *is* the number the
linked C returned — established by that run, not assumed by the compiler. A run
on which they differ publishes no line at all for that call and exits 70.

This is what makes the reference interpreter's copy of the number honest too.
The interpreter prints the declared number because that is what the model says;
the native run prints the declared number because it has just proved the
declaration. The two observations agree — and the agreement is a **translation
check** in the §L15 sense, backed by a run-time comparison, rather than two
programs echoing the same literal at each other with nothing checking either.

Two consequences are pinned rather than argued:

| Property | Pin |
|---|---|
| The check is emitted before the write | `the_outcome_check_precedes_the_observation_it_licenses` (core), `the_emitted_check_dominates_the_observation_it_licenses` (extern route), `the_composed_emission_observes_through_the_same_single_route` (composed route) |
| A wrong declaration exits 70 and observes nothing | `a_wrong_declared_outcome_exits_70_and_observes_nothing` — §5.3 |

Reordering those two emitted statements would turn the declared number from an
observation into a claim. That is why the ordering is asserted by three tests
and stated in the module header, rather than left as an accident of how the
function happens to be written.

**What the argument does not establish.** It says nothing about the callee's
implementation, about any other input, or about any other build. `succeeds
INTEGER` remains a *fixture literal outcome control* in the
`qcore_acquire`/`qcore_check` sense — a controlled model operation for a
selected trace — exactly as `succeeds BOOL` is for a nullable owned return.
A declared outcome is not a specification of the callee, and no verdict
anywhere is strengthened by one. Invariant 2's premise list is unchanged: this
card adds no assumption and removes none.

---

## 3. The census (done first)

### 3.1 Every reader of the extern observation format

Measured on `e01cac2` before the edit, by searching the whole tree for the
`value ` line form, for every consumer of the four published entry points
(`interpret_extern_program`, `trace_extern_events`, `emit_extern_c11`,
`emit_extern_statements`), and for every consumer of
`CompiledModule::extern_observation`.

| Reader | Location | How it reads the format | Effect of this card |
|---|---|---|---|
| Reference execution | `crates/qcore-core/src/extern_c_emit.rs` (`trace_extern_events`) | **produces** it | extended |
| Native emission | same file (`emit_extern_statements`) | **produces** it | extended, identically |
| Composed route | `crates/qcore-core/src/composed_std_io_c_emit.rs:80,160` | calls both of the above and interleaves the strings; renders no call line of its own | inherits |
| Driver publication | `crates/qcore-verification/src/driver.rs:2694,2877,2977` | carries the `Vec<String>` to `CompiledModule::extern_observation` | pass-through |
| Byte-compared test pins | `compiler/tests/extern_calls.rs`, `compiler/tests/pipe_operator.rs`, `compiler/tests/std_io_compose.rs`, `crates/qcore-core/tests/unit/extern_call.rs` | literal vectors | §3.3 |
| Native-vs-reference comparisons | `compiler/tests/extern_calls.rs`, `compiler/tests/pipe_operator.rs`, `compiler/tests/std_io_compose.rs`, `compiler/tests/unsafe_linkage.rs:579`, `compiler/tests/std_modules.rs:409` | compare the two runs with each other | self-consistent — see §3.3's finding |

**There is no parser of the extern observation anywhere.** This is the sharp
contrast with the task backend, whose F02 observer
(`crates/qcore-core/src/task_trace_relation.rs:265`, `parse_observed_trace`)
reads twelve line forms out of a program's own output and would have had to be
extended in lockstep. The extern route's line is written and byte-compared, never
re-read, so the format has exactly one producer and no grammar to keep in sync.

### 3.2 Every pinned digest over the extern observation format: **none**

The digests that exist near this area were checked individually and are all over
the **task** backend, a disjoint route:

| Digest | Location | Over what | Moved? |
|---|---|---|---|
| F02 emission digests (round-trip, concurrent) | `compiler/tests/f02_trace_relation.rs:53-54`; `compiler/tests/windows_threads.rs:26,31` | the task backend's *emitted C* | **no** — different backend, different file, no shared string |
| F02 domain/verdict digests (8, Rust ⇄ Lean) | `crates/qcore-core/tests/task_trace_relation.rs:243-343`; `proof/QCore/TaskTraceRelation.lean:488-507` | the model trace *domain* | **no** — no emitted text at all |
| Profile / package / task digests | `compiler/tests/profile_loader.rs`, `w3_evidence_v2.rs`, `counter_route_parity.rs`, `reactor_route_parity.rs` | manifests, packages, compile tasks | **no** |

The two frozen CT expectation fixtures that pin emitted text and a reference
observation were checked directly and are both on the mutex/task backend, not on
this route:

| Fixture | Pinned text | Moved? |
|---|---|---|
| `compiler/tests/fixtures/ct/CT34/expected.json` | `mtx_*`, `QCORE_IO_WRITE(stdout, "grow 1 1 2 0 4\n")`, `free(rN)`, and a `reference_observation` of `resource`/`create`/`lock` lines | **no** |
| `compiler/tests/fixtures/ct/CT26/expected.json` | `mtx_*`, `free(r1)`, and a `resource`/`create`/`lock`/`open`/`write` observation | **no** |

Neither names an extern callee, a `value ` line or a `v` name. Both are exercised
by `compiler/tests/ct_corpus.rs` in the workspace run below.

So the F02 re-pin precedent that `TRACE-STDIO` §5.1 set — re-pin deliberately,
re-verify the relation, state the move — **had nothing to apply to here**. That
is a measurement, not an assumption: the workspace run in §6 includes
`f02_trace_relation` and `windows_threads` unchanged and green.

### 3.3 The census's own finding

**No existing pin needed to move, because no existing pin covered a
declared-outcome binding.** Both literal `value …` pins on `e01cac2`
(`compiler/tests/extern_calls.rs:1363-1364`,
`compiler/tests/pipe_operator.rs:784-785`) are *undeclared* bindings, which this
card leaves byte-identical by design. Every declared-outcome fixture in the tree
— `fixtures/compose/chain.qcore` and its hand-written twin, and four in-test
sources in `std_io_compose.rs` — was pinned only by:

* native-vs-reference equality, which moves with the format on both sides and
  cannot detect a format change; or
* `contains("out: 42")` / `rendered_values` assertions, which are about the
  *rendering*, not about the trace line.

The whole existing suite therefore passed unchanged on the first run after the
edit. That is a gap, not a reassurance: the format could have moved silently.
It is closed here by adding four byte-compared pins over declared-outcome lines
(§5), the composed chain's entire observation among them.

---

## 4. Lean parity

### 4.1 Where the relation lives

The extern observation is modeled in exactly one Lean module.
`QCore.ExternCall` models admission, custody and the adapter but has no trace or
observation at all; `QCore.SingleEmissionRoute` and `QCore.PipeDesugaring`
mention neither. `QCore.ComposedStdIo` is the module that models what a step
*publishes*, on which stream — so that is where the trace half is modeled, and
`AuditComposedStdIo.lean` is the audit that grew.

The `.ext` case of `stepLines` previously published `⟨Stream.out, none⟩` for
every call, with `none` documented as "the extern route's synthesized call
trace, whose digits this model does not describe". It now publishes the number
the call's declaration states, through a new `externLineText`:

```lean
def externLineText (c : Call) : Option Text :=
  match c.binding with
  | some s => s.outcome.map render
  | none => none
```

`none` accordingly changed meaning, and the docstring changed with it: it is now
"a line naming **no number**", which is a claim about the line rather than a
gap in the model.

Five audited results, all `propext`-only, no project axiom:

| Theorem | What it says |
|---|---|
| `a_declared_outcome_is_named_on_the_calls_trace_line` | a call declaring `n` publishes one stdout line carrying exactly `render n` |
| `an_undeclared_outcome_names_no_number` | a call naming no number publishes `⟨out, none⟩` — the undeclared boundary, kept |
| `the_undeclared_shapes_name_no_number` | both undeclared shapes spelled out: binds nothing, and binds without an outcome |
| `the_trace_line_names_the_number_the_call_determines` | **the payoff**: the number a trace line names is the number `determinedBy` gives for the value it binds — the same relation `renderOperand` reads, so a trace line and a rendering of the same value cannot name two different numbers |
| `the_undeclared_trace_line_is_reachable` | a non-vacuity fixture: an admitted composed program whose call declares nothing and whose rendering names the literal instead |

The fourth is the one worth the module. Before it, the trace line and the
rendering were two independent publications that happened to be derived from the
same field; now the model states that they *cannot* disagree, and widening
either side has to move this theorem rather than slip past it. The fifth exists
so the fourth's `none` branch is about a shape an admitted program actually has
— the previously-existing `undeterminedOutcome` fixture is *refused*, so on its
own it would have made the undeclared trace line vacuous.

`the_extern_trace_shares_standard_output` is unchanged in statement (one stdout
line, zero stderr lines) and its proof was re-witnessed with the new text.

### 4.2 The one moved pin

`QCore.ComposedStdIo.the_payoff_fixture_is_admitted` — moved deliberately.

| | |
|---|---|
| Was | `lines payoff = some [⟨Stream.out, none⟩, ⟨Stream.err, some (render 42)⟩]` |
| Now | `lines payoff = some [⟨Stream.out, some (render 42)⟩, ⟨Stream.err, some (render 42)⟩]` |
| Ground | the payoff fixture's call declares `succeeds 42`, so its trace line now names 42 — the direction of the change is *more* information published, never less, and no refusal moved |
| Recorded | in the theorem's own docstring, with the before value |

The other three conjuncts of that theorem (`admitted`, `run … = []`,
`composedEffects`) are unchanged, so nothing about admission, custody or effects
moved with it. No other Lean statement or fixture in the repository changed.

---

## 5. What is pinned now

| Pin | Location | Kind |
|---|---|---|
| Undeclared line, byte-identical (core) | `crates/qcore-core/tests/unit/extern_call.rs` :: `a_binding_without_a_declared_outcome_observes_no_number` | reference **and** emitted `printf`, plus "no comparison is emitted for this call" |
| Undeclared line, byte-identical (route) | `compiler/tests/extern_calls.rs` :: `a_scalar_result_is_bound_and_read_at_a_declared_scalar_parameter` (pre-existing, unchanged) | reference |
| Undeclared line, byte-identical (pipes) | `compiler/tests/pipe_operator.rs` :: `a_scalar_stage_feeds_the_next_stage_and_emits_its_hand_written_twin` (pre-existing, unchanged) | reference |
| Declared line, both runs (core) | `crates/qcore-core/tests/unit/extern_call.rs` :: `a_declared_outcome_is_named_by_both_the_reference_and_the_emitted_run` | **new** |
| Negative outcome spelled as written | same file :: `a_negative_declared_outcome_is_observed_as_written` | **new** |
| Check dominates write (core) | same file :: `the_outcome_check_precedes_the_observation_it_licenses` | **new** |
| Declared line, whole trace (extern route) | `compiler/tests/extern_calls.rs` :: `a_declared_outcome_is_named_in_the_observation` | **new** — the six-line trace, so the diff against the undeclared pin is exactly two `-> N` fields |
| Check dominates write (extern route) | same file :: `the_emitted_check_dominates_the_observation_it_licenses` | **new** |
| Native run of a declared fixture | same file :: `accepted_fixtures_run_natively_against_a_linked_c_implementation`, new `scalar-outcome` row | **new** — under ASan/UBSan/LSan, against the real `buffer_api.c` |
| Fail-closed: wrong outcome | same file :: `a_wrong_declared_outcome_exits_70_and_observes_nothing` | **new** — §5.3 |
| Piped case | `compiler/tests/pipe_operator.rs` :: `a_declared_outcome_on_a_stage_reaches_the_observation_through_the_desugaring` | **new** — §5.2 |
| Composed chain, whole observation | `compiler/tests/std_io_compose.rs` :: `the_composed_extern_trace_names_the_numbers_its_renderings_render` | **new** — §5.1 |
| Check dominates write (composed route) | same file :: `the_composed_emission_observes_through_the_same_single_route` | **new** |

### 5.1 The composed route

The composed chain's full eleven-line observation is now a literal pin. It reads:

```
out: acquire qcore_extern_alloc r1 ok
out: call qcore_extern_fill b:r1 d:7
out: value qcore_extern_sum v1 b:r1 d:8 -> 56
out: 56
out: value qcore_extern_scale v2 v:v1 d:2 -> 112
err: [INFO] 112
out: value qcore_extern_scale v4 v:v3 d:2 -> 84
err: [WARN] 0x00000054
out: call qcore_extern_release c:r1
out: -7
out: return
```

Lines 3 and 4 are the point of the card: the call's own trace line and the
`print` that renders the same value now name the same number, on the same
stream, adjacent. Before this card line 3 ended at `d:8` and a reader had to
take on faith that the 56 below it came from the call above it. The composed
route needed no code change to get this — `composed_std_io_c_emit.rs` calls the
extern backend for both its reference lines and its statements, so there is one
lowering of a call site and it was already extended.

The literal binding `let value = 42;` is still observed as **nothing**: it calls
nothing, so `v3` appears only where a call reads it. Card
`EXTERN-SCALAR-LITERAL`'s boundary is unmoved and asserted alongside.

### 5.2 Pipes

`scalar_result_piped.qcore` with `succeeds 56` and `succeeds 112` on its two
stages emits byte-identical C11 against the hand-written statement sequence and
observes identically, `-> N` fields included. The desugaring was not touched:
a declared outcome on a stage is a declared outcome on the call that stage
becomes, and the hygienic temporary still reaches neither artifact.

### 5.3 The fail-closed direction

`a_wrong_declared_outcome_exits_70_and_observes_nothing` compiles a source whose
declaration is *wrong* — `qcore_extern_scale(21, 2) succeeds 41` against an
implementation that returns 42 — links it against the real `buffer_api.c` under
ASan/UBSan/LSan and reads the raw exit status:

* the checker **admits** it, which is correct and is asserted: a literal outcome
  control is a fixture control, and no checker in this repository computes a C
  function's result;
* the reference execution publishes `value qcore_extern_scale v1 d:21 d:2 -> 41`,
  believing the declaration as it always has;
* the native run exits **70**, writes `declared outcome violated:
  qcore_extern_scale -> 42` to standard error, and publishes **no** observation
  line for that call — asserted as the absence of any `value qcore_extern_scale`
  line on stdout;
* the positive control — the same program with `succeeds 42` — exits 0 and
  publishes `value qcore_extern_scale v1 d:21 d:2 -> 42`, matching the
  reference. Without it the refusal above would be consistent with a program
  that never works.

The fixture deliberately holds **no live acquisition** at the moment of the
refusal, so LSan has nothing to report and the status the test reads is the
emitted program's own 70 rather than a leak checker's. Doing this on a fixture
that abandons a live buffer would have required disabling leak detection for
that run, which is a weaker measurement; the shape was chosen instead.

A new `run_native_status` helper returns `(exit status, stdout, stderr)` without
asserting success. The existing `run_native` still asserts a clean exit and was
not weakened — every fixture it runs must still exit 0.

---

## 6. Validation

Full standing per-change gate, run in this worktree.

| Step | Result |
|---|---|
| Workspace tests | **3,703 passed · 0 failed · 15 ignored**, across 220 test targets (§6.1) |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean, 0 warnings |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **exit 0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **exit 0** |
| `python3 scripts/check_lean_audits.py` | **85 Lean audits passed**, axiom closure clean |
| `python3 -m unittest discover -s scripts/tests` | **193 tests, OK** |

Two Linux-only, non-Windows-gated native suites ran as part of this: the extern
route's ASan/UBSan/LSan fixtures (now including the new `scalar-outcome` row and
the wrong-outcome exit-70 case) and the composed route's per-stream comparison.

### 6.1 How the workspace suite was measured, and why not in one command

`cargo test --workspace` was **started three times and killed twice by the
environment**, not by a test failure — once mid-build and once mid-run, both
times with the process disappearing and no exit status written. The third run
completed 89 of its targets before being terminated the same way. Rather than
report a partial run as a full one, the suite was completed as a **disjoint
union** of package-scoped runs, each logged under `measure/`:

| Run | Packages | Targets | Passed | Ignored |
|---|---|---:|---:|---:|
| `workspace-tests-3.log` (compiler-only portion) | `qcore-compiler` | 86 | 1,259 | 7 |
| `cargo test -p qcore-compiler --test unsupported_close_route_parity` | the one target run 3 was cut off before | 1 | 3 | 0 |
| `tests-core-group.log` | `qcore-core`, `qcore-frontend`, `qcore-contract`, `qcore-schemas` | 38 | 1,066 | 1 |
| `tests-host-group-2.log` | `qcore-verification`, `qcore-verifier-host` | 45 | 376 | 0 |
| `tests-gateway-group.log` | `qcore-audit`, `qcore-c-gateway` | 47 | 985 | 7 |
| `tests-qcorec.log` | `qcorec` | 3 | 14 | 0 |
| **Total** | ten workspace members | **220** | **3,703** | **15** |

The partitions are disjoint by construction: run 3's `qcore-audit` and
`qcore-c-gateway` unittest targets (389 tests) are excluded from its column and
counted once in the gateway row.

**A gate prerequisite, not a regression.** Run 1 failed one target,
`libevent_submodule_matches_the_audited_revision`, because this worktree had
never run `git submodule update --init --recursive` — invariant 5's pinned
revision check doing exactly its job on an unpopulated `third_party/libevent`.
The submodule was initialised to the pinned `d82464a2…` and the target passes.

### 6.2 Two P02-FLAKE rows tripped, both verified in isolation

Constraint 16 names four load- or parallelism-sensitive rows. Two tripped here,
both with the exact signature the constraint records, and **neither was
weakened**:

| Test | Failure under load | In isolation |
|---|---|---|
| `qcore-verifier-host` :: `enforced_process_limits` :: `a_declared_process_limit_stops_the_fixture_from_forking` | `Io(Broken pipe)` where `Terminated` expected | `cargo test -p qcore-verifier-host --test enforced_process_limits -- --test-threads=1` → **12 passed** |
| `qcore-verifier-host` :: `confined_process_execution` :: `a_confined_fixture_cannot_write_outside_the_admitted_paths` | `Io(Broken pipe)` where `Terminated` expected | `cargo test -p qcore-verifier-host --test confined_process_execution -- --test-threads=1` → **12 passed** |

Both are on the constraint's own list (the `RLIMIT_NPROC` row, which counts every
process of the real UID, and the confined-fixture write test, whose recorded
symptom is precisely `Io(Broken pipe)` under load). Five sibling worktrees were
compiling concurrently — load average peaked above 140 on 16 cores — which is
the documented condition. Neither test is touched by this card: nothing here
goes near the provider host.

`-j 6` was used throughout to bound peak memory alongside those siblings; it
limits parallel `rustc` invocations only and changes no test's selection or
behaviour.

Corpus-neutral by construction: no file under `crates/qcore-c-gateway/`,
`crates/qcore-audit/`, `contract/qcore/` or `providers/` was read or written.
The leak count (272), the 185-unit corpus and the contract corpus (69 modules)
are untouched.

**Files changed** (8):

```
crates/qcore-core/src/extern_c_emit.rs        the field, and the three comments that state why
crates/qcore-core/tests/unit/extern_call.rs   4 new unit pins
compiler/tests/extern_calls.rs                4 new route pins + run_native_status
compiler/tests/pipe_operator.rs               1 new piped pin
compiler/tests/std_io_compose.rs              2 new composed pins
proof/QCore/ComposedStdIo.lean                externLineText, 5 theorems, 1 fixture, 1 moved pin
proof/AuditComposedStdIo.lean                 5 new audited names
docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md       §G02f's observation paragraph
```

---

## 7. Deliberately not done

* **No new syntax, no new keyword, no new diagnostic.** `succeeds INTEGER`
  already parses; a wrong declaration is a run-time finding, not a refusal, so
  §L21 is untouched. Constraint 14's keyword census applies vacuously: zero
  words added.
* **The undeclared line was not "fixed".** Widening the observation to every
  binding would require inventing a number on the reference side. See §1.3 —
  this is the remainder, and it is a correct one.
* **The declared number was not promoted to evidence.** Nothing reads the new
  field as a fact about a callee; no certificate field, verdict, premise or
  assumption changed. `declared_outcomes` was already on the certificate before
  this card.
* **`SummaryConflict`-style reporting of a wrong declaration at check time was
  not added.** It would require the checker to know a C function's result, which
  is precisely what it cannot do. The run is the right place for that
  discrimination and already performs it.
* **`docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` §G02f contains an adjacent stale
  sentence** — "A **scalar literal** binding (`let n = 42;`) is deliberately
  *not* added here and stays refused" — which card `EXTERN-SCALAR-LITERAL`
  superseded in wave 19 (`std_io_compose.rs` ::
  `a_scalar_literal_binds_on_the_extern_route_alone` admits exactly that form on
  the extern route). It is one paragraph from the text this card edited and was
  left alone rather than corrected in passing; see §8.

---

## 8. Named follow-ons

| Id | What |
|---|---|
| `EXTERN-OUTCOME-WIDTH` | the observation renders the declared `i32` outcome; a wider declared result (`qcore_extern_widen`'s `i64`) can be bound and passed but its outcome is `Option<i32>` on the model, so a wide declared outcome is not expressible. Companion to the existing `EXTERN-SCALAR-WIDTH`. |
| `EXTERN-OUTCOME-UNDECLARED` | *stated as closed, not open*: an undeclared binding stays unobserved on the argument in §1.3. Recorded here only so a later reader does not reopen it as an oversight. |
| `G02F-SCALAR-LITERAL-STALE` | `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` §G02f still says the scalar-literal binding is refused on the extern route; wave 19's `EXTERN-SCALAR-LITERAL` admitted it. One paragraph, doc-only, out of this card's scope (§7). |
| `EXTERN-OUTCOME-WIN-NATIVE` | the wrong-outcome exit-70 test is `#[cfg(not(target_os = "windows"))]`, following every other native run in `extern_calls.rs`. Confirming status 70 through the `windows_c` harness is unmeasured. |
| `WAVE20-WORKSPACE-COUNT` | the measured workspace total is **3,703** across 220 targets, against the ledger's wave-17/19 headline of 2,986. This card added 11 tests, so the delta is waves 18–20 plus whatever the earlier figure counted differently — an integrator reconciliation, not a finding of this card. The measurement method is §6.1. |

---

## 9. Draft increment paragraph

> **EXTERN-SCALAR-OUTCOME (trace half), wave 20.** The extern observation now
> names the number a call returned, wherever a `succeeds INTEGER` outcome makes
> it well-defined: `value CALLEE vN ARG… -> N`, in the reference execution and
> the emitted C alike, with the arrow taken from the backend's own existing
> rendering of a call result. The declared number is an observation rather than
> a claim because of an ordering the emitted C already had and this card made
> load-bearing — the comparison against the real result dominates the write, so
> a disagreeing run exits 70 having published no line for that call, and a run
> that publishes the line has established the equality. A binding with no
> declared outcome keeps card `PIPE-SCALAR-RESULT`'s line byte for byte, which
> is the correct remainder rather than an omission: no model computes a number
> for it. The census found one producer, no parser (unlike the task backend's
> F02 observer) and **no digest at all** over this format, so the `TRACE-STDIO`
> re-pin precedent had nothing to apply to; it also found that no existing
> literal pin covered a declared-outcome line, which is why the whole suite
> passed unchanged and why four new byte pins were added, the composed chain's
> entire eleven-line observation among them. Pipes and the composed route
> inherit the field through an untouched desugaring and an untouched single
> lowering. Fail-closed direction pinned natively: a wrong declaration is
> admitted by the checker, exits 70 under ASan/UBSan/LSan and observes nothing,
> beside its positive control. Five audited theorems in `QCore.ComposedStdIo`
> — including that a trace line and a rendering of the same value cannot name
> two different numbers — and one deliberately moved Lean pin, that module's
> payoff fixture, whose stdout line now carries the number it declares.
> Corpus-neutral by construction. Gate: 3,703 workspace tests (15 ignored, 220
> targets, measured as a disjoint union of package runs after the environment
> killed the full run twice), 85 Lean audits, 193 Python, both probes exit 0,
> fmt/clippy clean; two named P02-FLAKE rows tripped under five-sibling load and
> passed in isolation, unweakened.
