# AUDIT-IR-ADMISSION: borrow certificate bodies during audit admission

Base: main `93eb17441`, containing the IR/lifetime changes on remote-main
integration base `9b13cae73`. The original IR validation receipts remain frozen
at their recorded source and counts. Parent [Card](../../development/audit-ir-card.md).

Prediction before implementation: configured admission can validate an already
owned certificate by borrowing its bodies and context, retaining the complete
fresh-source reanalysis and exact identity/equality checks. It need not clone
all CFGs, source artifacts and contracts into a temporary bound certificate.
Both borrowed and owned binding routes must return exactly the same result,
including refusals for changed targets, contracts, compile provenance or profile.
No persisted `.qc` object becomes authority or substitutes for fresh lowering.

Extend optional Clang cache lifetime boundaries to the current and pinned unit
routes. Active readers retain their own leases and independent certificate
replay still runs. Release the final settled contract-environment owner after
its derived environment has taken over, and avoid retaining a second raw source
copy alongside prepared source in configured unit products. These changes must
preserve verdicts, metadata and selected inventories. No numeric memory-budget
acceptance is predicted from ownership changes alone.

The configured projector now calls a borrowed admission entry. It derives the
issuer trust context with the same helper as the existing owned binder and
passes borrowed fields through the same containment checks. The original
certificate is returned on success and remains available for the unchanged
Unknown downgrade on failure. Independent reanalysis still reconstructs every
summary and CFG for full equality; this removes only the additional temporary
bound-certificate clone, including its source and contract copies.

Current `evaluate_entry` and pinned `audit_unit` now use the unit-lifetime
wrapper already exercised by configured audits. All later readers retain their
own leases. Configured source-contract products carry `PreparedTranslationUnit`
directly and borrow its original raw source instead of retaining another raw
string. Whole-unit analysis drops its settled environment only after the final
derivation owns the environment and before declared-entry/final-row analysis;
the original caller input stays live for body evidence.

[gate.json](gate.json) owns the fresh combined main validation: **8,492 passed,
0 failed, 15 ignored**, **537 Rust units / 550 summaries**, runner exit 0;
**362 Lean audits**, exit 0; **374 Python tests**, exit 0. Submodules, Clang,
format, Clippy, census-feature check and both reevaluation examples exit 0.
Focused certificate-context checks pass 3 tests, current CLI 16, pinned CLI 1,
fragment store 9, lifetime 2, contextual move 1 and configured IR CLI 2. These
focused tests are already included in the full-suite total.

The main integration incorporates remote `9b13cae73`; source was measured at
main `93eb17441` plus the changed-source hashes in the receipt. The full tracked
Rust snapshot was held unchanged throughout the campaign. Exact commands,
exits and absolute private logs are retained at
`private/audit-ir-admission/20261022T001745Z/`. The earlier matrix mismatch
belongs to its frozen receipt at `45cff3b24`; newer remote main supplied the
reconciled matrix. No historical gate is overwritten and no new Lean semantic
rule or provider-owned interface is introduced by this slice.

The [measurement script](scripts/measure_memory.py) measures one audit and all
its descendants in a dedicated systemd user cgroup, with swap disabled and a
7,000,000,000-byte requested limit. It requires actual `MemoryPeak` accounting;
missing accounting, changed binary/config bytes, OOM, audit failure or a timeout
cannot count as a completed budgeted audit. The service is stopped after its
final accounting is captured. Build/setup is separate, process caches start
fresh, and the host page-cache state is uncontrolled. The script supports the
single-configuration JSON route and the public `current --config` route.

Two initial input attempts are retained, not treated as memory successes:
the checkout's old DB omitted a required explicit C standard, and the Clang-
prepared DB retained static/shared configurations which the single-configuration
JSON route deliberately rejects. The actual DB is not edited, configurations
are not collapsed and no target is removed to force acceptance. A measurement
of the published current route with its complete five-library profile selection
is in progress; numeric acceptance is recorded only when it has completed.
