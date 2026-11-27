# Reports directory pruning, archive restructuring, and hygiene gate

## Summary and user decisions

`docs/reports/` had grown to **980 flat card directories**, **27,456 files**, and **1,578,444,926 bytes (`1,505.32 MiB`)**, of which all `README.md` narratives combined were `17.02 MiB` (`1.1%`) while unreferenced raw corpus dumps (`resource-diagnostics*`, `parity.jsonl*`, `diagnostics.json*`, `gate-scope.json`, `/raw/before/`, `/raw/after/`, `.prepared.c.gz`), per-step execution logs, and aborted attempt directories accounted for over `1.34 GiB`.

Under explicit user decisions on 2026-11-27 (`A1` artifact pruning overriding constraint 21 for unreferenced bloat, `A2` physical archive categorization for unpinned historical directories, and `A3` standing operational rule + automated `land.py check` hygiene gate):

1. **Unreferenced bloat pruned with cryptographic provenance (`A1`)**:
   - Pruned **9,279 unreferenced bulk/log/attempt files** totaling **1,408,527,233 bytes (`1,343.28 MiB`, an `89.1%` net reduction from `1,505.32 MiB` to `163.54 MiB`)** across **594** card directories.
   - Wrote deterministic `<report-dir>/pruned-artifacts.json` manifests (`schema: "quirin-pruned-report-artifacts/1"`) recording sorted `path`, `bytes`, and 64-hex `sha256` for every pruned file.
   - Retained 100% of `README.md` narratives, compact diffs/summaries, scripts, `.c`/`.h`/`.qr`/`.qv` source fixtures, `mutations/` harnesses, and mechanically test/ledger-pinned inputs (including `tests/support/*.rs` `include_str!` migration records).
2. **Two-tier directory architecture (`A2`)**:
   - Kept **270** mechanically referenced or active report directories (+ this report = **271** top-level directories) at `docs/reports/<card>/` (preserving every Rust test, `tests/support/` `include_str!` input, Python trace test, `scripts/gate_scope_walkers.json` entry, `read-census.json` path, `contract/evaluations/*` baseline pointer, `contract/contributions/*` ledger pointer, and `reconciliation.json` `verify` file path).
   - Moved **710** historical unpinned report directories into 8 era and subsystem buckets under `docs/reports/archive/`:
     - `early-waves` (`135`), `waves-20-39` (`30`), `cycles-18-19` (`24`), `waves-40-47` (`80`), `wave-48` (`37`), `gateway-and-clang` (`86`), `contracts-and-premises` (`68`), and `language-and-tooling` (`250`).
   - Rebased relative Markdown links and `docs/reports/<card>` references across **575+** Markdown documents, `docs/agents/rules/*.md` (regenerating `docs/agents/*.md` via `scripts/gen_rule_docs.py`), and `docs/reviews/r0_3/acceptance_matrix.json#named_followups` (`0` broken report `.md` links across `13,785` checked links).
3. **Indexing, reconciliation, landing lookup, and standing hygiene gate (`A3`)**:
   - Extended `scripts/build_docs_index.py` to index both `docs/reports/<card>/README.md` and `docs/reports/archive/<bucket>/<card>/README.md`, enforce unique card directory names across tiers, and generate `docs/reports/README.md` plus per-bucket `docs/reports/archive/<bucket>/README.md` hubs.
   - Updated `scripts/unpublished_landings.py` and `scripts/reconciliation_sweep.py` to resolve archived landing gate records (including historical pre-archive git commit trees) and archived card/close directories transparently.
   - Added `scripts/check_report_hygiene.py` (enforcing unique card names, `pruned-artifacts.json` integrity, no failed/attempt directories, no raw bulk dumps, and a `512 KiB` per-file ceiling outside frozen legacy allowlists) and wired it into `scripts/land.py check` alongside the standing rule in `docs/agents/operations.md`.

## Validation summary (Windows host)

- `python -X utf8 scripts/build_docs_index.py --check`: exit `0` (`10` index/hub files fresh across `981` card directories).
- `python -X utf8 scripts/gen_rule_docs.py --check`: exit `0` (`289` rule files match generated topic pages and router).
- `python -X utf8 scripts/check_report_hygiene.py`: exit `0`.
- `python -X utf8 scripts/land.py check`: exit `0`.
- Strict link & matrix source verification: `0` broken `acceptance_matrix.json` source paths; `0` broken report `.md` links across `13,785` checked Markdown links.
- Python unit suites (`test_report_hygiene`, `test_build_matrix_sections`, `test_reconciliation_sweep`, `test_ct_stage_summary`, `test_task_context`, `test_unpublished_landings.CardNames`, `test_unpublished_landings.Generation`, `test_gate_scope_census`) and targeted Rust report-reading suites (`qcore-compiler` `qr_print_route_census`, `chunk_lifecycle_coverage`, `ct_corpus`; `qcore-verification` `callback_task_handoff`, `ctraces_capability_boundary`, `wait_failure_paths`): exit `0`.
