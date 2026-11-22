# SP6-SYMBOLIC-HYPOTHESIS-ATOMS: prediction

Committed before any reader, test, fixture, contract, specification or Lean
change. Base `38cf960fa` (branch `integrate/sp5-premise-ban`: SP5 merged onto
origin/main `b069dbda8`).

User decision 2026-11-20 (verbatim): "we don't want to write down the
sentences on premise clauses. We really dislike this kind of results so we
must write down as condition symbols like |-> or other types of logical
symbols to represent the prerequisites which compose for premise
conditions." User 2026-11-22, on SP5 leaving the cJSON hypotheses as
English: "We need to convert to be symbolized atoms of hypothesis."

## The plan the prediction is about

* The nine `resource_hypothesis(detach_*, "...")` atoms of
  `cJSON_DetachItemViaPointer` (`librdkafka/cjson.qv`) become C04.18.7 typed
  atoms (`quant proposition #detach_*(...) = FORMULA;`), each the typed
  rendering of its part of the declaration's C04.18.9 premise formula over
  registered terms, and each proved equivalent in Lean to its
  `QCore.CjsonRepresentationPremises` definition. The nine
  `resource_guarded_symbol_v1` prerequisite clauses that repeated each
  atom's sentence repeat its formula. The cJSON binding manifest, the
  guarded adoption plan and `cjson_local_binding`'s expected graph are
  migrated with them.
* `resource_hypothesis(NAME, "...")` is refused on every load route with a
  named diagnostic (`QF002`); the hypothesis graph no longer admits an
  opaque statement for a declared atom (a generated `legacy_atom_*`, the
  erasure of a named premise, keeps its rule). Tests, examples and
  fixtures state typed atoms; frozen texts are read through the test-side
  legacy reader.
* The atoms are evidence-only metadata (C04.18.1/C04.18.7): no verdict,
  standing or certificate reads a hypothesis atom.

## What can move

No analysis decides a verdict from a hypothesis atom, so no status moves.

| Route | Status moves | Content |
|---|---|---|
| corpus | none | byte-identical, or contract-source digests only |
| twelve-home (reads the corpus inputs) | none | as corpus |
| system | none | contract-source digests; graph digests and atom definitions of `cJSON_DetachItemViaPointer` where a sidecar prints that declaration's hypothesis graph |
| slice | none | as system |

Production bytes that change: `providers/qcore-provider-librdkafka/contracts/librdkafka/cjson.qv`
(the declaration text of `cJSON_DetachItemViaPointer`, so its pin
`contract_sha256` in `contracts.lock` is re-recorded at the same sites and
revisions, plus the binding marker digest), the crate's
`contract-inventory.lock` / `contracts.json` file digest (declaration and
clause counts: the nine `resource_hypothesis` clauses become nine
`quant proposition` clauses), and the expected cJSON graph and binding
manifest the native local binding checker holds. `bounds-contracts.lock`,
`outer-contracts.lock` and `providers/contract-load-path.json` are
predicted unchanged.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

No Unknown -> Safe, no Safe -> Unknown, no Unsafe added or removed; null
verdicts unchanged. A route that refuses to load after the change would be
a missed string hypothesis in a shipped module (a census failure, reported
as one), not a semantic result.
