# SP10-FORMAL-BINDING-DESCRIPTIONS: prediction

Committed after the census and before any reader, test, fixture, contract,
manifest, specification or Lean change. Base origin/main `2c60986b0`.

User decision 2026-11-20 (verbatim): "we don't want to write down the
sentences on premise clauses. We really dislike this kind of results so we
must write down as condition symbols like |-> or other types of logical
symbols to represent the prerequisites which compose for premise
conditions." User 2026-11-23: "Launch both" (the two SP7 follow-up cards).
The card discharges `SP7-FORMAL-BINDING-DESCRIPTIONS`.

## The plan the prediction is about

* Every `environment.variables` value of a binding manifest (C04.18.2) and of
  a flow model (C04.18.3) stops being an English description. It becomes a
  **typed environment reference**: the canonical JSON list of the G01p
  typed terms (`{"name","sort"}`, the encoding the manifests already use for
  quantifiers) that the environment component consists of. Each term is a
  parameter of a bound typed atom in the same sort, or a registered term in
  its rendered sort (C04.18.9: the vocabulary and the SP6, SP7 and new SP10
  blocks). A value that binds the identity of exact bytes is a content
  digest (64 lowercase hex). Nothing else is admitted.
* Every `FormalBinding` names a Lean definition as its proposition; its
  quantifiers are exactly the bound typed atom's parameters and its
  conclusion is the atom's formula (the form `quant_binding_migration` already
  wrote into every live manifest). The English quantifiers and conclusions
  built in code (`cjson_local_binding`, `plugin_source_witnesses`, test and
  example fixtures) are replaced by those derived forms.
* "Open" qualifiers (source, model, completeness or interpretation
  correspondence) become the never-derived assumption term of SP7's
  environment E2 (`#source_corresponds` for the cJSON detach environment it
  was defined for) or the new SP10 `#binding_source_corresponds`.
* Anything else is refused with the new diagnostic `QF005` on every load
  route (`hypothesis_bindings::resolve`, `resolve_bounds`, `resolve_model`,
  `premise_flow::compile`, `compile_bounds`, `compile_model`), fail-closed.
  Frozen manifests are read only through the test-side legacy reader.
* The 167 live manifests (166 `rk_plugin_callback_closure`, 1
  `detach_list_representation`) are rewritten with the descriptions as the
  only changed field, their markers patched in the seven librdkafka modules,
  and the locks re-recorded.
* Binding manifests and flow models are evidence-only metadata (C04.18.2,
  C04.18.3): no verdict, standing, certificate or classifier reads an
  environment description.

## What can move

No analysis decides a verdict from a binding manifest or flow model, so no
status moves.

| Route | Status moves | Content |
|---|---|---|
| corpus | none | byte-identical, or contract-source digests only |
| twelve-home (reads the corpus inputs) | none | as corpus |
| system | none | contract-source digests: the seven librdkafka module digests and the contract digests of the declarations whose text holds a binding marker |
| slice | none | as system |

Digest deltas, all from the marker lines and the manifests:

* 167 binding manifests get new content digests (new files under this
  report's `bindings/`; the old files stay in their reports).
* `providers/qcore-provider-librdkafka/contracts/librdkafka/`: `client.qv`
  (89 markers), `null_deref.qv` (71), `logging.qv` (3), `cjson.qv`,
  `kafka_mem.qv`, `memory.qv`, `queue_destructor.qv` (1 each).
* `contracts.lock` and `bounds-contracts.lock`: `contract_sha256` and
  `pinned_at` of each pinned declaration whose text holds a marker,
  re-recorded at the same sites and revisions; `contract-inventory.lock` and
  `contracts.json`: the seven file digests and manifest references.
* The flow model digests of the two sealed source adapters
  (`cjson_local_binding`, `plugin_source_witnesses`) and so their flow
  document digests.

Predicted unchanged: every hypothesis graph and graph digest, every atom
digest, premise erasure, interpreted clause, `outer-contracts.lock`,
`providers/contract-load-path.json`, and every evidence disposition and
sealed result value other than the digests above.

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
be a missed English description in a shipped manifest or module. That is a
census failure, reported as one, not a semantic result.
