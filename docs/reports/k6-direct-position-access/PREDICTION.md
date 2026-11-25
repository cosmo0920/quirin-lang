# K6-DIRECT-POSITION-ACCESS: prediction

Frozen before the rule-81 implementation commit and before any measurement of
an implementation build. The integrator measures with base `rev:5087f9295` (or
its landed successor). The forecast reads the re-census of the user's
2026-11-24 `current` run ([README](README.md) §1), the per-row reading of the
25 rows bundle K6 frees alone (§2), and unit probes on the card's test harness
(`probes/`, the scratch harness `probes/k6_probe_scratch.rs.txt`, real Clang,
the synthetic `k_*`/`d_*` contracts). The unit-scoped reproduction of the
production rows (`scripts/run_repro.sh`, base and tip binaries) was still
queued for an audit slot when this file was committed.

## Mechanism and what it can move

**Rule 81** reads `(&X)->f` as `X.f`, rule 10n (1)'s own reading, in two more
readers (each condition a formula; `#plain(#a)` is the same occurrence in the
plain spelling `p SEL . f`):

* the lexical `may_bind` reader: for every single `&` before a name,
  `#exempt(#a) == TRUE -> #binds(#a) == #binds(#plain(#a))`, where
  `#exempt(#a) == TRUE` holds exactly when
  `#opens_group(#a) == TRUE && #name_arrow(#a) == TRUE && #selection_chain(#a) == TRUE && #group_arrow(#a) == TRUE && #outer_address(#a) == FALSE`;
* the elaborator's place reader: for every arrow member over an address of a
  member or subscript place, `#place(#e) == #place(#plain(#e))`.

Both only remove a binding or a refusal, and only where the plain spelling
has none (`QCore.AddressArrowPlace.the_rule_only_adds`,
`the_rule_only_removes`, `at_least_the_plain_reader`). A row moves only when
every remaining leaf of it is such a spelling.

Of the 25 rows bundle K6 frees alone on 11-24, three are refused at the base
for this spelling alone (the plain spelling of each, probed over the unit's
own typedef shapes, is `Safe` at the base; the `(&X)->` spelling is `Safe`
only with rule 81):

* `rd_kafka_mock_partition_next_leader_response` (`rdkafka_mock.c:788`,
  `return TAILQ_FIRST(&mpart->leader_responses);`, its only leaf): rule 81 (1),
  the pointer-return premise.
* `rd_kafka_mock_msgset_est_record_size` (`rdkafka_mock_handlers.c:3478`,
  `size = (int32_t)(RD_KAFKAP_BYTES_LEN(&mset->bytes) / record_cnt);`, its
  only leaf): rule 81 (2), the rule 10z edge.
* `rd_kafkap_leader_discovery_tmpabuf_add_alloc_brokers`
  (`rdkafka_request.c:3587`, `size_t HostSize = RD_KAFKAP_STR_LEN(&NodeEndpoints->NodeEndpoints[i].Host) + 1;`,
  its only leaf): rule 81 (2) over a subscripted member.

The eight-home corpus (Fluent Bit `66910c10`) holds the same three units with
byte-identical sources (`rdkafka_mock.c`, `rdkafka_mock_handlers.c`,
`rdkafka_request.c`, `rdkafka_proto.h`, `rdsysqueue.h`, `rdkafka_mock_int.h`;
[scripts/same_sources.py](scripts/same_sources.py)), so the same three rows
are forecast there. None of their same-unit callers moves: each also carries
another blocker (`rd_kafka_mock_Fetch_reply_tags_partition_write`,
`rd_kafka_mock_buf_write_Metadata_Topic`, `rd_kafka_mock_sgrp_acquire_available_offsets`,
`rd_kafka_handle_Fetch_metadata_update`, `rd_kafka_handle_Produce_metadata_update`,
`rd_kafkap_share_leader_changes_apply` keep alias, IND, SYNTAX or UNC leaves).

## Predicted moves

| Route | Function | Today | Predicted |
|---|---|---|---|
| `current` (user run) | `rd_kafka_mock_partition_next_leader_response` | Unknown | Safe |
| `current` (user run) | `rd_kafka_mock_msgset_est_record_size` | Unknown | Safe |
| `current` (user run) | `rd_kafkap_leader_discovery_tmpabuf_add_alloc_brokers` | Unknown | Safe |
| corpus (eight-home) | the same three | Unknown | Safe |
| twelve-home | the same three | Unknown | Safe (the corpus inputs) |
| system, slice | — | — | no move by name |

No `Safe -> non-Safe` move on any route: rule 81 adds readings only. No new
`Unsafe` row is forecast; a read after a release spelled `(&X)->f` is now the
same `UseAfterRelease` accusation as its plain spelling (fixture
`near_misses_decide_as_their_plain_spelling`), which is the only way an
`Unsafe` could appear. The spelling is common in list-walk expansions
(`mk_list_foreach`, `cfl_list_foreach`, `TAILQ_FOREACH`: 192 K6 leaf groups
on 125 functions of `current` spell it on their line,
[census/arrow-census.json](census/arrow-census.json)), so a further move of
the same class on system or slice (a function whose only remaining leaves are
such spellings) is possible but not forecast by name.

## Predicted leaf changes

* `current` and corpus: between 3 and about 190 IND/COND/RET leaves removed
  where the refused place or the pointer-return premise is a `(&X)->f`
  spelling (the arrow census's 192 groups is the ceiling for K6's tags;
  call-actual leaves of the same spelling may also go), each possibly
  exchanged for another refusal of the same statement or an `unknown_alias`
  at the read (a bound root: a refusal, never an accusation).
* Zero change of `leak`, `double_release`, `summary_conflict` counts.

```measure-prediction
route corpus
move rd_kafka_mock_partition_next_leader_response Unknown -> Safe
move rd_kafka_mock_msgset_est_record_size Unknown -> Safe
move rd_kafkap_leader_discovery_tmpabuf_add_alloc_brokers Unknown -> Safe
delta Safe +3
route twelve-home
move rd_kafka_mock_partition_next_leader_response Unknown -> Safe
move rd_kafka_mock_msgset_est_record_size Unknown -> Safe
move rd_kafkap_leader_discovery_tmpabuf_add_alloc_brokers Unknown -> Safe
delta Safe +3
route system
none
route slice
none
```

## Expected effect on `current`

Three rows Unknown -> Safe, 1,173 -> 1,176 Safe (on the 11-24 report; the
integrator's `current` rerun after the round-37 landing is the measurement).
The census's band for the bundle (8 Safe, 5 to 10) is not reached: of the 25
rows K6 frees alone on 11-24, 16 are refused on purpose by earlier cards
(interior-address returns, a pointer difference, a sibling call beside a
deciding operand, file-scope record stores, formal copies; README §1.1) and 6
of the other 9 need readings this card leaves as named follow-ups (README §4).
