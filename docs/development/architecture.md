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
| `crates/qcore-schemas/` | L0 published schema surface (Rust API **17.0.0**): plugin protocol v1 types, resource-contract data model, mechanism vocabulary, source-identity records, `reason` category vocabulary. Serde types only; a test forbids workspace dependencies |
| `crates/qcore-c-gateway/` | L2 existing-C analyzer, custody summary inference, Clang discovery, typed-AST transport and opaque Boolean-field source imports; depends only on contract/schemas plus serialization, and on the `qcore-core` kernel for the `disjoint_footprint` bit-field storage-unit decision (wave 48, CG13, rule 10w). Of the 36 modules under `crates/qcore-c-gateway/src/`, **ten** are named by a standing constraint: `legacy_c.rs` (generic unrestricted-C checker — Clang gateway, CFG, resource abstract interpretation, certificate recheck; contracts enter via `ResourceContracts`; since GATEWAY-ANALYSIS-PATH-OPTIMIZATION the entry points, unit driver, per-function facts, `Parser` and `parse_statement` stay in it and its other parts are move-only child modules: `legacy_tokens.rs` (token stream, conditional scan, definition scan), `legacy_condition_routes.rs`, `legacy_return_routes.rs`, `legacy_classify.rs` (`classify_simple*`, the statement classifier), `legacy_token_shapes.rs`, `legacy_cfg_builder.rs`, `legacy_transfer.rs` (abstract state and transfer) and the census-only `legacy_parser_census.rs`; the statement and return fallback orders are the two tables of `legacy_route_table.rs`, generated since GATEWAY-RULE-API from `crates/gateway-rules/registry.toml`, whose rows read the run-time switches of `gateway_rules.rs`, and every position's consultation order is mapped in [the route map](../reports/gateway-analysis-path-optimization/ROUTE-MAP.md)), `legacy_summary.rs` (W4 interprocedural custody summary inference: SCC-ordered derivation, validation by re-analysis, declared-contract precedence — constraint 4), `typed_source.rs` (opaque typed C path import and source def/use), `typed_source_nodes.rs` (wave 48, CG01: route T's operator and literal closure and the per-consumer admission of rule 10r), `elaboration.rs` with `legacy_elaboration.rs` (wave 48, CG02: the common expression elaborator, its order model and its route-G fallback hooks, rule 10s), `legacy_location.rs` (wave 27, the presentation-only preprocessed → original `file:line` map — constraint 12a), `legacy_neutral.rs` (C03-NFR, the neutral-element route for undeclared recursive components — constraint 4), `legacy_joined_provenance.rs` (wave 31, the conditional-provenance pre-pass, extended to cyclic CFGs by a greatest fixed point at wave 35 — constraint 10b) with `provenance_rounds.rs` (wave 37, that pre-pass's proved round bound — constraint 10b), `legacy_declared_abi.rs` and `legacy_declared_modes.rs` (the two declared-contract-versus-C readers constraint 8 forbids moving), and `legacy_return_call.rs` (wave 34, the modeled call in return position — constraint 5b). This list is derived from the constraints, not from the directory: a module the constraints do not name is not thereby unimportant |
| `crates/qcore-gateway-rule-api/` | The C gateway's rule-package interface (GATEWAY-RULE-API, pluggable-cards seam B): one trait per [route-map](../reports/gateway-analysis-path-optimization/ROUTE-MAP.md) position over the host's own vocabulary, answers that are admit-with-obligations or refuse, the first-answer-wins `consult`, the reason-key reservation, the registry model and the run-time switch `QCORE_GATEWAY_RULES_OFF` (an unknown id or a hold is refused). Depends on `qcore-schemas` only (`qcore-contract` permitted; a test holds the set); a rule package depends on it, never on the gateway |
| `crates/gateway-rules/` | `registry.toml`, the gateway rule registry (id, crate, placements with consultation order or amended predicate, Lean modules, rule document, reserved reason keys, off behaviour). `scripts/gen_gateway_registry.py` generates `qcore-c-gateway`'s `gateway_rule_registry.rs` and the tables of `legacy_route_table.rs` from it; the Python suite checks committed equals generated and every ordered placement against the frozen route map. Rule crates `rule-<id>-<name>/` arrive with their migration cards: `rule-35-literal-decay/` (RULE35-PILOT, the first) holds rule 35's two arms over the API's `LexicalHost` and `TypedHost` capabilities; the gateway hosts them in `rule_hosts.rs` through the registry switch. A rule crate depends on the API (and `qcore-core`) only (`workspace_layering.rs`); `rule-37-result-constructors/` (RULE37-PILOT; rule 37's typed `sizeof` exemption and `asserted_allocation` premise arms, asked by the gateway's `result_constructor_rule.rs`); `rule-36-exact-formal-return/` (RULE36-PILOT) holds rule 36's lexical candidate, source conjunction, return route (the first `return-route` package) and exit classifier, hosted by the gateway's `legacy_exact_formal_return.rs`; `rule-33-removed-selection/` (RULE33-PILOT; rule 33's frame word shapes, caller expansion and selection decisions, asked by the gateway's `legacy_selection.rs` through the interface's `StatementExpansion` and `EntryRow`); `rule-34-element-destroyer/` (RULE34-PILOT; rule 34's transfer arm `destroyer-order`, asked by the gateway's `transfer_rules.rs` at the API's transfer position, route map §10, whose guard refuses an unordered destroyer with 34 off); `rule-32a-caller-frame-hold/` (RULE32A-PILOT; rule 32a's token hold `caller-frame-hold`, the first hold package and the first reserved reason key, asked by the gateway's `legacy_caller_frame_hold.rs` in all three token pipelines; not switchable) |
| `crates/qcore-verifier-host/` | L2 generic registry (0.4.0), source-scope resolution, bounded process transport, execution profiles, v1/v2 firewall and composition; depends only on frontend/schemas workspace crates |
| `crates/qcore-verification/` | Trusted adapter (0.4.0): explicit-registry source driver, generic model/source imports, core-derived obligations, lifecycle/observation witness replay and scope dispatch; depends on core/frontend/contract/schemas/gateway/host, never providers |
| `crates/quirin-source-analysis/` | Pinned C source-analysis adapters shared by the compiler facade and audit runner; keeps checked source summary and certificate constructors private, with gateway and verification machinery in their existing lower crates |
| `crates/qcore-audit/` | L3 configured audit driver: compile-database ingestion, compile-task identity, source selection (by located path: `input_paths.rs`), per-unit projection and reports over W2 profiles/W1 contracts; the configured current-revision route (`current_route.rs`, `current_cli.rs`, `pinned_route.rs`, `pinned_cli.rs`), contract selection by provider crate on the published load path (`providers.rs`) and the contracts-only evidence mode (`evidence.rs`) and its no-build route (`no_build.rs`), and contract revision modes: source identities (`c_identity.rs`), the pin evaluation and report section (`contract_revision.rs`), `quirin-audit pin` (`contract_pin.rs`) and `quirin-audit compare` (`revision_compare.rs`), and external `quirin-audit NAME` subcommands (`subcommand_plugins.rs`: `quirin-audit-NAME` on a declared search path, out of process, through the checker-plugin firewall). Links no provider module |
| `crates/quirin-audit-plugin/` | The SDK for external `quirin-audit` subcommand plugins (W45-AUDIT-SUBCOMMAND-PLUGINS): the versioned JSON request/response protocol, `run`, response builders, artifact verification, a test harness, ordered JSON text output (`json_text`), and the example plugin `quirin-audit-example`. Generic; links no core analysis crate (`qcore-schemas` for SHA-256 only) |
| `crates/quirin-audit-plugin-libevent/` | The first subcommand plugin, binary `quirin-audit-libevent`: `quirin-audit libevent prepare` prepares the pinned `libevent-core` configured-audit inputs from `third_party/libevent` (the byte-for-byte port of the retired Python preparation script). Depends on the SDK only |
| `crates/quirin-audit-plugin-measure/` | Subcommand plugin `quirin-audit-measure` (MEASURE-TOOL, pluggable-cards seam F): `quirin-audit measure` builds and runs measurement sides (revision snapshots at one fixed path, checkouts, binaries, published baselines), compares their rows through the `qcore_schemas::composition` sidecars into one native-comparison record with the declared-versus-measured check, and `measure publish` writes baselines ([usage](audit-subcommand-plugins.md#the-measure-plugin)). Depends on the SDK and `qcore-schemas` only |
| `crates/qcorec/` | CLI binary `quirin` (crate `qcorec`) plus its process-level tests; the explicit contract-load-path check calls the lower-layer `qcore-contract` loader directly |
| `compiler/src/` facades | `compiler/src/verifier_plugin.rs` is the compatibility facade for the host and first-party adapters; `compiler/src/verifier_plugins/{libevent,chunkio}/` retain first-party modules and reexport source adapters from `quirin-source-analysis` (Libevent/Monkey, ChunkIO); `compiler/src/external_{integration,source}.rs` are compatibility facades plus Monkey-specific composition |
| `quirin-audit pinned` | The explicit historical Fluent Bit corpus arbiter; its orchestration is in `qcore-audit`, with source-analysis adapters in `quirin-source-analysis` |
| `providers/` | Separate thirteen-member Cargo workspace (five `qcore-provider-*` packages, six `quirin-provider-*` libraries, the `conformance` harness and, since W45-USER-CONTRACT-PLUGINS, the `quirin-provider-plugin` SDK for user contract crates and checker plugins, which owns no contracts), excluded from the core workspace. Artifact crates depend only on `qcore-schemas`, serialization and `quirin-provider-shared`. Each provider owns its supplied `.qcore` files in `contracts/`; `providers/package.py` builds, verifies and archives the executable and contract bundle |
| `providers/quirin-provider-*/`, `contract/corpus.json` | Shared library crates: `quirin-provider-{libc,cfl,mpack,msgpack,fluent-bit}` each own one library's shared contracts (data only; libc only in `-libc`); `quirin-provider-shared` (lib `quirin_provider_shared`) holds the providers' protocol runtime, schemas-only capability evaluators and projection fixtures, the `language/`/`library/` policies, and re-exports the five as one bundle. The providers depend on it; no core crate may depend on any of them. Shared contracts and the validation index of shared/provider-owned sources; [current counts](../STATUS.md) are pinned by each crate's generated `contract-inventory.lock`, read by `compiler/tests/qcore_contract_corpus.rs`. No duplicate provider contracts |
| `cycle19-resource-contracts.json` | Repository-root resource selection; all module paths remain relative without traversal |
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
  `qcore-audit`, whose package and library names are unchanged — and
  `quirin-audit current`, the configured current-revision route, with resource
  contracts selected by provider crate (`--provider NAME[:MODULE]` or a JSON
  `providers` array; `NAME` selects the crate's checked resource view) or by
  manifest, and an opt-in contracts-only mode that loads every crate's view
  when nothing is selected (`crates/qcore-audit/README.md`). Inputs are prepared by
  `quirin-audit prepare` (`crates/qcore-audit/src/prepare.rs`: the Clang
  CMake build and the Monkey inputs, as the steps `prepare clang-build` and
  `prepare monkey`; `prepare --run` chains `quirin-audit current`) and, for
  `libevent-core`, the external subcommand `quirin-audit --plugin-path DIR
  --output OUT libevent prepare` (`crates/quirin-audit-plugin-libevent`).
- External subcommands: `quirin-audit [--plugin-path DIR]... --output DIR NAME
  ARGS...` runs `quirin-audit-NAME` from the declared plugin search path (never
  `PATH`; built-ins always win), and `--list-plugins` lists them
  ([writing one](audit-subcommand-plugins.md); contract spec C11.2).
- Contracts only without build settings: `quirin-audit contracts`
  (`crates/qcore-audit/src/no_build.rs`) audits explicit files or a `*.c` walk
  with the loaded contracts, the user's `-I`/`-D`/`--std` and stated defaults,
  and no compilation database; every verdict is relative to the recorded
  assumptions (`QCore.NoBuildScope`).
- Contract revision modes: `quirin-audit pin --source-root DIR` writes or
  checks the provider crates' `contracts.lock` files after human review, and
  `quirin-audit compare --before REPORT --after REPORT` reports the change
  between two runs (contract spec C06.2, `QCore.ContractRevisionMode`).
- Bounded native composition: `quirin-audit observed-wait CONFIG.json` joins
  selected Monkey QCore contracts, imported C source, independently loaded
  build identities and one complete timed-wait observation. See the
  [configuration and evidence boundary](../reports/cycle18-observed-integration/README.md).
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
`docs/QTT_ALIGNMENT.md` for the language surface, `contract/README.md`
for contract corpus conventions, `docs/CYCLE19_BUILD_INPUTS.md` for the
persistent audit setup (it replaces ephemeral `/tmp` build paths), and
`docs/design/` for design notes.
<!-- preserved:further-reading:end -->
