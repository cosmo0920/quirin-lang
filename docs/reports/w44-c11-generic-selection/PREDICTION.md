# W44-C11-GENERIC-SELECTION: prediction

Frozen before any corpus run of the card's reader. Base: main `70ce6b44`.

**Disclosure of order.** The reader (`legacy_generic_selection.rs`), its hook,
its unit tests, the fixture file `tests/generic_selection.rs` and the Lean model
were written, and the fixtures run, before this file was committed. No corpus
audit had been run with the reader when it was committed; the only corpus runs
at that point are BEFORE (the unchanged base) and the capture run, whose
diagnostics are byte-identical to BEFORE. The census below is from those two.

## Census (inputs to the prediction)

From the 187 distinct `-E -P` texts a transparent Clang wrapper captured
during a private run of the pinned audit ([census.json](census/census.json),
[census-table.json](census/census-table.json)):

* **85** `_Generic` occurrences, in **36** projected functions of **25** units
  (23 librdkafka, and Monkey's `mk_core/mk_string.c` and
  `mk_server/mk_config.c`). All 85 are inside function bodies and none is nested in another
  selection.
* **All 85 are glibc 2.43's `__glibc_const_generic` expansion**: `strchr` 51,
  `bsearch` 13, `strstr` 13, `strrchr` 6, `memchr` 2. No wide-character
  search, no `strpbrk`, and no other `_Generic` source (mpack's `mpack_write`
  selection is not expanded in any analysed unit).
* The census's independent Python reading of the admission rule admits all 85.
* BEFORE refusals at the 85 selection statements (a statement can carry
  several): `call to uncontracted function `_Generic`` **14** (10 functions,
  the card's table), branch condition 56, braced initializer 28, indirect
  access 12, loop / for-loop condition 3, return expression 2, unsupported
  assignment expression 1, and 9 live-obligation (leak) leaves in
  `ut_sasl_oauthbearer_oidc_assertion_create`.
* `bsearch` is not declared in any contract module; `strchr`, `strrchr`,
  `strstr` and `memchr` are declared (`libc/string.qcore`, derived interior
  results).

## Predictions

1. **P1.** All 14 `call to uncontracted function `_Generic`` leaves are
   removed; the family is empty after.
2. **P2.** The 6 of them whose call is `bsearch`
   (`ut_populate_internal_topic_metadata`, `rd_kafka_Node_new_from_brokers` ×2,
   `rd_kafka_populate_metadata_topic_racks`,
   `rd_kafka_topic_assignment_state_rack_search`,
   `rd_kafka_partition_internal_find_rack`) are exchanged one for one for
   `call to uncontracted function `bsearch`` at the same line. `bsearch` is not
   declared by this card (its comparator is a callback).
3. **P3.** The other 8 (`rd_dl_open` ×2, `ut_sasl_oauthbearer_oidc_assertion_create`
   ×2, `rd_string_render` ×2, `_mk_string_search`, `mk_string_char_search`) are
   removed without a replacement leaf at that statement.
4. **P4.** `mk_string_char_search` moves **Unknown → Safe** (the selection is its
   only leaf). None of the other 9 functions of the card's table changes status;
   `ut_sasl_oauthbearer_oidc_assertion_create` stays provisional `Unsafe`.
5. **P5.** In `mk_core/mk_string.c`, `mk_string_split_line` loses its
   `call to uncontracted function `mk_string_char_search`` leaf (W4 now derives
   a summary for a `Safe` callee in the unit) and stays Unknown. No other
   caller moves (the other callers are in units that do not define it).
6. **P6.** At the other 71 selections (26 functions whose selection statements
   were refused for other reasons), most leaves stay. Where a branch or loop
   condition was refused only because its calls were `_Generic`, the
   condition desugaring (W44-BRANCH-CONDITION-CALLS) may now admit it; I
   predict **between 0 and 30** such condition leaves removed, some exchanged
   for another refusal or an `unknown_alias`, and **at most 2** further status
   moves, none to provisional `Unsafe`.
7. **P7.** Leak stays **19**; `double_release`, `invalid_release` and
   `use_after_release` stay **0**. Any added leak or violation is classified by
   reading; if one is an artifact, the card is not done.
8. **P8.** `unknown_alias` moves by at most ±15.
9. **P9.** No unit changes status. Chunk-contracts, contracts and source are
   byte-identical to BEFORE; source-effects equals BEFORE.
10. **P10.** Every delta is in one of the 36 functions holding a selection, or
    is P5's caller.
