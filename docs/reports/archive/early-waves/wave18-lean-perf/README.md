# Proof-phase build cost — 2026-09-10 (LEAN-PERF)

The proof gate got cheaper without any theorem, domain, digest or axiom moving.
`docs/reports/lean-build-performance/README.md` is this card's template and its
rules were applied to every change here: an optimization is a proof-engineering
change, never a claim change.

Headline, measured back to back on one Linux checkout:

| gate run | before | after | change |
|---|---|---|---|
| **cold** (`rm -rf proof/.lake/build`, then `check_lean_audits.py`) | **121.4 s** | **61.0 s** | −50 % |
| — of which `lake build` | 96.7 s | 56.2 s | −42 % |
| — of which the 76 audits | ~24 s | ~4 s | −83 % |
| **incremental** (same command again, cache warm) | **36.5 s** | **4.2 s** | −88 % |
| cold CPU (user+sys) | 485.4 s | 360.5 s | −26 % |
| cold peak RSS | 3.04 GB | 2.26 GB | −26 % |
| cold build, summed over all 119 modules | 617.3 s | 440.4 s | −29 % |

A repeat of the optimized column before the last change landed read 61.6 s cold
and 5.0 s incremental, so the pair is stable at this machine's noise level.

The incremental row is what the integrator's per-merge gate actually pays in a
checkout that already has `proof/.lake` (see §5). All 76 audits pass in both
columns and `AuditAll` admits exactly `propext`, `Classical.choice`, `Quot.sound`
before and after. The raw campaign figures are
[`timings.log`](timings.log).

## 1. What was measured, and how

Base commit: `862f049` on this worktree's branch. **Deviation from the brief:**
the brief names base `ad58360` with 77 audits; the worktree was created from
`862f049`, which carries **76** `proof/Audit*.lean` files. Every number here is
against 76 audits. `ad58360` was never used, and one accidental
`git checkout ad58360 -- proof scripts` was reset before any measurement.

This machine is shared with other agents: 34 of 60 GB resident and swap
exhausted during the campaign, and repeat wall times for a single module varied
by up to 2.5× (`MutexInvariant` read 9.40 s and 3.80 s twenty minutes apart with
no source change). Two consequences, both applied:

* every before/after pair in this report was run **back to back** in one
  campaign, never against a number taken an hour earlier;
* per-module comparisons also report **CPU time** (user+sys), which is far less
  sensitive to what else is running.

Timings are from this checkout and are not portable performance limits.

## 2. Where the cost was

Cold build, per-module, from the campaign logs (`lake build`'s own per-module
report, so these are *under 16-way contention* — the wall-clock reality, not the
isolated cost):

| module | before | after |
|---|---|---|
| `QCore.TaskTraceRelation` | **89.0 s** | **55.0 s** |
| `QCore.CleanupVocabulary` | 53.0 s | 35.0 s |
| `QCore.CapacityBoundary` | 50.0 s | 25.0 s |
| `QCore.MutexInvariant` | 26.0 s | 18.0 s |
| `QCore.ConditionalGroupDepth` | 21.0 s | 19.0 s |
| `QCore.DescriptorCustody` | 16.0 s | 15.0 s |
| `QCore.OsEndpointPostState` | 12.0 s | 13.0 s |
| `QCore.LifetimeCorrespondence` | 11.0 s | 13.0 s |
| `QCore.VerifierPlugin` | 10.0 s | 9.4 s |
| `QCore.ChainedAccess` | 9.8 s | 9.2 s |
| **serial sum, all 119 modules** | **617.3 s** | **440.4 s** |

Only the top three were touched. Rows below them move by load alone — two of
them (`OsEndpointPostState`, `LifetimeCorrespondence`) move *up*, which is the
honest size of this machine's noise and the reason §1's pairing rule exists.

`TaskTraceRelation` was, and remains, the build's critical path. Across the five
cold runs of this campaign the whole `lake build` finished within 1–8 seconds of
that one module: 81.0 s inside 82.2 s, 72.0 s inside 73.2 s, 89.0 s inside
96.7 s, 57.0 s inside 57.9 s, 55.0 s inside 56.2 s. Nothing else in the build moves wall-clock time
until it comes down, which is why it was optimized first and why the other two
modules' wins show up in CPU and memory rather than in the cold wall.

Isolated per-module cost, measured with the machine otherwise idle of this
card's work, one module per `lake env lean` invocation:

| module | wall before | wall after | CPU before | CPU after | peak RSS before | peak RSS after |
|---|---|---|---|---|---|---|
| `TaskTraceRelation` | 49.5 s | **27.4 s** | 56.7 s | **28.2 s** | 2.38 GB | 2.18 GB |
| `CleanupVocabulary` | 23.8 s | **16.0 s** | 37.5 s | **16.6 s** | 2.54 GB | 2.23 GB |
| `CapacityBoundary` | 29.2 s | **11.9 s** | 60.2 s | **14.6 s** | 2.98 GB | 1.54 GB |

`CapacityBoundary` was the largest **CPU** cost in the project even though its
wall time ranked third, because its six domain proofs elaborate in parallel
inside the file. That matters on a 2- or 4-core CI runner, where CPU time is
wall time.

The audit phase was 76 serial `lake env lean` invocations. `AuditAll.lean` costs
3.8 s (it imports all of `QCore` and walks every declaration); the other 75 cost
about 0.3 s each, nearly all of it process start and `import`. So the phase was
~25 s of which ~21 s was 75 processes waiting their turn.

## 3. The four changes

### 3.1 `QCore.TaskTraceRelation` — one D3 expansion instead of four

D3 is 3,025 generated traces. Four public theorems measure it: its length, its
trace digest, its verdict digest and its admitted count. Each one separately
expanded the domain, so the `cbv` step, the kernel's check of that expansion and
the sharing/compilation of the resulting literal were paid four times.

They now share one expansion. A `private theorem domain_three_measurements`
states the conjunction of all four facts and proves it with

```lean
conv => pattern (occs := *) domainThree; all_goals cbv
decide +kernel
```

so every occurrence of `domainThree` in all four propositions is rewritten to
the same shared literal, checked once by the kernel, and the four decidable
propositions are then decided against it. Each public theorem is a projection:

```lean
theorem domain_three_size : domainThree.length = 3025 := domain_three_measurements.1
```

**Statement preservation.** The four theorem statements are byte-identical to
what they were, including the four pinned numbers. `domainThree`, `relate`,
`verdictDigest`, `admittedCount` and `traceDigest` are untouched, so the Rust
side (`crates/qcore-core/tests/task_trace_relation.rs`) folds the same digests
over the same domain and the two-sided tie is unchanged. The new declaration is
an intermediate lemma, not a weaker claim: it is strictly the conjunction of the
four facts that were already proved.

Measured, one theorem at a time on top of the module's definitions (base 0.68 s):

| D3 theorem | before | after |
|---|---|---|
| `domain_three_size` | 1.42 s | — |
| `domain_three_trace_digest` | 7.35 s | — |
| `domain_three_verdict_digest` | 27.35 s | — |
| `domain_three_admits_eighteen_perturbations` | 20.74 s | — |
| all four, fused | (53.9 s summed) | **23.8 s** |

An em dash means the theorem is no longer proved by its own tactic block — it is
a projection, and its cost is inside the fused row. In the module as a whole the
four rows above account for essentially all of its time: the same file truncated
just before them elaborates in 0.68 s.

### 3.2 `QCore.CleanupVocabulary` — kernel-only reduction

All 32 `decide` calls in the module became `decide +kernel`. Under plain `decide` the
elaborator evaluates the decidability instance *and* the kernel re-checks the
result; `+kernel` leaves the single evaluation to the kernel. Profiled, tactic
execution fell from 21.4 s to 0.44 s and nothing else grew.

**Statement preservation.** Same propositions, same decidability instances, same
`Decidable.decide` value; only which evaluator reduces it. `decide +kernel`
introduces no axiom — `native_decide` is the tactic that would add
`Lean.ofReduceBool`, and it appears nowhere in this repository before or after
this card.

### 3.3 `QCore.CapacityBoundary` — one domain enumeration instead of six

Six standing invariants are each a predicate over the same
target × premise × observation × outcome domain, and each one begins by
computing `growPostState` at the point. Six `of_domainAll (by decide)` calls
therefore enumerated the domain — and recomputed every post-state — six times.

A private `allInvariants` conjoins the six checks behind a *single*
`growPostState` match; `all_invariants_hold : domainAll allInvariants = true` is
decided once; `invariants_at` delivers it pointwise; and each invariant gets a
one-line pointwise lemma that extracts its own conjunct
(`simp only [Bool.and_eq_true] at hi; exact hi.1.1.1.1.1`). Inside each public
theorem exactly one line changed — `of_domainAll (by decide) …` became
`<name>_holds …` — and every `unfold`/`rw`/`simp` step after it is untouched.

**Statement preservation.** The six public theorems, the six invariant
predicates, `domainAll`, `of_domainAll` and the domain lists are unchanged. The
fused predicate repeats the six check bodies, which is a drift hazard; it is a
*fail-closed* one, because a check that changed without its copy here being
updated makes that invariant's pointwise lemma fail to elaborate rather than
pass silently. That is stated in the module comment.

### 3.4 `scripts/check_lean_audits.py` — audits in parallel

The 76 audits are independent read-only checks over artifacts a successful build
already produced; none of them writes anything. They now run through a
`ThreadPoolExecutor` with `--jobs` workers (default: the detected CPU count
**capped at 8**; `--jobs 1` reproduces the old one-at-a-time transcript exactly).

The cap is measured, not guessed. The phase is bounded below by the single
longest audit, so 8 workers and 16 workers finish within 0.2 s of each other,
while peak memory scales with the worker count: a typical audit peaks at ~490 MB
RSS and `AuditAll` at ~1.70 GB, so an uncapped default on a 64-core machine
would ask for far more memory to buy nothing.

Properties deliberately preserved:

* **Build first.** `lake build` still runs to completion, in the same serial
  `run_step`, before any audit starts; a failed build still prevents every audit.
* **Same command.** Each audit is still
  `lake env lean -DwarningAsError=true <Audit>.lean` run with `cwd=proof`.
* **Deterministic transcript.** Each child's output is captured to its own
  temporary file and replayed **in audit order**, not completion order, so the
  log (including `AuditAll`'s declaration count) reads as it always did.
* **Failure still fails.** A nonzero exit or per-audit timeout aborts the run
  with that audit's own captured output printed and the original exception
  raised; remaining futures are cancelled.
* **Ctrl-C still kills the trees.** A worker thread cannot see the main thread's
  `KeyboardInterrupt`, so `run_step` now registers each live child in a
  lock-guarded set and the parallel driver stops every registered process tree
  on any `BaseException`. The single-child guarantee the old code had is
  unchanged.

Backwards compatibility is explicit: `check_audits` reads the worker count with
`getattr(args, "jobs", None) or 1`, so a caller constructed before `--jobs`
existed still gets the sequential path. A test pins that.

Measured on the optimized tree, `--skip-build`, 16 detected CPUs:

| audit phase | wall |
|---|---|
| `--jobs 1` (previous behaviour) | 25.50 s |
| `--jobs 8` (the default here) | **3.74 s** |
| `--jobs 16` | 3.57 s |

Seven new runner tests (15 total, all passing): `--jobs` rejects zero and
negative; captured output is returned and not streamed; every audit runs exactly
once with the sequential command in quiet mode; results are reported in audit
order even when the first audit finishes last; a failing audit fails the run and
shows its own output and does not print the success line; the worker count is
capped at the number of audits present; and an args object without `jobs` keeps
the sequential transcript.

## 4. Axiom closure, before and after

Both pins are the `AuditAll.lean` line, which is the closure check itself — it
throws on any dependency outside `{propext, Classical.choice, Quot.sound}`:

```
before: QCore axiom audit: 29951 declarations checked; only standard logical axioms
after:  QCore axiom audit: 29990 declarations checked; only standard logical axioms
```

The **allowed set is identical** and every declaration still passes it. The
count moved by +39 because proof terms changed, and it is fully attributed by
direct per-module measurement:

| module | change | why |
|---|---|---|
| `CleanupVocabulary` | +32 | `decide +kernel` emits one auxiliary per use (32 uses); 475 → 501 measured at the 26-use midpoint |
| `TaskTraceRelation` | −2 | 346 → 344: four tactic proofs became projections, one private lemma added |
| `CapacityBoundary` | +9 | one fused predicate, two shared lemmas, six pointwise lemmas |

`grep -r 29951` over the whole repository (excluding `.git`, `.lake`, `target`)
returns nothing: the count is reported evidence, not a pinned expectation, and
no test, workflow or acceptance record reads it. No `proof/Audit*.lean` file was
edited, so the audited theorem list is unchanged name for name.

## 5. Cache reuse — measured, and the trust argument

* `proof/.lake/` is in `.gitignore`, so it persists in a checkout across gate
  runs, and `check_lean_audits.py` runs `lake build` (incremental) by default.
  **The per-merge gate in the primary checkout already gets incremental
  behaviour today**: measured `Passed Lean build in 0.2s` on the second run.
  Nothing was changed to enable this and nothing needed to be.
* Each delegated worktree has its own `proof/.lake`, so a fresh worktree pays a
  full cold build — 56.2 s now, 96.7 s before.
* CI checks out fresh (cold) and audits with `--skip-build` after `lean-action`'s
  own build, so CI pays the cold build plus the now-parallel audit phase.

**No warm-cache default was introduced, and the trust argument for the existing
one is:** Lake keys each artifact on a trace over the module's content, its
imports, the toolchain and the options, so content that changes is rebuilt and a
stale artifact cannot be silently accepted. That argument covers *staleness*, not
a *deliberately poisoned* `.lake` — an attacker with write access to the build
directory could plant an `.olean`. That residual is closed here the way the brief
asks it to be stated: **CI stays cold**, from a fresh checkout on a fresh runner,
so no local cache state can reach a merged claim. Sharing a build cache between
worktrees would need content addressing before it could be trusted, and is left
as the named follow-on `LEAN-PERF-CACHE` rather than done unsoundly.

## 6. Refused as irreducible, with the measurement

* **`TaskTraceRelation`'s remaining ~27 s is the cost of the D3 claim.** After
  fusion the module profiles as: `cbv` 3.5 s, kernel check of the shared
  expansion 11.0 s, `share common exprs` + `process pre-definitions` 6.3 s, the
  fused `decide +kernel` 6.0 s. The domain is 3,025 traces of 11 events and the
  kernel must both build the expansion and verify it. Going further would mean
  rewriting `spread`, `swapAt` or `get`, which are deliberate mirrors of the Rust
  helpers and part of the transcription story — a claim change, refused.
* **Dropping `cbv` is much worse, re-measured rather than assumed.** Plain
  `decide +kernel` with no pre-expansion: `domain_three_verdict_digest`
  **117.4 s**, `domain_three_admits_eighteen_perturbations` **106.2 s**, against
  27.4 s and 20.7 s for the `cbv` route. The prior card's choice is confirmed by
  measurement.
* **`decide +kernel` is not a blanket win.** It cut `CleanupVocabulary` from
  25.8 s to 17.4 s, and it made `CapacityBoundary` *worse*: 27.9 s → 39.7 s with
  every `decide` converted, and still 38.1 s with only the six domain proofs
  converted. Both were reverted there and the module was fixed by sharing
  instead. Any future application of `+kernel` must be measured per module.
* **The parallel audit phase is now bounded by `AuditAll.lean` alone** (2.8 s to
  4.7 s across the campaign runs). It imports all of `QCore` and collects
  the axiom dependencies of every one of ~30,000 declarations. Reducing it means
  checking fewer declarations, which is exactly what must not happen.
* **`MutexInvariant` needs no work.** It reads 22 s in the contended build and
  3.8 s isolated; its 77 `decide`s are individually cheap. Recorded so a future
  reader does not chase a number that is machine load.

## 7. Named follow-ons

* **`LEAN-PERF-CV-QUADRATIC`** — `CleanupVocabulary`'s two 40 × 40 theorems
  (`spelling_is_injective_on_the_sample_domain`,
  `one_declaration_binds_one_profile_on_the_sample_domain`) recompute `spell` for
  each element of each pair: about 6,400 evaluations across the two, and the
  largest remaining kernel item in that module at 5.5 s. Sharing a precomputed
  `sampleShapes.map spell` would require re-expressing both public statements
  over that list, so it was not attempted here. The same module's other whole-
  domain theorems could share one `sampleShapes` traversal by §3.1's pattern for
  a smaller win.
* **`LEAN-PERF-CAPACITY-DRIFT`** — `allInvariants` in `CapacityBoundary` repeats
  the six invariant check bodies. The repetition is guarded (§3.3) but not
  eliminated; deriving the fused predicate from the six rather than restating
  them would need each invariant's `growPostState` match to be factored out of
  its definition, which changes six private definitions the public theorems name.
* **`LEAN-PERF-CACHE`** — no shared, content-addressed Lean build cache exists
  across delegated worktrees; each new worktree pays the full 56.2 s cold build.
  Any sharing mechanism needs the content-addressing argument of §5 first.
* **`LEAN-PERF-AUDIT-IMPORTS`** — the 75 non-`AuditAll` audits cost roughly
  0.3 s each, essentially all process start and `import`. A single Lean process
  that ran every `#print axioms` would remove that, but it would also merge 76
  independently scoped checks into one environment, which changes what each audit
  is scoped to. Not attempted; recorded with its objection.

## 8. Validation

Run from the repository root on this worktree.

```sh
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo run -p qcore-compiler --example reevaluation_probe      # exit 0
cargo run -p qcore-compiler --example reevaluation_followup   # exit 0
python3 scripts/check_lean_audits.py
python3 -m unittest discover -s scripts/tests
```

Results are recorded in §9 of this file. No Rust production or test file was
changed by this card — the whole diff is three `proof/QCore/*.lean` modules, the
audit runner and its tests — so the workspace gate is untouched-green, the
Cycle 19 artifacts are unchanged by construction, the contract corpus count is
unchanged, CT coverage is unchanged and nothing is promoted.

## 9. Gate results

All run on this worktree at the committed tree.

| gate | result |
|---|---|
| `cargo test --workspace` | **2,882 passed, 0 failed, 8 ignored** across 174 test binaries |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py` | **76/76 audits passed**; `AuditAll` checked 29,990 declarations, allowing only `propext`, `Classical.choice`, `Quot.sound` |
| `python3 -m unittest discover -s scripts/tests` | **193 passed** (15 of them the audit-runner tests, 7 new) |

One gate finding, not caused by this card: the first `cargo test --workspace`
attempt failed `libevent_submodule_matches_the_audited_revision` because this
freshly created worktree had never run `git submodule update --init
--recursive`. The submodule was initialised at the pinned
`d82464a277d0f42703702c4dfd9af6af38595a83` and the full run then passed. The
standing gate already names that command as a once-per-checkout step; it is
recorded here because a delegated worktree starts without it.

## 10. Draft increment paragraph

> Proof-phase cost (LEAN-PERF): the Lean gate is half its cold cost and an
> eighth of its incremental cost with nothing proved changed — cold
> 121.4 s → 61.0 s, incremental 36.5 s → 4.2 s, cold CPU 485 s → 361 s, peak
> RSS 3.04 GB → 2.26 GB, all measured back to back on one loaded shared
> machine. Three redundancies were removed, each a proof-engineering change
> with the theorem statements byte-identical: `TaskTraceRelation`'s four D3
> measurements now share one `cbv` expansion of the 3,025-trace domain instead
> of paying for one each (49.5 s → 27.4 s isolated) and are projections out of
> one private conjunction; `CleanupVocabulary`'s 32 `decide` calls became
> `decide +kernel`, which drops the elaborator's duplicate evaluation
> (tactic time 21.4 s → 0.44 s); and `CapacityBoundary`'s six standing
> invariants — the project's largest CPU item at 60 s — now decide one fused
> predicate behind a single `growPostState` rather than enumerating the same
> domain six times (60.2 s → 14.6 s CPU, peak RSS halved). The 76 audits also
> run concurrently (`--jobs`, default detected CPUs capped at 8) with the
> transcript replayed in audit order, the build still serialised ahead of
> them, and Ctrl-C still stopping every process tree: 25.50 s → 3.74 s.
> `AuditAll` admits exactly `propext`, `Classical.choice` and `Quot.sound`
> before and after; its declaration count moves 29,951 → 29,990, attributed
> per module and pinned nowhere. Three measured refusals are recorded rather
> than pursued: dropping `cbv` on D3 is 4× worse (117 s), `decide +kernel` is
> *not* a blanket win (it made `CapacityBoundary` worse and was reverted
> there), and the parallel audit phase is now bounded by `AuditAll` alone,
> which is the cost of checking every declaration. No Rust file, no
> `proof/Audit*.lean`, no theorem, domain, digest or pinned number changed;
> nothing is promoted. Evidence:
> `docs/reports/wave18-lean-perf/README.md`.
