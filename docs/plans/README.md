# Planning and workstream index

Use this page to distinguish **live open engineering queues** from **completed or halted planning archives**. Planning documents propose architecture, scope, and sequencing; they do not establish implementation or safety claims.

## 1. Canonical source of open work

- **`named_followups` in [`docs/reviews/r0_3/acceptance_matrix.json`](../reviews/r0_3/acceptance_matrix.json)** is the sole machine-checked registry of open follow-ups and candidate cards.
- Every entry names the report (`docs/reports/<card>/README.md`) or plan that owns its specification and discharge condition.
- Before selecting or dispatching a card, search its exact ID in `acceptance_matrix.json` and check [`docs/STATUS.md`](../STATUS.md) for current measured baselines. Neither a historical plan row nor a mention inside an agent rule establishes that an item is still open.

---

## 2. Active engineering queues (partially executed; open slices remain)

| Plan / Queue | Current standing | Remaining open scope |
|---|---|---|
| [Gate and measurement time reduction](gate-time-reduction/README.md) | **9 of 13 requirements discharged** (`R1`–`R3`, `R6`–`R7`, `R11`–`R13`: precise scoper, lock-derived count pins, slot-wait quota exclusion, starvation deadlines, FIFO ticket queues, `short` and `build` lanes) | `R4` (`GATEWAY-TESTS-CROSS-TREE-INCLUDES`), `R5` (`W44-TEST-CRITICAL-PATH-ROUTE-PARITY`), `R8` full incremental baseline composition (`MEASURE-INCREMENTAL-BASELINE-COMPOSITION`), `R9` (`GATE-SCOPED-RUN-UNEXERCISED`), `R10` (`CORPUS-ROUTES-OUT-OF-HOST`) |
| [AUDIT-IR memory & component execution](../development/audit-ir-card.md) | **Slices `PLAN`, `FRAGMENTS`, `LIFETIMES`, `ADMISSION`, `WORKING-SET`, `CLANG-TRANSPORT`, and `RETAINED-PRODUCTS` landed** (default current run completed at 6.99 GB cgroup peak without OOM) | `AUDIT-IR-BUDGET`: enforced memory admission, component body paging, and measured **6.0 GB target / 7.0 GB ceiling** across cold and warm corpus/system/slice runs (`W45-CHECKPOINT-JOBS-MEMORY`) |
| [Clang gap card queue (`CG00`–`CG30`)](clang-gap-cards/README.md) | **All 31 initial cards (`CG00`–`CG30`) executed bounded slices** across waves 47–48 (55 `cg*` reports under `docs/reports/`) | Bounded post-card residuals (`CG00-G-CORRECTION-RESIDUALS` through `CG30-DIALECTS-FLAGS-TYPES`) registered in `named_followups` |
| [Unsupported coverage: typed access paths & callee closure](unsupported-coverage/README.md) | **`P0` soundness repair, `P1` census, `P2` statement fallback, `P3` expression positions, `P4` rounds 2–4, `P5` first slice (`CG11`), and `P6` alias census + `AG2`/`K4`/`K5` slices landed** | Full route-replacement parity on `P2`/`P3` (`W46-ACCESS-PATH-PARITY`), `P4` bottom-up residual plan (`W48-BOTTOM-UP-RESIDUAL-PLAN`), broader `P5` owned-field pointer stores, and remaining `P6` alias cards |
| [Fluent Bit system coverage queue](fluent-bit-system-coverage/README.md) | **`P0` system census, `P1.1` bounded backlog custody & fault injection, `P1.3` singleplex retry & fault injection, `P2` engine loop census + rule 44 measure, and `W47` loop-owner / condvar-join rules (`10p`/`10q`) landed** | `P1.2` backlog byte/count accounting, remaining `P1.1`/`P1.3` production backlog & retry residuals (`W45-FB-BACKLOG-*`, `FB-RETRY-*`), and `P2` slice measurement (`W45-FB-ENGINE-PROGRESS-MEASURE-SLICE-RUN`) |

---

## 3. Architectural roadmaps

| Document | Role and current standing |
|---|---|
| [Post-Cycle-9 roadmap (`Cycles 10–20` & `W0–W7`)](../ROADMAP_AFTER_CYCLE_9.md) | Milestone definitions from Cycle 10 through Cycle 20. Cycles 10–17 are complete at their bounded milestones; Cycle 18 has bounded implementation with aggregate acceptance pending; Cycle 19 (Fluent Bit custody/lifecycle) and Cycle 20 (verifier plugin ecosystem, v1 `Safe` firewall active) remain active. |
| [Checker generalization plan (`W0–W7`)](../GENERALIZATION_PLAN.md) | Architectural blueprint (2026-09-06) that decomposed the original monolithic compiler into the layered workspace (`qcore-schemas` L0 → `qcore-core` / `qcore-frontend` / `qcore-contract` L1 → `qcore-c-gateway` / `qcore-verifier-host` L2 → `qcore-verification` / `quirin-source-analysis` / `qcore-audit` / `qcorec` L3) and stood up the 50/50 mapped CT harness. Retained for crate-layering invariants (§§2–3). |

---

## 4. Completed or halted plan archives

These directories preserve the design and dispatch records of completed migrations or explicitly halted tracks. Do not dispatch work from them without checking `named_followups`.

| Archive | Status and completion record |
|---|---|
| [Pluggable, isolated implementation cards](pluggable-cards/README.md) | **Completed (Phases P0–P3).** Delivered `contract-inventory.lock` per provider, `evaluation_profiles.py`, `gen_lean_roots.py`, `gen_rule_docs.py`, `apply_amendments.py`, `gate_scope.py`, `gate_slot.py`, `quirin-audit measure`, `AUDIT-COMPOSITION`, `CONTRACT-SUPERSESSION`, `qcore-gateway-rule-api`, `CONTRIBUTION-LEDGERS`, and rule crates `rule-32a` through `rule-38`. Retained as the normative reference for seams A–G and user decisions 1–10. |
| [Quirin file-extension migration (`EXT-01`–`EXT-04`)](quirin-extension-migration/README.md) | **Active migration completed (`EXT-01`–`EXT-03`).** Live and archived sources use `.qv` (contracts), `.qr` (runtime/executable/library), and `.qrlib` (native ABI manifests) ([report](../reports/extension-physical-migration/README.md)). Legacy `.qcore`/`.qcorelib` remain warned compatibility aliases; undated final removal (`EXT-04`) awaits downstream readiness. |
| [Runtime source imports (`#import`)](runtime-imports/README.md) | **Completed.** Relative `.qr` module resolution (`#import "./helper";`) and `#import std.*;` migration landed ([report](../reports/archive/language-and-tooling/runtime-source-imports/README.md)). |
| [Contract hypothesis graphs & symbolic premises](contract-hypothesis-graph/README.md) | **Graph v1–v4 and symbolic formula v1 (`SP1`–`SP10`) landed; guarded contract adoption halted.** Graph versions 1–4 (`C04.18.1`, `18.2`, `18.7`, `18.8`) and symbolic premise formulas (`C04.18.9`, prose bans `QF001`/`QF002`) are implemented; native guarded contract adoption is **halted at user request** ([supplemental card](../reports/guarded-contract-adoption/SUPPLEMENTAL_CARD.md)). |
| [Cycle-19 custody B & W card series](cycle19-custody-cards/README.md) | **Executed (wave 42).** `B01`–`B04` expression-position calls ([report](../reports/b-series-integration/README.md)) and `W42` priority slices ([report](../reports/w42-priority-integration/README.md), [chunk profile](../reports/w43-chunk-source-profile/README.md)) completed; superseded for future C-gateway planning by `clang-gap-cards` and `unsupported-coverage`. |
| [Halted-worktree restart & context plan (2026-09-13)](resume-2026-09-13/README.md) | **Completed historical checkpoint (waves 31–40).** All four recovered worktree cards (`CT-WIRE-LANE-3`, `W31-CORPUS-CONTRACTS-2`, `CT03-PROOF-ALPHABET`, `WRITE-LEND-TASKS`) and context-packet routing (`scripts/task_context.py`, `DOCS-STRUCTURE-COMPRESSION`) landed. |
