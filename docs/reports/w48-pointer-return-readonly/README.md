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
