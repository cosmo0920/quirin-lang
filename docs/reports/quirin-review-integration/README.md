# Quirin review priority integration

Implementation base: `a8f2048d` (2026-09-20). The supplied review examined
`418af31dfa4567085f68882d7fab7d2e69a042c7`; [input identities](review-inputs.json)
record the review, source excerpts and pasted summary. Those documents supply
findings and priorities, not instructions overriding the user's request or
repository rules. Three isolated implementation agents use GPT-5.6 Sol with
medium reasoning, as requested.

## Selected work and acceptance

1. `review-audit-runner`: serialize cancellation with child spawning and
   registration, collect workers and child processes, and observe failures in
   completion order while retaining deterministic log replay. Controlled
   subprocess regressions must exercise the race and delayed-failure boundary.
2. `review-prepared-tu`: prepare immutable translation-unit inputs once per
   audit, retaining exact independent preprocessing forms and checked source
   coordinates. Measure invocation counts and stage costs; preserve verdicts,
   diagnostics, scope and input identity. This is not a persistent result cache.
3. `review-certificate-scope`: bind Safe certificates to the intended target
   denominator and reconstruct it independently at recheck. Reduced, duplicate,
   mismatched or incompletely analyzed scopes must fail closed. Pair the Rust
   rule with an audited Lean model and explicit correspondence premises.

The three cards are integrated on `codex/quirin-priority-gaps-20260920`.
Independent review, corpus parity, preparation measurement, reconciliation and
the combined standing gate are complete. `main` was not advanced.

## Verified baseline

The release binary built from `a8f2048d` and replayed at report-only successor
`8776f9b9` reproduces all five W42 artifacts byte for byte. The
[run record](baseline-run.json) includes command, provider-package environment,
binary digest and exit **0**; [byte identities](baseline-byte-identity.json)
cover diagnostics, contracts, source effects, ChunkIO contracts and Monkey
source evidence. Private outputs are under
`private/quirin-review/baseline-provider/` in the integration checkout.

The initial `private/quirin-review/baseline/` run also exited **0**, but omitted
`QCORE_CHUNKIO_PACKAGE`, so it emitted four byte-identical artifacts and no
source-effects artifact. That run is retained as an incomplete comparison;
the separately recorded provider-configured replay supplies the five-artifact
baseline. Neither run measures an end-to-end speedup.

## Independent preparation measurement

The [reproducible harness](measure_preparation.py) compares the base audit CLI
against PreparedTU revision `9e62c19b`, using the same pinned fixture paths,
contracts, compiler wrapper and output configuration. All JSON outputs are
byte-identical for each of the three fixtures. These are measured process
actions, not counts inferred from source or asserted by the new metrics.

| Fixture verdict | Before preparation actions | After preparation actions |
|---|---:|---:|
| Safe | 5 | 3 |
| provisional Unsafe | 8 | 3 |
| Unknown | 6 | 3 |

Every after row executes exactly one plain preprocessing, one syntax check and
one marked preprocessing. Compiler metadata probes are counted separately:
9/12/10 before and 7 for each after fixture. [Complete measurements](preparation-measurement.json)
record exact commands, binary digests, stage timings, output hashes and GNU-time
maximum RSS. RSS ranges from 83,028 to 84,656 KiB across these instrumented
debug runs; it is a per-run maximum, not aggregate concurrent memory. Python
wrapper overhead, host load and small fixtures preclude a total speedup or
memory-reduction claim. Raw logs and binary snapshots are in
`private/quirin-review/preparation-comparison/`.

The [combined-code measurement](combined-preparation-measurement.json) repeats
all three fixtures at `ed913c29` (code unchanged from `02b037df`) and confirms
exactly the same 5/8/6→3 actions, metadata counts and byte-identical JSON
outputs after certificate-scope integration. The Safe path introduces no extra
Clang process. Combined-run RSS spans 83,596–84,568 KiB; no speed or memory
reduction is claimed. Private evidence is in
`private/quirin-review/preparation-combined/`.

## Combined corpus parity

The [release replay](combined-run.json) exits 0 and reproduces all five frozen
W42 artifacts byte for byte, also matching the independent pre-change baseline.
[SHA-256 identities](combined-byte-identity.json) and the standard
[five-artifact semantic diff](combined-corpus-diff.json) preserve the comparison.
Release build, replay, diff, audit CLI build and preparation measurement all exit
0; commands and exits are in
`private/quirin-review/combined-measurements/facts.json`. Corpus outputs and
replay logs are in `private/quirin-review/combined-after/`. No Safe/Unsafe/Unknown,
contract, diagnostic, coverage or conformance count changes.

## Remaining review scope

The current base already implements bounded scalar helper calls. General owned
and borrowed helper parameters/results, error/resource summaries, buffer–socket
and FFI integration, typed C expression/alias expansion, runtime bounds, loop
summaries and persistent incremental caches remain separate follow-on work.
The annotation finding also requires route-specific treatment: the scalar-call
profile checks its annotations, while older executable profiles retain their
documented boundaries. This integration does not claim those features complete.

TU-wide typed declaration/AST indexing and correctly keyed CFG reuse remain open.

Full language conformance remains false. CT mapping, case-bound model proofs,
native observations and C projection coverage remain distinct evidence classes.
No provisional Unsafe finding is a validated upstream defect claim.

## Reconciliation and measurement boundaries

Configured C audits share one immutable preparation snapshot across identity,
inventory, analysis and reporting; measured logical Clang actions are 5→3 Safe,
8→3 provisional Unsafe and 6→3 Unknown with byte-identical JSON artifacts; no
total-speed or RSS reduction is claimed.

The canonical validation policy records the runner cancellation rule. The
source-identity policy records reconstructed certificate scope and separately
held expected context. Full compiler/build provenance and caller-intended
contract/profile/trust binding remain registered as
`CERT-SCOPE-COMPILETASK-PROVENANCE` and
`CERT-SCOPE-EXPECTED-TRUST-CONTEXT`. The matrix now carries 273 follow-ups.
`E01-TAIL-CERT-SCOPE` was a boundary named under `E01-F1-O`, not a separately
registered entry: the parent is annotated with its discharge, and remains open.
No registered entry is retired.

The [reconciliation record](reconciliation.json) addresses all 14 items from
the three cards. The residual sweep returns 55 pre-existing historical or
rule-only candidates outside these cards; it is not evidence that those names
are discharged. Both new review residuals are tracked.

## Final standing gate

The [gate](gate.json) and [complete run record](validation-run.json) distinguish
the full Rust measurement from the final mechanical lint cleanup. The full
Rust run compiled integrated source `02b037df`; `af88325b`, merged as
`e2d2f734`, subsequently removed an unnecessary temporary and borrow and added
two documented local lint allowances. It changes no branch, verdict, evidence
or public result representation. Final formatting, linting, probes, gateway
and audit regression checks validate that cleanup. The already successful full
Rust suite and corpus measurements were not repeated for those mechanical edits.

**4895 Rust passed, 0 failed, 10 ignored; 254 result summaries
over 252 targets; 151 Lean audits; 317 Python tests.**

| Command | Exit | Private log under `private/quirin-review/` |
|---|---:|---|
| `git submodule update --init --recursive` | 0 | `combined-gate/submodules.log` |
| `clang --version` | 0 | `combined-gate/clang-version.log` |
| `cargo test --workspace --no-fail-fast` | 0 | `combined-gate/cargo-test.log` |
| `cargo fmt --all -- --check` | 0 | `combined-gate/cargo-fmt.log` |
| `cargo clippy --all-targets -- -D warnings` | 0 | `combined-gate/cargo-clippy.log` |
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | `combined-gate/reevaluation-probe.log` |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | `combined-gate/reevaluation-followup.log` |
| `python3 scripts/check_lean_audits.py --jobs 2` | 0 | `combined-gate/lean-audits.log` |
| `cargo test -p qcore-compiler --test cycle20_verifier_plugins vp04_vp07_vp08_process_lifecycle_and_payload_budgets -- --exact --test-threads=1` | 0 | `combined-gate/isolated-process-lifecycle.log` |
| `cargo test -p qcore-verifier-host --test enforced_process_limits a_declared_process_limit_stops_the_fixture_from_forking -- --exact --test-threads=1` | 0 | `combined-gate/isolated-process-limit.log` |
| `cargo test -p qcore-verifier-host --test enforced_process_limits memory_growth_beyond_the_declared_limit_is_bounded -- --exact --test-threads=1` | 0 | `combined-gate/isolated-memory-limit.log` |
| `cargo test -p qcore-verifier-host --test confined_process_execution a_confined_fixture_cannot_write_outside_the_admitted_paths -- --exact --test-threads=1` | 0 | `combined-gate/isolated-confined-write.log` |
| `cargo test -p qcore-verifier-host --test standalone_host closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request -- --exact --test-threads=1` | 0 | `combined-gate/isolated-closed-input.log` |
| `cargo test -p qcore-c-gateway --test certificate_scope --test prepared_translation_unit --test prepared_failure` | 0 | `final-checks/gateway-focused.log` |
| `cargo test -p qcore-audit --test configured_cli` | 0 | `final-checks/audit-focused.log` |
| `python3 docs/reports/remaining-d02/build_matrix.py .` | 0 | `final-checks/matrix-premeasurement.log` |
| `python3 -m unittest discover -s scripts/tests` | 0 | `final-checks/python-measurement.log` |
| `python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/quirin-review-integration/gate.json` | 0 | `final-checks/matrix-final-corrected.log` |
| `python3 -m unittest discover -s scripts/tests` | 0 | `final-checks/python-final.log` |

The two successful matrix regenerations explain the two Python runs: the first measures
the suite after updating derived sections, and the second verifies the matrix
with the measured integrated gate. Clang 21.1.8 was recorded in this checkout.
Final focused checks pass 10 gateway tests and 17 configured-audit CLI tests
(27 passed, 0 failed, 0 ignored). The five isolated host-sensitive checks all
pass and are additional controls,
not additions to the workspace count. **Provider 122 is carried Unix evidence,
not remeasured**; provider and schema sources are unchanged.

The first measured-gate regeneration exited 1 because three descriptive keys
were outside the generator schema. Those counts remain in the note and run
record; removing the unsupported keys made regeneration pass without changing
any measurement or gate rule. The rejected command is preserved in
`private/quirin-review/final-checks/matrix-final.log` and the run record.

Per-card reports retain the earlier failed or superseded attempts: sandbox
LeakSanitizer/host-restriction failures, the scope semantic diagnostic-order
regression and its correction, interrupted scope runs, initial Clippy errors,
and stale-matrix Python failure. None is counted as a passing gate. The combined
run is unrestricted so native sanitizer and host-isolation tests can execute.
The initial reconciliation check required renumbering after the scope report
added its commit-provenance paragraph; the final check passes all 14 items and
15 file-content claims (`private/quirin-review/reconciliation-final-check.log`,
exit 0). Historical reports and the preceding 33,809-byte increment remain
unchanged. No optimistic fallback, reduced Unknown denominator or conformance
promotion is introduced.
