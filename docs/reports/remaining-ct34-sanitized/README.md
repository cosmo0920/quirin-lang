# CT34-SANITIZED — emitting a protection growth, and what the native run is evidence of

Wave 14. Card CT34-SANITIZED (CT34-GROWTH-NATIVE): decide the emission semantics
of `MutexInvariantEvent::GrowProtection`, implement it under exactly the
checker-admitted conditions, run a fixed sanitized native fixture, and wire
CT34's `native_sanitized` stage to that run.

Base: `main` at `009bde0`. All work is on this card's branch.

Card CT34-GROWTH (wave 12) recorded, as its boundary **CT34-GROWTH-NATIVE**,
that the growth route was check-only by pinned decision: "nothing in the C
backend reads a generation's declared shape", so both `emit_mutex_c11` and
`interpret_mutex_program` refused a `GrowProtection` event outright rather than
emit an `mtx_t` "protecting less than the model says". The wave-13 CT lane then
had to invent an `accept_check_only` control expectation to pin that refusal.

This card retires that boundary. It does **not** retire it by making the C mutex
protect more memory — no `mtx_t` can express an extent — but by answering the
question the boundary was avoiding: *what is a growth, at C level?*

## 1. The emission decision, derived from the model

### 1.1 A footprint is a declared shape

`CheckedMutexInvariantV1` protects a footprint by **discipline**, not by
hardware or by any library mechanism. The checker decides which accesses may run
under which guard; `DeclaredFootprintShape` is the standing premise that the C
layout matches what was declared. The emitted `mtx_t` is what makes the sections
mutually exclusive; it has never known, and cannot know, which bytes it
"protects". That is as true of a generation's *original* declared shape as of a
grown one — `create 1` emits `mtx_init` and says nothing about `x[0,4)`.

So the reasoning behind CT34-GROWTH-NATIVE — an emitted mutex would protect less
than the model says — is true of this backend's every generation, before any
growth. It is a statement about what a C mutex is, not about growth. Refusing to
emit growth on that ground refuses something the backend was never doing.

### 1.2 What growth therefore renders as

A growth changes a **declared shape**. At C level it is:

* **no operation on the `mtx_t` at all** — no acquire, no release, no
  re-initialisation, nothing; and
* the one **bookkeeping line** `grow <mutex> <guard> <resource> <start> <end>`,
  so the emitted program's observation and the reference execution stay
  comparable line for line.

The interesting consequence is not the growth statement itself but what the
checker admits *after* it: accesses to the grown region under that guard, which
the checker would have refused at `Q610 UnprotectedFootprint` before (measured,
§5 M7). Those lower to ordinary
reads and writes of a real allocation, inside the growing section's own
`mtx_lock`/`mtx_unlock` pair. **That** is what a sanitized run can witness, and
it is what the fixture is built to exercise (§3).

`crates/qcore-core/src/mutex_c_emit.rs` states this in its module header, so a
reader meets the non-claim before the code.

### 1.3 The admission: the backend trusts only checked modules

A growth renders **only** where the supplied certificate admits it. `admitted_growth`
requires an `AdmittedProtectionGrowth` matching, field for field, the event's

* event index, task, guard, interval and new predicate identity, and
* the generation — taken from `holding_mutex`, i.e. from the `Lock` the trace
  really minted, never from the record, so a record cannot name a generation the
  trace never locked —

and requires the certificate's own **effective** `protected_shape` to carry the
grown interval. That second binding is deliberate redundancy: a certificate that
admitted a growth its published shape denies is a certificate disagreeing with
itself, and is refused rather than rendered.

Anything else is `MutexEmissionRefusal::UnadmittedProtectionGrowth { event }`,
in **both** renderings. There is no path from an unchecked growth to emitted C.

`resource_extents` now also checks a growth's interval against the declared
grants, so a grown interval outside its allocation is `UndeclaredResource` —
the same rule every other footprint has always been under.

### 1.4 What is deliberately not claimed

* That a C mutex protects the grown region. Never claimed here or anywhere.
* Anything about contention, scheduling, publication or the C memory model. One
  thread, one target, one compiler invocation, exactly as the T03 backend has
  always been scoped.
* Anything about the invariant predicate, which stays `InvariantPredicateDeclared`.
* Card CT34-GROWTH's own boundaries are untouched: growth by a non-creator is
  still refused (**CT34-GROWTH-CREATOR**), both W5 halves still share `Q621`
  (**CT34-GROWTH-Q621**), a refused growth still produces the seven-entry
  cascade (**CT34-GROWTH-CASCADE**), and shrinking and replacement are still
  outside the profile entirely. No checker rule, no diagnostic code and no
  lowering changed in this card.

## 2. The pin update, in its new form

Card CT34-GROWTH's assertion lived in `emission_refuses_anything_outside_the_single_task_subset`,
where a hand-built `GrowProtection` was spliced beside `Spawn`, `TransferCustody`,
`RegisterWaiter` and `UnsupportedConstruct` and expected `UnsupportedEvent { event: 1 }`.
A growth is no longer outside the subset, so it leaves that loop — the other four
rows are byte-identical — and its assertion becomes two tests:

| Test | What it pins |
|---|---|
| `a_grown_generation_agrees_between_the_reference_and_a_sanitized_native_run` | the **emission**: one admitted growth, effective shape `[x[0,4), y[0,4)]`, a `grow 1 1 2 0 4` line in the reference, exactly one `mtx_init`, two `mtx_lock`, two `mtx_unlock` and one `mtx_destroy` — the same six a two-section program without a growth performs — both allocations freed, and the compiled program's stdout equal to the reference under ASan/UBSan/LSan |
| `an_unadmitted_protection_growth_is_refused_by_both_renderings` | the **load-bearing refusal half**, kept and widened |

The refusal test carries eleven rows, each refusing in both renderings:

* a growth spliced into a trace whose certificate never saw one (the hand-built
  trace card CT34-GROWTH pinned, now at `UnadmittedProtectionGrowth`);
* four event edits against a real certificate — a narrower interval, a guard the
  section never minted, another task, another predicate identity;
* three certificate edits against a real trace — an emptied record, a record
  moved to another event index, and a published shape that hides the growth its
  own record admits;
* and the positive control: the certificate the checker really issued renders.

A `certificate_checker` that started rendering growths it had not admitted would
break every one of these.

## 3. The sanitized fixture

`compiler/tests/fixtures/ct/CT34/growth_then_guarded_access.qcore`:

```qcore
import std.concurrency;
fn main() -> unit {
    thread scope s {
        let x = protected 4;
        let y = protected 4;
        let m = mutex guards x;
        with lock m as g {
            open g;
            write x with g;      // pre-growth: the originally declared region
            grow g guards y;
            write y with g;      // post-growth: the newly protected region
            close g;
        }
        with lock m as h { read x with h; read y with h; }
    }
}
```

It is built so the run witnesses something. The pre-growth write establishes that
the section was already working on real memory; the growth adds `y`; the
post-growth write is an access the checker admits **only because** of the growth
(before it, `y` has no protector and a guarded write to it is refused); and the
second section reads both back, so the two `read` sums are the bytes the two
writes really left behind. A rendering that skipped either write, or reordered
the growth, moves the observation.

Measured, and pinned in `expected.json`:

```
resource 1 4 / resource 2 4 / create 1 / lock 1 1 / open 1 / write 1 0 4 /
grow 1 1 2 0 4 / write 2 0 4 / close 1 / unlock 1 / lock 1 2 /
read 1 0 4 16 / read 2 0 4 24 / unlock 2 / destroy-mutex 1 / destroy 2 / destroy 1
```

The harness checks the pinned vector against `interpret_mutex_program`'s own
output **before** the native run, so the pin and the reference cannot drift
apart, and then compares the native stdout with the reference line for line.

### 3.1 The run's exact claims

**Shows.** On x86-64 Linux, one compiler invocation, one thread: the emitted C
compiles under `-std=c11 -Wall -Wextra -Werror -pedantic -O0
-fsanitize=address,undefined -fno-omit-frame-pointer`; it runs to completion
under `ASAN_OPTIONS=detect_leaks=1:halt_on_error=1` and
`UBSAN_OPTIONS=halt_on_error=1:print_stacktrace=1`; every access, including the
two to the grown region, is in bounds of a real allocation; both allocations are
reclaimed (LeakSanitizer); the growth performs no mutex operation, and the
accesses that follow it sit inside the growing section's own lock/unlock pair;
and the native and reference renderings agree line for line.

**Does not show.** That a C mutex protects the grown region — no `mtx_t` can
express an extent. Anything about contention, schedules, publication or the C
memory model. Anything about the invariant predicate. Anything about growth by a
non-creator, a shrink or a replacement, none of which the profile admits. And
nothing about macOS leak behaviour: Apple's AddressSanitizer has no
LeakSanitizer, so the harness runs there with `detect_leaks=0`, exactly as CT26
does.

## 4. CT wiring

CT coverage is **unchanged at 32/50 mapped, 18 unmapped**. What moves is
conformance, not mapping: harness-gated sanitized-native evidence goes **3 of 32
to 4 of 32** (CT03, CT17, CT26, **CT34**); proof stays 0 of 32.

### 4.1 The structural problem, and the rule that resolves it

CT34's own fixture is a *refusal*. It emits no C, so no compiler ever sees it,
and `SANITIZED_GATED_CASES` demanded a runner that compiles "the case". The
sanitized run can only be of a control — and schema 2 required every executed
stage's artifact to equal the case's own fixture.

Recording `case.qcore` as the artifact of a run that never touched it would have
been a false record. The rule was therefore **tightened rather than relaxed**:

| | before | after |
|---|---|---|
| artifact check | `artifact == <case fixture>`, a constant | `artifact == planned.artifact`, which the harness **derives**: the case fixture for every stage of every other case, and the sanitized control's own file for this case's four native stages |

The derived plan now decides the artifact, so the manifest must name the exact
file the runner executed. Naming `case.qcore`, or naming the case's *other*
control, both refuse (§5, M9/M10).

The legacy_c route's rule is unchanged and its comment still holds: a control
that has its own source is executed on *its* artifact. What changed is that such
a stage can now be recorded, with that artifact, when a harness runner really
executes it — and only then. A control that merely reaches `compile_files` for a
verdict assertion, which is what every other mutex control does, is not an
emission or execution observation and stays absent; that asymmetry is deliberate
and is recorded in §7.

### 4.2 The control expectations

`accept_growth` replaces `accept_check_only` on CT34's growth controls:

| Expectation | Asserts |
|---|---|
| `accept` | unchanged |
| `accept_growth` **(new)** | the control's own bytes parse, elaborate, lower and check; the published certificate records **at least one admitted protection growth**; `compile_files` returns a compiled module on the checked mutex route; and that module **carries emitted C** |
| `accept_growth` + `sanitized_run` | all of the above, plus: the emitted C contains the pinned substrings, the reference equals the pinned observation, and the compiled program's stdout equals the reference |
| `accept_check_only` | unchanged, and **retained with no user** |
| `reject` | unchanged |

The growth-recorded assertion is wave 13's, carried over verbatim: a control that
checked while performing no growth would attribute nothing, and mutation M7
shows it firing.

`accept_check_only` is retained rather than deleted. It existed to describe
exactly the state this card ended, so it now has no user; keeping it means a
future check-only route states its backend refusal instead of inventing a weaker
spelling. That it is currently unexercised is recorded in §7, not hidden.

Two new corpus-wide rules, both fail-closed:

* at most **one** control per case may carry a `sanitized_run` (the case's native
  stages name a single artifact);
* a mutex rejection carries a sanitized control **exactly when** it is in
  `SANITIZED_GATED_CASES` — both directions, so the gate cannot be edited alone.

### 4.3 Stage rows

Derived by `derive_stage_plan`, compared against the manifest, which is checked
rather than self-certifying.

| Stage | Status | Executor | Artifact | Obs | Host gate |
|---|---|---|---|---|---|
| `parse`, `elaborate`, `mutex_lowering`, `mutex_invariant_check` | executed | `run_mapped_case` | `CT34/case.qcore` | 1 each | — |
| `mutex_reference_execution` | **executed** | `run_mutex_controls` | `CT34/growth_then_guarded_access.qcore` | 1 | linux, macos |
| `c_emission` | **executed** | `run_mutex_controls` | same | 1 | linux, macos, windows |
| `native_execution` | **executed** | `run_mutex_controls` | same | 1 | linux, macos |
| `native_sanitized` | **executed** | `run_mutex_controls` | same | 1 | linux, macos |
| `proof` | unassessed | — | — | — | — |
| every other stage | not_applicable | — | — | — | — |

`c_emission` is gated on all three hosts and the rest on Unix, because the runner
asserts the emission on Windows and reports the sanitized run unexecuted rather
than skipping silently — the same split `run_checked_mutex_sanitized` has. The
route still has no CFG stage.

### 4.4 Both acceptance derivations

* Rust: `ct_corpus.rs::ct_acceptance_summary_is_derived_from_the_manifest` — passes.
* Python: `scripts/ct_stage_summary.py . --check` — exit 0, no DRIFT.
* `build_matrix.py . 0 --out <scratch>` (never over the committed matrix):
  50 cases, mapped 32 / unmapped 18, `native_sanitized` histogram
  `executed: 4`, and CT34's row carries the control artifact.

`acceptance.json`'s generated block was written by its own generator; its two
hand-maintained sentences were edited deliberately: the gated-sanitized count
`3 of 32 (CT03, CT17, CT26)` becomes `4 of 32 (…, CT34)`, and the counting rule
"a case's accepted control is never counted as that case's own stage, with one
recorded exception" becomes **two** recorded exceptions, naming this one.

### 4.5 Evidence-record supersession

`scripts/ct_stage_summary.py::check_provenance` requires every mapped case to
have an archived stage-evidence record agreeing with the manifest, and
`merge_evidence` refused two records claiming one case. Re-measuring a case an
earlier card mapped therefore had no expression at all.

The archived `docs/reports/wave13-ct34/ct_stage_evidence.json` is **not edited** —
rewriting an archived record would destroy the history it exists to keep. Instead
`merge_evidence` gained an explicit supersession: a later record may claim a case
an earlier one holds **only** by declaring `"supersedes": "<that record's exact
path>"`. A silent second claim stays the error it has always been; a declaration
naming the wrong record, or naming nothing (a case no earlier record claims),
both fail closed. Four new Python tests cover the positive case and all three
refusals.

## 5. Mutation matrix

Sixteen scratch mutations, each applied to a committed file, run, then reverted
with `git checkout -- .`. **All sixteen refuse.** Nothing was committed; the
baseline passes before and after.

| # | Mutation | Result | Refused by |
|---|---|---|---|
| M1 | sanitized control demoted to a plain `accept` | REFUSED | "only a control the backend renders can be compiled and run" |
| M2 | the `sanitized_run` block removed | REFUSED | "a mutex rejection's sanitized-native claim is exactly its sanitized control" |
| M3 | one line dropped from the pinned reference | REFUSED | "reference observation drifted from the pinned expectation" |
| M4 | the `grow` line removed from the pinned reference | REFUSED | same — the growth's own line is part of the observation |
| M5 | emitted C required to carry a mutex call (`mtx_trylock`) for the growth, and the `printf` pin dropped | REFUSED | "emitted C is missing `mtx_trylock`" — a growth emits no mutex call, which is the card's own claim, checked from the other side |
| M6 | a second control also claims a sanitized run | REFUSED | "at most one control carries the case's sanitized native run" |
| M7 | the control's `grow` statement deleted | REFUSED | "must check, so the case's refusal is not vacuous" — measured `[Q610 UnprotectedFootprint { event: 4, guard: 1 }, Q610 UnprotectedFootprint { event: 9, guard: 2 }]`: without the growth, both the post-growth write and the second section's read of `y` have no protector |
| M8 | the control grows onto its own declared footprint | REFUSED | same assertion, on a cascade led by `Q610` and carrying the growth's own `Q617 FootprintAlreadyProtected` (W4, this generation's shape included) |
| M9 | manifest names `case.qcore` as the sanitized artifact | REFUSED | "`native_sanitized` must name the exact file its runner executed" |
| M10 | manifest names the case's *other* control as the artifact | REFUSED | same |
| M11 | manifest claims the sanitized run is host-unconditional (`+windows`) | REFUSED | both acceptance derivations (`host_scope`) |
| M12 | manifest inflates the emission count to 2 | REFUSED | both derivations (`observation_counts`) |
| M13 | manifest reverts `native_sanitized` to `unassessed` | REFUSED | both derivations, and the per-case derived plan |
| M14 | CT34 removed from `SANITIZED_GATED_CASES` | REFUSED | "a mutex rejection's sanitized-native claim is exactly its sanitized control" |
| M15 | `acceptance.json` left at 3 gated sanitized cases | REFUSED | Rust derivation |
| M16 | the same, through the Python derivation | REFUSED | `DRIFT: acceptance.json stage_histogram is not what the manifest derives` |

Two rows are recorded as refusing somewhere other than one might predict. **M2**
and **M14** are the two halves of one biconditional, so removing either the
block or the gate entry trips the same assertion from opposite sides — which is
the point of pinning it in both directions. **M7** and **M8** refuse at the
*checker*, not at the native run: a control that does not perform an admitted
growth cannot check at all, because its post-growth write has no protector. That
is the honest reading and it is what makes the sanitized run attributable to the
growth rather than to a program that happened to compile.

The certificate-side mutations live in the core test rather than here (§2): an
emptied record, a relocated record, three edited records and a shape hiding the
growth, plus four event edits, all refusing in both renderings.

## 6. Lean

**Applicable, and done.** This card changes production semantics — the backend
now renders something it refused — so the harness-increment "not applicable"
judgment the wave-13 CT lane recorded does not carry over.

`proof/QCore/ProtectionGrowthEmission.lean` models the rendering decision:
events, an `AdmittedProtectionGrowth` record, the effective shape, two
independently written renderings (`stepStatements` producing C statements,
`stepLines` producing observation lines) and `observe`, the model's single
premise about C — that a `printf` prints its argument. **25 theorems**, no new
axiom (`propext`, and `Quot.sound` in two), registered in
`proof/AuditProtectionGrowthEmission.lean`; `check_lean_audits.py` passes **63
audits** (was 62).

| Group | Theorems |
|---|---|
| What a growth costs the `mtx_t` | `a_growth_statement_performs_no_mutex_operation`, `step_operations_are_the_events_own`, **`operations_are_exactly_the_traces_own_mutex_calls`** (the mutex operations of a rendering are exactly the ones the trace's own events call for, computed without reading the emitted C), `growth_contributes_no_mutex_operation` (deleting every growth moves none of them) |
| The two renderings agree | `step_renderings_agree`, **`emitted_program_observes_the_reference`** (on every trace, refusals included), `emitted_and_reference_renderings_agree` |
| Only an admitted growth renders | `growth_requires_a_minted_guard`, `unadmitted_growth_is_refused`, `growth_requires_a_record_at_this_index`, `growth_requires_the_published_shape`, `head_refusal_refuses_the_program`, `tail_refusal_refuses_the_program`, `a_refused_growth_refuses_the_whole_program`, `a_rendered_growth_was_admitted` |
| Independence | `a_growthless_trace_renders_independently_of_the_certificate` — a module that never grows renders the same under every certificate, so this admission cannot move one |
| The CT34 control's own trace, decided | it renders and agrees; its observation is pinned in full; it performs exactly six mutex operations; deleting the growth moves none of them; and five certificate edits each refuse it |

What the model does not do: it does not extract the trace from source, does not
model C semantics beyond the `printf` premise, and does not claim anything about
the mutex protecting memory. Rust-to-Lean correspondence here is by
transcription, as in `QCore.Concurrency.MutexInvariant`; the digest-tie technique
card F01 used for `check_concurrent` is not applied to the backend, and that stays
open (§7).

## 7. Gate

Run in this worktree at the final commit.

| Gate | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **2457 passed, 1 failed, 7 ignored** — the one failure is the recorded load-flaky `vp04_vp07_vp08_process_lifecycle_and_payload_budgets`, which passes in isolation (20/20, immediately after) and which this card does not touch |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py` | **63 audits pass** (was 62; `AuditProtectionGrowthEmission.lean` is new) |
| `providers/` `cargo test --no-fail-fast` | **94 passed, 0 failed** |
| `python3 -m unittest discover -s scripts/tests` | **128 passed, 1 failed** — see below |
| `python3 scripts/ct_stage_summary.py . --check` | exit 0, no DRIFT |
| `build_matrix.py . 0 --out <scratch>` | 50 cases, mapped 32 / unmapped 18, `native_sanitized` executed 4 |

**The one Python failure is an integrator carry item, not a defect.**
`test_build_matrix_sections.py::test_the_derived_facts_are_recomputed_rather_than_carried`
compares the committed `docs/reviews/r0_3/acceptance_matrix.json` against what
the generator now produces, and reports `lean_audits: 62 → 63`. This card was
instructed not to edit `acceptance_matrix.json`; the file is regenerated
whole-file by the integrator, exactly as the wave-13 CT lane's carry list
records. Regenerating it here (to a scratch path, never over the committed file)
shows the complete set of values that will move:

* `validation/lean_audits` 62 → 63 and one new entry in `lean_audit_files`;
* `ct_conformance` stage histogram and observation counts for
  `mutex_reference_execution` (1 → 2), `c_emission` (12 → 13),
  `native_execution` (12 → 13) and `native_sanitized` (executed 3 → 4,
  unassessed 18 → 17);
* CT34's `deepest_executed_stage` and `case_depth`
  `mutex_invariant_check` → `native_sanitized`, moving `depth_profile`;
* CT34's four native stage records, its `stage_evidence_record` pointer, its
  renamed auxiliary control entry and its `semantic_profile`;
* `ct_conformance/derived_from` gaining this card's evidence file.

Two further differences the generator reports are **pre-existing and not caused
by this card**: `dimensions/named_followups` reads 19 while the committed
matrix's own carried `named_followups` list already has 21 entries, and the
generator does not carry a `newly_decidable_not_mapped_reasons` block the
committed file holds. Neither field is touched by this card's diff.

New tests: 2 in `crates/qcore-core/tests/unit/mutex_c_emit.rs` (one of them
eleven refusal rows), 4 in `scripts/tests/test_ct_stage_summary.py`, and one new
CT34 control the existing harness tests iterate over. 25 new Lean theorems.

## 8. Recorded, not repaired

1. **`accept_check_only` is retained with no user.** It existed for exactly the
   boundary this card retired. Deleting it would remove the harness's only way to
   say "the checker admits this and the backend declines to render it", which is
   a real state a future route may need. Named **CT34-CHECKONLY-UNUSED**.
2. **A control's `compile_files` emission is not a `c_emission` observation
   unless the harness asserts on it.** CT27–CT33 and CT48 carry `accept`
   controls whose modules really do carry emitted C, and their `c_emission`
   stays `not_applicable`; CT34's carries `executed` because the harness takes
   that C, compiles it and runs it. The line is "a runner asserted on this
   stage's artifact", which is defensible and is now stated in the code, but it
   is a line rather than a law. Named **CT34-CONTROL-EMISSION**.
3. **No Rust/Lean digest tie for the backend.** `QCore.ProtectionGrowthEmission`
   is a transcription of the rendering decision, not a mechanically checked
   correspondence with `mutex_c_emit.rs`; card F01's two-sided digest technique
   is applied to `check_concurrent` and not here. Named
   **CT34-EMISSION-CORRESPONDENCE**.
4. **The growth's bookkeeping line is the model's, not C's.** `grow 1 1 2 0 4`
   is emitted by the backend so the two observations stay comparable; it
   corresponds to no C-level event. A reader who took the observation for a
   trace of machine behaviour would over-read it, which is why §1.4 and the
   fixture note both say so.
5. **CT34-GROWTH's own four boundaries are untouched** (§1.4), as are the
   profile's exclusions: no shrink, no replacement, no non-creator growth, no
   condition variables, rwlocks or try/timed locks.
6. **`proof` stays `unassessed` for CT34**, as for every CT case: no theorem is
   bound to a CT fixture. The Lean scenarios above are decided over a
   *transcription* of the fixture's trace, which is not the same thing. T00.7
   remains open.
