# QCore specification revision 0.2 — assembly changelog

`docs/QCORE_SPEC_r0_2.md` §0.1 refers to this file for the record of
assembly changes made to revision 0.2. Entries are append-only; each records a
deliberate change to a normative table or rule.

## 2026-09-06 — §54 diagnostic renumbering (W7 first bullet, landed with W3)

> §54 amendment (W7/W3): the `Q700`–`Q712` identifiers previously listed for
> Libevent event/callback diagnostics are reassigned to the provider
> namespace as `qcore.libevent.L00`–`L12`; provider manifests own
> library-specific diagnostic namespaces and the core allocates no numbers
> for library-specific conditions. `Q700`–`Q719` are reserved for core
> `verify scope` resolution and dispatch diagnostics; `Q700` (scope
> resolution failure) and `Q701` (scope obligations undischarged) are
> allocated. §54's diagnostic guidance sentence is unchanged. This is a
> deliberate renumbering of a pinned table, recorded here per the
> pinned-expectations invariant.

Implementation notes for this entry:

- `Q701`'s message changed from "verify scope is configuration-only" to
  "verify scope is unverified: one or more core-derived obligations were not
  discharged by core-checked evidence". The code is unchanged and the system
  behind it is strictly stronger, so no pinned expectation weakens: the
  reevaluation VP05 probe's `Q701` arm remains truthful during migration, and
  the bounded W3 conformance suite separately requires actual dispatch.
  Production profile mismatches can still take the `Q701` containment arm.
- `Q702` and `Q703` are newly allocated (conflicting evidence; unverified
  provider allegation). `Q704`–`Q719` remain reserved and unallocated.
- The Libevent mapping table ships in the provider's documentation
  (`compiler/src/verifier_plugins/libevent/mod.rs`). The Rust
  `EventDiagnostic` variants were renamed `Q7xx*` → `L0x*`/`L1x*` in the same
  change-set, so the collision never half-exists.
- Archived Cycle 18 reports under `docs/reports/` are **not** rewritten: they
  keep their dates and their original identifiers, superseded by this entry
  and by the provider table.
