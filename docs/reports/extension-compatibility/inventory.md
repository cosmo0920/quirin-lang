# EXT-01 tracked-input inventory

Base `64ddac3f60fe9b848dd789e895d67b809cce93e2`. This inventory is classification evidence for the [extension plan](../../plans/quirin-extension-migration/README.md), not a rename or a safety/coverage measurement. The [prediction](inventory-prediction.md) was committed as `5bc31ceb` before the classifier was implemented.

Run `cargo run -q -p qcore-frontend --example extension_inventory > docs/reports/extension-compatibility/mapping.tsv` from this checkout to reproduce the mapping. The Rust tool reads `git ls-files --stage -z` rather than walking the working tree, records each index blob ID, and invokes the frontend's actual `parse_source_program` or core's `load_native_library_manifest`. The former returns `SourceModuleMode::{Contract,Executable,Library}` and defaults to executable for a valid module without a mode header. A valid contract inclusion source is recognized by the dedicated `scan_contract_inclusion` parser; the unresolved full source is *deliberately refused* by `parse_source_program` until its loader elaborates it. This syntax result proposes `.qv` but does not claim loader resolution or semantic acceptance. A full parse refusal records its exact parser message and a token-header-based candidate only; the `decision` column distinguishes it from a parsed mode. The path proposal for `library` is `.qr` because its known consumers invoke source `check` or mixed multi-file source checking, not the contract loader. The inventory never dispatches a checker based on extension.

| Parser result | Tracked inputs | Proposed suffix / limit |
|---|---:|---|
| Executable source | 407 | `.qr` |
| Contract source | 187 | `.qv` |
| Library source | 9 | `.qr`; all frozen probes, see below |
| Contract inclusion syntax | 6 | `.qv`; full parser refusal pending loader elaboration |
| Source parse refusal | 7 | Candidate in mapping only; no accepted mode |
| Native ABI manifest | 5 | `.qrlib` |
| **Total** | **621** | 616 `.qcore` plus 5 `.qcorelib` |

There are 182 frozen `docs/reports/` `.qcore` inputs and 439 other inputs. No tracked `.qv`, `.qr` or `.qrlib` file exists at this base, and proposed targets do not collide with each other. [mapping.tsv](mapping.tsv) accounts for every tracked input with old path, Git index blob ID, result, proposed path, decision and frozen flag. [frozen-archive.tsv](frozen-archive.tsv) separately enumerates every frozen input and its blob ID. These are Git content identities, not SHA-256 claims. All frozen files stay at their old paths and their bytes are untouched; their proposed paths are future naming labels, not rename instructions.

The nine `module library;` inputs are historical source probes: three `remaining-d01` multi-file dimension/axis inputs (including both argument orders and the good-axis control), `remaining-v03`'s `S26` unknown-result-type input, four `remaining-v04` probes (`P18` unused-module and `P25`/`P26`/`P27` multi-file attribution), and `remaining-v07`'s `a11` unchecked-contract control. Their source parser mode is Library; the archived scripts and logs show `qcorec check` consumption, including mixed library files in multi-file invocations. None is a contract-loader input. The path mapping retains every frozen filename while documenting `.qr` as the future source spelling.

Seven refused fixtures remain explicit rather than silently acquiring an accepted mode:

| Source | Parser refusal / intended role | Candidate only |
|---|---|---|
| `compiler/tests/fixtures/ct/CT47/modeless.qcore` | intentionally omitted capture mode, refused by the parser | `.qr` (modeless candidate) |
| `docs/reports/remaining-v03/probes/S16_missing_result_annotation.qcore` | missing `->` result type | `.qr` (modeless candidate) |
| `docs/reports/remaining-v03/probes/X02_array_result.qcore` | result/body shape refusal | `.qr` (modeless candidate) |
| `docs/reports/remaining-v03/probes/X15_empty_result.qcore` | empty result type | `.qr` (modeless candidate) |
| `docs/reports/remaining-v07/probes/d02_boundary_record.qcore` | declaration refusal at `record R` | `.qr` (modeless candidate) |
| `docs/reports/w44-libc-residuals/measurement/stage3-names.qcore` | attribution-only `module contract;` fragment lacks required semicolons | `.qv` (declared-header candidate) |
| `docs/reports/w44-libc-residuals/measurement/stage4-names.qcore` | attribution-only `module contract;` fragment lacks required semicolon | `.qv` (declared-header candidate) |

The suffix-reference census is `git grep -l -z -E '\.qcore(lib)?' -- ':!docs/reports/extension-compatibility/*' | tr '\0' '\n'`. [reference-paths.txt](reference-paths.txt) lists **all 1,306 tracked files** containing a legacy suffix reference at this base: 673 under frozen `docs/reports/`, and 633 elsewhere. Every listed reference remains unhandled by this inventory-only branch. The frozen references must remain byte-exact; the 633 other paths are the explicit follow-up set for compatibility readers and EXT-02/03/04 migrations, including `crates/` (233), `compiler/` (175), `providers/` (72), `scripts/` (30), `examples/` (27), `proof/` (21), and other code, manifests, documentation and CI files. The list includes suffix strings in prose and virtual test paths; it does **not** assert all are active filesystem dependencies. A consumer-specific reference audit must distinguish them before any rename. The existing plan's loader, package, lock and native-linkage references remain open; no verdict, corpus or emitted-C parity claim is made here.

## Follow-ons for integration

- Reconcile the six contract-inclusion inputs with the compatibility loader's resolved modes and accepted include target suffixes, preserving pinned identities.
- Resolve the seven malformed candidates by fixture intent, not by accepting malformed syntax. Preserve their expected refusals when exercising aliases.
- Classify the 633 live reference-path rows by dependency before EXT-02/03 renames; retain the 673 frozen reference paths unchanged and label outdated naming beside the archive under EXT-04.
- Compare the integrated compatibility readers against this mapping and run renamed-copy parity at the combined EXT-01 boundary. This inventory alone cannot establish reader acceptance, warning emission, verdict preservation or emitted-C byte equality.

## Branch validation and incomplete gate

On `codex/extension-inventory` at `60a5c326`, `git submodule update --init --recursive` exited 0 and selected libevent `d82464a277d0f42703702c4dfd9af6af38595a83`. `cargo run -q -p qcore-frontend --example extension_inventory > docs/reports/extension-compatibility/mapping.tsv` exited 0 and emitted 621 rows. `cargo fmt --all -- --check` and `cargo clippy -p qcore-frontend --example extension_inventory -- -D warnings` each exited 0.

The standalone full-suite attempt using `python3 scripts/run_test_binaries.py` did **not** reach tests: its build failed with disk quota exceeded (`os error 122`, cargo rustc exit 101) in `private/extension-inventory-gate/test-binaries.log`; the initial zsh wrapper then exited 1 before writing a separate status file because `status` is read-only in zsh. This run is discarded as validation evidence. A second attempt with `CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=4`, `--jobs 4` and fresh `private/extension-inventory-gate/test-binaries-attempt2/` was interrupted at the integrator's request during compilation (exit 130, log `private/extension-inventory-gate/test-binaries-attempt2.log`) so the integrated EXT-01 tree can take one full standing gate without duplicate disk cost. Both incomplete attempt logs and exit notes remain worktree-private. Only this worktree's failed `target/` was reclaimed after jobs stopped. The integrator owns the required full gate on the combined EXT-01 tree; this branch does not claim it passed.
