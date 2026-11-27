# SP4-SYMBOLIC-LIBEVENT-PREMISES: prediction

Committed before any reader, vocabulary, contract, test or Lean change. Base
origin/main `1cd7da6b8` (SP1-SYMBOLIC-OPENSSL-PREMISES landed, records
committed).

User decision 2026-11-20 (verbatim): "we don't want to write down the
sentences on premise clauses. We really dislike this kind of results so we
must write down as condition symbols like |-> or other types of logical
symbols to represent the prerequisites which compose for premise
conditions." SP1 converted OpenSSL; the user launched SP2-SP4 on 2026-11-21.

## The plan the prediction is about

* The 7 prose `resource_premise` clauses of `qcore-provider-libevent`, all on
  `mk_http_thread_destroy` in `contracts/monkey/intrusive_list.qv`
  (`scheduler_exact_parent_embedding`, `scheduler_tls_params_owned`,
  `scheduler_callee_live`, `scheduler_borrowed_chain_live`,
  `scheduler_valid_membership`, `scheduler_no_later_alias_use`,
  `scheduler_valgrind_request_neutral`), state C04.18.9 formulas instead of
  English. Each keeps its name; the typed refinements
  (`quant proposition #sched_*`), their `resource_premise_decomposition`
  attachments and `resource_premise_flow_v1` chains are unchanged.
* New terms are added to the closed vocabulary in their own SP4 block
  (specification C04.18.9, `qcore_frontend::premise_formula::VOCABULARY`,
  `QCore.PremiseFormula.vocab`). No new token, operator, sort or grammar
  production.
* The crate refuses a prose premise (its contract-tree test, the nearest
  equivalent of OpenSSL's `prose_premise`: the libevent crate has no checker
  plugin).
* A premise stays a stated, undischarged assumption (C04.18): no consumer
  that decides a verdict reads it.

## What can move

Nothing decides a verdict from a premise. The declaration's text changes, so
`mk_http_thread_destroy` is re-pinned at `046accf6` and `66910c10` (the two
revisions it is pinned at) and the provider locks are resynced.

| Route | Status moves | Content |
|---|---|---|
| corpus | none | rows resting on `mk_http_thread_destroy` print the seven canonical formulas in place of the English statements |
| twelve-home (reads the corpus inputs) | none | as corpus |
| system | none | contract-source digests only |
| slice | none | contract-source digests only |

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**The corpus rows resting on `mk_http_thread_destroy`** (the consumers
`sched_thread_cleanup`, `mk_sched_threads_destroy_conn`,
`mk_sched_threads_purge`, `mk_sched_threads_destroy_all`, all Safe, and
`mk_sched_drop_connection`, Unknown, as RULE50-CONSUMER-COMPOSITION
recorded; the destroyer's own row, Unknown) keep their statuses. Their
dependency records list the same seven premise names; only each premise's
statement text changes, from the English sentence to the canonical formula,
together with the hypothesis-graph digests that cover the legacy atoms'
statements. No Unknown -> Safe, no Safe -> Unknown, no Unsafe added or
removed; null verdicts unchanged.

**System and slice** analyse no Monkey scheduler unit: no row there rests on
the declaration. Every route that loads the provider's contracts sees the new
digest of `monkey/intrusive_list.qv` among its contract sources.

**The lock diff** is predicted as exactly: in
`providers/qcore-provider-libevent/contracts.lock`, the
`mk_http_thread_destroy` entry's `contract_sha256`, `pinned_at` and its two
reviewed identities (re-pinned; the C site digests themselves unchanged); in
`contract-inventory.lock` and `contracts.json`, the digest of
`monkey/intrusive_list.qv`, with 5 declarations and 87 clauses unchanged; no
entry added or removed. `providers/contract-load-path.json` unchanged.

If a pin cannot be written at a revision, the declaration is out of force
there and the four Safe consumer rows would fall to Unknown: a pinning
failure, reported as one, not a semantic result.
