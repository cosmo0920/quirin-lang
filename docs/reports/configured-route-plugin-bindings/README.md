# CONFIGURED-ROUTE-PLUGIN-BINDINGS

A fail-closed defect (rules 10h and 53), launched by the user on 2026-11-18.
It discharges the last amendment of the named follow-up
`W45-PLUGIN-ROUTE-COVERAGE`. Base `974434463`.

## 1. Prediction (committed before implementation)

**What changes.** The configured `quirin-audit CONFIG.json` route will run
the checker bindings of the user load path it resolves through the same code
as `current` and `contracts` (`user_plugins::run`, the report fragments'
links, `fold_unit` and `fold_dependencies`). A contract manifest of schema 2
whose `load_path` is a user load path will carry that load path's bindings on
every route that loads it through `ContractInput`. The pinned corpus route
loads its manifest directly and checkpoints fragments that cannot carry
links, so it will refuse a manifest whose load path carries a checker
binding.

**Rows that can move.**

* The fixture of `crates/qcore-audit/tests/library_version_hold_routes.rs`:
  on the configured route, with the declaration in force, `use_reset` moves
  `Safe -> Pending` (`plugin-claim-uncertified`), with one `plugin_links`
  entry and a `plugin_checks` section counting 1 Pending. The other four
  standings keep their status: a held call applies no contract, so no link
  folds onto it.
* The live and pinned routes: none. Every measured route loads a manifest
  (`cycle19-resource-contracts.json`, the profiles' manifests) whose
  `load_path` is the shipped `providers/contract-load-path.json`. The shipped
  load path has no extension and no checker binding (rule 53), so
  `user_plugins::run` returns `None`, no links are set, and every report and
  sidecar is byte-identical. The current route's code path is touched (the
  manifest input now reads its load path's extension), but only to find
  none there.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

## 2. The defect

`user_plugins::run` was called in two places: `CurrentAudit::with_probe_databases`
(behind `with_contracts`) and `NoBuildAudit::with_plugin_checkers`. Both set
the report's links, and every per-unit fragment comes from
`AuditReport::fragment`, which copies them, so `fold_unit` and
`fold_dependencies` join each bound callee's verdict into its callers.

The configured route (`run_config_with_jobs` in `crates/qcore-audit/src/lib.rs`)
resolved the same `ContractInput` but never called `user_plugins::run`, and it
built each unit's fragment with `AuditReport::new(&revision)`, which carries no
links. A bound user contract in force therefore published its caller from
the host analysis alone.

Implementation found a second instance of the same defect. A schema-2 contract
manifest names a `load_path`, and that can be a user load path:
`quirin-provider-x:module` references then load user modules. But
`ContractInput::manifest` always set `extension: None`, so no route ran the
bindings of a load path reached through a manifest: `current
--contract-manifest`, `contracts --contract-manifest`, the configured route's
profile-default manifest, and the pinned route's manifest argument. This is the
same shape the provider route lowers to (`ResolvedProviders` writes exactly
such a manifest), so only the input's provenance decided whether the bindings
ran.

Reproduced on base sources with this card's tests
([log excerpt](evidence/base-reproduction.txt)). In force, `use_reset` was
`Safe` with no `plugin_links` on the configured route with `providers`, on the
configured route with `library_providers`, and on `current
--contract-manifest`. Each failing assertion is the missing `plugin_checks`
count, and its context records status `Safe` and links `null`. The manifest
test stops at its first failure, so
`contracts --contract-manifest` and the configured profile-manifest run were
not separately reproduced. They read the same `ContractInput::manifest`.
`current` and `contracts` with `--provider` passed.

## 3. The change

* **Configured route** (`lib.rs`). Before any unit, `user_plugins::run` runs
  on `contract_input.extension()`, the same call `current` makes. The
  console prints `user_plugins::tally`, the report's links are set, and the
  `plugin_checks` section goes into `report.assumptions.json`. Both unit loops
  (whole run and per-library slices) now take `report.fragment()`. The fold is
  shared code (`AuditReport::record` → `fold_unit`,
  `projection::project_prepared_unit_into` → `fold_dependencies`). Parked
  fragments keep their links, because the links stay in the parked shell. Unit
  tallies count the folded status. Per-library slices run the union input's
  bindings once. A link only folds onto a contract that the slice's own
  environment applies.
* **Manifest inputs** (`qcore-contract` `contract_loader`). The manifest
  parser and entry resolution are factored out of
  `load_resource_contract_manifest_text_stamped`, which is otherwise unchanged.
  The new `resource_contract_manifest_load_path` returns the load path a
  manifest names, loaded exactly as the loader loads it.
  `ContractInput::manifest` keeps that load path's extension, so every
  `ContractInput` route runs a manifest's bindings through the same code as a
  provider selection. If the load path cannot be read, `ContractInput::load`
  returns that refusal before loading anything. No route therefore loads a
  manifest whose bindings it could not see.
* **Pinned route** (`pinned_route.rs`). `user_plugins::refuse_unrun_bindings`
  refuses a manifest whose load path carries any checker binding
  (`checker-bindings-not-run`, naming the bound `crate:module`s). It runs
  before any work for an explicit manifest, and again where the profile's
  default manifest is resolved. The route publishes nothing. The pinned route
  cannot run bindings: its group checkpoints refuse a fragment with links
  (`into_fragment_parts`). Making it run them is out of scope and not needed
  (section 5).
* **Rule 53** is amended in place, and the gateway topic page is regenerated.
  There is no new rule.
* **Lean.** No semantic change. The fold is `QCore.VerdictSemigroup`'s join,
  the firewall's uncertified `Safe` is `Pending`, and a binding never lifts a
  hold (`QCore.LibraryVersionRange`). The change applies those to more routes.
  No theorem is added or changed.

## 4. Route inventory

Every `quirin-audit` built-in (`subcommand_plugins::BUILTINS`), the
configuration route, the subcommand host and the library callers. Only two
places in `crates/qcore-audit/src` call a resource-contract manifest loader:
`ContractInput::load` and the pinned route.

| Route | Resolves a user load path? | Bindings | Test |
|---|---|---|---|
| `current` (`current_cli` → `CurrentAudit`; `prepare --run` chains into it) | `--load-path`/`--provider`, `library_providers`, `--contract-manifest` or the profile's schema-2 manifest | run (`with_probe_databases`) | `the_current_route_holds_by_reason_with_a_checker_binding`; `the_manifest_routes_run_the_bindings_of_the_load_path_they_name` (current) |
| `contracts` (`no_build`) | `--load-path`/`--provider`, all-loaded, `--contract-manifest` | run (`with_plugin_checkers`) | `the_contracts_route_holds_by_reason_with_a_checker_binding`; `the_manifest_routes_…` (contracts) |
| configured `CONFIG.json` (`run_config_with_jobs`) | `load_path` with `providers`, `library_providers` or contracts-only all-loaded; the profile's schema-2 manifest | **run (was: none)** | `the_configured_route_holds_by_reason_with_a_checker_binding`; `the_configured_route_runs_bindings_under_per_library_providers`; `the_manifest_routes_…` (configured, profile manifest) |
| `pinned` (corpus arbiter) | only a manifest (argument or profile default), never `--load-path` | **refused** (`checker-bindings-not-run`) | `the_pinned_route_refuses_a_manifest_whose_load_path_carries_bindings`; `shipped_manifests_carry_no_checker_binding`; `a_fragment_with_checker_links_is_never_checkpointed` |
| `measure` (external `quirin-audit-measure`) | none of its own: it runs `pinned` and `current --selection` of each side's binary on shipped profiles | inherits those routes | `shipped_manifests_carry_no_checker_binding` (its routes' manifests) |
| subcommand host (`quirin-audit NAME`, rule 10h) | passes `--load-path` to the plugin as an identity; host verdicts come from an `--audit-report` a route above wrote | none run; no row can be `Safe` from a claim (uncertified `Safe` is `Pending`) | `subcommand_plugins.rs`: `uncertified_claims_are_pending` |
| `checkpoint` (verify, compose) | none: recomposes checkpoints a route above wrote; current-route rows carry their links and the run carries `plugin_checks` | inherits; a pinned group checkpoint never holds links | `a_fragment_with_checker_links_is_never_checkpointed` |
| `compare`, `pin`, `plugin` | `pin` and `plugin views` read a load path | publish no verdict (two reports' diff, locks, views) | n/a |
| `atomic-publication`, `callback-task-handoff[-fixture]`, `cycle18-*`, `event-payload`, `native-platform-matrix`, `observed-wait`, `timeout-teardown`, `wait-failures` | none: no contract manifest or load path is read | n/a | none |
| gateway API callers (`qcore-audit` and `qcore-c-gateway` examples, library users) | they apply the environment they are given; five examples load a manifest directly | none run, no refusal | residual: `GATEWAY-API-USER-LOAD-PATH-BINDINGS` |
| `quirin check --contract-load-path` (`qcorec`) | reads a load path | validates contracts, publishes no safety verdict | n/a |

## 5. Figures

* **Fixture** (`library_version_hold_routes.rs`, eight tests,
  [log](evidence/tip-tests.txt)). The configured route moves `use_reset`
  **Safe → Pending** in force: one `plugin_links` entry (`Pending`) and
  `plugin_checks` counting 1 Pending. It now asserts exactly what `current`
  and `contracts` assert at all five standings. Changed stays `Pending`;
  out of range, unreadable and ambiguous stay `Unknown`, with no link. The
  same in-force move happens on the configured route with `library_providers`,
  on `current --contract-manifest`, on `contracts --contract-manifest` and on
  the configured route over the profile's manifest. Nothing else moves, and
  nothing moves toward `Safe` or `Unsafe`.
* **Measured routes.** The prediction holds by construction. The routes'
  manifests name the shipped load path, which has no extension
  (`shipped_manifests_carry_no_checker_binding` checks
  `cycle19-resource-contracts.json` and every profile's manifest). So
  `user_plugins::run` returns `None`, no link is set, the pinned refusal
  never fires, and reports and sidecars are byte-identical. This host has no
  prepared live-route inputs (`build/cycle19`, the `private/w45-fb` and
  `private/w47-tail` preparations) and no published baseline, so `quirin-audit
  measure --declared` was not run. The integrator's landing measurement
  (`land.py publish`) compares the three live routes against the baseline.

## 6. Residuals

* `GATEWAY-API-USER-LOAD-PATH-BINDINGS` (registered). Library callers of
  `load_resource_contract_manifest*`, such as the census and probe examples,
  load a user load path's modules without running or refusing the bindings.
  They are not `quirin-audit` verdict routes, but a caller that publishes
  verdicts from such an environment has the same gap.
* `MANIFEST-ROUTE-USER-OVERRIDE-CHECK` (registered). A provider selection
  refuses a user module that redeclares a shipped function without an
  explicit override (`user_plugins::check_overrides`). A hand-written schema-2
  manifest naming a user load path now runs its bindings, but it skips that
  check. Its duplicates are decided by contract supersession (C06.6) instead.
* The test comment's `CONFIGURED-ROUTE-CHECKER-BINDINGS` was never registered
  in the acceptance matrix. Removing the comment retires nothing.
