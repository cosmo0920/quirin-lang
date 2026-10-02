# CG24-ALIAS-EXTENT-FRAME-VOCABULARY

Wave 48, round 11. Card CG24 of the Clang gap queue (entries K04, K06),
High; gateway rule [11e](../../agents/gateway.md#constraint-11e); contract
specification [C09.1](../../QCORE_CONTRACT_SPEC_r0_3.md#c091-reviewed-index-and-extent-contracts-planned)
(a CG24 status paragraph); grammar
[G05b](../../QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05b-bounds-contract-v1). Lean:
`QCore.IndexExtentFrames` ([`proof/QCore/IndexExtentFrames.lean`](../../../proof/QCore/IndexExtentFrames.lean),
audited in `proof/AuditIndexExtentFrames.lean`), a sibling extending
`QCore.IndexExtent`. Base: local main `6cab1ad9` (CG16 landed). Absorbs
`W46-SDS-CAT-SAFE-IN-PLACE-REALLOC` and `W45-FB-BACKLOG-REFERENCE-VOCABULARY`.

**Published property and scope.** CG16's `c.bounds.index-extent.v1`, with two
more kinds of declared fact a person can state about an external callee and
the consumer checks: a **frame** (what a call does, through each pointer
argument, to the caller's bounds facts) and a **derived relation** (a result
that is `base + OFF` with `LEN` units, on one base or several). The card
extends CG16's vocabulary (same selector, loader, consumer, verdict and review
lifecycle); it defines no parallel one and never lets a declaration enlarge a
source extent. Custody is a different property and is unchanged: a frame is
not a custody mode, and resource v1 rejects both clause families.

## 1. What changed

| Layer | Item | Owns |
|---|---|---|
| core | [`qcore_core::index_extent_frames`](../../../crates/qcore-core/src/index_extent_frames.rs) (new) | `FrameMode` (reads, writes, moves), `CallFrame` (unstated, CG16's preserves, closed), `Facts::call_frame` (an `impl` on CG16's `Facts`), `derived_within`, `multi_base_view`, `BodyEffect`/`refutes`, `frame_at_call` |
| contract | [`qcore_contract::bounds_contract`](../../../crates/qcore-contract/src/bounds_contract.rs) | `BoundsClause::{Frame, ReturnDerived}`, `ParameterKind::Slot` (`ptr<mut, ptr<mut, u8>>`), `BoundsFunctionContract::{frame, derived_bases}`, the static conditions (`frame_conditions`), four reserved spellings |
| gateway | [`qcore_c_gateway::index_extent`](../../../crates/qcore-c-gateway/src/index_extent.rs) (CG16's consumer, hooks) | each call's frame (`frame_of`, `apply_calls`, replacing CG16's all-or-nothing `calls_preserve`), frame writes in every written set (`written`), frame-tracked locals (`frame_tracked`), derived results (`return_derived`, sharing `preconditions_established` with `return_extent`), refutation (`check_own_frame`, `check_return_derived`), phase one and the kept bodies for every checked relation |
| Lean | `QCore.IndexExtentFrames` (new) | the model of the frame, containment and refutation decisions; 29 audited theorems; three shared-vector tables |

No custody code changed (no hook in `legacy_c.rs`, `typed_source.rs` or any
custody route; the custody loader and `qcore_schemas::contract` are
untouched). Shared items added: `qcore_core::index_extent_frames` (new
module), `BoundsClause::Frame`, `BoundsClause::ReturnDerived`,
`ParameterKind::Slot` and `ParameterKind::is_pointer`,
`BoundsFunctionContract::{frame, derived_bases}`. Shared foundations
(`elaboration.rs`, `instantiate`, summaries, the typed place) are not edited.
`address_taken` in the consumer was split into a per-node `address_root`
with the same reading.

### Semantics

* **Closed frames.** `ensures normal bounds_frame_reads(P)`,
  `bounds_frame_writes(P)` or `bounds_frame_moves(P)`, one for every pointer
  and slot parameter or none. A read position keeps every fact; a write
  position withdraws the facts of the local whose address `&x` it receives
  (its value facts) and keeps extent facts on objects it only writes; a move
  position also withdraws every fact on an object the source did not
  establish (the base may have been reallocated or released). Without a
  frame, a call is CG16's: every such fact is withdrawn unless the callee
  states `bounds_preserves_extents()`. Facts a held frame keeps rest on its
  holds (Pending at most). The callee touching nothing else and retaining
  nothing derived from a parameter is the declaration's `Declared` premise.
* **Frame-tracked locals.** CG16 tracks no local whose address is taken. A
  local whose every address is a direct actual of a resolved closed frame
  position (no derived base) is tracked: its frame writes join the written
  set of the full expression, loop, switch and counted `for`.
* **Derived relations.** `ensures normal bounds_return_derived(P, OFF, LEN,
  UNIT)`, once per base. After the call's preconditions are established: one
  base whose extent decides `0 <= start` and `start + LEN <= N` makes the
  result that base at `start` (so `q - a` is a same-object subtraction), unless
  the call may move a base the source did not establish; a base too small is
  `bounds-declaration-contradicted` and no fact; otherwise the declared view
  of `LEN` units, when every relation states the same one (multi-base).
* **Refutation by a body** (CG16's phase one, extended). A live write through
  an unwritten parameter declared read, or its hand-off to a stronger frame
  position, refutes the frame; returning a source object, a parameter that is
  no declared base, or a base at a constant element offset no relation states
  refutes the relation. A refuted declaration is withdrawn from the unit's
  callers. Anything the body does not establish stays `Declared`.

## 2. Cited claims (CG00's BEFORE controls)

`crates/qcore-c-gateway/tests/clang_gap_phase4.rs` reproduced on the base and
is unchanged by the card:

| Entry | Claim | Outcome |
|---|---|---|
| K04 | resource v1 has no read/write frame clause; Borrowed is not purity | reproduced (`k04_no_read_write_frame_clause_exists`: `resource_reads(p)` rejected). Unchanged: the frame vocabulary is a bounds vocabulary; resource v1 still rejects it (`the_resource_v1_consumer_rejects_frames_and_relations`) |
| K06 | `Derived` is one parameter with Exact/Interior; no offset, length, multiple bases or callback span | reproduced (`k06_derived_results_have_no_offset_or_length`). Unchanged for custody; the bounds vocabulary now states offset/length and several bases; the callback span is a residual |

Q29's claim (the custody `Derived` mode carries no offset or length) holds on
this base (`qcore-schemas` unchanged). Q20 and Q31 are cited for K04 as
places the vocabulary is read; neither states a frame, which holds. W46's
measurement of `cfl_sds_cat_safe` reproduced exactly: **170** uncontracted
call leaves on the corpus, the only obstacle of **3** functions
(`format_metric_type`, `append_bytes_value`, `sds_cat_safe`), in 36 non-Safe
functions. W45's "reach of a relation clause alone in this slice is 0"
reproduced (§3). No cited claim failed to reproduce.

## 3. Census, prediction and outcome

[PREDICTION.md](PREDICTION.md) was frozen at `2521eba1`, before any
implementation code. Measured with [`scripts/measure.sh`](scripts/measure.sh)
(corpus `66910c10` `--jobs 4`, system, tail-pause slice; the bounds property
selected on all three, with the empty bounds environment the audit routes
use); summarised by [`scripts/routes.py`](scripts/routes.py) and
[`scripts/census.py`](scripts/census.py) into
[`artifacts/routes.json`](artifacts/routes.json). BEFORE binary `86bb8683…`
(tree `2521eba1` = `6cab1ad9` + this card's prediction), AFTER binary
`cd5cb59d…` (tip `06d76faf` on the same base, before the rebases). Clang 21.1.8.

**Census (BEFORE).**

| Route | Custody | Bounds | `cfl_sds_cat_safe` leaves / functions / sole | backlog leaves / functions / sole | bounds: pointer to a callee with no bounds contract / sole | rows with a declared fact |
|---|---|---|---|---|---|---|
| Corpus | 645 / 10 / 4,179, `ceefb6c4…` | 162 Safe / 4,759 Unknown (4,921 rows), `5671d81d…` | 170 / 36 / 3 | 1 / 1 / 0 | 4,133 / 660 | 0 |
| System | 44 / 2 / 188, `eba699ee…` | 24 / 430 (454 rows), `addfef27…` | 0 | 17 / 7 / 0 | 371 / 31 | 0 |
| Slice | 29 / 1 / 239 / 1 Pending, `5278f146…` | 13 / 393 (406 rows), `206587f4…` | 0 | 0 | 358 / 30 | 0 |

Custody Unknowns that need a frame or a derived relation: the `cfl_sds_cat_safe`
family (an in-place reallocation through a slot; its custody half is not this
card's, §6) and no derived-result refusal at all (0 leaves naming a derived,
interior or result-alias refusal on any route). Bounds Unknowns a frame could
free today: **0**: with no bounds declaration selected, no fact on an object
the source did not establish exists, so no call withdraws one, and no local
is passed to a framed position. The reach of callee declarations (frames and
relations among them) is bounded by the functions with a pointer passed to a
callee with no bounds contract (the column above), which need
CG16-BOUNDS-CONTRACT-TRANSPORT first.

**Outcome.**

| Route | Measure | Predicted | BEFORE | AFTER |
|---|---|---|---|---|
| Corpus | custody `resource-diagnostics.json` | byte-identical | 645 / 10 / 4,179, `ceefb6c4…` | **byte-identical** |
| Corpus | bounds rows | byte-identical | 162 / 0 / 0 / 4,759, `5671d81d…` | **byte-identical** |
| System | custody | byte-identical | 44 / 2 / 188, `eba699ee…` | **byte-identical** |
| System | bounds rows | byte-identical | 24 / 0 / 0 / 430, `addfef27…` | **byte-identical** |
| Slice | custody | byte-identical | 29 / 1 / 239 / 1 Pending, `5278f146…` | **byte-identical** |
| Slice | bounds rows | byte-identical | 13 / 0 / 0 / 393, `206587f4…` | **byte-identical** |

Every prediction held: **no function moved on any route, for either
property**, so there is no Safe → non-Safe move, no added leak or violation to
classify, and no correction. The system and slice reports embed the measuring
checkout's paths, so their digests are this worktree's (CG16 §3). The corpus
custody digest is main's stated `ceefb6c4…`, and the corpus bounds tally equals
CG16's at `b6a3dc42` (162 / 4,759 over 4,921 rows), now measured on
`6cab1ad9`. The vocabulary's effect is shown by the executed controls (§4),
not by the routes, which select no bounds declaration.

The card was rebased onto main `01003bc1` (CG29: execution coverage and
reclamation) and then `1a6a99c0` (CG23: relational postconditions, a
resource-v1 clause in the contract loader) after the AFTER; the rebased tree
was checked (`cargo check --workspace --all-targets` 0) and gated (§7). Both
report their corpus byte-identical (`ceefb6c4…`), and this card's code reads
none of their items (the bounds loader and consumer are disjoint from the
resource loader CG23 extends), so the comparison is not repeated on the
rebased tree.

## 4. Controls

`tests/cg24_frames_and_relations.rs` (8 tests through the route-G consumer),
`crates/qcore-contract/tests/bounds_contract_frames.rs` (6, the loader) and
`crates/qcore-core/tests/index_extent_frames.rs` (6, the decisions and the
shared vectors):

| Control | Outcome |
|---|---|
| a read-only callee keeps a bounds fact (`p` declared 8 elements, `touch_it(p); p[7]`) | reads: Safe; writes: Safe (contents only); moves: Unknown; no frame: Unknown (CG16); custody: `touch_it` still uncontracted |
| a writing callee drops it (`int i = 2; index_of(&i); a[i]`) | reads: Safe; writes: Unknown (`bounds-unsupported-range`); no frame, an escaped address or a held frame: the local is untracked (Unknown) |
| pointer slot (`p = a + 1; slot_of(&p); p[2]`) | reads: Safe; writes: Unknown |
| loops | a read in the body keeps `i`; a write withdraws it |
| the sds control (`buf = make(16); old = buf; cfl_sds_cat_safe(&buf, "x", 1)`) | reads: `old[15]`, `buf[15]` Safe; writes (slot rewritten, object kept): `old` Safe, `buf` Unknown; moves: both Unknown; `"x"` for a length of 3: not Safe; custody: still uncontracted |
| derived result (`q = skip(a, 2)`, 4 elements) | in `int a[8]`: `q[5]` Safe, `q[6]` not Safe, `q - a` Safe (same object); in `int a[4]`: `bounds-declaration-contradicted`; unknown base: the view, `q[3]` Safe, `q[4]` not |
| multi-base (`pick(x, y)`) | `x[4]`, `y[8]`: `r[3]` Safe, `r[5]` not; `x[2]`: contradicted |
| a relation contradicted by its body refused | `q[0] = 1` under `reads`: refuted, the caller Unknown; under `writes`: not refuted, caller Safe; a `writes` frame handing to a `moves` callee: refuted; `return p + 1` / `return store` against `p + 2`: refuted; `return p + 2` / `&p[2]`: not |
| held frame and held relation | Pending as the sole obstacle, the hold named |
| loader | closed frames, one per parameter, frames as postconditions, slots only in frames, preserves vs moves, derived bases, results, exclusions, units: each rejects the module; resource v1 rejects both families |

## 5. Lean

`QCore.IndexExtentFrames` models the frame as a filter over CG16's facts and
the containment as a decision over `Count`/`IndexFact`;
`AuditIndexExtentFrames.lean` audits 29 theorems (only `propext`,
`Classical.choice`, `Quot.sound`; no placeholder): a write frame invalidates
the facts it may touch (`write_frame_withdraws_its_local`), a read frame
preserves them (`read_frame_preserves`), a write keeps unrelated extents
(`write_frame_keeps_unrelated_extents`), a move withdraws every declared fact
(`move_frame_withdraws_declared_facts`), frames only add precision
(`unstated_is_the_weakest`), a held frame's kept facts are held
(`held_frame_holds_declared_facts`), a derived relation's extent stays within
its base's (`derived_view_within_base`), a contradiction is exact and real
(`derived_contradiction_sound`), one contradicted base refuses a multi-base
view, and refutation (`a_refuted_frame_is_unstated`,
`a_body_within_its_frame_never_refutes`); `frameVectors` (11),
`derivedVectors` (11) and `refuteVectors` (9) are decided by `decide` and,
parsed from the Lean text, by the Rust module. Rust refuses an overflowing
containment that Lean's unbounded integers would decide (stricter, tested).

**Proof, binding, observation.** The theorems are about the model; the Rust
decisions are bound to it by the shared vectors; the consumer's reading of
Clang and its hooks are bound by the executed controls of §4 and observed on
the three routes (§3). Trust boundary 28 is a stated premise, and so is C06's
`Declared` reading of a resolved frame or relation.

## 6. Ledger

By [`scripts/update_ledger.py`](scripts/update_ledger.py) (`named_followups` 681 → 682 on main `1a6a99c0`: one retired, two registered):

* **CG24-ALIAS-EXTENT-FRAME-VOCABULARY: retired.** Every DISCHARGE clause
  holds: the BEFORE reproduced K04 and K06, the prediction was frozen first,
  the AFTER moved no function, both absorbed entries are re-registered with
  re-measured sizes, and the decisions have Rust controls and audited Lean.
  The scope it did not reach is registered.
* **W46-SDS-CAT-SAFE-IN-PLACE-REALLOC: amended.** Its bounds half landed (the
  sds control); its DISCHARGE is a resource-v1 reallocation-through-a-slot
  item with gateway and Lean parity, a provider declaration and an arbiter,
  which this card did not add. Re-measured: 170 / 36 / 3 on the corpus, 0 on
  system and slice.
* **W45-FB-BACKLOG-REFERENCE-VOCABULARY: amended.** Its relation is a custody
  relation (a wrapper refers to a chunk), with return-conditioned clauses and
  per-element identities; none landed. Re-measured: 7 system functions carry
  the backlog declarations' leaves (17), none as their only obstacle; 1
  corpus leaf; 0 on the slice.
* **Registered:** `CG24-CALLBACK-SPANNING-RELATIONS` (needs CG22's callees;
  0 functions on the audit routes) and `CG24-FRAME-AND-SLOT-PRECISION`
  (frames refuted but not summarised, loops ignore read frames, held frames
  track no local, no slot extent after a move, closure unchecked; 0 functions
  on the audit routes).

## 7. Gate

GATE_SECTION
