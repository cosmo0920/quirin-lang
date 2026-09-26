# W45-USER-CONTRACT-PLUGINS: user-defined contracts and checker plugins, with a four-value verdict

Base `84ebad57`, fast-forwarded before any change to local `main` at
`2f9131a8` (W45-CONTRACT-REVISION-MODE integrated). Users can now keep
resource contracts in their own crates, outside this repository, and attach
out-of-process **checker plugins** to them; the SDK is the new crate
[`providers/quirin-provider-plugin`](../../../providers/quirin-provider-plugin/README.md).
Every verdict is one of four values, **`Safe < Pending < Unknown < Unsafe`**,
and every roll-up is their join. `Pending` was proposed and approved by the
user on 2026-09-26 as an amendment to hard invariant 2. The corpus arbiter is
verdict-identical to the current baseline, with 0 `Pending` units and
functions (§6).

The normative text is language specification
[§L21.2](../../QCORE_SPEC_r0_3.md#l212-the-four-value-verdict-and-its-composition)
(with §L20 and §L21.1) and contract specification
[C11.1](../../QCORE_CONTRACT_SPEC_r0_3.md#c111-user-defined-contracts-and-checker-plugins)
(with C06.1, C06.2 and C13).

## 1. The verdict algebra

| Value | Meaning |
|---|---|
| `Safe` | every obligation of the scoped claim discharged |
| `Pending` | not decided by the tool, awaiting a **named human act**: a review edit (`awaiting-human-review`), a re-pin after review (`contract-pin-mismatch`) or a checkable certificate for a plugin's claim (`plugin-claim-uncertified`) |
| `Unknown` | not decidable by the tool as configured |
| `Unsafe` | a validated reachable violation, provisional under the existing policy, never an upstream defect claim |

Composition is the **join** (the maximum): associative, commutative and
idempotent, with `Safe` as identity. A verdict carries a multiset of reasons;
composition joins the values and unions the reasons (associative, `Safe` with
no reason as identity, commutative up to the order of reasons). `Pending` is
fail-closed: never `Safe`, always in the denominator, never lowered by
composition. It is constructible only from a `PendingReason`, the closed set
of the three human-resolvable categories (`ReasonCategory::is_human_resolvable`).
The algebra is `qcore_schemas::verdict` (`Verdict`, `ReasonedVerdict`,
`PendingReason`, `Chain`); `ReasonedVerdict` and `Chain` serialize but do not
deserialize, so no unchecked constructor exists.

**Chains.** A chain is an ordered composition of contract steps — a callee
chain, an obligation pipeline or a unit group (`ChainKind`) — and its verdict
is the fold. The fold does not depend on the order.

## 2. The status mapping

| Status before | After | Why |
|---|---|---|
| `awaiting-human-review` held call (C06.1) | **`Pending`** | a person resolves it by editing the text |
| `contract-pin-mismatch` held call (C06.2) | **`Pending`** | a person resolves it by reviewing the changed source (or contract) and re-pinning with `quirin-audit pin`; the tool cannot decide the unreviewed identity, and nothing but that review stands in the way |
| every other `Unknown` | `Unknown` | unchanged |

Concretely, in the gateway (`crates/qcore-c-gateway/src/legacy_verdict.rs`) a
function's status is the fold of its diagnostics' verdicts — a violation
`Unsafe`, an unresolved construct `Unknown`, a held call `Pending` — so a
function whose only unresolved diagnostics are held calls is `Pending`, and one
more unresolved construct makes it `Unknown` exactly as before. A unit is the
fold of its functions, joined with `Unknown` for a projection failure;
`UnrestrictedLegacyResult::verdict` reads a `Pending` unit back from the
summary it carries (the enum's `Unknown` variant carries every non-Safe,
non-Unsafe result, so no match moved). The reason codes are unchanged: a held
unit keeps `awaiting-human-review` or `contract-pin-mismatch`.

**The fold is the old roll-up.** The three-way decision this replaced
(`Unsafe` if any, else `Unknown` if any, else `Safe`) is the join of
`Safe < Unknown < Unsafe`. Read through the projection that sends `Pending` to
`Unknown`, the four-valued fold equals it on every input
(`fold_refines_the_historical_rollup`), and on inputs without a held call it
is the same value (`fold_is_the_historical_rollup_without_pending`). The Rust
test `verdict_fold.rs` checks all 128 subsets of seven representative
diagnostic kinds against a verbatim copy of the old decision, and seven
multi-function units against the fold. No roll-up disagreed, so no stop was
needed.

## 3. The invariant amendment

`AGENTS.md` hard invariant 2 now reads (the sentence on `Safe` kept verbatim):

> `Safe` only after all obligations discharge; `Unsafe` only for a validated
> reachable violation; `Pending` (non-Safe, in the denominator) only for a
> named human-resolvable reason; everything else `Unknown`. Composition is the
> join of `Safe < Pending < Unknown < Unsafe` (§L21.2).

Language spec §L20 defines `Pending` beside the other three, §L21.1 gains the
tenth category `plugin-claim-uncertified` and the human-resolvable rule, and
§L21.2 is new. Contract spec C06.1 and C06.2 say a verdict resting only on held
calls is `Pending`, C11.1 is new, and C13 defines `Pending` in the claim tuple.
`STATUS.md` gains the vocabulary row.

## 4. User contracts and the SDK

A user crate is laid out like `quirin-provider-*` (name `quirin-provider-NAME`,
`NAMESPACE`/`PUBLISHED`, `contracts/`, its own `contracts.lock`) and listed in
a **user load path** whose `extends` names the shipped
`providers/contract-load-path.json`
(`crates/qcore-contract/src/load_path_manifest.rs`). The extended manifest is
read in full, may not itself extend, and the two crate sets are validated
together, so a user crate can depend on a shipped crate and never take its name
or namespace. On a user load path every selected module is lowered as a
`crate:module` reference, `quirin-audit pin` writes only the user's locks, and
`quirin-audit plugin views --write` computes the user crates' resource views
with the loader.

```sh
quirin-plugin new work/quirin-provider-acme --name quirin-provider-acme
quirin-plugin load-path --extends /opt/quirin/providers/contract-load-path.json \
    --output work/user-load-path.json work/quirin-provider-acme
quirin-audit plugin views --load-path work/user-load-path.json --write
quirin-audit pin --load-path work/user-load-path.json --source-root SOURCE
quirin-audit contracts --load-path work/user-load-path.json \
    --provider quirin-provider-acme --source-root SOURCE --report out/r.json
```

**Collisions.** A selected user module declaring a function a shipped crate
declares is refused — by the audit and by `plugin views` — unless the user load
path carries an override naming the function, the user crate, the shipped crate
and a reason; each applied override is recorded in the selection's
`user_load_path` record (`user_plugins::check_overrides`). Names are read
lexically, which can only over-report.

**The SDK** (`providers/quirin-provider-plugin`, binary `quirin-plugin`): the
crate reader and `quirin-plugin.json` schema, both total-or-refuse; the user
load-path generator; the template (its one contract held awaiting review, so a
new crate makes nothing `Safe`); checker packaging and verification (the Rust
counterpart of `providers/package.py build`/`verify` for checker packages); and
the checker runtime over the shared provider transport. It carries no
production contracts and links no core crate.

**One deviation from the brief.** The brief asked for the crate to be
registered in `providers/contract-load-path.json`. A load-path record is a
contract crate: `providers/contracts.py` and the Rust reader require a
`contracts/` tree with at least one owned module, and an empty directory cannot
be committed. Carrying a module would ship a contract and move the pinned view
and corpus counts. The crate is therefore a workspace member listed, with the
conformance harness, in `NON_CONTRACT_PACKAGES`; the load path keeps 11 crates
and 49 view modules, and `test_contract_load_path.py` pins both that the SDK is
absent and the count of 11. The conformance dependency pin admits it with the
shared transport as its only first-party dependency (`sdk == 1`).

## 5. Checker plugins and the firewall

A binding (`checkers`: crate, module, sorted functions, package directory)
runs once per audit, before any unit, as a verified process package through
`VerifierRegistry` (`crates/qcore-audit/src/user_plugins.rs`), never
in-process. The request is protocol v1 framing with evidence v2 over a
core-derived task: one `parameter_mode_respected` obligation per bound
function, bound to the module's text digest (`quirin.user-contract-check.v1`).
The host's firewall (`crates/qcore-verifier-host/src/plugin_firewall.rs`,
`VerifierRegistry::verify_plugin_claim`) classifies the answer; `verify_v2` was
split into its gates and dispatch (`dispatch_v2`) and its unchanged firewall,
so the classifier reads the raw proposal without a second code path:

| The plugin | Verdict |
|---|---|
| `Safe`, and the registry issued a core certificate (trusted-core witness replay against the core's inventory) | `Safe` |
| `Safe` with no evidence (`QP008`) or evidence with no core checker (`QP016`) | `Pending`, `plugin-claim-uncertified` |
| `Unsafe`, and a trusted-core counterexample checker validated its witness for obligations of the core's inventory | provisional `Unsafe` |
| `Unsafe` with no witness or no counterexample checker (`QP010`) | `Pending`, `plugin-claim-uncertified` |
| `Unknown` | `Unknown` (`incomplete-obligation-coverage`) |
| a gate (`QP001`–`QP005`), spawn failure, crash, timeout, quota or output breach, protocol error (`QP006`), invalid envelope (`QP007`), a package that does not verify, or evidence that does not bind or that the checker refutes (`QP009`) | `Unknown` (`provider-failure`) |

`Pending` is exactly the unchecked claim — nothing checkable, or nothing to
check it with — so a person can resolve it by supplying a certificate or
witness. A claim that was checked and refuted, or does not bind, is not
resolved by a person and stays `Unknown`, the higher (more conservative) value.
The command line registers no trusted checker for user contracts, so every
plugin `Safe` and `Unsafe` it sees is `Pending` today
(`W45-PLUGIN-CERTIFICATE-CHECKER`, `W45-PLUGIN-COUNTEREXAMPLE-CHECKER`).

**The callee chain.** A bound function's verdict is joined into every function
whose control-flow graph applies its contract (a `ContractCall`, or an admitted
nested call), and each unit into its functions' (`user_plugins::fold_unit`,
hooked into `AuditReport::record`; the unit tallies and console lines follow the
published row). Without a binding nothing runs and every report is
byte-identical. The contract sidecar carries a `plugin_checks` section and the
console a tally.

## 6. Identity proof

On the 185-unit corpus there is no deferred contract, every pin is in force and
no checker is bound, so the prediction is **verdict-identical**, with a Pending
column of 0.

The arbiter (`docs/reports/w43-return-expression/scripts/run_audit.py`,
`--jobs 8`) ran at `62978cf3` into a fresh private directory
([run](identity/arbiter-run.json), exit 0). Later commits change formatting,
documentation and one test-only clippy fix, no analysis code.
[`scripts/identity.sh`](scripts/identity.sh) compared it with
`docs/reports/w45-fb-retry-lifetime/integration/artifacts/` (root aliased to
the baseline's worktree, W45-CONTRACT-REVISION-MODE's additive key named); all
three steps exited 0:

| Artifact | Result |
|---|---|
| diagnostics (verdict-bearing) | **byte-identical** ([result](identity/baseline-identity.json)) |
| chunk-contracts, source | byte-identical |
| contracts | identical after undoing W45-CONTRACT-REVISION-MODE's changes (its additive `contract_revision` key and libc texts, by its own restoration) and this card's one change, the `pending_human_review` section's `meaning` string, which now says a function resting only on held calls is `Pending`; no `plugin_checks` key is written ([script](scripts/contracts_difference.py), [result](identity/baseline-identity-contracts.json)) |
| source-effects | identical after masking digests, the checkout-bound digests every W45 card discloses ([result](identity/baseline-identity-source-effects.json)) |

[Verdict tallies](identity/verdict-tallies.json) ([script](scripts/verdict_tallies.py)):
units **4 Safe / 0 Pending / 10 provisional Unsafe / 167 Unknown / 4
data-only**, functions **389 / 0 / 12 / 4,433**. The console's hold tallies
read `… held Pending (awaiting-human-review)` and `… held Pending
(contract-pin-mismatch)`, both with zero counts ([tallies](identity/arbiter-tallies.txt)).
The diagnostics report gained no key: `status` simply admits the value
`Pending`, which no corpus row takes.

## 7. Tests

| File | Tests | What |
|---|---:|---|
| `crates/qcore-schemas/src/verdict.rs` | 7 | order; monoid laws; `Pending` never folds to `Safe`; reason multisets; `Pending` only from a `PendingReason`; chains; wire spelling |
| `crates/qcore-schemas/tests/reason_codes.rs` | (corpus) | two `plugin-claim-uncertified` rows; every category still reached |
| `crates/qcore-c-gateway/tests/verdict_fold.rs` | 3 | 128 diagnostic subsets against the old decision; monotonicity; seven units against the fold |
| `crates/qcore-c-gateway/tests/contract_deferred_review.rs`, `contract_pin_hold.rs` | 8, 5 | the held functions and units now `Pending` (updated from `Unknown`) |
| `crates/qcore-verifier-host/tests/plugin_firewall.rs` | 6 | every firewall row through the real registry, and agreement with `verify_v2` on `Safe` |
| `crates/qcore-audit/tests/user_contract_plugins.rs` | 7 | end to end: no checker; uncertified `Safe` (QP008 and QP016) is `Pending` and the chain folds; uncertified `Unsafe` is `Pending`; timeout, crash and an unverifiable package are `Unknown`; certified `Safe` accepted (library route); a collision refused, overridden and reported, and a shipped crate name refused; a user lock mismatch is `Pending` and `compare` classes it |
| `crates/qcore-audit/tests/contract_revision_audit.rs` | 3 | the mutation table's held callers now `Pending` |
| `providers/quirin-provider-plugin` | 5 | namespaces; the example crate and its load path; malformed crates refused; the template; packaging, verification and a live answer of the example checker |

## 8. Lean

`proof/QCore/VerdictSemigroup.lean`, audited by
`proof/AuditVerdictSemigroup.lean` (34 theorems): the algebra (`join_assoc`,
`join_comm`, `join_idem`, `safe_join`, `join_safe`, `fold_append`,
`fold_append_comm`, and the `Reasoned` laws); monotonicity
(`adding_a_link_never_lowers`, `every_link_is_below_the_fold`,
`fold_eq_safe_iff`); `pending_never_folds_to_safe`,
`a_non_safe_link_keeps_the_chain_non_safe`,
`pending_only_from_a_human_resolvable_reason`,
`unsafe_only_from_a_violation`; agreement
(`fold_refines_the_historical_rollup`,
`fold_is_the_historical_rollup_without_pending`,
`a_verdict_resting_on_a_deferred_clause_is_pending`, which reads
`QCore.ContractDeferredReview`'s verdict in four values); and the firewall
(`a_plugin_is_safe_only_with_an_accepted_certificate`,
`a_plugin_is_unsafe_only_with_a_validated_witness`,
`an_uncertified_safe_claim_is_pending`, `an_uncertified_unsafe_claim_is_pending`,
`a_plugin_failure_is_unknown`,
`a_plugin_is_pending_only_when_nothing_was_checked`,
`an_uncertified_plugin_link_keeps_the_chain_non_safe`). `QCore.ReasonCategory`
gains the tenth category, `isHumanResolvable`,
`a_human_resolvable_category_never_supports_unsafe` and
`exactly_three_categories_are_human_resolvable`; every existing theorem
re-proves over it. The diagnostic kinds and the trusted checkers' soundness are
premises; the Rust boundary is pinned by the tests of §7.

## 9. Gate

Measured at `32c7ced3` by [`scripts/gate.sh`](scripts/gate.sh) into the
worktree-private `private/ucp-gate-4/`, every step's output and exit captured
to its own file, nothing piped; `CARGO_INCREMENTAL=0`. `32c7ced3`, `ef560f09`,
`e65226ca` and `62978cf3` were work-in-progress commits of this card's branch,
squashed into its single commit afterwards, so they are not reachable from it;
the squashed tree differs from `32c7ced3` in this report, the increment
paragraph and `clang-version.txt` only.

| Step | Command | Exit | Count |
|---|---|---:|---|
| submodules | `git submodule update --init --recursive` | 0 | |
| clang | `clang --version` | 0 | Ubuntu clang 21.1.8 ([recording](clang-version.txt), this run's) |
| test | `python3 scripts/run_test_binaries.py` | 101 | **5,862 passed / 1 failed / 11 ignored**, 315 summaries over 305 units (`private/test-binaries/20260926T091358Z-1295240/`); the one failure is constraint 16's load-sensitive row, see below |
| clean | `cargo clean --profile dev` | 0 | |
| fmt | `cargo fmt --all -- --check` | 0 | |
| clippy | `cargo clippy --all-targets -- -D warnings` | 0 | |
| probe / followup | `cargo run -p qcore-compiler --example reevaluation_probe` / `reevaluation_followup` | 0, 0 | |
| lean | `python3 scripts/check_lean_audits.py` | 0 | **199** audits (+1, `AuditVerdictSemigroup.lean`) |
| python | `python3 -m unittest discover -s scripts/tests` | 0 | 358 tests, after the matrix regeneration (`build_matrix.py .`, exit 0; `named_followups` 434 → 441; gate figures carried, not supplied) |
| docs index | `python3 scripts/build_docs_index.py --check` | 0 | |
| contracts | `python3 providers/contracts.py check` | 0 | load path 11 crates, views unchanged |
| provider Python | `python3 -m unittest discover -s providers/tests` | 0 | 15 tests |
| packages (constraint 18) | `python3 providers/package.py build --output private/ucp-gate-4/packages`; `verify` × 5 | 0; 0 × 5 | rebuilt for the `qcore-schemas` 17.0.0 major |
| provider test | `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline --no-fail-fast` | 0 | **129 passed / 0 failed / 1 ignored**, 43 summaries |
| provider fmt / clippy | `cargo fmt --manifest-path providers/Cargo.toml --all -- --check`; `cargo clippy … -D warnings` | 0, 0 | |
| arbiter | `run_audit.py private/ucp-arbiter-1 62978cf3 --jobs 8`; `scripts/identity.sh` | 0; 0, 0, 0 | §6 |

**The one failure.** `vp04_vp07_vp08_process_lifecycle_and_payload_budgets`
(`compiler/tests/cycle20_verifier_plugins.rs:794`, its closed-input arm's
500 ms wall-clock bound) failed once in the runner's exclusive phase at a
1-minute load average of about 13–18, with sibling cards building. It is one
of constraint 16's named load-sensitive rows. Verified in isolation, as the
constraint requires and without weakening anything, it passed **5 of 5** at
load 3.7–4.1 (`private/ucp-gate-4/isolated/run1..5.log`, exits 0), and it
passed in the previous full run of this card (`private/ucp-gate-3/`, at
`e65226ca`). This card changed no code on that row's path. The full suite at
`e65226ca` also exposed two assertions of held functions still expecting
`Unknown` (`contract_revision_identities`, `pending_review_report`); both were
moved to `Pending` at `ef560f09`, the intended change of §2, and pass in this
run. Its clippy step also required boxing the host's dispatch refusal
(`32c7ced3`).

## 10. Follow-ups

Registered in `named_followups` (434 → 441) from
[followups.json](followups.json) by [scripts/update_ledger.py](scripts/update_ledger.py):
`W45-CHECKPOINT-COMPOSE-SEMIGROUP` (wire W45-AUDIT-CHECKPOINTS' `.qo` compose
to `qcore_schemas::verdict`), `W45-PLUGIN-CERTIFICATE-CHECKER`,
`W45-PLUGIN-COUNTEREXAMPLE-CHECKER`, `W45-PLUGIN-ROUTE-COVERAGE` (the
configured route and the arbiter run no plugin), `W45-PLUGIN-LINK-CFG-PREMISE`
(the callee chain reads `ContractCall` nodes), `W45-PLUGIN-OBLIGATION-PROPERTY`
and `W45-PLUGIN-EXECUTION-PROFILE`. Each names its discharge condition.
