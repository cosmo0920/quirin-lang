# W45-CONTRACT-DEFERRED-REVIEW: contracts whose evaluation is postponed to a human

Base `6bd9b383` (local `main`, "Record the fb-system-audit-cards integration
gate"). A resource-v1 contract, or one clause of it, can now be marked as
**awaiting human review**. The marked declaration is loaded, validated,
digested, packaged and listed, and then **withheld**: it discharges nothing and
accuses nothing, and every call site that would apply it is `Unknown` under the
new reason code `awaiting-human-review`. Nothing becomes `Safe` or `Unsafe`
because of it. A person resolves a review by editing the text (removing or
changing the marker), which changes the module's digest in the ordinary way; no
sign-off record exists. No production contract carries a marker, and the corpus
arbiter is verdict-identical to the archived baseline (§5).

The normative text is contract specification
[C06.1](../../QCORE_CONTRACT_SPEC_r0_3.md#c061-contracts-awaiting-human-review),
the grammar is [G05](../../QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05-resource-loader-v1),
and the reason category is a new row of language specification §L21.1.

## 1. Decisions

**Syntax.** Contract-level, a requirement beside the selector; clause-level,
a wrapper around one `ensures normal` clause, in place:

```qcore
extern fn k_close(h: ptr<mut, u8>, n: i32) -> unit
contract {
    requires resource_contract_v1;
    requires resource_review_pending("is the handle freed on the error path?", "owner");
    ensures normal resource_released(h, nonnull);
    ensures normal resource_borrowed(n);
    ensures normal resource_return_none();
};

    ensures normal resource_review_pending(resource_released(h, nonnull), "reason");
```

The reason is mandatory and the owner optional; each is one plain string
literal. The contract-level form is a `requires` clause for the reason the
literal-argument premise is one: it is a condition on *applicability*, not a
custody fact. The clause-level form wraps the clause rather than naming a
position, so the reviewer reads exactly which clause is in question and removing
the marker is unwrapping it (the tests assert that unwrapping reproduces the
bare text and environment byte for byte).

**Status vocabulary.** C06's `ContractTrust` gains a fourth status,
`PendingHumanReview`, instead of reusing `Proposed`. A proposal awaits the
*core's* validation and can be promoted by it, and C06 bars it only from
justifying `Safe`; a pending contract is resolved only by a person editing the
text and must justify neither `Safe` nor `Unsafe`. Reusing `Proposed` would
have changed what that status means.

**Withheld, not flagged.** The loader keeps a marked declaration out of the
function map (`ResourceContracts::functions`) and records it under
`pending_review`, with its module, the contract it states and every marker. A
consumer that knows nothing of the status therefore sees an uncontracted callee,
which is the refusing direction by default; the consumers below only attribute
that refusal to the review.

**A clause-level marker holds the whole contract.** A v1 contract is applied at
a call site as one conjunction, and no consumer applies part of one. Dropping
only the deferred clause would let the remaining, possibly weaker, clauses
decide (a deferred `resource_released(p)` would read as a call taking no custody
of `p`). So a clause-level marker holds exactly the call sites a contract-level
one does; the clause is what the report names. A position-precise hold is
registered as `W45-REVIEW-POSITION-PRECISE-HOLD`.

**Reason code.** `awaiting-human-review` is a new member of the closed §L21.1
vocabulary (`ReasonCategory::AwaitingHumanReview`, subject: the analyser, never
supporting `Unsafe`). A held call is counted apart from unsupported operations
in the gateway's unit summary, which names held calls only when there is one
(`…, N call(s) held awaiting human review`), so every existing summary is
spelled and classified as before. A summary naming a held call classifies as
`awaiting-human-review` ahead of every other category: whatever the projection
leaves open downstream of a withheld contract is not a statement about the
source, so no held row reaches a source-subject category.

**Schema version.** The two new variants break exhaustive matches, so
`qcore-schemas` moves 14.0.0 → **15.0.0** by the crate's own exhaustive-variant
precedent ([compatibility note](../../../crates/qcore-schemas/README.md#contracts-awaiting-human-review-1500)).
No wire byte moves for data without a marker. Constraint 18's rebuild and
verification of all five packages was run (§6).

## 2. Loader and consumers

| Place | What changed |
|---|---|
| `crates/qcore-schemas/src/contract.rs` | `ContractTrust::PendingHumanReview`; `PendingHumanReview`, `ReviewDeferral`, `ReviewScope`; `ResourceContracts::pending_review` (skipped when empty) |
| `crates/qcore-schemas/src/reason.rs` | `ReasonCategory::AwaitingHumanReview`; the projection summary's held component; the `awaits human review` rule, ahead of every source rule |
| `crates/qcore-contract/src/contract_loader.rs` | `review_marker` parses both forms total-or-reject; a marked declaration is validated in full, then withheld; `check_source_assumptions` recomputes the pending map; inherited declarations must carry their parent's markers |
| `crates/qcore-contract/src/extern_bridge.rs` | `pending_review_refusal`: the checked extern-call route's reason |
| `crates/qcore-verification/src/driver.rs` | the route's existing `Q023` refusal names the review (four lines) |
| `crates/qcore-c-gateway/src/legacy_review_hold.rs` | new: every call site of a pending callee is rewritten to a marked undeclared name before any reader sees it (the LOADER-ABI technique), so W4 derivation, W4 validation and certificate rechecks reproduce the hold |
| `crates/qcore-c-gateway/src/legacy_c.rs` | the three entry hooks, the held-call refusal at both statement positions, and the held count in the unit summary |
| `crates/qcore-audit/src/pending_review.rs` | new: the `pending_human_review` section and console tally |
| `current.rs`, `no_build.rs`, `compiler/examples/support/cycle19_compatibility.rs` | publish the section in the contract sidecar and print the tally |

A function *defined* in the unit under a pending name is analysed as an
undeclared definition: the pending contract is neither its checked interface
nor its accuser, and its callers stay held whatever its body or a W4 summary
of it proves (`a_defined_callee_does_not_lift_the_hold_from_its_callers`).
Provider packages carry the text as bytes (`providers/package.py` pins bytes,
not clauses) and every provider selection loads through this loader, so the
provider paths need no code of their own; `contracts.py check`'s resource-view
probe accepts a marked module as the loader does. The observation interpreter
does not recognise the marker and refuses it.

## 3. Tests

* `crates/qcore-contract/tests/contract_deferred_review.rs` (13): round trip of
  both forms and the serialized environment; several clause markers; removing
  the marker restores the environment; an unmarked environment serializes as
  before; refusals for a missing or blank reason, a malformed reason or owner,
  duplicate markers (two contract-level, contract beside clause in either
  order, a clause deferred twice) and markers on unsupported items (the
  selector, a literal premise, the wrong introducer, a nested marker, an
  unknown predicate, an error exit); inheritance of a whole deferral and of a
  clause deferral (renamed by position); the extern-call refusal; the
  observation interpreter's refusal.
* `crates/qcore-c-gateway/tests/contract_deferred_review.rs` (7): the same
  function `Safe` with the contract, `Unknown` held with the whole contract
  deferred, `Unknown` held with only the load-bearing clause deferred; a
  non-load-bearing deferred clause still holds (never dropped); a deferred
  release turns the double release and the would-be leak into `Unknown` and a
  deferred acquisition the leak, with no violation alleged; removing the marker
  restores the verdicts byte for byte; a defined callee does not lift the hold;
  a child inheriting a deferred clause is held at its own call sites; with
  Clang, the unit reason and its `awaiting-human-review` code, and a
  certificate whose recheck reproduces the pending map (a forged empty map is
  refused).
* `compiler/tests/extern_call_deferred_review.rs` (2): on
  `source.extern-call.v1`, both marker forms refuse the round trip with `Q023`
  naming the review; the unmarked control is admitted.
* `crates/qcore-audit/src/pending_review.rs` (2 unit) and
  `crates/qcore-audit/tests/pending_review_report.rs` (2): the section's
  entries, digests and counts, the unit `reason_code`, explicit zeros, and the
  `quirin-audit contracts` route end to end.
* `crates/qcore-schemas/tests/reason_codes.rs`: four new corpus rows and two
  refused spellings (a malformed or zero held count).
* `scripts/tests/test_baseline_identity_additive.py` (7): the identity tool's
  `--additive-field` disclosure ignores exactly the named key.

## 4. Lean

`proof/QCore/ContractDeferredReview.lean`, audited by
`proof/AuditContractDeferredReview.lean` (17 theorems):
`a_deferred_clause_withholds_the_declaration`, `applied_is_all_or_nothing`,
`a_pending_declaration_contributes_nothing`,
`a_verdict_resting_on_a_deferred_clause_is_unknown` and its corollary
`a_deferred_dependency_is_neither_safe_nor_unsafe`,
`the_review_reason_never_supports_unsafe`; monotonicity
`resolving_only_adds_contributions` and
`resolving_reviews_keeps_unrelated_safe` (with the three `Declaration` lemmas);
inheritance `inheritance_propagates_pending`,
`inheritance_propagates_each_clause`,
`an_inherited_deferral_withholds_the_child`,
`inheritance_chains_propagate_pending`; the unit reason
`a_held_summary_is_never_a_source_statement` and
`a_summary_without_held_calls_is_unchanged`. `QCore.ReasonCategory` gains the
eighth category and its families, and every existing theorem re-proves over it
(`exactly_two_categories_describe_the_analysed_source` is unchanged). The
clause contents are abstract premises; that the token rewrite reaches every
call site is the Rust boundary the gateway tests pin.

## 5. Corpus identity

No production contract carries a marker, so the prediction is no verdict
movement and one additive sidecar key. It was stated by the brief and is
recorded here after the fact; no separate prediction was committed before
implementation.

The arbiter (`docs/reports/w43-return-expression/scripts/run_audit.py`,
`--jobs 8`) ran at `3071477f` into a fresh private directory
([run](arbiter-run.json); an earlier run at `ff12f687`, before the schema
version bump and the documentation, gave the same five results), and
`baseline_identity.py … --root-alias <this worktree>=<hungry-thompson>
--additive-field contracts:pending_human_review` compared it with
`docs/reports/w45-sub-object-stores/integration/artifacts/`
([result](arbiter-identity.json), exit 0):

| Artifact | Result |
|---|---|
| diagnostics (verdict-bearing) | byte-identical |
| chunk-contracts, source | byte-identical |
| contracts | identical ignoring exactly `pending_human_review`, as JSON and byte for byte in the writer's pretty form |
| source-effects | identical after masking digests: 10 distinct build/configuration digests bind the checkout path, as every W45 card's identity record discloses ([script](scripts/source_effects_digests.py), [result](source-effects-digests.json)) |

The arbiter's console reads `pending human review: 0 declaration(s), 0
marker(s); 0 function(s) and 0 call site(s) held Unknown
(awaiting-human-review)`. The identity tool gained `--additive-field
ARTIFACT:KEY`, a disclosure beside the raw comparison that cannot name the
verdict-bearing artifact and does not change the exit rule.

## 6. Gate

Measured at `3071477f` (the Rust suite at `5beff34f`, which differs from it in
this report, the increment paragraph and the report index only). Both were
work-in-progress commits of this card's branch, squashed into the card's
single commit afterwards, so neither is reachable from it; the squashed tree
differs from `3071477f` in this report's §5 and §6 text alone. Logs under
the worktree-private `private/dr-gate-1/`; every exit captured to its own
file, nothing piped.

| Step | Command | Exit | Count |
|---|---|---:|---|
| submodules | `git submodule update --init --recursive` | 0 | |
| clang | `clang --version` | 0 | Ubuntu clang 21.1.8 ([recording](clang-version.txt)) |
| test | `CARGO_INCREMENTAL=0 python3 scripts/run_test_binaries.py` | 0 | **5,746 passed / 0 failed / 11 ignored**, 306 summaries over 296 units (`private/test-binaries/20260925T175603Z-569054/`) |
| clean | `cargo clean --profile dev` | 0 | |
| fmt | `cargo fmt --all -- --check` | 0 | |
| clippy | `cargo clippy --all-targets -- -D warnings` | 0 | |
| probe / followup | `cargo run -p qcore-compiler --example reevaluation_probe` / `reevaluation_followup` | 0, 0 | |
| lean | `python3 scripts/check_lean_audits.py` | 0 | 194 audits (+1, `AuditContractDeferredReview.lean`) |
| python | `python3 -m unittest discover -s scripts/tests` | 0 | 358 tests, after the matrix regeneration (`build_matrix.py .`, exit 0; `named_followups` 405 → 410; gate figures carried, not supplied) |
| docs index | `python3 scripts/build_docs_index.py --check` | 0 | |
| contracts | `python3 providers/contracts.py check` | 0 | resource views unchanged |
| provider Python | `python3 -m unittest discover -s providers/tests` | 0 | 15 tests |
| packages (constraint 18) | `python3 providers/package.py build --output private/dr-gate-1/packages`; `verify` × 5 | 0; 0 × 5 | |
| provider test | `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline --no-fail-fast` | 0 | 124 passed / 0 failed / 0 ignored, 39 summaries |
| provider fmt / clippy | `cargo fmt --manifest-path providers/Cargo.toml --all -- --check`; `cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings` | 0, 0 | |
| arbiter | `run_audit.py private/dr-arbiter-2 3071477f --jobs 8`; `baseline_identity.py` as §5 | 0; 0 | diagnostics byte-identical |

Constraint 16's load-sensitive rows, and
`real_packages_preserve_unknown_and_reject_mutations` in the provider suite,
all passed in these full runs; no isolated re-verification was needed and
none is claimed.

## 7. Follow-ups

Registered in the acceptance matrix's `named_followups` (405 → 410) from
[followups.json](followups.json) by [scripts/update_ledger.py](scripts/update_ledger.py):
`W45-REVIEW-POSITION-PRECISE-HOLD`, `W45-REVIEW-HELD-CALL-SITE-COUNT` (the
section counts recorded held-call refusals, a lower bound on textual call
sites), `W45-REVIEW-CONFIG-ROUTE-SECTION` (the configured `quirin-audit
CONFIG.json` route does not publish the section yet),
`W45-REVIEW-HELD-DIAGNOSTIC-CATEGORY` (a held call is listed under the
diagnostic category `unsupported`, as a withdrawn declaration is) and
`W45-REVIEW-INHERIT-OWN-MARKER` (an inheriting declaration cannot carry its own
marker). Each names its discharge condition.
