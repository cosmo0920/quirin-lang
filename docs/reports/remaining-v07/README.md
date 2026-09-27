# V07 — Unchecked clauses and scoped binders remain fail-closed

> Historical checkpoint: source filenames, runnable references or discovery scripts in this archive were updated by the [extension migration](../extension-physical-migration/README.md). Recorded results, hashes and gate counts describe the original revision, not a new run of the maintained files. Use [current status](../../STATUS.md) for present measurements.


**Date:** 2026-09-07. **Worktree HEAD:** `536dc01` (documentation-only commits
on top of implementation checkpoint `e3e7c2c`). **Card type:** verify_or_repair.

This is a report-only card. No tracked file was modified, no commit was made and
no verdict was promoted. Everything under this directory plus the session
scratchpad is the only thing written.

## Question

`docs/reports/w6-frontend-gates/README.md` recorded two bounded repairs at a
checkpoint where source task scopes were still rejected outright:

1. a nonempty source `contract { ... }` on a function rejects centrally with
   `Q021`, and a supplied semantic bundle cannot bypass it;
2. `with lock m as g` / `thread scope s` bind `g`/`s` only inside their body,
   header inputs resolve in the enclosing scope, and escapes reject.

Since then the native source-task increments landed
(`docs/reports/w6-task-recovery/README.md`,
`docs/reports/w6-owned-source-tasks/README.md`,
`docs/reports/w6-worker-allocation/README.md`,
`docs/reports/w6-handler-rethrow/README.md`,
`docs/reports/w6-completed-child-custody/README.md`,
`docs/reports/w6-reused-address/README.md`), which added a *second* compilation
route that runs **before** `check_whole_program`. The question this card answers
is whether the two gates survived that integration, and whether any live CLI
route — multi-file input, `emit`, `--plugins`, module modes, `verify scope`, or
the task route — can get an unchecked clause or an unresolved binder accepted.

The static reason the question is nontrivial:
`crates/qcore-verification/src/driver.rs:292` dispatches to
`compile_task_source` when `qcore_frontend::source_tasks::has_source_tasks`
holds, and `compile_task_source` (line 1100) **never calls
`check_whole_program`**, which is where the `Q021` gate lives
(`crates/qcore-frontend/src/whole_program.rs:149-157`). So the `Q021` gate is
structurally *not* on the task route; a separate gate has to hold there.

## Reproducing

```sh
cargo build -p qcorec
docs/reports/remaining-v07/probes/run_probes.sh \
    target/debug/qcorec [ABSOLUTE_PLUGIN_CONFIG.json]
```

The optional second argument is a `qcorec` provider-selection file. The archived
run used a real packaged provider:

```sh
python3 providers/package.py build --output "$SCRATCH/qcore-packages"
target/debug/qcorec plugins verify "$SCRATCH/qcore-packages/qcore.libevent"
target/debug/qcorec plugins enable "$SCRATCH/qcore-packages/qcore.libevent" \
    --config "$SCRATCH/plugins_libevent.json"
```

`verify` exited 0 and `plugins list --config` reported
`qcore.libevent 0.1.0 resource_contracts,event_traces,concurrency,observation_hooks`.
The config file is not archived here because it embeds absolute scratchpad paths;
`plugins enable` regenerates it. Complete captured output:
[`probes/probe_log.txt`](probes/probe_log.txt) (41 rows: 40 property probes plus
the d02 boundary-attribution row).

## Probe results

`Result` is the first diagnostic line, path-shortened. All classifications are
against the fail-closed invariant (AGENTS.md hard invariant 1) and the verdict
contract (invariant 2).

### A. Source-contract gate (`Q021`)

| # | Source | Expected | Actual | Class |
|---|---|---|---|---|
| a01 | `a01_requires.qcore` — `requires` clause, single file | reject `Q021` | `1:29 Q021 … refusing unchecked contract`, exit 1 | **fail-closed** |
| a02 | `a02_ensures.qcore` — `ensures normal` clause | reject `Q021` | `1:29 Q021`, exit 1 | **fail-closed** |
| a03 | `a03_quant.qcore` — `quant normal wall_clock <= 5` | reject `Q021` | `1:29 Q021`, exit 1 | **fail-closed** |
| a04 | `a04_first_main.qcore` + `a04_second_bound.qcore` — clause in the **second** file | reject `Q021`, attributed to file 2 | `a04_second_bound.qcore:1:29 Q021`, exit 1 | **fail-closed**, correct file |
| a05 | `a05_referenced_uncalled.qcore` — contract fn called only from a never-called function | reject `Q021` | `1:31 Q021`, exit 1 | **fail-closed** |
| a06 | `a06_never_referenced.qcore` — contract fn never mentioned anywhere | reject `Q021` | `1:31 Q021`, exit 1 | **fail-closed** |
| a07 | `a07_empty_contract_control.qcore` — `contract {}` control | accept | `checked 1 function(s); entry \`main\``, exit 0 | **control, as documented** |
| a08 | `emit -o /dev/null a01_requires.qcore` | reject `Q021` | `1:29 Q021`, exit 1 | **fail-closed** |
| a09 | `check --plugins plugins_empty.json a01_requires.qcore` | reject `Q021` | `1:29 Q021`, exit 1 | **no bypass** |
| a09b | same, task-route source `c05` | reject | `2:1 Q020`, exit 1 | **no bypass** |
| a10 | `check --plugins <enabled qcore.libevent> a01_requires.qcore` | reject `Q021` | `1:29 Q021`, exit 1 | **no bypass** |
| a10b | same, uncalled contract fn `a06` | reject `Q021` | `1:31 Q021`, exit 1 | **no bypass** |
| a10c | same, task-route source `c05` | reject | `2:1 Q020`, exit 1 | **no bypass** |
| a10d | `emit --plugins <enabled> a01_requires.qcore` | reject `Q021` | `1:29 Q021`, exit 1 | **no bypass** |
| a10e | `check --plugins <enabled> a14_verify_scope.qcore` | reject | `1:1 Q701` (scope unverified), exit 1 | **fail-closed, gate masked** |
| a11 | `a11_library_mode.qcore` — `module library;` | reject `Q021` | `2:31 Q021`, exit 1 | **fail-closed** |
| a12 | `a12_contract_mode.qcore` — `module contract;` | reject `Q021` | `2:31 Q021`, exit 1 | **fail-closed** |
| a13 | `a13_extern_contract_control.qcore` — `extern fn … contract {…};` | accept (documented extern-contract behavior) | exit 0 | **control, as documented** |
| a14 | `a14_verify_scope.qcore` — `verify scope` + contract clause, default registry | reject | `1:1 Q701`, exit 1 | **fail-closed, gate masked** |
| a15 | `a15_task_second_file_main.qcore` + `…_bound.qcore` — task program, clause in the **second** file | reject, attributed to file 2 | rejects `Q020`, but attributed to **`…_main.qcore:11:1`** | **fail-closed; attribution defect (D01)** |

Notes on a14/a10e: with every shipped provider the `verify scope` route cannot
reach acceptance at all — the core-derived obligation inventory is never
discharged, so `Q701` fires before elaboration. `Q021` is therefore not
*observable* on that route, but nothing is admitted either. Retest this row when
a provider can discharge a scope.

Note on a13: `extern fn … contract { … };` is a declared foreign contract
consumed by the contract machinery, not an unenforced source-body clause. The
`Q021` gate deliberately covers `DeclarationKind::Function` declarations only;
`docs/reports/w6-frontend-gates/README.md` states "extern contract-module
interpreters retain their existing behavior."

### B. Guard / thread-scope binders

| # | Source | Expected | Actual | Class |
|---|---|---|---|---|
| b01 | `with lock m as g { }`, `m` acquired in outer scope | `m` resolves; reject only at lowering | `4:3 Q500` structured control flow …, no `Q013`, exit 1 | **fail-closed, header resolves outward** |
| b02 | `with lock m as m { }`, outer `m` acquired | outer `m` resolves, binder does not capture the header | `4:3 Q500`, no `Q013`, exit 1 | **fail-closed** |
| b03 | `with lock missing as g { }` | reject `Q013` on `missing` | ``3:3 Q013: unknown name `missing` ``, exit 1 | **fail-closed** |
| b04 | binder `g` used after its body | reject `Q013` on `g` | ``5:3 Q013: unknown name `g` ``, exit 1 | **fail-closed** |
| b05 | second `with lock` body uses the first body's binder | reject `Q013` | ``5:22 Q013: unknown name `g` ``, exit 1 | **no sibling leak** |
| b06 | `with lock g as g { }`, `g` undefined outside | binder must **not** mask the unresolved header | ``3:3 Q013: unknown name `g` ``, exit 1 | **no masking** |
| b07 | `thread scope s { } drop s;` | reject `Q013` on `s` | ``3:3 Q013: unknown name `s` ``, exit 1 | **fail-closed** |
| b08 | `with lock m as g { let g = …; }` | reject `Q012` duplicate | ``4:22 Q012: duplicate local binding `g` ``, exit 1 | **fail-closed** |
| b09 | binder used inside its own body | resolves; reject only at lowering | `4:3 Q500`, no `Q013`, exit 1 | **binder is in scope in its body** |
| b10 | binder from file 1's body used in file 2 | reject `Q013` | ``b10_cross_file_second.qcore:2:3 Q013: unknown name `leaked` ``, exit 1 | **no cross-file leak** |

Every binder row that must reject does so during name resolution, i.e. *before*
lowering, so the rejection does not depend on the lowering boundary staying
closed.

### C. Interaction with the now-implemented task scopes

| # | Source | Expected | Actual | Class |
|---|---|---|---|---|
| c01 | `thread scope s { }` with no `concurrency checked;` | explicit reject | `1:1 Q600: task execution requires exactly one \`concurrency checked;\` marker`, exit 1 | **fail-closed** |
| c02 | worker that itself opens a scope and spawns (nested worker) | explicit reject | `3:1 Q600: spawned workers must be first-order leaf functions in this task profile`, exit 1 | **fail-closed** |
| c03 | `spawn s …` from inside a nested inner scope | explicit reject | `7:13 Q500: spawn requires the innermost active thread scope`, exit 1 | **fail-closed** |
| c04 | scope exit with an unjoined child | explicit reject | `7:9 Q600: ScopeBusy(4)`, exit 1 | **fail-closed** |
| c05 | contract clause on a spawned worker in a valid task program | reject | `2:1 Q020: source contracts require an implemented consumer for task execution`, exit 1 | **fail-closed (different code, see below)** |
| c06 | contract clause on an **uncalled** function in a task program | reject | `3:1 Q020`, exit 1 | **fail-closed** |
| c07 | **empty** `contract { }` on an uncalled function in a task program | reject | `3:1 Q020`, exit 1 | **fail-closed, stricter than a07** |
| c08 | nested `thread scope` control, all children joined | accept | `checked 2 function(s); entry \`main\`; scoped copy tasks (Linux C11)`, exit 0 | **admitted control** |
| c09 | `with lock v as g { thread scope s { … } }` — resolved guard binder wrapping a task scope | explicit reject | `5:5 Q500: statement is outside the copy-scalar task execution profile`, exit 1 | **resolving a binder does not enable lowering** |

c09 is the direct answer to the card's third question: a guard binder that
resolves cleanly (no `Q013`) still does not smuggle an unsupported concurrency
construct past lowering.

c08 is an accepted control, not a gap. `has_source_tasks`
(`crates/qcore-frontend/src/source_tasks.rs:11-12`) documents nested scopes as
in-profile, and the emitted C runs clean:

```
qcorec emit -o c08.c c08_nested_scope_control.qr
cc -std=c11 -fsanitize=address,undefined -o c08 c08.c -lpthread && ./c08
QCORE_TASK_SPAWN 1 0 / QCORE_TASK_RESULT 1 I32 42 / QCORE_TASK_JOIN 1 /
QCORE_TASK_RESULT 0 I32 42   (exit 0)
```

The same run was repeated for a variant that joins an outer-scope child from
inside a nested inner scope; it also exits 0 under ASan/UBSan. These are two
fixed observations, not a nested-scope semantics claim.

## The task route has its own gate, with a different code

The `Q021` gate is not reachable on the task route
(`compile_task_source` bypasses `check_whole_program` by construction). What
holds there instead is `crates/qcore-frontend/src/source_tasks.rs:200-204`,
inside `signature`, which `lower` applies to **every** function declaration
(line 480) — not only to reachable or spawned ones. That is why c06 (uncalled)
and a15 (second file) still reject.

Consequences worth recording, none of which is a bypass:

- The diagnostic is `Q020` / `SemanticPass::OrdinaryTypes` with the message
  "source contracts require an implemented consumer for task execution", not
  `Q021` "refusing unchecked contract". Two different codes now guard the same
  unsupported construct depending on which route the file takes. `Q021` is
  described in AGENTS.md as reserved for this boundary.
- The task gate rejects `contract { }` too (c07), whereas the sequential gate
  accepts it (a07). The two routes disagree on the empty-contract control.
- The span is the whole declaration, not the clause, so the caret lands on
  column 1 of the `fn` line rather than on the offending clause.

Under invariant 1 all three are conservative directions. They are recorded as a
consistency gap for whoever implements a source contract consumer, not as a
finding against this card's property.

## D01 — genuine defect: multi-file diagnostics misattributed at a file boundary

Probe a15 rejects correctly but names the **wrong input file**. The reduced
reproducer needs no task syntax, no Linux and no providers:

```sh
$ cat d01_boundary_first.qr
fn main() -> i32 { return 0; }
$ cat d01_boundary_second.qr
fn main() -> i32 { return 1; }
$ qcorec check d01_boundary_first.qr d01_boundary_second.qr
d01_boundary_first.qcore:2:1: Q010: duplicate declaration `main`
```

Expected `d01_boundary_second.qcore:1:1`. `d01_boundary_first.qcore` has one
line, so line 2 does not exist in the file the diagnostic names.

Root cause, `crates/qcore-verification/src/driver.rs:1283-1287`:

```rust
fn locate(offset: usize, ranges: &[FileRange]) -> SourceLocation {
    let range = ranges
        .iter()
        .find(|range| range.start <= offset && offset <= range.end)
```

`FileRange::end` is the *exclusive* end of a file in the combined buffer, but
the predicate accepts `offset == end`. `combine_files` (line 404) appends the
next file at that exact offset when the previous file already ends with a
newline, so the first byte of file *n+1* satisfies file *n*'s predicate and
`find` returns the earlier file. Any diagnostic whose span starts at byte 0 of a
non-first input file is attributed to the previous file, at a line one past its
end.

Confirmed occurrences in this run:

| Reproducer | Reported | Correct |
|---|---|---|
| `d01_boundary_first` + `d01_boundary_second` | `d01_boundary_first.qcore:2:1 Q010` | `d01_boundary_second.qcore:1:1` |
| `a15_task_second_file_main` + `…_bound` | `…_main.qcore:11:1 Q020` | `…_bound.qcore:1:1` |
| `a15_task_second_file_main` + `d02_boundary_record` (probe d02) | `…_main.qcore:11:1 Q000: expected a QCore declaration` | `d02_boundary_record.qcore:1:1` |

`a04` is unaffected only because `Q021` reports the *clause* span, which is
strictly inside file 2. Adding one padding line before the offending declaration
in file 2 also restores correct attribution, which pins the cause to the
boundary offset rather than to the route.

**Severity.** This is a diagnostic source-attribution defect, not a fail-closed
failure and not a `Safe` admission: every affected input is still rejected, with
the correct code, message and exit status. It nevertheless violates the artifact
half of the verdict contract (invariant 2) — a published result names the wrong
artifact — and it will mislead any consumer that maps diagnostics back to
sources. It is a one-character fix in `locate` (`offset < range.end`, with the
last range keeping the inclusive bound so an end-of-input span still resolves),
but this card may not modify tracked files, so **no repair was attempted and the
card stops here on the repair track.**

No existing test covers boundary attribution:
`compiler/tests/unit/driver.rs:276` (`refuses_unchecked_source_contracts_in_every_input_file`)
asserts `error.location.path == "bound.qcore"` but its `Q021` span is a clause
span, so it passes through the defect.

## Existing tests cited per row

| Rows | Test |
|---|---|
| a01–a03, a07 (direct-bypass form) | `crates/qcore-frontend/tests/unit/whole_program.rs:338` `source_contract_clauses_cannot_be_bypassed_by_supplied_semantics` |
| a04, a07 | `compiler/tests/unit/driver.rs:276` `refuses_unchecked_source_contracts_in_every_input_file` |
| a01–a03 (contract retention) | `crates/qcore-frontend/tests/unit/frontend.rs:40` `retains_function_contracts_as_structured_source`; `:83` `rejects_malformed_function_contract_clauses` |
| a14, a10e | `compiler/tests/cycle20_verifier_plugins.rs` (`verify scope` admission), `crates/qcorec/tests/plugin_packages.rs:113,223` |
| a09–a10d | `crates/qcorec/tests/plugin_packages.rs` (verify / enable / disable / config selection) |
| b01, b02, b09 | `crates/qcore-frontend/tests/unit/frontend.rs:278` `concurrency_header_binders_have_body_local_symbol_identity` |
| b03–b08 | `crates/qcore-frontend/tests/unit/frontend.rs:308` `concurrency_binders_do_not_hide_unresolved_inputs_or_escape_their_body` |
| c01 | `compiler/tests/unit/driver.rs:295` `resolved_scope_requires_checked_profile_and_guard_still_requires_lowering` |
| c02–c08 | `compiler/tests/source_tasks.rs` (11 tests), `compiler/tests/owned_source_tasks.rs`, `compiler/tests/ct_corpus.rs` |
| b10, a15, d01 | **none** — no test pins multi-file diagnostic attribution at a file boundary |

Test runs executed for this card (all in this worktree):

```
cargo test -p qcore-frontend --lib whole_program   11 passed, 0 failed
cargo test -p qcore-frontend --lib frontend::tests 16 passed, 0 failed
cargo test -p qcore-compiler --lib driver::        16 passed, 0 failed
cargo test -p qcore-compiler --test ct_corpus       4 passed, 0 failed
cargo test -p qcore-compiler --test source_tasks   11 passed, 0 failed
```

## Conclusion

**The card's property is fully covered and survived task-scope integration.**

- 41 live CLI probes. 38 reject, three accept, and all three acceptances are
  intended controls (`contract {}` on the sequential route, `extern fn` declared
  contract, a fully joined nested task scope that also runs clean under
  ASan/UBSan).
- **No bypass was found.** Not through `emit`, not through `--plugins` with an
  empty selection, not through `--plugins` with a real verified and enabled
  `qcore.libevent` package, not through `module library;` / `module contract;`,
  not through `verify scope`, and not through the task route that structurally
  skips the `Q021` gate.
- Binder scoping is intact on all ten probes, including two the w6 report did
  not cover: sibling-scope leakage and cross-file leakage.
- Resolving a binder does not enable unsupported lowering (c09).

One genuine defect was found and is **not** a failure of this card's property:
**D01**, multi-file diagnostic misattribution at an exact file boundary
(`crates/qcore-verification/src/driver.rs:1283`). Reproducer archived as
`probes/d01_boundary_{first,second}.qcore`. Per the card's constraints no repair
was written.

Two consistency gaps are recorded for the future source-contract consumer, both
in the conservative direction: the task route uses `Q020` where the sequential
route uses `Q021`, and it rejects `contract { }` where the sequential route
accepts it.

No raw Cycle 19 unit is promoted, no CT mapping changed, and no gate was
weakened. `git status` shows only untracked report directories.
