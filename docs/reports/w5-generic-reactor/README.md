# Generic reactor model and compatible Libevent adapter

2026-09-07. Reactor baseline: `46549a8`. The generation-indexed event and
callback lifecycle, descriptor custody, activation-order exploration and
observation refinement now reside in `qcore-core::event_reactor`.
The core has no runtime dependencies and knows no library profile, manifest,
source checkout or filesystem importer.

The model accepts an adapter-defined `ReactorProfile` and an explicit list of
accepted profiles. Empty acceptance data grants no base creation. Threading
support is a separate model premise; accepted profiles cannot bypass callback
authority, generation, lifecycle or synchronization checks. Descriptor-only
traces retain their existing behavior without inventing a library profile.

The Libevent adapter retains its exact loaded build-selector predicate, API
disposition lookup and pinned-source audit. It passes only profiles accepted
by that predicate to core. A concrete enum/program facade preserves existing
variant imports and type annotations; exhaustive conversion preserves operation
order and every field. Other shared public types remain reexports.

`checker-parity.json` records exact checker statement equality after five
explicit substitutions for generic profile/state access. The source-audit
implementation and helper behavior are unchanged. The existing adapter tests
remain; new tests cover provider-independent use, missing profiles, callback
authority, stale generations, cross-thread rejection and descriptor custody.
A profile bridge regression compares all 3,600 pairs from the 60 possible
Libevent profiles against the original predicate, including empty acceptance.
External-consumer regressions preserve enum variant imports.

The reactor-only full 185-unit audit, before merging the independent W4 access
repair, is byte-identical to `46549a8`: source and diagnostic JSON hashes and
compressed outputs are archived here. That baseline is 3 Safe / 94 provisional
Unsafe / 84 Unknown / 4 data-only. The subsequent W4 change deliberately
withdraws unsupported access projections; its separate evidence is in
[the indirect-access report](../w4-indirect-access/README.md). These two changes
must not be mistaken for a reactor-induced verdict shift.

This is a relocation with an explicit profile boundary, not a stronger model.
Declared traces, callback summaries and synchronization remain premises;
no arbitrary-source refinement or certificate provenance is newly proved.
Existing Lean semantics are unchanged. Callback wire schemas, specialized
provider extraction and registry-based production composition remain open.

Combined validation after the indirect-access merge passes **589 workspace tests**,
43 Python validator tests, formatting, Clippy with warnings denied, both
reevaluation examples, Lean build and all **19** axiom audits. The first audit
invocation encountered a stale pre-merge Lean object; rebuilding the new theorem
and rerunning all audits passed. Only the successful final logs are archived.

The final combined full 185-unit audit is byte-identical to the merged W4
indirect-access result: **2 Safe / 47 provisional Unsafe / 132 Unknown /
4 data-only**. `combined-audit-parity.json` records source and diagnostic hashes.
No raw units are promoted.
