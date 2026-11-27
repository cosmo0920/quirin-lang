# CG22-CALLBACK-REGISTRY-CONTRACT: prediction

Frozen before any implementation code. Base: local `main` at `84ec54d1d`
(DEEP-AST-TRANSPORTS-AND-WALL-TIME's gate records), no later main.

## BEFORE, reproduced

Binary `private/cg22r/bin/before` (`75e0dcb2…`, equal to the corpus
`run.json`'s `binary_sha256`), built from `84ec54d1d`:

| route | totals | report sha256 | wall (s, inside the lock) |
| --- | --- | --- | ---: |
| corpus | **881 / 9 / 3,944** | `ab8f5f14…` (the brief's) | 449.3 |
| system | 87 / 4 / 357 | `2efc56d6…` (embeds this worktree's paths) | 59.7 |
| slice | 45 / 1 / 354, 4 Pending | `b81b79d5…` (embeds this worktree's paths) | 28.2 |

## The members, sized on the BEFORE corpus

`scripts/size_members.py` ([corpus](artifacts/size-before-corpus.json); leaves
naming the callee / functions carrying one / functions it decides alone):

| member | shape | leaves / functions / alone |
| --- | --- | --- |
| `cmt_mpack_unpack_array` | synchronous invocation | 12 / 12 / 5 |
| `ctr_mpack_unpack_array` | synchronous invocation | 5 / 5 / 5 |
| `bsearch` | synchronous invocation, interior result | 7 / 6 / 1 |
| `qsort` | synchronous invocation | 4 / 3 / 0 |
| `pthread_once` | invocation (at most once, read as zero or more) | 2 / 2 / 1 |
| `cmt_mpack_unpack_map` | table dispatch | 10 / 10 / 3 |
| `ctr_mpack_unpack_map` | table dispatch | 9 / 9 / 3 |
| `rd_list_init` / `rd_list_new` | registration | 65 / 46 / 0, 33 / 26 / 0 |
| `rd_list_add` | element custody | 79 / 66 / 3 |
| `rd_list_destroy` | registry invocation, flag-conditioned self-release | 184 / 110 / 3 |
| `rd_list_clear` | registry invocation | 4 / 4 / 3 |
| `pb_write`, `pb_encode_varint` | registry invocation through `stream->callback` | 8 / 8 / 2, 5 / 4 / 2 |
| `rd_kafka_q_serve0`, `_pop_serve0`, `_serve_maybe_consume` | invocation over callee-supplied ops | 3 / 3 / 2, 3 / 3 / 2, 2 / 2 / 2 |
| `rd_kafka_op_destroy` | op callback member, type-switched union release | 132 / 83 / 2 |

System and slice carry none of these members except `pthread_once`
(system 2 / 1, slice 2 / 1).

**Every function a registry member decides alone reaches the list or stream
through a formal** (`rd_list_destroy(&rk->rk_conf.partition_assignors)`,
`rd_list_clear(&mcluster->request_list)`, `rd_list_add(&config->config,
entry)`, `pb_write(stream, …)` with `stream` a formal): no registration is in
force in the calling frame, so `callback_trace` cannot decide the invocation.
These stay **Unknown** under any registry vocabulary that is fail-closed.

**Every synchronous-invocation and table member's callback is itself
`Unknown`** in BEFORE (`scripts/callback_candidates.py`,
[candidates](artifacts/candidates-before.json)): of the 36 decoder calls
(cmetrics and ctraces), no table has all its handlers `Safe`, and every
array callback is `Unknown`. A callback with no contract in force makes the
invocation Unknown (the brief's rule), so these callers stay **Unknown** and
trade `call to uncontracted function `cmt_mpack_unpack_array`` for a named
callback refusal naming the uncontracted candidate.

## Callbacks currently read as benign

`scripts/registrations.py` over the parity census
([corpus](artifacts/registrations-before-corpus.json),
[system](artifacts/registrations-before-system.json),
[slice](artifacts/registrations-before-slice.json)): 1,224 corpus
registration rows over 109 callees (151 / 15 system, 255 / 20 slice). Two
callees are declared, both with the callback read as `resource_borrowed`:

| declaration | sites | callers |
| --- | --- | --- |
| `rd_kafka_topic_partition_list_sort` (`cmp` borrowed, invoked during the call; `RDKAFKA-SORT-COMPARATOR-EFFECTS`) | 3 corpus | **Safe:** `rd_kafka_topic_partition_list_sort_by_topic`, `rd_kafka_topic_partition_list_sort_by_topic_id`; Unknown 1 |
| `msgpack_packer_init` (`data` and `callback` borrowed, both kept in `pk`) | 2 corpus, 3 system, 2 slice | all Unknown |

Both comparators the two Safe callers pass (`rd_kafka_topic_partition_cmp_opaque`,
`rd_kafka_topic_partition_by_id_cmp_opaque`) are **Unknown** in BEFORE.

## The vocabulary (design, frozen here)

Every clause is `ensures normal`, total-or-reject (grammar G05), and a
declaration carrying one is **withheld from the function map** with the
retaining declarations (C04.14's precedent), so a consumer that knows nothing
of callbacks sees an uncontracted callee. A word `A` in an actual list is a
parameter name of the declaration (the callback receives that actual), `_` (a
value the callee supplies itself: an index, an element pointer of an object
it was handed, a buffer of its own), or, for a registry invocation only,
`element` (each element the holder holds).

1. **Synchronous invocation** (rule 13u): `resource_callback_invoked(F, A…)`.
   `F` a `fnptr` parameter, `resource_borrowed`. During the call only, `F`'s
   value is called zero or more times with `(A…)`; nothing keeps `F`.
2. **Table dispatch** (rule 13v): `resource_callback_table(T, A…)`. `T` a
   depth-one pointer parameter, borrowed, no kind: during the call only, the
   callee calls function pointers it reads from the array `T` designates,
   zero or more times, with `(A…)`.
3. **Registry** (rule 13w): `resource_callback_registered(F, H)` (after the
   call `H`'s registry holds `F`'s value, replacing any earlier one; a null
   `F` holds none), `resource_callback_registry_invoked(H, A…)` (during the
   call the callback `H` holds is called zero or more times with `(A…)`),
   `resource_callback_unregistered(H)` (after the call `H` holds no
   registration). `H` a depth-one pointer parameter, borrowed, no kind, or
   `result` beside `resource_return_owned(nonnull)` (a constructor).
4. **Element custody** (rule 13x): `resource_element_added(E, H)` (`E`
   becomes an element `H` holds: custody of `E`'s object passes to `H` when
   `H`'s registered callback releases its `element` argument, otherwise `H`
   only retains `E`, C04.14), `resource_elements_dropped(H)` (after the call
   `H` holds no element). **Flag-conditioned self-release**:
   `resource_released_if_constructed(H)` releases `H`'s object exactly when
   its registration in force was made by a constructor (`result` holder,
   `rd_list_new` setting `RD_LIST_F_ALLOCATED`), and nothing when it was made
   in place (`rd_list_init`'s `memset` clears the flag).
5. **Body checks** (rule 13y): in a unit that defines the declaration, a
   synchronous invocation stands only when the body mentions `F` only as the
   callee of direct calls whose actuals match `(A…)` position by position
   (a parameter word: that formal, unrebound; `_`: anything not naming `F`),
   and a registration only when the body mentions `F` only as the value of
   one store `H->M = F;`. Every other callback clause is a declared
   assumption (never covered, rule 13i).
6. **Rule 13z**: the two declarations that read a callback as borrowed are
   re-declared under the vocabulary (`rd_kafka_topic_partition_list_sort`:
   `resource_callback_invoked(cmp, _, _, opaque)`; `msgpack_packer_init`:
   `resource_retained_by(data, pk)` and `resource_callback_registered(callback, pk)`).

**Route G decides** (fail-closed; every refusal names its reason):
* the callback actual is a function designator of the unit (or, for a
  table, every designator of an automatic array's braced initializer, the
  array named only as whole actuals at table positions), or a null constant
  (registration only: no callback). Anything else is an **open set**:
  `callback-undecided`;
* every candidate has a contract in force (declared, or a validated in-unit
  summary; a withheld declaration is not one): else `callback-uncontracted`;
* **loop-safe composition**: every candidate's contract is the identity on
  the caller's state — exactly as many parameters as actuals, every
  parameter `resource_borrowed` with no kind, `resource_return_none()`, no
  side clause — so any number of invocations is the identity. Else
  `callback-effectful`. The one exception: an `element` position of a
  registry invocation may be `resource_released` (element custody);
* a registry invocation, an element addition, an unregistration or a
  self-release needs the holder's registration in force on every path:
  the holder is `&L` for a frame object named only at registry positions,
  or one tracked identity a constructor returned. Else
  `callback-registry-undecided`;
* an element added to a releasing registry moves into it only when the
  holder is a tracked identity (C04's containment: the holder's release
  cascades); into a frame holder it is refused
  (`callback-element-custody-undecided`);
* summaries are derived after every in-unit callback candidate (a
  designator at a callback position is a call-graph edge for W4's order).

**Not attempted** (recorded with sizes): `rd_kafka_op_destroy`'s
type-switched union release and its `rko_op_cb` member callback (a member
read through a pointer: an open set, rule 11p), the `rd_kafka_q_serve*`
family (the callback receives ops the callee owns and destroys them: a
callback releasing a callee-supplied object is refused by the loop-safe rule;
the reference part is balanced inside the body, so it would be decidable, but
every call would refuse), `pthread_getspecific`/`mk_sched_get_thread_conf`
(a result returning what a holder retains), `msgpack_pack_*` (the invoking
side of the packer registry: header inline bodies).

## Predicted movement

**Corpus.**

| function | BEFORE | predicted | why |
| --- | --- | --- | --- |
| `rd_kafka_topic_partition_list_sort_by_topic` | Safe | **Unknown** | named correction: its comparator `rd_kafka_topic_partition_cmp_opaque` is Unknown (rule 13z) |
| `rd_kafka_topic_partition_list_sort_by_topic_id` | Safe | **Unknown** | named correction: `rd_kafka_topic_partition_by_id_cmp_opaque` is Unknown |
| `rd_kafka_ApiVersion_check` | Unknown | Safe **or** Unknown | `bsearch`'s comparator `rd_kafka_ApiVersion_key_cmp` is a header static (`rdkafka_proto.h:208`) that reads two members and returns an `int`: Safe only if W4's scope summarises it as all-borrowed |
| `mk_http_thread_initialize_tls` | Unknown | Unknown | `pthread_once`'s routine allocates a TLS key (uncontracted) |

Totals: **879–880 / 9 / 3,944–3,945**. No other status move. No leak,
violation or `Unsafe` added: every function naming `rd_list_new` or another
registry member has other blockers (0 decided alone by a constructor), and an
unowed element or holder on a path with a refusal is refused, not accused.
Leaves trade: the 16 decoder functions decided alone by an unpacker, and every
other caller of a declared member, replace `call to uncontracted function`
with a named callback refusal (expected mostly `callback-uncontracted` for
the decoders, `callback-registry-undecided` for the rd_list and nanopb
callers, `retention-undecided` or `callback-registry-undecided` at
`msgpack_packer_init`).

**System and slice:** no status move; leaves change only at the
`msgpack_packer_init` sites (3 system, 2 slice) and `pthread_once`.

**Parity:** rows change only in functions whose leaves change, plus
`ContractCall` rows gaining the callback operands.

**Corrections expected:** exactly the two above. Any other Safe → non-Safe
move is a finding to be named in the report.
