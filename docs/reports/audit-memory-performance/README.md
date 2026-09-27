# Audit CPU and memory optimization

Base: `4529c377d356428d3948a9061467e717edeaa970`. Prediction before implementation: indexing the existing typed statement candidates and releasing discarded first-pass audit results should change zero verdicts, diagnostics, source identities, assumptions or evidence versions. Missing or ambiguous Clang facts must remain refused. This card measures a bounded real build-configured selection before and after from the same working directory; time and peak RSS are observations on a contended host, not general throughput guarantees. The full standing gate and parity evidence are required before merging onto main.

Initial observation: the user's four-worker `current` command without checkpoint flags held approximately 3.3 GiB RSS after eleven minutes; another independent sixteen-worker audit was also running. Neither process was altered. Static findings to measure: typed statement lookup repeatedly enumerates all candidates and scans source prefixes for line numbers; a two-phase refutation run retains discarded first-pass products while computing the second pass. These are cost mechanisms, not yet measured runtime shares. Per-function Clang subprocess work in the Boolean source importer is a separate candidate requiring bounded memory design.

## Implemented and measured

The typed statement fallback now indexes candidate paths once per function, shares function trees across CFG rebuilds, and computes source lines from a prebuilt offset index. Its Clang JSON decoder consumes top-level declarations individually, retaining function bodies and field facts instead of materializing the entire translation-unit tree. Exact token matching, ambiguity refusal, current binding predicates and field-owner checks remain. Malformed metadata and non-`TranslationUnitDecl` roots are refused; the latter explicitly guards the decoder's Clang-root assumption. See the [typed-index implementation report](../audit-typed-index/README.md).

Audit refutation unions now borrow report rows instead of cloning all reports. Discarded phase-one results are dropped before phase two begins. A Drop-count regression verifies this lifetime boundary; these benchmark selections have one phase and do not measure that saving. No provider contract, verdict rule or Lean semantic definition changes; the standing Lean audit remains required.

All four verdict-bearing artifacts are byte-identical in each comparison: diagnostics, contracts, chunk contracts and source effects. Exact files are archived under [artifacts](artifacts/), with hashes and commands in the linked comparisons. This is bounded parity, not a full-corpus replay or a coverage promotion.

| Selection | Configurations / jobs | Wall seconds before → after | User CPU seconds before → after | Maximum RSS KiB before → after |
| --- | --- | --- | --- | --- |
| `flb_task.c` | 2 / 1 | 19.21 → 9.28 | 19.02 → 9.15 | 377,176 → 299,132 |
| `flb_input_chunk.c` | 2 / 1 | 50.74 → 26.61 | 50.11 → 26.38 | 506,036 → 375,588 |
| Both files | 4 / 4 | 64.67 → 20.63 | 85.06 → 43.50 | 1,421,896 → 1,123,624 |

Evidence: [task](task-parity.json), [input](input-parity.json), [pair](pair-parity.json). The four-worker sample is 3.13× faster with 21.0% lower maximum RSS. These are single samples on a shared, contended host, measured with GNU time; maximum RSS is its resource statistic, not aggregate host memory. The final table's runs did not overlap each other. Earlier final input/pair runs accidentally overlapped briefly (27.44 / 15.01 seconds and 375,612 / 1,118,648 KiB); retained private logs disclose them, but the table uses subsequent sequential repeats. Other agents' processes were neither stopped nor changed.

The [index-only intermediate comparison](indexed-parity.json) measured 64.67 → 15.78 seconds but essentially unchanged memory (1,421,896 → 1,408,852 KiB), motivating incremental decoding. Host variation means the incremental decoder's time cannot be isolated from these samples.

## Reproduction and provenance

Run `measure.py` from the repository root with `--binary`, a fresh `--label`, `--jobs`, and `--prepared` pointing to the existing Clang preparation directory. Repeat `--source` for a pair and set `--case pair`. `compare.py BEFORE AFTER --output RESULT --archive DIRECTORY` checks all four artifacts without normalization. These commands use `current` and require no new prepare phase when the source tree and compilation database still exist.

Baseline binary SHA-256: `eb6c5e16bbce0464468256156e6dabbc118a2f241766002d5479b313a4f29e45`, built from base `4529c377` at documentation-only commit `45237b61`. A first build briefly overlapped pending source edits; it was discarded and a pristine rebuild completed before copying the measured baseline. `baseline-pristine-build.log` owns that build. Final binary SHA-256: `1ad1daba370697fc438cc0ebd404ac63d465e26163e0a49f1db9efa687631186`, built at `5fc0175a`. Later edits are documentation and comments. The raw run records' `source_tip` is the checkout tip at invocation, not necessarily the preserved binary's build revision. All runs used the same prepared Fluent Bit revision `046accf6aded7906f66a1baf0202da349e521736`, source paths, profiles and corresponding selections, with Clang 21 and no checkpoint flags.

Private logs and binaries: `/home/cosmo/.codex/worktrees/audit-checkpoint-batches/qcore-c/private/audit-memory-performance/`. Each named run retains `run.json`, `time.txt`, stdout, stderr and four artifacts. The baseline/final build and focused refutation checks have separate `.log` and `.exit` files. The [integrated gate record](gate.json) owns final validation.

## Remaining work

This card owns three performance follow-ons: per-function Clang parsing in the Boolean source importer; repeated typed-access model reconstruction in CFG building (cache only with exact source/frontend identity); and bounded handling of buffered Clang stdout and retained final report products. Clang's own memory and whole-corpus scaling remain outside these bounded measurements. Existing checkpoint files and running audits were untouched.

## Validation and handoff

The complete standing gate at `c695cea4` passed: **331 Rust units / 344 summaries, 6,065 passed / 0 failed / 11 ignored; 213 Lean audits; 365 Python tests**. Submodule setup, formatting, all-target Clippy with warnings denied, and both reevaluation examples exited 0. `cycle20_verifier_plugins`, including the previously reported process-lifecycle budget regression, passed all 20 tests without weakening its deadline. Clang was recorded in this worktree. Exact commands, exits, durations and private log paths: [gate steps](gate/steps.json), [summary](gate.json). The focused refutation suite also passed 5 tests. The Sol 6 medium implementation agent performed a final read-only parity review of the integrated decoder/index and found no correctness issue; review does not substitute for tests.

The final documentation checks are recorded in `gate/final-checks.json`. Source changes are complete; user authorization requires merging this gated branch onto `main` and pushing the configured remote. The main checkout and remote are rechecked immediately before integration. Archived verdict files survive private worktree removal; raw performance runs and full gate logs remain at the explicit private paths above.
