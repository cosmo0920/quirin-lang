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
The pinned source file SHA-256 values are `9e820273…` for `rdlist.h` and
`8ed0e410…` for `rdkafka_admin.c` in the `66910c10` source checkout.

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

## Remaining route and gate evidence

The system and tail-pause slice before reports are the shared exact-base
artifacts under `/tmp/qcore-rk-outer-base/private/rk-outer/measure-base/`:
system 97 / 4 / 347 / 0 Pending (SHA-256 `1483b2aa…`), slice 52 / 1 /
347 / 4 Pending (SHA-256 `9c0ae577…`). Their after runs and the full
standing gate are queued under the coordinator's heavy-run lock.
