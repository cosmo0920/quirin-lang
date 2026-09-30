# CG12-VOID-VIEW-READBACK: `void *` views and readbacks

Wave 48, round 7. Gap entry M05 (evidence Q18), route G. Gateway rule
[11j](../../agents/gateway.md#constraint-11j). Absorbs
`W47-FIELDS-BY-MEMBER-VOID-ESCAPES`.

## 1. The input, reproduced and corrected

The gap analysis (static, no Rust run) says: storage into a `void *` member is
supported, and a read back from a tracked `void *` member is refused because
the pointee type is not known. CG00 turned that into a BEFORE control
(`m05_readback_from_a_void_field_is_refused`) and attributed 76 corpus leaves
on 33 Unknown functions to M05.

| Claim | Status |
|---|---|
| a tracked pointer stored into a `void *` member of an admitted store base is admitted (ALIAS-AGGREGATE) | **reproduced** (a record the reader reads whole) |
| its readback is refused | **reproduced**, as P2's target-free borrowed alias: `char *c = s->v;` is admitted as a read with no identity, and `q_free(c)` is `UnknownAlias { variable: "c" }` |
| CG00's control refuses at the readback | **correction**: over the control's `struct S` (a bit-field, arrays, a function pointer) the `indirect memory access` refusal is the **store** `s->v = q_alloc()`, which ALIAS-AGGREGATE's reader does not admit for a record it did not read whole; the readback is never reached |
| the 76 corpus leaves are readbacks of tracked members | **correction**: CG00's attribution (`attribute.py`, cause `cast-from-void`) takes any cast whose operand is `void *` on a leaf's path. Read line by line, every one of the 76 is a **view** of an untracked `void *` (a comparator's `const void *` formal, nanopb's `field->pData`, cmetrics' `void *instance`) or a view over a call result or arithmetic; none is a readback of a tracked member |
| `W47-FIELDS-BY-MEMBER-VOID-ESCAPES`: 80 escapes | **correction** (§6): 62 of the 80 are `(void) x;` discards (Clang `ToVoid`), which leave no pointer value; 18 carry one |

So the card is two rules, both in `void_view.rs`: the **view** (what the
corpus needs) and the **readback** (what the entry names).

## 2. What landed (gateway rule 11j)

**The view** (`void_view.rs` `view_of`, `view_position`; the step in
`typed_access_path.rs` `Reader::path`). An explicit cast whose Clang kind is
`BitCast`, from `void *` (`const`/`restrict` allowed) to an object pointer,
is the same pointer with the target's pointee type (C11 6.3.2.3p1/p7; CG08's
`CAST_TABLE` row, no custody effect). It stands only where `*`, `->` or `[]`
reads a pointer through, and only over an untracked object:

- over the root: the chain is marked `root_untracked_only`, so the flow reads
  it as never bound or as rule 11d's bound-untracked root and refuses a
  tracked identity (whose object has the type its contract gives it);
- over a pointer loaded from a place of a root the closure proves never bound
  (rule 10k's premise: every such load is outside the tracked graph).

A view over a load from any other root, a view of a root as `struct TAG *`
where the body declares another name of that C02-P-ALIAS class, and every
other cast (from another object pointer, a pun; to a function or `void`
pointer; `NoOp`; any other kind) keep their refusal. At a pointer
destination, the explicit view to the destination's own pointee type is
read as rule 10m's implicit one.

**The readback** (`decide_readback`, `body_facts`, the adapter
`legacy_void_view.rs`, the `readback` field of `IndirectScalarAccess`). A
pointer destination (an automatic local) reading `R->F`, `F` a `void *`
member, takes the member's identity instead of P2's target-free borrowed
alias only when: `R` is an admitted store base and `F` its `void *` member;
every known stored type and every view of `R->F` in the body agree (else the
statement is refused as a pun, M22); every reference to `R` is a member
access, its own assignment, a null test or a whole `Released`/`Consumed`
actual of a contracted call; `R->F` is only stored (`=`) and read; and
`bound_read_resolves` holds at the node (O04-READ-BOUND: the owner one live
identity this body acquired, the binding exactly the field link, on every
path per rule 11c, the child live with no unresolved effect). A refused
statement or uncontracted call poisons the binding; a later admitted store
replaces the link. `bound_read_actuals` still refuses the member at every
call actual (ALIAS-AGGREGATE's Lean theorems stand unchanged).

**Shared foundations, additive only.**

- `typed_access_path.rs`: `StatementPlan::readback`; in `Reader::path`, the
  view step (locals `pointer_operand`, `after_view`, `load_view`,
  `saw_view`, `pending_key`, `root_view_key`), the position and alias-peer
  checks, and `root_untracked_only: saw_view` on the chain it builds; in
  `Reader::statement`, `explicit_source` asked before `pointer_source` at
  both pointer destinations and the readback candidate.
- `legacy_c.rs`: `LegacyInstruction::IndirectScalarAccess::readback`
  (`None` at every older construction site), the hook in
  `typed_path_statement`, the binding in the access's transfer, and
  `mod void_view_route`.
- No change to `elaboration.rs`, `typed_place.rs` or `call_instantiation.rs`.

## 3. Lean

`QCore.VoidViewReadback` (`proof/QCore/VoidViewReadback.lean`), audited by
`AuditVoidViewReadback.lean` (29 theorems, no placeholder, no project
axiom):

- the view: `view_vectors_hold`,
  `a_view_is_exactly_a_bit_cast_from_void_to_an_object_pointer`,
  `a_cast_between_object_pointers_is_never_a_view`,
  `a_non_object_target_is_never_a_view` (unlisted forms refused),
  `position_vectors_hold`, `a_view_over_a_tracked_identity_is_refused`,
  `a_view_over_a_link_is_refused`,
  `a_view_over_an_untracked_root_adds_no_premise`;
- the readback's static decision: `decision_vectors_hold`,
  `identity_exactly_when`, `a_type_mismatched_view_is_refused`,
  `an_escaping_owner_gives_no_identity`,
  `an_escaping_member_gives_no_identity`,
  `an_unlisted_owner_keeps_the_old_reading`,
  `the_flow_binds_only_an_admitted_resolved_readback`;
- the flow's event model: `event_vectors_hold`,
  `a_readback_returns_the_stored_identity` (no intervening store),
  `an_intervening_unknown_store_yields_no_identity`,
  `a_partial_link_yields_no_identity`,
  `a_released_child_is_never_read_back`,
  `the_readback_is_the_last_store`,
  `a_readback_returns_only_a_stored_identity`, and the non-vacuity pair
  `the_flagship_readback_yields_its_identity` /
  `one_poison_flips_the_flagship`.

Bound to Rust by shared vectors: `VIEW_VECTORS` (each row compiled by Clang
and classified, `tests/unit/void_view.rs`), `POSITION_VECTORS`,
`DECISION_VECTORS` and `EVENT_VECTORS` (each body run through the gateway,
`tests/cg12_void_view_readback.rs`).

## 4. Tests

- `tests/cg12_void_view_readback.rs` (8): the vectors, the corpus view shapes
  in every position, unlisted views refused, and the card's two refusal
  controls: a store, then an uncontracted call taking the owner, then a
  readback (the readback is `UnknownAlias`, never the identity); reading
  back as another struct type (three spellings, refused as a pun).
- `tests/unit/void_view.rs` (8): the decision tables and Clang's reading of
  every view vector.
- **Flipped pins, each deliberately and named in its test** (BEFORE kept in
  the comment):
  - `clang_gap_phase2::m05_readback_from_a_void_field_is_refused`: CG00's
    body still refuses (its refusal is the store, §1); over a record the
    reader reads, the same body is `Safe`;
  - `indirect_chain_access::typed_extended_paths_preserve_unlisted_refusals`:
    `((struct c_leaf *)b->item->leaf->any)->weight = 1` under a never-bound
    root, Unknown → Safe;
  - `sub_object_stores::a_cast_base_keeps_its_refusal`:
    `((struct x_rec *) p)->count = 1` on a `void *` formal, Unknown → Safe,
    with a new row: the same view of a tracked `void *` stays refused;
  - `typed_const_pointer_value::every_other_cast_keeps_the_refusal`: three
    rows (an explicit view to the destination's type, a view in a path over
    a never-bound load, in a statement and a condition), Unknown → Safe,
    with a new row: a double cast through another object pointer stays
    refused.

## 5. Prediction against outcome

[`PREDICTION.md`](PREDICTION.md) §1 was frozen before the BEFORE run
finished and §2 from it, both before any AFTER run.

| Measure | Predicted | Measured (base `c374de96`) |
|---|---|---|
| corpus Unknown → Safe | +1 to +3 (`rd_strcmp3` 85%, the two `XXH_*` 55%) | **+2**: `LZ4_write16`, `LZ4_write32` (not in CG00's list: `*(U16 *)memPtr = value`, a scalar store through a view of a `void *` formal); none of the three predicted |
| Safe → non-Safe, Unsafe added | 0 | **0** |
| CG00's 76 M05 origins lifted | at most 53 | **18** (58 remain) |
| system | +0 to +1 | **+2** (`sb_get_context`, `sb_get_releasable_output_queue_space`) |
| tail-pause slice | +0 | **+0** |
| census escapes followed | 0 | **0** |

Misses, each read: `rd_strcmp3` stays Unknown because the `return f(*p, …)`
position refuses a pointer-valued typed actual with no cast at all
(`return rd_strcmp(*a, *b)` on `const char **` formals is refused too); the
view itself is read. The two `XXH_*` keep the `return ?:` refusal of the
arm holding a call (`XXH_swap32(…)`); without the call the same view is
`Safe`. `sb_get_context` returns
`((struct flb_input_instance *) config->storage_input_plugin)->context`, a
view over a load from a never-bound formal, and its caller is freed by the
summary.

## 6. Measurements

`scripts/measure.sh NAME` runs the brief's corpus arbiter (`--jobs 4`), the
system route and the tail-pause slice; `scripts/identity.py` compares
two runs; `scripts/leaf_moves.py` lists every added and removed leaf.

| Route | BEFORE (base `c374de96`) | AFTER (`7a057d80`) |
|---|---|---|
| corpus | 608 / 10 / 4,216, `3a25c4e9…` (= baseline) | **610 / 10 / 4,214**, `a3564bcc…` |
| system | 41 / 2 / 191, `5b922bb8…` | **43 / 2 / 189**, `10602caa…` |
| slice | 28 / 1 / 240 / 1 Pending, `f0930cab…` | 28 / 1 / 240 / 1 Pending, `dd8e6d4d…` |

Attribution ([`artifacts/runs/`](artifacts/runs/)):

- **corpus** 65 functions change: 2 Unknown → Safe (above), 63 stay Unknown.
  Leaves: indirect-access refusals −81, uncontracted `LZ4_write16/32` −5
  (the new summaries), unrecognized destination −2, branch and return −1
  each; **unresolved identity +90 on 40 functions** and one indirect-access
  refusal moved to a later line. The +90 are views now read whose root does
  not resolve untracked at the flow (a bound, tracked or refused-path root,
  e.g. cio's `cf = (struct cio_file *) ch->backend` then `cf->…`), each where
  an indirect-access refusal stood on the same or an earlier line: a
  refusal replaced by the typed access's own obligation, not a new claim.
  No leak or violation was added; no Unsafe changed.
- **system** 8 functions: 2 Unknown → Safe; 6 stay Unknown (indirect −7,
  return −1, uncontracted −1; unresolved identity +4 in `handle_dlq_if_available`,
  `flb_input_chunk_get_projected_write_size`, `input_chunk_append_raw`).
- **slice** 1 function changes leaves (one indirect-access refusal becomes
  two unresolved identities).

**Readbacks on the corpus** (parity `position: "void-view-readback"`):
116 distinct pointer destinations read a `void *` member back; 115 have no
admitted store base, 1 an owner that escapes (`mk_server_loop_balancer`,
`evl->data`); **0** are bound to an identity. The readback is exercised by
the fixtures only (`CG12-READBACK-NO-STORE-BASE`).

**CG00's list re-derived** (`scripts/rederive.py`, artifacts
`rederive-{before,after}.json`): all 33 functions stay Unknown; 18 of the 76
origins lift (`pb_enc_varint` 8, `pb_field_set_to_default` 3,
`rd_kafka_assignor_topic_cmp` 2, and one each in `encode_extension_field`,
`pb_check_proto3_default_value`, `rd_kafka_cgrp_get_unsubscribing_topics`,
`rd_kafka_confval_set_type`, `rd_list_get_int32`); the 58 left are
classified in `CG12-VIEW-RESIDUAL-FORMS` (`scripts/residual_origins.py`).

## 7. The loop-owner census's `void *` escapes

`W47-FIELDS-BY-MEMBER-VOID-ESCAPES` registered 80 conversions of an input
instance pointer to `void *` or an integer. `thread_entry_census` /
`loop_owner_instance` are unchanged (no reader of theirs is shared with the
gateway rule). `scripts/census_escapes.py` re-reads the archived premised
census (`w47-premises-instance/artifacts/census/fixed-premised.json.gz`) and
each site's line in its export
([`artifacts/census-escapes.json`](artifacts/census-escapes.json)):

| Class | Count | Followed by rule 11j |
|---|---:|---:|
| `(void) ins;` discard (Clang `ToVoid`, no value survives) | 62 | – (nothing to follow) |
| `void *` actual of a call | 9 | 0 |
| local conversion | 2 | 0 |
| store into a `void *` member | 7 | 0 |

Rule 11j is intra-procedural and follows **0** of them: none of the 7
member stores is into an admitted store base that the storing function
reads back (the one local owner, `flb_sp_stream_create`'s `stream`, never
reads `stream->in`). The conversions back happen in other functions, e.g.
`config->storage_input_plugin` (stored at `flb_storage.c:751`) is read back
as its own record type `struct flb_input_instance *` at
`flb_storage.c:503` and in `sb.c:111`; the latter is now read by the view
(over a never-bound formal, no identity) and `sb_get_context` is Safe on the
system route, but that reads no instance identity. The 18 non-discard
escapes and the other-type view (`in_blob.c:761`) stay registered, amended.

## 8. Ledger

Applied by [`scripts/update_ledger.py`](scripts/update_ledger.py) (text in
`scripts/ledger_text.py`, idempotent): 661 → 663.

- **`CG12-VOID-VIEW-READBACK` amended, not retired.** Every other DISCHARGE
  clause holds (BEFORE reproduced with its corrections, PREDICTION.md frozen,
  AFTER attributed, the absorbed entry re-measured, Rust refusal controls,
  audited Lean). The entry's scope, a stored identity connected to its
  post-cast view on the corpus, is met by the rule but by **no** corpus site.
- **`W47-FIELDS-BY-MEMBER-VOID-ESCAPES` amended** with the re-measured
  classes (§7): 0 followed, 62 discards, 18 carrying a value.
- **Registered:** `CG12-READBACK-NO-STORE-BASE` (115 sites) and
  `CG12-VIEW-RESIDUAL-FORMS` (58 of CG00's 76 M05 leaves on 26 functions,
  and the 90 unresolved-identity leaves on 40 functions that replace
  indirect-access refusals).

## 9. On the final main (`e93dbe4e`)

Main moved while the card ran; the card merged `b59aaae2` (CG07), then
`fe9c5b48` (W48-CALLEE-CLOSURE-R3, CG14) and last `e93dbe4e` (CG15), and
re-took BEFORE on each main tip before its AFTER (the `fe9c5b48` AFTER was
stopped when `e93dbe4e` was announced). Binaries:
[`artifacts/runs/binaries.sha256`](artifacts/runs/binaries.sha256).

| Route | BEFORE, main `b59aaae2` | AFTER, merge `2ed80c5d` | BEFORE, main `e93dbe4e` | AFTER, merge `dc6dbea4` |
|---|---|---|---|---|
| corpus | 608 / 10 / 4,216, `8b9e4047…` | 610 / 10 / 4,214, `6b65aba4…` | 623 / 10 / 4,201, `d42a99e5…` | **626 / 10 / 4,198**, `08b9a260…` |
| system | 41 / 2 / 191, `0f6d2051…` | 43 / 2 / 189, `f6e36791…` | 42 / 2 / 190, `8572e708…` | **44 / 2 / 188**, `5e4143cf…` |
| slice | 28 / 1 / 240 / 1 P, `b82c9954…` | 28 / 1 / 240 / 1 P, `ce39fea6…` | 29 / 1 / 239 / 1 P, `aebf9346…` | **29 / 1 / 239 / 1 P**, `7bd764fb…` |

The `b59aaae2` merge moves exactly what the base did (the same 65 corpus and 8
system functions, the same leaf totals). On `e93dbe4e` the corpus gains
**+3**: `LZ4_write16` and `LZ4_write32` as on the base, and
**`LZ4_writeLE16`**, a joint move with CG14 (its `LZ4_isLittleEndian` call
became Safe on main; its `LZ4_write16` call is now Safe through CG12's
summary; its `BYTE *p = (BYTE *) memPtr;` is the explicit view at a pointer
destination). One more function changes leaves only:
`LZ4_compress_generic_validated`, whose `LZ4_write*` call leaves go (the new
summaries) and whose three `op++`-style refusals are reported by the formal
mutation gate instead of the increment refusal, on the same lines. Totals
there: indirect-access −81, uncontracted −7, increment −3, unrecognized
destination −2, branch and return −1 each, unresolved identity +90 −4, formal
mutation +3, indirect-access +1 (moved). System +2 (the same two functions),
slice unchanged in status (one function's leaves). No Safe → non-Safe, no
leak or violation added, no Unsafe changed. On the final merge the 116
readback sites and the 58 residual M05 leaves are unchanged.

## 10. Gate

`scripts/gate.sh` runs the standing gate; exits under [`gate/`](gate/).

**The full gate was stopped at the shared-host disk threshold.** On the
final merge the test build of `scripts/run_test_binaries.py` took this
worktree's `target/` to 25 GB while free disk fell from 48 GB to 23 GB (the
brief's floor is 40 GB). The run was stopped (its captured task only),
`cargo clean --profile dev` freed the build (48 GB free again), and no build
was restarted. What ran:

| Step | Exit | Figures |
|---|---:|---|
| submodule update | 0 | |
| `python3 scripts/run_test_binaries.py` | **not completed** (stopped mid-build) | |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 on `cbfa044e` (base plus `b59aaae2`); not re-run after the later merges | |
| reevaluation probe / follow-up | not run | |
| `python3 scripts/check_lean_audits.py` | 0 | 259 audits (258 on main + `AuditVoidViewReadback.lean`) |
| `build_matrix.py .` (gate figures carried, labelled as such) | 0 | derived Lean audits 258 → 259 |
| `python3 -m unittest discover -s scripts/tests` | 1, then **0** after the regeneration | 365 |

Targeted Rust evidence: `qcore-c-gateway`'s whole suite on the base (2,164
passed, 4 failed: the four pins §4 flips, since updated), then every touched
and adjacent target on the `fe9c5b48` merge (`cg12_void_view_readback` 8,
`clang_gap_phase2` 14, `indirect_chain_access` 15, `sub_object_stores` 22,
`typed_const_pointer_value` 7, `cg14_aggregate_values` 9,
`w48_callee_closure_r3` 4, `access_path_statements` 7, all passed). The
workspace suite, clippy on the final tree and the two probes are owed to the
integrator's check. `providers/` is untouched, so the provider gate is not
due.
