# MEMBER-RELEASE-UNTYPED-BASE prediction

Rows are relisted against `baseline:7b726587b`
(`private/baselines/7b726587bdf6a8213f451fa1c78297bb5e476ef7/` in the main
checkout), where every row below is `Unknown`; the measurement compares
`rev:662dc54a4` (MEMBER-EFFECT-COMPONENT-SCOPE's measured tip) with this
card's last code commit, and no row below is one the parent's change moves
(its arm reads only an untyped member over a typed base). Committed after the
census (report §1) and before the semantic change.

## Revision 1 (current), committed before the implementation

**The change** (rule 38j amended again, in place). Where the lexical
call-actual recogniser refuses `B->F` as an **untyped base**, at a
`Released { nullable: true }` position of a place rule 5k or rule 38j
admitted, Clang's reading stands in under the parent's conditions (the
`LValueToRValue` value of an arrow `MemberExpr` named `F` over the parameter
`B`, an object pointer with no `volatile`/`_Atomic`, no array decay, no
function pointer) plus the base's: its type a pointer with no qualifier the
rule refuses, and the member Clang resolved a member named `F` of a complete
`struct` the unit defines (never a `union`'s; an incomplete record has no
member to resolve). For both arms the parameter's own declared type must
carry no `volatile`/`_Atomic` (the rvalue read drops it), and the value read
may be implicitly converted to the callee's object-pointer parameter
(`BitCast`/`NoOp`, `char *` to `void *` for `free`, `rd_free`,
`mk_mem_free`). Every lexical premise is unchanged.

### Rows (`baseline:7b726587b`)

**Corpus.** Seven functions move:

| row | body | predicted |
|---|---|---|
| `mk_core/mk_memory.c::mk_ptr_free` | `mk_mem_free(p->data); p->len = 0;`, `mk_ptr_t` a typedef of an untagged struct | Unknown -> Safe (rule 38j, `data` stale) |
| `rdkafka_mock_sharegrp.c::rd_kafka_mock_sgrp_record_release` | scalar reads and stores through three typedef-spelled bases, then `rd_free(state->owner_member_id); state->owner_member_id = NULL;` | Unknown -> Safe (rule 38j, `owner_member_id` cleared) |
| `rdkafka_aux.c::rd_kafka_Node_destroy` | `rd_free(node->host); if (node->rack) rd_free(node->rack); rd_free(node);`, `rd_kafka_Node_t` declared by disagreeing typedefs | Unknown -> Safe (rule 5k) |
| `rdkafka_aux.c::rd_kafka_Node_free` | `rd_kafka_Node_destroy((rd_kafka_Node_t *)node);` | Unknown -> Safe (the destructor's summary) |
| `rdkafka_mock.c::rd_kafka_mock_error_stack_destroy` | `if (errstack->errs) rd_free(errstack->errs); rd_free(errstack);` | Unknown -> Safe (rule 5k) |
| `rdkafka_share_acknowledgement.c::rd_kafka_share_ack_batch_entry_destroy` | `if (!entry) return; rd_free(entry->types); rd_free(entry);` | Unknown -> Safe (rule 5k) |
| `rdkafka_share_acknowledgement.c::rd_kafka_share_ack_batch_entry_destroy_free` | `rd_kafka_share_ack_batch_entry_destroy((…_t *)ptr);` | Unknown -> Safe (the destructor's summary) |

**Justification of each new Safe.**

- `mk_ptr_free(mk_ptr_t *p)`: `p` occurs only as `p->data` and `p->len`
  (P1); the place `(p, data)` has one event, `mk_mem_free(p->data)` at the
  nullable position of `monkey/memory.qv`'s `mk_mem_free` (inherits
  `libc::free`, `resource_released(ptr, nullable)`), and no other occurrence
  (P2); no loop, no `goto` (P3); no `R(p)`, so rule 38j keeps `p` and states
  `data` stale. The reader leaves `mk_ptr_t` opaque (the typedef defines the
  record), so the recogniser refuses the base; Clang reads `p->data` as the
  `LValueToRValue` value of the `char *` member `data` of the complete
  untagged struct, converted (`BitCast`) to `void *`. `p->len = 0` is a
  scalar store the parent run already lowers (the function's only leaf was
  the release). Fixture `the_corpus_bodies_with_a_typedef_spelled_base`.
- `rd_kafka_mock_sgrp_record_release(mshgrp, pmeta, state)`: `state` occurs as
  `state->G` only (P1); `(state, owner_member_id)` has one event,
  `rd_free(state->owner_member_id)` (inherits `libc::free`), and after it only
  the null store `state->owner_member_id = NULL;` (P2), in the same block with
  no `return` between, so the effect is `cleared`; no loop (P3). The record
  typedef defines the struct (reader: untyped); Clang reads a `char *`
  member's value of a complete struct. Its other statements (scalar reads,
  `pmeta->acquired_cnt--`, enum stores) carry no leaf on the base side. Same
  fixture, the body verbatim.
- `rd_kafka_Node_destroy(rd_kafka_Node_t *node)`: rule 5k: `node` occurs as
  `node->host`, `node->rack`, the null test, and `rd_free(node)` (P1); each
  place one release, before it only a null test (P2); no loop (P3); a
  top-level `rd_free(node);` follows with no `return` (P4). `rd_kafka_Node_t`
  is typedef'd forward in `rdkafka.h` and defining in `rdkafka_aux.h`, which
  the reader reads as disagreeing (untyped). Clang reads each actual as a
  `char *` member's value of the complete `struct rd_kafka_Node_s`. The base
  side's two `unresolved resource identity `node`` leaves come from the two
  refused member statements (the fixture shape loses them once both are
  admitted). Fixture `the_corpus_destructors_that_release_the_base_too`, both
  typedefs written as librdkafka does.
- `rd_kafka_Node_free(void *node)`: its one statement calls the destructor,
  now `Safe` with a published summary releasing its parameter, with a cast of
  the formal (its only leaf on the base side was `call to uncontracted
  function `rd_kafka_Node_destroy``). Same fixture, same unit.
- `rd_kafka_mock_error_stack_destroy(errstack)` and
  `rd_kafka_share_ack_batch_entry_destroy(entry)`: rule 5k as for
  `rd_kafka_Node_destroy` (the leading `if (!entry) return;` is a null test of
  the base before every release), one `rd_free` of a `*`-spelled member of a
  record whose typedef defines it; the other leaf on the base side is the
  base's unresolved identity, as above. Same fixture.
- `rd_kafka_share_ack_batch_entry_destroy_free(void *ptr)`: as
  `rd_kafka_Node_free`. Same fixture.

**Stays Unknown.** `rdkafka_offset.c::rd_kafka_offset_file_close` (named by
the parent as a base-arm body): its base resolves lexically and its release
is `fclose`, `nonnull`, which rule 38j never admits. The other functions with
an untyped-base leaf keep other leaves (census `other_blockers`): for example
`rd_kafka_mock_sgrp_fetch_session_destroy` (a nonnull
`rd_kafka_topic_partition_list_destroy` through `RD_IF_FREE`),
`rd_kafka_assignor_destroy` (`rd_kafkap_str_destroy`, no provider
contract), `metric_view_destroy` (members of an anonymous union), the mock
assignment setters (member stores). Callers of `mk_ptr_free` and
`rd_kafka_mock_sgrp_record_release` see their member-effect summary only
through rule 38j's direct-call route, and each keeps other leaves
(`cb_stream_page_finished`: the base's unresolved identity;
`rd_kafka_mock_sgrp_expire_locks`: loop and identity leaves): none moves.
`rd_kafka_TopicPartitionInfo_destroy` and
`rd_kafka_ConsumerGroupDescription_destroy` lose their uncontracted
`rd_kafka_Node_destroy` leaves but keep their own release leaves. The widened
implicit conversion applies to the parent's untyped-member arm too; the only
functions whose leaves that arm decides besides `cmt_opts_exit`
(`ctr_id_destroy`, `destroy_label`, `destroy_chunk_raw`) release through
`cfl_sds_destroy`/`flb_sds_destroy`, whose parameter is the member's own type,
so the conversion changes nothing there.

**System and slice.** No rule-38i release leaf has a typedef-spelled base
(census): no move.

No row may move to `Unsafe`; a move to `Safe` other than these is a finding to
explain before landing.

```measure-prediction
route corpus
move mk_core/mk_memory.c::mk_ptr_free Unknown -> Safe
move rdkafka_mock_sharegrp.c::rd_kafka_mock_sgrp_record_release Unknown -> Safe
move rdkafka_aux.c::rd_kafka_Node_destroy Unknown -> Safe
move rdkafka_aux.c::rd_kafka_Node_free Unknown -> Safe
move rdkafka_mock.c::rd_kafka_mock_error_stack_destroy Unknown -> Safe
move rdkafka_share_acknowledgement.c::rd_kafka_share_ack_batch_entry_destroy Unknown -> Safe
move rdkafka_share_acknowledgement.c::rd_kafka_share_ack_batch_entry_destroy_free Unknown -> Safe
delta Safe +7
route system
none
route slice
none
```

### The user's `current` run (Fluent Bit 046accf6)

The census estimate (report §1, with inherited wrapper contracts counted)
finds the same bodies in `lib/librdkafka-2.15.0/src/` and
`lib/monkey/mk_core/`: **`mk_ptr_free`, `rd_kafka_mock_sgrp_record_release`,
`rd_kafka_Node_destroy`, `rd_kafka_mock_error_stack_destroy`,
`rd_kafka_share_ack_batch_entry_destroy` Unknown -> Safe**, and with them the
two wrappers `rd_kafka_Node_free` and
`rd_kafka_share_ack_batch_entry_destroy_free` where the run analyses them.
`rd_kafka_Node_destroy` and `rd_kafka_share_ack_batch_entry_destroy` leave the
uncontracted-callee list (their W4 summaries are now published); no callee
whose summary is a member effect leaves it (`mk_ptr_free` and
`rd_kafka_mock_sgrp_record_release` stay withheld from the function table).
`rd_kafka_Node_free` and `rd_kafka_share_ack_batch_entry_destroy_free` leave
the "only uncontracted" set. No unit or library changes status (each unit
keeps other Unknown functions).
