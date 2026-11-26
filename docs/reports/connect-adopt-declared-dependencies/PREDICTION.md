# CONNECT-ADOPT-DECLARED-DEPENDENCIES: prediction before implementation

Base `1ecaf97e9` (origin/main), census commit `df682b93d`. User decision of
2026-11-26: "Accept all" (D1 (a), D2, D3, G1–G7, L1–L6 of the
[census](README.md)).

## What changes

* `linkage_contract_v1`: `quant #link NS::NAME;`, `premises { link { ... }; };`
  and the module-level `#include NS::NAME as quant #link;` elaborate to C15.2
  dependencies; the target's identity (its lock `contract_sha256` and its
  crate version as `since`) is recorded by `providers/contracts.py sync` in
  the load-path manifest, never inline. The written
  `requires resource_contract_dependency(...)` form is retired.
* D1 (a): a dependency is satisfied through the manifest's contract-level
  graph (the target crate on the dependent's load path, the graph acyclic),
  not through Cargo.
* D2: the 11 `contract inherit libc::F` directives are links by elaboration;
  their text and pins do not change.
* 11 declarations gain `requires linkage_contract_v1;` and their links and are
  re-pinned (digest only; their reviewed source identities are unchanged):
  `rd_calloc`, `rd_strdup`, `sb_allocate_backlogs`, `sb_destroy_backlogs`,
  `sb_remove_chunk_from_segregated_backlog`,
  `sb_append_chunk_to_segregated_backlog`,
  `sb_append_chunk_to_segregated_backlogs`, `sb_segregate_chunks`,
  `sb_release_output_queue_space`, `cb_queue_chunks`,
  `flb_config_set_program_name`.

## Prediction

A dependency that resolves adds no obligation and withholds nothing, so no
verdict moves on any route. Every one of the 30 links resolves on every
pinned route:

* every target is loaded where its dependent is. The routes that load a
  dependent (`cycle19-resource-contracts.json` for corpus, twelve-home,
  system and slice; `contract/corpus.json` with the whole load path) also load
  `libc/memory.qv`, `libc/string.qv`, `monkey/intrusive_list.qv`,
  `chunkio/chunk_residency.qv`, `chunkio/chunk_sizes.qv` and
  `chunkio/resource_open_close.qv`;
* every target's recorded digest is its lock's `contract_sha256`, which the
  pins already require to equal its text;
* no target is held (each is a pinned or flexible declaration in force), so no
  hold propagates;
* the graph is acyclic (every target is a libc, monkey or chunkio
  declaration that links nothing).

One residual only a route measurement settles: the monkey and chunkio
targets' pins are reviewed at 046accf6 and 66910c10, not 34155955 (slice).
Wherever a target's pin is out of force while its dependent's is in force, the
dependent is now held too. The storage-backlog dependents are reviewed at the
same two revisions as their targets; `flb_config_set_program_name` is also
reviewed at 34155955, where `libevent::mk_list_del` is in force only if its
sites equal a reviewed identity's. A slice row that moves for this reason is
listed by the measurement, not accepted.

The re-pinned declarations change only their own digest, re-pinned with the
same reviewed identities at 046accf6, 34155955 and 66910c10, so each stays in
force exactly where it was.

```measure-prediction
route corpus
none
route twelve-home
none
route system
none
route slice
none
route current
none
```

**Chain.** Measuring this tip against `1ecaf97e9` refuses the link between
them with `cx_refuse_dependency_graph`: the tip's sides record a non-empty
dependency graph (each route's `contract_dependencies` record, absent at the
base), so the two measurements are two segments, by design, and the chain
reports no verdict move within either.

**Rows that could move, and why none should.** A link that did not resolve
on a route would hold its dependent (`dependency-unresolved`, `Unknown`;
`dependency-changed`, `Pending`). The rows that rest on the dependents are
the `sb.c` rows (system and slice), the rows calling `rd_calloc`/`rd_strdup`
(librdkafka units in corpus, twelve-home and system) and the rows across a
`mk_list_del` call that rest on `flb_config_set_program_name`'s frames.
`sb_remove_chunk_from_segregated_backlog` is `Unsafe` at `9aed1913a` (the
Priority 0 chain); if its specification were withheld its row could move.
The targeted test `linkage_shipped_resolution` resolves every shipped link
against the shipped manifest and the cycle19 module set and asserts an empty
held set; any route on which a hold appears would be a defect of this card,
reported row by row, never accepted.
