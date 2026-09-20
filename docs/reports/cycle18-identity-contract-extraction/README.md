# Cycle 18 timed-wait identity contract extraction

Date: 2026-09-20. Base: `c4d3f187`. Card scope: extract the bounded observed
timed-wait identity/context acceptance policy from Rust conditionals into an
executed QCore contract, without changing replay outcomes or promoting any
verdict, attestation, callback coverage, or safety claim.

## Result

`contract/qcore/monkey/wait_identity_policy.qcore` is a compiled-in but
separately hash-bound policy with **3 declarations and 29 clauses**. It decides
the exact semantic profile, thread and producer-trust classifications,
manifest-digest shape, four bounds, callback/flags/count relations, and the
actual/expected equality relations for contracts, source rules, observation
rules, payloads, revision, source profile, scope, backend and every individual
summary/contract/observer-source identity.

Rust still owns byte decoding, generic hashing and character classification,
fact derivation, source projection and the operational event interpreter. It
passes typed integer/text facts to `check_typed_contract_facts`; there is no
overall authority Boolean. Missing facts, unequal identities, unsupported
policy expressions and type mismatches fail closed. The dynamic source,
contract and observer-source maps are checked one entry at a time by the same
contract clause.

`wait_observation_rules_digest()` binds framed `OBSERVED_WAIT_RULES` bytes and
the exact compiled policy bytes. The explicit
`check_observed_wait_composition_with_policy` override names caller-selected
policy as trusted configuration and requires the admitted context to bind its
exact bytes. A policy swap under a stale context refuses before replay.
`observed_wait_bounds` derives run, dispatcher, event-identity and record counts
from the trace; policy clauses decide whether those actual counts are in the
supported one-run/one-dispatcher/one-event/eleven-record profile.

The successful result remains
`caller_prepared_artifact_identity_conditional_replay`. The event model replay
is unchanged. It still establishes neither native provenance nor an all-path
source result, `Safe`, artifact attestation, or composed callback coverage.

## Negative evidence and proof parity

The focused integration test preserves the baseline accepted fixture and all
pre-existing context/assumption/payload refusals. It additionally establishes:

- changing the policy bytes under the default context refuses;
- rebinding a policy mutation that requires `attested_runtime_evidence` still
  refuses the unchanged producer-supplied observation fact;
- coherently rebinding a mutated `records_bound_actual == 12` clause refuses the
  eleven-record fixture, proving the contract executes;
- a clause requiring an absent primitive refuses rather than defaulting;
- trace-derived bounds expose a second run, dispatcher and event identity and
  a ten-record trace as the corresponding primitive counts.

`QCore.ObservedWaitComposition` models optional text facts, per-identity
equality, exact profile/bound admission, stale policy identity, missing callback
identity and the mutated record clause. `AuditObservedWaitComposition.lean`
prints each new theorem's axiom closure; no placeholders or project axioms were
introduced.

## Focused validation

Logs are worktree-private under `measure/identity-policy/`.

| Command | Exit | Result |
|---|---:|---|
| `git submodule update --init --recursive` | 0 | pinned libevent `d82464a277d0f42703702c4dfd9af6af38595a83` initialized |
| `cargo test -p qcore-verification --test observed_wait_composition` | 0 | 3 passed, 0 failed |
| `cargo fmt --all -- --check` | 0 | clean |
| `cargo clippy -p qcore-verification --all-targets -- -D warnings` | 0 | clean |
| `cd proof && lake build QCore.ObservedWaitComposition` | 0 | 5 jobs completed |
| `cd proof && lake env lean AuditObservedWaitComposition.lean` | 0 | all listed theorem closures printed; permitted axioms only |
| `git diff --check` | 0 | clean |

The first direct Lean audit invocation exited 1 because the fresh worktree had
no compiled `QCore` library yet; the subsequent explicit module build and audit
both exited 0. The integrator owns the combined standing gate after sequential
merge.

## Integrator reconciliation list

1. Cherry-pick typed contract fact engine commits `1c173b4d` and `66a469d9`
   before this card's implementation commit, unless they are already present.
2. In `qcore-audit`, replace the legacy
   `sha256_hex(OBSERVED_WAIT_RULES.as_bytes())` context field with
   `wait_observation_rules_digest()`. Derive configured bounds from the decoded
   trace with `observed_wait_bounds`; do not copy the policy's desired numbers
   into actual evidence facts.
3. Reconcile the sibling source-policy extraction by replacing this test's
   `MONKEY_SOURCE_REVISION` use with `monkey_source_revision()?` after that API
   lands. Callback and flag expectations here are already derived from the
   source projection requirement and compared by contract, so no duplicate
   callback literal remains in this policy.
4. Keep `wait_identity_policy.qcore` outside the resource-contract selection
   manifest. It is an observation admission policy; adding it to the existing
   manifest would change the pinned resource-contract corpus and is outside
   this card.
5. No acceptance matrix, status ledger, global corpus pins, `QCore.lean`, or
   increment record was changed. Apply any combined figures and run the full
   standing gate only after all Cycle 18 extraction cards are merged.
