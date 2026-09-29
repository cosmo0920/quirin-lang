# CG03-CALL-SUMMARY-INSTANTIATION: frozen prediction

Frozen before the AFTER corpus arbiter run and committed before the
implementation commit. It re-freezes CG00's
[CG03 prediction](../cg00-clang-gap-baseline/predictions/CG03.md), amended only
where a reason is given. The route is G (the custody gateway). The fixture
probes of the three new routes ran before this file was written. No corpus,
system or slice run of the implementation had run.

## Base and BEFORE

- **Base.** Local `main` `f19204fa` (= origin/main, round 2 landed).
- **BEFORE.** The release `quirin-audit` built at the base, run into
  `private/cg03/before`. It reproduces CG02's archived AFTER byte for byte:
  `resource-diagnostics.json` `963d7c9f…`, 543 / 10 / 4,281.
- **CG00's function list on the current baseline.** Re-derived by
  `scripts/rederive.py`. All 40 functions CG00 attributed to K12/K03 are
  still non-Safe (39 Unknown, 1 Unsafe), and all 43 leaves are present at
  the same lines: 22 return, 15 K03, 6 argument-shape. No predicted function
  has moved since CG00's `ae17152b` baseline.

## What the card changes (upper bounds)

1. **Return operand** (K12, 22 CG00 leaves; 267 return leaves corpus-wide).
   A refused `return` in a scalar-result function that is one condition leaf
   with calls is admitted exactly as the same leaf in an `if` is. Each call's
   contract must be in force in this translation unit, with a resource-free
   result. The callee must be outside the caller's recursive component. Two
   calls must each be the identity.
2. **Typed substitution** (K12 argument-shape, 6 leaves; K03, 15 leaves).
   A value-preserving pointer cast of a named object is that object. Data
   whose evaluated reads are scalars is data.
3. **Effectful actuals** (CG02's 3 `cg_sink(i++)`-shaped consultations).

## Predicted verdicts

**Safe (at most 11) of CG00's 13 strict freed-alone functions:**

- `rd_hdr_nextNonEquivalentValue` and `rd_hdr_medianEquivalentValue`: two
  local identity calls.
- `rd_hdr_highestEquivalentValue`: a cascade through the first one's
  validated summary.
- `LZ4HC_hashPtr`: one local call in a macro's arithmetic.
- `rd_kafka_broker_ApiVersion_at_least` and `…_no_lock`.
- `rd_kafka_cgrp_assignment_is_lost` and `cmt_metric_has_start_timestamp`.
  Both are member-address actuals that the classifier already admits in a
  condition.
- `rd_kafka_topic_result_free`, `rd_strtup_free` and
  `rd_kafka_metadata_destroy`: a cast of a formal at a `Released` position
  (typed substitution).

**Amended: CG00's `rd_kafka_msg_partitioner_fnv1a` and `…_murmur2` stay
Unknown.** Reason: their callees `rd_fnv1a` and `rd_murmur2` are defined in
other corpus units. CG00 counted "a corpus function" as contracted, but this
unit has no contract in force for them: B01 would already have admitted an
externally declared contracted callee here. The route admits no uncontracted
callee.

**Unknown:**

- The four alias-held functions CG00 named (`rd_kafka_topic_partition_hash`,
  `rd_kafka_sasl_cyrus_ready`, `rd_kafka_assignment_lost`, `person_hash`).
- Every K03 function whose actual is pointer arithmetic (CG16:
  `rd_array_shuffle`, `rd_flags2str`, `rd_string_render`, the
  `flb_input_chunk.c` three, `mk_rconf_read`).
- `cmt_create`, `rd_kafka_conf_kv_split` and `rd_kafka_ApiVersions_copy`, whose
  lexical refusal a `sizeof` fixture did not reproduce: at most these three
  move, and only if their refusal is the typed substitution's case.
- The `rd_kafka_curr_msgs_get` callers (`rd_kafka_destroy_app`,
  `rd_kafka_dump0`, `rd_kafka_stats_emit_all`) unless their `&local` actuals
  are the missing custody substitution.

**Beyond CG00's list:** further Unknown → Safe moves are possible. There are
245 other refused returns, and a newly Safe callee cascades to its callers.
Each move is attributed in the AFTER.

## Invariants predicted to hold

1. No Safe → non-Safe move.
2. No Unknown → Unsafe move is *expected*. A newly admitted scalar return makes
   the exit a decided exit, so a leak held across it may now be reported.
   Every added leak or violation is classified by reading.
3. Sidecars (`monkey-source.json`, `…chunk-contracts.json`, `…contracts.json`,
   `…source-effects.json`) are byte-identical.
4. `W44-CALL-ARGUMENT-CUSTODY-POSITION` refusals are unchanged: a place at a
   custody-taking formal stays refused by the typed substitution.
5. No leaf moves in the condition desugaring. Its calls are now instantiated
   through the one application point with identical refusals.
