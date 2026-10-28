# Whole-record holder frames and exact collector/loop identity

Native comparison ran on `348a2081a220e17fe049083752544a623a7f2ad4` against published main `80c8aea989029c10b3dfbd0179715b7ea050262c`, based on ordered-lifetime integration source `153fd9cc36a3298372041add9d3d50ed4662a310`. Work remains unmerged on `codex/event-holder-identity`.

The final standing gate covers `2bb3260b4095fe1199549464d798fe4a65fc86d4`. The [source equivalence receipt](evidence/source-equivalence.json) records its only tracked difference from the native checkpoint: two S3 positive fixtures now use internal helpers and explicitly refuse their unreplayed external variants. All other tracked inputs, including checker, schema, provider/contracts and Lean sources, are identical. The native receipt retains its actual earlier source pin.

A source-derived lifetime record now carries separately checked holder frames.
A frame identifies one exact formal, struct and direct holder member. Its
complete straight-line body has no stores or unread effects, and every use of
the root passes the original member-path stability census or an already
source-derived internal callee frame. Frames compose bottom up. A whole-record
caller obtains a frame only from the exact typed direct actual and an internally
linked helper or independently replayed external home. The complete caller body must preserve
the outgoing holder too: a later opaque whole-record call or holder write
withholds forwarding. Ordinary borrowed
custody remains insufficient. Both root readers can consult the framed record's
custody component, while actual calls still apply the full lifetime hook.

The portable schema major is 24.0.0. Inferred and cross-unit records retain
frames together with their effects; home replay independently regenerates and
compares them. Missing or altered frame evidence cannot authorize a caller.
Unaware consumers still see no ordinary borrowed contract. No provider contract
text or inventory pin changes in this card.

Exact immutable loop snapshots and copies of collector pointers retain their original
formal-rooted identity. Admission requires unique typed declaration,
initialization, one write, no address escape, compatible interpretation and
fully read straight-line control. Every use of a copied collector pointer participates in the
holder census. A matching numeric id, record type or allocation candidate set
creates no equality. Alias names must be unique across formals and locals;
shadowing cannot create spelling-wide identity. Address initializers, by-value
record formals and pointer-slot aliases are refused. The typed source census and
caller controls exercise those boundaries. Wrong-holder removal retains poisoned
outgoing obligations.
A Safe body forwarding a poisoned lifetime effect is not a discharged lifetime;
its effect remains explicit and cannot become an ordinary borrowed summary.

Independent alias/liveness obligations remain intact. In reduced controls using copied collector pointers the lifetime identity discharges, but the overall row remains Unknown
because the existing alias-peer read refuses. This card does not grant a free
liveness or nonnull premise to recover that row. Direct whole-record registration
and loop snapshot controls do obtain replayed Safe certificates.

Sixteen audited Lean theorem checks model preservation under a closed write
frame, refusal of writes/escapes/unread calls, exact formal/record/member matching
and rejection of rebound/addressed/retyped/conditional aliases. They are abstract
model theorems: faithful Clang extraction, P-HOLDER-PATH-STABLE's original member
use premise, declared API semantics, link resolution and Rust/Lean correspondence
remain assumptions.

The focused source checkpoints retain commands and outputs under
`private/` in this worktree: frame-test-1 through frame-test-11, frame-clippy-1
through frame-clippy-5, initial-check.log and lean-frame-3.log. The holder/identity focused
run has 10 tests passed (5 cross-unit, 5 holder/identity); the matching Lean audit
passes all 16 theorem checks. Initial focused attempts exposed the two opaque
root readers and tests incorrectly treating a forwarded poison obligation as a
discharged Safe result. Controls now check the exact retained poison/effects.
The control with a copied collector pointer initially expected full custody closure; it instead
preserves and asserts the independent UnknownAlias refusal. Clippy's initial
argument-count and single-match failures were corrected without suppression.
The first submodule setup failed on sandbox DNS, then on disk exhaustion;
the host setup succeeded at the pinned d82464a277d0f42703702c4dfd9af6af38595a83.
No failed attempt counts as validation. Superseded source prerequisite runs
and interrupted admission waiters are retained separately. The superseded native
waiters started no native audits; none of those runs supplies final gate credit.

Open follow-ons remain on FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME: production
collector-by-id selection, conditional per-record instance/host/loop correlation,
store-derived loop equalities, liveness transport for copied collector pointers,
whole-record member-operand removal helpers, guarded/transitive imported-home
frames, all-path threaded join/destroy ordering, descriptor custody and automatic
backend contract-body validation. The bounded frame and exact identity work do
not retire this tracker.

The final gate records every command, exit and private log in [gate-steps.json](evidence/gate-steps.json); all 14 steps exited 0. This run captured its own Clang banner.

| Check | Recorded result |
|---|---|
| Root Rust | 8847 passed, 0 failed, 15 ignored; 574 units, 593 summaries, 24 filtered |
| Provider Rust | 133 passed, 0 failed, 1 ignored; 43 summaries |
| Lean | 391 audits |
| Python | 554 tests, 0 skipped |
| Packaging and static checks | Five packages rebuilt and verified; root/provider fmt and Clippy, both reevaluation probes passed |

The initial standing attempt on native checkpoint `348a2081a` completed with Rust 8,845 passed / 2 failed / 15 ignored and provider 133 passed / 0 failed / 1 ignored; Python admission did not run. The two failures were S3 positive snippets assuming a same-unit externally linked body could supply lifetime transport. The stricter source-bound rule requires an internal helper or independently replayed home for registration forwarding as well as removal. The corrected fixtures use internal helpers and explicitly retain Unknown for their unreplayed external variants; the bounded focused rerun passed all 21 tests. No checker definition was relaxed. The queued focused slot request was cancelled before it launched a child; it is not a failed test execution. The initial fixture formatting check exited 1 before `cargo fmt`; no private copy of that terminal diff was retained. The final gate above owns the actual formatting and Clippy results.

The declared native comparison completed on all three routes. Resource and NULL rows, denominators and verdict hashes match the published baseline. These are conditional profile observations; provisional Unsafe rows are not validated upstream defects.

| Route | Resource Safe / Unknown / Pending / provisional Unsafe | NULL Safe / Unknown / provisional Unsafe |
|---|---|---|
| corpus | 1013 / 3811 / 0 / 10 (4834 rows) | 2549 / 2355 / 17 (4921 rows) |
| slice | 57 / 342 / 4 / 1 (404 rows) | 190 / 216 / 0 (406 rows) |
| system | 101 / 343 / 0 / 4 (448 rows) | 211 / 243 / 0 (454 rows) |

The [native receipt](evidence/native-receipt.json) retains exact raw reports and SHA256-checked gzip round trips. The validator compares every payload in full; expected revision and working-directory identity renewal requires independently retained preimages, including task/obligation and reading-site binding recomputation. The complete reviewed patch ledger records every remaining delta. Run `python3 docs/reports/event-holder-identity/evidence/validate-native.py --receipt docs/reports/event-holder-identity/evidence/native-receipt.json --reviewed-deltas docs/reports/event-holder-identity/evidence/reviewed-deltas.json.gz` to replay archive validation. This checks archived observations and identity attribution; it does not establish backend-body conformance.

Focused failures, missing exit records and superseded prerequisites are retained in [attempts.json](evidence/attempts.json); none supplies final gate credit. Private originals are under `private/holder-identity/` and the worktree-wide `private/frame-*` / `private/lean-frame-*` paths. A records-only Python rerun follows the completion commit and is recorded separately. The proposed [integration amendments](integration-amendments.json) append the existing open tracker; they do not advance the live profiles or claim production closure.

The retirement preservation target is root `private/retired-worktrees/2026-10-28-sweep/codex-event-holder-worktree/`, containing the private logs and a report copy. Tracked source remains on the branch. Native measurement inputs and outputs remain in root `private/event-holder-native-20261028/`. The records-only admission is retained as `private/holder-identity/records-python.log`, `.exit` and `.json` after the completion commit.
