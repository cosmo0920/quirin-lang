# Cycle 18 native platform matrix

Base: `229578d8ad617803bed3f02bcc1f0f59251d560d`.
Status: fail-closed matrix capability implemented; four current Linux rows
observed and three foreign-host rows remain `Unknown`.

## Result

`quirin-audit native-platform-matrix CONFIG.json` checks a seven-row
host/backend/sanitizer/thread-adapter/observer matrix. Each required profile is
an executable Quirin declaration. The configured audit reads and hashes the
bundled probe, policy, build/link command records, core and pthread archives,
executables, CMake caches, compilation databases, raw runtime records, run manifests, pinned Libevent checkout and
historical validation record. It rereads inputs before atomic publication and
refuses report aliases, including hard links and output inside the source tree.
Configure/build/compile/link argument vectors are checked positionally against
the selected paths; a basename or unrelated define cannot satisfy a role.

The Linux measurement used pinned Libevent
`d82464a277d0f42703702c4dfd9af6af38595a83`, Clang 21.1.8, Linux
7.0.0-31-generic x86_64 and real `libevent_pthreads.a` after
`evthread_use_pthreads`. A distinct aborting guard timer holds the loop. A
worker activates the tested event with `EV_READ`; the callback records flags
`2` on the dispatcher thread. This distinguishes activation from timer expiry.

| Host / backend | Sanitizers | Current result | Native process |
|---|---|---|---|
| Linux / epoll | ASan + UBSan + LSan | observed | exit 0; distinct activating thread; `EV_READ` on dispatcher |
| Linux / poll | ASan + UBSan + LSan | observed | exit 0; same primitive checks |
| Linux / select | ASan + UBSan + LSan | observed | exit 0; same primitive checks |
| Linux / epoll | TSan | observed | exit 0; same checks; no TSan report |
| macOS / kqueue | ASan + UBSan | `Unknown` | no current run |
| Windows / WIN32 | ASan | `Unknown` | no current run |
| Windows / IOCP profile | ASan | `Unknown` | no current run |

The foreign rows preserve earlier native results from
`docs/cycles/CYCLE_18_LIBEVENT_VALIDATION.md`, SHA-256
`f0717d75797037234bdf0aefb7b7738c5ee0d1bcdec527ffe4738f0e5cc76c44`.
Those results lack the current sanitizer, observer, thread-adapter and selected
artifact join and therefore do not promote a row.

## Scope and evidence

The checked property is only `generic_libevent_cross_thread_activation`.
Observer scope is `generic_libevent_only`; no Monkey callback or wrapper runs.
Every row carries `observer_transparency: not_claimed`. Quirin compares the
primitive source/build/runtime identities, host/backend, three thread IDs,
callback reason and counts. Hashes identify producer-supplied artifacts but do
not attest the producer.

The policy adds ten declarations and 68 clauses. Matching audited Lean models
cover fail-closed carried history, current joined evidence and transparency.
Rust-to-Lean correspondence remains review-maintained under constraint 28.

The reproducible runner is `scripts/cycle18_native_platform_matrix.py`; the
Linux CI job invokes it and the configured audit without uploading generated
evidence. Complete measurement files are worktree-private under
`measure/cycle18-native-platform-matrix/`; focused logs are in its `final/`
subdirectory.

| Command | Exit / result |
|---|---|
| `python3 scripts/cycle18_native_platform_matrix.py --output measure/cycle18-native-platform-matrix` | 0; three ASan/UBSan/LSan processes and one TSan process |
| `cargo run -p qcore-audit --bin quirin-audit -- native-platform-matrix measure/cycle18-native-platform-matrix/audit-config.json` | 0; four observed / three Unknown |
| `python3 scripts/check_cycle18_platform_controls.py ...` | 0; three rehashed controls each refused with audit exit 1 |
| `cargo test -p qcore-verification -p qcore-audit native_platform_matrix --no-fail-fast` | 0; 6 focused tests passed |
| `cargo fmt --all -- --check` | 0 |
| `cargo clippy -p qcore-verification -p qcore-audit --all-targets -- -D warnings` | 0 |
| `lake build QCore.NativePlatformMatrix` | 0 |
| `lake env lean AuditNativePlatformMatrix.lean` | 0; no project axioms; `propext` only on one Boolean theorem |

Final native hashes are in the private configured report. Common ASan/UBSan/LSan
executable: `faae850ac480553054e710b6e7ec526071b8684e6e140393b61c77d8ad3a4654`;
TSan executable: `cb2824d1a72043f41d1547ba5c5fc049840decaeb50a920032acd621b06e9960`;
probe source: `a2ee320c04c2b8f2834f6bb6a0b571d11e32a895ce08e13e5720778fff12250d`.
The controls rehashed a core/pthread archive-role swap, a raw same-thread
runtime plus its manifest, and an observer-transparency assertion. They reached
the exact argv, primitive Quirin identity and transparency gates respectively.

## Remaining host blockers

- macOS must run the current identity-bearing probe with forced kqueue, native
  pthread adapter and Apple ASan/UBSan artifacts.
- Windows needs a native equivalent using `evthread_use_windows_threads`, a
  forced WIN32 row and a separate IOCP regression-profile row. The Linux
  `gettid` harness is intentionally not claimed portable.
- These generic probes do not establish Monkey observer transparency or any
  Monkey callback composition result.

## Integrator reconciliation list

- Update the shared `contract/qcore` corpus pins for the added ten-declaration,
  68-clause policy.
- Keep `CYCLE18-NATIVE-PLATFORM-MATRIX` registered, or replace it with explicit
  macOS/kqueue and Windows/WIN32+IOCP current-join residuals.
- Do not change Monkey callback accounting, Cycle 18 aggregate status, CT
  coverage or full-conformance status from this generic probe.
- Preserve historical Linux/macOS/Windows results while describing the three
  foreign rows as current selected-observer `Unknown`.
- Reconcile the new workflow with integration-time CI consolidation; it
  intentionally publishes no generated evidence artifact.
