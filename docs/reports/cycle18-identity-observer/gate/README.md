# Validation record

Validation ran from the dedicated observer worktree at semantic tip
`a4260014244f6819c335614fa98e89ab8f6c9a6f`. Logs are private, untracked files
under the absolute directory
`/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-cycle18-observer/measure/cycle18-identity-observer/gate/`.

| Command | Exit | Measured result | Log |
|---|---:|---|---|
| `git submodule update --init --recursive` | 0 | Libevent at `d82464a277d0f42703702c4dfd9af6af38595a83` | startup terminal record |
| `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=4 cargo test --workspace --no-fail-fast` outside ptrace | 0 | 4,940 passed, 0 failed, 10 ignored | `workspace-escalated.log` |
| `cargo fmt --all -- --check` | 0 | no diff | `fmt.log` |
| `CARGO_BUILD_JOBS=2 cargo clippy --all-targets -- -D warnings` | 0 | warnings denied | `clippy.log` |
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | 8/8 probes passed | `probe.log` |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | 10/10 probes passed | `followup.log` |
| `python3 scripts/check_lean_audits.py` | 0 | 153/153 audits passed | `lean.log` |
| `python3 -m unittest discover -s scripts/tests` | 0 | 317/317 passed | `python.log` |
| `python3 build_matrix.py ../../.. --out /tmp/cycle18-identity-observer-matrix.json` from `docs/reports/remaining-d02` | 0 | scratch output derives 153 audit files; committed matrix untouched | `matrix.log` |
| `python3 scripts/cycle18_timed_wait_observation.py ...` outside ptrace | 0 | 11 records; ASan/UBSan/LSan clean | `native.log` |

An initial in-sandbox workspace run reached 4,834 passes and 106 failures. The
sampled failures were native sanitizer cases reporting that LeakSanitizer cannot
operate under ptrace. The required outside-ptrace rerun above is the reported
standing result; the diagnostic first run remains in `workspace.log` rather
than being hidden.

The final native manifest is
`measure/cycle18-final-native/output/run-manifest.json`. It records observation
SHA-256 `5da7bb4ea058c072888334cdab25a40a781421a47a8d7ec5c15263a41382f342`,
binding SHA-256 `039e7c0199052488dd21ab7fb9fea9c5c685dab5981b97873c3ce94272e0fbf2`,
instrumented pinned `libevent_core.a` SHA-256
`4b3653782bd43a50aa8cbf14e371076d4e7993a614c9e97f893fca2e2d7ba0c0`,
and executable SHA-256
`736f5dd62a6b10b24d83503dee2dece0b148e6864dae7c68fd3c34b18e1a7387`.
