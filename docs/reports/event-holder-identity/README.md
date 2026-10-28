# Whole-record holder frames and exact collector/loop identity

Implementation checkpoint on `codex/event-holder-identity`, based on ordered
lifetime integration source 153fd9cc3. The full card gate and declared native
comparison are pending; no native movement or complete production closure is
claimed by this checkpoint.

A source-derived lifetime record now carries separately checked holder frames.
A frame identifies one exact formal, struct and direct holder member. Its
complete straight-line body has no stores or unread effects, and every use of
the root passes the original member-path stability census or an already
source-derived internal callee frame. Frames compose bottom up. A whole-record
caller obtains a frame only from the exact typed direct actual and an internally
linked helper or independently replayed external home. Ordinary borrowed
custody remains insufficient. Both root readers can consult the framed record's
custody component, while actual calls still apply the full lifetime hook.

The portable schema major is 24.0.0. Inferred and cross-unit records retain
frames together with their effects; home replay independently regenerates and
compares them. Missing or altered frame evidence cannot authorize a caller.
Unaware consumers still see no ordinary borrowed contract. No provider contract
text or inventory pin changes in this card.

Exact immutable loop snapshots and collector copies retain their original
formal-rooted identity. Admission requires unique typed declaration,
initialization, one write, no address escape, compatible interpretation and
fully read straight-line control. Every collector-copy use participates in the
holder census. A matching numeric id, record type or allocation candidate set
creates no equality. Wrong-holder removal retains poisoned outgoing obligations.
A Safe body forwarding a poisoned lifetime effect is not a discharged lifetime;
its effect remains explicit and cannot become an ordinary borrowed summary.

Independent alias/liveness obligations remain intact. In reduced collector-copy
controls the lifetime identity discharges, but the overall row remains Unknown
because the existing alias-peer read refuses. This card does not grant a free
liveness or nonnull premise to recover that row. Direct whole-record registration
and loop snapshot controls do obtain replayed Safe certificates.

Fourteen audited Lean theorem checks model preservation under a closed write
frame, refusal of writes/escapes/unread calls, exact formal/record/member matching
and rejection of rebound/addressed/retyped/conditional aliases. They are abstract
model theorems: faithful Clang extraction, P-HOLDER-PATH-STABLE's original member
use premise, declared API semantics, link resolution and Rust/Lean correspondence
remain assumptions.

The focused source checkpoints retain commands and outputs under
`private/` in this worktree: frame-test-1 through frame-test-6, frame-clippy-1
through frame-clippy-3, initial-check.log and lean-frame-2.log. The final focused
run has 9 tests passed (5 cross-unit, 4 holder/identity); the matching Lean audit
passes all 14 theorem checks. Initial focused attempts exposed the two opaque
root readers and tests incorrectly treating a forwarded poison obligation as a
discharged Safe result. Controls now check the exact retained poison/effects.
The collector-copy control initially expected full custody closure; it instead
preserves and asserts the independent UnknownAlias refusal. Clippy's initial
argument-count and single-match failures were corrected without suppression.
The first submodule setup failed on sandbox DNS, then on disk exhaustion;
the host setup succeeded at the pinned d82464a277d0f42703702c4dfd9af6af38595a83.
No failed attempt counts as validation.

Open follow-ons remain on FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME: production
collector-by-id selection, conditional per-record instance/host/loop correlation,
store-derived loop equalities, collector-copy alias-liveness transport,
whole-record member-operand removal helpers, guarded/transitive imported-home
frames, all-path threaded join/destroy ordering, descriptor custody and automatic
backend contract-body validation. The bounded frame and exact identity work do
not retire this tracker.
