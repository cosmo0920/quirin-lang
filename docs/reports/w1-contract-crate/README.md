# W1: contract-crate relocation

Date: 2026-09-06. Status: the W1 relocation step of the
[generalization plan](../../GENERALIZATION_PLAN.md) is executed. No analysis
logic changed, no verdict moved, and no fail-closed gate was touched; the
workspace now has six members with compiler-enforced downward layering.

## Why three crates, not one

`crates/qcore-contract` must sit *below* `qcore-compiler`: the legacy-C
certificate recheck (`check_legacy_safety_certificate`) calls
`contract_loader::check_source_assumptions`, and parameterizing that call
away would let a caller substitute a rubber-stamp loader — a fail-closed
violation. The loader in turn needs the `.qcore` parser, so the front end
had to move below the contract crate first. The measured dependency closure
of the front end is eight modules (3,727 lines) with zero edges back into
the rest of the crate and one mutual recursion (`frontend` ↔ `quant_index`),
which fixed the cut:

| Crate | Modules (moved verbatim) | Workspace deps |
| --- | --- | --- |
| `crates/qcore-core` (L1 seed) | `usage`, `ast`, `index` | none |
| `crates/qcore-frontend` (L1) | `lexer`, `parser`, `check`, `quant_index`, `frontend` | `qcore-core` |
| `crates/qcore-contract` (L1) | `contract_loader`, `contract_observation` | `qcore-core`, `qcore-frontend`, `qcore-schemas` |

`check` (the axis/dimension static environment) lives in `qcore-frontend`
rather than the core seed because its unit tests exercise it through the
parser; both placements satisfy §3.1's layering requirement. The remaining
semantic kernel migrates into `qcore-core` at the W5 consolidation.

`qcore-compiler` re-exports every moved module at its previous path
(`pub use qcore_frontend::frontend;` etc.), so no caller changed; 38 unit
tests and two `.jsonl` observation fixtures moved with their modules, and
the workspace total is unchanged at 408 tests.

## Deliberately deferred

- **`chunk_contract` stays in `qcore-compiler`.** It consumes
  `ChunkSourceSummary`/`ChunkFrameSummary`/`ChunkSourceField` from the
  ChunkIO provider module, and `ChunkSourceSummary::instantiate` is semantic
  logic over `chunk_lifecycle` state. Moving the interpreter now would
  invert the layering through the provider. The W5 task 3 split (summary
  data types to `qcore-schemas`, `instantiate` split off like
  `check_source_assumptions` was) unblocks the move; recorded in the crate's
  `lib.rs` and the plan status.
- The other W1 remainders (leftover literal contract sites, richer
  observation/lifecycle clause vocabulary) are unchanged by this relocation.

## Guard

`compiler/tests/workspace_layering.rs` now pins the full dependency matrix:
schemas and core depend on nothing in the workspace, frontend only on core,
contract only on core/frontend/schemas, the grab-bag on the four extracted
crates, the CLI on the grab-bag. Any new upward edge fails the test.

## Standing gate

- `cargo test --workspace`: 408 passed, 0 failed, across 20 suites
  (compiler lib 323 + integration 45; qcore-core 15; qcore-frontend 19;
  qcore-contract 4; qcorec CLI 2).
- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `reevaluation_probe` exit 0; `reevaluation_followup` exit 0.
- `lake build`: 51 jobs, success (no semantic change; Lean untouched).
- Contract corpus counts, submodule pin, and Cycle 20 plugin tests
  unchanged and green.
