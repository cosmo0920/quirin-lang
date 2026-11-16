# LIBEVENT-CORE-CENSUS-GUARD-CORRELATION: prediction

Committed before the tip measurement. What had run when it was written:

* the base side's `libevent-core` configured audit (`fe6d5760d`, the
  revision before rule 49 entered the integration line; run `base-3`):
  21 units, 0 Safe / 0 Pending / 0 provisional Unsafe / 21 Unknown;
  `buffer.c` and `bufferevent_sock.c` are the only units whose enumeration is
  `refused` (`conditional-arm-depth-disagreement`), with 28 and 13 names in
  `absent_from_raw_scan`;
* [`scripts/raw_census.rs`](scripts/raw_census.rs) on those two files with the
  tip's gateway: the raw scan sees 73 and 8 names; guard correlation decides
  both, adding 30 and 13 names and losing none. Of `buffer.c`'s 30, 28 are its
  base `absent_from_raw_scan` list and two, `evbuffer_cb_suspend` and
  `evbuffer_cb_unsuspend`, sit inside `#if 0` (lines 3481–3505), so no
  configuration compiles them. `bufferevent_sock.c`'s 13 are exactly its base
  `absent_from_raw_scan` list.

No tip audit had run.

## Predicted tip (`0f6e17118`), same prepared inputs

* **Census.** `buffer.c` and `bufferevent_sock.c`: enumeration `established`,
  no refusal, `absent_from_raw_scan` empty (the 28 and 13 names are now raw
  names). Route totals: 2 → 0 refused units, `absent_from_raw_scan` 62 → 21
  (event.c 13 and evutil.c 8 unchanged).
* **Denominator.** `buffer.c`'s `source_functions` 101 → 103: the two `#if 0`
  definitions join it and are `not_projected` (build-excluded, kept in the
  denominator, as `strlcpy.c`'s and the other units' guarded-out definitions
  already are). `bufferevent_sock.c` stays at 21. Every other unit's
  denominator is unchanged.
* **Rows.** No function row and no unit row changes status: the 41 names that
  enter the raw half were already analysed at base (they were in the
  expansion), and both units stay Unknown on resource-projection grounds. The
  unit totals stay 0 / 0 / 0 / 21 Unknown. The only content change on the two
  units is the enumeration block and, for `buffer.c`, the two new
  `not_projected` names (console `101/101` → `101/103`).
* **Other changes since base.** The tip also carries the r11/r12 cards'
  gateway changes (member paths, the callback route, contract loading); none
  is predicted to touch this route's rows. Any other row change is
  unpredicted and will be read and attributed (with a third side at
  `15d072e45`, rule 49 alone, if needed).
* **Nothing moves toward Safe.** A complete census only lifts the enumeration
  bar on a whole-unit Safe; both units still have Unknown functions.

```measure-prediction
route libevent-core
none
```
