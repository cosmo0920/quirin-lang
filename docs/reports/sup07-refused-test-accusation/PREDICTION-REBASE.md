# SUP07-REFUSED-TEST-ACCUSATION: prediction after the rebase

Frozen before the code it predicts (the withhold rule's replacement and the
`&NAME->F` reading below). [PREDICTION.md](PREDICTION.md) stays as frozen; its
AFTER was measured on the original base and held exactly (below).

## What moved

While the card ran, `main` moved from `cffe232d4` to `f50fa2009`, merging
`7493e76a2` ("MONKEY-VHOST-PROPERTY-CUSTODY: rule 12e, a refused null test
never establishes a leak on its null edge"): on the null edge of a refused
null test, every site the tested binding may denote that is nullable and may
be live is **withheld** (an unresolved effect), in `refine_pointer_null`
(`refused_null_test.rs`, `QCore.RefusedNullTest`, `AuditRefusedNullTest.lean`,
three controls in `monkey_vhost_property_custody.rs`). It carries no gateway
rule text, ledger or report entry; it takes the identifier this card's brief
assigned (12e).

## Measured

| Run | Corpus | Digest |
|---|---|---|
| BEFORE, `cffe232d4` | 647 / 11 / 4,176, leak 42 | `a7d68d49…` |
| AFTER of PREDICTION.md (tied settle only), on `cffe232d4` (`0a7b99cf2`) | **647 / 10 / 4,177**, leak 41: only `mk_http_error_page` moves, as predicted; system and slice byte-identical; probe unchanged | `25c1820b…` |
| BEFORE2, `main` `f50fa2009` (withhold rule) | 647 / 9 / 4,178, leak 39: `mk_http_error_page` and `mk_sched_add_connection` Unsafe → Unknown, nothing else | `dde4837e…` |

On the rebased tree (both rules), two of this card's controls fail: the
withhold rule removes the accusation of a copy lost to a refused overwrite
before the test (`a_copy_lost_before_the_refused_test_stays_accused`) and of a
copy leaked on the test's non-null edge that joins the null edge
(`a_leak_on_the_non_null_edge_stays_accused_across_the_join`), both
established. The withhold rule therefore breaks the brief's "never remove an
accusation whose decisive facts are all established"; it needs no tie and is
joined by union. It is correct on the corpus only because no corpus function
has those shapes.

`mk_sched_add_connection`'s leaves are false by reading (PREDICTION.md:
on the null edge of `if (!conn)` the copy is NULL or was never made; the
:201 copy is reached only through `conn = (struct mk_sched_conn *) data`). The
tied rule misses them only because `frame_protected_locals` reads
`&conn->event` and `&conn->channel.streams` as taking `conn`'s address, so
`conn` is no conditional name and provenance never ties it.

## The change

1. **Replace the withhold rule** by this card's tied settle (rule 12e): remove
   its hook from `refine_pointer_null` and its module, unit test and Lean
   model (`QCore.RefusedNullTest`, `AuditRefusedNullTest.lean`); keep its
   three controls, which the tied rule passes. Parity: the corpus, system
   and slice of the result against BEFORE2.
2. **Tie names** (`LegacyCfg::tie_names`, new): the conditional names with
   C's reading of `&NAME->F` (and `&(NAME)->F`): the address of a member of
   the object `NAME` points to, never of `NAME` (C11 6.5.2.3, 6.5.3.2: `->`
   binds tighter than unary `&`). Only rule 12e's provenance reads them (a
   second derivation, only where they differ from the conditional names);
   rule 10b's conditional release, rule 10a's frames and every other reader
   keep `conditional_names`.

## Predicted AFTER2 (rebased tree with 1 and 2)

1. Corpus **647 / 9 / 4,178**, leak 39, `resource-diagnostics.json` equal
   to BEFORE2 in every status and leaf: `mk_http_error_page` (tied) and
   `mk_sched_add_connection` (tied by `conn` once `&conn->F` is read as C
   reads it) Unknown, their leak leaves gone, every other leaf unchanged.
   Against BEFORE (`cffe232d4`): exactly those two functions move.
2. Functions whose tie the `&NAME->F` reading adds but carry no leaf on the
   settled edge: no leaf change. Any leaf that moves is read and named.
3. System and slice: byte-identical to BEFORE2 and to BEFORE in statuses and
   leaves (no tied refused edge there before; the reading may add ties, which
   are read if anything moves).
4. Probe unchanged.
5. All six controls of this card and the three of the withhold rule pass, plus
   a control for `mk_sched_add_connection`'s shape. Lean audits: main's 281
   minus `AuditRefusedNullTest` plus `AuditRefusedTestAccusation` = 281.
