# Pluggable, isolated implementation cards

> [!NOTE]
> **Current status (Phases P0–P3 completed and active across the repository).** Every infrastructure, data-decoupling, and gateway-rule seam proposed below (`MEASURE-TOOL`, `GATE-SLOTS`, `GATE-SCOPE`, `AUDIT-CANONICAL-PATHS`, `AUDIT-COMPOSITION`, `CONTRACT-SUPERSESSION`, `CONTRACT-LOCKS`, `EVALUATION-PROFILES`, `LEAN-ROOTS-GENERATED`, `RULE-DOCS-GENERATED`, `AMENDMENTS-SCHEMA`, `GATEWAY-RULE-API`, `RULE-SWITCHES`, `CONTRIBUTION-LEDGERS`, `FRAMES-AS-CLAUSE`, and pilot rule crates `rule-32a` through `rule-38`) has **landed and is live**. Zero phase-table cards remain in `acceptance_matrix.json#named_followups`. This document is retained as the authoritative architectural specification for seams A–G and user decisions 1–10.

## Execution summary by seam

| Seam | Delivered mechanism and live location |
|---|---|
| **A — Decoupled contracts, profiles & ledgers** | Per-provider `providers/*/contract-inventory.lock` (`providers/contracts.py sync`), `contract/evaluations/*` (`scripts/evaluation_profiles.py`), `contract/contributions/*` (`scripts/contribution_ledgers.py`), and `FRAMES-AS-CLAUSE` (`C04.6.4`). |
| **B — Pluggable gateway rule packages** | `crates/qcore-gateway-rule-api`, `crates/gateway-rules/registry.toml` (`scripts/gen_gateway_registry.py`), `crates/gateway-rules/rule-32a-*` through `rule-38-*`, and runtime `QCORE_GATEWAY_RULES_OFF` switches (with holds marked `not-switchable`, §6.10). |
| **C — Generated Lean roots** | `proof/QCore/<Module>.lean` + `proof/Audit<Module>.lean`, aggregated automatically by `scripts/gen_lean_roots.py` into `proof/QCore.lean` and `proof/AuditAll.lean`. |
| **D — Generated rule docs & amendment schema** | `docs/agents/rules/<id>.md` rendered by `scripts/gen_rule_docs.py`; card integration records in schema `quirin-amendments/2` applied by `scripts/apply_amendments.py` and `scripts/land.py`. |
| **E — Scoped card gate & named slots** | `scripts/gate_scope.py` (precise compile-time and `strace`-verified run-time reader scoping) and `scripts/gate_slot.py` (`rust-suite`, `audit`, `lean`, `short`, and `build` classes; see [`docs/agents/validation.md`](../../agents/validation.md)). |
| **F — Out-of-process measurement & canonical paths** | `quirin-audit measure` subcommand plugin (`crates/quirin-audit-plugin-measure`), canonical unit path matching, and published landing baselines (`scripts/land.py publish`). |
| **G — Semigroup composition & supersession** | `qcore-schemas::composition` (`⊕` provenance-keyed component union, `QCore.AuditComposition`) and `qcore-schemas::contract_supersession` (`pinned_at` lock v3 supersession, `C06.6`). |

---

## 1. Original problem measured (2026-10-24 baseline)

Written 2026-10-24 from W48 implementation round 1 (rules 32–35). The [generalization plan](../../GENERALIZATION_PLAN.md) §3 set the crate-layer target; this plan added the rule-package layer beneath the C gateway and decoupled card workflows.

- **Implementation, contracts, and corpus were previously one knot**: `compiler/tests/qcore_contract_corpus.rs` held ~1,200 lines of per-file literals, `FRAMES` lived in Rust code, every verdict figure was a global corpus total that drifted across sibling branches, and cycle records bound directly to moving global counts.
- **Shared-file merge contention**: Across 300 pre-plan commits on `main`, `legacy_c.rs` and `proof/QCore.lean` changed in nearly every commit, `docs/agents/gateway.md` in 274, and `STATUS.md` / increment logs conflicted on every parallel wave.
- **Single-lock gate serialization & duplicated scripts**: A single global `flock` serialized 7 GB audits and 24-minute test suites together while ad-hoc `measure.py` / `mirror_inputs.py` scripts diverged across report folders, and literal checkout paths in compile databases broke when moving checkouts.

## 2. Architectural goal and preserved hard invariants

Every analyzer change has a **bounded, declared influence** (predicted rows, assumptions, and evaluation profiles), contracts and profiles are versioned data with lock files, and an isolated rule card is a self-contained package (rule crate + Lean module + `docs/agents/rules/<id>.md` + registry row + report).

Preserved without exception:
1. **Fail-closed (`Unknown`)**: a missing, disabled, or refusing rule package falls through to refusal/`Unknown`; no refactor flips `Unknown` to `Safe`.
2. **Parity before deletion**: a route leaves `legacy_c.rs` only after its rule package reproduces pinned verdicts byte for byte with the diff archived under `docs/reports/`.
3. **Intentional pins**: lock-file updates remain deliberate, reviewed diffs in the same commit (`providers/contracts.py sync`).
4. **Verdict contract, Lean parity, and downward dependency layering** ([generalization plan](../../GENERALIZATION_PLAN.md) §§2–3).

## 3. Seven architectural seams

### A. Decouple implementation from contracts and corpus

- **Contract bundles with lock files**: Each provider crate's `contracts/` carries a generated `contract-inventory.lock` (`providers/contracts.py sync`), checked by `qcore_contract_corpus.rs` and `contract-count-pins-from-locks`.
- **Contract knowledge as clauses**: Hardcoded function tables such as `FRAMES` move into `.qv` contract clauses (`C04.6.4`) read through the contract loader with archived parity runs.
- **Evaluation profiles as data**: `contract/evaluations/<name>/` binds route inputs, link maps, property selections, contract digests, and a `frozen@<rev>` or `live` policy (`scripts/evaluation_profiles.py`), with cycle bindings in `contract/evaluations/bindings.json`. Live profiles (`eight-home`, `fluent-bit-system`, `fluent-bit-tail-pause`, `twelve-home`) follow a periodic `REBASE-CORPUS` cadence while historical cycle profiles stay frozen.
- **Per-rule contribution ledgers & impact predictions**: Each switchable rule package records its per-profile delta (`TIP` vs. `TIP` with `--rules-off <id>`) rendered into [`docs/development/contributions.md`](../../development/contributions.md), and `quirin-audit measure --declared PREDICTION.md` checks declared vs. actual row movements.

### B. Gateway rule packages (code)

- **`crates/qcore-gateway-rule-api`**: Depends only on `qcore-schemas` and `qcore-contract`; defines position traits matching [`ROUTE-MAP.md`](../../reports/gateway-analysis-path-optimization/ROUTE-MAP.md), query/result envelopes, and reason-key reservations.
- **`crates/gateway-rules/rule-<id>-<name>/`**: Independent rule crates depending on `qcore-gateway-rule-api` (and `qcore-core` for pure kernel models), never on `qcore-c-gateway` or sibling rule crates.
- **`crates/gateway-rules/registry.toml`**: Declares `id`, crate, positions, consultation order, Lean modules, and rule doc; `scripts/gen_gateway_registry.py` generates the dispatch table verified against `ROUTE-MAP.md`.
- **Runtime switches (`QCORE_GATEWAY_RULES_OFF`)**: Disables listed admission packages fail-closed (`run.json` records active switches). Hold rules (`off = "not-switchable"`) refuse disabling (§6.10).

### C. Proof packages (Lean)

- Each rule or semantic card adds `proof/QCore/<Module>.lean` and `proof/Audit<Module>.lean`; `scripts/gen_lean_roots.py` generates `proof/QCore.lean` and `proof/AuditAll.lean` (enforced by `test_generated_roots.py`).

### D. Record packages (docs)

- Each numbered rule lives in `docs/agents/rules/<id>.md` and renders into `docs/agents/*.md` and `docs/agents/README.md` via `scripts/gen_rule_docs.py`.
- Card integration metadata uses schema `quirin-amendments/2` (`scripts/apply_amendments.py`), updating the current increment file, in-place `status_edits` in `docs/STATUS.md`, `named_followups`, and `contributions.md`.

### E. Elastic gate

- **Scoped card gate**: `scripts/gate_scope.py --base <rev>` selects affected packages, test targets, Lean audits, Python checks, and conditional provider checks (`docs/agents/validation.md#scoped-card-gate`).
- **Full landing gate**: Run once per landing on the merged commit (`gate_scope.py --full --landing` / `scripts/land.py`).
- **Named resource slots**: `scripts/gate_slot.py` manages `rust-suite`, memory-admitted `audit`, `lean`, `short`, and `build` classes.

### F. Measurement as a tool

- **`quirin-audit measure`** (`crates/quirin-audit-plugin-measure`): Out-of-process subcommand plugin running behind the plugin firewall, producing `native-comparison` artifacts, per-rule contribution ledgers, and `PREDICTION.md` checks over canonicalized input paths and published baselines (`scripts/land.py publish`).

### G. The semigroup integration interface

Two distinct algebraic compositions govern results and routes:

1. **Across results (commutative integration semigroup `⊕`, `qcore-schemas::composition`, `QCore.AuditComposition`)**: Each component binds `(unit, row key, property, profile, provenance, verdict tuple, reason leaves)`. Provenance-keyed union is associative, commutative, and idempotent on identical bytes; published verdicts fold via `Safe < Pending < Unknown < Unsafe` while unioning assumptions and reasons. Conflicting bytes under one provenance key retain both components and join the row with `Unknown` (`composition-conflict`).
2. **Within one unit's analysis (ordered route consultation)**: Packages at a route position consult in deterministic registry order; first admission or hold wins, and refusal is the identity.
3. **Contract bundle supersession (`qcore-schemas::contract_supersession`, `C06.6`)**: Across bundles declaring `(library, function)`, a strictly newer reviewed `pinned_at` stamp (lock v3) supersedes an older one (retaining the superseded digest in report metadata); equal stamps with differing text, missing/malformed stamps, or unpinned declarations withhold the function as `Unknown` (`contract-conflict`).

---

## 4. Standard isolated-card workflow

1. Author the rule/feature crate, `proof/QCore/<Module>.lean` + `Audit<Module>.lean`, `docs/agents/rules/<id>.md`, registry entry (running `gen_lean_roots.py`, `gen_rule_docs.py`, `gen_gateway_registry.py`), and `docs/reports/<card>/PREDICTION.md`.
2. Implement and validate unit/Lean checks (`cargo test -p <crate>`, `gate_lean.py`).
3. Measure profile deltas and contribution ledgers via `quirin-audit measure --declared PREDICTION.md` when corpus/system/slice verdicts can move.
4. Run `python3 scripts/gate_scope.py --base <sha>` and commit `gate.json`, `README.md`, and `integration-amendments.json` (`apply_amendments.py --check`).
5. The integrator merges, runs `apply_amendments.py`, executes the full landing gate and `scripts/land.py check` / `publish`.

---

## 5. Historical phase table (all phases completed)

| Phase | Cards | Standing |
|---|---|---|
| **P0 — Infrastructure** | `MEASURE-TOOL`, `GATE-SLOTS`, `GATE-SCOPE`, `AUDIT-CANONICAL-PATHS` | **Completed** |
| **P1 — Decoupling data** | `AUDIT-COMPOSITION`, `CONTRACT-SUPERSESSION`, `CONTRACT-LOCKS`, `EVALUATION-PROFILES`, `LEAN-ROOTS-GENERATED`, `RULE-DOCS-GENERATED`, `AMENDMENTS-SCHEMA` | **Completed** |
| **P2 — Rule API & pilots** | `GATEWAY-RULE-API`, `RULE-SWITCHES`, `CONTRIBUTION-LEDGERS`, `FRAMES-AS-CLAUSE`, `RULE32A-PILOT`..`RULE37-PILOT` (+ `rule-38`) | **Completed** |
| **P3 — Operations workflow** | `docs/agents/operations.md` and `validation.md` card/landing workflow | **Completed** |

---

## 6. Recorded design decisions (2026-10-24 through 2026-10-31)

1. **Contract pins as lock files**: Approved; literal count tables read generated locks, and contract edits carry reviewed lock diffs in the same commit.
2. **Cycles as frozen evaluation profiles**: Historical cycles stay frozen at their last measured revision; live Fluent Bit profiles (`eight-home`, `fluent-bit-system`, `fluent-bit-tail-pause`, `twelve-home`) re-base periodically (`REBASE-CORPUS`).
3. **Runtime rule switches (`QCORE_GATEWAY_RULES_OFF`)**: Approved for admitting rule packages.
4. **Scoped gate per card, full gate per landing**: Approved (`GATE-ELASTIC`, `docs/agents/validation.md`).
5. **Memory-budgeted audit slots**: Default 7 GiB planning budget (4/5 GiB selectable; pinned corpus admitted at 11 GiB), with `rust-suite` draining `audit` slots.
6. **Pilot-first rule extraction**: Rules `32a`, `33`–`37` migrated with byte-identical parity runs before broader extraction.
7. **Canonicalized unit paths**: Audit unit matching canonicalizes paths and mirrors mismatched compile-db roots cleanly.
8. **Out-of-process measurement plugin & file-granularity scoper**: `quirin-audit measure` runs behind the subcommand plugin firewall; `gate_scope.py` defaults to `--reader-granularity file`.
9. **Semigroup integration & `pinned_at` supersession (9a)**: Approved; provider locks carry reviewed `pinned_at` stamps over a rolling three-revision Fluent Bit window.
10. **Hold rules are non-switchable (`GATEWAY-RULE-API-HOLD-OFF`, `RULE32A-PILOT`, `QCore.CallerFrameHoldSwitch`)**: Because disabling a hold rule would either *widen* admissions (violating Hard Invariant 1) or misattribute an underlying contract's outside callers, hold packages set `off = "not-switchable"`, refuse `QCORE_GATEWAY_RULES_OFF` lists naming them, and carry no switch-measured contribution ledger.

## 7. Permanent boundaries

- Parity runs remain mandatory whenever moving an existing route, table, or pin out of `legacy_c.rs`.
- Route consultation order is semantic and guarded by `ROUTE-MAP` parity tests; never conflate intra-run ordered route consultation with cross-run commutative `⊕` report composition.
- Freezing a historical cycle profile is a records policy, not new verification evidence; committed-equals-generated checks remain enforced in Python and Rust gates.
