# Moving Fluent Bit revisions — 2026-09-07

The configured audit can now follow a moving checkout using
`"source_revision_policy": "checkout_head"`. The fixed revision remains the
historical evidence baseline, rather than an admission requirement for every new
resource projection. Existing configurations default to `profile_pin` and the
Cycle19 compatibility adapter remains unchanged.

## Change and trust boundary

- Resolve checkout HEAD once to a commit and use that immutable identity for
  selected-file Git-blob comparisons. Keep profile clean-path checks; edits to
  selected files outside those paths also reject. A final HEAD check rejects
  revision changes observed during the run.
- Bind the revision-selection policy and actual commit into every new-mode
  compile-task identity. Even identical C bytes under a new commit retire the
  old identity. Existing compiler, preprocessing, source and dependency hashes
  remain bound.
- Reuse declared path selections and resource contracts as explicit assumptions.
  Do not admit historical audited-file digests, construct witnesses, native
  observations or provider source evidence on the new revision. The sidecar
  identifies those source-specific obligations as Unknown. Source-contract
  configuration is rejected in this mode; required evidence, unknown libraries,
  empty selections and unsupported/ambiguous compile commands still reject.
- `prepare_cmetrics_clang_audit.py` accepts `--revision` (default local HEAD),
  isolates committed source/builds by resolved commit, and generates a
  current-revision resource configuration. Selecting the original profile commit
  retains the historical process-evidence configuration. No fetch or upstream
  working-tree edit is performed. Explicit `--output` remains available.
- `contract/profiles/fluent-bit/current-audit.json` selects the five normal
  library groups from the sibling checkout. Paths/versioned library layouts
  remain explicit profile inputs; a changed layout needs a matching selection.
  Compiler-option errors now identify the library and source file.

These are input selection and provenance changes, not language/resource semantic
changes. Lean definitions and verdict rules are unchanged. Stable input files
and compiler execution remain caller premises; the end-of-run check is not a
filesystem transaction or build-hermeticity proof. Historic Safe/Unsafe/Unknown
counts are not assertions about a newer revision.

## Current source execution

Resolved local revision: `d1ab8abff6cbaee89cee5efeeeb2667d1910ac52`.
The committed source was cloned independently under
`build/cycle19/current-revision`; developer edits to unrelated S3 files were
left untouched. Clang CMake configuration, the `cmetrics-static` native build,
and a repeat preparation completed. Only that native target was built; a
successful raw resource projection does not imply a native full-library build.

The counter-only configured audit completed: **1 unit / 0 Safe / 1 provisional
Unsafe / 0 Unknown / 0 gateway failures**. Historical provider evidence was not
requested or reused. `cmetrics-audit.json.gz`, its assumption sidecar, generated
configuration and build provenance archive the inputs and result.

The full configuration rejects before publishing a report: the original GCC
build lacks an explicit supported dialect for its frontend flags; the fresh full
Clang database contains an unsupported CTraces option at `ctr_attributes.c`.
No options were dropped or rewritten. Per-library measurements below use the
same unmodified Clang database in independent scopes, not a complete-corpus
certificate. The Monkey forced-Libevent leg is not selected by this configuration.

## Separate library scopes at the current revision

| Library | Units | Safe | Provisional Unsafe | Unknown | Data-only | Outcome |
|---|---:|---:|---:|---:|---:|---|
| CMetrics | 31 | 0 | 16 | 15 | 0 | Completed |
| librdkafka | 94 | 0 | 10 | 81 | 3 | Completed |
| ChunkIO | 13 | 0 | 2 | 11 | 0 | Completed |
| CTraces | — | — | — | — | — | Unsupported compiler option |
| Fluent Bit input-chunk wrapper | — | — | — | — | — | Conflicting compilation commands |

The completed independent scopes total **138 units: 0 Safe / 28 provisional
Unsafe / 107 Unknown / 3 data-only**, with zero gateway failures. The counter-only
run above is already contained in CMetrics and must not be added to this total.
These results cannot be treated as all 185 historical units or compared as a
revision-only verdict delta: the build configuration and selected corpus also
changed. Per-library configurations, reports, assumptions and logs are archived
alongside `current-library-results.json`. Rejected groups are visible here and
were not silently removed from a claimed full audit.

## Validation

Three new CLI regressions cover changed-source reanalysis while historical mode
still rejects, dirty selected bytes outside clean_paths, historical provider
selection, missing required evidence, unknown policies, and commit-only identity
retirement. All **853 workspace tests** pass. Submodule initialization, fmt,
Clippy with warnings denied, both reevaluation examples, Lean build and all
**20 Lean audits** pass. Compressed logs are retained here.

The existing pinned CMetrics configuration still completes with **1 checked
function contract / 6 Unknown** and **1 provisional Unsafe resource unit**.
Its main report is byte-identical to the pre-change artifact. The assumption
sidecar differs from the pre-existing artifact, so full provenance byte parity is
not claimed; `historical-comparison.json` records both hashes. The historical
185-unit compatibility corpus was not rerun or relabeled. A preparation attempt
against the old cached build rejected its unrecognized compiler-cache entry type;
the existing pinned audit itself ran successfully.

See [usage](../../../crates/qcore-audit/README.md#Auditing-a-moving-Fluent-Bit-checkout).
