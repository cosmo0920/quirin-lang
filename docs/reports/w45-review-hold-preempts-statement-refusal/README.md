# W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL

A verdict-contract soundness repair of gateway rule 10i
([amendment](../../agents/gateway.md#constraint-10i)). Base: local `main` at
`fefb9199`. Branch `worktree-agent-a699eb30a516b6732`.
[PREDICTION.md](PREDICTION.md) was frozen before any arbiter run (its order is
disclosed there).

## 1. The defect

Two rewrites hold a call for a named human act: a declaration awaiting review
(`qcore_review_pending__`, contract spec C06.1) and a pinned declaration whose
lock the source does not match (`qcore_pin_mismatch__`, C06.2). The held
statement refuses with a `Pending` reason. Two things made a row `Pending`
below its real standing:

1. **The held refusal replaces the statement's other refusal.** The statement
   classifier checks the hold before the lexical access gate
   (`legacy_c.rs`, the `W45-CONTRACT-DEFERRED-REVIEW` hook), and a statement
   reports one refusal. `i_ins = flb_input_new(ctx->config, …)` reported only
   the held call, although with `flb_input_new` accepted the `ctx->config`
   actual keeps `indirect memory access is outside the resource projection`.
2. **The held call has no effect.** Nothing the accepted declaration would
   make of the path is seen: a second release, a leak, an argument the
   declaration does not admit (`k_close(0)` against `resource_released(h, …)`:
   `arguments do not match resource contract`), or a body the accepted
   declaration's own row refutes (the revision-mode fixture's
   `dropped-release`, where a re-pin as written leaves `box_drop` in
   `summary_conflict` and its caller's call withdrawn).

`Pending` claims that a named human act alone stands between the row and
`Safe` (rule 10i, §L21.2). In every case above the act would leave the row
`Unknown`, so publishing `Pending` put a verdict below the function's real one
(`Pending < Unknown`): hard invariant 2.

A refusal on a **later** statement is not masked: the refused statement's
writes are havocked and the path goes on, so the held analysis records it
(fixture `a_later_refusal_on_the_path_is_recorded_by_the_held_analysis`). The
`pending_human_review` section's `meaning` says a path stops at its first
refused statement; that holds for the second held call it counts
(`W45-REVIEW-HELD-CALL-SITE-COUNT`), not for every refusal.

## 2. Where the fix belongs, and the mechanism

A per-statement repair (keep the access refusal beside the held one) would fix
case 1 only partly: which actuals a position admits depends on the
declaration in force (a `Borrowed` position reads its actual; the classifier
must know the contract to decide), and it cannot see case 2 at all. The hold
itself must stay: it is what keeps a withheld declaration from discharging or
accusing. So the reasons are **re-derived**
(`crates/qcore-c-gateway/src/legacy_review_standing.rs`):

- `ResourceContracts::with_human_holds_in_force` (qcore-schemas): the
  environment with every pending-review and pin-mismatch declaration moved into
  the function map as written, nothing held; the refutation table is kept.
- `analyze_legacy_c_translation_unit_selected_partial` (every unit route) and
  `analyze_legacy_c_tokens` (the single-function entry) run their held
  analysis as before. If some row is `Pending`, `review_standing::settle_with`
  runs the same analysis once more under that environment, with parity rows
  muted, and joins every `Pending` row with each reason above `Pending` its
  function carries there. Each is recorded as an `Unsupported` diagnostic
  `reason standing beside a call held for a human act: with every held
  declaration in force as written, this function still carries: …`; a
  violation there is recorded the same way (`a double release candidate, not
  validated: …`), never as a violation. The row is re-folded, so it is
  `Unknown`; the unit status is re-folded.
- A row whose second analysis is `Safe` keeps `Pending`. A failed second
  analysis, a missing row, or a `Pending` row with nothing held keeps nothing
  `Pending` (an `unavailable` standing reason).
- Rows that are not `Pending` are never read; a unit with none is analysed
  exactly once. So the change can only move a `Pending` row to `Unknown`.

The held-call diagnostics stay, so the `pending_human_review` tally, the unit
reason's held-call component and its reason code are unchanged (see §6 for
the code).

## 3. The other holds (task item 3)

| Prefix | Verdict of the hold | Preemption | Disposition |
|---|---|---|---|
| `qcore_review_pending__` | `Pending` | masked a standing reason | **fixed** (§2) |
| `qcore_pin_mismatch__` | `Pending` | the same | **fixed** (§2) |
| `qcore_refuted_by_body__` | `Unknown` (`stale-evidence`) | replaces the statement's refusal | attribution only |
| `qcore_abi_withdrawn__`, `qcore_mode_withdrawn__` | `Unknown` | the same | attribution only |
| `qcore_premise_unmet__` (with `loop_owner__`, `thread_sync__`) | `Unknown` | the same | attribution only |
| `qcore_source_spelled__` (escape) | `Unknown` | the same | attribution only |

An `Unknown` hold absorbs what it preempts: the preempted reason is a
statement refusal (itself `Unknown`), and a violation the withdrawn
declaration would allege is not validated while it is withdrawn. Proved as
`an_unknown_hold_absorbs_every_refusal`; measured size of a verdict effect: 0
by construction, so nothing is registered. The Quirin-language extern-call
route (`qcore_verification::driver`, `pending_review_refusal`) emits a `Q023`
diagnostic per declaration and continues, so it masks nothing.

## 4. Fixture before and after

`crates/qcore-c-gateway/tests/review_hold_standing.rs` (held-out `k_*` family,
9 tests). BEFORE is the base tree's behaviour of the same shapes (the base
tests below, which pinned it).

| Shape | Declarations in force | BEFORE (held) | AFTER (held) |
|---|---|---|---|
| `i = k_input_new((*ctx)->config, …)` (the observed shape) | `Unknown` (access refusal) | `Pending` | **`Unknown`**, standing reason names the refusal |
| the same with a formal actual | `Safe` | `Pending` | `Pending` |
| `k_close(h, 0); k_close(h, 0);` | `Unsafe` (double release) | `Pending` | **`Unknown`**, standing `double release candidate, not validated` |
| open and forget | `Unsafe` (leak) | `Pending` | **`Unknown`**, standing leak candidate |
| open, test, close | `Safe` | `Pending` | `Pending` |
| a refused store after the held call | `Unknown` | `Unknown` | `Unknown` (no standing reason: the held analysis records it) |
| an unrelated held declaration | as without it | byte-identical | byte-identical |

Each row is checked under both holds (review and pin mismatch), on the unit
route, the single-function entry and the Clang production route (unit
verdict).

**Corrections to existing pins** (each was an instance of the defect; none
is a `Safe` move):

- `contract_deferred_review.rs::a_deferred_contract_accuses_nothing` and
  `contract_pin_hold.rs::a_withheld_pin_accuses_nothing`: `TWICE` and `LEAKED`
  `Pending → Unknown` (the accepted declaration alleges the violation);
  `BALANCED` stays `Pending`; no violation is alleged in any.
- `sr5_soundness.rs::a_genuine_hold_is_still_pending`: its held call
  `k_close(0)` does not match the declaration in force, so `Pending →
  Unknown`; a formal actual is now the genuine hold. The unread-definition
  test uses the formal actual too (its gates are unchanged).
- `qcore-audit` `contract_revision_audit.rs`: the pinned mutation table's
  `dropped-release` and `changed-transfer` callers `Pending → Unknown` (the
  flexible rows of the same table are exactly the second analysis: `Unknown`
  there), `premature-destroy` stays `Pending` (flexible: `Safe`); the
  end-to-end test's `user_drop` `Pending → Unknown` with reasons
  `contract-pin-mismatch, unsupported`, and `compare`'s coverage-lost reasons
  follow. `contract_revision_identities.rs`: revision C drops the release,
  `user_drop` `Pending → Unknown`. The W45-CONTRACT-REVISION-MODE report's
  harness table is frozen evidence of its checkpoint and is not edited.

## 5. Corpus, system and slice

MEASUREMENT

## 6. The two in-flight branches

BRANCHES

## 7. Lean

`QCore.ReviewHoldStanding` (`proof/QCore/ReviewHoldStanding.lean`), audited
in `proof/AuditReviewHoldStanding.lean`: 22 theorems, no placeholders, no
project axioms (`propext` at most). The model: a row's held reasons and the
reasons of its analysis with every held declaration in force; `cap` (a
violation of the second analysis counts `Unknown`); `publish h r = if h =
Pending then join Pending (cap r) else h`; `standing` (what `settle` appends).

- `Row.pending_only_if_every_reason_is_a_hold`: a published `Pending` has
  every reason of both analyses `Safe` or `Pending`, and a held link for a
  human-resolvable reason (task: Pending only if every non-Safe reason is a
  hold).
- `Row.a_held_call_never_removes_another_reason`: on a row the hold touched,
  every reason of the second analysis, capped, is below the published verdict.
- `Row.never_below_the_verdict_without_the_rewrite`,
  `never_below_the_resolution`: the published verdict is at least the capped
  verdict without the rewrite; `publish_never_lowers`,
  `only_a_pending_row_moves`, `a_held_row_is_never_published_safe`,
  `the_second_analysis_never_publishes_unsafe`.
- `Row.the_settled_fold_is_published` (with `pending_join_standing`): the
  Rust fold over the held reasons plus the standing reasons is exactly
  `publish`.
- `an_unknown_hold_absorbs_every_refusal`: the other holds (§3).
- `Row.the_observed_shape_is_unknown`, `Row.an_alleged_violation_is_unknown`.

Premises stated, not proved: which reasons each Rust analysis records, and
that the second analysis is the held one with the holds lifted (pinned by the
fixture tests); a declaration another unit's body would refute once accepted
is not seen (`W45-REVIEW-HOLD-CROSS-UNIT-REFUTATION`).

## 8. Records and follow-ups

Gateway rule 10i amended and its router row; language spec §L21.2 and
contract spec C06.1/C06.2 sentences; STATUS verdict and follow-up rows;
[wave-47.md](../../development/increments/wave-47.md) paragraph.
[`scripts/update_ledger.py`](scripts/update_ledger.py) (idempotent) applies
[`followups.json`](followups.json): 551 → 553.

- Retires `W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL`, which
  W47-LOOP-OWNER-INSTANCE-FLOW registers on its branch; it is not registered
  on `main`, so the retirement is a no-op here and says so. **Integrator:**
  re-run the script after merging that branch.
- Registers `W45-REVIEW-HOLD-CROSS-UNIT-REFUTATION` (size 0 on the three
  routes at `main`) and `W45-HELD-UNIT-REASON-CODE-NAMES-HOLD-FIRST` (an
  `Unknown` unit with a held call keeps the human-resolvable unit
  `reason_code`, since `ReasonCategory::classify` names the held premise
  first; the verdict is unaffected; size 0 at `main`).

A census build (`indirect-census`, off by default) records the second
analysis's streams too for a unit with a `Pending` row; no such unit exists on
the routes today.

## 9. Gate

GATE
