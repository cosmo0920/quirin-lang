# Configured process source-contract audit

> Historical checkpoint: source filenames, runnable references or discovery scripts in this archive were updated by the [extension migration](../extension-physical-migration/README.md). Recorded results, hashes and gate counts describe the original revision, not a new run of the maintained files. Use [current status](../../STATUS.md) for present measurements.


Baseline `59f5c09`, 2026-09-07. The normal `qcore-audit CONFIG.json` command now
connects explicit package/policy configuration to the actual pinned C checkout,
strict compile-database options, trusted source import and independent replay of
process-proposed evidence. A codebase needs no Rust application adapter to call
the source importer or construct summaries. This completes that command-level
integration of the existing bounded Boolean checker; broader lifecycle/provider
extraction and aggregate registry composition remain open.

## Configuration and evidence flow

The optional schema-1 `source_contracts` object identifies the source-contract
configuration, source-projection rules and selected libraries with explicit
checkout-relative helper roots. Its binding inventory must match those selected
libraries; each must also belong to the resource audit and declare Boolean field
models. Unknown fields, absent packages, unsupported flags, empty selections,
missing models and escaping roots reject. See the [configuration reference](../../../crates/qcore-audit/README.md#configured-source-contract-processes).

Helper source files are compared with their actual pinned Git blobs, even when
they fall outside a profile's clean-path or audited-file inventory. Exactly one
compile entry is required. After source import, each helper's captured raw digest
must match the verified snapshot. Existing declared-layout/prototype and protected
field checks remain in force. Main units similarly use the selected path's pinned
bytes and strict compile options. No revision string alone creates source authority.

The loaded registry retains immutable policy strings and verified package manifest
snapshots. The command imports each selected unit through its declared field model,
dispatches the selected provider, and records only independently admitted function
contracts. Unsupported imports, failed processes and rejected evidence remain
Unknown with reasons. Raw definitions absent from the typed import, including
build-excluded definitions, remain visible as Unknown. There is no in-process
fallback and no checked claim for an empty inventory.

The report embeds `source_contracts` with selected roots/revisions, loaded package
and policy inventories, source-rule bytes/digest, helper identities, per-unit
function results, admitted tasks/witnesses/bounds and checked/Unknown totals.
These function results never modify the resource-projection unit verdicts.
Omitting the option preserves the original report shape. Required library evidence
that this driver cannot supply still rejects; a Boolean witness cannot stand in
for lifecycle, callback or native-observation evidence.

## Second-codebase command and regression evidence

The real local Git fixture contains a different record (`sample_record.ready`)
declared as the existing Locked Boolean axis. Its schemas-only executable computes
paths from typed statement input. The audit supplies neither expected paths nor
caller-created source summaries. Its manifest permits concrete revisions selected
and pinned by the host; the wildcard itself is not checkout provenance.

The standalone command checks `sample_ready` and `sample_toggle` across four
Boolean-state combinations each. An opaque-call function and a build-excluded
definition remain Unknown. The unit remains **Unknown** in resource projection:
**2 checked function contracts /2 Unknown functions**, with no raw promotion.
Removing process selection reproduces the exact resource report after removing
only the additional source-contract field. The helper control checks a separately
compiled, pinned metadata-reset function and its declared frame policy.

Seven end-to-end tests cover that route, helper import, rejected contracts, failed
selected process execution, missing/duplicate/mismatched selections, absent models
or packages, unsupported flags, escaping roots, changed main/helper bytes, and
missing/conflicting helper compile entries. A process whose installed manifest
differs from its compiled identity fails at runtime; its functions remain Unknown
without fallback. Five existing common transport/evaluator tests also execute in
the new standalone conformance binary.

`second-codebase/` archives the actual configuration, source/profile/policy inputs,
compile database and package manifest. `checkout.bundle` retains the test Git pin.
Absolute paths in those files identify the observed run. To generate and execute
a fresh fixture at the current checkout, use:

```sh
QCORE_KEEP_PROCESS_AUDIT_FIXTURE=/tmp/qcore-audit-fixture-path \
  cargo test --manifest-path providers/Cargo.toml -p qcore-provider-conformance \
  --test configured_audit \
  pinned_second_codebase_runs_configured_process_checks_without_a_rust_import_adapter -- --exact
audit_fixture=$(cat /tmp/qcore-audit-fixture-path)
cargo run -p qcore-audit -- "$audit_fixture/audit.json"
```

The checked-in test creates a fresh real pin and package hash each run. Archived
reports remain evidence for their recorded identities, not for a rebuilt executable
with a different digest. See `second-codebase-run.json` and `cli.log.gz`.

## Validation and remaining boundaries

The standing gate passes **754 workspace tests**, **31 excluded-provider tests**,
43 Python tests, formatting, Clippy, both reevaluation probes, Lean build and all
20 audit modules. No core semantics, Lean definition, published schema or protocol
admission rule changes. Production provider packages are unchanged.

All four full 185-unit Cycle19 artifacts remain byte-identical to the preceding
owned-task report: **2 Safe /42 provisional Unsafe /137 Unknown**, plus four
data-only units. The production source-effect record retains three checked
function contracts and 93 Unknown functions. Persistent Monkey inputs and the
existing schema-1 ChunkIO package are reused. This does not delete the retained
specialized compatibility route or claim that the entire corpus now runs through
the new configured function-contract mode.

The source importer and witness checker remain trusted host components. The
supported property is a declared single-field Boolean contract under explicit
live/exclusive/domain assumptions; actual recursive C layout/linker equality and
stable compiler/package/input files remain outer premises. No OS sandbox, general
heap/lifecycle inference, arbitrary callback import, native observation or full
source/runtime refinement is added. Report publication is not a multi-file
filesystem transaction. Full W5 and the broader generalization goal remain open.
