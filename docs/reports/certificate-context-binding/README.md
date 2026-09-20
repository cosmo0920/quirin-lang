# Certificate context binding

2026-09-20. Declared base `119115de`. Final measured code tip `18348057`.
This card discharges the registered follow-ups
`CERT-SCOPE-COMPILETASK-PROVENANCE` and
`CERT-SCOPE-EXPECTED-TRUST-CONTEXT` at the scope their registrations name.

## Result

The configured audit now admits a legacy-C Safe certificate through an
audit-owned context boundary in `qcore-audit`. The gateway still owns
structural certificate replay and target-scope reconstruction. The audit layer
adds the facts only it owns: the existing complete `CompileTaskIdentity` record
and an independently derived identity of the exact resource contracts, retained
contract sources, source profile and declared-contract trust policy.

`ExpectedAuditCertificateContext` is derived from the caller's immutable
`PreparedTranslationUnit`, independently constructed compile-task identity,
independently loaded `ResourceContracts`, profile digest and explicit policy
that declared contract sources are trusted assumptions. It is never copied
from the presented certificate. The prepared-context gateway adapter derives
the target scope from the retained raw, line-marked and `-E -P` products without
another Clang process.

Admission performs all of these checks:

1. the gateway structurally replays the certificate and compares its target
   scope with the caller-held expected target;
2. both compile-task records are complete accepted-input records carrying the
   required resource-contract-manifest and source-profile dependencies;
3. the presented task agrees with the certificate's raw/effective input,
   dialect, explicit partial scope, source transport and ordered gateway
   request fields;
4. the expected task independently agrees with the caller's prepared artifacts
   and request;
5. the profile identity agrees with the `source_profile` dependency in each
   compile-task record;
6. the trust identity is rederived from the certificate's current contracts at
   every admission, so replacing a structurally valid certificate inside an old
   wrapper cannot retain stale authorization; and
7. the full presented and expected compile-task and trust identities compare
   equal.

The actual configured resource-projection paths call this boundary before
publishing Safe. A missing or mismatched context does not abort the whole run
and does not shrink the denominator. It records the unit Unknown, retains every
selected definition, changes the aggregate summary and every retained function
row to Unknown, and preserves the refusal reason. This is distinct from the
existing denominator-only withholding path, where already established
per-function findings can remain.

## Negative and positive evidence

`crates/qcore-audit/tests/certificate_context.rs` exercises the public binding
and the configured projection path:

| Case | Required result |
|---|---|
| unchanged prepared input, task, contracts, profile and policy | admission succeeds |
| changed recorded compiler version and default target | compile-task comparison rejects |
| changed build command with byte-identical gateway input | compile-task comparison rejects |
| changed dependency provenance with unchanged analyzed bytes | compile-task comparison rejects |
| changed valid contract environment | trust comparison rejects |
| changed source-profile digest | profile/task or trust comparison rejects |
| coherent smaller artifact with its own valid certificate and task | caller-held target/task rejects |
| accepted task missing required contract/profile dependencies | context construction rejects |
| valid alternate certificate installed in a stale wrapper | admission rederives contracts and rejects |
| configured projection receives a profile/task mismatch | unit and both function rows are Unknown; the two-function denominator remains |

The compiler mismatch is derived through `compile_task_identity` with a typed
recorded `ClangIdentity`, not by editing a digest string. `ClangIdentity::recorded`
supports reconstruction of an external caller's persisted expectation; its
documentation states that the record is a caller claim, not execution
attestation. The ordinary configured audit still obtains its presented compiler
identity through `ClangIdentity::capture`.

## Lean model and premises

`QCore.CertificateScope` now models the audit-bound certificate with separate
compile-task and trust-context identities. Audited general theorems prove that
contextual acceptance implies equality with both expected identities and that
arbitrary inequality of either identity rejects. Concrete witnesses show a
changed compile task and a changed contract/profile/trust environment reject;
the original exact-coverage, duplicate, partial-scope, coherent-substitution
and matching positive witnesses remain.

The model uses logical identity equality. Rust uses exact typed-record equality
and SHA-256 identities over canonical serialized data. SHA-256 collision
resistance, faithful Clang/C extraction and Rust–Lean correspondence remain
explicit premises under standing constraint 28. The Lean audit is an abstract
binding/refusal proof, not extraction of the Rust implementation.

## Scope limits

This result authenticates equality with the existing `CompileTaskIdentity`
record; it does not turn that record into a compiler or machine attestation.
The record describes the captured Clang program/version/default target, declared
driver, directory/file, complete ordered arguments, transported frontend
options, raw/effective source identities, selected scope and named dependency
digests. As already documented by that type, it does not hash each individual
header, prove that the declared driver is equivalent to the Clang that ran,
attest the executable or its host environment, or verify declared contract
claims against library bodies. Header influence is bound through the effective
preprocessed bytes, and a different header with identical relevant contents is
the same represented task.

These are boundaries of the registered discharge, not claims silently promoted
by this card and not newly promised follow-up work. No new provenance or
attestation residual is introduced here. The two registered entries asked for
binding to the *existing* compile-task record and caller-held contract/profile/
trust identity; those exact conditions are exercised and discharged.

## Standing gate

The fresh worktree submodule initialization first exited `1` because the
sandbox denied the linked worktree's shared Git module metadata; the approved
retry exited `0` at pinned libevent `d82464a2`. Final commands, counts and
private logs:

| Command | Exit and measured result | Private log |
|---|---|---|
| `git submodule update --init --recursive` | approved retry `0` | tool transcript; pinned revision verified by the Rust suite |
| `clang --version` | `0`; Ubuntu clang 21.1.8 | committed `clang-version.txt` and `clang-version.exit` beside `gate.json` |
| `CARGO_BUILD_JOBS=2 cargo test --workspace --no-fail-fast` | `0`; 4,898 passed, 0 failed, 10 ignored, 255 strict summaries | `rust-full-final2.log` |
| `cargo fmt --all -- --check` | `0` | `fmt-final.log` |
| `CARGO_BUILD_JOBS=2 cargo clippy --all-targets -- -D warnings` | `0` | `clippy-final.log` |
| `cargo run -p qcore-compiler --example reevaluation_probe` | `0` | `reevaluation-probe.log` |
| `cargo run -p qcore-compiler --example reevaluation_followup` | `0` | `reevaluation-followup.log` |
| `python3 scripts/check_lean_audits.py --jobs 2` | `0`; 151/151 audits | `lean.log` |
| `python3 -m unittest discover -s scripts/tests` | `0`; 317 tests | `python.log` |

The first complete workspace run at `bc01012d` also passed 4,898 / 0 / 10,
but is a superseded baseline because review subsequently strengthened refusal
reporting. The first attempted final run after that review was interrupted
explicitly when a second review found retained Safe function rows under an
Unknown unit. Neither run contributes the final count. The final cited run is
at `18348057` after the unit, aggregate and per-function downgrade regression.
Provider packages were unchanged; the gate carries 122 provider tests rather
than presenting them as remeasured. The acceptance matrix already records the
unchanged 151-file Lean audit inventory, so this card changes no generator-owned
matrix field and does not regenerate the hand-maintained tracker.

## Integrator reconciliation list

1. **`CERT-SCOPE-COMPILETASK-PROVENANCE`: retire.** The expected context now
   carries the full existing `CompileTaskIdentity`; construction refuses missing
   required fields and rejected preprocessing, independently binds it to the
   prepared input, binds the presented task to the certificate request/artifacts
   and compares the complete records. Compiler/default-target, build-command
   and dependency-provenance mismatches are exercised. Apply the root-owned
   source-identity amendment, which correctly preserves the attestation,
   individual-header and driver-equivalence exclusions above.
2. **`CERT-SCOPE-EXPECTED-TRUST-CONTEXT`: retire.** The caller independently
   derives exact contract/source/profile/declared-trust identity; admission
   rederives it from the certificate's current contracts and exercises contract,
   profile and stale-wrapper mismatches. A refusal leaves the unit and every
   retained function Unknown. Apply the same root-owned source-identity
   amendment; do not replace its boundary language with a broader claim.
3. **Tracker/history/status:** root integrator owns removal of exactly those two
   registered rows and global bookkeeping. This card adds no new registered
   residual, changes no corpus/CT/conformance count and touches no global
   STATUS, increment history, standing rule or hand-maintained matrix section.
4. **Evidence reconciliation:** code/model commits are `bc01012d`, `878ae9e6`
   and `18348057`, followed by the report/gate commit. The 151-audit derived
   matrix inventory is unchanged because the existing audit file was extended.
