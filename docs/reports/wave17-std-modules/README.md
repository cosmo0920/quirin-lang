# STD-MODULES — `std.fmt`, `std.io`, `std.logger` (wave 17)

Card brief: *"Audit and provide `std.fmt`; `std.io`; `std.logger` — format and
logging features. Some of the QCore sources use this feature implicitly, so we
need to tidy up the feature."* Plus a mid-card amendment on an asynchronous,
non-blocking logger with barriers (§8).

Base commit: main `8ec8402`.

The headline of this card is a **measurement, and it contradicts the card's own
premise**: the implicit source-visible surface is **empty**. QCore source has no
printing, logging, formatting, string-interpolation or IO construct of any kind,
and never had one. What looked like implicit output is two other things — a
compiler-synthesized trace harness in the C backends, and an `io` *effect label*
stamped on every extern call regardless of what the callee does. Both are
recorded below with evidence. Because there was nothing to migrate, the "tidy
up" took the only other fail-closed form available: the three modules are
**provided** with a real, minimal, checked v1 surface, and every one of their
capabilities is refused without its own import.

---

## 1. Phase 1 — the census

Two independent sweeps agree: a keyword census over every source `.qcore` file,
and a direct reading of the frontend's statement/type/intrinsic surface.

The reproducible half is `census.py` beside this report
(`python3 docs/reports/wave17-std-modules/census.py` from the repository root),
scanning all **290** source-side `.qcore` files — everything outside
`contract/qcore/`, which holds declarative C contracts rather than source
programs. Its result, on the tree *after* this card:

```
source-side .qcore files scanned: 290
string literals outside comments: 0
character literals outside comments: 0
output-shaped statement heads: 8      <- all eight are this card's own fixtures
```

Zero string literals and zero character literals anywhere outside comments, and
the only output-shaped statement head in the entire repository is the syntax
this card introduced. Before this card the last number was **0**.

### 1.1 Class (a) — implicit source-visible formatting/logging/IO: **EMPTY**

| Probe | Result | Evidence |
|---|---|---|
| A print/output statement | none | `StatementKind` has 21 variants and none of them writes: `Let`, `LetIndex`, `Assignment`, `Expression`, `Drop`, `Move`, `Discharge`, `Guard`, `If`, `Match`, `While`, `Return`, `Raise`, `Break`, `Continue`, `Defer`, `ErrorRegion`, `Unsafe`, `ThreadScope`, `Footprint`, `WithLock` (`crates/qcore-frontend/src/frontend.rs:174-227`) |
| A logging construct | none | `census.py` over all 290 source-side `.qcore` files: no `print`, `log`, `emit`, `trace`, `format` at any statement head. The only `log`-named things in the repo are `flb_log_print` / `cio_log_error` / `w_print` inside **contract** files under `contract/qcore/`, which model C callees and are not callable from a source program |
| String interpolation / a format string | none | no expression tree exists at all: `StatementKind::Expression` is a payload-free marker |
| A string or character type | none | the atomic type list is `unit bool i8 i16 i32 i64 u8 u16 u32 u64 isize usize f32 f64` (`frontend.rs:1037-1040`); `SourceTypeKind` has 15 variants, none textual; `ExecutableResultType` is `I32` or `Unit` |
| A string literal as a value | none | `TokenKind::StringLiteral` exists in the lexer but is consumed at exactly two sites, both `verifier scope` plugin metadata (`frontend.rs:786`, `:806`). `census.py` measures zero string and zero character literals outside comments across all 290 files |
| Stream/file IO through an intrinsic | none | the complete intrinsic set is `qcore_buffer_new/read/write`, `qcore_error_payload`, `qcore_acquire`, `qcore_check` |
| A `read`/`write` statement | present but **not IO** | `read`/`write NAME [with GUARD]` are guarded shared-**memory** accesses in `source.mutex.guard.v1` and `source.epoch.bound.v1`; in emitted C they become a `memset` over a model byte interval and a checksum sum (`mutex_c_emit.rs:402,410`) |

**This class being empty is the card's principal finding.** It is why nothing
was migrated: there was no implicit source construct to put behind a module.

### 1.2 Class (b) — backend-internal trace emission: real, and not source-visible

Every `printf`/`fprintf` in the repository is synthesized per-IR-event by a
backend and read back by the compiler for translation validation. No source
construct requests, names, parameterizes or suppresses it.

| Backend | What it writes | Driven by |
|---|---|---|
| `qcore-core/src/c_emit.rs:154-166` | `OK %d` / `ERR %u`, then `%c %llu` trace events, then `P <resource>` per retained owned resource | the CFG walk |
| `qcore-core/src/task_c_emit.rs:479-483, 534-544` | `QCORE_BUFFER_*`, `QCORE_TASK_RESULT …` | the buffer-event ring |
| `qcore-core/src/mutex_c_emit.rs:344-462` | `lock`/`unlock`/`open`/`close`/`read`/`write`/… | per `MutexEvent` |
| `qcore-core/src/extern_c_emit.rs:309-395` | `acquire …`, `raise …`, `return` | per `ExternEvent` |

`c_emit.rs:311` reads that same stdout back (`fn …(stdout: &str)`), which is
what makes it a harness protocol rather than a language feature. **Recorded as
out of scope for this card**, per the brief. It is worth stating plainly that
this is the thing a reader would most plausibly mistake for "implicit output":
every emitted program prints, and no source program asked it to.

### 1.3 Class (c) — extern-gated usage: measured, not moved

All nine declared extern functions in the fixtures
(`compiler/tests/fixtures/extern_calls/contracts/buffer_api.qcore`) are
alloc/fill/sum/release/out-slot/attach. **None prints, logs or does stream IO**,
and none could: the extern argument grammar admits only integers, bindings and
the `out` marker, and no `AbiType` can carry a path or a format string, so a
`printf`-shaped callee is not expressible.

The one genuinely loose thread is the **`io` effect label**:

```rust
// crates/qcore-core/src/extern_call.rs:594-596
pub fn derived_effects(callee: &ExternCallee) -> EffectRow {
    let mut effects = BTreeSet::from([Effect::Io]);
```

`Effect::Io` is stamped on **every** extern call unconditionally, whatever the
callee does. The four source fixtures declaring `effects { io, alloc, mut }`
declare it only to satisfy that containment check. So before this card the
language had a capability *name* for IO with **nothing gating it and nothing
performing it**. That is the closest thing to the brief's "used implicitly", and
this card's answer is that `Effect::Io` now has an operation that really
performs it and a module that really gates it. The extern route's own use of the
label is untouched, per the brief's class-(c) instruction.

---

## 2. Phase 2 — what was provided

Modules provided with a real checked surface: **`std.fmt`, `std.io`,
`std.logger`** — all three. No skeletons; no module was provided without an
operation. Named non-needs are in §7.

New route `source.std-io.v1` /
`SourceExecutionProfile::StandardIoV1`. Grammar (also now in
`docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` §G02g):

```ebnf
stmt      = "let", NAME, "=", [ "-" ], INTEGER, ";"
          | "print", RENDERING, OPERAND, ";"          (* std.io  + std.fmt *)
          | "log", LEVEL, RENDERING, OPERAND, ";"     (* std.logger + std.fmt *)
          | "flush", ";"                              (* std.logger *)
          | "return", ";" ;
RENDERING = "dec" | "hex" ;
LEVEL     = "info" | "warn" | "fatal" ;
OPERAND   = [ "-" ], INTEGER | NAME ;
```

| Module | v1 surface | Justification for the size |
|---|---|---|
| `std.fmt` | two renderings, `dec` and `hex` | `dec` is exactly the signed-decimal the sequential backend already renders results with. `hex` is the **one** construct this card adds beyond the backend's existing repertoire, present so the selector is non-vacuous and a wrong-rendering refusal is testable in both directions. A one-member vocabulary would have been untestable. |
| `std.io` | `print RENDERING OPERAND;` → one line on stdout | the minimum that makes standard output a source-chosen act rather than a harness artifact |
| `std.logger` | `log LEVEL RENDERING OPERAND;` → one level-tagged line on stderr; `flush;` → sink handoff | the level set is three because a severity with one inhabitant carries no information; `flush` is the amendment's barrier (§8) |

Files: `crates/qcore-core/src/std_io.rs` (model, checker, certificate, recheck),
`crates/qcore-core/src/std_io_c_emit.rs` (reference execution + C11 emission),
`crates/qcore-frontend/src/source_std_io.rs` (grammar and lowering),
`crates/qcore-core/src/module_imports.rs` and
`crates/qcore-frontend/src/module_imports.rs` (registry and gates),
`crates/qcore-verification/src/driver.rs` (route).

---

## 3. The effect discipline, and its argument

**A rendering cannot touch custody, and that is structural rather than
checked.**

```rust
pub enum StdIoOperand {
    Literal(i32),
    Scalar(String),
}
```

`StdIoOperand` has two constructors and **neither can denote a resource**. There
is no operand form for an owned buffer, a borrow, a loan share or a handle. So a
`print` or `log` cannot consume, release, store or retain custody of anything —
not because the checker refuses it, but because the profile cannot express it.

Consequences, each pinned by a test:

* the derived effect row is exactly `{Io}` for a writing program and empty
  otherwise — **never `Alloc`, never `Mut`**
  (`the_derived_effect_row_is_exactly_io`);
* a real resource binding is refused twice over — as a binding (`Q030`) and as
  an operand (`Q028`) — using `qcore_acquire`, which is a genuine resource
  acquisition reachable *without* the concurrency import, so the test is not
  vacuous (`rendering_a_resource_refuses`, measured `["Q030", "Q028"]`);
* an owned Buffer is refused earlier still, at the concurrency import gate
  (`rendering_an_owned_buffer_refuses`, measured `["Q015"]`), pinned so the
  buffer form cannot quietly become renderable if that gate moves;
* `print dec move total;` is refused (`a_moved_operand_refuses`).

This is **stronger** than borrow-only. It is deliberately stronger: the brief
expected borrow-only as the sound v1 answer, and refusing the whole class is the
fail-closed choice that needs no custody argument at all. It is also consistent
with the sentence the spec already carried at §L11 — *"Matching an error,
logging it, or replacing its tag does not release its payload."* Admitting a
**borrowed** render of a live resource is out of v1 and named
**`STD-FMT-BORROW`**; it requires its own custody argument, and the
`CheckedStdIoModule` doc comment records that such a widening must also start
carrying the sequential passes' evidence, which this route legitimately does not
today.

### Interaction with the concurrency profile

Measured and refused, not assumed: a `print` inside a `thread scope` stays with
the task route and is refused there
(`printing_inside_a_thread_scope_keeps_the_task_routes_refusal`). Route
selection puts `source.std-io.v1` **last**, after the concurrent and extern
routes, on the same principle the extern route already used — a module that both
prints and spawns stays with the more specific model and keeps its refusal. In
Lean, `concurrency_grants_no_output_capability` records that
`std.concurrency` grants none of the three capabilities: a threaded module gets
no free output channel.

---

## 4. Migrations, refusals, and the tidy-up

**Migrated implicit-use sites: 0** — because the census found none (§1.1). This
is the honest count. No existing `.qcore` file changed; no golden surface moved;
no pinned expectation was edited.

**New refusals introduced** (all previously *inexpressible* rather than
permitted, since the constructs are new):

| Refusal | Code | Test |
|---|---|---|
| `print` without `import std.io;` | `Q015` | `printing_without_the_io_import_refuses` |
| `log` without `import std.logger;` | `Q015` | `logging_without_the_logger_import_refuses` |
| `flush` without `import std.logger;` | `Q015` | `flushing_without_the_logger_import_refuses` |
| a rendering without `import std.fmt;` | `Q015` | `rendering_without_the_fmt_import_refuses` |
| `std.io` alone does not grant the logger | `Q015` | `the_io_import_alone_does_not_grant_the_logger` |
| `std.logger` alone does not grant stdout | `Q015` | `the_logger_import_alone_does_not_grant_standard_output` |
| a misspelled module (`std.fmtt`, `std.stdio`, `std.log`) | `Q014` | `a_misspelled_module_refuses_as_an_unknown_module` |
| another file's imports do not grant this one | `Q015` | `importing_the_modules_in_another_file_does_not_grant_this_one_the_capability` |
| a resource operand | `Q028` | `rendering_a_resource_refuses` |
| writing without declaring `io` | `Q029` | `writing_without_declaring_io_refuses` |
| shape violations (unknown reserved-word rendering/level, `flush` with an operand, printing outside the entry, no return) | `Q030` | five tests |
| a tampered certificate | recheck failure | `the_published_certificate_is_independently_rederivable` |

`Q015` is **reused**, not duplicated: the brief asked to prefer reusing
`Q014`/`Q015`, and "this file used a capability it did not import" is exactly
what `Q015` means. The message names the module, so the three capabilities are
distinguishable without three codes. `Q028`–`Q031` are new, taken under a new
`StandardIo` semantic pass through the §L21 process, with spec §L21 and the
implemented grammars updated **in the same change** as the implementation.

Also pinned: **importing is not using.** A file that imports all three and never
prints stays on the sequential route
(`a_module_that_imports_without_printing_needs_no_route`), and a logger-only
file that only flushes needs no `std.fmt`
(`a_logger_only_module_needs_no_rendering_import`) — measured, not assumed, and
the reason `flush` is deliberately not a rendering.

---

## 5. Reserved words and the golden-surface collision

Eight words newly reserved: `print`, `log`, `flush`, `dec`, `hex`, `info`,
`warn`, `fatal`.

Checked against every `.qcore` file in the repository. Three occurrences of
candidate words exist (`print`, `log`, `fatal`) and **all three are inside `//`
comments** in contract files — no collision.

**One real collision, resolved by renaming this card's syntax.** The natural
severity spelling `error` is already the reserved error-type declaration keyword
with 34 uses across the source corpus. Per the `work` → `effort` precedent of
card T08-EPOCH-SRC, the *syntax* was renamed rather than the pinned surface
edited: the level is spelled **`fatal`**. The refusal is pinned as a test
(`the_error_level_spelling_refuses_because_error_is_reserved`) so the decision is
recorded in executable form, not only in prose.

Reserving these words is load-bearing in the same measured way `grow` and `rank`
are: without it the statement collects `print`/`dec`/… as referenced names and
refuses at `Q013`. Both sides of that boundary are pinned — see §6.

---

## 6. Deviations found by measurement

1. **`Q028` is not the code for an unbound operand.** The established
   name-resolution gate runs before this route, so `print dec missing;` refuses
   at `Q013`, not `Q028`. Measured, and the test was corrected to assert the
   truth rather than the design's expectation
   (`rendering_a_name_nothing_binds_refuses_at_name_resolution`). `Q028` is
   still reachable and non-vacuous: it fires for a name that *is* bound but is
   not one of the entry's integer scalars, which is exactly the resource case.
2. **An unknown rendering or level word refuses at `Q013`, not `Q030`.** An
   unreserved word in that position is collected as a referenced name. The
   refusal is correct and fail-closed; only the message is less specific. Both
   halves of the boundary are pinned — the unreserved case at `Q013`
   (`an_unknown_rendering_refuses`, `an_unknown_level_refuses`) and the reserved
   case reaching this route's own `Q030`
   (`a_reserved_word_in_a_rendering_position_reaches_this_routes_refusal`). The
   alternative — reserving every possible misspelling — would be wrong. Named
   follow-on **`STD-FMT-DIAGNOSTIC`**.
3. **The spec's module registry sentence was stale.** §L04 read "The current
   registry contains `std.concurrency`" and had not been updated when wave 5
   added `std.extern`. Corrected to name all five.
4. **The Lean `Imports` structure literals grew three fields.** Existing theorem
   *names and meanings* are preserved; the proofs were restructured around a
   `setterFor` formulation and one generic
   `admitted_capability_requires_its_module`, so all five modules are now gated
   by literally the same argument and none can drift. Recorded as a deliberate
   widening.

---

## 7. Non-claims and named non-needs

**Non-claims.** This is a bounded v1 module surface, not stdlib conformance.

* **No format-string language.** A rendering is one of two closed constructors;
  nothing in a source program supplies a template. There is no parsing of format
  specifiers to guarantee anything about.
* **No durability.** `flush` is `fflush` — *sink handoff*. No `fsync`, no device
  write, no crash survival is claimed or measured (the O02/O03 discipline).
* **No stream interleaving.** Each stream's own line order is checked; the
  relative order of stdout and stderr is not modeled, not asserted anywhere, and
  not measured. C gives no such order between two separately buffered streams.
* **No logger configuration model.** A level is a label; it selects no stream,
  filters nothing, and no consumer behavior is specified. The census demanded
  none.
* **No text, no encoding, no files.** No string or character value, no path, no
  descriptor, no file IO.
* **Rendering agreement is compared, not proved.** That the C `printf`
  conversions render what the Rust `Rendering::render` renders is established by
  per-stream comparison against a real run, not derived from a model of C's
  conversion specifiers.
* **The Lean theorems do not prove the Rust lowering faithful.** Source token
  extraction and driver wiring are tested Rust boundaries, as in the
  `ConcurrencyPrelude` precedent.

**Named non-needs** (surface deliberately not built, because the census
justified none of it): file/stream opening, descriptors and paths; log filtering,
sinks, appenders or configuration; structured/keyed logging; a `Display`-style
user-extensible rendering; string values and interpolation; width, precision,
padding or locale; timestamps in log lines.

**Named follow-ons.**

| Name | What it is |
|---|---|
| `STD-FMT-BORROW` | admit a borrowed render of a live resource, with its own custody argument and the sequential-pass evidence a widened route would owe |
| `STD-FMT-DIAGNOSTIC` | a specific message for an unreserved word in a rendering/level position, which today refuses correctly but generically at `Q013` |
| `STD-LOGGER-ASYNC` | the asynchronous logger — see §8, with the measured blocker |
| `STD-IO-COMPOSE` | composing this route with the task or extern routes, each of which is a separate checked model |

---

## 8. The amendment: an asynchronous, non-blocking logger with barriers

**Outcome: the synchronous logger is the checked v1 surface. The async logger is
recorded as named follow-on `STD-LOGGER-ASYNC` with a measured blocker.** This
is the outcome the amendment explicitly listed as fully valid, and it is not a
matter of effort — the ledger capability required does not exist.

### 8.1 The measurement

An async logger in this language means a background writer reached through the
checked constructs, with messages handed to it after it starts. Two independent
findings say v1 cannot express that honestly:

1. **The task ledger has no post-creation transfer.** Its complete operation set
   is `acquire`, `use_resource`, `read_resource`, `drop_resource`, `enter_scope`,
   `exit_scope`, `reserve` / `reserve_with_loans`, `rollback`, `commit`,
   `complete`, `join`, `fail_join`
   (`crates/qcore-core/src/scoped_tasks.rs`). Every parent→child custody edge is
   established at **creation** (`reserve`/`commit`). There is no `send`,
   `enqueue` or transfer-to-a-running-child. A bounded queue whose custody is
   modeled therefore cannot be written: the messages would have to cross the
   boundary after the worker exists, and no ledger operation says that.
2. **The checked mutex/guard backend emits one thread.** Its own module header
   states it: *"This backend emits **one** thread"*
   (`crates/qcore-core/src/mutex_c_emit.rs:4`, `:83`), and a trace carrying task
   structure is checked but has no native rendering. So the mutex profile cannot
   supply a real concurrent sink either.

Faking it was the alternative and is excluded by the brief and by this
repository's rules: a hidden thread or unmodeled queue inside a logger runtime
would be exactly the unmodeled effect channel §3 exists to refuse, and
pretending asynchrony in the reference interpreter while emitting something else
natively would break the line-for-line comparison the route rests on.

### 8.2 What was landed from the amendment

**The barrier.** `flush;` is implemented, checked and executed. Its semantics
are defined checkably and narrowly: it emits a real `fflush` on the logger's
stream and **produces no line**, so it never moves either stream's observation —
which is what makes it comparable against a native run that produces no extra
bytes. Two Lean theorems state exactly that
(`flush_changes_no_line_count` over an arbitrary insertion point,
`repeated_flush_changes_no_line_count`), and a third records that it is not
inert in the effect row (`the_barrier_still_carries_io`): a barrier still needs
`Io`, so it still needs its module. The `logger_only.qcore` fixture runs
natively under sanitizers and writes nothing on either stream.

Per the amendment's §3: this is **sink handoff, not durability**. No `fsync` is
performed and none is claimed. On whether the existing `wait m;` could express a
barrier — it could not: `wait` is a mutex-profile acquisition announcement on a
route that emits one thread, so it names no sink and hands nothing off. `flush`
is a new checked operation under the reserved-word discipline of §5.

**Non-blocking on write** is therefore not a v1 property: v1's write is
synchronous, and saying so is more useful than a policy for a queue that cannot
exist. When `STD-LOGGER-ASYNC` is taken up, the amendment's requirement stands —
a full-queue policy must be an explicit declared policy (drop-with-count or
block), never silent unboundedness — and the ordering comparison will need the
F02 task-trace-relation treatment (comparison up to an admitted reordering) or
the feature is not v1-honest.

**Sanitizer non-claim, carried forward:** ThreadSanitizer does not intercept C11
`thrd_create` (measured in T04), so were an async logger landed, data-race
freedom would rest on the checker's discipline and not on measurement. This card
runs no threaded native program, so the point is recorded rather than relied on.

---

## 9. Evidence

### 9.1 Tests

`compiler/tests/std_modules.rs` — **31 tests, all passing**, including two
sanitized native runs. Fixtures: `compiler/tests/fixtures/std_modules/lines.qcore`
(all three modules, both renderings, all three levels, a barrier) and
`logger_only.qcore` (a bare barrier).

### 9.2 Sanitizer evidence — reported honestly

Two fixed ASan/UBSan/LSan runs, x86_64 Linux, `-std=c11 -Wall -Wextra -Werror
-pedantic -g -O0 -fsanitize=address,undefined -fno-omit-frame-pointer`, with
`ASAN_OPTIONS=detect_leaks=1:halt_on_error=1` and
`UBSAN_OPTIONS=halt_on_error=1:print_stacktrace=1`:

| Fixture | Native result |
|---|---|
| `lines.qcore` | compiles clean under `-Werror`; runs with exit 0; **stdout matched the reference's stdout projection and stderr matched the stderr projection, per stream** (`the_emitted_program_matches_the_reference_per_stream`) |
| `logger_only.qcore` | compiles clean; exit 0; wrote nothing on either stream, as the barrier model predicts |

Honest scope of this evidence: these programs allocate nothing, so the leak
check is a *negative control* — it confirms the emitted program introduces no
allocation, which is itself the effect discipline's prediction, and it is not
evidence about a route that does allocate. The brief asked for one fixture per
module that emits real output through the native path; `lines.qcore` covers
`std.fmt`, `std.io` and `std.logger` in one program, and `logger_only.qcore`
covers the barrier alone. Apple ASan provides no LSan, so leak detection stays a
Linux-only observation (the existing convention).

### 9.3 Lean

`proof/QCore/ModuleImports.lean` (registry generalised two → five modules) and
the new `proof/QCore/StdIo.lean` (effect discipline, stream separation, barrier,
gating). `proof/AuditStdIo.lean` audits **45 theorems**; `QCore.lean` imports
`QCore.StdIo` so `AuditAll.lean` includes it in the full axiom closure. **No new
axiom and no placeholder** — every audited theorem depends on nothing, or only
on the allowed `propext` / `Classical.choice` / `Quot.sound`.

Load-bearing statements: `no_program_allocates`, `no_program_mutates`,
`derived_row_is_io_or_empty` (the effect discipline);
`print_never_writes_stderr`, `log_never_writes_stdout`,
`every_level_uses_one_stream` (stream separation);
`flush_changes_no_line_count`, `the_barrier_still_carries_io` (the barrier);
`admitted_capability_requires_its_module` and its five instantiations,
`no_module_substitutes_for_another`, `concurrency_grants_no_output_capability`
(gating).

### 9.4 Corpus neutrality

**Corpus-neutral by construction.** This is source-language work: no gateway,
contract-loader, audit or provider file was touched, no `contract/qcore/` file
changed (corpus stays **67 modules**), no CT manifest entry changed (**32/50**),
no pinned artifact was edited. The expectation to verify at consolidation is
that the canonical CMetrics artifacts and the 185-unit baseline are
**byte-identical**, with the headline unchanged at 2 Safe / 42 provisional
Unsafe / 137 Unknown / 4 data-only.

### 9.5 Gate

Full command evidence is in `gate.txt` beside this report. Summary:

| Gate step | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **169 suites pass, 1 fails** — the failure is the recorded P02 `RLIMIT_NPROC` load-flake, verified clean in isolation (12/12) and deliberately not weakened |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean; three of this card's own lints were found and fixed, not suppressed |
| `reevaluation_probe` / `reevaluation_followup` | exit 0 / exit 0 |
| `check_lean_audits.py` | **73/73 pass** (72 → 73 for this card's new audit); no new axiom |
| `python3 -m unittest discover -s scripts/tests` | 161 tests, 1 expected failure — see below |
| `cargo test -p qcore-compiler --test std_modules` | 31 pass, including both sanitized native runs |

Two environment findings, recorded rather than worked around:

* The `third_party/libevent` submodule was never initialized in this fresh
  worktree, so three suites failed on the pinned-revision check. The pin was
  **not** weakened; the gate's own documented
  `git submodule update --init --recursive` fixed it, and all three were
  re-run individually and pass.
* The shared disk filled to 100% three times during the card (five sibling
  worktrees at 25–36 GB of build artifacts each), killing two workspace runs
  mid-flight. This card's build tree was pruned twice, returning ~32 GB to the
  pool, and the suite then completed. **No result above is inferred from a
  truncated run** — the 169/1 figures come from one complete run.

**One expected failure, left for the integrator.** `scripts/tests` reports
`test_the_derived_facts_are_recomputed_rather_than_carried` failing with
`lean_audits: 72 != 73`. That is the derived-value tripwire working exactly as
designed: this card adds one audited Lean module, and
`docs/reviews/r0_3/acceptance_matrix.json` carries the pre-card count. The base
commit `8ec8402` was self-consistent (72 audit files, matrix value 72), so the
delta is entirely this card's and is not a pre-existing failure. **This card's
brief forbids editing acceptance files**, so the matrix is deliberately left
stale for the wave integrator to regenerate — the same handling every wave's
derived sections receive.

---

## 10. Draft increment paragraph

> Standard output modules (STD-MODULES): the census contradicts the card's
> premise and is the result — QCore source has **no** printing, logging,
> formatting or IO construct, and never had one (21 statement kinds, none
> writing; no string or char type; zero string and character literals outside
> comments across 290 source-side files; the complete intrinsic set is six model operations). What
> looked implicit is two other things, both recorded: compiler-synthesized trace
> instrumentation in four C backends, read back for translation validation, and
> an `Effect::Io` label stamped on every extern call regardless of what the
> callee does — a capability *name* with nothing gating it and nothing
> performing it. With nothing to migrate, `std.fmt`, `std.io` and `std.logger`
> are provided instead, as `source.std-io.v1`: two renderings, a line to stdout,
> a level-tagged line to stderr, and a `flush` barrier, each gated by its own
> module through the existing `Q014`/`Q015` (no new import code) and refused
> without it, with `Q028`-`Q031` under a new `StandardIo` pass through the §L21
> process. The v1 effect discipline is **structural rather than checked**: the
> operand type has two constructors and neither can denote a resource, so a
> rendering cannot consume, release, store or retain custody — stronger than the
> expected borrow-only, and the derived row is exactly `{Io}`, never `Alloc`,
> never `Mut`. Two sanitized native runs match the reference **per stream**; no
> interleaving between stdout and stderr is modeled or claimed. The amendment's
> asynchronous logger is refused on a measured blocker rather than faked: the
> task ledger's only parent-to-child custody edge is at creation — there is no
> transfer-to-a-running-child operation — and the checked mutex backend emits one
> thread, so a bounded queue cannot be modeled honestly; the barrier half landed
> as sink handoff (`fflush`), explicitly not durability. `error` was measured to
> collide with the reserved error-type keyword, so the severity is spelled
> `fatal` (the `work` → `effort` precedent); the stale §L04 registry sentence is
> corrected to all five modules. 31 new tests, 45 audited Lean theorems, no new
> axiom. Corpus-neutral by construction: contract corpus stays 67 modules, CT
> stays 32/50, nothing promoted. Evidence:
> `docs/reports/wave17-std-modules/README.md`.
