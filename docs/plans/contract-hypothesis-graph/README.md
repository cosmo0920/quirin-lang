# CONTRACT-HYPOTHESIS-GRAPH-V1: atomic hypotheses below premises

Status: candidate implementation card, not dispatched work and not an
implementation or coverage claim. Registered in the acceptance matrix under
the full card ID `CONTRACT-HYPOTHESIS-GRAPH-V1`.

## Question

Can Quirin replace each opaque named contract premise with a replayable,
context-bound conjunction of smaller hypotheses, while preserving the current
fail-closed verdicts and keeping proposition status separate from the authority
of its evidence?

Today C04.18 gives a declaration a stable premise name and a prose statement,
and C06.5 publishes that premise with every dependent row. Except for explicitly
specified readers, the premise is evidence metadata: no static decision reads
its prose. This is the correct conservative boundary, but it leaves a reviewer
or agent unable to identify which part of a broad premise is open, refuted,
observed only under bounds, or already derived by the checker.

This card introduces the records-first substrate for that decomposition. It
does **not** make an opaque assertion true, turn agreement into provider
authority, or promote a verdict.

## Proposed model

The model has four distinct layers:

1. A **hypothesis** is one stable, machine-addressable proposition.
2. A **premise** is a formula over hypotheses. Version 1 admits conjunction
   only; there is no negation, disjunction, threshold or default truth.
3. **Evidence** supports, refutes or leaves a hypothesis open, and separately
   records its warrant: `derived`, `observed`, `declared`, `human-reviewed` or
   `provider-proposed`.
4. A contract clause and each published verdict name the transitive premise and
   hypothesis dependencies they actually consulted.

Truth disposition and warrant are orthogonal. An observed proposition is not a
derived theorem; a reviewed declaration is not body validation; a provider
proposal is never core authority. Each hypothesis instance is bound to the
verdict contract's artifact, property, semantic profile, scope, bounds, source
identity and evidence version. Evidence from incompatible contexts cannot be
conjoined.

The canonical graph is acyclic and content-addressed. A premise node contains
an ordered canonical set of child hypothesis IDs. A hypothesis contains its
typed subject and predicate, or an explicitly opaque statement, plus its
context key. Duplicate IDs with different content, missing children, cycles,
non-canonical encodings and context mismatches reject total-or-reject.

## Compatibility surface

The first implementation is additive to the `.qv` contract format and reserves
no `.qr` source-language word. The exact syntax is selected during the card,
but it must meet these compatibility rules:

- Every existing `resource_premise(NAME, "STATEMENT")` elaborates to one
  premise containing one opaque hypothesis. Its contract digest, pin behavior,
  standing and verdict effect are unchanged.
- A structured premise may name several distinct hypotheses and means `all` of
  them. Version 1 has no `any` or `not` spelling.
- Recognized typed predicates have a closed registry owned by the core. An
  unknown typed predicate rejects the module; it is never reinterpreted as an
  opaque assertion.
- An opaque hypothesis may be reported and reviewed, but this card supplies no
  consumer that discharges a safety obligation from its text.
- Inclusion and inheritance copy the graph and check agreement exactly as they
  do clauses and named premises. A child cannot shed or silently replace an
  inherited hypothesis.
- Adding, removing or changing a hypothesis changes the owning contract digest
  and requires the ordinary pin/review flow.

Illustrative syntax, not an accepted grammar decision:

```qcore
requires resource_hypothesis(callback_set_closed,
    opaque("the run's callback candidate set is complete"));
requires resource_hypothesis(callbacks_queue_neutral,
    opaque("no candidate retains the served queue"));
requires resource_hypothesis(queue_live_during_callback,
    opaque("the queue outlives every admitted callback window"));
requires resource_premise_all(served_queue,
    callback_set_closed,
    callbacks_queue_neutral,
    queue_live_during_callback);
```

The implementation may choose a less keyword-heavy predicate form after the
required reserved-word and grammar census. The semantic model above, not this
illustration, is the card's interface.

## Scope and implementation ownership

In scope:

- versioned schema types for hypothesis IDs, propositions, conjunctions,
  dispositions, warrants, context keys and evidence references;
- total-or-reject loading, canonicalization, inclusion, inheritance and fresh
  retained-text replay in `qcore-contract`;
- per-declaration and per-verdict transitive dependency recording, including
  dependencies inherited through validated summaries;
- audit sidecar sections that list each premise, its atoms, context, standing,
  dependent rows and dependent Safe rows;
- an erasure projection back to the current C04.18 premise records;
- a small executable Lean model of graph validity, context-compatible
  conjunction, erasure and monotonic refusal;
- provider package/schema migration required by a published schema change.

Out of scope:

- using an opaque hypothesis to promote `Pending` or `Unknown` to `Safe`;
- arbitrary Boolean logic, probabilistic confidence or majority voting;
- inferring independence because hypotheses have different IDs;
- whole-program C correctness, Rust-to-Lean extraction or proof of the Clang
  import boundary;
- automatically accepting LLM-authored hypotheses or evidence;
- rewriting all existing prose premises in this card.

The target ownership is `qcore-schemas` for wire types, `qcore-contract` for
loading and replay, `qcore-c-gateway` for consultation capture,
`qcore-audit` for run-wide composition/reporting, and `qcore-core` plus Lean
for the small composition rule. Compatibility facades must not become the new
implementation home.

## Soundness requirements

1. **No verdict movement.** With every current contract loaded through the
   compatibility elaboration, all static and runtime verdicts, diagnostics and
   reason categories reproduce their base. Additive graph evidence is compared
   separately.
2. **No authority laundering.** A warrant describes provenance; it never
   changes the proposition's disposition by itself. In particular,
   `provider-proposed`, `declared` and `observed` are not aliases for derived.
3. **Context equality before conjunction.** Evidence can discharge one graph
   instance only when its complete context key is compatible. A source pin,
   build profile or bound mismatch leaves the atom open and the dependent
   result non-Safe.
4. **Correlation is explicit.** Facts that are only valid jointly remain one
   relational hypothesis. Splitting them cannot manufacture an independence
   premise.
5. **No optimistic graph failure.** A missing atom, malformed graph, cycle,
   unknown typed predicate, stale digest or replay mismatch refuses the
   structured premise. It never falls back to the prose legacy premise.
6. **Consultation accuracy.** The graph follows rule 13f's actual consultation
   closure. Declined probes may over-approximate only under the existing
   recorded boundary; the graph must never omit an atom used by the decision.
7. **Static/runtime separation.** Rule 31 may project a closed-world atom only
   through its applicable pinned outer envelope. It does not change the static
   row and can only lower through the existing join.

## Required controls

At minimum, the implementation must include:

- byte-equivalent legacy-premise elaboration and retained-text replay;
- duplicate-ID/same-content acceptance and duplicate-ID/different-content
  rejection at the intended scope;
- missing-child, cycle, non-canonical-order and unknown-predicate rejection;
- same proposition under two incompatible source/build/bounds contexts refusing
  composition;
- separate controls for derived, bounded-observed, declared,
  human-reviewed and provider-proposed warrants with identical proposition
  truth status;
- a relational hypothesis that cannot be replaced by two independently
  evidenced atoms;
- inheritance that preserves every parent atom and rejects a conflicting child;
- summary dependency closure and an actual-call control that names exactly the
  atoms of the declaration consulted;
- forged provider evidence that fails the certificate firewall;
- outer-contract coverage and uncovered controls that preserve the static row;
- mutation controls for atom removal, context substitution, evidence-version
  substitution and digest replay.

## Acceptance

The card completes only when all of the following hold:

1. A committed design table defines every schema field, disposition, warrant,
   context-compatibility rule and canonical digest input before implementation.
2. Current `resource_premise` inputs elaborate to the one-opaque-atom form and
   replay through the retained source without changing their pins or standing.
3. The root and provider schema/package compatibility paths are migrated
   deliberately; all five provider packages are rebuilt and verified if the
   published schema changes.
4. Every current audit route publishes the hypothesis graph and per-row closure,
   and an erasure comparison reproduces the prior premise/dependency view.
5. The corpus, system and tail-pause measurements show zero unexplained verdict,
   diagnostic, reason, static/runtime projection or contract-standing movement.
6. Rust tests cover every total-or-reject and firewall direction above; matching
   audited Lean proves graph acyclicity assumptions are respected by the
   composition rule, incompatible contexts never discharge a conjunction,
   erasure preserves the old premise record, and adding an open/refuted atom
   cannot promote a result.
7. The standing gate and the provider gate pass with commands, exits, counts and
   private log paths recorded in the card report.
8. The report either proposes one separately registered semantic-consumer card
   per typed hypothesis family, or records why the graph remains evidence-only.

## Non-claims and follow-on boundary

This card does not prove that any current prose premise is true and does not
reduce the set of declared assumptions by splitting their text. It makes the
dependency structure explicit and safely reviewable. A later semantic card may
allow a recognized typed hypothesis to discharge an obligation only with a
consumer-specific Rust rule, matching audited Lean, positive and refusing
controls, exact context binding and measured verdict effects. LLMs may propose
atoms, source locations and candidate evidence, but the core accepts only the
closed typed vocabulary and independently replayable evidence.
