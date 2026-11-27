# W43-INDEXED-RESOURCE-COLLECTIONS: frozen prediction

Written on 2026-09-25 before any rule exists, on base `ec70709a` merged with
W43-INDIRECT-ACCESS-GENERAL (`6bb838c8`). It is not edited after the
implementation commit; corrections live in the report.

## The census it rests on

[census/census.txt](census/census.txt), from
[`scripts/collection_census.py`](scripts/collection_census.py) over the BEFORE
run and the analysed `-E -P` unit texts of a capture run (180 of 185 units
paired; the same five unpaired units as W43-LIBC-CONTRACTS, which project no
function). An *acquiring* call is a selected declaration whose result is
`resource_return_owned` (16 names); a *releasing* call has a
`resource_released`/`resource_consumed` parameter (13 names). W4 summaries are
invisible to the census, so acquisitions through them are under-counted.

* **(a) element stores.** 1,824 `NAME[...] = ...` statements in 251 functions,
  but only **12** store an acquiring call's result and **4** a name bound by
  one (`owned-name`). Of those 16, the index is a plain name 8 times, `n++`
  5 times and an expression 3 times. 53 store another name, 41 a null
  constant, 25 an uncontracted call's result.
* **(b) element releases.** **16** (13 indexed by a name, 2 by an expression,
  1 by a constant); no descriptor release (`close(fds[i])`) is contracted.
* **(c) loops.** 20 loops contain (a) or (b): 9 fill loops (1 in the canonical
  counted form `for (I = 0; I < B; I++)`), 11 release loops (9 canonical) and
  **0** partial-cleanup loops. The corpus fills by append (`v[n++] = x`) over
  pointer or list traversals, not by counted loops.
* **(d) blocked functions.** 12 Unknown functions hold a leaf on a collection
  line (43 leaves); **0 would clear fully**. Every one carries between 2 and
  111 other leaves.
* **(e) the allocation's own blocker.** 224 `indirect memory access` leaves in
  211 Unknown functions are refused only for a `sizeof(*v)`/`sizeof *v`
  operand (4 sole-blocked). That is the dominant spelling of the array
  allocation itself (`v = malloc(n * sizeof(*v))`), and it is a type-query
  rule, not this card's; it is registered, not implemented.

## The planned rule (what is forecast)

Consulted only when the existing classifier refused a statement or condition,
with every base a name declared as an array of pointers (`T **v` or
`T *v[N]`) in a function whose declarations the typed reader reads:

* `v[I] = ACQUIRE(...)`, `v[I] = NULL` (`I` a declared integer, a literal, or
  `I++`); `RELEASE(v[I])` for a one-parameter releasing contract;
  `if (!v[I])`/`== NULL` conditions;
* a canonical counted `for` loop around them, with its exit substitution
  `I := B`.

A base that does not resolve to exactly one live, non-null identity this body
acquired keeps the **original refusal byte for byte** (its description and
frame). A range proof that fails after the base resolved records
`UnknownAlias` on the element place.

## Forecast

Read function by function from the census list:

* In every census function the collection base is a formal
  (`destroy_prometheus_*_list`, `free_metrics`, `free_resource_attributes`,
  `rd_kafka_conf_dump_free` through a cast alias), a heap array whose own
  allocation statement is refused (`sizeof(*v)` or a `->` in the size:
  `rd_kafka_ListGroups_resp_cb`, `rd_kafka_mock_sharegroup_get_member_ids`,
  `rd_kafka_anyconf_dump`, `rd_kafka_telemetry_encode_metrics`), an automatic
  array with a braced initializer (`ut_tq_test`), a function whose
  declarations the reader refuses (`decode_labels`, whose own `label_index++`
  is already refused as an unmodeled increment), or an index expression the
  rule does not admit (`append_entry_to_prometheus_label_list`,
  `rd_kafka_oauthbearer_unsecured_token0`).
* Therefore **no statement passes the base premise** in the pinned corpus.

| Measure | Forecast |
|---|---|
| verdict-bearing diagnostic leaves removed / added | **0 / 0** |
| function status moves | **0** |
| unit status moves | **0** |
| leak pin | **271**, unchanged |
| `unknown_alias` / `unsupported` | unchanged |
| violation candidates (`double_release`, `use_after_release`, element leaks) | **0** |
| diagnostics artifact | differs from BEFORE only in the `scope` string (the new no-bounds clause) |

The capability is therefore measured on fixtures, not on the corpus. The
residual that would move the corpus is (e), then the append-over-traversal
fill shape and element stores from a name.
