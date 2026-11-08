# Landing checkpoint 556460e31

`land.py publish` measured main `556460e31f4b0fe00cfa477855771ed50f209236`
(integrate/r7, 14 cards; full gate on the identical tree `8732e0ab8` passed:
Rust 627 units, 9,384 passed / 0 failed / 17 ignored, Lean 426 audits, Python
756 tests, providers 26 tests) against the newest published baseline
`7b726587bdf6a8213f451fa1c78297bb5e476ef7`. All three routes completed with no
unmatched rows and no composition conflicts. The publication refused because
the corpus (`eight-home`) and system (`fluent-bit-system`) figures moved;
`fluent-bit-tail-pause` reproduces its baseline. This page records why the
moves are the landed cards' measured moves.

[Complete comparison](native-comparison.json.gz) (gzip of the measurement's
`native-comparison.json`, sha256
`6e2d69332b6ec02f0d6581b6f7bc7554a56a3543b026e15e5f8945fff92ecff4`; run
directory `private/measure/landing-556460e31f4b0fe00cfa477855771ed50f209236-20261108T202723640567Z`).

## Figures

| Route | Before (7b726587b) | After (556460e31) |
|---|---|---|
| corpus functions | 1,054 Safe / 3,769 Unknown / 11 Unsafe | 1,079 Safe / 3,745 Unknown / 10 Unsafe |
| corpus units | 9 Safe / 163 Unknown / 9 Unsafe / 4 data-only | 9 Safe / 164 Unknown / 8 Unsafe / 4 data-only |
| corpus NULL | 2,549 Safe / 2,355 Unknown / 17 Unsafe | unchanged |
| system functions | 101 Safe / 343 Unknown / 4 Unsafe | 103 Safe / 341 Unknown / 4 Unsafe |
| system NULL | 211 Safe / 243 Unknown | unchanged |
| slice | | unchanged |

Every Unsafe is provisional; none is presented as an upstream defect.

## Attribution

Each of the 28 moved rows (26 corpus, 2 system) appears with the same move in
a landed card's own declared measurement (its `integration/` record), and
every move those measurements report appears here, except one that nets out:

- OUT-PARAMETER-SCALAR-FRAME-PREMISE: `cio_memfs_content_copy`,
  `cio_utils_read_file` (Unknown to Safe).
- OUT-PARAMETER-BODY-SUMMARY: `rd_kafka_message_timestamp`,
  `rd_kafka_toppar_get_avail` (Unknown to Safe).
- SCALAR-STATEMENT-FORMS (rule 41): `LZ4_createStreamDecode`,
  `LZ4_getIndexOnHash`, `LZ4_initStream`, `LZ4_stream_t_alignment`,
  `LZ4F_returnErrorCode`, `LZ4HC_rotatePattern`, `LZ4_streamHC_t_alignment`,
  `rd_kafka_version_str`, `XXH32_canonicalFromHash`,
  `XXH64_canonicalFromHash`, `_next_target` (Unknown to Safe; the two
  unpredicted moves are justified in its report).
- MEMBER-EFFECT-COMPONENT-SCOPE: `cmt_opts_exit`, `ctr_id_destroy`,
  `destroy_chunk_raw` (corpus) and `destroy_chunk_raw` in both system
  configurations (unit#0, unit#1) (Unknown to Safe).
- MEMBER-RELEASE-UNTYPED-BASE: `rd_kafka_Node_destroy`, `rd_kafka_Node_free`,
  `rd_kafka_mock_error_stack_destroy`, `rd_kafka_mock_sgrp_record_release`,
  `rd_kafka_share_ack_batch_entry_destroy`,
  `rd_kafka_share_ack_batch_entry_destroy_free`, `mk_ptr_free` (Unknown to
  Safe).
- CHAINED-ASSIGNMENT-INNER-DESTINATION: `rd_kafka_sasl_safe_string` and unit
  `rdkafka_sasl_scram.c` (provisional Unsafe to Unknown; the modelling defect
  of CURRENT-UNSAFE-TRIAGE).
- Netted out: `cio_file_content_copy` goes Unknown to Safe in
  OUT-PARAMETER-SCALAR-FRAME-PREMISE and back to Unknown in
  OUT-SLOT-CALL-WRITE-ALIASING, so it does not move against the baseline.

No Safe row moved to Unknown or Unsafe.

## NULL documents

The NULL verdict digests of the corpus and system changed but no NULL verdict
moved and no row's `null` dictionary changed (4,921 corpus, 454 system and
406 slice NULL rows compared by library, file and function). The carrier's
`custody` field follows the custody moves above (corpus 25 Unknown to Safe, 1
Unsafe to Unknown; system 2 Unknown to Safe) and its `combined` field changes
in 19 corpus rows (18 Unknown to Safe, 1 Unsafe to Unknown) and the 2 system
rows; every other difference is `hypothesis_context` carrying the new
evidence version.
