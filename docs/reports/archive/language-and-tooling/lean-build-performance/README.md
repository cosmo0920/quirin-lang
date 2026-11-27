# Lean build performance and bounded audit runner — 2026-09-08

The stalled job was `QCore.TaskTraceRelation`, not the import-only `QCore.lean`.
Its D3 domain contains 3,025 generated trace permutations. Plain `decide`
repeatedly reduces the nested schedule/transposition expressions while
classifying a trace, then repeats evaluation during kernel checking.

## Repair

Finite-domain proofs use `decide +kernel`. For D3's verdict digest and admitted
count, `conv => lhs; arg 2; cbv` first expands the finite input list once, with
an explicit finite `cbv.maxSteps` budget of 4,000,000. The kernel then checks
the result over concrete traces. No definitions, domains, theorem statements,
expected digests or counts changed. The allowed axiom boundary is unchanged;
no native evaluator axiom, `sorry`, or unverified computed result is used.

A blanket `decide_cbv` experiment was rejected: its default 100,000-step
budget failed four proofs. Merely increasing that budget still exceeded a
120-second trial. The final implementation uses `cbv` only to expand the
input to the two expensive proofs, not to evaluate the entire classification.

The audit runner now builds by default, selects the pinned toolchain by
starting in `proof`, streams child output, prints progress every 15 seconds,
and limits the build to 600 seconds and each audit to 120 seconds. Timeout or
Ctrl-C stops the active process tree. A build failure prevents audits against
stale artifacts. CI retains its existing pinned build step, runs the runner's
regression tests, uses explicit `--skip-build` after that build, and has a
15-minute job limit.

## Local validation

- Original module: did not finish within a 120-second diagnostic limit;
  this is a lower bound, not an exact baseline duration.
- Corrected build with the affected module uncached: **47.5 seconds**;
  other module artifacts were reused. This is not a clean whole-project build.
- Following incremental `lake build`: **0.13 seconds**.
- The exact reported `lake -d proof lean proof/QCore.lean` command: exit 0,
  **0.30 seconds** with cached artifacts. Elan still attempted its default
  `stable` release lookup; starting inside `proof` avoids that lookup.
- **31/31 audits passed**; AuditAll checked **14,743 declarations**, allowing
  only `propext`, `Classical.choice`, and `Quot.sound`. See
  [the complete build/audit log](lean-validation.log).
- **8 Python runner regression tests passed**, including actual silent-process
  heartbeats, nonzero exits, and termination of a descendant that ignores SIGTERM.
- Standing gates: submodule initialization, **1,379 workspace tests passed**
  (one ignored), formatting, Clippy, and both reevaluation examples passed.
  The workspace test run required execution outside the sandbox because
  LeakSanitizer cannot inspect processes under its ptrace restrictions.

Timings are from this Linux checkout and are not portable performance limits.
No checker verdict, CT coverage or source-language conformance claim changes.

## Commands

From the repository root:

```sh
python3 scripts/check_lean_audits.py
python3 -m unittest scripts.tests.test_check_lean_audits
```

For build-only work: `(cd proof && lake build)`. Keep `proof/.lake` to reuse
checked artifacts. `--verbose` shows Lake build commands; `--build-timeout`,
`--audit-timeout` and `--heartbeat` set finite positive seconds. Only use
`--skip-build` after a successful build of the current sources.
