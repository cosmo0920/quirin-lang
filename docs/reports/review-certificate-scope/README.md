# E01-TAIL-CERT-SCOPE — independent Safe-certificate denominator

2026-09-20. Declared base: `a8f2048d`. Review priority D2 was input for
analysis, not specification authority. This card closes the recorded
`E01-TAIL-CERT-SCOPE` boundary for legacy-C Safe certificates.

## Result and boundary

`LegacySafetyCertificate` now carries a `LegacyCertificateScope` containing
the complete selected definition inventory, one coverage row per target, a
canonical inventory digest, the raw/line-marked evidence needed to reconstruct
main-source selection, and digests of the raw source, authoritative `-E -P`
text, marked evidence and exact gateway request. The request retains ordered
include paths, definitions and typed preprocessing/frontend options, plus the
unmodified `CSourceInput`, dialect and scope.

`check_legacy_safety_certificate` reconstructs the denominator independently:
preprocessed-translation-unit scope scans the retained analyzed text;
main-source scope repeats the raw/marked union from retained artifacts and
checks that the marked content pairs with the analyzed `-E -P` text. It then
requires sorted unique target rows, `AnalyzedSafe` coverage for every target,
exact equality with the summary's function keys, and recomputed inventory and
request/artifact identities. It never uses `checked_functions` as the target
set. That field remains a checked compatibility index.

Structural replay establishes what the retained artifacts describe. It cannot
authenticate that those artifacts are the audit the caller intended: a
coherently substituted smaller source is a different, internally valid input.
`legacy_certificate_expected_context` independently derives the intended
context from the caller's `C11TranslationUnit`, and
`check_legacy_safety_certificate_for` requires the certificate to match it.
Callers requiring target authentication retain or rederive that context rather
than copying it out of the certificate. This authenticates the intended
artifact/request/scope, not the complete verdict authorization context.

`MainSourceDefinitions` remains an explicit legitimate partial scope: header
definitions are excluded by the scope label and include-depth attribution,
while header bytes still affect the preprocessed/evidence identities. Raw-only
`#if 0` definitions remain in the selected denominator and withhold Safe when
unread. K&R definitions keep their classified unanalyzable refusal. No
excluded or Unknown selected definition can appear inside a Safe certificate;
the public coverage variants preserve explicit vocabulary for a future result
format without weakening this rule.

The exact marked bytes come from the immutable prepared translation unit on the
audit path, so certificate issuance adds no Clang process there. The legacy
compatibility entry point may still obtain marked evidence separately. This
card changes certificate/API shape, not C verdict semantics; no corpus or
conformance count is promoted.

## Negative evidence

`crates/qcore-c-gateway/tests/certificate_scope.rs` checks the independent
boundary directly:

| Mutation or boundary | Required result |
|---|---|
| remove one target, checked index and summary row together | structural replay rejects against reconstructed evidence |
| duplicate a selected name | replay rejects noncanonical denominator |
| relabel one selected row Excluded or Unknown | replay rejects Safe coverage |
| change the scope enum without changing evidence | replay rejects the label/evidence mismatch |
| mutate raw source, marked evidence or ordered request fields | replay rejects content pairing or recomputed identity mismatch |
| substitute a coherent smaller certificate and its own digest/evidence | structural replay accepts it as its own artifact; caller-held intended context rejects it |
| change only a selected unit's header | main-source denominator stays explicitly partial; input identity changes and the old certificate does not match |
| retain a raw-only `#if 0` definition or a K&R definition | issuance remains Unknown, never Safe |

The pre-existing `e01_denominator_coverage` and `e01_source_enumeration` tests
continue to pin the raw-union and K&R cases. Prepared-TU tests show the audit
path shares the exact preprocessing products rather than reconstructing them.

## Lean parity and premises

`QCore.CertificateScope` separates structural acceptance from caller-context
acceptance. General audited theorems show that structural acceptance implies
exact all-analyzed rows, a duplicate-free reconstructed target list and a bound
inventory digest; contextual acceptance implies equality with the caller's
expected targets and input identity; arbitrary unequal denominators or input
identities reject. Concrete witnesses cover reduction, duplication,
unresolved coverage, coherent smaller substitution, header identity change and
the matching non-vacuity control. `AuditCertificateScope.lean` registers the
theorems, and `AuditAll.lean` covers the complete module.

The Lean digest is collision-free logical content. Rust uses SHA-256 over
length-prefixed canonical rows and serialized request fields. Collision
resistance, faithful Clang extraction and Rust–Lean correspondence remain
premises. The certificate request identity does not replace audit-level build
provenance: compiler executable/version, target environment, filesystem/header
provenance and checkout identity remain the external `CompileTaskIdentity`
boundary. The caller-intended contract/profile/trust environment also remains
external. Ordinary replay checks the certificate's own contract source
assumptions, but the expected target context does not compare those contracts
with a separately held caller expectation.

## Residuals and discharge conditions

- `E01-TAIL-CERT-SCOPE` is discharged: selected denominator and coverage are
  reconstructed from separately retained source evidence, and caller-context
  replay detects coherent scope substitution.
- `CERT-SCOPE-COMPILETASK-PROVENANCE` remains open. Discharge requires binding
  the caller-held expected context to the audit's `CompileTaskIdentity`
  (compiler identity, target/environment and dependency provenance) and a
  mismatch test. The present API deliberately calls its inner record a gateway
  request/artifact identity and makes no full-build attestation claim.
- `CERT-SCOPE-EXPECTED-TRUST-CONTEXT` remains open. Discharge requires a
  caller-held expected contract/profile/trust identity and a mismatch test;
  rechecking the certificate's own contract sources is structural containment,
  not proof that the caller intended that contract environment.
- SHA-256 collision resistance, faithful C extraction and Rust–Lean
  correspondence remain stated trust premises under standing constraint 28;
  this card does not relabel them proofs.

## Standing gate

Final commands, exits, counts and private log paths are recorded in
[`gate.json`](gate.json). Logs live under the worktree-private
`measure/review-certificate-scope-final/`; no log is committed. The fresh
worktree submodule initialization is part of that run.

| Command | Exit and measured result | Private log |
|---|---|---|
| `git submodule update --init --recursive` | `1`: the sandbox denied the linked worktree's git-module directory; approved retry `0`, pinned libevent `d82464a2` | `submodule.log`, `submodule-retry.log` |
| `clang --version` | `0`, Ubuntu clang 21.1.8 | `clang-version.txt` |
| `CARGO_BUILD_JOBS=2 cargo test --workspace --no-fail-fast` | `0`; 4,895 passed, 0 failed, 10 ignored, 254 strict result summaries | `rust-escalated-final.log` |
| five affected gateway test binaries after the mechanical Clippy cleanup | `0`; 54 passed, 0 failed, 1 ignored | `focused-final.log` |
| `cargo fmt --all -- --check` | `0` | `fmt-final.log` |
| `CARGO_BUILD_JOBS=2 cargo clippy --workspace --all-targets -- -D warnings` | `0` | `clippy-final.log` |
| `cargo run -p qcore-compiler --example reevaluation_probe` | `0`; zero failed expectations | `reevaluation-probe.log` |
| `cargo run -p qcore-compiler --example reevaluation_followup` | `0`; zero failed expectations | `reevaluation-followup.log` |
| `python3 scripts/check_lean_audits.py --jobs 2` | `0`; 151/151 audits | `lean.log` |
| `python3 -m unittest discover -s scripts/tests` | `0`; 313 tests | `python-final.log` |

The full Rust run measured code tip `9c270eb7`. The only following source
commit, `af88325b`, resolved the four Clippy findings by simplifying two
expressions and adding two localized, documented lint allowances that preserve
the public by-value result and explicit private PreparedTU evidence boundary.
The affected certificate and semantic-replay binaries, and every other gate
step, ran at `af88325b`; the integrator's combined gate owns the final full-Rust
replay. This split is recorded rather than presenting the earlier Rust run as a
measurement of the later tree.

Rejected and superseded attempts remain distinct. `rust.log` was the sandboxed
run stopped after host restrictions and diagnostic-order regressions were
identified; `rust-escalated.log` was stopped when the final PreparedTU commit
arrived. Neither contributes a count. The initial Clippy run exited 101 with
the four findings repaired by `af88325b`. The first Python run executed all 313
tests and exited 1 only because the committed derived Lean count was 150; the
matrix generator preserved the carried sections and changed the derived audit
count/list to 151, after which all 313 passed. A command naming nonexistent
example `reevaluation_followup_probe` exited 101 and is retained as
`reevaluation-followup-invalid.log`; it is not a gate attempt, and the required
`reevaluation_followup` command passed.

## Integrator reconciliation list

The scope-owned code/model commits are `517e58ba`, `55f32edd`, `5358072d`,
`8ad203f5` and `af88325b`, followed by this report/gate commit. Commits
`00344321`, `6fecb101`, `b316d08a` and `9c270eb7` are the PreparedTU dependency
chain already owned by its card; they are not additional certificate-scope
changes. Integration already consumed the combined code tip, so only this
report/gate commit remains to reconcile.

1. **Standing gateway/source-identity amendment:** record
   `E01-TAIL-CERT-SCOPE` discharged by the reconstructible certificate target
   inventory and caller-held expected-context checker. Preserve the distinction
   between structural self-consistency and intended-target authentication.
2. **Tracking amendment:** add open residual
   `CERT-SCOPE-COMPILETASK-PROVENANCE` with this exact discharge condition:
   bind the expected certificate context to `CompileTaskIdentity`, including
   compiler/target/environment and dependency provenance, and exercise a
   mismatch. Do not describe the inner digest as full build provenance.
   Also add `CERT-SCOPE-EXPECTED-TRUST-CONTEXT`, discharged only when a
   caller-held expected contract/profile/trust identity is compared and a
   mismatch is exercised.
3. **Public interface:** `LegacySafetyCertificate` gains `target_scope`;
   `LegacyCertificate{Coverage,Definition,ScopeEvidence,InputIdentity,Request,
   Scope,ExpectedContext}`, `legacy_certificate_expected_context` and
   `check_legacy_safety_certificate_for` are public and re-exported by the
   compiler facade. Existing direct struct initializers must use a real issued
   certificate or provide the new scope record.
4. **Evidence bookkeeping:** register this report/card and its residual in the
   acceptance matrix during integration. Do not change corpus verdict counts,
   CT counts, full-conformance status or global STATUS from this card.
5. **Dependency reconciliation:** merge the final PreparedTU card before this
   card and preserve the adapter that passes its exact raw, marked and `-E -P`
   artifacts into certificate issuance without an additional Clang process.
