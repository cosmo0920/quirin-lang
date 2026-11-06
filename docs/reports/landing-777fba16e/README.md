# Landing checkpoint 777fba16e

`land.py publish` measured main `777fba16efa49c67634fd9eec0b3db12051b0edd`
(integrate/r4, full gate passed: Rust 9,272 passed / 0 failed / 16 ignored,
Lean 413 audits, Python 659 tests) against the newest published baseline
`c8c7edf2165a2065c234f3a63b4eed23e937ced6`. All three routes completed with no
unmatched rows and no composition conflicts. The publication refused because
the corpus (`eight-home`) and slice (`fluent-bit-tail-pause`) figures moved;
`fluent-bit-system` reproduces its baseline. This page records why the moves
are the landed cards' measured moves.

[Complete comparison](native-comparison.json.gz) (gzip of the measurement's
`native-comparison.json`, sha256
`737b19e09ccc4bd5f20be0f49dc7a441c01271cebb6a565eb62b96014320a631`; run
directory `private/measure/landing-777fba16efa49c67634fd9eec0b3db12051b0edd-20261106T162852512228Z`).

## Figures

| Route | Before (c8c7edf21) | After (777fba16e) |
|---|---|---|
| corpus functions | 1,013 Safe / 3,811 Unknown / 10 Unsafe | 1,054 Safe / 3,769 Unknown / 11 Unsafe |
| corpus units | 8 Safe / 165 Unknown / 8 Unsafe / 4 data-only | 9 Safe / 163 Unknown / 9 Unsafe / 4 data-only |
| corpus NULL | 2,549 Safe / 2,355 Unknown / 17 Unsafe | unchanged |
| slice functions | 57 Safe / 342 Unknown / 4 Pending / 1 Unsafe | 58 Safe / 341 Unknown / 4 Pending / 1 Unsafe |
| system functions | 101 Safe / 343 Unknown / 4 Unsafe | unchanged |

Every Unsafe is provisional; none is presented as an upstream defect.

## Attribution

Each of the 43 moved rows (42 corpus, 1 slice) appears with the same move in a
landed card's own declared measurement since `c8c7edf21`, and every move those
measurements report appears here:

- LIBC-GAP-DECLARATIONS: `cio_file_native_resize`, `cio_file_native_sync`,
  `rd_kafka_transport_shutdown`, `_rd_strcasestr`,
  `mk_plugin_sched_get_thread_conf` (Unknown to Safe).
- CFL-MPACK-EXTERNAL-DECLARATIONS: 15 cmetrics/ctraces rows to Safe, unit
  `ctr_attributes.c` to Safe, and `decode_statsd_message` with unit
  `cmt_decode_statsd.c` to provisional Unsafe (the validated double release of
  its README §4.1).
- LIBRDKAFKA-HEADER-INLINES (integrate-r4 comparison): the 17 librdkafka rows
  to Safe; its comparison also contains the two cards above.
- MEMBER-EFFECT-SUMMARY: `metric_release_storage`,
  `cmt_metric_exp_hist_snapshot_destroy`, `rd_kafka_metadata_partition_clear`,
  `free_chunk_header` (corpus) and `flb_tail_fs_stat_remove` (slice) to Safe.

No Safe row moved to Unknown or Unsafe.

## NULL documents

The NULL verdict digests changed but no NULL verdict moved: all 4,921 corpus
and 406 slice NULL rows keep their verdict. Fourteen corpus NULL dictionaries
change content only (checks and contract dependencies). The carrier's
`combined` field changes in 19 corpus rows (18 Unknown to Safe, 1 Unknown to
Unsafe) and one slice row, matching the custody moves above; every row's
`hypothesis_context` changes with the evidence version.
