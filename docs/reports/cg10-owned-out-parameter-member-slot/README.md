# CG10-OWNED-OUT-PARAMETER-MEMBER-SLOT: an owned value delivered into a member slot of a caller-supplied object

Card CG10-OWNED-OUT-PARAMETER-MEMBER-SLOT. USER DECISION 2026-11-21 (through
the integrator's brief): launch the card. Base `8e7e8d509` (integrate/r48).
[PREDICTION.md](PREDICTION.md) was committed (`7de74455f`) before the first
implementation commit (`bedc1a5c1`). Gateway [rule 72](../../agents/gateway.md#constraint-72),
contract specification §C04.13.1, five pinned declarations, Lean
`QCore.OwnedOutMemberSlot` (13 audited theorems).

## 1. The 13 rows and their callees (step a)

Every caller and callee body was read at Fluent Bit `046accf6`
([PREDICTION.md](PREDICTION.md) section 1 has the table). The rows are round
4's (`../cg10-indirect-access-sole-residuals-r4/census/rank-tip.json`); on
this card's base run each is `Unknown` with the access refusal as its only
leaf (section 4). One shape: a callee stores a fresh allocation through an out
slot (`*output_buffer = cfl_sds_create_size(len)`, `*outbuf = rd_malloc(…)`)
whose actual is the address of a member of an object the caller reaches from
one of its own formals:

* cmetrics `unpack_opts_{ns,ss,name,desc}` (`&opts->F`, `opts = map->opts`,
  `map = (struct cmt_map *) context`) and `unpack_static_label_component`
  (`&label->key`, `&label->val`, `label = (struct cmt_label *) context`), all
  through `cmt_mpack_consume_string_tag`;
* ctraces `unpack_instrumentation_scope_{name,version}`,
  `unpack_link_trace_state`, `unpack_span_schema_url`,
  `unpack_span_status_message` (`&context->scope_span->instrumentation_scope->name`,
  `&context->span->status.message`; `struct … *context = ctx`), through
  `ctr_mpack_consume_string_or_nil_tag`;
* librdkafka `rd_kafka_msgset_writer_compress_{gzip,lz4,snappy}`
  (`&ciov->iov_base`, `ciov` a formal), through `rd_kafka_gzip_compress`,
  `rd_kafka_lz4_compress`, `rd_kafka_snappy_compress_slice`.

Each callee returns `0` with a fresh, owned value in the slot (ctraces: or
NULL for a nil tag) and nonzero with the slot unwritten or written NULL. None
of the 13 holds a frame-owned record: every slot is a member of the caller's
caller's object.

## 2. The contract form (step b)

**No new grammar.** The callees are C04.13 return-code dispositions with an
out slot: `resource_on_return(eq, 0, resource_out(slot, nonnull|nullable))`,
`resource_on_return(ne, 0, resource_borrowed(slot))` (the class leaves the
slot unwritten), every other formal borrowed, `resource_return_none()`. What
was missing was the consumer's reading of a member-path actual, now §C04.13.1:
where the slot's object is the caller's, every class delivers fresh custody or
nothing through it, so the class needs no decision; the delivered custody is
that object's.

**On the brief's step (c) wording.** The brief asks that "after the call the
member holds an owned value tracked like the field-link model's assignment
case: its later release ends the record, and an unreleased one is a leak
obligation". That is the reading for a member of a record this frame owns (a
tracked owner). None of the 13 has one: in each the member lies in an object
the caller supplied, where the field-link model's own semantics is the
owner-escape case (`cascade_field_links_on_escape`: a linked child escapes
with its escaping owner). Rule 72 implements that case: the delivered custody
is the caller's object's, owed by no exit of this frame, and nothing the frame
owed is dropped (`QCore.OwnedOutMemberSlot.an_admitted_delivery_keeps_the_frames_obligations`,
`the_delivered_identity_is_held`). The tracked-owner case keeps the gate's
refusal: all five declared callees are dispositions, whose unwritten class
cannot be read as a field store without a decided class, and no plain-`Out`
heap declaration exists in production, so implementing it would have no
consumer and no measurement. It is registered
(CG10-OWNED-OUT-MEMBER-TRACKED-OWNER).

## 3. The reading (rule 72, step c)

`owned_out_member.rs` (pure: the minted contract, the slot reader, the
caller-supplied roots, the decision), `legacy_owned_out_member.rs` (the
parser's lowering), and the flow in rule 10v's `legacy_pointer_store.rs`
(value class `owned-out`). In order:

1. **The minted contract.** A callee with exactly one out slot (plain `Out`,
   or a disposition `Out`/unwritten there and equal elsewhere) whose contract
   with that slot `Borrowed` is the neutral element is installed under
   `qcore_owned_out_member__F` (a reserved prefix).
2. **The statement.** Only for exactly the gate's refusal of `CALL`,
   `NAME = CALL` or `return CALL`, asked after every other route and before
   rule 12k: the slot actual is `& R -> F (-> G)* (. H)*`, `R` a
   caller-supplied root (an unrebound formal, or a name every write of which
   is a root, an object-pointer cast of one or a member read through one:
   premise P-72-CALLER-OBJECT). The statement is classified under the minted
   name with the slot replaced by the null constant; the other actuals are
   read by rule W44 (the lz4 caller's `?:` included).
3. **The lowering.** The slot's chain read, the call's block, and a rule-10v
   `PointerStore` of class `owned-out` (access: the call; fallback: the gate's
   refusal).
4. **The flow.** The owner is rule 10v's, read under rule 14n's named premise
   where the chain read is (`opts = map->opts` binds `opts` target-free and
   possibly NULL): untracked admits; tracked or unresolved refuses; then 10v's
   old-value premise (P-CG11-CALLER-SLOT).
5. **The summary.** A body with such a store publishes no summary (10v's
   `has_pointer_store`; rule 70's retention summary refuses it), so no caller
   reads `Borrowed` for a formal whose member now owns custody.

**Fixtures** (`crates/qcore-c-gateway/tests/owned_out_member_slot.rs`, a new
target, recorded in the gate read census): the eight census forms are `Safe`
with the production modules and `Unknown` without them; refused (never
`Safe`): a record this body allocated, a pointer to this frame's own object, a
rebound formal, an indexed slot, a call inside a larger expression, `&v.key`
of a local record; a buffer dropped beside the delivery is `Unsafe`; a caller
of a body with a delivery sees `call to uncontracted function`; a local `&s`
slot keeps C04.13's reading (`Safe` when released on the `0` edge, `Unsafe`
when not, refused untested); the refusal text is the gate's. Unit tests in
`tests/unit/owned_out_member.rs` (5: the decision vectors, the slot vectors,
the roots, their refusals, the minted contract).

## 4. Measurement

See section 4 below once filled; route and inputs in
[evidence/runs.md](evidence/runs.md).
