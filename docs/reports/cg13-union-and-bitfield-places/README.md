# CG13-UNION-AND-BITFIELD-PLACES: union-member and bit-field place steps

Card CG13 of the [Clang gap card queue](../../plans/clang-gap-cards/README.md)
(wave 48, round 3; base `f19204fa` = local `main` = origin/main at dispatch;
main `cb20c28e`, carrying CG11-POINTER-VALUED-STORES, merged before finishing
and every measurement retaken on it, §3.0).
Gap entries M06 (unions) and M07 (bit-fields); evidence Q16, Q33 and example E
of the [analysis](../../reviews/clang-gap-2026-09-29/supplied/). Route **G**
(the custody gateway) and the **core** `disjoint_footprint` model; route T is
unchanged, since no typed-source consumer decides a union member or a
bit-field. Rule: [gateway 10w](../../agents/gateway.md#constraint-10w). The
prediction was frozen before any source change ([PREDICTION.md](PREDICTION.md),
commit `7b4d716b`).

## 1. What the card adds

`crates/qcore-c-gateway/src/union_bitfield.rs` holds the logic, with thin hooks
elsewhere.

**The step.** A member whose `FieldDecl` a complete `union` owns is a *union
step*. A `struct` bit-field whose width Clang gives is a *bit-field step*, in
the storage unit C11 makes it: a maximal run of adjacent non-zero-width
bit-fields in declaration order (C11 3.14). No layout is guessed. A union
bit-field, a bit-field without a width, an unread owner and a
`volatile`/`_Atomic` base are refused (`RecordFacts`, collected once per unit
beside the field facts).

**The decision** (`decide`, one row per step, `DECISION_VECTORS`). The access
through a step is the path's own access, or `pointer-read` when a pointer is
read from memory after the step.

| Storage | Access | Outcome |
|---|---|---|
| union | scalar read | neutral, whatever member is live |
| union | pointer read | target-free borrowed alias (P2); **refused** when punned |
| union | scalar store | refused-write frame; neutral when the union is custody-free |
| union | pointer store | **refused** |
| bit-field | scalar read / store | read / read-modify-write of its unit, neutral |
| bit-field | pointer access | refused (a bit-field is never a pointer) |
| union bit-field, unread | any | refused |

- **Punning** (`punned`). A pointer read is punned when the function stores
  through, increments or takes the address of another member of a union of
  the same record, or writes a whole object of the union's type, anywhere in
  the function. A callee's store is not visible; that is the stated premise
  `union-live-member-view`. A pointer so read is still only a target-free
  borrow, so no custody claim rests on it.
- **The frame.** A store through a union that can hold a pointer (a member
  not transitively a builtin scalar, an enumeration or an array of them)
  overwrites its pointer-valued siblings' representation. It carries W43's
  refused-write frame (`writes_through_pointer`), which poisons every binding
  except a spared local, so no sibling pointer member keeps a tracked
  identity. No route binds a pointer place or a field link through a union
  member at all (the pointer-store refusal here and CG11's), which is what
  makes that invalidation complete. A store through a custody-free union is
  frame-neutral.
- **Tracked roots.** A union step joins rule 10t's decision unchanged
  (`MemberStorage` on each `PlaceStep`), but no pointer loaded after a union
  step is a link of the chain model (`Refusal::UnionHop`).

**Bit-field units and the core model** (the R part: connected, not
re-invented). Two bit-fields of one unit are one memory location: disjoint
bit ranges never make their footprints disjoint. When the elaborator refuses
an unordered pair of two bit-field places of one object as not proved
disjoint, `shared_unit_refusal` builds a `LayoutContract` from Clang's own
`-fdump-record-layouts-complete` output for that record (`parse_layout`,
`layout_contract`), runs `check_disjoint_footprint_program`, and names the
refusal `SharedBitfieldUnit` (Q661) when the core refuses the pair as one
unit. The pair is refused either way. A pair the core proves disjoint (two
units) is **not** admitted here, since field disjointness is CG18's. The layout
is read only on that path (a second Clang run, parity text only), so it costs
nothing where no such pair exists.

**Composition with CG11** (rule 10v, merged from main `cb20c28e`). CG11 reads a
pointer-typed store's destination with `typed_place::read_place`, which now
reads a union step, and then asks `Reader::path` with `pointer_store_target`.
There the union step is decided as `Access::PointerStore` and refused whatever
value CG11 classified: a null, a string or a function pointer stored through a
union member stays refused, because no contract fixes the view of overlapping
storage. On the merged corpus this happens in 69 consultations. Before the
merge the same stores were refused by `read_place` itself, so neither card's
figures move from the composition.

**Shared items added** (additive only, for composing with CG03 and CG11):

- `typed_access_path.rs`: fields `StatementIndex::records`, `AstIndex::records`,
  `RetainedDeclarations::records` and `Reader::records`, and the fn
  `record_layout_dump()`. `Reader::path` gains the union/bit-field hook and
  `union_frame`, and admits a `union` base only for a union step.
- `typed_place.rs`: the field `PlaceStep::storage`, the variant
  `Refusal::UnionHop` and the parity key `storage`. `read_place` reads a union
  or bit-field step instead of refusing it (`Refusal::Field` stays for a union
  bit-field).
- `elaboration.rs`: the default method `Context::shared_bitfield_unit` and the
  variant `Refusal::SharedBitfieldUnit`. `elaborate` maps `NotDisjoint` through
  the method; no existing variant's meaning changes. `legacy_elaboration.rs`:
  `GContext` implements the method.
- `typed_c_ast.rs`: the fn `record_layout_dump`.
- `Cargo.toml`: `qcore-c-gateway` depends on `qcore-core` (L2 → L1, no
  provider).

## 2. Cited claims

| Claim | Reproduced? |
|---|---|
| M06 (Q16): "typed access rejects union owners" | **Partly.** `Reader::path` refused every union-owned field (Q16), but CG00's BEFORE control (`m06_…`) showed G reads a scalar union member through a never-bound root by an older route; reproduced at this base (BEFORE control passes before the flip). |
| M06 (Q33): the core marks `UnionMember` unsupported | **Yes** (`core_footprint_refuses_every_listed_construct`, unchanged). This card models union overlap in the gateway's decision and frame, not in the core. |
| M06: "some address-taking of union-typed members exists through another route" | Not re-checked by a control of this card and not relied on. |
| M07 (Q16): "G's typed access rejects bit-fields" | **Partly**, as CG00 recorded: stores refused, reads admitted by an older route (BEFORE control). |
| M07 / example E (Q33): the core represents bit-field storage-unit sharing | **Yes** (`m07_core_keeps_one_bitfield_unit_one_memory_location`; `the_core_model_keeps_one_unit_one_memory_location` over this card's contract). |
| CG00: 1,629 leaves on 467 Unknown functions; 55 alone (48 strict) | **Re-derived on this base: 1,629 / 467; 61 alone / 48 strict** (six functions freed of another blocker by CG10/CG02, none moved; [population-before.json](artifacts/population-before.json)). |

## 3. Prediction against outcome

### 3.0 On main `cb20c28e` (after CG11), the published figures

BEFORE was retaken on the merged base. The release binary was built at main
`cb20c28e` itself and run into `private/cg13/before-main` (exit 0). It
reproduces CG11's AFTER byte for byte: `resource-diagnostics.json`
`a6dc500b…`, 548 / 10 / 4,276. AFTER is this card's merged tip
(`private/cg13/after-main`, exit 0). See
[corpus-comparison-main.json](artifacts/corpus-comparison-main.json),
[identity.json](artifacts/identity.json) and
[decisions-main.json](artifacts/decisions-main.json).

| Route | BEFORE | AFTER | Moves |
|---|---|---|---|
| corpus (`66910c10`) | 548 / 10 / 4,276 | **554 / 10 / 4,270** (`97107d95…`) | the same **6** Unknown → Safe as at the card's base (§3.1), with identical leaf deltas; 200 Unknown functions change diagnostics; sidecars byte-identical |
| system (`046accf6`) | 33 / 2 / 199 | **33 / 2 / 199** | none; 1 function's diagnostics change |
| tail-pause slice (`90978ac6`) | 26 / 1 / 242 / 1 Pending | **26 / 1 / 242 / 1 Pending** | none; 3 functions' diagnostics change |

There is no correction, no Safe → non-Safe move, and no added leak or
violation on any route. Rule 10w's decisions on the merged corpus: 670
admitted and 173 refused, of which 104 are punned pointer reads and 69 are
pointer stores through a union member that reach it through CG11.

### 3.1 At the card's base `f19204fa`

Corpus arbiter (`--jobs 8`): BEFORE `private/cg13/before`, AFTER
`private/cg13/after2`, both exit 0 (after1, the same rule without the parity
rows, gave byte-identical diagnostics). Identity and moves:
[corpus-comparison.json](artifacts/corpus-comparison.json).

| Prediction | Outcome |
|---|---|
| BEFORE reproduces `963d7c9f…` | **Held.** `resource-diagnostics.json` `963d7c9f…` = CG02's archived AFTER; 543 / 10 / 4,281; sidecars `cbbcfd9c…`, `57acfc58…`. |
| Unknown → Safe 16 (band 6–40) | **6**, at the band's floor. |
| ≥ 60% of them in the re-derived list | **Held: 6 of 6.** |
| the named expected set | **Missed for 10 of 16.** See below. |
| Safe → non-Safe 0 | **Held.** No correction. |
| provisional Unsafe added 0 (0–2) | **Held.** Unsafe 10 → 10, leak leaves unchanged. |
| M06 leaves removed 400–900 | **Held: 428** (1,629 → 1,201; Unknown functions with one 467 → 405). |
| `unknown_alias` added 20–300 | **Missed: +374** (372 net) over 155 functions. |
| system 0–10 move, none Safe → non-Safe | **Held: 0 move** (32 / 2 / 200), 1 function's diagnostics change. |
| tail-pause slice 0–3 | **Held: 0 move** (26 / 1 / 242 / 1 Pending), 3 functions' diagnostics change. |

**Corpus: 543 / 10 / 4,281 → 549 / 10 / 4,275.** The six moves, each by
reading:

- `metric_data_point_count` (cmt_encode_opentelemetry_batch.c): the
  anonymous-union pointer members `metric->gauge` … `metric->summary` are
  compared with NULL and read through (`metric->gauge->n_data_points`) on a
  never-bound formal. The function stores through none of them, so no read is
  punned. 15 leaves.
- `LZ4_favorDecompressionSpeed`, `LZ4_setCompressionLevel` (lz4hc.c): a scalar
  store through the `LZ4_streamHC_t` union's `internal_donotuse` member. The
  union can hold a pointer, so the store carries the frame; the formal root is
  spared.
- `rd_kafka_event_opaque`, `rd_kafka_event_stats`,
  `rd_kafka_event_topic_partition_list` (rdkafka_event.c): a pointer return
  through `rkev->rko_u.…` on a never-bound formal (rule 10k's pointer-return
  premise), unpunned.

**The ten predicted moves that missed**, by reading. The union step is read
in each; a second cause, hidden by CG00's first-cause attribution, keeps the
function:

- `convert_*_value` ×5 and `convert_any_value`: the actual through the
  anonymous union is read. The leaf that stays is the call's: the same shape
  is Safe against a borrowed fixture contract (a route-G probe), so the
  callee's parameter position decides it.
- `pack_variant`: 5 of 7 leaves go; `pack_array` and `pack_kvlist` keep
  theirs, for the same reason.
- `LZ4_resetStream_fast`: `&(ctx->internal_donotuse)`, a parenthesised
  address (rule 10n).
- `rd_kafka_admin_timeout_remains(_us)` and `rd_kafka_message_latency`: a
  call in a `return` (rule 10k).
- The telemetry `calculate_*` functions: a call result stored into a member
  of a union local.

**Leaf deltas.** Unsupported: −310 IND, −104 branch condition, −7 return, −6
switch, −4 increment, −2 destination, −1 for-loop condition, and −7 callee
leaves (`LZ4_setCompressionLevel` ×6 and `metric_data_point_count`, now
summarised). One new `arguments do not match resource contract for
metric_data_point_count` leaf appears: a caller now meets the new summary,
whose argument does not match.

**The +374 `unknown_alias` leaves**, over 155 functions, classified by
reading the largest (`rd_kafka_mock_cluster_cmd` 23, `rd_kafka_admin_worker`
11, `rd_kafka_produceva` 11, the cmetrics `kvlist_fetch_*` and `exemplar`
functions). Each is an admitted union place whose base is not one live
identity: a target-free alias from an array element (`vu` in
`rd_kafka_produceva`), a call result (`value` in `kvlist_fetch_double`), or an
out-parameter written through (`*out`). These are rule 10a's reading, not
defects: an unresolved alias stays Unknown.

Remaining M06 leaves by class of their source line
([residuals-after.json](artifacts/residuals-after.json), a reading aid, not a
causal attribution):

| Class | Leaves |
|---|---:|
| another call on the line (CG03/CG05) | 343 |
| a pointer-valued store (CG11) | 229 |
| an address `&` deeper than one member (rule 10n) | 219 |
| a release of a pointer read through a union (stays refused) | 131 |
| an indirect call (CG22) | 35 |
| a cast (CG08) | 10 |
| other: punned reads, value `?:`, aggregate copies, pointer returns on bound roots | 234 |

Rule 10w's own decisions in the AFTER parity stream
([decisions-after.json](artifacts/decisions-after.json), `position:
union-bitfield`, deduplicated by function, node offset and outcome): **670
consultations admitted, 104 refused, every refusal a punned pointer read**
(17 function names, the OpenTelemetry tagged-union initialisers among them).
By step: scalar read neutral 248, framed store 184, borrow 173, punned 104,
custody-free store 71. No bit-field step is consulted anywhere in the corpus,
which agrees with M07's 0 leaves. The punned refusals are registered as
`CG13-UNION-PUNNING-FLOW-INSENSITIVE` (§6).

## 4. §8 axes and the flipped controls

**Flipped deliberately** (each test names the flip and its BEFORE):

- `clang_gap_phase2::m06_union_stores_are_refused_and_reads_are_neutral` →
  `m06_union_member_steps_are_read_by_rule_10w`: `u->i = 1` is Safe
  (BEFORE: refused, IND); `w->u.i = 1` is added; a pointer store through the
  union stays refused. The route-T refusal is unchanged.
- `clang_gap_phase2::m07_bitfield_stores_are_refused_and_reads_are_neutral` →
  `m07_bitfield_steps_are_unit_accesses`: `s->bf = 1` is Safe (BEFORE:
  refused, IND). Route T is unchanged.
- `typed_access_path` unit tests: `unsupported_pointer_store_union_cast_and_call_refuse`
  loses its union row (→ `unsupported_pointer_store_cast_and_call_refuse`),
  and `field_owner_refuses_typedef_union_but_accepts_typedef_struct` →
  `union_member_reads_are_decided_by_rule_10w`. Both the direct and the
  typedef-hidden union reads are admitted (BEFORE: refused), and the test
  adds the punned, framed, custody-free and bit-field rows.
- `typed_access_positions::union_bitfield_cast_and_call_operands_refuse` →
  `cast_and_call_operands_refuse_and_union_and_bitfield_operands_are_read`:
  the `p->u.a`, `p->bits` and typedef-hidden `u->x` condition operands are
  read (BEFORE: refused). The cast and call operands stay refused.
- `typed_place::casts_calls_unions_bitfields_and_volatile_bases_refuse`:
  `s->un.u` and `s->bf` are now root-only places (BEFORE: `Refusal::Field`).

The first gate run (exit 101, 13 failed of 6,640) found older cards' pins
whose only refusal was the union or bit-field step. Each is flipped by name,
with its BEFORE in a comment:

- `access_path_statements::pointer_store_and_union_still_refuse`: its three
  union rows now certify. The pointer-store and aggregate rows stay.
- `access_path_positions::a_condition_keeps_every_path_refusal`: the
  typedef-hidden union operand, bare and behind a scalar cast, is now Safe.
  `…an_enumeration_member_is_a_scalar_on_positive_evidence`: the union beside
  the enumeration is now Safe.
- `access_path_position_residuals::an_arrow_through_an_address_and_an_address_comparison_read`:
  `(&q->lock)->word` is now Safe.
- `chained_tail::every_non_conforming_tail_shape_keeps_its_refusal`:
  `uh->view->leaf` moves to its `TYPED` rows (an unpunned borrow in the
  declared tail).
- `o04_arms::every_untyped_link_keeps_its_unknown_and_its_diagnostic`, and
  `o04_constructor_caller::ambiguous_actual_shapes_keep_their_unknown` and
  `borrowed_field_actuals::every_other_refused_shape_keeps_its_unknown_and_its_diagnostic`
  (union link and union base rows): the actual through the union is read at
  the borrowed position. The gate refusal gives way to the fixture's own
  obligation (a nullable result used without a test), so these stay Unknown,
  never Safe. This is the W47 re-pin pattern these files already use.
- `indirect_chain_access::typed_extended_paths_preserve_unlisted_refusals`:
  `o->u->x = 1` through a union pointee is now Safe.
- `address_of_member::every_other_refused_member_shape_keeps_its_unknown_and_its_diagnostic`:
  `&hold->lock.tag` (a scalar member of the custody-free `a_lock_t` union)
  certifies at the borrowed position. Its only refusal was the union: rule
  10n's typed address fallback already hands a continued selection through a
  struct. A callee writing through the address reaches only the union's
  storage, and no union member is ever linked (O04-B09B10's premise). So
  this address carries no frame, and needs none.
- `typed_field_access`: the `union base` and `bitfield member` rows of
  `ambiguous_accesses_keep_their_existing_unknown_and_diagnostic` now certify.
  In `a_certificate_resting_on_the_classification_is_rederived_from_source`,
  the union substitution fails the program-summary gate instead of the
  containment gate, and the certificate still does not survive.
- `compiler/tests/workspace_layering.rs`: `qcore-c-gateway` may depend on
  `qcore-core`. This is a deliberate pin update for the
  `disjoint_footprint` connection. The edge points down (L2 → L1), and the
  gateway already reached the kernel through `qcore-contract`.
  `docs/development/architecture.md` says so.

**Refusal controls added** (`tests/union_bitfield_places.rs`, route G):

- concurrent (unsequenced) stores to two bit-fields of one unit, and of two
  units, are refused. The unit test
  `unsequenced_stores_to_one_unit_are_named_by_the_core_model` shows the
  core naming the first `SharedBitfieldUnit` over Clang's real layout, and
  leaving the second `NotDisjoint`;
- a punned union pointer read is refused, in a statement, an actual, and
  through a member chain;
- a custody pointer stored through a union member (`u->p = q_alloc()`,
  through a local, through `r->u.s.q`) is refused, and a release of a pointer
  read through a union is an unresolved alias, never the owner's release;
- a framed store keeps the tracked root, and a store after the release is not
  Safe.

**Axes.** Position: statement, compound assignment, increment, condition,
actual and return are covered. Effect: borrowed read, scalar store, framed
store and release (refused) are covered. Place: formal, tracked local,
embedded union, anonymous union, typedef-hidden union, union reached through
a member pointer, and a union local (record root, never-bound premise) are
covered. Type: scalar, pointer and bit-field are covered. Evaluation:
unsequenced pairs are covered. Refused: union bit-fields, `volatile`/`_Atomic`
bases, pointer stores, a hop after a union step on a tracked root, and a
union live member decided by a tag. Evaluation order: no order C does not
fix is introduced; an unsequenced bit-field pair stays refused.

## 5. Evidence

- **Rust.** Unit tests in `tests/unit/union_bitfield.rs`: the decision
  vectors, refusals independent of the flags, the Lean binding, the unit
  reading, custody-free unions, the punning check, the layout reader, the
  core model over this card's contract, and the elaborator naming over a real
  dump. Route-G controls are in `tests/union_bitfield_places.rs`; the
  flipped controls are in §4.
- **Lean** (`proof/QCore/UnionBitfieldPlace.lean`, audited by
  `proof/AuditUnionBitfieldPlace.lean`, no placeholders, axioms only
  `propext`):
  - `decision_vectors_hold`, bound to `DECISION_VECTORS` by
    `cg13_lean_vectors_match_the_rust_tables`;
  - `unlisted_forms_are_refused`, `a_bitfield_pointer_access_is_refused`,
    `an_admitted_step_is_listed`;
  - `a_union_pointer_store_is_refused`, `a_punned_pointer_read_is_refused`,
    `a_union_scalar_read_is_neutral`;
  - `bitfield_store_is_unit_rmw` (a read then a write of the unit, every bit
    outside the range kept, every bit in it set);
  - `a_bitfield_footprint_is_its_unit`;
  - `disjoint_bits_do_not_make_disjoint_footprints` and
    `one_unit_is_never_disjoint`, both reusing
    `QCore.DisjointFootprint.shared_bitfield_unit_is_never_disjoint`;
  - `a_union_store_invalidates_sibling_identities`,
    `a_custody_free_union_store_is_frame_neutral`,
    `a_spared_local_keeps_its_binding`.
- **Premises stated, not proved.** Faithful Clang extraction of field owners,
  widths and record layouts (trust boundary 28), and `union-live-member-view`.

## 6. Ledger (627 → 627 on main `cb20c28e`; 623 → 623 at the card's base)

- **Retired:**
  - `CG13-UNION-AND-BITFIELD-PLACES`. Every DISCHARGE clause holds: BEFORE
    reproduces the cited refusals with two partial corrections (§2), the
    prediction is frozen, AFTER is attributed, the absorbed follow-ups are
    closed or amended with re-measured sizes, and the Rust refusal controls
    and audited Lean are in place.
  - `W46-UNION-MEMBER-PATH`. It is superseded by a recorded decision, which
    is its discharge's second arm. A union path is read without a live-member
    reading, because only a pointer read depends on the live member: it is
    refused when punned and is otherwise a custody-free borrow. Its residuals
    are re-registered with sizes below.
- **Amended:**
  - `W44-CHAIN-INCOMPLETE-RECORD`: the union part is absorbed; the non-union
    part is not re-measured under its own census.
  - `W43-LIBC-GETPEERNAME-SOCKADDR-UNION`: the decision is reported and not
    taken. The transparent union is a declared-ABI reader class, not a place.
    No contract changes, so the provider gate is not due. It is re-measured
    at 1 leaf in `mk_socket_ip_str`, unchanged.
  - `CG11-POINTER-VALUED-STORES`: 229 pointer-store leaves through a union;
    rule 10w's pointer-store refusal.
  - `CG11-UNREAD-POINTER-PLACES`: its union part is now read and decided
    (refused by rule 10w, 69 consultations on the merged corpus). The cast,
    array, call and volatile parts stand.
  - `CG18-FOOTPRINT-CONNECTION`: the bit-field connection exists; admitting a
    disjoint pair is CG18's.
- **Registered:**
  - `CG13-UNION-PUNNING-FLOW-INSENSITIVE`, 104 consultations over 17
    function names ([decisions-main.json](artifacts/decisions-main.json));
  - `CG13-UNION-LEAVES-OTHER-CAUSES`: 1,201 M06 leaves (891 union-only) over
    405 Unknown functions, by class.

[scripts/update_ledger.py](scripts/update_ledger.py) applies these edits.

## 7. Gate

The gate ran at tip `8252e21c` with [scripts/gate.sh](scripts/gate.sh), logs in
`private/cg13/gate2`, exits in `gate/`, record in [gate.json](gate.json). Every
step exited 0:

- submodule step;
- `run_test_binaries.py`: 372 units, 385 summaries, **6,640 passed, 0
  failed**, 11 ignored
  (`private/test-binaries/20260929T162051Z-4130235/summary.json`);
- fmt and clippy;
- both reevaluation probes;
- `check_lean_audits.py`: **232** audits (231 + `AuditUnionBitfieldPlace.lean`);
- Python tests: 365 OK, rerun after the matrix validation edit;
- docs index.

The first run (`private/cg13/gate`, tip `9cf7752e`) exited 101 on 13
failures, each resolved in §4. `providers/` is unchanged, so the provider
gate is not due. Clang 21.1.8 is recorded in `gate/clang-version.txt`.

## 8. Scripts

- [measure.sh](scripts/measure.sh): the corpus, system and slice runs.
- [population.py](scripts/population.py): CG00's population re-derived.
- [leaves_of.py](scripts/leaves_of.py): a candidate function's leaves with
  their source lines.
- [residuals.py](scripts/residuals.py): the remaining M06 leaves by class.
- [punned.py](scripts/punned.py): rule 10w's decisions in a parity stream.
- [update_ledger.py](scripts/update_ledger.py): the ledger edits, applied to
  main's matrix after the merge.
- [identity.py](scripts/identity.py): the arbiter runs' digests.
- [gate.sh](scripts/gate.sh): the standing gate.
- The comparison is CG02's
  [`compare.py`](../cg02-elaboration-core/scripts/compare.py), unchanged.
