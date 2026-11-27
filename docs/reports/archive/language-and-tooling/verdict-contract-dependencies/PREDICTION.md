# VERDICT-CONTRACT-DEPENDENCIES: prediction (frozen before implementation)

Base: local `main` at `cdc422ad1`. Frozen before any implementation code.

## Verdicts

* No verdict moves on any route. Corpus stays **881 / 9 / 3,944**, system
  87 / 4 / 357, tail-pause slice 45 / 1 / 354 with 4 Pending (the brief's
  figures; BEFORE is measured, not assumed).
* The verdict-only projection of every AFTER report (the report with the new
  per-row `contract_dependencies` key, the new per-unit evidence key and the
  new sidecar section removed) is byte-identical to the BEFORE report, so its
  SHA-256 equals BEFORE's raw `resource-diagnostics.json` digest on all three
  routes. The typed-path parity streams are identical in order after address
  erasure (nothing the card adds is read by a parity consumer).
* The `premise` annotation added to `rd_kafka_q_wait_result` and
  `rd_kafka_q_purge0` changes their `contract_sha256`; after re-pinning at
  `66910c10` and `046accf6` both stay pinned in force, so no call is held and
  no verdict moves. Without the re-pin they would be withheld
  (`contract-pin-mismatch`) and `rd_kafka_toppar_purge_and_disable_queues`
  would go Pending: that is the control that the premise is covered by the pin
  digest.

## The dependency field

* Every analysed row carries the declarations its derivation consulted. Safe
  rows: most rest on at least one declaration; a minority rest on none (pure
  arithmetic or read-only bodies).
* Body standing: few declarations are **validated** by a body in an analysed
  unit (most declarations are libc, external-library or Fluent Bit API
  functions whose bodies the corpus does not analyse, or whose bodies analyse
  to Unknown). Predicted: of the corpus's 881 Safe rows, fewer than 15% rest
  only on validated declarations (or none); the large majority rest on at
  least one declared assumption.
* The most-depended-on declared assumptions are the libc and Fluent Bit
  allocator and string/list primitives (`free`, `malloc`/`calloc`,
  `flb_free`/`flb_malloc`/`flb_calloc`, `flb_sds_destroy`, `mk_list_*`
  style helpers); `rd_kafka_q_purge0` appears on
  `rd_kafka_toppar_purge_and_disable_queues`'s Safe row with the
  `served_queue` premise and standing *declared assumption*.
* Size: names in the rows add roughly 10–20% to the corpus
  `resource-diagnostics.json` (about 10 MB). Above 25% the rows switch to a
  compact encoding.

## mk_vhost_set tip probe

Both rows stay Unknown with the same diagnostics; each now lists its
dependencies.
