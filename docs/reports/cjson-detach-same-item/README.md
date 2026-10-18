# Checked same-item detach

The user chose checked same-item transfer on 2026-10-18. The implementation
starts from `e8b9ff07defe4c6375b1547ce2016c68a8ded98c` and is integrated onto
`94b3267f36a1d4a84c20f75226da836430f627b3`.
[The frozen prediction](PREDICTION.md) precedes implementation. Dated checkpoint
sections below retain their original pending statements; final receipts supersede
those statements without changing the frozen prediction.

The C body does not establish membership merely by accepting a non-NULL item.
Checked containment, physical list representation, identity-preserving return
and the no-transfer NULL path are separate obligations. Final gates and all
three measurements pass; production custody counts do not improve. The actual
declaration remains conditional and all six wrappers remain Unknown.

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

A nonmember input that rewires links is a violated API/precondition example,
not an upstream defect claim. The source body alone does not prove membership.

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

The first complete green standing gate at `0091c0a4d` records **518 units /
531 summaries, 8,324 passed / 0 failed / 15 ignored**, 342 Lean audits and
zero exits for every standing step run there. Its fresh provider gate records
133 Rust / 0 failed / 1 ignored, 25 Python, all five builds/verifications and
zero exits throughout. Its release exits 0, SHA-256
`a8f3c85800d0469cac5ef43f9a00245ac3695d52f2cd28ff8691a4409eb5dcd4`.
The final actual-source regression adds one root Rust test, making the later
final standing total 8,325. Root Python and matrix closeout follow final
measurement rather than being borrowed from either checkpoint.

The complete first frozen `0091c0a4d` sequence exits 0 on all routes and keeps
all statuses/diagnostics, all 5,781 null dictionaries and every runtime moved
record unchanged. It nevertheless withdraws the detach declaration in its
defining cJSON unit. These complete raw/comparison artifacts remain in
`artifacts/attempt-0091`; they do not establish the repaired production adapter connection.

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


## Final validation and compatibility receipt

The final analyzer/proof source is `9bfe86a4115c4a7e3e4fb0de93c2e037ef5fcfbe`.
The actual-input regression at `6bc8509c3` retains its exit-101 failure; the
bounded repair and all qualifier/alias/frame negatives pass. No source change
follows the final standing gate. The retained commands and raw outputs are in
[final standing evidence](artifacts/final-standing-gate/commands.json),
[provider evidence](artifacts/provider-gate/commands.json) and
[release evidence](artifacts/release/commands.json). Private originals are under
`/tmp/qcore-detach-same-item/private/detach-same-item/` in `gate-host-4`,
`provider-gate-2` and `release-2`.

The host-access standing gate exits 0: **518 units / 531 summaries, 8,325 Rust
passed / 0 failed / 15 ignored**, fmt, Clippy with warnings denied, both
reevaluation examples, initialized submodules and **342 placeholder-free Lean
audits** using pinned Lean 4.35.0. The actual gate records Clang 21.1.8 in this
worktree. The fresh provider workspace gate exits 0: **133 Rust passed / 0
failed / 1 ignored**, **25 Python**, check, fmt, Clippy, contract sync/check,
and all five provider package build/verify pairs. The provider conformance
workspace depends on the changed gateway, so its suite is rerun rather than
borrowed from `0091c0a4d`; all five newly verified package manifests/digests
happen to reproduce that earlier checkpoint exactly.

This closes one unpublished **schema 19** compatibility event: the exhaustive
`ResourceReturnMode::Removed` addition, S3's public
`RetainingContract.lifetime: Option<LoopLifetimeContract>`, and
`ResourceSummaryRecord.lifetime_effects: Vec<ForwardedLifetime>`. Exhaustive
matches and old Rust struct literals must migrate even though S3's absent-field
wire encoding is preserved by defaults. The schema README names all three
migrations and distinguishes wire compatibility from Rust source compatibility;
all active dependencies, locks and five packages use 19. S3's frozen schema-18
measurements remain historical semantic evidence, superseded for publishable
compatibility by this cumulative source and gate.

The release build exits 0 with `-j3`. Its immutable private binary is
`private/detach-same-item/quirin-audit-detach-9bfe86a41`, SHA-256
`9f10a5bbf704eb3c56af4c2e6c6c6ebe78e761cd4b7208937701c2a5295c3f06`.
The first frozen `0091c0a4d` binary and complete all-three-route refusal sequence
are preserved separately under `artifacts/attempt-0091`, including their
standing/provider/release receipts. They are not silently replaced by final
measurements.

Before the final corpus audit, the new release completes the actual defining-unit
integration control with the exact production database, profile, 64-module
trusted bundle and six-home map. This selects **one unit after whole-database
member setup**, not one Clang invocation. The jobs-1 attempt was cleanly
cancelled through its verified exclusive captured process group (exit 130);
the fresh jobs-4 run changes only worker count and output paths and exits 0.
Both attempts, input hashes and raw module aliases are in
[the control receipt](artifacts/actual-unit-control/completed-jobs4/checked-control.json).
The production declaration is **pinned-in-force / DeclaredAssumption**, with
zero failed or withdrawn units, one unvalidated unit, zero validated units,
four dependent rows and zero dependent Safe rows. The named representation
premise is active. This establishes the production reader connection, not
unconditional C conformance, membership, a production Safe body or runtime
coverage. No bound, assertion, closure policy or native sanitizer was disabled.

## Final corpus and all seven body outcomes

The final `measure-final-2` corpus exits 0 against the completed S3/RK baseline:
**961 Safe / 3,864 Unknown / 9 provisional Unsafe**, identical over all 4,834
selected custody rows. This implements the checked transfer without claiming
six promotions. The production declaration is pinned-in-force with its exact
`detach_list_representation` premise: four dependent rows, zero Safe rows,
zero failed/withdrawn units, one unvalidated unit and zero validated units.
The physical template remains conditional DeclaredAssumption. The ordinary
callee projection is still Unknown.

[The complete seven-body records](artifacts/seven-body-review.json.gz) retain
both sides, all diagnostics, locations and dependency carriers. Every row below
is Unknown before and after; no caller is counted as discharged.

| Function | Final refusal / obligation |
| --- | --- |
| `cJSON_DetachItemViaPointer` | Ordinary projection has 11 leaves: three branch/aggregate refusals, six indirect accesses and two unresolved `item` identities. Conditional physical-template evidence is separate. |
| `cJSON_DetachItemFromArray` | Nested `get_array_item` inside the modeled call remains unsupported; exact membership and removed-result transport are not supplied by a borrowed lookup. |
| `cJSON_DetachItemFromObject` | Unsupported whole-return expression; same-item custody and membership must survive result transport. |
| `cJSON_DetachItemFromObjectCaseSensitive` | The same unsupported whole-return boundary, independently retained. |
| `cJSON_DeleteItemFromArray` | Nested `cJSON_DetachItemFromArray` inside `cJSON_Delete` remains unsupported. |
| `cJSON_DeleteItemFromObject` | Nested `cJSON_DetachItemFromObject` remains unsupported. |
| `cJSON_DeleteItemFromObjectCaseSensitive` | Nested `cJSON_DetachItemFromObjectCaseSensitive` remains unsupported. |

Exactly four full corpus rows change: the callee's ordinary diagnostics grow
7 to 11, and the three direct wrappers exchange uncontracted-detach leaves for
the nested lookup/whole-return refusals above. The other 4,830 full rows are
identical. The callee and three direct wrappers explicitly consult the new
declaration. Aggregate diagnostic counts consequently gain two unsupported
and two unresolved-alias leaves; this is not a new validated violation.

[The full artifact review](artifacts/corpus/review-findings.json) retains raw
and normalized keys and complete changed records. Monkey/source and chunk
artifacts are identical after explicit checkout normalization. All 64 normalized
contract-module keys agree; only CJSON module text changes, with its intentional
pin/count/lock/declaration additions. Six conditional-home environment identities
change across 185 units (1,110 raw fields), with the same six sidecar changes.
One empty lifetime-summary key is renamed in both prepass/evidence carriers;
no lifetime effect is erased. All three source-effect payloads are identical
after the finite published build/configuration/task/witness/obligation identity
mapping (29 raw changed fields); their full raw evidence is retained.

Every one of the 4,921 corpus null dictionaries, custody/combined verdicts,
all three runtime scenario count sets and complete moved-row records is
unchanged. Runtime all-loaded custody stays **931 Safe / 30 Pending / 3,864
Unknown / 9 provisional Unsafe**. The outer selection's complete objects agree
after explicit checkout-prefix normalization; no envelope or candidate inventory
is widened. Static custody, null/combined, loaded-runtime projection, conditional
model proof and native fixture observations remain distinct evidence scopes.

The fresh [before](artifacts/before-plan.json.gz) and
[after](artifacts/after-plan.json.gz) plans give **260 → 258 single-body waits,
2,224 → 2,223 candidate bodies, hypothetical ceiling 365 → 363**, with no-extra-
body same-unit/cross-unit closure **4 / 4 → 4 / 4**. These are planning-model
counts. Two wrappers leave the call-only census for unsupported return effects;
the array wrapper now waits on `get_array_item`. No body or caller becomes Safe.
The former six-caller detach hypothesis is therefore not realized by the
ownership vocabulary alone. `get_array_item` now has three hypothetical closure
rows, and `rd_kafka_log_buf` leads the remaining table at four; neither number
proves membership, usable summary transport or a Safe promotion.

## Final configured routes, provenance and closeout

All three final audits exit 0 with four workers and the unchanged six-home map:

| Route | Safe | Unknown | Pending | Provisional Unsafe | Selected custody rows |
| --- | ---: | ---: | ---: | ---: | ---: |
| Corpus `66910c10` | 961 | 3,864 | 0 | 9 | 4,834 |
| System `046accf6` | 99 | 345 | 0 | 4 | 448 |
| Slice **actual `34155955e4962a56d94eb23c03a002880899a0d1`** | 53 | 346 | 4 | 1 | 404 |

The [system](artifacts/system/review-findings.json) and
[slice](artifacts/slice/review-findings.json) preserve all statuses and full
diagnostics. Their four/eight raw changed rows contain only module checkout
paths in existing review holds, preserving two compile configurations per
function. Complete compile/dependency/inventory changes are retained (45/48
finite metadata fields), alongside the new loaded detach premise and intentional
pin/lock/count/module additions. Neither configured selection consults the
detach declaration in a selected function, so no selected-body validation is
claimed. Configured Monkey/source output is absent on both sides and explicitly
marked absent; the other four artifact families are archived and compared.
All 64 normalized module keys agree, with only CJSON text changed; all source-
effect payloads are unchanged. All **4,921 + 454 + 406 = 5,781 null dictionaries**,
static combined verdicts, every loaded-runtime scenario and complete moved-row
record agree. Raw source/contract paths and duplicates remain available.

The completed S3/RK baseline is reused, with no new before audit: analyzer/proof
`d0a48bfd612e875254e72a4852e6930e718159cf`, measurement HEAD
`14dfc6aac79070f3c1d946e22c1247addf7978e8`, immutable binary SHA
`f5ca84194827defe0c5dc36f59d7424ccf1e91eb252e324e0e2d5c458eedd974`.
Its private outputs are
`/tmp/qcore-fb-lifetime-s3/private/fb-lifetime-s3/measure-final-1/{corpus,system,slice}`;
[baseline provenance](artifacts/baseline-provenance.json) records the integration
base and public S3 source. The final output root is
`/tmp/qcore-detach-same-item/private/detach-same-item/measure-final-2`, with
corpus primary nested under `corpus/corpus`. Each route's `after/after-hashes.json`
binds source `9bfe86a4115c4a7e3e4fb0de93c2e037ef5fcfbe` and both running/immutable
binary SHA `9f10a5bbf704eb3c56af4c2e6c6c6ebe78e761cd4b7208937701c2a5295c3f06`
after completion. Only report/tracking changes follow that measured source.

[Measurement commands/exits](artifacts/final-measurements/commands.json) record
2,922.387 / 402.801 / 333.651 seconds for corpus/system/slice, each exit 0.
Native invocations, stdout/stderr and all five artifact families are retained;
[comparison commands/exits](artifacts/final-review/commands.json) bind the
read-only full-row/field/key/null/runtime review and fresh replan. One post-audit
recorder initially expected the configured sidecar's optional top-level premise
field; corpus stores it on the declaration instead. The recorder setup failure
and repair are disclosed in `artifacts/route-recorder-setup.json`; binary/hash
assertions and native audit exit 0 are unchanged. Matrix, root Python and index
closeout receipts follow under `artifacts/closeout`.

The closeout regenerates the acceptance matrix with the cumulative schema-19
[gate](gate.json), then runs **374 root Python tests, all passing**, after that
final matrix. Documentation-index generation/check and `git diff --check` exit
0. The ledger is **802 → 805**, marks the detach decision approved/implemented,
retains the fresh **258 / 2,223 / 363; closure4/4** plan, and registers the three
follow-ons below. Exactly one newest increment is added to wave-48 part9, which
remains **30,979 bytes**, below32KiB; no rotation is needed. The actual Clang
recording beside this gate is copied from this same worktree/source gate-host-4,
not borrowed from another report. These report/tracking changes do not alter
the frozen analyzer/proof/binary measured above.

## Follow-ons

The checked transfer decision is approved and implemented; the remaining work
below does not reopen that ownership decision.

- `DETACH-LOOKUP-MEMBERSHIP`: derive exact child identity and membership from a
  checked lookup frame, including the NULL/missing-key/index branches and
  invalidating writes. Borrowed-from provenance alone must never populate the
  containing-owner relation.
- `DETACH-RESULT-WRAPPER-COMPOSITION`: transport an existing removed identity
  through whole-return and nested-call wrappers, preserving parent custody,
  membership prerequisites, result aliases, NULL branches and the named
  representation premise in the summary and independent replay. Ordinary W4
  inference or a fresh-result route must not erase those effects.
- `DETACH-LIST-REPRESENTATION-CLOSURE`: independently establish the named
  concrete/list relation and its frame/lifetime requirements where stronger
  source assurance is claimed. The current declaration remains conditional;
  the physical-template check neither establishes caller membership nor turns
  the still-unprojected production body into a Safe function. Runtime coverage
  needs an actual covering envelope rather than a static-model theorem.

## For the integrator

1. Preserve one unpublished schema-19 event covering Removed and S3's two public
   fields, with this cumulative gate/package receipt as the compatibility
   closeout. S3's frozen schema-18 evidence remains a dated semantic checkpoint.
2. Preserve S3's guarded partition, lifetime effects and independent replay,
   plus the explicit rejection of condition/lifetime carriers around Removed
   results until that composition has its own checked rule. Keep the physical
   representation premise DeclaredAssumption, caller membership separate and
   same-result identity intact; no ordinary W4 or fresh-result erasure.
3. Keep the RK loader's corrected eight kept members/81 total clauses; no
   Fluent Bit instance promises belong on `rd_kafka_plugin_new`. Retain the
   current implemented detach decision, final measured plan and all three
   residual trackers. Historical predictions keep their dated statements.
4. Retain the exact gated/measured source and frozen binary provenance when
   merging the card's report/status/ledger/increment/matrix amendments. Equal
   figures need no duplicate full gate. A later semantic change needs a fresh
   gate and measurement rather than implicit adoption of this frozen receipt.
