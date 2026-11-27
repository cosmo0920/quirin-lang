# Gate and measurement time reduction: requirements and status

> [!NOTE]
> **Current status (9 of 13 requirements discharged; 4 open / partially open).** Originally recorded on 2026-11-05 (`GATE-TIME-REQUIREMENTS` against `96a3efd3b`) and extended with `R11`–`R13` on 2026-11-06/08. Current authoritative gate rules live in [`docs/agents/validation.md`](../../agents/validation.md). Open items remain tracked in `acceptance_matrix.json#named_followups`.

## 1. Execution status matrix (`R1`–`R13`)

| Req | Summary | Tracker ID(s) | Current standing |
|---|---|---|---|
| **R1** | Exact run-time reader & directory-walker attribution (`gate_scope_walkers.json` + `strace` census) | `GATE-SCOPE-RUNTIME-READS-HEURISTIC` | **Discharged** — [gate-scope-precise-selection](../../reports/gate-scope-precise-selection/README.md) |
| **R2** | Test-only edits (`tests/*.rs` and `#[cfg(test)]` lib modules) stay scoped even in `FULL_CRATES` | `GATE-SCOPE-FULL-CRATES-TEST-ONLY`, `GATE-SCOPE-LIB-UNIT-TEST-MODULES` | **Discharged** — [precise selection](../../reports/gate-scope-precise-selection/README.md), [lib unit modules](../../reports/archive/language-and-tooling/gate-scope-lib-unit-test-modules/README.md) |
| **R3** | Derive aggregate contract counts from provider `contract-inventory.lock` files instead of global Rust literals | `CONTRACT-COUNT-PINS-OUTSIDE-LOCKS` | **Discharged** — [contract-count-pins-from-locks](../../reports/archive/contracts-and-premises/contract-count-pins-from-locks/README.md) |
| **R4** | Move `qcore-c-gateway/tests/*.rs` cross-tree `include_str!` checks on `proof/` and `providers/` to a leaf test target | `GATEWAY-TESTS-CROSS-TREE-INCLUDES` | **Open** |
| **R5** | Select heavy corpus route-parity and composition-mutation binaries only when their seam changes | `W44-TEST-CRITICAL-PATH-ROUTE-PARITY` | **Open** |
| **R6** | Card-gate tolerance for card-added Lean audits (`QUIRIN_CARD_GATE_BASE`) + provider lock metadata check | `CARD-GATE-NEW-AUDIT-MATRIX-COUNT`, `GATE-SCOPE-PROVIDER-LOCK` | **Discharged** — [gate-scope-precise-selection](../../reports/gate-scope-precise-selection/README.md) |
| **R7** | `quirin-audit measure` excludes slot-wait time from `--timeout-ms` analysis quota | `MEASURE-SLOT-WAIT-OUTSIDE-QUOTA` | **Discharged** — [measure-quota-and-incremental](../../reports/archive/language-and-tooling/measure-quota-and-incremental/README.md) |
| **R8** | Incremental declared measurement over changed TU/contract/`.qo` digests composed via `⊕` | `MEASURE-INCREMENTAL-BASELINE-COMPOSITION` | **Partially implemented / Open** — initial mechanism in [measure-quota-and-incremental](../../reports/archive/language-and-tooling/measure-quota-and-incremental/README.md); full default replacement parity remains open |
| **R9** | Historical red-commit scoper replay + completed scoped landing proof | `GATE-SCOPED-RUN-UNEXERCISED` | **Partially complete / Open** — red-commit replays archived in R1/R2 reports; tracker remains open in matrix |
| **R10** | Extract corpus routes (`pinned_route.rs`, `current_route.rs`) into an out-of-process subcommand plugin | `CORPUS-ROUTES-OUT-OF-HOST` | **Open** (gated on `W45-SUBCOMMAND-CERTIFICATE-ROUTE` and `W45-SUBCOMMAND-HOST-REPORT-TRUST`) |
| **R11** | Prevent single-audit starvation: 2 h I/O stall deadlines, bounded `rust-suite` drains, live `gate_slot.py status` | `GATE-SLOT-ADMISSION-STARVATION` | **Discharged** — [gate-slot-admission-starvation](../../reports/archive/language-and-tooling/gate-slot-admission-starvation/README.md) |
| **R12** | FIFO ticket-ordered slot admission and uncapped slot-wait exclusion under separate `--wall-limit-ms` | `GATE-SLOT-FIFO-ORDER`, `MEASURE-SLOT-WAIT-EXCLUSION-CAP` | **Discharged** — [gate-slot-fifo-order](../../reports/archive/language-and-tooling/gate-slot-fifo-order/README.md) |
| **R13** | Dedicated `short` fast lane (≤30–60 min, ≤1–2 GiB) and unslotted `build` class outside `EXCLUSIVE` | `GATE-SLOT-SHORT-BUILD-WAIT`, `GATE-SLOT-SHORT-JOB-LANE` | **Discharged** — [short-job lane](../../reports/archive/language-and-tooling/gate-slot-short-job-lane/README.md), [builds unslotted](../../reports/archive/language-and-tooling/gate-slot-builds-unslotted/README.md) |

---

## 2. Invariants every gate/measurement optimization preserves

1. **Fail-closed & verdict contract** (`AGENTS.md` invariants 1 & 2): no gate or measurement shortcut may publish a verdict that a full run would not.
2. **Replay before narrowing**: any narrower test selection must replay historical red-commit diffs and prove zero missed failing units before becoming the default.
3. **Parity before deletion** (invariant 3): moving corpus routes out of the host (`R10`) requires archived byte-for-byte verdict parity.
4. **Intentional pins** (invariant 5): per-file `contract-inventory.lock` pins and submodule commits remain enforced; `R3` moved only *how* aggregate counts are derived, not whether contract changes require reviewed lock updates.
5. **Full landing arbiter**: the integrator's per-landing gate remains the full suite whenever analyzer, gateway, or contract semantics change.

---

## 3. Detailed specification of remaining open requirements (`R4`, `R5`, `R8`–`R10`)

- **R4 (`GATEWAY-TESTS-CROSS-TREE-INCLUDES`)**: Relocate Lean-correspondence and provider-text `include_str!` tests out of `crates/qcore-c-gateway/tests/*.rs` into a leaf test package so editing a `proof/QCore/*.lean` or `providers/**/*.qv` file selects only Lean / provider checks + the leaf target rather than triggering full `qcore-c-gateway` test builds.
- **R5 (`W44-TEST-CRITICAL-PATH-ROUTE-PARITY`)**: Gate the heavy route-parity binaries (`unsupported_close_route_parity`, `file_close_route_parity`, `filesystem_chunk_close_route_parity`) and `qcore-verification` composition-mutation binaries on changes to their route/composition seams rather than every edit in their crate.
- **R8 (`MEASURE-INCREMENTAL-BASELINE-COMPOSITION`)**: Re-audit only translation units whose source, contract, or `.qo` input digests differ from the published baseline, composing unchanged baseline components via `⊕` (`qcore-schemas::composition`), and verify exact report equality against full runs across multiple landings before making incremental mode the default.
- **R9 (`GATE-SCOPED-RUN-UNEXERCISED`)**: Complete end-to-end validation of scoped card runs alongside full historical red-commit replay coverage for `R4`/`R5`.
- **R10 (`CORPUS-ROUTES-OUT-OF-HOST`, longer term)**: Move `pinned_route.rs`, `current_route.rs`, and corpus source-analysis adapters behind an out-of-process `quirin-audit` subcommand plugin once `W45-SUBCOMMAND-CERTIFICATE-ROUTE` (trusted-core witness/certificate replay for plugin claims) and `W45-SUBCOMMAND-HOST-REPORT-TRUST` (authenticated host report bindings) land with audited Lean parity.

---

## 4. Historical baseline measurements (2026-11-05/08 incidents)

- **Baseline `96a3efd3b` (2026-11-05)**: Full landing gate 49 min (Rust suite 43 min 21 s including slot wait, providers 3 min 18 s, Python 67 s, Clippy 53 s, Lean 23 s, fmt 6 s, probes 1 s); concurrent 3-profile `quirin-audit measure` runs took 3 h 58 min (`CFL-MPACK r2`) and 6 h 23 min (`LIBC-GAP`) including slot waits.
- **Starvation & ordering incidents (2026-11-06/08)**: A stalled `current` experiment spun 10.3 h without I/O (`R11`), non-FIFO `flock` wakeup delayed a queued measurement 23 h until the capped wait exclusion killed it (`R12`), and a 3 min `prepare clang-build --link` waited 5 h behind audits (`R13`)—all resolved by the `gate_slot.py` deadlines, FIFO ticket queue, `short` lane, and `build` class documented in [`docs/agents/validation.md#gate-slots`](../../agents/validation.md#gate-slots).
