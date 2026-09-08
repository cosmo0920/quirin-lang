# P08 — Reproduce the configured five-provider composition (bounded)

2026-09-08. Baseline `c60fd91` ("Consolidate the second delegated
T04/P05/P06/P07/O04/F01/CT increment into the status ledger"). Workstreams
W5/W3, lane Integrator, kind integration.

**Outcome: the bounded composition is audited and pinned; the full
five-provider completion gate stays OPEN.** P06 (CTraces) and P07 (librdkafka)
landed design-only, so two of the five slots have no implemented capability and
no core checker. This card audits the composition that exists — four
implemented capability routes across three packages, with the two design-only
slots registered, consulted and explicitly retained as `Unknown` — and records
what the fifth-and-sixth capability would still need.

**Nothing is promoted.** No production source file changed: the whole diff is
two new test files and this report. Raw Cycle 19 units stay 2 Safe /
42 provisional Unsafe / 137 Unknown plus four data-only units, CT coverage
stays 15/50 mapped, the contract corpus stays 59 modules, `qcore-schemas` stays
5.0.0, protocol v1 and evidence v2 are unchanged, and no diagnostic code was
added, changed or downgraded.

| Evidence | Path |
|---|---|
| Package-side audit (2 tests) | `providers/conformance/tests/composed_five_providers.rs` |
| Profile-side audit (2 tests) | `compiler/tests/composed_five_providers.rs` |

Dependencies, all landed at their stated boundaries:
[P04](../remaining-p04/README.md), [P05](../remaining-p05/README.md),
[P06](../remaining-p06/README.md) (design), [P07](../remaining-p07/README.md)
(design), [P03](../remaining-p03/README.md),
[E03](../remaining-e03/README.md).

---

## 1. What the composition actually is

Two registries carry the name "the five providers", and they are not the same
registry. Both are pinned here, because the difference is the whole reason
packaging cannot move a verdict.

### 1.1 The profile-declared registry (`load_first_party_registry`)

`compiler/src/first_party.rs` walks `contract/profiles/*` and registers every
provider manifest it finds. Measured:

| Profile directory | Providers it declares |
|---|---|
| `chunkio-core` | — |
| `cmetrics-counter` | — |
| `fluent-bit` | `qcore.chunkio`, `qcore.cmetrics`, `qcore.ctraces`, `qcore.librdkafka` |
| `libevent` | `qcore.libevent` |

**No single profile binds all five** (finding P08-F5). The default registry is
an assembly of two source sets, and every entry in it is the **built-in v1
adapter**: `package_version` `0.1.0`, `protocol_versions` `[1]`,
`evidence_versions` `[1]`, `certificate_checker`
`qcore.unsupported_assertions.v1`, no observation hook, no filesystem input, no
network.

### 1.2 The packaged registry (`providers/package.py`)

| Package | Version | Declared libraries | Evidence | Certificate checker |
|---|---|---|---|---|
| `qcore.chunkio` | **0.5.0** | `chunkio`, `fluent_bit` | 1, 2 | `qcore.boolean_field_contract.v2` |
| `qcore.cmetrics` | **0.2.0** | `cmetrics` | 1, 2 | `qcore.boolean_field_contract.v2` |
| `qcore.libevent` | **0.2.0** | `libevent` | 1, 2 | `qcore.reactor_callback_program.v1` |
| `qcore.ctraces` | 0.1.0 | `ctraces` | 1, 2 | `qcore.unsupported_assertions.v1` |
| `qcore.librdkafka` | 0.1.0 | `librdkafka` | 1, 2 | `qcore.unsupported_assertions.v1` |

Five packages declare **six libraries, each by exactly one package**. That
disjointness is what makes provider absence visible rather than silently
covered, and it is asserted.

---

## 2. Acceptance 1 — comparison across all constituent routes

Four evidence-v2 capability routes are implemented. Each is derived by the
trusted core, transported to a real process, and admitted only after the core
re-evaluates its own contract or re-runs its own model.

| Route | Package | Capability | Semantic profile | Witness kind |
|---|---|---|---|---|
| `chunkio.boolean_field` | `qcore.chunkio` | `resource_contracts` | `qcore.c_boolean_field_contract.v2` | `boolean_field_contract` |
| `cmetrics.boolean_field` | `qcore.cmetrics` | `resource_contracts` | `qcore.c_boolean_field_contract.v3` | `boolean_field_contract` |
| `chunkio.counter` | `qcore.chunkio` | `resource_contracts` | `qcore.c_counter_contract.v1` | `counter_contract` |
| `libevent.reactor_callback` | `qcore.libevent` | `event_traces` | `qcore.reactor_callback_program.v1` | `reactor_callback` |

Driven through **one registry holding all five packages**:

- **The 4 x 5 answer matrix.** Twenty cells. Exactly **four** are `Safe` — the
  diagonal. The other **sixteen** are `Unknown` with **no certificate** and
  `QP005` ("plugin does not declare the requested capability and target
  profile"), because the declared libraries are disjoint. No package can
  substitute for another, in either direction.
- **Source identity.** The four routes' `UnitIdentity` values are pairwise
  distinct, asserted for all six pairs.
- **Coverage.** Each certificate's coverage set equals the core's own
  `coverage_for(capability)` for that task — never a provider claim — is
  nonempty, and the four routes' obligation-identifier sets are pairwise
  disjoint.
- **Assumptions.** Each certificate republishes the task's own premise set;
  composition later refuses any component that would understate it (E03
  repair 2), which is why the premise comparison is meaningful rather than
  decorative.
- **Observation identities.** There are none. No shipped manifest declares a
  `required_observation_hook`, and no route mints a `contract_observation`
  witness — both asserted. The composition therefore carries **no observation
  identity at all**; the observation-bearing capabilities (Libevent
  `check_observation_refinement`, the ChunkIO cleanup-outcome family) remain in
  process with their named blockers.
- **Diagnostic categories.** Every refusal in this audit lands in the existing
  `QP001`–`QP016` vocabulary or in a provider-attributed
  `qcore.<id>.<local>` code. No new code, no changed code.
- **Verdicts under packaging.** For each of the four routes, the outcome
  through a registry holding **only its implementer** is compared with the
  outcome through the **five-provider** registry using full structural
  equality — verdict, diagnostics and the whole certificate, including its
  coverage denominator. They are equal. Registering the other four providers
  changes nothing.

### 2.1 The profile-declared registry answers none of them

Asked the same reactor-callback route, **all five** profile-declared built-ins —
including `qcore.libevent`, which *implements* the capability as a package —
answer `Unknown` with `QP004` and no certificate, because their manifests admit
evidence version 1 only. An unknown plugin id is `QP002`, not an optimistic
resolution.

This is finding **P08-F1** and it is deliberately **not repaired**: it is
fail-closed, and it is the mechanical reason "no new `Safe` promotion caused by
packaging" holds for the default route. Nothing reaches a v2 certificate unless
a caller explicitly selects a built package.

---

## 3. Acceptance 2 — absence, version mismatch, timeout, conflicting evidence

Each row is exercised **per provider**, over the real built packages.

### 3.1 On the four implemented v2 routes

| Failure mode | How it is produced | Result |
|---|---|---|
| Absence | registry built from the other four packages | `Unknown`, `QP002`, no certificate |
| Disabled | `set_enabled(id, false)` on the five-provider registry | `Unknown`, `QP003`, no certificate |
| Version mismatch (manifest) | installed manifest reduced to `evidence_versions = [1]` | `Unknown`, `QP004`, refused **before** the process is spawned |
| Version mismatch (transport) | projection `schema_version = 0` on each route's own input | `request.validate()` fails; `Unknown`, `QP001` |
| Version mismatch (profile) | declared target profile moved to `qcore.c_boolean_field_contract.v3` for an i32 source | the process itself declines with its own `qcore.chunkio.*` diagnostic; `Unknown`, no certificate |
| Timeout | installed manifest `limits.timeout_ms = 1` | `Unknown`, `QP006`; `ProcessExecutionRecord::outcome == WallClockTimeout` |
| Conflicting evidence (unbound projection) | a well-formed projection the core never imported | `Unknown`, `QP001` ("source projection is not bound to the core task") |
| Conflicting evidence (rebound) | the same projection rebound to a recomputed task digest | past the host binding, then `Unknown`, `QP009`, no certificate |
| Conflicting evidence (false contract) | contract mutated so the real source violates it | `Unknown`, `QP009` on all four routes — a `Safe` proposal cannot bypass the core's own evaluation |
| Conflicting evidence (refused model) | a reactor program `check_event_program` rejects | `Unknown`, `QP009`, no certificate |

Every one of absence, disabled, timeout and the unbound/rebound projection is
additionally shown to **block the composite** at `QP011` when merged with an
honest `Safe` component. A failing component is never skipped and never covered
by the providers that did answer.

### 3.2 On the CTraces and librdkafka slots

They have no v2 route (§4), so the same four failure modes are exercised on the
route they do have — protocol v1 — for **all five** packages uniformly:

| Failure mode | Result |
|---|---|
| Present and consulted | `Unknown`, no certificate (v1 is non-certifying by rule) |
| Absence | `QP002` |
| Disabled | `QP003` |
| Version mismatch (`protocol_versions = [2]`) | `QP004` |
| Timeout (`timeout_ms = 1`) | `QP006`, `outcome == WallClockTimeout` |

"Explicitly retained" means exactly this: both packages are built, registered,
really executed, and answer — they are not dropped from the registry and they
cannot stand in for an implementing provider.

---

## 4. Acceptance 3 — what parity is claimed, and what is refused

### 4.1 Claimed

- **Registry-composition parity for the four implemented capabilities.** For
  `chunkio.boolean_field`, `chunkio.counter`, `cmetrics.boolean_field` and
  `libevent.reactor_callback`, the five-provider registry reproduces the
  single-provider answer exactly: same verdict, same diagnostics, same
  certificate, same coverage denominator, same premise set, same attribution.
- **Single-route composition exactness.** `compose_verification_with_inventory`
  accepts a real package's certificate against the core's own required set;
  a duplicate neither adds nor destroys coverage; a widened required set is
  `QP011`; an empty required set is `QP016`. This is the non-vacuity control:
  the relation does accept real package evidence.
- **Failure containment.** Every failure mode in §3 leaves the affected
  obligations `Unknown` with the correct existing `QP` code and no certificate,
  and blocks any composite it participates in.

### 4.2 Explicitly refused

- **No parity claim for CTraces or librdkafka capabilities.** There are none to
  compare. Their profile inventories import and assert nothing (P06 §1.1,
  P07), their processes carry no v2 proposal function, and — measured here —
  no evidence-v2 task can be derived *for* them at all, because
  `BooleanFieldSourceTask::derive` binds `target.library` to the model library
  the source was imported under and neither library declares a
  `boolean_field_models` entry in any profile. The design-only outcomes cannot
  satisfy the five-provider completion gate and are not presented as doing so.
- **No multi-provider composition claim.** See §5.
- **No re-measurement of the four pinned 185-unit artifacts.** See §6.
- **No claim about the in-process routes P04/P05 retained.** Their removal is
  still P09.

---

## 5. Measured boundary: multi-provider composition is unreachable (P08-F2)

The card's original shape assumed the shipped packages could co-certify a unit.
They cannot, and the reason is worth recording precisely, because it was found
by measurement rather than by reading.

Two independent gates block it, in opposite directions:

1. **Across capabilities — E03-B2.** `derive_task` folds the capability's
   semantic profile into `unit.build_digest`, so two capabilities over one
   source certify different `UnitIdentity` values and `compose_verification`
   refuses them with `QP015`. Measured here against the real packages, not a
   fixture.
2. **Across providers — the model-library binding.**
   `BooleanFieldSourceTask::derive` refuses any target whose `library` is not
   the model library the summary was imported under. A source imported for
   `chunkio` can therefore never be aimed at `qcore.cmetrics`, and vice versa.
   The two Boolean routes are consequently over two different C sources under
   two different profiles, so their units differ too.

**All six cross-route pairs compose `Unknown` with `QP015`**, asserted. It
follows that `QP012` (conflicting or unequal premises) and `QP013`
(incomparable bounds) are **not reachable through the shipped packages at all**:
`QP015` fires first in every reachable pairing. E03's composition relation is
therefore still exercised only by its own fixtures for those two codes, and
this card does not upgrade that.

The honest summary: **today's five-provider composition is five isolated
single-provider verifications sharing one registry.** The registry is real, the
isolation is enforced and pinned, and the merge relation is exercised only in
its single-component and duplicate forms.

---

## 6. What could not be re-measured

- **The four pinned 185-unit Cycle 19 artifacts.** The profile pins Fluent Bit
  at `66910c10`; the developer checkout is at `90a01091`, so the audit's own
  pin gate refuses to run, exactly as recorded by C02, G01, P04 and P05. The
  archived artifacts were **not touched and not overwritten**.
- **The canonical CMetrics configured audit.** It needs a prepared Clang CMake
  build tree (`scripts/prepare_cmetrics_clang_audit.py`), which this worktree
  does not have.

Both are unchanged **by construction, not by measurement**: `git diff --stat`
against the baseline is empty and the only additions are two test files and
this report, so no analysis, gateway, audit, host, schema or provider byte
participates in either artifact's derivation. The audit-facing conformance
tests that *do* run here (`configured_audit`, `configured_source_contract`,
`cmetrics_unit`, `source_effect`, `counter_effect`,
`reactor_callback_capability`, `packages`) all pass unchanged.

CORPUS-REPIN remains open and now also names P08 as a card that did not
discharge it.

---

## 7. The full five-provider completion gate: still OPEN

To claim the gate, each remaining slot needs, in order:

| Slot | What is missing |
|---|---|
| `qcore.ctraces` | An implemented `ctraces.span.dropped_counts.v1`: a core checker, a derivable evidence-v2 task (a profile entry with an audited file, a source witness and a field model — the entry currently declares a source root and nothing else), a `SemanticWitness` variant or an existing one that fits, a provider proposal function, and the P06-I4 manifest repair |
| `qcore.librdkafka` | An implemented `qcore.librdkafka.queue_final_destroy.v1`, plus repair of the six declared-but-unloaded mechanism operations P07 measured as unfaithful, plus the same task/checker/witness/proposal chain and the P06-I4 manifest repair |
| Both | A profile that binds them, since the `fluent-bit` profile's inventories for these libraries are empty |
| Composition | Either E03-O3 (capability-independent unit identity) or a second provider able to certify an existing unit, before any multi-provider merge is reachable at all |

Until then, `qcore.ctraces` and `qcore.librdkafka` are registered, executed and
`Unknown` — which is the fail-closed answer, not a gap in this audit.

---

## 8. Findings recorded, not repaired

- **P08-F1** — the profile-declared built-in registry is permanently
  evidence-v1-only at `package_version 0.1.0`, while the shipped packages are
  at 0.5.0/0.2.0/0.2.0/0.1.0/0.1.0 with evidence `[1, 2]`. Consequence: the
  default first-party registry can never reach a v2 certificate; every
  implemented capability is `QP004` there. Raising the built-in
  `evidence_versions` would let built-ins be asked v2 questions they answer
  `Unknown` to — no soundness change, but a pinned-manifest change with no
  consumer, so it is left alone and pinned as-is.
- **P08-F2** — multi-provider composition is unreachable through the shipped
  packages (§5); `QP012`/`QP013` are unreachable with it.
- **P08-F3** — `qcore.ctraces` and `qcore.librdkafka` still name
  `certificate_checker: qcore.unsupported_assertions.v1`, which nothing in this
  repository implements, in **both** the package and the built-in manifest.
  This is P06-I4; it is asserted here so the dangling identity cannot drift
  silently, and it is harmless only because those providers never propose.
- **P08-F4** — the full five-provider completion gate is open (§7).
- **P08-F5** — no single profile binds all five providers; the default registry
  is assembled across `fluent-bit` and `libevent`.

---

## 9. Validation

Full standing gate on this worktree, 2026-09-08, Linux, Clang available:

| Gate | Before | After |
|---|---|---|
| `cargo test --workspace` | 1141 passed, 0 failed | **1143 passed, 0 failed** |
| `cargo test --manifest-path providers/Cargo.toml --workspace` | 56 passed, 0 failed | **58 passed, 0 failed** |
| `cargo fmt --all -- --check` | clean | clean |
| `cargo fmt --manifest-path providers/Cargo.toml --all -- --check` | clean | clean |
| `cargo clippy --all-targets -- -D warnings` | clean | clean |
| `cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets -- -D warnings` | clean | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 | exit 0 |
| `python3 -B -m unittest discover -s scripts/tests` | 43 passed | 43 passed |
| `cd proof && lake build` | success | 67 jobs, success |
| `python3 -B scripts/check_lean_audits.py` | 23 audits | **23 audits passed**, no new axiom |

`lake` is not on the default `PATH` on this host; it was run from
`~/.elan/bin`. The Libevent submodule required
`git -c protocol.file.allow=always submodule update --init --recursive`; the
pin test passes at `d82464a2`.

No Lean obligation is added: no semantic rule, checking rule or diagnostic code
changed. The composition relation and its fifteen
`QCore.VerifierPlugin.Composition` theorems are E03's and are untouched.

The package-side test was run four times against the final file (three repeats
plus the full providers-workspace suite) to check for flakiness in the 1 ms
timeout rows; all four runs passed.

---

## 10. Integrator notes

Proposed, not applied — `AGENTS.md`, `CLAUDE.md`,
`docs/reevaluation/ct_manifest.json` and
`docs/reviews/r0_3/acceptance_matrix.json` are untouched.

Suggested status-ledger paragraph:

> Bounded five-provider composition (P08): the five shipped packages are
> registered in one registry and every implemented evidence-v2 route is driven
> through it. Four routes are implemented (ChunkIO Boolean field, ChunkIO
> counter, CMetrics Boolean unit-result, Libevent reactor callback) and each is
> answered by exactly one package; the other sixteen cells of the 4 x 5 matrix
> refuse with QP005, and registering all five reproduces every single-provider
> answer exactly — verdict, certificate, coverage and premises. The CTraces and
> librdkafka slots are registered, executed and Unknown: no evidence-v2 task can
> be derived for them, so the **full five-provider completion gate stays open**
> and now names the two capabilities P06 and P07 specified but did not
> implement. Absence, disabled, manifest/transport/profile version mismatch,
> a 1 ms wall-clock timeout and four kinds of conflicting evidence are
> exercised per provider and each blocks the composite at its existing QP code.
> Multi-provider composition is measured unreachable through the shipped
> packages — capability-scoped unit identity (E03-B2) in one direction and the
> core's model-library binding in the other — so all six cross-route pairs
> refuse with QP015 and QP012/QP013 stay fixture-only. No production file
> changed; the four pinned 185-unit artifacts were not re-measured
> (CORPUS-REPIN). Evidence: `docs/reports/remaining-p08/README.md`.

Acceptance-matrix suggestion: P08 closes as *bounded audit complete, completion
gate open*, carrying two Rust test files as its evidence classes and no Lean
class of its own (it changes no semantics). Its open items are P08-F1 through
P08-F5, and it should be linked from P06-I4 and E03-O3.

No CT case is mapped or unmapped. No contract-corpus count moves. No raw unit is
promoted.
