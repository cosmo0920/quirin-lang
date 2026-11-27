# SP3-SYMBOLIC-FLUENT-BIT-LIBC-PREMISES: prediction

Committed before any reader, contract, test or Lean change. Base `1cd7da6b8`
(origin/main after the SP1 landing and its records).

User decision 2026-11-20 (verbatim): "we don't want to write down the
sentences on premise clauses. We really dislike this kind of results so we
must write down as condition symbols like |-> or other types of logical
symbols to represent the prerequisites which compose for premise conditions."
SP1 converted OpenSSL; this card converts `quirin-provider-fluent-bit` and
audits `quirin-provider-libc`.

## Census at base (comments excluded, counted per clause)

| Crate | File | Premise | Prose clauses |
|---|---|---|---|
| quirin-provider-fluent-bit | `fluent_bit/input_chunk_name.qv` | `untracked_handback` (`flb_input_chunk_get_name`) | 1 |
| quirin-provider-fluent-bit | `fluent_bit/loop_owner.qv` | `ingestion_active` (`flb_input_collector_resume`) | 1 |
| quirin-provider-fluent-bit | `fluent_bit/output_singleplex.qv` | `synchronous_output_reachability` (`flb_output_task_singleplex_enqueue`) | 1 |
| quirin-provider-libc | `libc/errno.qv` | `nested_call_admission` (`__errno_location`, `strerror`) | 0 |

The brief counted 5 Fluent Bit clauses (2 per name) and 2 libc clauses; SP1's
ledger counted 3 and 0. The recount agrees with SP1: each Fluent Bit name is
stated once (the other mention of each is a comment), and both libc
`nested_call_admission` premises are `resource_premise_all` conjunctions of
`quant proposition` atoms (card QUANT-CONTRACT-HYPOTHESES), whose erased
statement is `all(...)` over atom identifiers, not prose.

## The plan the prediction is about

* The three Fluent Bit premises keep their names and declarations and state
  C04.18.9 formulas. New terms are appended to the closed vocabulary in one
  block headed by this card (Rust table, specification table, Lean `vocab`);
  no grammar change and no new sort. Premises stay undischarged metadata.
* Both crates refuse a prose premise in their own tests (neither has a
  checker binary).
* The libc crate's contracts are unchanged.

## What can move

Nothing decides a verdict from a premise. The three declarations are pinned
in `providers/quirin-provider-fluent-bit/contracts.lock`, so each is re-pinned
at exactly the revisions it is pinned at now; with that re-pin every one stays
in force where it was.

| Route | Loads quirin-provider-fluent-bit | Status moves | Content |
|---|---|---|---|
| corpus | yes (its resource view) | none | the premise text of rows resting on the three declarations |
| twelve-home (reads the corpus inputs) | yes | none | as corpus |
| system | yes | none | as corpus |
| slice | yes | none | as corpus |

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**Rendered text.** Where a premise of the three declarations is rendered (a
row's dependency record, the sidecar's `contract_dependencies`, a reading-site
environment text, `declared_premises`), the English sentence is replaced by
the canonical formula. Digests that cover that text (environment texts, the
contract bundle) change with it; no status, leaf or count does.

**The lock diff** is predicted as: `contract-inventory.lock` of the Fluent Bit
crate changes exactly the `sha256` of the three modules; `contracts.lock`
changes exactly the three declarations' `contract_sha256`, `pinned_at` and
re-reviewed identities (same revisions and sites as before), no entry added or
removed; `bounds-contracts.lock`, `outer-contracts.lock`, the libc locks and
`providers/contract-load-path.json` unchanged (the Fluent Bit crate publishes
no module).
