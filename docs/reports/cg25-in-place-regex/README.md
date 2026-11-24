# CG25-IN-PLACE-REGEX

Discharges `CG25-IN-PLACE-REGEX-USE-CERTIFIED` and
`CG25-IN-PLACE-REGEX-SHADOWED-NAME`, both registered by
CG25-THREAD-KEY-AUTOMATIC-REVISIT (probes R2, R3, R4 of
[its report](../cg25-thread-key-automatic-revisit/README.md)). USER DECISION
2026-11-23: the "soundness first" set of four implementation cards, with
both regex follow-ups in this one card. Base `2c60986b0` (main). Gateway
[rule 74i](../../agents/gateway.md#constraint-74i).

## 1. Direction assessment

Both follow-ups are fail-open: a wrong `Safe`.

* **Use before certification (R3, R4).** CG25's in-place use
  (`QCore.ResourceKinds.transferI (.use …)`, route G's
  `report_kind_violations`) reads only the released bit. Rule 11v (7)'s
  `report_uncertified_key_uses` asked for certification of `thread_key`
  uses only. It filtered on `finalised_by_value`.
* **Shadowed name (R2).** `in_place_actuals` asked `one_automatic_object`
  of a `regex` actual, which counts only the function's own `VarDecl`s.
  It asked `every_reference_is` only of a key.

**Unknown, not Unsafe.** The refusing direction for an uncertified use is
`Unknown` (an `Unsupported` refusal), as rule 11v (7) gives a key. An
`Unsafe` must be a validated reachable violation. A use before the test or
on the failure edge is a violation only on a run where `regcomp` failed, and
nothing here validates that it can (a constant, valid pattern never fails).
`regerror` on the failure edge is also defined behaviour, so an accusation
there would be wrong, not just unvalidated. A use after `regfree` keeps 11v's
`use_after_close` candidate, unchanged.

**Every in-place kind.** The certified-use check now reads every kinded
borrowed position of every in-place kind (today `regex` and `thread_key`),
by address as well as by value. The one-declaration check now applies to
every in-place actual. A key keeps its exact refusal texts, so 11v (7)'s
pins are unchanged.

**The registered premise was wrong about `rd_regex_match`.** Both
follow-ups said that `rd_regex_match`, the only tracked `regex` object on
any route, "uses `re` only after the zero edge". It does not. Under
`HAVE_REGEX` it calls `regerror(r, &re, errstr, errstr_size)` on the
failure edge (`lib/librdkafka-2.15.0/src/rdregex.c:124-128`). `regerror`'s
`preg` is kinded `regex` and borrowed, as `regexec`'s is (`regex.qv`).
The "verbatim" fixture in `tests/resource_kinds.rs` had replaced that call
with `k_wipe(errstr, …)`. POSIX defines `regerror` on the `regex_t` of the
failed `regcomp`, and leaves `regexec` on it undefined. No contract clause
tells the two apart, providers are out of scope, and a rule that names a
callee would be a hardcoded route. So the check refuses both, and
`rd_regex_match` moves **Safe -> Unknown**. That is the fail-closed direction
and an over-refusal, recorded as
`CG25-IN-PLACE-REGEX-REGERROR-FAILURE-EDGE` (§5). The prediction stated
this before the implementation ([PREDICTION.md](PREDICTION.md)).

## 2. Changes

* `crates/qcore-c-gateway/src/legacy_in_place_custody.rs`:
  * `report_uncertified_key_uses` reads every in-place kind's use and
    initialiser positions (the `finalised_by_value` filter is gone).
    `uncertified_use_refusal` keeps 11v (7)'s text for a key. Every other
    kind gets "use of in-place object `NAME` not certified by its
    initialiser status (rule 74i)". The name is kept, so the call site in
    the shared `legacy_transfer.rs` is untouched.
  * `Parser::automatic_object` (`one_automatic_object` and
    `every_reference_is`) gates every by-address in-place actual.
    `automatic_key` is that check plus the `thread_key` kind.
* `crates/qcore-c-gateway/tests/in_place_regex_certified.rs`: new, through
  the production `regex.qv`.
* `proof/QCore/InPlaceCertifiedUse.lean`, `proof/AuditInPlaceCertifiedUse.lean`:
  new. `proof/QCore.lean` was regenerated.
* `docs/agents/rules/74i.md`: new. `docs/agents/gateway.md` and
  `docs/agents/README.md` were regenerated.

## 3. Pins

`tests/in_place_regex_certified.rs`, run at base
(`private/card-logs/pins-at-base.log`: 4 passed, 5 failed; every failing
assertion read `Safe` at base) and at tip (9 passed):

| Test | Shapes | Base | Tip |
|---|---|---|---|
| `the_rd_regex_match_shape_stays_safe` | `resource_kinds.rs`'s `rd_regex_match` shape, the `r == 0` form, two uses after the zero edge | Safe | Safe (control) |
| `a_use_before_the_status_test_is_refused` | probe R4 | Safe | Unknown (74i) |
| `a_use_on_the_failure_edge_is_refused` | probe R3 | Safe | Unknown (74i) |
| `a_use_before_the_initialiser_is_refused` | `regexec` before `regcomp` | Safe | Unknown (74i) |
| `a_regerror_on_the_failure_edge_is_refused` | `regerror` on the failure edge; `rd_regex_match` as rdregex.c spells it | Safe | Unknown (74i, over-refusal) |
| `a_use_after_regfree_stays_a_candidate` | `regexec` after `regfree` | Unsafe | Unsafe, `use_after_close`, no 74i refusal |
| `an_untracked_use_keeps_the_borrowed_reading` | `regexec(re, …)`, `re` a formal | Safe | Safe |
| `a_shadowing_local_beside_a_file_scope_reference_is_refused` | probe R2; a file-scope reference before the shadowing block | Safe | Unknown (11v (2) text) |
| `a_shadowing_local_alone_is_the_one_object` | shadowing local, every reference its own: freed, never freed | Safe, Unsafe `leak` | the same |

`tests/resource_kinds.rs`, `tests/thread_key_in_place.rs`,
`tests/in_place_mention_guard.rs` and `tests/resource_kinds_lean.rs` pass
unchanged (`private/card-logs/pins-tip-1.log`). No existing pin moved.

**Lean**, `QCore.InPlaceCertifiedUse`. `transferC` is CG25's `transferI`
with every `.use` read as rule 11v (7)'s `useK`:

* `the_key_use_is_rule_11v_7` and `every_other_step_is_cg25` show that
  only uses change.
* `each_step_agrees_or_refuses`: each step is CG25's, or CG25's plus one
  refusal. From it follow `a_run_without_refusal_is_cg25` and
  **`no_verdict_becomes_safe`** (a verdict that was not `Safe` under CG25
  is not `Safe` now).
* `a_safe_verdict_used_only_certified_objects`.
* `oneDeclaration`, `one_declaration_iff`, `initChecked`,
  `one_declaration_is_cg25`, `a_shadowed_name_is_refused`.
* Witnesses: probes R2, R3, R4 and the use before the initialiser are each
  `Safe` under `runI` and not `Safe` under rule 74i; the control's two
  paths stay `Safe`; a use after `regfree` stays a candidate.

The audit's `#print axioms` lines list only `propext` and `Quot.sound`.

## 4. Measurement against the prediction

The arbiter is opt-in `current --monkey-linked` over Fluent Bit 046accf6.
Each side ran from its own `git archive` tree with its own release binary
([runs](evidence/runs.md)):

| | Safe / Pending / Unknown / Unsafe | moved rows |
|---|---|---|
| base `2c60986b0` | 1,172 / 2 / 3,622 / 8 | |
| tip `a7e8bea5b` | 1,171 / 2 / 3,623 / 8 | `rdregex.c:rd_regex_match` Safe -> Unknown |

**The measurement matches the prediction exactly.** One row moved, the one
predicted, with one added leaf: the rule 74i refusal at its `regerror`.
No other row changed status or leaves, no row became `Safe`, and no new
`Unsafe` appeared. Access-gate rows and leaves are unchanged (2,118 / 215 /
8,992).

**Pinned routes.** `quirin-audit measure --declared` cannot run on this
host, which has no `build/cycle19` inputs and no baselines, so it was not
run and no result is claimed. The prediction block expects `rd_regex_match`
Safe -> Unknown on the corpus and no move on system or slice. The
integrator measures with base `rev:2c60986b0` against
[PREDICTION.md](PREDICTION.md).

## 5. Follow-ups

* `CG25-IN-PLACE-REGEX-REGERROR-FAILURE-EDGE` (registered): restore
  `rd_regex_match` soundly with a contract clause that marks a use defined
  on a failed object (`regerror`'s `preg`), so that rule 74i certifies
  only the uses that need a compiled object. This is a provider and
  contract-language change.
* `FRAME-HOLDER-SHADOWED-NAME` (registered, unassessed): rule 12t's frame
  holder (`legacy_retained_by.rs`) and the callback registry's
  `frame_registry_holder` (`legacy_callback_route.rs`) ask
  `one_automatic_object` without `every_reference_is`. So a local holder
  that shadows a file-scope object of the same name may be read as one
  frame object. The direction was not assessed.

## 6. Files likely to conflict

* `docs/agents/gateway.md`, `docs/agents/README.md`, `proof/QCore.lean`:
  generated. Rerun `gen_rule_docs.py` and `gen_lean_roots.py`.
* `crates/qcore-c-gateway/src/legacy_in_place_custody.rs`: only this card
  is known to edit it.
* `docs/reviews/r0_3/acceptance_matrix.json` and the increment record are
  written only by `apply_amendments.py`.

## 7. Gate

`sh private/toolchain/env.sh python3 scripts/gate_scope.py --base
2c60986b0 --out private/gate-1` ran at `a7e8bea5b`, the implementation
commit, and exited 0. The record is [gate.json](gate.json). The scope was
full, because the diff reaches `qcore-c-gateway`. All Lean was checked
because `proof/QCore.lean` changed. No provider gate ran, because
`providers/` is unchanged.

| Step | Exit | Counts |
|---|---|---|
| submodules, fmt, clippy | 0, 0, 0 | |
| Rust suite | 0 | 681 units, 726 summaries; 9,980 passed, 0 failed, 22 ignored, 148 filtered |
| reevaluation probe, follow-up | 0, 0 | |
| Lean (`check_lean_audits.py`) | 0 | 492 audits |
| Python suite | 0 | 807 tests, 0 skipped |

Logs are in `private/gate-1/` (per-unit Rust logs under `rust-suite/`). The
later commits add only this directory's records. After them,
`gen_rule_docs.py --check`, `apply_amendments.py --check` and the Python
suite were rerun (§8).

## 8. Record checks after the gate

Run at the records commit:

* `python3 scripts/apply_amendments.py docs/reports/cg25-in-place-regex/integration-amendments.json --check`: 0.
* `python3 scripts/gen_rule_docs.py --check`: 0.
* `QUIRIN_CARD_GATE_BASE=2c60986b0 python3 -m unittest discover -s
  scripts/tests` (the environment variable `gate_scope.py` sets): 0, with
  807 tests and 1 skipped (`private/card-logs/python-recheck-2.log`).
  Without the variable the run exits 1 on one test,
  `test_the_derived_facts_are_recomputed_rather_than_carried`. That test
  compares the committed matrix's 491 Lean audits with the tree's 492, the
  new `AuditInPlaceCertifiedUse.lean` making the difference. It is the
  landing check, and the integrator regenerates the matrix at landing
  (`private/card-logs/python-recheck.log`).
