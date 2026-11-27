# MEMBER-EFFECT-STORES, body half: prediction

Committed before the semantic change. Base `c5cd10688` (integrate/r34-k-batch);
card tip before this file `d3cdc0be6` (the re-census and its stop). USER
DECISION 2026-11-25 (verbatim): "Option 1: caller-slot premise", and (verbatim)
"Plus, banned for just written down as English description. We must write down
with logical symbols for them."

## What was known when this was written

From the re-census ([README.md](README.md) section 1,
[evidence/rows-1124.json](evidence/rows-1124.json)), on the user's `current`
report of 2026-11-24 (1,173 Safe / 3,750 Unknown / 11 Unsafe): the 21 rows K3
left to this owner are Unknown on their store leaves alone
(`indirect memory access is outside the resource projection`).

## Planned change

New lettered rule **38k** (38k and 79 are free on the base and on origin/main;
38i is a re-attribution that never admits, so the admission does not fit
there). On the typed statement route, after rule 72 and before rule 12k, a
gate-refused store `P->F[.G…] = V` is read as a **hand-off** of `V`'s custody
into the caller's object when:

* `P` is a formal no statement writes or takes the address of, the place is
  `P->F` followed by `.` selections only, every step an ordinary (non-union,
  non-bit-field) member, read from the Clang AST;
* `V` is a whole call whose contract in force returns `Owned` (never
  `Reallocated`: realloc self-assignment stays refused, no non-null clause and
  no provider change here), or a local this body acquired, each optionally
  under pointer-to-pointer casts only (brief item 4);
* H1: every other occurrence of the place is a null test or a `sizeof`
  operand; H2: the local is assigned once, mentioned before the store only as
  its acquisition, a null test, a member access or a `sizeof` operand, and
  never after it; H3: at the store the value is null or one live, unreleased
  identity of this body (decided on the flow); the store is in no loop and the
  body has no `goto`/`setjmp`; after the store no call releases or consumes
  anything and the formal is not handed on whole;
* the owner is untracked (rule 10v's owner decision) and the old value is left
  to the caller by the stated premise P-CG11-CALLER-SLOT, which every row with
  a hand-off carries in its assumptions as a typed formula.

The store is a rule-10v `PointerStore` of the new value class
`member-handoff`, so the body publishes no summary (rule 10v's gate); every
caller of it stays refused at the call.

## Predicted movements (the 21 rows on `current`)

* **Unknown -> Safe (11)**: `rd_buf_init`, `rd_kafka_assignment_init`,
  `rd_kafkap_Fetch_reply_tags_set_topic_cnt`, `rd_kafka_snappy_init_env`,
  `create_counter_instance`, `create_gauge_instance`,
  `create_untyped_instance`, `create_summary_instance`,
  `create_histogram_instance`, `create_exp_histogram_instance`,
  `mk_vhost_set_property`.
* **Unknown -> Safe only if the in-unit callee's derived summary returns
  `Owned` (2)**: `otel_span_set_attributes` (`set_attributes_from_ctr`),
  `otel_span_set_events` (`set_events_from_ctr`); otherwise they stay Unknown
  with the hand-off refused as a value whose call returns no custody.
* **Stay Unknown, refused by name (8)**: `rd_list_grow`,
  `rd_kafka_topic_partition_list_grow` (realloc self-assignment),
  `rd_kafka_sasl_scram_build_client_final_message_wo_proof`,
  `rd_kafka_sasl_scram_generate_nonce`,
  `rd_kafka_topic_partition_set_metadata_from_rktp_stored`, `mk_buffer_cat`
  (the place used after the store), `rd_kafkap_Fetch_reply_tags_set_topic` (a
  pointer hop below the formal), `unpack_opts_unit` (a local alias of the
  formal).
* **Callers**: none moves to Safe; each call of an admitted body stays refused
  (no summary).
* **No** Safe -> non-Safe move, no Unknown -> Unsafe move.

```measure-prediction
route corpus
move rd_buf_init Unknown -> Safe
move rd_kafka_assignment_init Unknown -> Safe
move rd_kafkap_Fetch_reply_tags_set_topic_cnt Unknown -> Safe
move rd_kafka_snappy_init_env Unknown -> Safe
move create_counter_instance Unknown -> Safe
move create_gauge_instance Unknown -> Safe
move create_untyped_instance Unknown -> Safe
move create_summary_instance Unknown -> Safe
move create_histogram_instance Unknown -> Safe
move create_exp_histogram_instance Unknown -> Safe
move mk_vhost_set_property Unknown -> Safe
route system
none
route slice
none
```

The corpus rows are expected only where the pinned corpus (Fluent Bit
`66910c10`, its librdkafka, cmetrics, ctraces and Monkey) holds the same
bodies; this card did not read the pinned trees, so a missing corpus move with
the same function Safe on `current` is a corpus-source difference, not a miss.
Any other corpus move must be this shape (a formal's member taking an owned
call result or a local acquisition as its only blocker) and is reported, not
counted as a miss. System and slice: no function of theirs is known to have
this as its only blocker. Twelve-home: the corpus moves where its homes hold
those units, nothing else. The integrator's `current` rerun: 11 (up to 13)
rows Unknown -> Safe, the corresponding store leaves removed, every unit
status otherwise unchanged.

## Addendum after the landing measurement (2026-11-25, not a prediction)

The block above had no twelve-home route, so the integrator's landing
measurement (`private/measure/r35-landing/native-comparison.json`, main
checkout) listed every twelve-home move as unpredicted. Written after that
measurement, for the record only: the twelve-home moves are the corpus ones.

```measure-prediction
route twelve-home
move rd_buf_init Unknown -> Safe
move rd_kafka_assignment_init Unknown -> Safe
move rd_kafkap_Fetch_reply_tags_set_topic_cnt Unknown -> Safe
move rd_kafka_snappy_init_env Unknown -> Safe
move create_counter_instance Unknown -> Safe
move create_gauge_instance Unknown -> Safe
move create_untyped_instance Unknown -> Safe
move create_summary_instance Unknown -> Safe
move create_histogram_instance Unknown -> Safe
move create_exp_histogram_instance Unknown -> Safe
move mk_vhost_set_property Unknown -> Safe
move ctr_link_set_trace_state Unknown -> Safe
```

`ctr_link_set_trace_state` moved on corpus and twelve-home, unpredicted
because this card did not read the pinned trees: the pinned ctraces
(`c-gateway-wave-pin`, `lib/ctraces/src/ctr_link.c:78-89`) spells
`link->trace_state = cfl_sds_create(trace_state); if (!link->trace_state)
return -1;`, rule 38k's admitted shape (README section 11). On `current`
(`046accf6`) the body releases the old value first and stays refused.
