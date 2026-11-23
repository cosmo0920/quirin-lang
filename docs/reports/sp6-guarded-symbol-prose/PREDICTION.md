# SP6-GUARDED-SYMBOL-PROSE: prediction

Committed before any reader, test, fixture, contract, specification or Lean
change. Base `95f36b628` (branch `integrate/sp6-hypothesis-atoms`: SP6 merged
onto origin/main `1f2007667`).

User decision 2026-11-20 (verbatim): "we don't want to write down the
sentences on premise clauses. We really dislike this kind of results so we
must write down as condition symbols like |-> or other types of logical
symbols to represent the prerequisites which compose for premise
conditions." User 2026-11-22: "We need to convert to be symbolized atoms of
hypothesis." User 2026-11-23: "Launch SP6-GUARDED-SYMBOL-PROSE". User
decisions of 2026-11-23 on the stop: environment E2, success_case R1,
invalid_membership I2, and a `&&`-spelled source document rendered with `∧`.

## The plan the prediction is about

* The 17 `resource_guarded_symbol_v1` texts of `cJSON_DetachItemViaPointer`
  (`librdkafka/cjson.qv`) stop being string literals. Each becomes an
  unquoted C04.18.9 typed-rendering formula over registered terms, and the
  terms that are missing get an SP7 block. The nine `detach_*` prerequisites
  keep their formulas byte for byte. The eight English symbols become
  formulas, each proved in Lean equal to its definition.
* The `resource_guarded_decomposition_v1` document stops being a string. Its
  clauses are written unquoted in the clause, and the readable G05-guarded
  document is rendered from them, byte-identical to today's, so the
  readable SHA is unchanged.
* A string text or a string document is refused on every load route with
  `QF003`. Frozen texts are read through the test-side legacy reader.
* Guarded symbols are proposed metadata (C04.18.5, C04.18.6). No verdict,
  standing, certificate or classifier reads one.

## What can move

No analysis decides a verdict from guarded metadata, so no status moves.

| Route | Status moves | Content |
|---|---|---|
| corpus | none | byte-identical, or contract-source digests only |
| twelve-home (reads the corpus inputs) | none | as corpus |
| system | none | contract-source digests (the `cjson.qv` file digest, `cJSON_DetachItemViaPointer`'s contract digest) |
| slice | none | as system |

Production bytes that change:

* `providers/qcore-provider-librdkafka/contracts/librdkafka/cjson.qv`: the
  17 guarded symbol clauses, the document clause and the card's comment.
* Its pin `contract_sha256` in `contracts.lock`, re-recorded at the same
  sites and revisions.
* The crate's `contract-inventory.lock` / `contracts.json` file digest.
  Declaration and clause counts are unchanged.

The following are predicted unchanged: the hypothesis graph, the binding
manifest, the guarded document's readable SHA, `bounds-contracts.lock`,
`outer-contracts.lock` and `providers/contract-load-path.json`.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

No Unknown -> Safe, no Safe -> Unknown, and no Unsafe added or removed. Null
verdicts are unchanged. A route that refuses to load after the change would
be a missed string guarded symbol in a shipped module. That is a census
failure, reported as one, not a semantic result.
