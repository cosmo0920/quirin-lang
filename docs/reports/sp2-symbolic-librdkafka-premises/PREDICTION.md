# SP2-SYMBOLIC-LIBRDKAFKA-PREMISES: prediction

Committed before any vocabulary, contract, checker, test or Lean change.
Base `1cd7da6b8` (origin/main with SP1-SYMBOLIC-OPENSSL-PREMISES landed).

User decision 2026-11-20 (verbatim): "we don't want to write down the
sentences on premise clauses. We really dislike this kind of results so we
must write down as condition symbols like |-> or other types of logical
symbols to represent the prerequisites which compose for premise conditions."
User 2026-11-21: "Launch SP2–SP4".

## The plan the prediction is about

* Every `resource_premise(NAME, "…")` clause of `qcore-provider-librdkafka`
  becomes `resource_premise(NAME, FORMULA)` in the C04.18.9 language. Census
  at base (comments excluded): 220 clauses under 12 names in 9 files
  (`rk_plugin_callback_closure` 180, byte-identical; `conf_property_writer`
  18; `asserted_allocation` 9 in three statements; `served_queue` 2;
  `cjson_allocator_hooks` 2 in two statements; `rk_log_enqueue_no_event_callback`
  2; `rk_log_enqueue_forward_not_last` 2; `cjson_list_invariant`,
  `share_ack_batches_nonempty`, `detach_list_representation`,
  `broker_monitor_callback`, `assignor_callback` 1 each). The brief's 224 counts
  four mentions inside comments.
* New vocabulary terms in a delimited SP2 block (Rust table, specification,
  Lean), existing sorts and typing rules only (no grammar change). One new
  term is the falsum `#false` (prop), so a negation is `P -> #false`.
* The crate has no checker plugin; its own test refuses a prose premise (the
  nearest equivalent; a load-time hook is proposed as a follow-up).
* The nine `resource_hypothesis(detach_*, "…")` atoms keep their text: the
  C04.18.9 formula is a premise form, not a hypothesis form (a named follow-up
  carries them to C04.18.7 typed atoms with their guarded symbols and binding).
* A premise stays a stated, undischarged assumption (C04.18): its identity is
  NAME plus the canonical printed formula, and no consumer that decides a
  verdict reads it. The one exact-identity consumer,
  `qcore-c-gateway`'s same-item detach representation check
  (`REPRESENTATION_STATEMENT`), is changed to the new canonical statement in
  the same commit, so it admits exactly the declarations it admitted before.

## What can move

The shipped load path (`providers/contract-load-path.json`) carries the
crate, so every pinned route that loads shipped contracts loads it.

| Route | Status moves | Content |
|---|---|---|
| corpus | none | premise statement text only |
| twelve-home (reads the corpus inputs) | none | premise statement text only |
| system | none | premise statement text only |
| slice | none | premise statement text only |

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**Rendered text.** Wherever a librdkafka premise's statement is printed (a
contract dependency record's premise list, a reading-site environment text,
`declared_premises`, a hypothesis graph's legacy atom), the canonical formula
replaces the English sentence. Premise names, origins and the rows that list
them are unchanged.

**Graphs and bindings.** A legacy atom's statement is part of a declaration's
hypothesis graph, so the graph digest of every declaration carrying a
converted premise changes. The 167 content-addressed binding manifests that
pin such a graph (`rk_plugin_callback_closure` 166, `detach_list_representation`
1) are refreshed (new manifests, same atoms, models and audits, the graph
digest the only changed field) and their markers updated; no atom, refinement,
flow or typed proposition changes.

**Locks.** `contracts.lock` (and the bounds, outer and inventory locks where
they digest a premised declaration) change exactly the declarations that
carry a converted premise: `contract_sha256`/digests and `pinned_at` only, no
entry added or removed. The load-path manifests change only the crate's
module digests.
