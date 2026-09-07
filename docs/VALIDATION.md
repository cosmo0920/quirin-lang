# Current validation and evidence boundaries

The current specification is [revision 0.3](QCORE_SPEC_r0_3.md); its
[acceptance ledger](reviews/r0_3/REVIEW.md) tracks implementation repairs and
remaining requirements. Full source-language conformance is not claimed.

The supplied archive review reported 39 Python validator tests, but explicitly
ran no Rust, Lean, native or sanitizer checks. Its probes/log package was not
among the five supplied files. Those historical claims are not substituted for
fresh repository validation. The older 48-test Python design experiment is
preserved [separately](obsolete/VALIDATION_r0_2.md); its prototype
is not a current compiler test command.

For a complete checkout, run the standing gate:

```sh
git -c protocol.file.allow=always submodule update --init --recursive
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo run -p qcore-compiler --example reevaluation_probe
cargo run -p qcore-compiler --example reevaluation_followup
(cd proof && lake build)
python3 -B scripts/check_lean_audits.py
python3 -B -m unittest discover -s scripts/tests -v
```

Clang is required for existing-C tests; `QCORE_CLANG` explicitly selects it.
Lean uses `proof/lean-toolchain`; ensure the elan bin directory is on PATH.
Native/sanitizer cases retain their platform and runtime requirements.
[Persistent build inputs](CYCLE19_BUILD_INPUTS.md) reproduce the pinned Monkey
compile database without temporary source/include dependencies.

Per-change execution logs live under `docs/reports/`. The r0.3 acceptance ledger
links the new semantic repairs after their gates pass. Mapped CT stages, model
proofs, source projections, native observations and provider certificates remain
separate evidence categories. Eleven of 50 CT cases are mapped; no raw Cycle19
unit is promoted by documentation or build success.

The [combined revision integration](reports/r03-integration/README.md) records
549 Rust tests, 43 Python validator tests, both probes, format/Clippy, Lean and
19 axiom audits passing, plus the unchanged 185-unit audit using persistent build
inputs. This is bounded validation, not full language conformance.

The later [W5 combined integration](reports/w5-resource-accounting/README.md)
passes 565 Rust tests, 43 Python tests, the standing gate and 19 Lean audits.
Separate provider-package tests and two reproducible builds against schemas
0.2.0 are archived in [the package report](reports/w5-provider-packages/README.md).
This adds architectural/transport evidence, not new language conformance.

[CLI package integration](reports/w5-cli-packages/README.md) subsequently
passes 569 workspace tests, the standing gate and 19 Lean audits, including
seven CLI process tests. Package snapshot verification does not establish
source correctness or immunity to later package-file mutation.

The [trusted verification adapter integration](reports/w5-verification-adapter/README.md)
passes 571 workspace tests and the standing/Lean gates. All relocated
derivation/replay/dispatch implementations retain byte parity.

The [generalization checkpoint](reports/generalization-integration-20260907/README.md)
passes579 workspace tests, the standing gate and19 Lean audits. Fresh full185
audit output matches the merged W4 forwarding result exactly:3 Safe /94
provisional Unsafe /84 Unknown /4 data-only. The report separately records an
open synthetic indirect-access probe; broader conformance is not claimed.

The [generic reactor integration](reports/w5-generic-reactor/README.md), including
the merged indirect-access repair, passes 589 workspace tests, 43 Python tests,
the standing gate and all 19 Lean audits. Unsupported reads now remain Unknown;
typed access restoration and broader conformance remain open.

[Declared callback record integration](reports/w5-reactor-callback-records/README.md)
passes 600 workspace tests, 43 Python tests, the standing gate and 19 Lean
audits, plus three excluded provider conformance tests. Full 185-unit source
and diagnostic output is byte-identical. Callback source derivation and
protocol witness admission remain open.

[Source-derived field provider integration](reports/w3-source-field-provider/README.md)
passes 611 workspace tests, 43 Python tests, the standing gate and 19 Lean
audits, plus four excluded provider conformance tests. The real ChunkIO process
provides replayed evidence for three bounded function contracts; 93 selected
functions remain Unknown. Full 185-unit source/diagnostic output and the prior
contract sidecar remain byte-identical. General lifecycle import remains open.

[Provider statement execution](reports/w5-provider-statement-execution/README.md)
passes the same 611 workspace tests and standing gate, plus nine excluded
provider workspace tests (four conformance and five executor tests). The process
computes source paths from typed statements; source import and contract replay
remain trusted. The program-input schema/profile are version 2 and ChunkIO is
package 0.3.0; this is not full source-language or lifecycle conformance.

[Context-specific custody recursion](reports/w4-contextual-recursion/README.md)
passes 619 workspace tests, including eight new public-API tests with actual C
imports and certificate rederivation, plus nine provider tests, 43 Python tests
and all standing/Lean gates. Zero/nonzero integer contexts cover bounded
decreasing calls; general recursive inference and source conformance remain open.

[Source task foundations](reports/w6-source-task-foundations/README.md) pass 636
workspace tests, nine provider tests, 43 Python tests and all standing/Lean gates.
Structured task resolution and the owned-resource ledger are checked separately;
the source driver still rejects task execution and CT coverage remains 5/50.
The companion [schema 1.0 migration](reports/schema-1-compatibility/README.md)
records standalone consumer compatibility and rebuilt provider identity parity.

[Native scoped-copy source tasks](reports/w6-native-source-tasks/README.md) pass
663 workspace tests, nine provider tests, 43 Python tests, formatting, Clippy,
both reevaluation probes, Lean build and 20 axiom audits. Real source/CLI C11
worker execution, creation failures and join-failure abort are covered on Linux.
One native copy fixture passes ASan/UBSan/LSan outside the tracing sandbox.
CT15 is newly mapped with target-specific stages; CT19 now uses actual sibling
failure source. Coverage is 6/50 mapped, 44 unmapped. All four full 185-unit
audit outputs are byte-identical; raw verdicts remain unchanged.

[Explicit task recovery](reports/w6-task-recovery/README.md) passes 686 workspace
tests, nine provider tests, 43 Python tests and every standing/Lean gate, with
20 axiom audit modules. Result handlers preserve prior siblings and exact worker
errors. CT13 maps actual Linux creation recovery with retained-storage checks;
coverage is 7/50 mapped, 43 unmapped. Normal and second-create failure controls
pass ASan/UBSan/LSan outside tracing; environmental tracing failures are archived.
All four full 185-unit audit outputs remain byte-identical with no promotions.

[Nullable removal and custody transfers](reports/w4-nullable-removal/README.md)
pass 716 workspace tests, nine provider tests, 43 Python tests, all standing
gates and 20 axiom audits, including 48 new model theorems. Nineteen public C11
tests cover nullable removal, ten reproduced false-Safe patterns, certificate
rederivation, singleton controls and nullable-old reallocation. Final replay of
the ten original probes gives six Unknown/four Unsafe, with no remaining Safe.
The full 185 audit has 2 Safe/42 provisional Unsafe/137 Unknown/four data-only;
five prior provisional Unsafe units become Unknown and 22 units change diagnostics.
Three other reports remain byte-identical and no raw unit is promoted. CT
coverage remains 7 mapped/43 unmapped; broader source and model correspondence
remain open.

[Configured source-contract dispatch](reports/w5-configured-source-contracts/README.md)
passes 722 workspace tests, 17 separate provider tests, 43 Python tests, workspace
and provider format/Clippy, both reevaluation probes, Lean build and 20 axiom
audits. The second-codebase process tests exercise actual Clang imports and
reject false postconditions, missing packages and identity/scope substitutions.
Process execution coverage is Unix-bound. No kernel or Lean semantics changed.
Both full 185-unit audits are byte-identical across all four artifacts to the
preceding baseline and to each other. Counts remain 2 Safe / 42 provisional
Unsafe / 137 Unknown, plus four data-only units; source evidence remains three
checked function contracts and 93 Unknown. CT mapping remains 7/50, with 43
unmapped; these test totals do not change that conformance boundary.

[Configured frame helpers and diagnostic attribution](reports/w5-configured-frame-helpers/README.md)
pass 737 workspace tests, 19 separate provider tests, 43 Python tests, both
format/Clippy gates, both probes, Lean build and 20 axiom audits. Ten new generic
frame theorems and separate-TU process tests cover protected-field preservation
under explicit source/build/ABI premises. Corpus pins are updated for 58 modules.
The full production comparison preserves all verdicts and source evidence;
the final report adds the selected helper-policy inventory. No raw promotion;
CT mappings remain 7/50, with 43 unmapped.

The [owned source-task increment](reports/w6-owned-source-tasks/README.md)
adds actual Buffer move/return and native creation rollback. CT01/CT02/CT12
now use real capture syntax and typed Buffer custody. Its 29 new Lean theorems
cover the model transfer/typing rules; fixed Linux native/sanitizer runs remain
separate from unproved compiler/runtime refinement. All four full 185-unit
Cycle19 artifacts retain exact byte parity with the preceding frame-helper report.

The [configured process-audit increment](reports/w5-configured-process-audit/README.md)
adds a real pinned second-codebase command run and seven end-to-end provider tests.
Main/helper source pinning, build-excluded Unknowns, failed process transport,
rejected evidence and explicit package/policy selection are exercised without
hand-constructed source summaries. Semantic replay and Lean rules are unchanged;
resource verdicts retain parity.

## Validated checkpoint at halt (2026-09-07)

Implementation revision `e3e7c2c` passed 819 workspace tests, 45 excluded provider
tests, 43 Python evidence-validator tests, formatting, Clippy, both reevaluation
examples, Lean build and 20 axiom audits. The [Clang CMake report](reports/w2-clang-cmake-build/README.md)
archives these gates, two successful native CMetrics builds, exact full 185-unit
parity and every configured-audit difference. CT coverage is 11 mapped / 39
unmapped. The [handoff](HANDOFF_2026-09-07.md) records unfinished requirements;
this checkpoint is not aggregate Cycle18/19/20 or full v0.3 acceptance.
