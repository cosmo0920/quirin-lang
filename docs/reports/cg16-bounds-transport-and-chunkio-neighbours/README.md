# CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS

Wave 48, round 21. Authority: the ledger entries `CG16-BOUNDS-CONTRACT-TRANSPORT`
(the CHUNKIO-META-LEN-BOUNDS-CONTRACTS amendment's five open items) and
`CHUNKIO-FILE-ST-NEIGHBOUR-BOUNDS`. Gateway rules
[14f](../../agents/gateway.md#constraint-14f)–[14j](../../agents/gateway.md#constraint-14j);
contract specification C09.1 (status paragraph "Body evidence, refutation
phase and tips"), C06.2, C06.4, C06.5; grammar G05b. Lean:
`QCore.BoundsBodyStanding` (18 theorems, `proof/AuditBoundsBodyStanding.lean`)
and `QCore.BoundsTransport` (14 theorems, `proof/AuditBoundsTransport.lean`).
Base: local main `84ec54d1d`. Prediction frozen in [PREDICTION.md](PREDICTION.md)
(`18f61b181`) before implementation code.

**Published property and scope.** CG16's `c.bounds.index-extent.v1` (rules
11s, 11e, 12l, 12r), with body evidence for its declarations. Custody is
unchanged. No verdict reads anything this card adds.

## 1. Task A: the rest of the transport

| Item | Outcome | Rule | Controls |
|---|---|---|---|
| Header-defined bodies on the routes (the one that matters most) | **Done, as evidence.** Phase one already analysed every declared body a unit defines, header `static inline` definitions included, to withdraw a refuted declaration in-unit; the result was discarded. It is now kept per unit and folded per declaration into `body_evidence` of the bounds selection record, with a bounds Safe roll-up (C06.5's shape, rules 13g–13k). | 14f, 14g | `crates/qcore-c-gateway/tests/bounds_body_evidence.rs` (4), `crates/qcore-audit/tests/bounds_transport.rs::a_header_body_is_evidence_for_its_declaration_and_not_a_row`, core vectors `crates/qcore-core/tests/bounds_body_standing.rs` |
| Cross-unit refutation phase on the routes | **Done.** The table is the union of every unit's refuting bodies (from the evidence, computed under the declarations as loaded); phase two runs when the custody or the bounds table is not empty, with the bounds table in `BoundsContracts::refuted`. | 14h | `a_body_refuting_its_declaration_withdraws_it_in_every_unit` (another unit's caller Safe → Unknown, never Unsafe), `the_empty_table_changes_nothing` |
| Tip tracking for bounds pins | **Done.** `pin --bounds --track-tip/--untrack-tip`; the record's `tip-in-force` standing, the tip record, the notice, `assumptions.tip_tracked` and the console notice, as C06.2 gives resource declarations. | 14i | `a_bounds_tip_entry_is_a_named_assumption_in_force_and_held_otherwise` (in force, other sites → `source-changed`, a deleted ref → `tip-unresolved`), `tip_vectors_match_lean` |
| `.qo` checkpoint transport | **Not done; refused instead of dropped.** Before the card a checkpointed `current` run with `QCORE_BOUNDS_PROPERTY` computed bounds rows into fragments no section carries and the composer never wrote: the requested rows were silently absent. It now refuses. | 14j | `checkpoint_cli.rs::a_checkpointed_run_refuses_the_bounds_selection` |
| G05a inclusion and inheritance | **Not done; kept refused.** The bounds loader refuses `#include` and `contract inherit`; the probe leaves such a module out of every view; a load path naming one in a bounds view refuses the selection. | 14j | `an_inheriting_bounds_module_never_reaches_a_bounds_view`, `bounds_contract.rs::malformed_declarations_reject_the_whole_module` (inheritance case added) |

**Header bodies: why evidence and not rows.** C09.1 and rule 12r define the
routes' denominator as each unit's main-source definitions, and the spec does
not admit header rows; this card does not change that. A header definition is
analysed once per including unit (each unit's own preprocessed body), so the
declaration is the right subject; a row would need a custody row to join
(there is none), and would multiply one function by its including units.

**Coverage, conservatively.** A `Safe`, unrefuted body covers its quant checks
and preconditions. A frame clause is covered only by a positive, syntactic
frame check (rule 14g): the refutation check (rule 11e(4)) does not follow
copies, so "not refuted" is not evidence of a frame. A derived relation is
covered by nothing even when every `return` returns the declared base and
offset, because its length is a fact about the caller's object. Return
extents, `bounds_preserves_extents()` and global extents are covered by
nothing.

**Why the two items are not done.** Checkpoints: a `.qo` file would need
bounds rows and body evidence in a section, the bounds environment's digest
and the bounds refutation table's digest in its unit identity (a reuse under
another environment or table must recompute), and composition of the bounds
selection record; a partial version would publish rows from a stale
environment. Inclusion and inheritance: the resource loader's agreement gate
compares loaded records; a bounds gate must compare quant predicates and
counts under the positional renaming, and the bounds environment has no
retained-evidence replay (resource environments do, in
`check_source_assumptions`). No bounds module needs either today (0 modules,
0 checkpointed bounds runs). Both stay on `CG16-BOUNDS-CONTRACT-TRANSPORT`
with their discharge controls named.

## 2. Task B: the chunkio neighbours

`lib/chunkio/include/chunkio/cio_file_st.h` is byte-identical at `66910c10`
and `046accf6` (sha256 `02eb3909…`). New module
[`chunkio/file_st_bounds.qv`](../../../providers/qcore-provider-chunkio/contracts/chunkio/file_st_bounds.qv),
in the crate's bounds view, pinned with `pin --bounds` at both revisions
([pin log](artifacts/pin.log), [check log](artifacts/pin-check.log): 5
recorded at each, exit 0):

| Accessor | Body | Declared | Body evidence (unit test on the pinned body) |
|---|---|---|---|
| `cio_file_st_get_hash` (l.70) | `return map + 2;` | `bounds_frame_reads(map)`, `bounds_return_derived(map, 2, 4, bytes)` | partly validated: the frame confirmed; the relation covered by nothing (its length is the caller's); the body's formation `map + 2` has no extent |
| `cio_file_st_infer_content_len` (l.104) | scalar arithmetic, one `get_meta_len(map)` | `quant checks map, Idx #i: 22 <= #i && #i < 24`, `bounds_frame_reads(map)` | validated (the escape to `get_meta_len` decided inside; the hand-off at a read position) |
| `cio_file_st_set_content_len` (l.116) | writes `map[10..13]` through `(uint8_t *) &map[10]` | `quant checks map, Idx #i: 10 <= #i && #i < 14`, `bounds_frame_writes(map)` | partly validated: the byte view (rule 14g) decides the four writes inside; the frame walk cannot follow the view |
| `cio_file_st_get_meta` (l.89) | `return map + 24;` | **not declared**: its length is the metadata length read from `map[22..23]` | — |
| `cio_file_st_get_content` (l.95) | `map + 24 + get_meta_len(map)` | **not declared**: an offset read from the object | — |
| `cio_file_st_get_content_len` (l.129) | the byte view of `map[10..13]`, `map[24 + meta_len]` under `size > content_offset` | **not declared**: `bounds_extent(map, size, bytes)` and an index read from the object | — |

**`CG16-BOUNDS-SYMBOLIC-RANGES` is not closed narrowly** for "a length read
from fixed bytes of a covered extent". The consumer keeps facts on locals and
parameters, never on object contents; a length read from `map[22..23]` would
need content facts that every write through an alias, a frame position or a
call withdraws, and no corpus caller has an extent to compare it with. The
missing vocabulary (a content-read length term with content facts and their
invalidation) is named on that entry.

## 3. Prediction against outcome

MEASUREMENT

## 4. Lean

`QCore.BoundsBodyStanding`: coverage (`a_refuting_body_covers_nothing`,
`nothing_covers_these`, `body_evidence_needs_a_safe_body`,
`frame_evidence_needs_the_frame_check`, `a_safe_body_alone_never_covers_a_frame`),
units (`a_held_unit_keeps_no_evidence`, `a_refuting_unit_fails`), standing
(`a_held_declaration_is_held`, `no_examined_body_is_a_declared_assumption`,
`validated_needs_every_unit_and_clause`, `a_failure_wins`,
`partly_names_its_uncovered_clauses`), roll-up
(`only_validated_needs_every_dependency_validated`,
`an_unknown_dependency_is_an_assumption`) and the shared vectors.
`QCore.BoundsTransport`: refutation (`the_empty_table_changes_nothing`,
`its_own_body_keeps_it`, `a_refuted_declaration_is_withdrawn`,
`a_withdrawal_never_accuses`, `a_caller_of_a_refuted_declaration_is_never_safe`,
`mem_bounds_table_iff` from `QCore.RefutedContractWithdrawal`,
`the_table_is_stable_across_phases`), tips (`a_tip_is_in_force_iff`,
`an_out_of_force_tip_is_held` through `QCore.BoundsPinHold.standing`,
`a_tip_assumption_is_listed_iff_in_force`, `tip_vectors_hold`), checkpoints
(`a_composition_would_omit_the_requested_rows`,
`a_bounds_run_is_never_composed_from_checkpoints`). Not modelled: the C
bodies, the frame walk and Git (trust boundary 28; the Rust controls above).

## 5. Follow-ups

FOLLOWUPS

## 6. Gate

GATE
