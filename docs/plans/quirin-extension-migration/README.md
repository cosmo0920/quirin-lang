# Quirin file-extension migration (`EXT-01`–`EXT-04`)

> [!NOTE]
> **Current status (completed active migration).** Compatibility readers (`EXT-01`, [report](../../reports/extension-compatibility/README.md)) and physical repository-wide renames (`EXT-02` and `EXT-03`, [combined report](../../reports/extension-physical-migration/README.md)) are **complete and merged**, followed by `#import` runtime resolution ([report](../../reports/runtime-source-imports/README.md)). Zero `EXT-*` items remain in `acceptance_matrix.json#named_followups`. Legacy `.qcore` and `.qcorelib` spellings remain accepted as compatibility aliases with stderr warnings; undated final removal of legacy reader support (`EXT-04`) awaits downstream readiness.

## 1. Canonical suffix scheme (in force)

| Meaning | Current suffix | Legacy alias | Classification and trust rule |
|---|---|---|---|
| Quirin checkpoint object | `.qo` | `.qo` | Binary audit checkpoint; schema/version migration is independent of source suffixes. |
| Quirin formal contract source | `.qv` | `.qcore` | **Contract mode (`module contract;`)**. Identifies contract declarations, not a proof certificate or proved C body. |
| Quirin runtime / library source | `.qr` | `.qcore` | Executable (`module executable;` or unmarked) and helper library (`module library;`) source files. |
| Quirin native ABI manifest | `.qrlib` | `.qcorelib` | Native-library ABI manifest grammar (`v1` declared / `v2` symbol-generated); grants no unspoken contract or linkage authority. |

The file extensions do not alter grammar, checking obligations, provider trust, or verdicts. A wrong-extension / wrong-mode combination refuses (`Q403` / `Q039`) rather than silently selecting a weaker checker, and legacy `.qcore`/`.qcorelib` inputs emit an explicit compatibility warning without changing safety verdicts.

---

## 2. Migration slices and completion standing

| Slice | Scope | Standing |
|---|---|---|
| **EXT-01** — Compatibility readers & warnings | Parser-derived mode classification; `.qv`, `.qr`, and `.qrlib` reader support; duplicate legacy/new alias refusal; immediate stderr compatibility warnings. | **Completed** — [EXT-01 report](../../reports/extension-compatibility/README.md) |
| **EXT-02** — Contract-mode `.qv` rename | Atomic rename of live and archived contract modules, `#include` targets, provider exports, bundle selections, and lock/inventory paths with zero verdict drift. | **Completed** — [EXT-02/03 report](../../reports/extension-physical-migration/README.md) |
| **EXT-03** — Runtime `.qr` & ABI `.qrlib` rename | Rename of classified runtime sources, examples, CLI help, test fixtures, and `.qrlib` manifests, preserving emitted C bytes and diagnostics. | **Completed** — [EXT-02/03 report](../../reports/extension-physical-migration/README.md) |
| **EXT-04** — Legacy alias retirement | Eventual removal of `.qcore` / `.qcorelib` fallback readers once external consumers require no compatibility window. Warning emission from `EXT-04` is already active since `EXT-01`. | **Undated option (not scheduled)** — legacy warnings and collision/wrong-mode refusal tests remain enforced in CI. |

---

## 3. Historical planning record (2026-09-27, base `2e44e963`)

The inventory and design notes below preserve the original pre-migration baseline (`2e44e963`) and the user's 2026-09-27 scope override requiring historical/archived inputs and runnable references to migrate to `.qv`, `.qr`, and `.qrlib` while attributing recorded measurements to their original commits.

- **Pre-migration baseline inventory (`2e44e963`):** 616 tracked `.qcore` files (223 in `compiler/`, 1 in `contract/`, 46 in `crates/`, 182 in `docs/reports/`, 41 in `examples/`, 123 in `providers/`) and 5 `.qcorelib` files (621 total classified inputs: 195 `.qv`, 421 `.qr`, 5 `.qrlib`).
- **Consumer boundaries coordinated during migration:**
  - *Frontend*: `crates/qcore-frontend/src/contract_inclusion.rs` included-module suffix checks and module-mode staging.
  - *Contracts*: `crates/qcore-contract/src/{bundle,load_path_manifest,contract_views}.rs`, provider exports, and `providers/contract-load-path.json`.
  - *Auditing*: `crates/qcore-audit/src/contract_pin.rs`, source-contract/profile loaders, and `crates/quirin-source-analysis/`.
  - *Provider packaging*: `providers/contracts.py`, `providers/package.py`, `contract-inventory.lock` pins, and reproducible package digests.
  - *Runtime & native linkage*: `quirin check`/`emit`, `.qrlib` `from MODULE` contract slice references, and tracked-file test inventories.
