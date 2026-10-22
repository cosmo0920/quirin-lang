# AUDIT-IR-LIFETIMES: release obsolete audit working sets

Base: `45cff3b248fb1ac26ffc2829efe377bb7a1eb63f`, with the uncommitted
AUDIT-IR-PLAN and AUDIT-IR-FRAGMENTS slices retained.
Parent [Card](../../development/audit-ir-card.md).

Prediction before implementation: releasing optional Clang reuse caches at
configured-unit boundaries changes cost only. Active readers retain their own
reference-counted leases. All inference, source/contract identity checks,
certificate issuance/admission and selected targets remain unchanged. A
contextual CFG may move into its final function analysis only after all global
body-evidence consumers have finished; other functions' inferred summaries and
contracts stay live. Unselected contextual bodies may then be released.

This slice must demonstrate that cache ownership releases its values (including
on error/panic), live readers survive eviction, and moving contextual CFGs
preserves summaries and metadata. It does not establish a 6–7 GB whole-job peak;
that acceptance requires dedicated-cgroup measurement including child processes.

The implemented release boundary covers configured JSON audit workers and their
source-contract fold. `with_audit_unit_lifetimes` clears all four thread-local
Clang reuse tables before and after work. Fresh launches still use the existing
full identity, failure and parsing rules. The active `SourceContext` and
`StatementIndex` remain owners while needed; cleanup removes optional cache
owners rather than invalidating those readers. Certificate self-check and
independent admission still execute before the configured worker exits.

The final function-analysis pass begins only after declared-entry checks and
contract dependency evidence finish. It releases contextual CFGs outside the
selected rows, then removes each selected body's CFG from the derivation and
moves it into the existing analyzer. Inferred summaries, conflict records and
contract environments stay available. The contextual/non-contextual distinction
is retained explicitly so derived-return lowering follows the same branch.
The source string retained for the source-contract fold moves instead of cloning.

The function summaries still own their output CFGs, and a unit's active AST can
remain large. This is obsolete-owner removal, not per-component disk paging or
a memory-budget guarantee. No analysis route or obligation is deleted, and no
semantic Lean rule is changed.

Before extending implementation: the three contract-settlement routes currently
keep the ABI-withdrawn environment and every successive guarded/slot/detach/
reference/callback environment alive through later analysis. Replace those
shadowed owners with one borrowed-or-owned environment, dropping each obsolete
owned predecessor after its successor is produced. Preserve the five settling
functions' order, exact inputs, ABI dependency recording and original caller
input evidence. Prediction: identical contract environments, reader observations,
verdicts and diagnostics, with fewer simultaneously live environment owners.

The settlement owner replacement is implemented in the single-function,
component-census and whole-unit routes. Each still calls ABI withdrawal, guarded
retention, slot moves, detach, references and callbacks in the original order.
The whole-unit recorder still records the ABI change before the owned environment
moves. No prior environment is read after replacement; the original input remains
borrowed for later declared-body evidence. This change affects all three routes,
unlike the configured-only cache boundary described above.

Validation on the combined working source is recorded in [gate.json](gate.json).
The whole Rust run exits **0**: **536 units, 549 summaries, 8,483 passed,
0 failed, 15 ignored**, with no failed-unit substitution. The focused runs
pass 9 fragment tests, 2 AST lifetime tests, 1 contextual-CFG move test and
2 configured CLI tests; those are already included in the full-suite total.
`cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo check -p qcore-c-gateway --features indirect-census`, both reevaluation
examples, submodule initialization and Clang discovery all exit 0. The Lean
runner passes **361 audits**, exit 0; no proof source changed.

The first full Python run exits 1 with 2 failures among 374 tests: the existing
committed-matrix mismatch and missing registrations for the new tests' own
scratch-directory walks. The latter is corrected deliberately in
`scripts/tests/test_tracked_files.py`: six walks inspect only the configured
fixture's temporary IR outputs, one only the store test's temporary object
directory. No repository input discovery is added and the tripwire is unchanged.
The fresh full Python rerun exits **1**, **373 passed / 1 failure / 374 total**;
only `test_the_derived_facts_are_recomputed_rather_than_carried` remains.
The unchanged committed HEAD has 361 Lean audit sources while its matrix says
360, as recorded by the earlier same-HEAD
[boundary receipt](../audit-ir-plan/committed-matrix-boundary.json).
The matrix is not silently regenerated, so this is **not a fully green standing
gate**.

Exact argv, exits, counts, source hashes and absolute private log paths are in
the gate receipt. Primary logs:
`private/audit-ir-lifetimes/20261021T235334Z/`;
post-hygiene Python:
`private/audit-ir-lifetimes/20261022T000620Z-python/`.
Both source guards report unchanged Rust and the same HEAD. The post-hygiene
run also pins its Python test's bytes. Earlier interrupted and failed attempts
are retained in [attempts.json](attempts.json); none supplies the final pass
counts. Fragment serialization development failures remain at the private
path named there. The task-specific temporary Cargo target is reclaimed after
all validation processes finish; private evidence is retained.

Follow-ons remain the parent Card's per-component AST leases, eviction of final
summary CFGs while retaining certificate replay inputs, other audit routes,
authenticated `.qc` admission and dedicated-cgroup whole-job measurement.
The original 17 GB workload has not been identified or remeasured by this
receipt. No production corpus parity, wall-time improvement or numeric memory
saving is claimed from these fixture and standing-gate results.
