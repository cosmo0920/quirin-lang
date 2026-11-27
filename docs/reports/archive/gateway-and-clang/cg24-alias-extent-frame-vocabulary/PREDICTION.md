# CG24-ALIAS-EXTENT-FRAME-VOCABULARY: frozen prediction

Frozen before any implementation code of the card was written (only the
measurement script exists). Base: local main `6cab1ad9` (CG16 landed; stated
corpus 645 / 10 / 4,179, `ceefb6c4…`). The BEFORE run was waiting on the
shared heavy-step lock when this was written; its digests and every census
number are recorded in the report, measured on that BEFORE, not here.

## What the card will add (the subject of the prediction)

An extension of CG16's bounds vocabulary (contract specification C09.1,
grammar G05b, `qcore_contract::bounds_contract`), never a parallel one, and its
consumer in route G's bounds reader (`index_extent.rs`), the core decisions
(`qcore_core::index_extent`) and a Lean sibling of `QCore.IndexExtent`:

* **Frame effects**, distinct from custody modes: per pointer parameter (and a
  new pointer-to-pointer *slot* parameter), `bounds_frame_reads(P)`,
  `bounds_frame_writes(P)` or `bounds_frame_moves(P)`, stated for every pointer
  parameter or for none (a *closed* frame). A read frame keeps every fact; a
  write frame withdraws the facts of a local whose address it receives (value
  facts) and keeps extent facts on objects it only writes; a move frame (the
  in-place reallocation of the sds case) also withdraws every fact on an
  object the source did not establish. A local whose address is passed only
  directly to closed, resolved frame positions may carry facts.
* **Derived relations**: `bounds_return_derived(P, OFF, LEN, UNIT)`, one or
  several bases (multi-base), the result being `P + OFF` with `LEN` units;
  containment in the base's extent is checked at the call, and a body that
  returns something else, or writes through a parameter it declares read
  (or moves one it declares written), refutes its declaration (callers then
  see an uncontracted callee).
* **Not in this card**: the custody half of W46-SDS-CAT-SAFE-IN-PLACE-REALLOC
  (a resource-v1 mode for reallocation through an out slot) and the relation,
  return-conditioned and per-element clauses of
  W45-FB-BACKLOG-REFERENCE-VOCABULARY; callback-spanning relations (they need
  CG22's indirect callees). These are custody vocabulary, outside the bounds
  property this card extends; both entries are predicted to be amended with
  re-measured sizes, not closed.

## Custody (the existing property)

No custody reader calls the bounds consumer and no custody code changes.

| Route | Baseline (main `6cab1ad9`) | Predicted AFTER |
|---|---|---|
| Corpus arbiter | 645 / 10 / 4,179, `ceefb6c4…` | **byte-identical** `resource-diagnostics.json` (band 0) |
| System | as measured BEFORE | byte-identical (band 0) |
| Tail-pause slice | as measured BEFORE | byte-identical (band 0) |

## Bounds (the property this card extends, reported separately)

The audit routes select the bounds property with an **empty** bounds
environment (CG16-BOUNDS-CONTRACT-TRANSPORT is open), and the new clauses are
reachable only through a bounds declaration. Predicted with the property
selected on all three routes:

| Route | Predicted AFTER bounds rows | Band |
|---|---|---|
| Corpus | byte-identical to BEFORE (CG16's tally there: 162 / 0 / 0 / 4,759 at `b6a3dc42`; this base's figures as measured) | 0 moves |
| System | byte-identical to BEFORE | 0 moves |
| Tail-pause slice | byte-identical to BEFORE | 0 moves |

Census predictions (each measured on the BEFORE and recorded in the report):

* bounds facts a call withdraws on these routes: **0** (a fact on an object
  the source did not establish exists only through a declaration), so the
  frame vocabulary's reach on the audit routes today is **0** functions;
* the functions a callee declaration (frames, extents, derived results) could
  reach are bounded by those whose bounds Unknown includes a pointer passed to
  a callee with no bounds contract (CG16: 4,133, 660 as the only reason, at
  `b6a3dc42`); predicted the same order on this base;
* custody: `cfl_sds_cat_safe` stays an uncontracted call on every route
  (predicted unchanged from W46's 170 call sites order of magnitude; exact
  figures re-measured), and W45-FB-BACKLOG-REFERENCE-VOCABULARY's slice reach
  stays 0.

## Controls (route G, executed)

* a read-only callee keeps a bounds fact (Safe); a writing callee drops a
  local's fact (Unknown); a moving callee drops a declared object's fact;
* the sds control: `cfl_sds_cat_safe(&buf, …)` declared `bounds_frame_moves`
  on its slot withdraws the facts of `buf` and of an alias of the old base;
* a derived result inherits its base's object when containment is decided,
  is refused (contradiction) when the base is too small, and a body that
  returns another object or writes through a read-declared parameter refutes
  its declaration;
* held frames and relations stay `Pending` only as the sole obstacle;
* resource v1 still rejects `resource_reads` and an offset-carrying
  `resource_return_interior` (CG00's K04 and K06 controls unchanged).
