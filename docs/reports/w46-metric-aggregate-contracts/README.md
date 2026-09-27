# W46 metric aggregate contracts

The [prediction](PREDICTION.md) was committed at `a3ee486e` before editing the
contracts. Base: `207cebe4cdb745f6216ff2b511c4b7fd134581b9`.

## Source boundary and declarations

Six public CMetrics getters were added to the existing
`cmetrics/metric_values.qcore`: `cmt_metric_hist_get_value`,
`cmt_metric_hist_get_count_value`, `cmt_metric_hist_get_sum_value`,
`cmt_summary_quantile_get_value`, `cmt_summary_get_sum_value`, and
`cmt_summary_get_count_value`. The prior corpus census has 23
`uncontracted-callee` sites for this family (3, 4, 6, 3, 4, 3 respectively),
all in Unknown functions; it identifies no sole-blocker function for these
names. The actual after run, rather than this census, determines any verdict
movement.

The histogram bodies are at `lib/cmetrics/src/cmt_metric_histogram.c:170-194`;
the summary bodies at `lib/cmetrics/src/cmt_summary.c:129-158`. Their public
prototypes are in `cmt_metric.h:137-141` and `cmt_summary.h:61-64`. Source,
headers, and `cmt_math.h` have identical bytes at both reviewed Fluent Bit
revisions `66910c10a4d7eafa810b84229cf2dcf7b0f26f97` and
`046accf6aded7906f66a1baf0202da349e521736`. The two corresponding
read-only roots were the main checkout's
`build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source` and
`private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source`.
The histogram and summary source SHA-256 digests at both pins are
`c0188c906b1e56a1e0294dd2e9abf2183f159ca6d8403b0bd52f824c03117cc7`
and `fb3df5cd38613e844e2e37a5356cf3762f0d40a6d8ee96d5b1d7767fcc62925c`.

The bodies make atomic scalar loads, with bit-pattern conversion for sum and
quantile values. `cmt_summary_quantile_get_value` returns zero on a null
metric/quantile array or invalid index; the histogram bucket getter has no
index check. These resource-v1 declarations assert only unchanged caller
custody and no resource result. They make no bounds, null-safety, conversion,
atomicity, or concurrency claim. The reviewed atomic backend and conversion
behavior is documented by [W46 metric/trace contracts](../w46-metric-trace-contract-gaps/README.md);
this card adds no direct `cmt_atomic_*` declaration. The pinned declarations
remain source premises, not C-body proofs. The generated lock records exact
declaration and definition identities at both pins; ABI/pin mismatches fail
closed.

Each `struct cmt_metric *` maps to `ptr<mut, u8>`, `int` to `i32`,
`uint64_t` to `u64`, and `double` to `f64` under the existing LP64 ABI.
`providers/contracts.py sync` updates the package manifest and load-path
inventory, and the active CMetrics source-contract policy pins its new
manifest digest. The module count remains 118 corpus / 54 selected; the
resource-v1 declaration count becomes 225 (219 + 6).

## Controls and proof boundary

`metric_aggregate_contracts.rs` loads the actual owner module. Its accepting
fixture calls all six getters on a caller-owned pointer and releases it;
withdrawal of each specific declaration restores a named uncontracted-call
refusal. Separate controls retain a leak for an unreleased caller allocation
and refuse a call after release. `qcore_contract_corpus.rs` pins all six names
and their exact clause counts. Lean's `aggregateReadArities` records the four
unary and two indexed neutral vectors, proves their count and unchanged duty,
and is registered in `AuditMetricTraceContractGaps.lean`. The proof concerns
the modeled contract vector, with reviewed source-to-model correspondence as
a premise.

## For the integrator

Measure corpus and system after integration, attribute each changed leaf and
verdict, then update `docs/STATUS.md`, the acceptance matrix and exactly one
wave-46 increment paragraph from that primary evidence. No new residual is
retired: histogram bucket bounds, child array lifetime, backend concurrency,
and encoder/decoder ownership remain outside these scalar declarations.
Retain the W46 contract-gap expansion's eight follow-ups and existing OR-1/OR-7
refutations and held callers. This card changes no shared ledger file.

## Gate

Focused checks before the full gate: `cargo test -p qcore-c-gateway --test
metric_aggregate_contracts --offline` exited 0 (2/2); `cargo test -p
qcore-compiler --test qcore_contract_corpus --offline` exited 0 (4/4);
`quirin-audit pin --check` exited 0 at each reviewed revision (23 recorded,
0 not in force, 0 stale); `providers/contracts.py check` exited 0;
`scripts/check_lean_audits.py` exited 0 (212/212); and the script unit suite
exited 0 (364/364). `cargo fmt --all -- --check` first exited 1 on layout
in the new Rust controls and pinned corpus table; `cargo fmt --all` corrected
it before the final gate. `scripts/build_docs_index.py --check` first exited 1
after this report appeared; regeneration and the repeated check exited 0.

The required fresh-worktree `git submodule update --init --recursive` first
exited 1 because the sandbox could not create its Git worktree module
directory. The integrator initialized it from a local cache at exit 0; the
worktree now reports pinned `d82464a2`. A second scoped escalation request
was interrupted while awaiting approval and launched no full gate.

The integrator ran the full standing/provider gate at pre-repair tip
`5dcf8ff8` under `private/w46-next-full-gate/`. Its `steps.json` records
every exact command and exit, with a same-named `.log` and `.exit` for each
step. The workspace runner exited **101**: 325 units, 337 summaries, 6,028
passed, 3 failed, 11 ignored; its unit detail is in
`private/test-binaries/20260927T034856Z-191845/summary.json`. The three
failures came from two binaries: `qcore-compiler::contract_loader` (two tests)
found the deliberate count was still 219 rather than 225 and the six new
neutral declarations absent from its parity list; `qcore-audit::source_contract_config`
(one test) found the active CMetrics manifest digest had accidentally been
shortened to 54 characters. The original failing stdout logs remain under
that private test-binaries directory. These were real card errors, not
environmental failures.

The same full run recorded exit **0** for submodule update, workspace fmt and
Clippy, both reevaluation examples, Lean audit (212/212), script tests
(364/364), docs index, provider Cargo tests (129 passed, 0 failed, 1 ignored
across 43 summaries), provider fmt and Clippy, `providers/contracts.py check`,
package build, verification of all five first-party packages, provider Python
tests (15/15), and Clang version (`Ubuntu clang version 21.1.8`). Logs and
exits for each are in `private/w46-next-full-gate/`.

The two missed pins were repaired after that run: `contract_loader.rs` now
lists all six declarations with their exact arities and expects
`(54 selected, 225 v1, 216 rejected)`; the source policy now carries the
full 64-character manifest SHA-256. Focused post-repair checks all exited 0:
`contract_loader` 10/10, `source_contract_config` 12/12,
`providers/contracts.py check`, and workspace fmt. Their commands, exits and
logs are in `private/w46-metric-aggregates/repair-1/steps.json`. The
integrator's final union gate will establish a full green result at the
merged revision; the pre-repair workspace failure remains part of this
card's evidence.
