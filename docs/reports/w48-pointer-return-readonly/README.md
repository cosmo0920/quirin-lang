# W48 pointer return through a read-only call actual

Card W48-POINTER-RETURN-THROUGH-CALL-ACTUAL-ROOT, base `fa04aab0f`.
The committed prediction, implementation, and measured gate belong to this
report. The before corpus was rerun on this exact base in a shared detached
worktree before the prediction was committed.

## Source shape under review

The pinned librdkafka `rdkafka_admin.c` has six
`rd_kafka_admin_result_ret_*` helpers that pass
`&rko->rko_u.admin_result.results` to `rd_list_cnt` and then return the
list's `rl_elems` pointer. Four other accessors use the same list-count and
pointer-return shape. The pinned `rdlist.h` defines the static inline
`rd_list_cnt(const rd_list_t *rl)` with the sole statement
`return rl->rl_cnt;`. This body reads its actual and does not write through
it, retain it, or return a pointer.
The pinned source file SHA-256 values are
`9e820273add1503102190cd59586e78c674265bdd869a065519b1a9e072ae025`
for `rdlist.h` and
`8ed0e410007040262668e3b7b83dacf69241a8184ba174e7d32ab1d9dd4b1179`
for `rdkafka_admin.c` in the `66910c10` source checkout.

The current gateway's `position_pointer_root` refuses `rko` because the
lexical and AST `may_bind` readers conservatively mark names in call actuals.
The exception must be tied to a checked read-only body and an exact actual,
must keep every other binding cause, and must leave the normal call's own
verdict obligations in force. A `Borrowed` custody mode by itself does not
establish a read-only frame.

## Current-base measurement and prediction

The shared corpus baseline is `/tmp/qcore-rk-outer-base/private/rk-outer/
measure-base/corpus/resource-diagnostics.json`, retained read-only from an
arbiter run at exact base `fa04aab0f`. Its `run.json` records pinned source
`66910c10`, release binary SHA-256 `336609c8…`, `--jobs 4`, and exit 0.
The diagnostic digest is `fd731251…`, 941 / 9 / 3,884. A byte-exact copy and
the bottom-up plan derived from it are kept in this worktree's ignored
`private/w48bu-next/`. The shared [run record](artifacts/before/shared-run.json),
compressed [bottom-up plan](artifacts/before/plan.json.gz), and
[family leverage](artifacts/before/family.json) are archived here. The
[committed prediction](PREDICTION.md) follows that
measurement and precedes the rule code (`fcaad9ae1` before `6219f7d42`).
The shared exact-base system and tail-pause slice reports also exited 0.
Their invocation sidecars name the same prepared checkouts and compile
databases used by this card's after script, four jobs, and selection SHA-256
`d7e8ce76…` (system) and `11cc7e23…` (slice). Their profile-directory path
points to the detached base checkout; the profile and selection files are
byte-identical in this worktree.

The current plan finds 264 Unknown functions waiting on exactly one
in-corpus callee body, versus 338 in the earlier W48 report. Three remaining
admin helpers each have one return leaf and one singly blocked caller;
`rd_kafka_ConfigResource_configs` has one such return leaf with no caller
freed by it. The prediction is +6 to +7 Safe, subject to the arbiter.

## Rule and checks

`readonly_actual_frame.rs` checks the exact source signature and body of
`rd_list_cnt` in the unit being analysed. Only a direct AST `FunctionDecl`
call with one `&ROOT->field[.field]*` actual receives the read-only fact.
The lexical and AST readers replay their `may_bind` decisions without that
one actual; other calls, writes, addresses, protected formals, uncertain
declarations and closure bindings still refuse. The normal call analysis
must separately validate its own summary. The Lean model
`QCore.ReadOnlyActualFrame` proves that a checked exact actual preserves the
pointer-return root decision and another binding event still refuses it;
it relies on the standing token/AST extraction and Rust–Lean correspondence
premises.

Scoped preparation: `cargo check -p qcore-c-gateway -j2` exit 0; the three
lexical frame fixtures and one AST direct/shadow/write fixture pass (0
failures); `lake build QCore.ReadOnlyActualFrame` and
`lake env lean AuditReadOnlyActualFrame.lean` exit 0, with only the three
standard Lean axioms reported. The isolated release build
(`private/w48-pointer-return/tip-build-1/build.log`) exited 0; its
`quirin-audit` SHA-256 is
`e2bfb4d310d0ab062fb35bab327c2959760b3717fe9fe0679dae09aeeba8771c`.
Full gate and arbiter results follow below.

## Corpus arbiter and attribution

The isolated W48 binary ran the pinned `66910c10` corpus with four jobs and
typed-path parity enabled (`private/w48-pointer-return/tip-corpus-1/`, exit
0, diagnostic SHA-256
`ba2c53e1a67e018071c569e81d7a2c6521087033890bfc887165a02e6ac2ec92`).
Against the shared exact-base run, verdicts moved **941 / 9 / 3,884 →
952 / 9 / 3,873** (Safe / provisional Unsafe / Unknown): eleven Unknown →
Safe, no other status move. The [attribution](artifacts/after/compare.json)
finds exactly eleven removed `unmodeled call, aggregate or indirect access in
return expression` leaves, no other leaf delta and zero same-status rows
with changed diagnostics. The [run record](artifacts/after/run.json) fixes
the source, profile, input and binary identity.

All eleven moves are in `rdkafka_admin.c` and use the same checked source
shape: one direct `rd_list_cnt(&formal->list_field)` actual, followed by a
return of that list's `rl_elems` pointer. The six `rd_kafka_admin_result_ret_*`
helpers are `topics`, `resources`, `groups`, `acl_results`, `acl_bindings`,
and `delete_acl_result_responses`. Five direct accessors are
`rd_kafka_ConfigEntry_synonyms`, `rd_kafka_ListOffsets_result_infos`,
`rd_kafka_DescribeUserScramCredentials_result_descriptions`,
`rd_kafka_AlterUserScramCredentials_result_responses`, and
`rd_kafka_ElectLeadersResult_partitions`. The checked `rd_list_cnt` body is
the included pinned `rdlist.h` definition with one scalar read; the actual
is an address of a member under a formal, and the return is a read of that
same list's pointer field. The rule omits only this actual from the two
binding readers; the normal call obligations still discharge separately.

The committed prediction expected three of the helpers and three public
callers to move, possibly with `rd_kafka_ConfigResource_configs`. The
arbiter instead admitted eleven direct bodies and **no callers**. The three
ACL public callers still carry their `call to uncontracted function` leaf
even though the helper body is now Safe. `ConfigResource_configs` still
carries its return leaf: the typed return reader has no admitted plan for
`config->config.rl_elems` (parity `new_admitted=false`). The earlier
24-function closure was a hypothetical upper bound, not observed coverage.
Neither residual was overridden by this rule.

The refreshed [bottom-up plan](artifacts/after/plan.json.gz) has 260
singly body-blocked Unknown functions (264 before), 2,228 candidate bodies
(2,232 before), and an all-body hypothetical ceiling of 372, unchanged.
Its no-extra-body closure rises from 0 same-unit / 8 cross-unit to 4 / 12.
`cJSON_DetachItemViaPointer` remains the largest one-body candidate: six
callers, seven own leaves in branch and indirect-access families. The
[family leverage](artifacts/after/family.json) is archived for subsequent
cards. These closure numbers are planning bounds; only the eleven measured
status moves above are current Safe claims.

## System and slice comparison

The system and tail-pause slice before reports are the shared exact-base
artifacts under `/tmp/qcore-rk-outer-base/private/rk-outer/measure-base/`:
system 97 / 4 / 347 / 0 Pending (SHA-256
`1483b2aaad5b8eeec49a288998f46139b988e61581eac3b8a8aee650c76fc38b`),
slice 52 / 1 / 347 / 4 Pending (SHA-256
`9c0ae577e27984b7b63339ca0dee1e8a8b7e256c3419232f3cd6da455e399afa`).
The W48 binary's after system and slice runs both exited 0 under
`private/w48-pointer-return/tip-system-1/` and `tip-slice-1/`. Their
diagnostic SHA-256 values are `ee5da93803fd7a244f16096de2afa99e382cbf0a6087a0f93d9d8728f621f820`
and `83a1d25514063508d06a89b76cf8180dd88257e96596a155373f8345cd0d5016`.
Their invocation [system](artifacts/after/system-invocation.json) and
[slice](artifacts/after/slice-invocation.json) sidecars retain the same
prepared source checkout, compile database, four-job selection and source
identity as the base. Full diagnostic files differ in checkout/report path
metadata; `compare_configuration_rows.py` serializes every unit's file and
every function's `(name, status, diagnostics)` in configuration order.
The [system](artifacts/after/system-row-check.json) has 448/448 identical
rows with projection SHA-256 `2b6fd51b…` on both sides; the
[slice](artifacts/after/slice-row-check.json) has 404/404 with
`ed0b5469…`. Both status and diagnostic movement are exactly zero. The
legacy per-function [system](artifacts/after/system-compare.json) and
[slice](artifacts/after/slice-compare.json) comparisons agree, but collapse
configuration duplicates; the row checks are the coverage evidence.

## Standing gate

The first full-gate attempt (`private/w48-pointer-return/gate-final-1/`)
stopped on a `/tmp` disk quota error before it could record step exits. A
second attempt (`gate-final-2/`) recorded sandbox submodule exit 128, then
the Rust test build hit the same quota after 51 seconds; neither attempt
supports a test verdict. The worktree's 13 GiB `target` was moved intact to
`/home/cosmo/Documents/GitHub/qcore-c/private/w48-target` with the
coordinator's approval. A worktree `target` symlink resolves paths already
embedded in compiled test executables; `CARGO_TARGET_DIR` sends subsequent
builds to the preserved directory. No build output or failed-run log was
deleted. This storage location is the sole exception to the card's isolated
worktree output convention.

The sandboxed third attempt (`gate-final-3/`) completed every step. It
recorded submodule 128 because this sandbox cannot write the common Git
metadata; the coordinator ran the exact submodule command outside the
sandbox with exit 0. Rust test-binary runner exit 101 covered 512 units and
525 summaries: 8,080 passed, 143 failed, 15 ignored. All 143 failures
are classified from the named test logs: 92 sanitized native execution
failures (90 print LeakSanitizer's ptrace fatal error; two assert an exit
code from a sanitized child), 35 missing CLI executables from paths
embedded before target relocation, 15 process-confinement permission or
`unshare` denials, and one new parser-field declaration-order fixture.
The target symlink resolves the 35 path failures, and a focused CLI test
passed after it was added. The parser field classification was repaired,
then its focused test passed. Clippy's two explicit-auto-deref diagnostics
were repaired and a focused full-target Clippy check passed. The sandbox's
remaining steps recorded fmt 0, probe 0, followup 0, Lean audits 0,
matrix regeneration 0, Python suite 0, documentation index 0, and Clang
version 0. The 92 sanitizer and 15 confinement results require the full
gate outside this sandbox; no sanitizer or test was weakened.

The first outside-sandbox run (`gate-final-4/`) recorded step exits
0 / 101 / 0 / 0 / 0 / 0 / 0 / 0 / 0 / 0 / 0 in the order of
`scripts/gate.sh`. Its Rust runner recorded 512 units and 525 summaries:
8,219 passed, four failed, 15 ignored. All four failures were in the
audit-plugin example harness. Its compiled `CARGO_BIN_EXE` path referred to
the worktree `target`, but the first symlink attempt had landed *inside* a
small directory recreated by the prior gate, leaving that path unresolved.
The small directory was preserved at
`private/w48-pointer-return/misplaced-target-dir`; the worktree `target`
was replaced with the symlink to the relocated build directory for the
gate. The symlink was preserved afterwards as
`private/w48-pointer-return/final-target-link`. The focused
four-test harness then passed with exit 0.

The fifth full gate ran outside the sandbox from the repaired code commit
`d329c3171` with the relocated target and **all eleven steps exited 0**:
submodule, Rust test binaries, fmt, Clippy, reevaluation probe, followup,
Lean audits, matrix regeneration, Python suite, documentation index, and
Clang version. Its private logs and per-step exits are under
`private/w48-pointer-return/gate-final-5/`; `summary.txt` records the exit
vector. The Rust runner covered 512 units and 525 summaries with **8,223
passed, 0 failed, 15 ignored**. Lean passed **336/336** audits, including
the new read-only actual frame audit. The Python suite passed **374** tests.
The recorded Clang banner is `Ubuntu clang version 21.1.8 (6ubuntu1)`.
The [gate record](gate.json) pins these figures and the adjacent
`clang-version.txt` / `clang-version.exit` record its presence. W48 made
no provider package or provider-owned contract change, so the matrix's
`provider_tests=133` is explicitly carried from the previous
`OUTER-CONTRACT` gate, not presented as a fresh provider run. The
[acceptance matrix](../../reviews/r0_3/acceptance_matrix.json) was
regenerated against this gate record after the final exits. Its committed
matrix validator was then rerun separately: `python3 -m unittest discover
-s scripts/tests` exited 0 with 374 tests (`gate-final-5/post-matrix-python.log`
and `.exit`).
