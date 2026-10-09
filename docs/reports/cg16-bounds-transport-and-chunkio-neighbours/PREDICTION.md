# CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS: prediction (frozen before implementation code)

Base: local `main` at `84ec54d1d` (corpus 881 / 9 / 3,944, verdict-row digest
`321a8913…`; system 87 / 4 / 357; slice 45 / 1 / 354, 4 Pending; follow-ups
740; Lean 303; rules through `13t`). This card's rule ids are `14f`–`14j`.
Written after reading the two ledger entries, contract specification C06.1,
C06.2, C06.5, C09.1 and C09.2, grammar G05a–G05c, gateway rules 11s, 11e, 12l,
12r and 13f–13k, the report `docs/reports/chunkio-meta-len-bounds-contracts/`,
the bounds consumer (`qcore_c_gateway::index_extent`, its phase one and
`quant_checks`), the audit's bounds selection (`bounds_selection.rs`,
`projection.rs::record_bounds`, `report.rs::write_bounds_selection`), the pin
tool (`contract_pin.rs`), the refutation two phases (`refuted_contracts.rs`)
and the checkpoint route (`checkpoint.rs`, `checkpoint_run.rs`), and both
pinned revisions of `lib/chunkio/include/chunkio/cio_file_st.h`
(byte-identical, sha256 `02eb3909…`), before any implementation code.

## What the code already does (read, not assumed)

* Phase one of `analyze_bounds_preprocessed` already analyses, in every unit,
  the body of every declaration that states a frame, a quant check, a return
  extent or a derived relation **when the unit defines it**, header
  `static inline` definitions included, and withdraws a refuted declaration
  from that unit's callers. What is missing for header bodies is publication:
  the phase-one result is computed and discarded, so a body check is
  invisible on the routes (the CHUNKIO-META-LEN prediction miss).
* The audit side of a tip entry already decides through `pin_standing`
  (`InForceAtTip`); `pin --bounds` refuses `--track-tip`, and the bounds
  selection record does not publish a tip as a named assumption.
* A checkpointed `current` run with `QCORE_BOUNDS_PROPERTY` set computes bounds
  rows into per-unit fragments that the `.qo` sections never carry and the
  composer never writes: the requested bounds rows are silently absent.

## Design decisions

1. **Header bodies: evidence, not rows.** C09.1 and rule 12r say a
   header-defined function has no row on the audit routes; this card does not
   change the spec. The phase-one result of every declared body a unit
   defines is published as **body evidence for the declaration**, per unit, in
   the bounds selection's sidecar, folded per declaration as rules 13g–13j
   fold custody evidence: `validated` only when every unit that examined a body
   validated it and at least one did, with every clause covered; otherwise
   `partly-validated` (covered clauses listed) or a `declared-assumption`;
   `held` for a declaration held by a review marker or a pin. A refuting body
   is a failure in its unit and wins there. Coverage (conservative, "handle
   as safe as possible"): a body whose row is `Safe` and does not refute
   covers its quant checks and its preconditions (`bounds_extent`,
   `bounds_index`); a frame clause is covered only by a separate positive
   frame check on the body (no write except to a local's own storage or, for a
   write or move frame, through the parameter itself; the parameter never
   copied, never assigned, never passed except at a closed frame position no
   stronger than its own); a derived relation only when every `return`
   returns a declared base at a declared constant offset; a return extent,
   `bounds_preserves_extents()` and `bounds_global_extent` are covered by
   nothing. No verdict reads any of it. A Safe roll-up of the bounds rows
   (resting on no declaration / only validated / partly / a declared
   assumption) is published beside it (rule 13k's shape).
2. **Cross-unit refutation phase.** The routes' bounds rows get the two-phase
   table C06.4 gives custody: phase one's refutations (a published row's
   `refutes_own_contract` and the body evidence's failures) are unioned; when
   the table is not empty every unit's bounds rows are analysed again with it
   in `BoundsContracts::refuted`; with an empty table nothing runs twice.
3. **Tip tracking.** `pin --bounds --track-tip/--untrack-tip` as for resource
   locks; a bounds tip entry in force is listed as a named assumption
   (`tip-tracked:FUNCTION`), its standing is `tip-in-force`, and each tip
   entry carries the C06.2 notice in the record and on the console.
4. **`.qo` checkpoints.** Fail closed first: a checkpointed run refuses the
   bounds selection rather than omit its rows; transporting bounds rows and the
   environment in the checkpoint is done only if it can carry the environment
   and table digests in the unit identity with reuse controls, otherwise
   registered with its size.
5. **Inclusion and inheritance (G05a).** Bounds modules elaborated through the
   load path's existing elaborator before the bounds loader reads them, with
   the inherited-declaration agreement check (child's loaded clauses equal the
   parent's under the positional renaming) and an inherited-marker control; or,
   if that cannot be done soundly inside this card, recorded with the reason.

## Task B: the chunkio neighbours (read at both revisions, byte-identical)

* `cio_file_st_get_hash(char *map)`: `return map + 2;` Declared (pinned with
  `pin --bounds` at both revisions): `bounds_return_derived(map, 2, 4, bytes)`
  and `bounds_frame_reads(map)`. Body evidence: **not validated** expected (the
  body forms `map + 2` on a formal with no extent, an Unknown formation);
  the relation is confirmed, the declaration a declared assumption.
* `cio_file_st_infer_content_len(char *map, size_t size)`: scalar arithmetic and
  one `cio_file_st_get_meta_len(map)`. Declared: `quant checks map, Idx #i: 22
  <= #i && #i < 24;` and `bounds_frame_reads(map)`. Body evidence: validated
  expected (the escape to `get_meta_len` is decided inside).
* `cio_file_st_set_content_len(char *map, uint32_t len)`: writes `map[10..13]`
  through `(uint8_t *) &map[10]`. Declared only if the quant body check reads a
  same-width byte-view cast of `&P[k]` as `P + k` (narrow, sound); otherwise
  its missing vocabulary is recorded. Not decided before measuring the
  consumer.
* `cio_file_st_get_meta`, `cio_file_st_get_content`,
  `cio_file_st_get_content_len`: need a length (or offset) read from bytes of
  the object (`map[22..23]`). Expected **not** implemented: a narrow closure
  of CG16-BOUNDS-SYMBOLIC-RANGES for that shape needs facts on object
  contents, which the consumer does not track (facts live on locals and
  parameters; any write through an alias or a call would have to withdraw
  them), and no corpus caller has an extent to compare it with. Recorded on
  the owning entries.

## Predicted measurements

| Route | Prediction | Why |
|---|---|---|
| Corpus arbiter (custody) | byte-identical: 881 / 9 / 3,944 | no custody code reads bounds contracts, bounds evidence or the bounds table |
| System route | byte-identical (87 / 4 / 357) | as above |
| Tail-pause slice | byte-identical (45 / 1 / 354, 4 Pending) | as above |
| Typed-path parity (three routes) | equal multisets | as above |
| Corpus with `QCORE_BOUNDS_PROPERTY`: custody report | byte-identical to the custody run | the bounds environment feeds only the bounds rows |
| Bounds selection | 4 declarations from 2 modules (5 if `set_content_len` is declared), all pinned in force at `66910c10` | the new module is pinned at both revisions |
| Bounds rows | every row's identity changes (environment digest); **no verdict moves**: 162 Safe / 4,759 Unknown; combined 111 / 4,801 / 9 | every corpus caller of an accessor passes `cf->map`, a member with no extent (CG16-BOUNDS-POINTER-EXTENTS); `infer_content_len` has no corpus caller |
| Changed reasons | the 9 meta-len obligations unchanged; the 2 `cio_file_st_get_hash(cf->map)` obligations in `cio_file.c` read the new declaration instead of `states no bounds contract` (3 more for `set_content_len` if declared); still `bounds-missing-extent` | as above |
| Callers passing `cf->map` | 11 calls (14 with `set_content_len`), every one Unknown | CG16-BOUNDS-POINTER-EXTENTS |
| Bounds Unsafe | none added | no caller establishes a smaller extent |
| Body evidence | the declared bodies examined in each corpus unit that includes `cio_file_st.h` (expected 7: `chunkio.c`, `cio_chunk.c`, `cio_file.c`, `cio_file_unix.c`, `cio_meta.c`, `cio_scan.c`, `flb_input_chunk.c`); `get_meta_len`, `set_meta_len`, `infer_content_len` validated (if the positive frame check covers their frames; otherwise partly-validated, frame uncovered); `get_hash` a declared assumption | the bodies at both revisions |
| Bounds Safe roll-up | 162 rest on no declaration, 0 on any | no Safe row consults a chunkio declaration |
| Refutation table | empty; no second phase | no declared body refutes its declaration |
| Wall time | the bounds-selected corpus within noise of BEFORE | phase one already analysed the declared bodies |

Any other move is a defect of this card, or a correction to be named.
