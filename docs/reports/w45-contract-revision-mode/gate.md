# W45-CONTRACT-REVISION-MODE gate

This gate covers the card after the integration review, merged with `main` at
`84ebad57` (W45-FB-RETRY-LIFETIME). It was measured at `9541bb39`, a
work-in-progress commit of the card's branch, by
[`scripts/gate.sh`](scripts/gate.sh). That branch was then squashed into one
commit on `84ebad57`. The squashed tree differs from the measured one only in
this file and `clang-version.txt`. Each step's output and exit are in its own
file under the worktree-private `private/rm-gate/`, and no step was piped.

| Step | Command | Exit | Count |
|---|---|---:|---|
| submodules | `git submodule update --init --recursive` | 0 | |
| clang | `clang --version` | 0 | Ubuntu clang 21.1.8 ([recording](clang-version.txt)) |
| test | `CARGO_INCREMENTAL=0 python3 scripts/run_test_binaries.py` | 0 | **5,840 passed / 0 failed / 11 ignored**, 312 summaries over 302 units (`private/test-binaries/20260926T072828Z-222033/`) |
| clean | `cargo clean --profile dev` | 0 | |
| fmt | `cargo fmt --all -- --check` | 0 | |
| clippy | `cargo clippy --all-targets -- -D warnings` | 0 | |
| probe / followup | `cargo run -p qcore-compiler --example reevaluation_probe` / `reevaluation_followup` | 0, 0 | |
| lean | `python3 scripts/check_lean_audits.py` | 0 | 198 audits (`main`'s 197 and `AuditContractRevisionMode.lean`) |
| python | `python3 -m unittest discover -s scripts/tests` | 0 | 358 tests, run after the matrix regeneration (`build_matrix.py .`, exit 0; `named_followups` 428 → 434) |
| docs index | `python3 scripts/build_docs_index.py --check` | 0 | |
| contracts | `python3 providers/contracts.py check` | 0 | resource views unchanged; every lock lists every declaration (62 pinned, 100 flexible) |
| provider Python | `python3 -m unittest discover -s providers/tests` | 0 | 15 tests (the 49-module resource-view pin included) |
| packages (constraint 18) | `python3 providers/package.py build --output private/rm-gate/packages`; `verify` × 5 | 0; 0 × 5 | |
| provider test | `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline --no-fail-fast` | 0 | 124 passed / 0 failed / 0 ignored, 39 summaries |
| provider fmt / clippy | `cargo fmt --manifest-path providers/Cargo.toml --all -- --check`; `cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings` | 0, 0 | |

Constraint 16's load-sensitive rows and
`real_packages_preserve_unknown_and_reject_mutations` passed inside these full
runs, so no isolated re-verification is claimed.

Earlier full gates:

* The same gate, pre-merge with the version-2 locks, at `4449e20c`: every step
  exit 0; 5,813 passed / 0 failed / 11 ignored over 301 units; 196 Lean
  audits; 358 and 15 Python tests; provider 124
  (`private/rm-gate-summary-v2-premerge.txt`).
* The card's first gate, at `92dd7ad6`: every step exit 0; 5,810 passed
  (`private/rm-gate-v1/`).

Runs outside the gate:

* **Arbiter on the merged tree.** `run_audit.py private/rm-arbiter-5 2ef12d99
  --jobs 8`, exit 0. Then `scripts/identity.sh` against
  `w45-fb-retry-lifetime/integration/artifacts`, all three exits 0
  ([identity/](identity/)).
* **Arbiter before the merge.** `private/rm-arbiter-4`, `2501709c`, exit 0
  ([identity/pre-merge/](identity/pre-merge/)).
* **Pilot slice runs.** `run-current.sh`: `card2-slice` and `variant2-slice` at
  both revisions, then `merged-slice` at both, every one exit 0.
  `pilot.py` exit 0; `singleplex.sh` with all three steps exit 0.
* **Mutations.** `mutations.py`, exit 0, with `all_met: true`.
* **Pins.** Every `pin` and `pin --check` log, with its exit, is in
  [locks/](locks/).
