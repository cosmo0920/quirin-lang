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
