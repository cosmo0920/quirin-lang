# Checked same-item detach

Work in progress from `e8b9ff07defe4c6375b1547ce2016c68a8ded98c`.
The user chose checked same-item transfer on 2026-10-18.
[The frozen prediction](PREDICTION.md) precedes implementation.

The C body does not establish membership merely by accepting a non-NULL item.
Checked containment, physical list representation, identity-preserving return
and the no-transfer NULL path are separate obligations. Measurements and gates
are pending; no verdict or coverage change is claimed by this preparation.

The implementation adds `resource_return_removed(item, nullable)` paired with
`resource_removed(item, parent, nullable)` and `resource_owned(parent,
nonnull)`. Loader checks require matching nullability, mutable depth-one heap
pointers, a distinct parent and no other custody positions or container/user
effects. The call preflight demands a unique live settled parent, one exact
identity whose only containing owner is that parent, and a settled contained
status. A null item transfers nothing. Unknown, borrowed, joined-owner,
already-detached or wrong-parent inputs refuse before any transfer. The
result receives the item's old binding and no produced site. Existing aliases
still designate that same obligation; releasing both aliases is not two valid
releases. Parent release reaches the remaining children only.

The bounded physical adapter compares the entire unlink body after parameter
renaming and normalization of actual zero constants, then checks a complete,
unambiguous record with same-record `child`, `next` and `prev` pointers.
Extra/missing writes, another return, extra calls and unreadable/qualified
link fields refuse the declaration in its defining unit. A successful
conditional template check does not make the body Safe or the declaration
Validated. Its exact named representation premise states the concrete/list
abstraction, including first.prev = tail, terminating last.next and the
singleton's self prev. It assumes no concurrent link writer. It supplies
neither membership nor the conclusion of the unlink. The premise remains a
DeclaredAssumption in every dependent verdict and certificate. No runtime
envelope is expanded; absent a covering envelope it is runtime Unknown.

The initial adapter deliberately refuses a null/unproven parent, even though
the C API accepts null parent. It also refuses whole-return and passed-result
consumers that currently require fresh or resource-free custody; it never
coerces this result to fresh-owned or forgets it into a neutral W4 summary.
The three borrowed lookup wrappers do not yet establish exact membership and
are expected to retain Unknown until that separate obligation is derived.

## Implementation checkpoint (before S3 integration)

The local pointer type view accepts one `*` and optional top-level `const` on
the by-value formal slot. It preserves the original source/signature in the
certificate and rejects pointee const, volatile/atomic qualifiers and deeper
pointers. This fixes an opaque classification of cJSON's actual `* const item`
without widening the shared lexical reader. Caller preflight also rejects an
identity participating in an unresolved replacement. Rebinding the item to
its own result preserves it; overwriting the sole parent name still leaves
that parent's obligation owed. An unproved forwarding wrapper stays Unknown
and yields no borrowed-only summary.

Targeted checks on the initial implementation are green: five model/parity,
two physical-frame, two loader, seven caller/native/replay and six
call-instantiation tests (22 distinct tests). `targeted-1` ran the first five
families with five caller tests; `targeted-2` reran the changed loader and
expanded seven-test caller binary. Commands/exits/logs are under
`/tmp/qcore-detach-same-item/private/detach-same-item/{targeted-1,targeted-2}`.
The native source fixture executes all 21 positions in lists of lengths
1–6, including singleton/two-node head, checks remaining order/tail links,
cleared item links, unrelated-object frame and no-write NULL paths. This is
an observation of the retained fixture, not execution of Fluent Bit's binary
or a production safety claim.

Pinned Lean 4.35.0 was checked with `lake env lean --version`. Both changed
modules compile with warning-as-error and their two print audits exit 0.
`AuditDetachSameItem` prints 17 model theorems, including ownership-inventory
conservation and physical write-frame/successor/tail-link properties. Changed
oleans are private; unchanged dependencies were borrowed read-only from the
main proof cache. This is targeted evidence, not the full Lean audit gate.
Commands and outputs are in `private/detach-same-item/lean-4` in this worktree.

Earlier attempts are retained: `compile-1` failed on a missing import/two
exhaustive null-model matches; `compile-2` exposed an unupdated schema version
dependency; `compile-3` passes. `caller-tests-1` was 3/4 while the physical
adapter classified top-level const as opaque; `caller-tests-2` is 4/4.
`physical-tests-1` through `-3` retain the initial refusal and its diagnostic
localization; `physical-tests-4` is 2/2 after the bounded signature fix.
`lean-1` records a reserved binder parse failure, `lean-2` a shell setup-path
failure before Lean launched, and `lean-3` a successful detach module compile
followed by a missing private dependency. The isolated private dependency
layout in `lean-4` fixes that setup; no assertion or linter was disabled.

Compile-only consumer checks also pass before integration: `cargo check
--locked --offline -p qcore-audit -j1` and the corresponding
`-p qcore-compiler --tests -j1` each exit 0. Commands and logs are in
`private/detach-same-item/{audit-check-1,compiler-check-1}` in this worktree.
These checks establish this checkpoint's compile compatibility only; the
cumulative S3/schema-19 gate remains pending.

The cumulative unpublished schema 19 event covers both S3's two new public
struct fields and this exhaustive result variant. Existing absent-field wire
bytes remain compatible for S3 defaults; Rust struct literals and exhaustive
result matches require migration. All active dependency/lock expectations
move to 19. Provider inventory/pin refresh, the full standing/provider/package
gates, all three fresh tip measurements and final tracking/increment/matrix
reconciliation remain pending after rebase onto the integrator's final S3 base.

## Integration addendum

Rebased onto integrator base `94b3267f36a1d4a84c20f75226da836430f627b3`,
which contains completed S3 and RK outer-scope receipts. The frozen prediction
above remains the preimplementation record. The C04.18 conflict retains both
S3's scoped/snapshot rules and this card's representation reader. S3's guarded
partition, lifetime fields and summary refusal remain intact. A Removed
result beside conditional or registered-lifetime syntax now rejects before
it can enter a richer retaining carrier; dedicated negative controls require
the expected boundary error. The compiler's stale outer-member comment is
corrected from 12 to 8 without changing the retained count of 81 clauses.

Generated source pins add only `cJSON_DetachItemViaPointer`, with identities
reviewed at both `66910c10` and `046accf6`. Both strict checks,
`pin --verify-contracts`, and provider inventory sync/check exit 0. Logs and
exact commands are under `private/detach-same-item/pins-cumulative-1`.
Preparation submodules, loader controls and debug audit build exit 0 in
`prepare-cumulative-1`. The two initial preparation runner setup errors
(missing plan-writer import, then absent plan) launched no checks and are
retained in `prepare-setup-failures.json`. Cumulative full gates and
measurements follow this source checkpoint.

Standing attempt `gate-host-1` at `772e19f46` records Rust-build and Clippy
exit 101: the existing `clang_gap_phase4` extent-vocabulary test exhaustively
listed the old four result modes. Its deliberate schema-19 migration adds
Removed as the fifth result mode without changing the assertion that derived
extents carry no offset or length. No Rust suite count follows from that
failed build. Submodules, recorded Clang, fmt and both reevaluation examples
exit 0; all 342 full Lean audits pass with the pinned toolchain. The complete
attempt remains retained; a repaired source checkpoint and full gate follow.

Standing attempt `gate-host-2` at `ac47d08ba` runs all 518 units / 531
summaries: 8,321 passed, 3 failed, 15 ignored. All remaining standing steps,
including Clippy and 342 Lean audits, exit 0. The three test failures expose
incomplete deliberate declaration pin migrations: the independent production
mode table needs the same-item Removed entry; the selected v1 declaration
total increases 491 to 492 (64 modules / 331 rejected unchanged); and the
five-clause ordered corpus tuple belongs after the existing cJSON `print`
entry, matching source declaration order. The repair keeps exact equality,
adds the explicit parent/item/result modes and retains every declaration.
The new detach controls all pass. This entire failed attempt is retained;
final validation follows another frozen source checkpoint.

## Superseding the first production refusal

The complete first frozen `0091c0a4d` sequence exits 0 on all routes and keeps
all statuses/diagnostics, all 5,781 null dictionaries and every runtime moved
record unchanged. It nevertheless withdraws the detach declaration in its
defining cJSON unit. These complete raw/comparison artifacts remain in
`artifacts/attempt-0091`; they are not the final conformance evidence.

Exact preprocessing under the recorded production compile command reproduces
`Err("parent-record")` in the new actual-input frame control at `6bc8509c3`
(exit 101, `actual-frame-before-1`). The macro-expanded signature is ordinary
`cJSON * ... (cJSON *parent, cJSON * const item)`; the physical frame/null
normalization is correct. The shared typed reader deliberately classifies a
typedef that defines its aggregate as opaque. The older fixture used a direct
`struct` pointer and missed this distinction.

The bounded repair resolves only a unique complete named nonunion defining
struct alias in detach's local formal view. Shared classification stays opaque;
original source/signature and certificate replay inputs remain unchanged.
Anonymous/union/pointer/conflicting aliases, ambiguous tags, const pointees,
volatile/atomic/deeper pointers and malformed frames still refuse. The exact
full preprocessed production unit plus the extracted retained frame now pass
(`actual-frame-after-1`), as do all seven caller/native/replay controls using a
defining-typedef fixture. No abstract transfer rule or representation assumption
changed; the existing Lean model applies with this explicit extraction boundary.
A final cumulative standing/provider/release/measurement checkpoint follows.
