# SLOT-MOVE-AND-INTERIOR-BORROW: prediction

Frozen before any implementation code or contract. Base: local `main` at
`6cbcc4e9c`.

## BEFORE (reproduced)

Binary `private/smib/bin/before`, sha256 `03a961a7…` (equal to `run.json`'s
`binary_sha256`):

| route | Safe / Unsafe / Unknown | Pending | digest |
| --- | --- | --- | --- |
| corpus `66910c10` | **846 / 9 / 3,979** | 0 | `686fca16…` (byte for byte the brief's) |
| system | 87 / 4 / 357 | 0 | `95df8068…` (embeds this worktree's paths) |
| slice | 45 / 1 / 354 | 4 | `490bca64…` (embeds this worktree's paths) |

Ranking (`scripts/rank.py`): population **571**, **403** decided by one callee
alone, as W43-ABI-ENUM-AND-CONST-PP left it (system 26 / 18, slice 38 / 27).

## Membership re-measured on main

`scripts/size_members.py` ([size-before-corpus.json](artifacts/size-before-corpus.json);
leaves naming the callee / functions carrying one / functions it decides alone):

| class | member | leaves / functions / alone | decided-alone functions |
| --- | --- | ---: | --- |
| 2 | `sds_cat_safe` | 46 / 9 / 4 | `format_bool`, `format_double`, `format_int64`, `format_string` (`ctr_encode_text.c`) |
| 2 | `cfl_sds_cat_safe` | 170 / 36 / 3 | `format_metric_type` (splunk_hec), `append_bytes_value` (text), `sds_cat_safe` |
| 2 | `format_metric` | 14 / 6 / 3 | the three `format_metrics` (influx, splunk_hec, text) |
| 2 | `mk_header_prepare` | 10 / 6 / 2 | `headers_setup`, `mk_plugin_header_prepare` |
| 2 | `rd_kafka_parse_Metadata0` | 2 / 2 / 2 | `rd_kafka_parse_Metadata`, `_admin` |
| 2 | `setupRackAwareAssignment0` | 33 / 11 / 2 | two unit-test `setupRackAwareAssignment` |
| 2 | `ut_init_member_internal` | 2 / 2 / 2 | `ut_init_member`, `_with_rackv` |
| 2 (system) | `flb_config_task_map_grow` | system ranking 1 alone | |
| 8 | `rd_kafka_admin_result_ret_groups` | 4 / 4 / 4 | four `*_result_groups` wrappers |
| 8 | `rd_kafka_admin_result_ret_resources` | 3 / 3 / 3 | three `*_result_resources` wrappers |
| 8 | `rd_kafka_admin_result_ret_topics` | 3 / 3 / 3 | three `*_result_topics` wrappers |
| 8 | `get_object_item` | 2 / 2 / 2 | `cJSON_GetObjectItem`, `_CaseSensitive` |
| 8 | `get_or_create_external_metadata_kvlist` | 20 / 7 / 2 | `decode_resource_metrics_metadata`, `decode_scope_metrics_metadata` |
| 8 | `rd_kafka_topic_partition_get_private` | 12 / 10 / 2 | `rd_kafka_topic_partition_set_leader_epoch`, `_set_current_leader_epoch` |
| out slot | `rd_slice_reader0` | 2 / 2 / 2 | `rd_slice_reader`, `rd_slice_peeker` |
| out slot | `cJSON_ParseWithLengthOpts` | 2 / 2 / 2 | `cJSON_ParseWithLength`, `_ParseWithOpts` |
| out slot | `strtol`, `strtoll`, `strtoul`, `strtoull`, `strtod` | 9 / 9 / 0 | none alone |

Every size reproduces the ledger's (class 2: 7 callees / 18; class 8: 6 / 16)
except that `cfl_sds_cat_safe` now decides `sds_cat_safe`'s own row.

## Reading the members (both revisions, every body byte-identical)

* **Pure slot moves:** `cfl_sds_cat_safe` (`*buf` replaced by
  `cfl_sds_cat(*buf, …)` only when it succeeds; a NULL slot or NULL `*buf`
  does nothing), `sds_cat_safe` (forwards its slot to it), and the four
  `format_metric` bodies (forward the slot to `cfl_sds_cat_safe` and to other
  static helpers).
* **Not slot moves:** `mk_header_prepare` (in-place lifecycle of `sr`'s iov
  with free-on-release flags), `rd_kafka_parse_Metadata0` (a fresh owned
  metadata into the out slot on success, `resource_out` under a disposition),
  `setupRackAwareAssignment0` and `ut_init_member_internal` (unit-test
  constructors building into caller storage). None is declared.
* **`flb_config_task_map_grow`** reallocates `config->task_map`, a field
  reached from a borrowed parameter, not a `T **` slot parameter: not covered
  by the slot reading; recorded, not declared.
* **Class 8:** `ret_groups`/`_resources`/`_topics` return the op's result list
  array (`rko->…results.rl_elems`); `get_object_item` a child of `object`;
  `rd_kafka_topic_partition_get_private` the partition's `_private`, created
  and linked in on first use; `get_or_create_external_metadata_kvlist` a child
  fetched through `cfl_kvlist_fetch` or created and inserted through a
  custody-taking insert.
* **Out slot:** `rd_slice_reader0` writes `seg->seg_p + rof` (reached from
  `slice`) through `p`, and only when it returns nonzero; the `strto*`
  functions write a pointer into `nptr` through a nullable `endptr`;
  `cJSON_ParseWithLengthOpts` writes into `value` through
  `return_parse_end` and stores `value` in the file-scope `global_error`.

## Design (frozen)

* **Class 2, rule 12x.** `ensures normal resource_slot_moved(s, N);` on a
  `ptr<mut, ptr<mut, u8>>` parameter that is `resource_borrowed`. Withheld
  from the function map with the retaining declarations (route G only). At
  a call: `&L` with `L` bound to exactly one live tracked identity that no
  structure other than names holds: the slot keeps the identity (it always
  owns exactly one object), every **other** name bound to it is invalidated
  (its later use refuses, `slot-move-alias`, never accused); `&L` with `L`
  definitely NULL under `nullable`: nothing moves; a formal slot the caller
  names only as whole actuals of slot-moving positions (or null tests):
  passed through. Everything else refuses (`slot-move-undecided`). A
  composition with a return-code disposition (rule 11r) is admitted by the
  loader only as a refusal-free reading if a member needs one; none does.
* **Class 8, rule 12y.** `ensures normal resource_return_borrowed_from(p, N);`
  loads to a derived result with a new extent, `Borrowed`: no release
  obligation; a release of it refused (never accused); a parent release or any
  call that may mutate the parent (it receives the parent, an alias of it,
  an ancestor, or any other plain name) ends it, and a later use refuses
  (`interior-borrow-ended`); a store through it is a write to the parent
  (admitted where the parent is live or a caller's object, and it ends the
  parent's other borrows). A lazily created child is the same mode; its body
  check admits a child created and linked into the parent.
  Derived-result inference over a `Borrowed` exit publishes a `Borrowed`
  summary (never a plain interior one). The derived base reader reads through
  a pointer cast of a place.
* **Out slot, rule 12z.** `ensures normal resource_out_derived(s, p, N);`:
  the slot receives a pointer derived from `p` at the interior extent (C04.3.1)
  or keeps its old value; it owns nothing. Withheld. At a call: `&L` with `L`
  holding no tracked identity: `L` becomes a derived binding over `p`; a
  formal slot named only as whole actuals of such positions: passed through;
  NULL under `nullable`: nothing; everything else refuses.
* **Body checks, rule 13a.** In a unit that defines a declared member, each
  clause stands only when the body validates it structurally (slot: every
  mention of the slot a null test or a whole actual at a slot-moving
  position; borrowed result: every return NULL or reached from the parent by
  member reads, through locals so assigned, or a child created and stored
  into a member of the parent; out slot: every store through the slot reached
  from the base, every other mention a null test or a pass-through).
  Otherwise the declaration is withdrawn in that unit.

## Predicted outcome

| item | prediction |
| --- | --- |
| class 8 corpus | **+10 to +14**: the ten `ret_*` wrappers (needs the cast base and the borrowed summary), `cJSON_GetObjectItem` and `_CaseSensitive` (2), the two `set_*leader_epoch` (2; needs the store through the borrow). `get_or_create_external_metadata_kvlist`: 0 (its body check needs `cfl_kvlist_fetch` and the created-and-inserted child; not declared or withdrawn) |
| class 2 corpus | **+4 to +7**: `format_bool`/`_double`/`_int64`/`_string` (4), `format_metric_type`, `append_bytes_value`, `sds_cat_safe` (3). `format_metric`: 0 (its body forwards the slot to undeclared static helpers; no slot-move summary inference). The other four members: 0 |
| out slot corpus | **+2**: `rd_slice_reader`, `rd_slice_peeker`. `cJSON_ParseWithLengthOpts`' callers: 0 (a formal retained by `global`). `strto*`: 0 |
| cascade | 0 to +6 among callers of the newly summarised wrappers (`cJSON_GetObjectItem` callers, the `*_result_*` accessors' callers) |
| corpus total | **+16 to +29** (central +23) |
| Safe → non-Safe | none |
| new leaks / violations / Unsafe | none (every new rule refuses, never accuses) |
| re-attributed Unknown functions | 20 to 50 (other blockers behind the declared calls) |
| system | 0 to 2 (`cfl_sds_cat_safe` callers), `flb_config_task_map_grow` not covered |
| slice | no move |
