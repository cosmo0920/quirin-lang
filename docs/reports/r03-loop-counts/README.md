# Revision 0.3 R06: exact repetition and maximum counts

`Flow` keeps net custody exact. The former general-loop path interpreted both
Terminating and Widened maximum counts as exact repetition; for an acquisition
body with maximum two it produced net two, which does not cover zero iterations.
The source driver rejects loops, so this defect concerns the supplied-model
summary/certificate API, not a demonstrated source-to-C acceptance.

The API now distinguishes Exact and AtMost. Historical Terminating/Widened
constructors retain their documented maximum-count meaning. Positive maximum
counts for nonzero-net bodies reject with NonExactCustody. A zero maximum
returns zero flow. Zero-net bodies retain the supported requirement/peak bound
and scale total cost. Exact counts use repeat_exact; bounded_loop remains a
compatibility spelling documented as exact repetition. The producer must supply
valid exact-count evidence; this API does not prove a source loop's iteration
count or allocation-identity invariant.

Certificate replay recomputes the selected evidence semantics. The ledger
morphism converter now rechecks the certificate before using its net change.
Tests cover 0/1/2 maxima, both net signs, zero-net early exits, exact-to-maximum
mutation, and dropping an admitted costly iteration. No maximum acquisition
count becomes an exact retained allocation count. Supporting nonzero-net
maximum summaries still requires explicit alternatives or a richer abstraction.

Lean adds a reduced count-evidence model, proves rejection of positive maxima
with nonzero net, zero net for accepted maxima, preservation of exact evidence,
and total-cost coverage of early exits. It does not prove Rust/Lean refinement
or identity/custody completeness from numeric balance. The older Lean exact
summary scales peak/entry conservatively; it is not identical to Rust's tighter
sequential fold. The new theorems concern count/net and total cost only.

All standing gates passed on the combined c7039ad integration plus this change
and the independent audit preflight: 529 workspace tests, format, Clippy, both
probes, Lean build and 18 axiom audits. Compressed logs are archived here. The
final revision integration runs the gate again after sibling repairs merge.
