# Architecture and code entry points

Structural map retained from the migration checkpoint. Current measurements
are in [STATUS.md](../STATUS.md); a reference below to the agent ledger or a
"ledger row below" resolves there. The numbered constraints resolve through
[the rule router](../agents/README.md), not this architecture page.

Current reference, routed by [AGENTS.md](../../AGENTS.md). Rule numbers are stable.
Paths in backticks are repository-relative. Read every applicable rule,
including cross-topic rules, before editing; the index is not a substitute.

<!-- preserved:architecture:start -->
## What this project is

QCore-C is a research language for quantitative types, owned resources, typed
recoverable errors, checked cleanup, and checked concurrency, lowering to C11.
It also ships a conservative checker for *existing* C source (the
"unrestricted-C gateway"). Normative specification: `docs/QCORE_SPEC_r0_3.md`
and `docs/QCORE_CONTRACT_SPEC_r0_3.md`. Current consumer grammars are in
`docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md`; implementation acceptance is tracked
in `docs/reviews/r0_3/acceptance.json`. Revision 0.2 remains historical; broad
parser/model support is not full source-language conformance. Work proceeds in
numbered bounded "cycles" recorded under `docs/cycles/`.

## Repository map

| Path | Contents |
|---|---|
| `compiler/` | Rust library crate `qcore-compiler` (lib `qcore_compiler`, 0.4.0): parser, checker, reference semantics, C backend, gateway/host compatibility APIs, first-party providers. Still the L1/L2 grab-bag; decomposition target in `docs/GENERALIZATION_PLAN.md` §3 |
| `crates/qcore-core/` | L1 kernel: quantitative types, ownership/error/cleanup/CFG semantics, concurrency/translation models, chunk lifecycle, generic library/container/reference-count and reactor/descriptor custody, checked task IR/reference execution and C11 task emission. Also `os_endpoint`, `chunk_integrity`, `capacity_boundary`, `mutex_invariant`. No runtime or workspace dependencies |
| `crates/qcore-frontend/` | L1 front end: `lexer`, `parser`, `check`, `quant_index`, `frontend` elaboration and source lowering. Depends on `qcore-core` only |
| `crates/qcore-contract/` | L1 contract machinery: total-or-reject loader, observation and declared chunk-path policy evaluation. Depends on core/frontend/schemas |
| `crates/qcore-schemas/` | L0 published schema surface (Rust API **14.0.0**): plugin protocol v1 types, resource-contract data model, mechanism vocabulary, source-identity records, `reason` category vocabulary. Serde types only; a test forbids workspace dependencies |
| `crates/qcore-c-gateway/` | L2 existing-C analyzer, custody summary inference, Clang discovery, typed-AST transport and opaque Boolean-field source imports; depends only on contract/schemas plus serialization. Of the 36 modules under `crates/qcore-c-gateway/src/`, **ten** are named by a standing constraint: `legacy_c.rs` (generic unrestricted-C checker — Clang gateway, CFG, resource abstract interpretation, certificate recheck; contracts enter via `ResourceContracts`), `legacy_summary.rs` (W4 interprocedural custody summary inference: SCC-ordered derivation, validation by re-analysis, declared-contract precedence — constraint 4), `typed_source.rs` (opaque typed C path import and source def/use), `legacy_location.rs` (wave 27, the presentation-only preprocessed → original `file:line` map — constraint 12a), `legacy_neutral.rs` (C03-NFR, the neutral-element route for undeclared recursive components — constraint 4), `legacy_joined_provenance.rs` (wave 31, the conditional-provenance pre-pass, extended to cyclic CFGs by a greatest fixed point at wave 35 — constraint 10b) with `provenance_rounds.rs` (wave 37, that pre-pass's proved round bound — constraint 10b), `legacy_declared_abi.rs` and `legacy_declared_modes.rs` (the two declared-contract-versus-C readers constraint 8 forbids moving), and `legacy_return_call.rs` (wave 34, the modeled call in return position — constraint 5b). This list is derived from the constraints, not from the directory: a module the constraints do not name is not thereby unimportant |
| `crates/qcore-verifier-host/` | L2 generic registry (0.4.0), source-scope resolution, bounded process transport, execution profiles, v1/v2 firewall and composition; depends only on frontend/schemas workspace crates |
| `crates/qcore-verification/` | Trusted adapter (0.4.0): explicit-registry source driver, generic model/source imports, core-derived obligations, lifecycle/observation witness replay and scope dispatch; depends on core/frontend/contract/schemas/gateway/host, never providers |
| `crates/qcore-audit/` | L3 configured audit driver: compile-database ingestion, compile-task identity, source selection, per-unit projection and reports over W2 profiles/W1 contracts. Links no provider module |
| `crates/qcorec/` | CLI binary `quirin` (crate `qcorec`) plus its process-level tests |
| `compiler/src/` facades | `compiler/src/verifier_plugin.rs` is the compatibility facade for the host and first-party adapters; `compiler/src/verifier_plugins/{libevent,chunkio}/` are the in-process first-party providers (Libevent/Monkey, ChunkIO); `compiler/src/external_{integration,source}.rs` are compatibility facades plus Monkey-specific composition |
| `compiler/examples/cycle19_fluent_bit_libraries_audit.rs` | Production audit orchestration for the pinned Fluent Bit corpus |
| `providers/` | Separate six-member Cargo workspace (five `qcore-provider-*` packages plus the `conformance` harness), excluded from the core workspace. Artifact crates depend only on `qcore-schemas` and serialization; `providers/package.py` builds, verifies and archives the installable packages |
| `contract/qcore/` | Declarative `.qcore` contract corpus (**88 modules** — the ledger row below is the one current figure); counts pinned by `compiler/tests/qcore_contract_corpus.rs` |
| `proof/` | Lean 4 executable core and safety theorems; `Audit*.lean` print axiom surfaces. The gateway-backing modules are **abstract rule theorems, not extraction of the Rust analyzer** and not whole-C correctness — every such module says so in its own docstring, no extraction, refinement or simulation theorem exists anywhere in `proof/`, and constraint 28 carries the two premises this leaves standing (`GATEWAY-LEAN-ABSTRACT-NOT-EXTRACTION`) |
| `docs/cycles/` | Per-cycle requirement/acceptance records (`CYCLE_2` … `CYCLE_20`) |
| `docs/reports/` | Archived evidence packages (pinned audits, observations, triage) — one directory per card |
| `docs/reevaluation/` | Raw audit logs, probe JSON, `ct_manifest.json` (CT01–CT50 tracking, schema 2) |
| `third_party/libevent` | Submodule pinned to the audited revision; a test fails if absent or moved |

<!-- preserved:architecture:end -->

<!-- preserved:entry-points:start -->
## Key entry points

- Library: `parse_source_program`, `elaborate_source`, `check_whole_program`,
  `compile_files` / `compile_files_with_registry`,
  `analyze_unrestricted_c11` / `validate_unrestricted_c11`.
- CLI (binary `quirin`, crate `qcorec`): `quirin check <file>`,
  `quirin emit -o out.c <file>`,
  `quirin plugins list|inspect <id>|verify <package>|enable|disable`.
- Configured audits: `quirin-audit CONFIG.json` — the binary of crate
  `qcore-audit`, whose package and library names are unchanged. Inputs are
  prepared by `scripts/prepare_cycle19_clang_audit.py`,
  `scripts/prepare_cmetrics_clang_audit.py` and
  `scripts/prepare_libevent_core_audit.py`.
- Wave close: `scripts/reconciliation_sweep.py` sweeps each merged card's
  integrator-addressed section into individually addressable items and
  `--check`s the close's own `docs/reports/wave<N>-close/reconciliation.json`
  for a verdict on every one; `--residuals` lists residual names the wave's
  reports introduced that no tracker mentions. It decides nothing — see
  `CLAUDE.md`'s close rule and `docs/reports/w32-reconcile-sweep/README.md`.
- Published examples: `examples/README.md` indexes one directory per family —
  `stdio`, `pipes`, `branching`, `extern`, `unsafe-linkage`, `threads`,
  `events`, `strings` (text reaching a stream), `sockets` (checked TCP/UDP over
  loopback, now including declared IPv4/IPv6 addresses), `http` (an executed
  HTTP/1.1 subset, beside the same bytes written by hand), `loops` and
  `records` (the first aggregate type; `EXAMPLES-AGGREGATES`, wave 28). Every
  program compiles through the real pipeline and is byte-compared against its
  reference execution by a workspace test. Since card `EXAMPLES-SUITE-WIRE` all
  **twelve** families are held to one standard by
  `compiler/tests/examples_suite.rs`, which reaches them by **directory scan**
  rather than a hand-kept list — so a new family is covered structurally rather
  than by an author's diligence — while `examples/{strings,sockets,http}` keep
  their own card test files' sanitized native runs unedited beside it
  (`STRING-SLICE-IO-EXAMPLE-SUITE`, `HTTP-V1-EXAMPLE-SUITE` and `LOOP-EXAMPLE`
  all closed there). The index also records the family's **spelling rule** —
  *pipe the flow, parenthesize the assembly* — under which `examples/events/`
  keeps its own standing pipes-only rule and `examples/{pipes,stdio}` stay as
  the deliberate piped/unpiped twins (`docs-compact-3`).
- Native threading examples: `examples/threads/README.md` (`cl.exe`);
  `build-windows.ps1 -Run` emits, builds and runs the copied-worker,
  owned-buffer and shared-reader examples into `target/thread-examples/`.
- Editor: `editors/emacs/qcore-mode.el` gives lexical `.qcore`
  highlighting; see `editors/emacs/README.md`. Highlighting does not imply
  executable or verified support.

<!-- preserved:entry-points:end -->

<!-- preserved:further-reading:start -->
## Where to read more

Beyond the pointers in the ledger above: `docs/QCORE_MODULE_MODES.md` and
`docs/QTT_ALIGNMENT.md` for the language surface, `contract/qcore/README.md`
for contract corpus conventions, `docs/CYCLE19_BUILD_INPUTS.md` for the
persistent audit setup (it replaces ephemeral `/tmp` build paths), and
`docs/design/` for design notes.
<!-- preserved:further-reading:end -->
