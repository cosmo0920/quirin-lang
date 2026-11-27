# Native platform test isolation and sanitizer portability

Base: `c6d4d65c5f6d9d37e30af32cdd99256434141164`.

The all-Unknown configured audit test used the developer's shared Libevent
worktree. Its intentional cleanliness gate includes ignored files; locally,
`test/regress.gen.c` and `test/regress.gen.h` triggered the reported refusal.
The focused pre-change run reproduced it (exit 101; 0 passed, 1 failed).

The fixture now clones the local object database into its temporary directory
and checks out the policy-pinned revision explicitly. No network is needed and
no generated file or user change in the shared submodule is removed. The audit
still checks the actual revision and complete worktree status. The same test
checks refusal without report publication for ignored, untracked and tracked
changes, restores each input, and successfully audits the clean fixture again.
The focused crate run passes: exit 0; 29 passed, 0 failed, 0 ignored.

The user also reported that the resource-helper sanitizer harness unconditionally
requested `detect_leaks=1`, which Apple's runtime rejects before executing the
fixture. The standing workspace run reproduced that failure. The test now uses
`detect_leaks=0` on macOS and retains `detect_leaks=1` on Linux, matching existing
sanitizer harnesses in this repository. ASan, UBSan, halt-on-error, the real helper
calls, output assertions and release remain enabled. The focused resource-helper
suite passes on this Apple Clang host: exit 0; 12 passed, 0 failed, 0 ignored.
This is macOS ASan/UBSan evidence, not macOS leak-detection evidence; Linux was
not executed in this session.

The callback task-handoff and timeout-teardown replays were using Darwin's
`__error` against Linux-recorded call sequences. An explicit
`QCORE_TEST_LINUX_LP64` fixture option selects `__errno_location` and the Linux
LP64 `uint64_t` spelling for those two import helpers only. Both are needed:
selecting the accessor alone repaired callback handoff but correctly failed the
timeout policy's exact source-summary digest. Selecting both reproduces an
existing pinned digest without editing any policy or historical evidence.
Default host imports keep their prior declarations. This is replay of a Linux
source model on macOS, not a new native macOS timeout observation.

The initial targeted attempt recorded 12/13 callback tests passing and 0/7
timeout tests passing. Its stricter task-spawn assertion expected a later effect
refusal, but the source-profile vocabulary already rejects that call. The final
test asserts that `cb_timeout` is Unknown at import and that projection refuses.
A separate control imports the supported `__error` spelling and verifies the
exact missing-effect refusal. Final targeted results: 13 callback tests and
7 timeout tests pass, exit 0.

The macOS all-target Clippy run also found `LEASE_FREE_WORKERS` and the
`diagnostic_sink` test module unused, because all their callers are Linux-only.
They now carry the same Linux target condition; no test is disabled by this
change and Linux code is unchanged.

Only test code changes. Production audit semantics and Lean definitions are
unchanged. The fixture still reports 0 observed and 7 Unknown rows. The existing
`CYCLE18-NATIVE-PLATFORM-MATRIX` foreign-host and observer-transparency follow-up
remains open; this fix adds no native observations or coverage claims.

Validation logs and captured exits are private to this worktree under
`private/native-platform-test-fix/`. Full standing-gate results follow below.
The checkout was already initialized; no submodule update was needed. Provider
sources, contracts and packages are unchanged, so provider-specific gates do
not apply.

## Validation

Measured on macOS arm64 with Apple Clang 21.0.0. The full workspace run began
with only the native-platform isolation fix; the sanitizer and callback fixes
were added after their failures were reproduced. It is not a green final-tree
workspace run. All nine failed Rust targets were subsequently confirmed green
in the focused runs below. No result is replaced or hidden.

All log stems below are under `private/native-platform-test-fix/`, paired with
`.log` and `.exit` unless otherwise noted.

| Command | Exit and result | Log stem |
|---|---|---|
| `cargo test -p qcore-audit --lib native_platform_matrix::tests::configured_all_unknown_gap_report_is_valid` (before fix) | 101; 0 passed / 1 failed | `baseline` |
| `cargo test -p qcore-audit --lib` | 0; 29 passed | `focused` |
| `cargo test --workspace --no-fail-fast` | 101; 4,961 passed / 27 failed / 10 ignored, 269 summaries | `workspace` |
| `cargo test -p qcore-compiler --test resource_helper_functions` | 0; 12 passed | `resource-helpers` |
| `cargo test -p qcore-compiler --test http_computed_length --test profile_loader --test resource_helper_functions --test socket_event_compose --test std_http --test std_socket --test std_socket_inet --no-fail-fast` (outside sandbox) | 0; 259 passed | `host-confirmation-2` |
| `cargo test -p qcore-verification --test callback_task_handoff --test timeout_teardown --no-fail-fast` (initial fix attempt) | 101; 12 passed / 8 failed | `callback-confirmation` |
| Same command after complete fixture fix | 0; 20 passed | `callback-confirmation-2` |
| `cargo test -p qcore-compiler --lib monkey --no-fail-fast` | 0; 18 passed | `monkey-host-confirmation` |
| `cargo fmt --all -- --check` | initial 1; final 0 after formatting | `fmt`, `fmt-final` |
| `cargo clippy --all-targets -- -D warnings` | initial 101; final 0 after Linux-only helper conditions | `clippy`, `clippy-confirmation` |
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | `probe` |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | `followup` |
| `python3 scripts/check_lean_audits.py` | 0; 165 audits passed | `lean` |
| `python3 -m unittest discover -s scripts/tests` | 1; 336 run, 17 failures, 6 skipped | `python` |
| Same Python command outside sandbox | 1; 336 run, 17 failures, 6 skipped | `python-host-confirmation` |

The initial workspace failures comprised the resource-helper sanitizer runtime
failure, five callback-handoff failures, seven timeout source-identity failures,
thirteen native socket failures in the restricted environment, and one
profile-loader scratch-file failure. All seven affected compiler targets pass
outside the sandbox; the profile loader uses a fixed shared temporary path,
consistent with interference from concurrent suites. Its test code was not
changed. One confirmation invocation used nonexistent test-target names and
exited 101 before running tests (`host-confirmation`); the corrected command
and result are recorded above.

Python failures are in existing guard-hook, reconciliation and census tests,
including macOS `/var` versus `/private/var` path spellings. No Python or hook
source is changed by this patch. These results are separate from the requested
Rust test fixes. Clang's clean stdout recording is `clang-version-final.txt`
with separate `.stderr` and `.exit` files.

The final diagnostic Python run used
`TMPDIR=/private/tmp python3 -m unittest discover -s scripts/tests` outside the
sandbox (`python-canonical-tmp`, exit 1): 336 run, 328 passed, 2 failed,
6 skipped. This removes all 15 path-alias failures. The two remaining failures
are `ReferenceTransactionGuard.test_the_incident_shape_is_refused_even_carrying_the_token`
and `ReferenceTransactionGuard.test_the_wave33_incident_shape_is_refused` in
`test_main_guard_hooks`. They concern existing ancestor-worktree detection on
this host and remain outside these test-harness fixes. The full standing gate
is therefore not claimed green. No hook is bypassed for committing or pushing.
