# AUDIT-IR: bounded component execution and 6–7 GB memory budget

> [!NOTE]
> **Current status (7 of 8 slices implemented; parent `AUDIT-IR-BUDGET` 6.0 GB target remains open).** Iterative SCC planning, versioned `.qc` CFG fragments, unit-scoped Clang/environment lifetimes, borrowed certificate admission, bounded worker windows + cgroup headroom sampling, disk-spooled Clang AST transport, and parked retained phase products have **all landed**. A full default `quirin-audit current` run completed without swap or OOM at **6,985,576,448 bytes** whole-cgroup peak ([working-set report](../reports/archive/language-and-tooling/audit-ir-working-set/README.md)), ~14.4 MB below the 7.0 GB ceiling; meeting the **6.0 GB ordinary target** with cold/warm margin across corpus, system, and slice selections (`AUDIT-IR-BUDGET`, `W45-CHECKPOINT-JOBS-MEMORY`) remains open.

## 1. Slice execution summary

| Slice | Purpose | Standing & evidence |
|---|---|---|
| **AUDIT-IR-PLAN** | Interned, non-recursive SCC planner in `qcore-c-gateway` driving W4 summary order | **Completed** — [audit-ir-plan](../reports/archive/language-and-tooling/audit-ir-plan/README.md) (exact SCC parity verified on all directed graphs ≤4 vertices + production routes) |
| **AUDIT-IR-FRAGMENTS** | Versioned unit manifest & atomic content-addressed `.qc` (`quirin-audit-ir-cfg/1`) capture/verify | **Completed (opt-in `QCORE_AUDIT_IR_DIR`)** — [audit-ir-fragments](../reports/archive/language-and-tooling/audit-ir-fragments/README.md) |
| **AUDIT-IR-LIFETIMES** | Unit-scoped Clang AST/layout cache eviction, `Rc`/`Arc` reader leases, single active contract-settlement owner | **Completed** — [audit-ir-lifetimes](../reports/archive/language-and-tooling/audit-ir-lifetimes/README.md) |
| **AUDIT-IR-ADMISSION** | Zero-clone borrowed certificate replay & cache release across current and pinned routes | **Completed** — [audit-ir-admission](../reports/archive/language-and-tooling/audit-ir-admission/README.md) (prior 4-job run hit 7 GB OOM, motivating `WORKING-SET`) |
| **AUDIT-IR-WORKING-SET** | Bounded in-flight task window, streaming JSON output, `malloc_trim(0)`, cgroup-v2 headroom sampling, `--memory-budget-mib` planner | **Completed** — [audit-ir-working-set](../reports/archive/language-and-tooling/audit-ir-working-set/README.md) (**6,985,576,448 B** peak, 0 OOM on 156 sources / 157 configs) |
| **AUDIT-IR-CLANG-TRANSPORT** | Seekable disk-spooled `.qc` transport for Clang AST dumps >1 MiB with Linux page-cache discard advice | **Completed** — [audit-ir-clang-transport](../reports/archive/language-and-tooling/audit-ir-clang-transport/README.md) (−486.5 MB / −8.25% peak on 30k-function disk-backed control) |
| **AUDIT-IR-RETAINED-PRODUCTS** | Transactional run-local `.qc` parking of phase report vectors and callback/pre-pass facts | **Completed** — [audit-ir-retained-products](../reports/archive/language-and-tooling/audit-ir-retained-products/README.md) (completes under 7.0 GB ceiling with 69 pressure events) |
| **AUDIT-IR-BUDGET** | Enforced child/component memory admission, final-row streaming, per-component body paging, and **≤6.0 GB target / ≤7.0 GB ceiling** across cold & warm corpus/system/slice runs | **Open** (`W45-CHECKPOINT-JOBS-MEMORY` registered in `acceptance_matrix.json`) |

---

## 2. Objective and non-negotiable acceptance budget

Make a cold, complete existing-C audit practical on an 8 GB Linux runner:
- **Ordinary execution target**: at most **6,000,000,000 bytes** (6.0 GB decimal, not GiB).
- **Hard whole-job ceiling**: at most **7,000,000,000 bytes** (7.0 GB decimal) across one dedicated cgroup (`memory.peak`, `memory.events`, including charged file cache and concurrent Clang/provider/worker child processes).
- **Scope**: Rust-based C auditing (including preparation when part of the advertised command; build/toolchain setup disclosed separately). Lean proof builds remain separate development validation.
- **Zero semantic loss**: No verdict, diagnostic, selected target, assumption, source identity, contract dependency, contract standing, or certificate replay check may be dropped or weakened to fit memory.

---

## 3. Architectural invariants of the IR and cache substrate

1. **Semantic working representation, not a new C compiler**: Reuses `LegacyCfg`, typed Clang AST readings, and existing contract/evidence schemas while keeping Quirin `.qr` compilation's `TypedCfg` distinct.
2. **Authenticated unit manifests & interning**: Symbols, types, contracts, clauses, and evidence objects intern once per identity domain using deterministic dense IDs; compiler provenance remains a declared premise.
3. **Component-order execution & leased lifetimes**: Strongly connected components execute callees before callers; obsolete ASTs, unselected contextual CFGs, and intermediate settlement environments drop as soon as no active lease or certificate replay requires them. Oversize components must fail closed with affected targets retained as `Unknown`, never silently truncated.
4. **Disposable, untrusted `.qc` persistence**: Content-addressed `.qc` objects (`≤64 MiB`, `≤32` nested IR instructions, atomic temp-sync + hard-link publication) never act as their own trust authority—stale, missing, or mismatched entries rebuild from authenticated source inputs or retain `Unknown`.
5. **Transitive invalidation**: Any change to source, compile flags, contracts, profile, policy, bounds, or checker identity invalidates transitive consumers; cache eviction alters cost only, never verdicts.

---

## 4. Remaining requirements to close `AUDIT-IR-BUDGET`

1. **Final-summary body paging & final-row streaming**: Release or page retained summary CFGs during replay, stream final report rows and sidecars without keeping full multi-phase vectors resident simultaneously, and bound child Clang/provider memory alongside worker threads (`W45-CHECKPOINT-JOBS-MEMORY`).
2. **Cold and warm cgroup acceptance**: Demonstrate `≤6,000,000,000` bytes target and `≤7,000,000,000` bytes ceiling across pinned corpus, configured `fluent-bit-system`, and `fluent-bit-tail-pause` selections (both cold and warm, recording elapsed wall time so serial slowdowns are visible) with archived full-record parity (custody, bounds, null, runtime, and consultation records) before adding a standard 8 GB CI regression job.
