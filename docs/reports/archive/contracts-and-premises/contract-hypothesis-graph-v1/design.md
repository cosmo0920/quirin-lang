# Pre-implementation design and prediction

Base: `198f6fc4349d3a47591a2378835ae9286859d918`. This table is committed before implementation.

The graph is evidence-only. Existing semantic readers of exact legacy premises retain their existing scope; structured hypotheses introduce no semantic consumer. Prediction: zero changes to existing contract digests, pins, standing, verdicts, reasons, diagnostics, static or runtime projections on corpus/system/tail-pause routes. Graph publication is additive and erased separately.

| Record / field | Definition and canonical input |
| --- | --- |
| Graph.version | Integer 1, mandatory; unknown versions reject |
| Graph.atoms | Sorted unique atom IDs and complete atom records |
| Graph.premises | Sorted unique premise IDs and complete conjunction records |
| Atom.id | Declaration-local lowercase identifier; equal duplicates coalesce, conflicting duplicates reject |
| Atom.proposition | Tagged opaque statement or typed subject/predicate; opaque text never discharges; closed typed registry initially empty, unknown predicates reject |
| Premise.id | Declaration-local name, distinct from atom IDs |
| Premise.children | Nonempty strictly increasing atom IDs; no premise child, hence cycles structurally impossible; missing children reject |
| Premise.erasure | Exact legacy name, statement, origin; structured conjunction renders a deterministic statement listing IDs |
| Context | Artifact, property, semantic profile, scope, bounds, assumptions, source identity, build identity, evidence version; exact equality of all fields, no normalization or wildcard |
| Instance.context | Caller-held complete context shared by every atom; incomplete/empty identity rejects |
| Evidence.atom | Exact atom content digest and ID, never statement matching |
| Evidence.context | Exact instance context, including source/build/bounds/evidence version |
| Evidence.disposition | open, supported, refuted; independent from warrant |
| Evidence.warrant | derived, observed, declared, human-reviewed, provider-proposed; none changes disposition or conveys core authority |
| Evidence.reference | Nonempty provenance string; not a certificate or authority token |
| Graph.digest | SHA256 of versioned canonical serde JSON, structs in field order, sorted maps, strictly sorted children, UTF8 strings, no whitespace; never replaces existing contract digest |
| Replay | Reparse retained source, reconstruct graph, compare canonical bytes/digest; no fallback on mismatch |
| Inheritance | Existing inclusion/inheritance token expansion copies graph clauses; exact agreement required, child replacement rejected |
| Closure | Lift rule13f's declaration consultation closure, including summary dependencies, to every atom of each consulted declaration; held/withdrawn names remain attributed |
| Erasure | Drop graph additions to reproduce existing premise/dependency records exactly |

Grammar decision: contextual `.qv` clauses `requires resource_hypothesis(ID, "OPAQUE STATEMENT");` and `requires resource_premise_all(ID, ATOM[, ATOM]*);`. No new lexer token or reserved `.qr` word. Typed records are representable on the wire but rejected until a separately registered typed-family card owns a registry entry and consumer. Atom declarations and conjunction children must be canonical; unused atoms reject. Legacy premises elaborate as one atom with a generated disjoint ID and preserve exact old erasure. Per-declaration scope avoids unrelated modules' names colliding. No empty conjunction, any, not, confidence, independence or evidence clauses are admitted.

Evidence claims are reportable only. The kernel's compatibility/composition function refuses open/refuted or context-mismatched claims; even supported/derived claims cannot authorize a verdict because no graph consumer is installed. Provider propositions never enter the existing certificate firewall as new authority. Runtime envelopes retain their existing premise-name coverage and static/runtime join.
