# CG18-FOOTPRINT-CONNECTION

Wave 48, round 7. Card CG18 of the Clang gap queue (entries M20, M21, M22,
M26, W08), route G and the core model; gateway rule
[11k](../../agents/gateway.md#constraint-11k). Lean: the CG18 section of
`QCore.DisjointFootprint` ([`proof/QCore/DisjointFootprint.lean`](../../../proof/QCore/DisjointFootprint.lean),
audited in `proof/AuditDisjointFootprint.lean`). Base `b59aaae2` (main with
CG07); the final main `436e0ee7` merged and re-measured (§3.1). Absorbs `W43-INDIRECT-BYTE-STORE-REPRESENTATION-PREMISE`.

CG00 found that this card frees no corpus function; it is judged by what it
proves and by making no `Safe` claim it cannot back. It frees none.

## 1. What changed

**Core model** ([`disjoint_footprint.rs`](../../../crates/qcore-core/src/disjoint_footprint.rs),
additive: every new behaviour is a new event, so every older trace and
certificate is unchanged, `without_the_premise_nothing_changes`):

| Entry | Event | Decision | Premise named on the certificate |
|---|---|---|---|
| M21 | `DeclareFreshAllocation { alloc }` | two allocation identities are disjoint only when both live generations were declared fresh (C11 7.22.3p1), judgement `DistinctAllocations`; otherwise `Q662` as before; a reallocated generation is not fresh | `DistinctAllocationsDisjoint` (`P-DISTINCT-ALLOCATIONS-DISJOINT`), stamped only when a judgement used it |
| M22 | `DeriveView { handle, base, view }` | a character view is admitted with no premise (C11 6.5p7); an effective-type view is admitted; a punned view is the excluded `TypePunnedAlias` (`Q667`); a view of a bit-field unit or a descriptor refuses (`Q659`); a view is never a field base | `EffectiveTypeDeclared` (`P-EFFECTIVE-TYPE`) |
| M26 | `ByteCopy { destination, source, bytes, may_overlap }`, `ByteFill { destination, bytes }` | both ranges inside their handles (`Q653`, zero bytes `Q650`); a `memcpy` whose ranges share a byte stays the excluded `OverlappingCopy` (`Q667`); a `memcpy` across two allocations needs the M21 premise (`Q662`); `memmove` may overlap | `ByteRepresentation` (`P-BYTE-REPRESENTATION`) on every admitted transfer |

The certificate gains `fresh_allocations`, `derived_views` and
`byte_transfers`, and the independent recheck compares all three.

**Mutex invariants** (W08, [`footprint_mutex.rs`](../../../crates/qcore-core/src/footprint_mutex.rs)).
`compose_footprints_with_mutex_invariant` runs `CheckedMutexInvariantV1` and
`DisjointFootprintsV1` unedited and admits the pair only when every interval
the mutex trace names (shares, protected shapes, accesses, borrows, facts,
growths) is exactly the exported interval of a live derived handle (else
`Q665`), and every pair of two resources has both declared fresh (else
`Q662`: the mutex model alone calls two resources never overlapping, which is
the free cross-allocation claim M21 forbids). An admitted composition drops
`DeclaredFootprintShape` for the footprint premises. This is the design
document's follow-on F3, with the identity rule it was blocked on: a resource
is the allocation whose live handle exports exactly the named interval.
`source.mutex.guard.v1` reaches it
([`source_mutex_footprint.rs`](../../../crates/qcore-frontend/src/source_mutex_footprint.rs)):
each `protected` declaration is one fresh allocation. The route's own
certificate is unchanged.

**Route G** ([`footprint_connection.rs`](../../../crates/qcore-c-gateway/src/footprint_connection.rs)).
The elaborator refused every unordered pair of two path places (rule 10s);
CG13 left the pairs the core proves disjoint refused for this card. A pair is
now admitted when both places lie, by `.` member steps and parentheses alone,
in two different members of one `struct` object whose base is **structurally**
identical on both sides, and the core model admits `AssertDisjoint` on the two
members of one allocation generation under a layout contract read from
Clang's `-fdump-record-layouts-complete`. The struct is read from the members'
own `FieldDecl`s (RecordFacts `struct_members`), never from the base's type
spelling. Everything else keeps the refusal: two base objects (M21), a
subscript, `*` or `->` above the diverging member, one member on both sides,
a union member, an unnamed struct, two bit-fields of one C11 unit.

**Generations** (M20). Rule 11a's current and previous identities of a site
are two core allocations; the older summary has none; a reallocation is the
core `Reallocate`, which retires every earlier handle (`Q652`), as the flow
reads the old pointer as `UnknownAlias`. A replay of rule-11a events against
the core checker checks the correspondence (`rule_11a_generations_agree_with_the_core_model`);
no liveness check of the flow changed.

**Shared items added** (additive only): `elaboration.rs`
`Context::disjoint_places` (default `false`) and `EvalGraph::check_with`
(`check` is the declining case and now calls it; `elaborate` asks
`check_with`); `union_bitfield.rs` `RecordFacts::struct_members`;
`legacy_elaboration.rs` one hook. `typed_place.rs`, `call_instantiation.rs`
and every `FORM_TABLE` row are untouched.

**Diagnostics.** No code is allocated (the `Q-NAMESPACE-DECISION` icebox):
eight conditions of the `Q650`–`Q667` band are widened, stated in §L21 and in
[diagnostics](../../agents/diagnostics.md#cg18-footprint-connection-amendment).

## 2. Cited claims

Each claim of the gap analysis (§5 rows, evidence Q08, Q10, Q20, Q29, Q33),
reproduced as an executed test on this base:

| Entry | Claim | Outcome |
|---|---|---|
| M20 | generations exist in the core and are not connected to general C places | **reproduced**: `m20_core_retires_handles_of_a_reallocated_generation`; the gateway's stale alias is `UnknownAlias` (`m20_stale_aliases_are_unresolved_not_generation_checked`) |
| M21 | the core refuses disjointness across two allocations and does not assume distinct ids disjoint | **reproduced**: `m21_core_refuses_disjointness_across_allocations` (`Q662`); custody needs no disjointness (`m21_cross_allocation_accesses_need_no_disjointness_for_custody`, `Safe`) |
| M22 | G assumes effective-type rules; T rejects `restrict`; the core excludes `TypePunnedAlias` | **reproduced**: a punned scalar load is custody `Safe`, T's refusal text, the core's `Q667` |
| M26 | custody contracts do not express byte semantics; the core excludes `OverlappingCopy` | **reproduced**, with a correction of emphasis: in CG00's harness `memcpy` is uncontracted, so the gateway refuses it outright; custody contracts elsewhere (`W43-LIBC-RESULT-ALIAS`) are unchanged |
| W08 | the footprint prototype is not integrated and mutex composition is follow-on; `source.socket.v1` is a limited route | **reproduced**: before the card no code composed the two checkers (design document F3 open) |

## 3. Prediction and outcome

[PREDICTION.md](PREDICTION.md) was frozen at `4971751a`, after the BEFORE and
before any release binary with the card's code was built. The gateway hook
had been drafted in the worktree; the prediction says so.

| Measure | Predicted | Outcome |
|---|---|---|
| BEFORE reproduces main | — | corpus **byte-identical** to main's `8b9e4047…` (608 / 10 / 4,216); system 41 / 2 / 191 and slice 28 / 1 / 240 / 1 Pending, the stated baselines (`819a2841…`, `d663914d…`) |
| AFTER digests | — | corpus `41c7cefd…`; system and slice unchanged |
| Corpus Unknown → Safe | 0 (band 0–2) | **0** |
| Safe → non-Safe | 0 | **0** on every route |
| Provisional Unsafe added | 0 (0–1) | **0** |
| Functions whose leaves change | 2 (0–3) | **2**: `mk_http_parser` (3 `unsupported` → 3 `unknown_alias`), `rd_kafka_mock_partition_pidstate_get` (1 → 1) |
| Pairs admitted | 3 (0–3) | **5 pairs** (corpus parity: `p->header_max`/`p->header_min`, `p->header_val`/`p->start`, and three pairs of `pidstate->lo = pidstate->hi = pidstate->window = 0`, which the prediction counted as one row) |
| System, slice | byte-identical | **byte-identical** (41 / 2 / 191; 28 / 1 / 240 / 1 Pending); no pair consulted on either |
| New premise in a published verdict | none | **none** |

**Attribution.** In both functions the admitted statements' accesses now
reach the flow, which reports each as an unresolved identity
(`unknown_alias`) where it reported the statement unsupported before: the
roots (`p`, `pidstate`) are bound by the body and not read as untracked
there (rule 11d's residual classes). Both functions keep other leaves. The
five cross-object pairs (`rd_kafka_send`, `rd_kafka_toppar_enq_msg`,
`rd_kafka_mock_sharegroup_target_assignment_set`, `mk_sched_register_thread`,
`decode_static_field`) keep the refusal (`no-common-object`):
`CG18-CROSS-OBJECT-PATH-PAIRS`.

**A discarded attempt.** The first AFTER build read the struct from the
base's type spelling; Clang does not desugar a pointer to a typedef
(`rd_kafka_mock_pid_t *`), so `pidstate`'s pairs refused as `unnamed-record`.
The run was stopped before it finished, kept under
`private/cg18fc/discarded-after-1`, and not used; the fix (`a03ef0d4`) reads
the members' own declarations, and the AFTER above is its build. The first
BEFORE attempt was likewise stopped (main moved to `b59aaae2` during it) and
kept as `private/cg18fc/discarded-before-killed-attempt`.

### 3.1 Re-measured on main `436e0ee7` (the final tip)

Main moved four times during the card (W48-CALLEE-CLOSURE-R3, CG14, CG15,
CG19, CG09). The card merged `e93dbe4e` and measured it (0 moves, the same two
functions, system and slice byte-identical, gate green: 7,108 passed;
[identity](artifacts/identity-e93dbe4e.json), [gate exits](gate-e93dbe4e/)),
then merged the final tip `436e0ee7` (`107fa2aa`), took a BEFORE on
`436e0ee7` itself (binary `6bc9348f…`) and an AFTER on the merge (binary
`94d7075d…`). A BEFORE taken on `fe9c5b48` (`d1288b20…`, 616 / 10 / 4,208)
and its unfinished AFTER were stopped when main moved again and are not used.

| Route | BEFORE `436e0ee7` | AFTER merge `107fa2aa` | Movement |
|---|---|---|---|
| Corpus | 625 / 10 / 4,199, `ac2d4e19…` (= main's figure) | 625 / 10 / 4,199, `8391ba75…` | **0 moves**; leaves change in the same 2 functions (unsupported −4, unknown_alias +4) |
| System | 42 / 2 / 190, `63396983…` | byte-identical | 0 |
| Tail-pause slice | 29 / 1 / 239 / 1 Pending, `4c4d5653…` | byte-identical | 0 |

The member-pair consultations are the same ten (pair, decision) rows as on
the base ([member-pairs-main.json](artifacts/member-pairs-main.json)): 5
admitted, 5 cross-object refused. The movement relative to main is exactly
the card's own; no landed card changed it ([identity](artifacts/identity-main.json),
[comparison](artifacts/after-main-corpus.compare.json)).

**Composition notes for the integrator.** The card constructs no
`LegacyInstruction::IndirectScalarAccess` and matches none exhaustively, so
CG12's `readback` field needs no edit here. The W08 composition
(`qcore_core::footprint_mutex`) is separate from CG19's atomic route and its
`event_order_represented`; neither reads the other. CG15's
asynchronous-modification premise (rule 11h) is not restated: rule 11k names
only the effective-type, `restrict`, byte-representation and allocator
premises.

## 4. Axes (queue §4)

| Axis | Covered | Refused |
|---|---|---|
| Place | two members of one struct object, through `.` steps; bit-fields of two units; a bit-field and an ordinary member | two objects, a subscript/`*`/`->` above the member, a union member, an unnamed struct, one unit |
| Evaluation order | none chosen: a pair is admitted only as a commuting pair of the unordered group | — |
| Type | character and effective-type views (core) | punned views (core `Q667`, CG13's punned pointer read) |
| Boundary | the three premises, named where taken | `restrict` relied on nowhere (T refuses it) |
| Evidence | Clang's layout (trust boundary 28), `source.mutex.guard.v1` traces | C mutex evidence (none exists) |
| Verdict | no Safe claim added | — |

Route: G (member pairs), core consumers (`footprint_mutex`, the frontend
composition). Source profile: the custody gateway; `source.mutex.guard.v1`.

## 5. Tests and controls

- **CG00's controls** (`clang_gap_phase3.rs`, `clang_gap_core.rs`): **none
  flipped**. M20, M21, M22, M26 and W08's gateway controls pin custody
  limits that rest on none of the footprint premises, so none moves
  (`the_cg00_limits_are_unchanged` re-asserts them); the core controls
  (`core_footprint_refuses_every_listed_construct`,
  `m21_core_refuses_disjointness_across_allocations`) still hold for traces
  without the new events.
- **One flip, named:** CG13's `unsequenced_stores_to_two_bitfields_are_refused`
  (`tests/union_bitfield_places.rs`) pinned `(r->bf = 1) + (r->bh = 2)`, two
  bit-fields of two units, as refused "field disjointness is not admitted by
  this card"; it is now `Safe` by rule 11k, asserted in the same test. The two
  one-unit rows stay refused. CG02's `cg02_path_pairs_are_not_disjoint_and_not_called_undefined`
  and CG13's unit test run the elaborator with contexts that decline the new
  hook, so they are unchanged.
- **Admissions** (`tests/footprint_connection.rs`): member pairs through `->`
  and `.`, nested members, bit-fields of two units, a typedef'd pointer.
- **Refusal controls**: two allocations assumed disjoint without the premise
  (core `cg18_two_allocations_are_disjoint_only_under_the_named_premise`,
  gateway `s->a = t->b++`, composition `two_allocations_are_not_assumed_disjoint_without_the_premise`,
  frontend `two_protected_resources_compose_only_under_the_named_premise`);
  a punned read (`cg18_views_carry_their_premise_or_are_refused`); an
  overlapping `memcpy` (`cg18_memcpy_needs_disjoint_ranges_and_memmove_does_not`,
  `memcpy(p, p + 1, 4)`); plus the subscript, union, one-unit, unnamed-struct
  and pointer-hop pairs.
- **Shared vectors**: `pairVectors`, `copyVectors` and `generationVectors`
  are parsed from the Lean file and decided by the Rust checker and replay.

## 6. Lean

The CG18 section of `QCore.DisjointFootprint`, 21 theorems registered in
`AuditDisjointFootprint.lean`, no placeholder, no project axiom:

- **disjointness only under the named premise**:
  `without_the_premise_nothing_changes`,
  `the_premise_reads_only_distinct_allocations`,
  `cross_allocation_disjointness_needs_the_premise`,
  `an_admitted_judgement_is_grounded_or_premised`,
  `the_premise_never_admits_a_shared_unit`, `pair_vectors_hold`;
- **a punned view is refused or carries its premise**:
  `a_punned_view_is_refused`, `an_admitted_view_names_its_premise`,
  `a_unit_or_descriptor_has_no_view`;
- **byte transfers** (unlisted forms refused): `an_admitted_transfer_stays_inside_its_handles`,
  `an_admitted_memcpy_shares_no_byte`, `a_cross_allocation_memcpy_needs_the_premise`,
  `copy_vectors_hold`;
- **the mutex-invariant composition is sound**:
  `composition_binds_every_footprint`,
  `composition_backs_every_mutex_disjointness` (wherever the mutex profile
  calls two named footprints non-overlapping, the footprint model decides
  them disjoint, across resources only under the premise),
  `two_resources_without_the_premise_do_not_compose`,
  `an_underived_interval_does_not_compose`;
- **generation agreement with rule 11a**:
  `the_previous_generation_is_the_former_current`,
  `the_summary_has_no_footprint`,
  `two_generations_of_a_site_are_not_assumed_disjoint`,
  `generation_vectors_hold`.

## 7. The absorbed entry

`W43-INDIRECT-BYTE-STORE-REPRESENTATION-PREMISE`'s DISCHARGE has two arms; the
card takes the second: rule 11k (4) records that the premise a
character-typed store rests on (a field link is not invalidated by it; a
rewritten pointer is not used afterwards) is `P-BYTE-REPRESENTATION`, the same
premise the Borrowed `memset`/`memcpy` declarations rest on, under rule 10a's
"ordinary C object model under the no-undefined-behaviour reading"
(`AOM-POISON-NO-UB`). The entry is retired; no size to re-measure (it named
no corpus population).

## 8. Records and gate

The standing gate by [`scripts/gate.sh`](scripts/gate.sh), at the final-main
merge `107fa2aa`; logs `private/cg18fc/gate-final` (worktree-private), exits
in [`gate/`](gate/), summary in [`gate.json`](gate.json). Clang 21.1.8.

| Step | Exit | Count |
|---|---|---|
| submodule update (`--init --recursive`) | 0 | — |
| `python3 scripts/run_test_binaries.py` | 0 | **7,136 passed / 0 failed** / 11 ignored (407 units, 420 summaries; `private/test-binaries/20261001T003144Z-949205`) |
| `cargo fmt --all -- --check` | 0 | — |
| `cargo clippy --all-targets -- -D warnings` | 0 | — |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | — |
| `python3 scripts/check_lean_audits.py` | 0 | 260 audits |
| `python3 -m unittest discover -s scripts/tests` | 0, and 0 after `build_matrix.py . --gate gate.json` (`09a` 0, `09b`) | 365 |
| `python3 scripts/build_docs_index.py` | 0 | — |

Two earlier gates are kept. At `c4e5942b` (base `b59aaae2`) the first
runner had 3 failures: CG13's two-unit control, which rule 11k admits (the
deliberate flip of §5, made at `1d2ded8c`), and two timing-sensitive rows in
crates the card does not touch
(`checkpoint_cli::interrupted_run_preserves_completed_units_for_resume`,
`cycle20_verifier_plugins::vp04_vp07_vp08_process_lifecycle_and_payload_budgets`,
constraint 16); the full re-run passed 7,074 / 0 ([exits](gate-b59aaae2/), `03c`).
At the `e93dbe4e` merge every exit was 0, 7,108 passed
([exits](gate-e93dbe4e/)). `providers/` is unchanged, so the provider gate
was not due.

## 9. Follow-ups

- **Retired:** `CG18-FOOTPRINT-CONNECTION` (every DISCHARGE clause holds) and
  `W43-INDIRECT-BYTE-STORE-REPRESENTATION-PREMISE` (§7).
- **Registered:** `CG18-CROSS-OBJECT-PATH-PAIRS` (5 corpus functions, 0 system,
  0 slice) and `CG18-FOOTPRINT-CONSUMERS` (CG12's type view, a byte-safety
  property, C mutex evidence, the flow's generations).
- Scripts: [`scripts/`](scripts/) (`measure.sh`, `corpus.sh`, `compare.py`, `member_pairs.py`,
  `identity.py`, `update_ledger.py`, `gate.sh`); artifacts in
  [`artifacts/`](artifacts/).
