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
