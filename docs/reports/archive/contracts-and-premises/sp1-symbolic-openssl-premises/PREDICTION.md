# SP1-SYMBOLIC-OPENSSL-PREMISES: prediction

Committed before any lexer, loader, checker, contract, test or Lean change.
Base `3c0f8e474` (integrate/ag-gaps: origin/main `3b12177a3` plus the CG25
and AG1-AG3 landings).

User decision 2026-11-20 (verbatim): "we don't want to write down the
sentences on premise clauses. We really dislike this kind of results so we
must write down as condition symbols like |-> or other types of logical
symbols to represent the prerequisites which compose for premise conditions.
This must be applied for OpenSSL's premises." Same day: ASCII tokens reusing
the language's existing tokens; convert every OpenSSL premise, then the
OpenSSL provider refuses a prose statement.

## The plan the prediction is about

* `requires resource_premise(NAME, FORMULA);` beside the string form: a
  bounded, sorted formula language over a closed, spec-registered vocabulary
  (`quant all #x in SET: BODY`, `|->`, `&&`, `->`, `in`, `==`, `|`, `-`,
  `{}`, `#` terms). A malformed formula, an unknown term, an ill-sorted
  application or an unbound variable rejects the module.
* The 47 `resource_premise` clauses of `quirin-provider-openssl` (four names:
  `openssl_ex_data_callbacks` 30, `openssl_builtin_bio_methods` 8,
  `openssl_unstated_callback_slots` 7, `openssl_bio_free_callbacks` 2) state
  formulas instead of English. The OpenSSL checker refuses a prose premise.
* A premise stays a stated, undischarged assumption (C04.18): its identity is
  NAME plus the canonical printed formula, which every rendering of the
  premise prints. No consumer that decides a verdict reads it.

## What can move

Nothing decides a verdict from a premise, and no pinned route loads the
OpenSSL user crate (it is reached only through
`providers/user-crates-load-path.json`).

| Route | Status moves | Content |
|---|---|---|
| corpus | none | unchanged |
| twelve-home (reads the corpus inputs) | none | unchanged |
| system | none | unchanged |
| slice | none | unchanged |

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**The OpenSSL user path** (`quirin-audit contracts` or `current` with the user
load path): no function or binding changes status. Content changes only where
a premise's text is rendered (a reading-site environment text, a dependency
record's premise list): the statement is the canonical formula instead of the
English sentence.

**The lock diff** is predicted as exactly the 30 declarations that carry an
OpenSSL premise changing `contract_sha256` and `pinned_at` in
`providers/quirin-provider-openssl/contracts.lock` (every declaration naming
one of the other three premises also names `openssl_ex_data_callbacks`), the
changed module digests in `providers/user-crates-load-path.json`, and no entry
added or removed. `providers/contract-load-path.json` is unchanged.
