# Checker generalization plan

> [!NOTE]
> **Architectural authority & historical status.** Originally authored on 2026-09-06 against the pre-split 23.8k-line monolithic `compiler/` crate. All foundational workstreams (`W0`–`W7`) have **executed** (decomposing the repository into the layered workspace below, establishing `.qv` contract bundles with lock files, standing up `quirin-audit`, and mapping **50/50** CT conformance cases), extended further by [`docs/plans/pluggable-cards/README.md`](plans/pluggable-cards/README.md). Sections **§2 (Invariants)** and **§3 (Layered Workspace Target)** remain active architectural constraints enforced by `compiler/tests/workspace_layering.rs` and [`docs/agents/operations.md`](agents/operations.md). For current architecture and measurements, see [`docs/development/architecture.md`](development/architecture.md) and [`docs/STATUS.md`](STATUS.md).

## Current execution summary (`W0`–`W7`)

| Workstream | Target | Current standing |
|---|---|---|
| **W0 — Workspace layering** | Extract `qcore-schemas` (L0) and `qcorec` CLI (L3); enforce downward-only dependencies | **Completed** (`workspace_layering.rs` enforces L0 → L1 → L2 → L3 + rule packages) |
| **W1 — Contracts as data** | Total-or-reject contract loader (`qcore-contract`); eliminate Rust contract literals | **Completed** (`.qv` corpus across 11 locked provider crates + user plugin crates) |
| **W2 — Profiles as data** | Versioned source-set profiles (`contract/profiles/`), CMake compile-db & preprocessor transport | **Completed for bounded profiles**; evaluation profiles managed via `contract/evaluations/` |
| **W3 — Obligations & dispatch** | Core-derived obligations, `qcore-verifier-host`, evidence-v2 witness replay, permanent v1 `Safe` refusal | **Bounded implementation active**; general C lifecycle source derivation remains open |
| **W4 — Custody summary inference** | SCC-ordered interprocedural summary inference & independent certificate rederivation (`qcore-c-gateway`) | **Implemented & extended** (`Out`, `Stored`, `Removed`, `Reallocated`, countdown/threshold recursion, `CG01`–`CG30` rules) |
| **W5 — Configured audit & providers** | `qcore-audit` (`quirin-audit`), `quirin-source-analysis`, reproducible `providers/*` packages, rule crates | **Implemented**; legacy Cycle-19 routes remain retained until full G4 parity (`REEVALUATION_SPEC.md` §9) |
| **W6 — Generic CT01–CT50 corpus** | Manifest-keyed source/C conformance harness (`docs/reevaluation/ct_manifest.json`) | **50/50 mapped** (48 distinct fixtures; 5 case-bound Lean proofs; 10 Linux/macOS sanitized-native rows; mapping ≠ full conformance) |
| **W7 — Hygiene** | Lean 4.35 CI gate (`check_lean_audits.py`), diagnostic namespaces, routed `docs/agents/` & `docs/STATUS.md` | **Active continuous policy** (`512+` Lean audits; `Q7xx` / `QP***` / `QI***` / `QF***` namespaces) |

---

## 1. Architecture review at the 2026-09-06 baseline

*(Retained as historical context explaining why `W0`–`W7` were sequenced as below.)*

At the 2026-09-06 baseline (`compiler/src` = 23,804 lines in a single monolithic crate):
- **Generic kernel (~14,000 lines)**: Cycle 5–17 semantic passes (usage, ownership, quant indices, typed errors, error regions, CFG, weak memory, concurrency, lowering) and `legacy_c.rs` had zero third-party coupling.
- **Under-connected substrate**: Declarative contracts, the Cycle-20 plugin protocol (`verifier_plugin.rs`), `LibraryMechanismSummary`, and `CT01`–`CT50` (then 0/50 mapped) existed without data-driven loaders or process boundaries.
- **Over-fitted integration (~5,300 lines)**: Hardcoded `AuditedLibrary` enums, upstream substring matchers (`external_source.rs`), Libevent/Monkey/ChunkIO name tables, and a 966-line audit example compiled directly inside the trusted crate.

---

## 2. Invariants that every workstream must preserve

1. **Fail-closed everywhere.** `Unknown`, never optimistic fallback, for missing/unloadable contracts, unmatched profiles, unsupported constructs, provider absence/failure/mismatch, and unproven call effects. No workstream may convert an `Unknown` to `Safe` as a side effect of refactoring.
2. **Reevaluation gate ordering** ([`REEVALUATION_SPEC.md`](REEVALUATION_SPEC.md) §9): G1 certificate containment before expanding provider coverage; Cycle 19's existing analysis routes are removed only after G4 parity is recorded.
3. **Parity before deletion** (Cycle 20C rule): each extraction must reproduce the previous pinned verdicts, coverage tables, and diagnostics before the hardcoded path is removed, with the comparison archived under `docs/reports/`.
4. **Verdict contract**: every result carries `(artifact, property, semantic profile, scope, bounds, assumptions, evidence version)` over `Safe < Pending < Unknown < Unsafe`; projection percentage is never presented as safety percentage; unknowns and `Pending` stay in the coverage denominator.
5. **Standing per-change gate**: workspace tests (`--no-fail-fast`), `cargo fmt --check`, Clippy `--all-targets -- -D warnings`, Lean audits (`check_lean_audits.py`, placeholder-free, no project axioms), Python tests, and the reevaluation probe examples (`reevaluation_probe`, `reevaluation_followup`) keep passing (see [`docs/agents/validation.md`](agents/validation.md)).
6. **Trusted input stays declared.** Contract and profile files are *trusted assumptions*, and certificates must record which ones were assumed. Moving a table from Rust into `.qv` or JSON data makes it declarable, diffable, and per-source-set—never automatically proved.
7. **Pluggability direction of dependencies.** New and moved code must respect the downward layering of §3: providers and subcommand plugins depend only on the published schema/plugin surface, never on core internals; the core never depends on any provider; the CLI and audit driver depend on hosts/registries, not on individual third-party provider internals.

---

## 3. Pluggability target: layered workspace architecture

### 3.1 Target crate layout (live workspace)

Dependencies point strictly downward (`compiler/tests/workspace_layering.rs`):

```text
crates/qcore-schemas               # L0: published, versioned portable data surface
  verdict lattice & composition (⊕), diagnostics & reason categories,
  obligation inventory, evidence/witness schemas (v1 refusal + v2),
  plugin protocol frames, profile/manifest formats, contract & supersession
  data models, source/build/target identities. Serde types only; zero analysis logic.

crates/qcore-core                  # L1: trusted semantic kernel
  usage, ownership, quant indices, typed errors, error regions, CFG,
  weak memory, execution coverage, reclamation, concurrency, custody algebra,
  event reactor, stdlib/buffer/string/socket/http/float models, pure checkers.

crates/qcore-frontend              # L1: .qr / .qv front end, elaboration & whole-program driver
  lexer, structural/formal clause parsers, runtime #import inspector,
  closed index resolver, route elaborators.

crates/qcore-contract              # L1: .qv contract loader, inclusion/inheritance,
  hypothesis graphs, bounds/null/cell/callback loaders & observation interpreters.

crates/qcore-gateway-rule-api      # L1.5: gateway rule position traits & envelopes
  (depends only on qcore-schemas and qcore-contract; seam B of pluggable-cards).
crates/gateway-rules/rule-*        # Pluggable C-gateway rule packages (depend on
  qcore-gateway-rule-api and optional qcore-core; never on qcore-c-gateway).

crates/qcore-c-gateway             # L2: existing-C analyzer, Clang discovery,
  typed_c_ast, expression elaborator, typed places, W4 summary inference,
  bounds/null/numeric/atomic/callback routes, rule-registry host.

crates/qcore-verifier-host         # L2: provider registry, discovery, process isolation,
  execution quotas, certificate firewall, and composition orchestration.

crates/qcore-verification          # L2.5: trusted integration adapter above L1 + L2
  explicit-registry source driver, obligation derivation, witness replay,
  scope dispatch, typed source-contract replay (zero provider crate dependencies).

crates/quirin-source-analysis      # Shared pinned ChunkIO & Libevent source adapters
  above verification and C gateway (no compiler, audit, or provider dependency).

crates/qcore-audit                 # L3: `quirin-audit` orchestration & routes
  compile-db ingest, pin/lock verification, AUDIT-IR component execution,
  pinned/current/configured routes, sidecar & component report emission,
  subcommand plugin host (`crates/quirin-audit-plugin*`).

crates/qcorec                      # L3: `quirin` CLI binary (`check`, `emit`, `build`, `plugins`).

providers/*                        # Out-of-tree provider workspace (`providers/Cargo.toml`)
  qcore-provider-{libevent,chunkio,cmetrics,ctraces,librdkafka} and
  quirin-provider-{libc,fluent-bit,monkey,cjson,sqlite,shared,openssl,libcurl,plugin},
  each owning `.qv` contracts + generated `contract-inventory.lock`.

contract/                          # Evaluation profiles (`evaluations/`), contributions,
  and source-set profiles (`profiles/`).
proof/                             # Lean 4 (`v4.35.0`) models (`QCore/*.lean`) & audits (`Audit*.lean`).
```

### 3.2 Migration strategy

1. **Extract L0 (`qcore-schemas`) and L3 CLI (`qcorec`) first (`W0`)**, turning `qcore-compiler` into a shrinking compatibility facade.
2. **Land every new capability directly in its target crate** (`W1` → `qcore-contract`, `W3` → `qcore-verifier-host`, `W4` → `qcore-c-gateway`, `W5` → `qcore-audit` / `quirin-source-analysis` / `providers/*`, gateway rules → `crates/gateway-rules/*`). Never add new logic to `qcore-compiler`.
3. **Archive parity before route deletion** (Hard Invariant 3).

---

## 4. Workstreams (`W0`–`W7`)

### W0 — Workspace layering: extract the schema surface and the CLI
- **Standing**: Executed ([W0 report](reports/archive/language-and-tooling/w0-schema-crate/README.md)). Extracted `qcore-schemas` and `qcorec` and installed the downward-only workspace dependency guard (`workspace_layering.rs`).

### W1 — Contract loader: make `.qv` (formerly `.qcore`) the single source of contracts
- **Standing**: Executed ([loader parity](reports/archive/cycles-18-19/cycle19-contract-loader/README.md), [crate extraction](reports/archive/early-waves/w1-contract-crate/README.md), [literal migration](reports/archive/early-waves/w1-literal-sites/README.md), [clause vocabulary](reports/archive/early-waves/w1-clause-vocabulary/README.md)). Total-or-reject `resource_contract_v1` loading, `#include` / `contract inherit` (`C15`), per-provider `contract-inventory.lock` files, and `pinned_at` supersession (`C06.6`) live in `crates/qcore-contract`.

### W2 — Source-set profiles and build transport as data
- **Standing**: Executed for bounded profiles and CMake build transport ([profile manifests](reports/archive/early-waves/w2-profile-manifests/README.md), [provider discovery](reports/w2-provider-discovery/README.md), [source-profile boundary](reports/archive/early-waves/w2-source-profile-boundary/README.md), [preprocessor transport](reports/w2-preprocessor-transport/README.md), [Clang CMake build](reports/w2-clang-cmake-build/README.md), [Boolean-field profiles](reports/archive/early-waves/w2-boolean-field-profiles/README.md), [Monkey typed profile](reports/archive/early-waves/w2-monkey-typed-profile/README.md)). Configured audits preserve include classes, macro ordering, and persistent `quirin-audit prepare clang-build` provenance.

### W3 — Core obligation generation and verify-scope dispatch
- **Standing**: Bounded dispatch and v2 witness replay implemented ([evidence v2](reports/archive/early-waves/w3-evidence-v2/README.md), [verifier host](reports/archive/early-waves/w3-verifier-host/README.md), [source-field provider](reports/w3-source-field-provider/README.md)). Protocol v1 `Safe` proposals remain permanently refused; uncertified user/subcommand plugin claims yield `Pending` at best (`C11.1`–`C11.2`); general C lifecycle source derivation remains open.

### W4 — Interprocedural custody summary inference
- **Standing**: Implemented and continuously extended in `crates/qcore-c-gateway/src/legacy_summary.rs` ([initial W4](reports/archive/early-waves/w4-custody-inference/README.md), [extensions](reports/archive/early-waves/w4-custody-extensions/README.md), [removed](reports/archive/early-waves/w4-removed-custody/README.md), [out forwarding](reports/archive/early-waves/w4-out-forwarding/README.md), [contextual recursion](reports/archive/early-waves/w4-contextual-recursion/README.md), [unsigned countdown](reports/archive/early-waves/w4-unsigned-countdown/README.md), [nullable removal](reports/w4-nullable-removal/README.md), [immutable out aliases](reports/archive/early-waves/w4-immutable-out-aliases/README.md), [gateway crate](reports/archive/early-waves/w4-gateway-crate/README.md)). Summaries validate via body re-analysis and independent certificate rederivation; declared contracts take precedence (`SummaryConflict` on disagreement).

### W5 — Provider extraction and orchestration-as-configuration
- **Standing**: Configured `quirin-audit` orchestration, reproducible `providers/*` packages, CLI plugin verification, generic reactor/custody kernel split, and shared `quirin-source-analysis` are implemented ([configured audit](reports/w5-configured-audit/README.md), [provider packages](reports/w5-provider-packages/README.md), [CLI packages](reports/w5-cli-packages/README.md), [generic reactor](reports/archive/early-waves/w5-generic-reactor/README.md), [configured process audit](reports/w5-configured-process-audit/README.md), [driver consolidation](reports/archive/early-waves/w5-generic-driver-consolidation/README.md), [CMetrics provider](reports/w5-cmetrics-unit-provider/README.md)). Full provider semantic parity and legacy Cycle-19 route retirement remain gated on G4 parity.

### W6 — Generic acceptance corpus (`CT01`–`CT50`)
- **Standing**: Harness stood up ([W6 harness](reports/archive/early-waves/w6-ct-harness/README.md)) and expanded across sequential, task, mutex, epoch, buffer, string, socket, cell, and C-twin lanes to **50/50 mapped cases** in `docs/reevaluation/ct_manifest.json` (48 distinct source fixtures, 5 case-bound Lean proofs, 10 gated Linux/macOS sanitized-native rows; see [`docs/STATUS.md`](STATUS.md)). Mapping tracks scenario coverage by stage and does not claim full language conformance.

### W7 — Hygiene: names, diagnostics, Lean CI, and documentation routing
- **Standing**: Implemented and maintained ([Lean CI](reports/archive/early-waves/w7-lean-ci/README.md), [generic Lean names](reports/archive/early-waves/w7-generic-lean-names/README.md), [diagnostic attribution](reports/w7-diagnostic-attribution/README.md), [documentation routing](reports/archive/language-and-tooling/docs-structure-compression/README.md)). `Q700`–`Q703` belong to core `PluginVerification`; Libevent uses provider-local `qcore.libevent.L00`–`L12`; `AGENTS.md` routes to `docs/agents/` and `docs/STATUS.md`.

---

## 5. Sequencing summary

```text
W0 schema crate + CLI split
        │
W1 contracts-as-data (.qv + locks) ──┬─▶ W4 custody inference ──┐
W2 profiles-as-data                ──┤                          ├─▶ W5 provider crates/processes,
W3 obligations + v2 dispatch       ──┘                          │     quirin-audit, rule packages
W6 CT01–CT50 corpus (50/50 mapped, stage-tracked) ──────────────┘
W7 hygiene, Lean CI & doc compaction (continuous)
```

For post-`W0–W7` card isolation and C-gateway coverage queues, see the [planning index](plans/README.md).

## 6. Permanent non-goals of generalization

- Never promotes any pinned translation unit toward `Safe` by refactoring alone.
- Never opens Cycle 19 fail-closed gates without per-path identity imports and required evidence.
- Never treats `.qv` contract files or profile manifests as proved C bodies—they remain declared assumptions recorded on every dependent verdict.
- Never claims OS-level memory/network sandboxing of external provider processes beyond enforced process-group and quota boundaries.
