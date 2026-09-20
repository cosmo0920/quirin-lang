# Prepared translation unit sharing (review priority O1)

## Result

The configured `qcore-audit` paths now construct one immutable
`PreparedTranslationUnit` per unit and reuse it for compile-task identity,
main-source definition inventory, resource projection and diagnostic coordinate
reporting. Preparation retains the `clang -E -P` output and the independently
produced line-marker output as separate strings. It syntax-checks the former,
derives the inventory and line map once, and records an outcome, elapsed wall
time and actual successful Clang spawn count for each of those three stages.

This is within-run sharing only. It adds no persistent cache, CFG cache or typed
AST cache, and changes no resource model or verdict rule. The independent Safe
certificate checker still re-analyzes the certificate's preprocessed source and
re-derives inferred custody summaries. Preparation is evidence supplied to
issuance and reporting, never a reason to skip certificate rechecking.

## Fail-closed boundary

Preparation always returns one snapshot, including an original validation
failure. Compile-task identity records that same failure and projection reads it
without retrying. A deliberately failing-once Clang wrapper pins this property:
the rejected `-E -P` stage occurs once and the prepared analysis returns the
stored error rather than retrying into a verdict. The auxiliary marked
preprocessing still supplies the best available denominator; its own failure is
represented by the existing `ExpansionUnavailable` inventory and location
refusals.

The exact raw source, compiler dialect, function scope, include paths,
definitions, ordered preprocessing options and declared input identity are
retained with the prepared products. The separately implemented certificate
scope work can therefore bind issuance to this same snapshot without starting
another preprocessing process.

## Parity and invocation measurement

`prepared_translation_unit.rs` compares the complete legacy result on synthetic
Safe, provisional Unsafe and Unknown programs before and after using the prepared
entry point. It also drives a header-defined whole-function macro and checks that
the shared inventory names the expanded definition while the shared coordinate
map attributes it to the main-source invocation line. The two preprocessing
forms must differ and are never reconstructed from one another.

An independent harness at
`docs/reports/quirin-review-integration/measure_preparation.py` ran the baseline
`a8f2048d` binary and after revision `9e62c19b` over the same three configured
fixtures. Its command log excludes compiler version/target probes and classifies
the analysis actions. JSON outputs were byte-identical for every pair.

| Fixture verdict | Baseline logical Clang actions | Prepared logical Clang actions | Prepared stages |
|---|---:|---:|---|
| Safe | 5 | 3 | one `-E -P`, one syntax, one marked `-E` |
| provisional Unsafe | 8 | 3 | one `-E -P`, one syntax, one marked `-E` |
| Unknown | 6 | 3 | one `-E -P`, one syntax, one marked `-E` |

Compiler identity/discovery probes are reported separately by that harness:
9/12/10 at baseline and 7 for each prepared fixture. They are not folded into
the O1 analysis-action counts. GNU `time` observed peak RSS in KiB as
83,028→83,404 (Safe), 84,656→83,860 (Unsafe), and 84,252→83,936 (Unknown).
Those small changes move in both directions and establish no memory reduction.
The harness records per-stage timings and complete command lines in
`docs/reports/quirin-review-integration/preparation-measurement.json`; this
card makes no total-speedup claim.

The configured-audit regression independently wraps Clang and asserts exactly
one invocation of each preparation action. The CLI prints aggregate stage wall
times and spawn counts for the run without adding nondeterministic timings to
the reproducible JSON artifacts.

## Limitations

- Compatibility callers that invoke the historical public validation,
  enumeration and mapping functions separately still pay for separate work.
  The configured audit paths use the prepared route.
- Per-function typed AST queries, TU-wide declaration indexing and CFG reuse
  remain the review's O2/O3 work. This card does not cache them.
- A preparation failure records elapsed time and actual spawns for attempted
  stages; an unattempted syntax stage is explicitly `not_attempted`.
- Peak RSS and stage timings are host observations, not proof evidence and not
  performance guarantees.

## Validation

The required submodule initialization completed at pinned libevent revision
`d82464a277d0f42703702c4dfd9af6af38595a83`. Private logs are under
`measure/review-prepared-tu/gate/`.

| Command | Exit | Result / private log |
|---|---:|---|
| `git submodule update --init --recursive` | 0 | pinned revision above |
| `cargo test --workspace --no-fail-fast` | 0 | 4,888 passed / 0 failed / 10 ignored, 253 summaries; `cargo-test-escalated.log` |
| `cargo fmt --all -- --check` | 0 | `fmt.log` |
| `cargo clippy --all-targets -- -D warnings` | 0 | `clippy.log` |
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | `reevaluation-probe.log` |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | `reevaluation-followup.log` |
| `python3 scripts/check_lean_audits.py --jobs 2` | 0 | 150/150 audits; `lean.log` |
| `python3 -m unittest discover -s scripts/tests` | 0 | 313 tests; `python.log` |

Two earlier attempts are retained but excluded from the gate. `cargo-test.log`
used `script`, whose pseudo-terminal made LeakSanitizer report that it could not
run under ptrace, and was interrupted. `cargo-test-direct.log` ran inside the
sandbox, encountered the same native-test environment limitation and completed
there after its local session had been interrupted; sandbox-local process
inventory did not expose the still-running host process. Neither is a valid
workspace result. `cargo-test-escalated.log` is the one complete unrestricted
run and is the only run counted above.

No Lean source changed because preparation sharing changes orchestration and
retained evidence, not the semantic model.

## Integrator reconciliation list

1. Apply the prepared-TU commits `863530cd`, `1ae3c7d8`, and `9e62c19b` before
   the certificate-scope commit, then resolve certificate issuance to consume
   `raw_source()`, `line_marked_source()` and the retained request fields. Do not
   reintroduce marked preprocessing on the prepared audit path.
2. Keep the independent measurement record under
   `docs/reports/quirin-review-integration/` as the authority for baseline and
   combined after counts. This report references rather than duplicates its
   command logs and timing samples.
3. No `docs/STATUS.md`, increment record, acceptance matrix or global count
   update is proposed: verdicts and evidence coverage do not move. The exact
   proposed review-close text is: “Configured C audits share one immutable
   preparation snapshot across identity, inventory, analysis and reporting;
   measured logical Clang actions are 5→3 Safe, 8→3 provisional Unsafe and 6→3
   Unknown with byte-identical JSON artifacts; no total-speed or RSS reduction
   is claimed.”
4. Preserve O2/O3 as open optimization scope: TU-wide typed declaration/AST
   indexing and correctly keyed CFG reuse are not implemented here.

