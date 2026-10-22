# AUDIT-IR-FRAGMENTS: source-rechecked CFG artifacts

Implementation base: `45cff3b248fb1ac26ffc2829efe377bb7a1eb63f`, with the
uncommitted AUDIT-IR-PLAN slice retained. Parent specification:
[AUDIT-IR](../../development/audit-ir-card.md).

Prediction before implementation: with capture disabled, every existing
analysis and artifact remains unchanged. Successful enabled captures must
preserve complete CFG equality, reports, assumptions and target inventories.
Capture is observational: persisted CFGs cannot replace fresh source lowering,
validate a contract, confer trust or bypass a certificate check. Capture I/O,
identity, size or round-trip failure aborts the opted-in audit; no successful
capture may silently omit a selected target or a CFG construction refusal.

The fragment format retains unit source/build/trust context once,
content-address actual construction inputs (including rewritten tokens and the
effective contract environment), and preserve complete typed CFG records.
Bounded atomic object publication and exact caller-held context checks precede
readback. A unit completion manifest records all captured construction variants
and explicitly lists selected targets for which no CFG was captured. It is an
artifact inventory, never a safety certificate or a proof that every C effect
was modeled. Fresh lowering remains mandatory because typed readings and later
consumers still consult source facts outside the CFG.

The 6 GB target / 7 GB ceiling and body/AST eviction remain unmeasured and open.

Persisted IR uses `.qc`: `objects/OBJECT_ID.qc` and
`units/UNIT_ID/INDEX_ID.qc`, including `.qc` temporary publication files.
Payloads use canonical typed JSON encoding; `.json` remains reserved for
ordinary configuration, reports and validation receipts in this slice.

`QCORE_AUDIT_IR_DIR=DIR quirin-audit audit.json` enables capture for configured
units. With the variable absent, the gateway hook returns immediately and the
existing configured analysis runs directly. The capture wrapper retains the
original analysis result and the builder hook records its actual success or
refusal after contract dependency recording has ended. It never feeds a decoded
CFG back into inference. Unit context includes compile-task identity, source
profile, selected properties, trust policy, contract sources, the complete
source-definition inventory, retained preprocessing outputs/errors and request
options. The checker digest identifies the observed executable file; it is not
an execution attestation.

Typed serialization covers the transitive CFG operands, including nested call,
callback, lifetime, publication and refusal data. Sealed source inventories gain
serialization only. Pointer-store classes decode through a closed label table,
without leaking strings to manufacture static references. Each object is bounded
to 64 MiB; instruction nesting is checked iteratively to depth 32 and serde's
normal JSON depth limit remains active. Reads verify hashes, canonical typed
round trips, referenced construction inputs, independently expected context and
complete equality with the freshly built CFG. Publication syncs a temporary file
and uses an atomic no-overwrite hard link; conflicting bytes fail. Completion
indexes preserve selected-target order, missing captures and construction
refusals; partial object files cannot act as completion indexes.

This slice changes representation and observation, with no new inference or
cache-admission rule. Existing Lean semantic definitions remain applicable; the
standing Lean audit is still required. Source-free replay, complete non-CFG fact
storage, component leases/body eviction and memory measurement remain open in
the parent Card. The format is an initial opt-in artifact format with no legacy
`.json` artifact migration path.

The final combined implementation, including the requested `.qc` suffix, is
validated by the [lifetime gate receipt](../audit-ir-lifetimes/gate.json) and its
[validation account](../audit-ir-lifetimes/README.md). Fragment tests pass 9/9
and configured CLI controls 2/2; every published object and completion index
has extension `.qc`. Full Rust passes 8,483 tests with 15 ignored; Lean passes
361 audits. The final Python suite has the one pre-existing committed-matrix
failure, so the complete standing gate is not claimed green. The two earlier
fragment-format campaigns were interrupted on user steering and are retained
in the shared attempts receipt. Fresh source lowering remains mandatory and
no memory-budget acceptance is claimed.
