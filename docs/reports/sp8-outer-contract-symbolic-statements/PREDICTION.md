# SP8-OUTER-CONTRACT-SYMBOLIC-STATEMENTS: prediction

Committed before any reader, test, fixture, contract, specification or Lean
change. Base `95f36b628` (branch `integrate/sp6-hypothesis-atoms`: SP6 landing,
not yet on origin/main).

User decision 2026-11-20 (verbatim): "we don't want to write down the
sentences on premise clauses. We really dislike this kind of results so we
must write down as condition symbols like |-> or other types of logical
symbols to represent the prerequisites which compose for premise
conditions." User 2026-11-23: "Launch the outer contract prose card too".

## The plan the prediction is about

* The statement of each outer contract definition,
  `quant #outer [[contract]] NAME, "STATEMENT";` (C04.20), becomes a C04.18.9
  formula over registered terms (an SP8 vocabulary block appended after the
  existing ones): `rk_plugin_library` on `rd_kafka_plugin_new`
  (`qcore-provider-librdkafka`, `librdkafka/outer.qv`) and
  `flb_external_plugin` on `flb_plugin_load` (`quirin-provider-fluent-bit`,
  `fluent_bit/outer.qv`). The citation on `flb_plugin_proxy_create` carries
  no statement and is unchanged. The candidate inventory digest the statements
  cite (`outer-candidates:5c3f…8147`) stays a checked identity, carried as a
  typed constant the candidate check reads.
* A string statement is refused on every load route with a named diagnostic
  (`QF004`); frozen texts are read through the test-side legacy reader.
* An outer statement is reviewable metadata: no static verdict reads it, and
  the runtime projection decides from the contract's identity
  (`NAME@DIGEST`), its pin standing, its envelope lines and the candidate
  inventory digest the statement cites (the same digest before and after);
  it prints the statement in its record and decides nothing from the rest of
  it.

## What can move

No static route reads an outer module unless `QCORE_OUTER_PROJECTION` is set,
and the projection reads no statement content, so no status moves.

| Route | Status moves | Content |
|---|---|---|
| corpus | none | byte-identical, or contract-source digests only |
| twelve-home (reads the corpus inputs) | none | as corpus |
| system | none | contract-source digests only (the two `outer.qv` files' bytes) |
| slice | none | as system |

Production bytes that change: the two `outer.qv` modules (one header clause
each), so each defining loader's `contract_sha256` in its crate's
`outer-contracts.lock` is re-recorded at the same sites and revisions, and the
contract identities move by digest (`rk_plugin_library@…`,
`flb_external_plugin@…`, the `Pending` reason a runtime projection names). Each
crate's `contract-inventory.lock` / `contracts.json` moves by the `outer.qv`
file digest only (declaration and clause counts unchanged).
`contracts.lock`, `bounds-contracts.lock`, `outer-candidates.json` and
`providers/contract-load-path.json` are predicted unchanged.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

No Unknown -> Safe, no Safe -> Unknown, no Unsafe added or removed; null
verdicts unchanged. A runtime projection (when selected) moves no row: its
`Pending` reasons name the new digests and nothing else. A route that refuses
to load after the change would be a missed string statement in a shipped
module (a census failure, reported as one), not a semantic result.
