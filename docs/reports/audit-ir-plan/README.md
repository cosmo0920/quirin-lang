# AUDIT-IR-PLAN implementation receipt

Base: `d03f6c864f84cfc837db09f7bdb0a2292bc32aab`.
Specification: [AUDIT-IR](../../development/audit-ir-card.md).

Prediction recorded before implementation: extracting the existing call graph
into interned IDs and an iterative component plan must preserve exact ordered
SCC membership, recursion flags and every existing analysis/evidence result.
The original algorithm remains a test oracle. No contract, schema, semantic
rule, proof definition or pinned source identity is changed.

## Initial implementation

`qcore-c-gateway::audit_ir::AuditPlan` consumes the existing W4 call graph.
Its symbol table owns each name once; forward/reverse edges, DFS state,
component membership and component dependencies use dense IDs and vectors.
Temporary standard-library hash tables resolve names, while explicit lexical
ordering preserves deterministic IDs and the original traversal. The existing
producer still constructs its ordered graph at the boundary; no blanket map
replacement or measured speedup is claimed.

W4 now reads the plan's exact ordered components and recursion flags. Its
inference, declared precedence, retry order, ranked/neutral recursion, own-body
validation and evidence publication gates are unchanged. Per-component name
vectors remain at that existing interface; bodies/CFGs are not persisted or
evicted yet. The original graph is released after conversion, and the parity
recorder reconstructs a borrowed name view only when recording is enabled.
Non-body dependency names remain in forward/reverse edges and never become
analyzed bodies, inferred contracts or safety evidence.

This is representation work, not a new analysis rule. Existing audited models
`QCore.CustodySummary`, `QCore.SummaryValidationPremises`,
`QCore.NeutralFrameRecursion`, `QCore.NeutralPrefixPreservation` and
`QCore.MemberSummaryOrder` retain their meaning. No proof of Rust extraction or
general Tarjan correctness is added or claimed.

## Bounded parity

[The receipt](bounded-parity.json) records three passing Rust controls:
all **66,067** directed graphs on zero through four bodies (including self
edges), external non-body boundaries, and a **20,000-body** chain on a 256 KiB
thread stack. The exhaustive control compares exact ordered components with
the original algorithm, recursion flags, the complete JSON graph view,
forward/reverse edges and predecessor component indexes. Its archived oracle
is byte-equal to the base function after restoring its function name.
The executable harness is `crates/qcore-c-gateway/tests/unit/audit_ir.rs`.
These observations are bounded planner parity, not a full production-corpus
replay or a memory measurement.

## Validation provenance

Implementation began at the base above. The shared checkout received the
unrelated guarded-decomposition integration before the recorded gate; that
gate's starting HEAD is `45cff3b248fb1ac26ffc2829efe377bb7a1eb63f`, plus this
card's changes. Exact commands, counts, exits and logs are recorded separately
below; no borrowed gate is presented as this card's own.

The first focused compile failed (exit 101) on an ambiguous empty-array test
comparison; `is_empty()` fixed it. The next focused run passed 3/3, formatting
passed and all-target Clippy with warnings denied passed. The subsequent full
workspace build exhausted disk before executing tests; both the runner and
the outer recorder exited 1, and the recorder could not write `rust.exit`.
The raw failure and supplemental `interruption.json` remain private. That
attempt is excluded from validation evidence. Only executable build artifacts
written during this run's exclusive Cargo build interval were reclaimed,
with exact paths/times/sizes retained in `own-build-executables.json` and
`reclaimed-build-output.json`; sources and older build outputs were untouched.
A retry uses its own `/tmp` target with `CARGO_INCREMENTAL=0`,
`CARGO_PROFILE_DEV_DEBUG=0` and four build/test workers. Optimization level,
debug assertions and overflow checks remain development defaults.

`git submodule update --init --recursive` exited 128 because the existing
Libevent Git config is read-only in this sandbox. Read-only inspection
independently confirmed the installed commit equals the gitlink,
`d82464a277d0f42703702c4dfd9af6af38595a83`; this is an existing checkout, not a
new worktree. An approved retry outside the read-only sandbox subsequently
exited 0 (`20261021T174337Z-retry/submodules-retry.json`). The original failed
attempt remains recorded.

Private root: `/home/cosmo/Documents/GitHub/qcore-c/private/audit-ir-plan/`.
The initial gate is `20261021T173818Z/`; the fresh target retry is
`20261021T174337Z-retry/`. Each retains commands, environment and per-step logs.

### Actual gate outcomes

[Machine-readable commands, exits, counts and source hashes](gate.json) retain
every failure. **The full standing gate is not green.** No provider-owned input
or published schema changed, so provider packaging gates are not added.

| Command / scope | Exit | Result / private log relative to the private root |
| --- | --- | --- |
| `git submodule update --init --recursive` | 128, then 0 | Original read-only-config error; approved retry in `20261021T174337Z-retry/submodules-retry.json` |
| `cargo test -p qcore-c-gateway --lib audit_ir::tests -- --nocapture` | 0 | 3 passed; `20261021T173818Z/focused.log` |
| `cargo fmt --all -- --check` | 0 | `20261021T173818Z/fmt.log` |
| `cargo clippy --all-targets -- -D warnings` | 0 | `20261021T173818Z/clippy.log` |
| `python3 scripts/run_test_binaries.py --jobs 4 --log-dir …/rust-binaries` | 101 | 536 units / 549 summaries; 8,361 passed / 108 failed / 15 ignored; `20261021T174337Z-retry/rust.log` |
| Exact captured failed-unit executables, sequentially outside sandbox | 0 | 47/47 units pass with identical test inventories and denominators; `20261021T175508Z-unsandboxed-rechecks/rechecks.json` |
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | `20261021T174337Z-retry/reevaluation-probe.log` |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | `20261021T174337Z-retry/reevaluation-followup.log` |
| `python3 scripts/check_lean_audits.py --jobs 2` | 0 | 361 audits; `20261021T174337Z-retry/lean.log` |
| `python3 -m unittest discover -s scripts/tests` | 1 | 374 run, 373 passed / 1 failed; `20261021T174337Z-retry/python.log` |

The sandboxed Rust failures include LeakSanitizer's fatal ptrace restriction
and process-confinement/quota fixtures. Rechecks use Cargo's original captured
argv, cwd and environment; no sanitizer or test condition is disabled. The
recheck driver is retained at `private/audit-ir-plan/recheck_failed.py`.
Replacing only those failed units yields latest per-unit evidence of
**8,469 passed / 0 failed / 15 ignored** across the same 536 units / 549
summaries. This is a composition of recorded runs, not a standalone full-run
exit 0; the original exit 101 and every failed row remain in the receipt.

Python's sole failure is the committed matrix's stale Lean audit count:
`test_the_derived_facts_are_recomputed_rather_than_carried` expects the matrix
to match the tracked source count. [Committed-HEAD evidence](committed-matrix-boundary.json)
shows 360 in the matrix versus 361 audit sources at the same validation HEAD.
The unrelated guarded-decomposition integration added the audit; this card
adds or edits no proof file. Regenerating that integration's committed matrix
is left to its owner, rather than silently editing another card's evidence.
Final Python after tracking all card files again ran 374 tests with the same
sole failure (exit 1), recorded in `20261021T175857Z-final-python/receipt.json`.
The task-owned temporary build target was removed after all test/recheck
processes completed; logs and receipts remain in the private root.

## Follow-ons and acceptance boundary

The parent [AUDIT-IR card](../../development/audit-ir-card.md) remains open:
`AUDIT-IR-FRAGMENTS` must bind and round-trip complete body facts and manifests;
`AUDIT-IR-LIFETIMES` must actually release AST/CFG/buffer/report allocations;
`AUDIT-IR-BUDGET` must admit work under the aggregate child-inclusive budget
and measure cold/warm complete audits on the corpus/system/slice selections.
`W45-CHECKPOINT-JOBS-MEMORY` and the existing audit-memory-performance
follow-ons remain open. No runtime budget, persistent IR, incremental cache
admission, production coverage change or 6–7 GB success is claimed here.
`AUDIT-IR-INTEGRATION-MATRIX-LEAN-COUNT` remains open until the integration owner
regenerates the matrix from current source under its owned gate and reruns the
committed-matrix Python checks. Until then this slice has implementation and
the passing evidence above, but does not claim full-gate closure.
