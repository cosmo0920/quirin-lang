# W44-INDIRECT-CHAIN-ACCESS: frozen prediction

Frozen before the rule is wired into the classifier. The census binary is the
instrumentation commit `5365dd94`: the chain reader and the member-chain census
exist, no classifier route calls the reader, and every verdict is unchanged. The
census run reproduces all five BEFORE artifacts byte for byte
([census-identity.json](census-identity.json)). BEFORE itself reproduces the
combined baseline's diagnostics byte for byte
([before-identity.json](before-identity.json)).

The census answers come from the functions the rule calls
(`indirect_chain_access::chain_statement` and `chain_condition`). The lexical
half of this forecast is therefore the rule's own answer, not a mirror of it.
The flow half (the obligation of each link) is forecast from the census's
never-bound bit.

## The family at base (`c1e69bb5`)

W43's own classifiers, re-run on this census, reproduce both ledger sizes:

* statements, class `chain`: **5,565 leaves / 2,049 functions / 212 sole**;
* conditions, class `chain`: **1,248 / 632 / 1**.

[census/chain-census.txt](census/chain-census.txt) splits those 6,813 leaves by
the hardest census record at each leaf's key.

| Reader outcome | Leaves | Functions | What it is |
|---|---:|---:|---|
| `declarator-not-a-chain` | 1,522 | 652 | W43's place reader reads a typed declarator (`T * x = e`, `size_t n = e`) as a chain. No member is selected. |
| `no-member-chain` | 1,899 | 1,012 | No statement at the key has an access path of two or more selections: single accesses, and conditions with two separate arrows. |
| `unexplained` | 143 | 135 | W43 saw a member chain, but this census has no record at the key. |
| `dot-link` | 1,287 | 517 | A nested aggregate sub-object: `a->b.c`, `s.a.b`. |
| `root-untyped` | 580 | 300 | The root is declared through a typedef the reader answers `Opaque`, for example `rd_kafka_buf_t *`. |
| `root-undeclared` | 396 | 122 | The root is not a name the function declares, or the function's declarations were not read. |
| `call` | 383 | 253 | The statement carries a call. This is the W44-INDIRECT-CALL-ARGUMENT-ACCESS subject. |
| `subscript` | 296 | 136 | A subscript on the path: `a[i].f`, `a->b[i]`. |
| `incomplete-record` | 195 | 151 | A record on the chain is incomplete, ambiguous or a `union`. |
| **`admitted`** | **64** | **50** | Every chain is typed and the statement shape is admitted. |
| `pointer-member` | 29 | 21 | A pointer-valued final member. |
| `other-member` | 14 | 9 | An array or aggregate final member. |
| `non-pointer-link` | 5 | 4 | `->` after a non-pointer member. |

By depth, counting the member selections of the deepest real chain: depth 2 has
1,883 leaves, depth 3 has 825, depth 4 has 123, and depth 5 or more has 33.
3,564 leaves have no real chain (depth 0), and 385 have depth 1 (a subscript
followed by one member).

## Forecast

1. **Leaves leaving their families: 74, in every status.** These are the leaves
   whose every census record at the key the recogniser admits
   ([census/admitted.txt](census/admitted.txt)).
   * 69 are in Unknown functions and 5 in provisional-Unsafe functions.
   * 64 are in the W43 chain classes and 10 are outside them.
   * By family: indirect access 46, conditions 26, increment 2.
   * By arm:
     * **24 `clear`**: every chain root is never bound. These are forecast
       removed.
     * **50 `flow`**: some root is a name the body may bind. These are forecast
       exchanged for `unknown_alias` on the root: the roots are formals and
       parameters the flow leaves unbound, so none resolves to a live identity
       with a bound member place.
2. **Exchanges: at most 74, expected 50.** One `unknown_alias` per `flow` leaf,
   fewer where two leaves share a (function, line, root). A `clear` leaf
   exchanges only if its never-bound root is possibly null at the statement.
   Expected: none.
3. **Functions: 0 Unknown → Safe.** No Unknown function has every leaf `clear`.
   **Units: 0 moves.**
4. **Leak pin: 271, unchanged.** An admitted member store poisons nothing where
   the refusal poisoned every binding outside its frame, so a
   `POISON-UNMASK-ON-ADMISSION` move is possible. None is forecast.
5. **Violation candidates: 0.** No admitted chain's root is a released
   identity.
6. **Downstream: at most 10 leaves** moved away from the admitted coordinates
   (binding precision), each to be attributed.
7. **Headline counts:**
   * `unsupported`: 45,848 − 74 = **45,774** (± downstream);
   * `unknown_alias`: 4,358 + ≤ 50 = **≤ 4,408** (± downstream and dedup);
   * `leak`: **271**.

## Ledger forecast

Both entries' discharge conditions name this card's rule, which extends the
typed route to record-pointer member chains in statements and conditions, with
each intermediate dereference checked against field links, Lean parity and a
measured re-pin. Both are forecast **retired**, and each measured residual is
re-registered with its size at the AFTER census:

* nested aggregates;
* typedef'd and undeclared roots;
* subscripts;
* incomplete records;
* chains carrying a call;
* pointer and other final members;
* chains whose root the body may bind;
* the leaves W43's classes counted that are not member chains.
