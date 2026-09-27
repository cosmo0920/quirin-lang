# TRACE-STDIO — one printing route, and the evidence that stood on the old one (wave 19)

> Runtime import maintenance: archived source inputs and runnable snippets now use `#import`; original results and hashes describe their recorded revision. See the [spelling attribution](../runtime-source-imports/spelling-attribution.json) for maintained source identities.

Card brief: *"The concurrency module's emitted code carries DEBUG PRINTS — the
compiler-synthesized `printf` trace instrumentation the STD-MODULES census
classified as class (b). These debug prints must NOT remain as raw synthesized
printf; rewrite the mechanism using the `std.io` surface (and `|>` where it
genuinely fits), INCLUDING the verification procedure on `examples/threads`
that currently stands on those prints. The verification evidence must be
MIGRATED, never weakened."*

Base commit: main `614c70f`.

**What landed.** All 46 synthesized print sites across five C backends now
expand to one shared C-level emission route, `QCORE_IO_WRITE` / `QCORE_IO_FLUSH`
— the same route `source.std-io.v1`'s `print`, `log` and `flush` lower to — so
an emitted QCore program contains exactly **one** `fprintf` spelling, in the
route's own prelude. **Not one printed byte moved.** Nothing was deleted: the
census found no print without a reader, so the migration table has no
"removed" row. The evidence that stood on the old mechanism was migrated and,
where it was weakest, made stronger: `examples/threads` had three of four
programs with no test at all and a README transcript nothing verified, and now
all four build under `-Wall -Wextra -Werror`, run clean under ASan/UBSan/LSan,
byte-match their reference executions, and the published transcript is read out
of `README.md` and compared with a real run.

Five pins moved deliberately, each recorded in §5 with the half it keeps.
23 audited Lean theorems in a new `QCore.SingleEmissionRoute`.

---

## 1. Phase 1 — the census

### 1.1 The print sites

Every synthesized print in the repository, by producing backend. The counts are
`printf` + `fprintf` + `fflush` occurrences in each emitter, measured on
`HEAD~` and on the tree after this card.

| Backend | Sites | What it writes | After |
|---|---:|---|---|
| `crates/qcore-core/src/c_emit.rs` (sequential CFG) | 4 | `OK %d` / `ERR %u`, `%c %llu` trace events, `P <resource>` | 4 through `io_emit::write_line` / `write_text` |
| `crates/qcore-core/src/task_c_emit.rs` (checked tasks) | 16 | `QCORE_BUFFER_*`, `QCORE_TASK_*` | 14 literal `QCORE_IO_WRITE` in the fixed runtime, 2 through `io_emit::write_line` |
| `crates/qcore-core/src/mutex_c_emit.rs` (mutex guards) | 17 | 16 trace lines (`lock`/`unlock`/`open`/`close`/`read`/`write`/`grow`/…) plus 1 `stderr` runtime-failure diagnostic | 17 literal `QCORE_IO_WRITE` |
| `crates/qcore-core/src/extern_c_emit.rs` (extern calls) | 6 | 5 observations (`out …`, `call …`, `raise …`, `return`) plus 1 `stderr` declared-outcome violation | 6 literal `QCORE_IO_WRITE` |
| `crates/qcore-core/src/std_io_c_emit.rs` (`source.std-io.v1`) | 3 | `print` → stdout, `log` → stderr, `flush` | 3 through `io_emit::write_line` / `io_emit::flush` |
| **Total** | **46** | | **46 routed, 0 raw, 0 removed** |

The two `stderr` diagnostics are included deliberately. They are not trace
observations and no reader parses them, but they are writes an emitted program
performs, and leaving them raw would have made "exactly one `fprintf` spelling"
false — which is the property the whole card rests on.

### 1.2 The readers — everything that stood on those bytes

| Reader | Location | Reads |
|---|---|---|
| `parse_c_observation` | `crates/qcore-core/src/c_emit.rs:310` | the sequential harness's `OK`/`ERR`/`A`/`R`/`M`/`P` lines |
| `validate_translation` | `crates/qcore-core/src/translation_validation.rs:62` | that parse, against the reference interpreter |
| `parse_observed_trace` (F02's observer) | `crates/qcore-core/src/task_trace_relation.rs:265` | the task backend's 12 line forms |
| `render_task_observation` | `crates/qcore-core/src/task_execution/reference.rs:762` | the reference side of every task byte comparison |
| `interpret_mutex_program` | `crates/qcore-core/src/mutex_c_emit.rs:201` | the reference side of the mutex comparisons |
| `interpret_extern_program` | `crates/qcore-core/src/extern_c_emit.rs:152` | the reference side of the extern comparisons |
| `interpret_std_io_program` | `crates/qcore-core/src/std_io_c_emit.rs:56` | the reference side, per stream |

Line-for-line native/reference comparisons: `unit/c_emit.rs`,
`unit/translation_validation.rs`, `unit/mutex_c_emit.rs`, `compiler/tests/`
`{f02_trace_relation, windows_threads, source_tasks, owned_source_tasks,
scoped_read_loans, scoped_read_shares, error_payload_tasks, checked_mutex_guards,
extern_calls, unsafe_linkage, std_modules, pipe_operator, ct_corpus}.rs`, and
`crates/qcorec/tests/cli.rs`. Every one of these compares **printed bytes**, and
every one is unchanged by this card.

Sanitizer fixtures over the four backends are all Rust tests (no shell or Python
runner sanitizes backend output). All were re-run; see §6.

### 1.3 The pinned surface — what a change to emission must move

| Pin | Where | Moved? |
|---|---|---|
| F02 emission digests (round-trip, concurrent) | `compiler/tests/f02_trace_relation.rs:53-54`; `compiler/tests/windows_threads.rs:26,31` | **yes** (§5.1) |
| F02 domain/verdict digests (8 values, Rust ⇄ Lean) | `crates/qcore-core/tests/task_trace_relation.rs:243-343`; `proof/QCore/TaskTraceRelation.lean:488-507` | **no** — these are digests over the *model* trace domain, not over emitted C |
| CT34 `emitted_c_contains` growth line | `compiler/tests/fixtures/ct/CT34/expected.json:105` | **yes** (§5.2) |
| CT03/CT17/CT26/CT34 `reference_observation`, `stdout_contains`, `stdout_order`, `native_stderr` | the four `expected.json` | **no** — printed bytes unchanged |
| Emitted-C source pins in unit tests | `unit/c_emit.rs:100-110`; `unit/extern_call.rs:1326`; `compiler/tests/std_modules.rs:119-123`; `compiler/tests/scoped_read_loans.rs:290` | **yes** (§5.3) |
| Literal output transcripts | `crates/qcorec/tests/cli.rs`, `unit/translation_validation.rs`, `unit/research_closure.rs`, `unit/extern_call.rs`, `compiler/tests/{extern_calls,unsafe_linkage,owned_source_tasks,source_tasks}.rs`, `crates/qcore-core/src/task_execution/tests.rs` | **no** — printed bytes unchanged |
| `examples/threads/README.md` transcript | `README.md` §"Reading the output" | **no** bytes; the section around it was rewritten (§4) |
| `docs/reports/windows-c11-threads/f02-emission-parity.json` | archived | **no** — deliberately left frozen (§5.1) |

### 1.4 The census's own finding

**Nothing in `examples/threads` verified its output.** `build-windows.ps1 -Run`
— the only runner CI executes over the examples — checks `$LASTEXITCODE` and
nothing else. `core_context.qcore` had one test; `two_workers.qcore`,
`owned_buffer.qcore` and `shared_readers.qcore` were referenced by no test at
all. The README's seven-line transcript was prose. That was the one place where
drift in the mechanism this card moves would have gone silent, and §4 closes it.

---

## 2. The migration

### 2.1 The route

`crates/qcore-core/src/io_emit.rs` (new) is the single route:

```c
#define QCORE_IO_WRITE(stream, ...) ((void) fprintf((stream), __VA_ARGS__))
#define QCORE_IO_FLUSH(stream) ((void) fflush((stream)))
```

installed once per emitted program, after that program's own `<stdio.h>`. On
the Rust side, `io_emit::write_line`, `write_text` and `flush` build the call
for sites a backend constructs programmatically (9 of the 46); the other 37 sit
inside fixed C runtime text and spell the macro literally.

Two design decisions, both recorded in the module header:

**A macro, not a variadic wrapper.** A `static void qcore_io_write(FILE *,
const char *, ...)` would read more like a helper, and it was rejected by
measuring what it costs. These programs are built with `-Wall -Wextra -Werror`
on Unix and `/W4 /WX` on MSVC; a wrapper hides every call site's format string
from the compiler and silently drops the `-Wformat` / C4477 checking that today
catches a conversion/argument mismatch in *generated* code. The portable way to
keep it is for the call site to remain a literal `fprintf` invocation.
`__attribute__((format(printf, …)))` restores it for GCC and Clang only, and
the Windows thread subset must compile under `cl`. So the honest statement of
"one route" is the one this card can defend: **an emitted program contains
exactly one `fprintf` spelling, and every line it writes goes through it** —
which is checkable, and is checked (§3).

**The newline belongs to the helper.** `write_line(stream, body, args)` appends
`\n`; there is no argument a caller can pass that produces a partial line. The
line structure every reader in §1.2 depends on is therefore a property of the
route rather than a convention each backend follows.

### 2.2 The tie to `std.io`

The route is not merely *shaped like* `std.io`'s; it is the same one.

* `std_io_c_emit.rs` emits through `io_emit` — `print`, `log` and `flush` all
  travel it, so there is no second mechanism left in the language surface.
* The four trace backends take their **stream** from `qcore_core::std_io::Stream`,
  `source.std-io.v1`'s own type. A backend cannot name a stream that route does
  not model, because no other spelling exists.
* `c_emit.rs`'s outcome word renders an `i32` in decimal, which is exactly
  `std.fmt`'s `dec`; it now takes the conversion from
  `Rendering::Dec.c_conversion()` rather than writing `%" PRId32 "` again.

**What was deliberately not done:** the other conversions the trace lines use
(`PRIu64`, `PRIu32`, `%c`, `%s`, `%zu`, `%lu`) have no `std.fmt` counterpart and
none was added. `Rendering` *is* the source-visible `std.fmt` grammar —
`Rendering::parse` feeds the frontend — so widening it to serve a backend would
be a language change wearing a refactor's clothes. Recorded as the boundary it
is; a `std.fmt` widening is a separate card with its own surface argument.

### 2.3 The migration table

| Site class | Count | Disposition |
|---|---:|---|
| Trace observations a reader parses | 40 | **moved** onto the route, bytes unchanged |
| Runtime-failure diagnostics on `stderr` (mutex `qcore_fail`, extern declared-outcome violation) | 2 | **moved** onto the route, bytes unchanged |
| `source.std-io.v1` `print` / `log` / `flush` | 3 | **moved** onto the route, bytes unchanged |
| Dead debug noise with no reader | **0** | — |
| Kept raw because the evidence could not survive | **0** | — |

There is no "removed" row because the census found nothing to remove: every one
of the 46 sites is read by at least one of §1.2's readers, or is one of the two
diagnostics whose absence would break the single-route property. There is no
"kept raw" row because no reader's evidence turned out to depend on the raw
`printf` form — every reader consumes printed bytes, and printed bytes did not
move.

---

## 3. What is checked, and how

### 3.1 Output is byte-identical — measured, not argued

All four `examples/threads` programs were emitted, compiled and run on
`614c70f` and again after the migration, and their stdout and stderr compared:

```
two_workers.out    IDENTICAL      two_workers.c     generated-C moved (38 lines)
two_workers.err    IDENTICAL
owned_buffer.out   IDENTICAL      owned_buffer.c    generated-C moved (38 lines)
owned_buffer.err   IDENTICAL
shared_readers.out IDENTICAL      shared_readers.c  generated-C moved (48 lines)
shared_readers.err IDENTICAL
core_context.out   IDENTICAL      core_context.c    generated-C moved (38 lines)
core_context.err   IDENTICAL
```

That is the card's central claim in one measurement: **the emitted C moved and
the printed bytes did not.** Everything in §1.2 and every unmoved pin in §1.3
follows from it.

### 3.2 The single-route property, on real emitted programs

`io_emit::direct_stdio_calls` scans an emitted program and returns every line,
outside the route's prelude, carrying a direct output call. It is asserted
empty — and, non-vacuously, that the program does write through the route — for:

* every checked-task program, including all four published examples
  (`compiler/tests/thread_examples.rs`);
* every program in the sequential corpus (`unit/c_emit.rs`);
* a mutex-guard program including its `stderr` diagnostic (`unit/mutex_c_emit.rs`);
* an extern-call program including its `stderr` diagnostic (`unit/extern_call.rs`);
* a `source.std-io.v1` program (`compiler/tests/std_modules.rs`).

Spot check on a real emitted example — the only two matches in 465 lines are the
prelude's own comment and definition:

```
$ grep -n "printf\|fputs\|puts\|fwrite" two_workers.c
208:   compiler synthesizes expand to QCORE_IO_WRITE, so the only `fprintf`
213:#define QCORE_IO_WRITE(stream, ...) ((void) fprintf((stream), __VA_ARGS__))
```

### 3.3 Lean — `QCore.SingleEmissionRoute` (23 audited theorems)

The migration itself is established by measurement; a model cannot show that
bytes did not move. What Lean is worth here is the **decision procedure** that
reports "no second route", because a test that reports nothing is only as good
as the scan behind it.

* Soundness and completeness as an exact characterization in both directions
  (`empty_means_no_direct_call`, `no_direct_call_means_empty`,
  `violation_free_iff`), over a substring decision proved correct against
  `List.IsInfix` (`containsInfix_iff`).
* **Maximality** (`the_reported_spelling_is_maximal`): the spelling a violation
  names is one of greatest length among those the line contains. This is not
  decoration — `fprintf` contains `printf`, `fputs` contains `puts`, and the
  first implementation of this procedure reported the short one and was caught
  by its own test. The theorem makes the spelling table's ordering load-bearing,
  and `spellings_sorted_by_decreasing_length` decides that ordering, so
  reordering the table now breaks the build.
* Exemption is whole-line identity, not similarity
  (`a_look_alike_line_is_not_exempt`: a macro named `QCORE_IO_WRITER` that
  writes directly is still reported).
* Composition (`concatenation_is_violation_free_iff_both_are`): a program
  assembled from a prelude and several backend fragments is violation-free
  exactly when every fragment is — which is why installing the prelude once and
  routing each write is sufficient, with no whole-program argument.
* A two-sided tie: `the_domain_outcomes_are_pinned` and the Rust
  `the_domain_outcomes_agree_with_the_lean_model` walk the same eleven lines and
  independently produce `[8, 7, 7, 6, 6, 5, 4, 0, 0, 0, 0]`. The `5` is the
  `fputs` line, not the `4` a naive scan gives for the `puts` inside it: that
  single entry is the maximality property made observable.

Axioms: `propext`, `Classical.choice`, `Quot.sound` only. No new axiom, no
placeholder. `scripts/check_lean_audits.py` reports **80 audits** (was 79).

**Not proved in Lean**, stated in the module header: that the Rust emitters call
this procedure's subject (tested, not modeled); that printed bytes are unchanged
(measured); anything about C's `fprintf` semantics, buffering, interleaving or
durability. And the spelling table is a *declared* list — a program could reach
a stream through a function pointer or a platform call it does not name. That is
a real boundary of the procedure, on both sides, and it is written down rather
than glossed.

---

## 4. `examples/threads` — the verification procedure, migrated and strengthened

### 4.1 The per-site distinction, decided by measurement

The brief asked for source-meaningful prints to become declared `#import std.io;`
usage and verification machinery to stay synthesized, "the distinction per site
recorded". Measured, the distinction is uniform, and it is not a judgement call:

**A threaded module cannot carry a source-level `print` at all.** Route
selection puts `source.std-io.v1` last, after the concurrent and extern routes,
so a module that both spawns and prints stays with the more specific task model
and keeps its refusal. Both spellings of the attempt were run:

| Attempt | Result |
|---|---|
| `effects { io, throws<SpawnError>, spawn, join, may_block }` + `first_value \|> print dec;` | `Q020: unsupported effect in task execution` |
| the same without `io` in the row | `Q500: unsupported expression statement in task execution` |

So **all 4 of 4** published examples are verification machinery, and all four
stay compiler-synthesized — now routed through `std.io`'s emission path. Both
refusals are pinned in `thread_examples.rs::a_threaded_module_cannot_carry_a_source_print`,
converting the reason from an omission into an asserted decision: if route
selection ever changes, that test fails rather than this report quietly becoming
false.

**On `|>`:** the brief said to use it "where it genuinely fits — do not force
it". It fits nowhere in these four files, because no source print is expressible
in them. No new example was invented to create an occasion: a fifth `.qcore`
file demonstrating `x |> print dec` would exercise `source.std-io.v1`, which
`compiler/tests/pipe_operator.rs` already covers natively, and would make an
existing prose count of source-side `.qcore` files stale for no gain.

### 4.2 What the examples are verified by now

`compiler/tests/thread_examples.rs` (new, 5 tests) replaces the prose:

| Before | After |
|---|---|
| `core_context.qcore`: one test (creation success/failure, byte-compared) | kept, plus the below |
| `two_workers`, `owned_buffer`, `shared_readers`: **no test** | each compiled `-Wall -Wextra -Werror`, run under ASan/UBSan (LSan on Linux), stdout byte-compared with `render_task_observation` of the reference execution |
| README transcript: **prose, verified by nothing** | read out of `README.md` and compared with a real sanitized run, plus a shape check so the extraction cannot silently degenerate |
| single printing route: n/a | asserted for all four emitted programs, non-vacuously |
| `build-windows.ps1 -Run`: exit codes only | unchanged — see §7 |

Measured output, unchanged from the baseline in every case:

```
$ two_workers          $ owned_buffer                $ shared_readers
QCORE_TASK_SPAWN 1 0   QCORE_BUFFER_ALLOC 0 1 42     QCORE_BUFFER_ALLOC 0 1 1
QCORE_TASK_SPAWN 2 0   QCORE_BUFFER_ALLOC 0 2 7      QCORE_BUFFER_WRITE 0 1 42
QCORE_TASK_RESULT 2 I32 7   QCORE_TASK_SPAWN 1 0     QCORE_TASK_SPAWN 1 0
QCORE_TASK_JOIN 2      QCORE_BUFFER_READ 0 2 7       QCORE_BUFFER_LOAN 1 1
QCORE_TASK_RESULT 1 I32 42  QCORE_BUFFER_DROP 0 2    … (19 lines)
QCORE_TASK_JOIN 1      … (11 lines)
QCORE_TASK_RESULT 0 I32 7
```

`examples/threads/README.md` gains a paragraph stating plainly that none of this
output is a source construct, naming the two refusals, and pointing at the test
that verifies the transcript.

---

## 5. Deliberate pin updates

Five, each with the half it keeps.

### 5.1 F02 emission digests — the third recorded move

| Program | Before | After |
|---|---|---|
| round-trip (`fixtures/ct/CT01/case.qcore`) | `5c27711846f9a518` | `e33534d5c7da5c6c` |
| concurrent (`fixtures/f02/concurrent_round_trip.qcore`) | `0b511480da31c73d` | `910e7f975a896ff1` |

Moved in `compiler/tests/f02_trace_relation.rs` and `compiler/tests/windows_threads.rs`.

**The load-bearing half is kept, and it is not the digest.** F02's claim is that
the native rendering satisfies the admitted causal order; the digest exists so
that a change to emission *retires* the recorded native evidence rather than
silently reinterpreting it. That retirement was honoured: the relation was
re-verified against fresh native runs in the same commit, and
`the_native_rendering_relates_to_the_admitted_causal_order`,
`the_native_evidence_binds_its_whole_identity` and the adversarial refusal test
all pass on the new emission. The observer, `parse_observed_trace`, reads the
same lines it read before — those bytes did not move — so the relation is being
checked over the same information, not a substitute for it. The fresh evidence
record is `native-runs.json` beside this report.

F02's model-side digests are untouched and were verified untouched: the eight
domain and verdict digests shared between
`crates/qcore-core/tests/task_trace_relation.rs` and
`proof/QCore/TaskTraceRelation.lean` are folded over the *model* trace domain,
not over emitted C, and both sides still agree.

`docs/reports/windows-c11-threads/f02-emission-parity.json` is **deliberately
left frozen**. It is an archived record of the Windows increment's own
header-only move, and rewriting an earlier card's archive to mention a later
one is the hygiene problem wave 15 recorded under CT-LABELS. The new values live
here instead.

### 5.2 CT34 `emitted_c_contains`

`printf("grow 1 1 2 0 4\n")` → `QCORE_IO_WRITE(stdout, "grow 1 1 2 0 4\n")`.

The load-bearing half is CT34-SANITIZED's claim that `GrowProtection` really
*renders* in the C backend rather than being omitted — and that half is fully
intact: the line is still asserted present in the emitted C, with its exact
rendered content. The fixture's `reference_observation` array, including
`grow 1 1 2 0 4`, is untouched, so the observation this case compares is
unchanged in both directions. This is harness fixture territory, updated
deliberately with this record.

### 5.3 Emitted-C source pins in three test files

| Pin | Load-bearing half kept |
|---|---|
| `unit/c_emit.rs:100-110` — two indented `printf` lines | The pin is about **indentation and readability**, not about `printf`: it exists because Rust string-literal continuations once stripped the indentation. The lines are still asserted at four-space indentation. The trace line is no longer wrapped, because the route builds it as one call. |
| `unit/c_emit.rs:125` — the column-0 rule | Widened by one prefix, `/*`, because a file-scope block comment introduces the route's prelude. The rule's actual content — *no statement sits at column 0* — is unchanged, and every other allowed prefix is untouched. |
| `unit/extern_call.rs:1326` — the out-slot observation | Still asserts that the *observed* nullness, not the declared one, reaches the rendered line. |
| `std_modules.rs:119-123` — five `source.std-io.v1` emission lines | Still asserts which stream, which rendering, which level tag, which operand expression, and that the barrier is a real flush. |
| `scoped_read_loans.rs:290` — `strip_prefix` on the loan announcement | Still finds the announcement line and still asserts the lender's slot and generation are read after creation returns — the one place a loan is visibly not a move. |

---

## 6. Validation

Run on Linux x86_64, Clang 21.1.8, from this worktree at the card's final tree.

| Gate | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **2,975 passed, 0 failed, 8 ignored**, exit 0 |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py` | **80 audits** pass (was 79) |
| `python3 -m unittest discover -s scripts/tests` | 193 pass |
| Fixed sanitizer fixtures | all re-run green (§6.2) |
| `examples/threads` Linux build-and-run | executed, output in §4.2 |

### 6.1 The workspace run

One run, no retries, and **none of the recorded flakes fired** — the P02
`RLIMIT_NPROC` conformance test, the two confined-fixture tests and
`ExecutableFileBusy` all passed on the first attempt, so nothing here rests on a
re-run.

### 6.3 F02, re-verified

`native-runs.json` beside this report is the fresh evidence record, regenerated
through `QCORE_F02_EVIDENCE` on the migrated emission. Both programs relate to
their admitted causal order with `"is_reference_order": true`, at 7 and 11
events over 16 and 31 admitted edges — the same figures as the archived F02
record, over the new generated C.

### 6.2 Sanitizer fixtures

Every sanitized run over the affected backends was re-executed, since all of
them compile emitted C this card changed: `unit/c_emit.rs`,
`unit/translation_validation.rs` (8-way corpus), `unit/mutex_c_emit.rs`
(LSan on),
`compiler/tests/{checked_mutex_guards, extern_calls, unsafe_linkage, std_modules,
pipe_operator, scoped_read_loans, scoped_read_shares, error_payload_tasks}.rs`,
and `ct_corpus.rs`'s `sanitized_heap_run` (CT03, CT17) and `sanitized_c_run`
(CT26, CT34). All green. The new `thread_examples.rs` adds four more sanitized
native runs, one per published example.

---

## 7. Deviations, and what was deliberately not done

* **One acceptance-file edit, against the brief's instruction.** The brief says
  not to edit acceptance files. Adding a Lean audit makes
  `docs/reviews/r0_3/acceptance_matrix.json`'s *derived* `lean_audits` count
  (79) and `lean_audit_files` list stale, and
  `scripts/tests/test_build_matrix_sections.py` compares them against the live
  `proof/` directory — so leaving them turns a standing gate red. Two
  instructions cannot both hold. The resolution: the two derived fields were
  updated and **nothing else** — no verdict, no coverage number, no
  integrator-supplied gate figure. Flagged here rather than buried.
* **`build-windows.ps1` is unchanged.** It asserts exit codes only, which §1.4
  records as a genuine gap, but no MSVC host was available to this card and
  adding an unverified stdout assertion to a CI script is how a green gate
  becomes someone else's red one. Named follow-on **TRACE-STDIO-WIN-RUN**.
* **The Windows F02 digests are not re-measured on Windows.** They were moved to
  the Linux-measured values, on the same reasoning the Windows increment itself
  recorded: emission is host-independent apart from the embedded compatibility
  header, which this card did not touch. Honestly flagged in the test comment
  and here; a Windows run should confirm.
* **`Rendering` was not widened** to cover `PRIu64`/`%c`/`%s`/`%zu`/`%lu`
  (§2.2). That would change the source-visible `std.fmt` grammar.
* **A variadic-function route was rejected** for a measured reason, not a
  stylistic one (§2.1): it drops `-Wformat` checking on generated code.
* **No new example program** (§4.1).
* **`f02-emission-parity.json` left frozen** (§5.1), and so is
  `docs/reports/wave17-std-modules/README.md` §1.2, whose class-(b) table names
  the four backends' `printf` sites. It was true of the tree it measured; this
  report supersedes it rather than rewriting an earlier card's archive.

## 8. Named follow-ons

| Name | What |
|---|---|
| `TRACE-STDIO-WIN-RUN` | `examples/threads/build-windows.ps1 -Run` checks exit codes only. Give it the stdout comparison `thread_examples.rs` performs on Unix, and run it on a real MSVC host. |
| `TRACE-STDIO-WIN-DIGEST` | Re-measure the two F02 emission digests on Windows and confirm the Linux values, closing §7's flagged assumption. |
| `STD-FMT-CONVERSIONS` | The trace backends' non-`dec` conversions have no `std.fmt` counterpart. Deciding whether `std.fmt` should grow an unsigned/character vocabulary is a source-surface question with its own grammar and refusal argument. |
| `TRACE-STDIO-ROUTE-COMPLETENESS` | `direct_stdio_calls`'s spelling table is declared. A program could reach a stream through a function pointer or a platform call it does not name; the procedure cannot see that, on either side. |

---

## 9. Draft increment paragraph

> Single emission route (TRACE-STDIO, 2026-09-10): the compiler-synthesized
> `printf` trace instrumentation in the four concurrency/sequential C backends
> and `source.std-io.v1`'s own `print`/`log`/`flush` — 46 sites — now expand to
> one shared C-level route (`QCORE_IO_WRITE`/`QCORE_IO_FLUSH`,
> `qcore_core::io_emit`), taking their stream from `std.io`'s own `Stream` and
> the outcome word's conversion from `std.fmt`'s `dec`, so an emitted program
> contains exactly one `fprintf` spelling. **No printed byte moved**, measured
> before and after on all four published examples; the emitted C moved, and
> five pins were deliberately updated with their load-bearing halves kept: the
> two F02 emission digests (relation re-verified against fresh native runs;
> F02's model-side digests untouched), CT34's `emitted_c_contains` growth line,
> and three emitted-C source pins. Nothing was removed — the census found no
> print without a reader. The `examples/threads` evidence was migrated and
> strengthened: three of the four programs had no test at all and the README
> transcript was verified by nothing, and all four now build under
> `-Wall -Wextra -Werror`, run clean under ASan/UBSan/LSan, byte-match their
> reference executions, and have the published transcript read out of
> `README.md` and compared with a real run. A source `print` is measured
> impossible in a threaded module (Q020 with the `io` effect, Q500 without),
> pinned, so all four sites are verification machinery by measurement rather
> than by choice. 23 audited Lean theorems (`QCore.SingleEmissionRoute`) decide
> the single-route procedure — soundness, completeness, and the maximality that
> makes the spelling table's ordering load-bearing — with a two-sided eleven-line
> tie. Checker semantics, diagnostics, ledgers and verdicts are unchanged and the
> corpus is untouched by construction: no analyzer, contract, profile or gateway
> file is in the diff. Evidence:
> `docs/reports/wave19-trace-stdio/README.md`.
