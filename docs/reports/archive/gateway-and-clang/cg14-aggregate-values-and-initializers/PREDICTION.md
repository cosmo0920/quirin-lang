# CG14-AGGREGATE-VALUES-AND-INITIALIZERS: frozen prediction

Frozen before any AFTER run and before any analyzer change was committed. The
rule's first draft existed in the working tree and had been run on fixtures
only (the probe rows named under "Tests" below); no corpus, system or slice
run of it had been made. It re-freezes CG00's
[prediction](../cg00-clang-gap-baseline/predictions/CG14.md), amended where
reasons are given.

**Base.** Local `main` = `origin/main` = `c374de96` (round 6 landed).

**BEFORE.** The corpus arbiter on the base reproduces the baseline:
608 Safe / 10 Unsafe / 4,216 Unknown, `resource-diagnostics.json`
`3a25c4e968e8f646f85a61378fb50fd41287534a7d6a2e91725ef47b018a19ca`,
identical to CG10-BOUND-UNTRACKED-ROOTS' AFTER.

## Scope this prediction is for

- **M08.** A whole-record copy (`P = Q`, `T l = Q;`) is admitted only for a
  custody-free record (every member, transitively, a scalar, enumeration or
  array or record of them): the source's typed read and the destination's
  typed store. A record with a pointer member is refused by name
  (`pointer-members`). A record passed by value as an actual, returned, or
  compared in a condition is not read.
- **M09.** A declaration of an automatic record or array local initialised by
  a braced list whose every leaf carries no identity (zero-fill, a scalar the
  typed reader reads, null, a string literal, a function, a custody-free record
  read). A local that can hold a pointer gets rule 10v's flow guard (refused
  where a place rooted at it is bound; such places forgotten), so its body
  publishes no derived summary.
- **M10.** A compound literal only as a whole actual of a contracted call at a
  `Borrowed` formal, with an identity-free initializer that reads no memory.
- **M11.** A character array initialised by a string literal: a declaration
  with no custody effect; extent and terminator facts recorded, no bound.

## CG00's lists re-derived on this base

[scripts/rederive.py](scripts/rederive.py) over the BEFORE run
(`artifacts/before/rederive.json`). All 17 functions CG00 lists as freed alone
and all 28 freed with one other card are still Unknown on the base (one,
`mk_lib.c::chunk_header`, is not projected in this corpus run).

Of the 17 freed alone, by reading their leaves:

- `LZ4F_freeCDict`, `LZ4F_freeCompressionContext`,
  `LZ4F_freeDecompressionContext`: a pointer-bearing record (`cmem`) passed
  by value. **Stay Unknown** (not read).
- `rd_kafka_metadata_cache_entry_by_id_cmp`, `get_member_id`,
  `rd_kafka_topic_partition_get_topic_id`, `rd_buf_get_segment_at_offset`,
  `rd_kafka_mock_topic_find_by_id`: a record in a `return`, a condition or an
  actual. **Stay Unknown.**
- The four `TAILQ_FOREACH` bodies (`rd_kafka_mock_broker_find`,
  `…_error_stack_find`, `…_msgset_find`, `rd_kafka_mock_offset_is_aborted`):
  CG00's `aggregate-value` cause here is the address of an embedded head, not
  a copy. **Stay Unknown** (a correction to CG00's attribution if it holds).
- `cmt_histogram_buckets_default_create`: a compound literal in a `return`
  actual. **Stays Unknown** (only a call statement is read).
- `unittest_rdvarint`: literals inside `fails += CALL(…)`. **Stays Unknown.**
- `rd_kafka_toppar_pause`, `rd_kafka_toppar_resume`,
  `rd_kafka_MetadataRequest`: `RD_KAFKA_NO_REPLYQ`, a by-value literal at a
  call. **At most these 3 move**, and only if the callee is contracted and
  the formal is `Borrowed`.

Of the 28 freed with one other card: `LZ4_isLittleEndian` and
`XXH_isLittleEndian` have only the initializer leaf (a custody-free union
`{ 1 }`) and are **predicted Safe**. The three `calculate_*` telemetry bodies
(a zero-initialised custody-free union plus one indirect-access leaf) may
move. The rest keep other blockers.

## Predictions

1. **BEFORE reproduces** on all three routes: corpus as above; system
   41 / 2 / 191; tail-pause slice 28 / 1 / 240 / 1 Pending.
2. **No Safe → non-Safe** on any route: the rule only replaces a refused
   statement, and a newly Safe body with a pointer-bearing initializer
   publishes no summary.
3. **Corpus Unknown → Safe: at least 2** (`LZ4_isLittleEndian`,
   `XXH_isLittleEndian`) **and at most 15** (the 3 replyq bodies, the 2
   endianness probes, the 3 telemetry bodies, and at most 7 others whose
   only refusals are these forms). CG00's upper bound of 17 is not the
   bound: 14 of its 17 are outside this scope.
4. **Unknown → Unsafe: at most 3**, each classified by reading; a copy
   through a released pointer is an accusation this rule can newly expose.
   No provisional Unsafe is presented as an upstream defect.
5. **Leaves.** The braced-initializer refusal falls by at least 150 of CG00's
   513 M09/M10 leaves; the indirect-access refusal falls where a custody-free
   copy was its only reason; the unrecognised-destination refusal falls by the
   M11 string arrays (at least 4). `unknown_alias` may move either way where a
   refused statement stops poisoning bindings; each such change is attributed.
6. **System and slice:** at most 2 status moves each, none Safe → non-Safe.
7. **Sidecars** (`monkey-source.json`, contracts, chunk contracts,
   source-effects) byte-identical.
8. **Tests** (probe rows, fixture level):
   - `access_path_statements::pointer_store_and_union_still_refuse`: the
     `h->value = h->value` row flips to **Safe** (a custody-free copy); the
     `s->p = q` row keeps its refusal.
   - CG00 `m08_aggregate_copies_are_refused`: **unchanged** (`struct S` holds
     pointers: `pointer-members`).
   - CG00 `m09_braced_initializers_are_refused`: `{ .a = 1 }` flips to
     **Safe**; `{ .p = q }` keeps the braced-initializer refusal (an initial
     alias); the two T rows are unchanged.
   - CG00 `m10_compound_literals_have_no_identity`: the G row flips to
     **Safe**; the T row is unchanged.
   - CG00 `m11_string_initializers_are_refused_or_unresolved`: the two
     `char buf[] = "abc"` rows flip to **Safe** (no bound claimed); the
     `const char *s = "abc"` row is unchanged.
   - New refusal controls: a copied record with an owned field released
     through both copies, and a returned block compound literal, are never
     Safe.
9. **Ledger:** `CG14-AGGREGATE-VALUES-AND-INITIALIZERS` amended, not retired
   (pointer-bearing copies, by-value/return/condition positions and route T
   stay open); residuals registered with sizes.
