# W45-CALLEE-SUMMARY-CASCADE: prediction (frozen before the first corpus run of the implementation)

Frozen on base `b160d765`. **Disclosure:** the brief asks for this file before
implementing. It was written after the rules were implemented and unit-tested
(13 tests) but **before any corpus run of the implementation binary**; nothing
below was read off an AFTER run. The figures come from the baseline
diagnostics, the BEFORE capture and the census in [census/](census/).

## What the census found (baseline, reproduced byte for byte by BEFORE)

* 4,516 Unknown functions; 2,614 carry 9,200 `call to uncontracted function`
  leaves; **412** are blocked by nothing else; 300 by exactly one callee.
* Leaf classes: same unit 3,737; other unit 3,139; header `static` 1,281; no
  pinned body 957; header of external linkage 86.
* **Single-callee leverage is small.** Validating any one callee's body would
  unblock at most 12 functions (`LZ4_decompress_generic`, 121 body leaves);
  170 callees unblock at least one.
* **A cross-unit route does not dominate.** With every existing `Safe` home
  body admitted across units, it would unblock **2** functions
  (`cio_file_real_size`, `cmt_initialize`) and strip the last uncontracted
  leaf from 17 more. Not implemented.
* **Family leverage** (every leaf of one family removed from the bodies of the
  callees, fixed point over same-unit summaries): indirect memory access 186,
  return expression 37 (owned by W45-RETURN-EXPRESSION-FAMILIES), branch
  condition 12, assignment expression 5, braced initializer 4, others ≤ 4.
  Indirect access is the lever, and its callee-body sub-shapes split into
  sub-object stores (owned by W45-SUB-OBJECT-STORES), call statements,
  conditions, declarations and assignments.
* Of the 412 roots, 253 need a callee with no same-unit body (another unit,
  no pinned body, a refused header), and 159 have a same-unit cone.
* **The destructor body.** Among cheap cones the recurring shape is
  `if (p->m) release(p->m); release(p);`: the member release is refused at a
  custody position, the member null test is refused in the condition, and the
  first refusal poisons `p`, so the final `release(p)` reads `unresolved
  resource identity`.

## The change

1. A typed pointer read (`B->F`, `B[I]`, `*B`) is admitted as a whole actual at
   a `Released { nullable: true }` position; the object released is outside
   the projection, so the position is lowered as `Borrowed`; the typed read
   and the `ArgumentReads` obligations are unchanged.
2. A controlling expression that is a null test of one pointer member
   `B->F` lowers to the typed read of `B` (no refinement).
3. Before every call to a function defined in the unit, `ArgumentReads` over
   the plain-name actuals refuses an actual whose target owns sub-custody.

## Prediction

* **Functions Unknown → Safe: 13 to 44.** The lower figure admits only the
  member releases whose callee is a declared nullable release (`free`,
  `rd_free`, `flb_free`, `mk_mem_free`, `cfl_sds_destroy`, …); the upper admits
  every release-shaped callee, whose summary may or may not be nullable. Both
  are a textual estimate over the BEFORE capture
  ([scripts/predict_estimate.py](scripts/predict_estimate.py)); the listed
  lower movers are `cio_error_set`, `destroy_label`,
  `cmt_histogram_buckets_destroy`, `cmt_opts_exit`, `ctr_id_destroy`,
  `destroy_chunk_raw`, `rd_kafka_Node_destroy`, `rd_kafka_Node_free`,
  `rd_kafka_consumer_group_metadata_destroy`,
  `rd_kafka_mock_error_stack_destroy`,
  `rd_kafka_share_ack_batch_entry_destroy`,
  `rd_kafka_share_ack_batch_entry_destroy_free`, `rd_kafka_snappy_free_env`.
* **Uncontracted-call sole blocker (412)** drops by roughly half the movers
  (the `_free` wrappers and callers of the destructors): **−5 to −20**.
* `unsupported` falls by several hundred (up to ~437 member-release and ~1,000
  member-null-test leaves are candidates; each is removed only where its base
  obligations and the rest of its statement admit). `unknown_alias` **rises**:
  a member null test or release over a base that is unresolved reports
  `unresolved resource identity` where the refusal stood.
* **Leaks:** 37 ± a few. A newly modeled destructor body releases nothing a
  caller tracks, so no caller-side leak is created by rule 1; a new leak can
  only appear where a newly admitted statement exposes an exit that was
  unmodeled, and each is classified by reading.
* **The guard (rule 3)** refuses only where an actual owns sub-custody at a
  call to a unit-defined function: 0 to 5 functions may leave `Safe`
  (refusing direction), each attributed.
* Units: no move expected.
