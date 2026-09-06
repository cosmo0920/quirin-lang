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
separate evidence categories. Five of 50 CT cases are mapped; no raw Cycle19
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
