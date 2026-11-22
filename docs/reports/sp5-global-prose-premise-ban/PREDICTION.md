# SP5-GLOBAL-PROSE-PREMISE-BAN: prediction

Committed before any reader, test, fixture, specification or Lean change.
Base origin/main `1d69b7b81` (SP1-SP4 landed: no provider premise is prose,
`git grep -nE '^\s*requires resource_premise\([a-z_0-9]+, *"' -- 'providers/*.qv'`
is 0).

User decision 2026-11-20 (verbatim): "we don't want to write down the
sentences on premise clauses. We really dislike this kind of results so we
must write down as condition symbols like |-> or other types of logical
symbols to represent the prerequisites which compose for premise
conditions." User 2026-11-22: "Launch SP5".

## The plan the prediction is about

* The contract format refuses `resource_premise(NAME, "STATEMENT")` in every
  vocabulary and on every load route, with one named diagnostic, at the one
  reader every route shares (`qcore_frontend::premise_formula::premise_clause`,
  called by the structural parser for every module and by the contract
  loader, the bounds loader, the local-condition reader and the operator
  gate). The module is rejected; the premise is never dropped.
* Crate tests and fixtures that state a string premise state a C04.18.9
  formula over registered terms instead; tests of the string form's
  behaviour become tests that it is refused. A frozen historical text that
  states prose is read only through an explicitly named test-side legacy
  path that no production load route can reach.
* Specification C04.18, grammar G05, rule 13h and the provider `.qv`
  comments that describe the string form are updated. Lean:
  `QCore.PremiseFormula`'s clause acceptance reflects that a string statement
  is not a premise.
* No production premise changes: every shipped premise is already a formula.

## What can move

Nothing decides a verdict from a premise, and no shipped module states a
string premise, so no module that loaded at base is refused.

| Route | Status moves | Content |
|---|---|---|
| corpus | none | byte-identical, or contract-source digests only |
| twelve-home (reads the corpus inputs) | none | as corpus |
| system | none | contract-source digests only (or byte-identical) |
| slice | none | contract-source digests only (or byte-identical) |

The only production bytes that change are comments in provider modules
that describe the retired string form (`librdkafka/cjson.qv`,
`librdkafka/client.qv`, `fluent_bit/loop_owner.qv`,
`fluent_bit/output_singleplex.qv`, `fluent_bit/input_chunk_name.qv`,
`libc/errno.qv`). A comment is outside every declaration's pinned text, so
no declaration is re-pinned and none leaves force; the per-file digests in
`contract-inventory.lock` / `contracts.json` (and wherever a route prints a
contract-source digest) move, with declaration and clause counts unchanged.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

No Unknown -> Safe, no Safe -> Unknown, no Unsafe added or removed; null
verdicts unchanged. A route that refuses to load after the change would be a
missed string premise in a shipped module (a census failure, reported as
one), not a semantic result.
