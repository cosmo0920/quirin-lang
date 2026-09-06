# Declared reactor callback record interoperability

2026-09-07, baseline `0a23d6a`. This extends the generic reactor split with a
schema-only callback record format and a trusted runtime consumer. It does not
add a callback protocol witness or derive callback authority from arbitrary C.

## Data and interpretation

`qcore-schemas::reactor` publishes version 1 `ReactorCallbackProposal` records.
Each entry identifies the exact assignment operation and event generation and
carries argument identity/mode, trigger reasons, payload permissions, and
explicit defer/spawn flags. Every field is required; unknown fields and enum
variants reject. Lists preserve duplicates so the interpreter can reject them
rather than silently discard conflicting declarations. Unsupported numeric
versions remain representable data, rejected by the consumer.

`qcore-verification::reactor_callback` exports a declared program's complete
assignment inventory and consumes proposed records against that same program.
The consumer rejects missing, extra or duplicate assignments, event identity
substitution, duplicate triggers or payloads and any changed summary field.
Only exact authority correspondence reaches the unchanged core model checker;
the original complete operation sequence and explicit accepted-profile list
remain authoritative. Empty inventories cannot hide other lifecycle or
descriptor obligations. Record order is irrelevant because operation positions
bind each entry; operation order in the model is unchanged.

The program, callback summaries, synchronization and accepted profiles are
explicit caller-supplied model premises. The exporter does not prove those
premises and the consumer does not turn them into source evidence. This API
cannot discharge a plugin obligation: it implements no host witness checker,
adds no `SemanticWitness` variant and changes no first-party verdict route.
Model acceptance and diagnostic behavior remain exactly the core's behavior
once the record correspondence checks pass. Lean model definitions are unchanged.

Monkey's existing `CallbackEffectSummary` is a different record: five declared
library-specific effects. It has no lossless mapping to generic callback
identities/permissions, so its existing profile and composition consumer remain
intact. Neither record is silently used in place of the other.

## Process and negative evidence

The `callback_proposal_writer` binary in the excluded provider conformance
workspace constructs schema records independently. Its normal dependency tree
contains only schemas and JSON/serialization dependencies. Trusted core and
verification dependencies are test-only. The child-process test compares the
parsed record with the trusted declaration and then consumes it through the
runtime adapter. A matching proposal reproduces the model certificate;
permission widening, generation substitution, omitted/duplicate assignments and
unsupported versions reject. This binary is a conformance fixture, not a
packaged verifier or an implementation of the plugin transport protocol.

Schema tests check required fields, unknown nested fields, invalid enum/type/
width values, duplicate JSON object keys, retained duplicate lists and version
handling. Adapter tests exercise every mode/reason/access variant, multiple
assignments, complete coverage, malformed identities, all changed summary
fields, permission widening, empty inventories and exact model diagnostic
parity. The core remains runtime dependency-free; existing public Libevent and
Monkey interfaces are unchanged.

## Remaining work

The generic callback record vocabulary and declared-model exchange are now
implemented. Source-bound callback inventories, protocol witness admission,
specialized provider extraction and registry-based Fluent Bit composition
remain open. The fixed conformance process is not evidence of production
reachability, source-model refinement, synchronization or upstream defects.

The full excluded provider conformance suite also passes: all five actual
packages retain their protocol Unknown behavior and mutation rejection. This
is fresh package/transport parity; the prior two-build reproducibility record
remains dated evidence in the provider packaging report.

## Final combined validation

All **600 workspace tests**, **43 Python validator tests**, formatting,
Clippy with warnings denied, both reevaluation examples, Lean build and all
**19** axiom audits pass. The excluded provider conformance suite passes its
three tests (two existing package/dependency checks and the new process exchange).
Provider conformance Clippy and formatting pass independently. The final Rust
workspace run includes the additional review regressions.

The fresh full **185-unit** audit is byte-identical to `0a23d6a` in both source
and diagnostic JSON: **2 Safe / 47 provisional Unsafe / 132 Unknown / 4 data-only**.
`audit-parity.json` records the hashes. Compressed logs and outputs accompany
this report; no raw result is promoted.
