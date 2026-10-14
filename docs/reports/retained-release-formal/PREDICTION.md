# RETAINED-RELEASE-OF-A-FORMAL-READ-UNKNOWN: prediction

Frozen before any implementation code. Base: local `main` at `f3e1d724c`.
Rule placement: amend rule 12n in place (a new id, if one is truly needed, is
`15w`).

## 1. Reproduction (base, measured once with a throwaway probe test)

Over `tests/fixtures/contracts/op_callbacks.qv`, with the unit storing
`handler` (borrows everything, returns nothing) into `op.op_cb`:

| body | row |
| --- | --- |
| `void drop_formal(struct op *o) { c_op_destroy(o); }` | **Unknown**, one leaf: `UnknownAlias { variable: "o" }` |
| `drop_local` (a `c_alloc` result, null-checked, then `c_op_destroy`) | Safe |
| `void drop_arms(struct box *o) { c_op_free_all(o); }` | Safe |
| `void drop_plain(struct op *o) { c_free(o); }` | Safe |

## 2. Where the `UnknownAlias` comes from

Instrumenting `Parser::member_candidates` and the member branch of
`callback_operands` (throwaway `eprintln!`, removed) shows, for
`drop_formal`, three CFG builds: the first two (W4 summary derivation of
`drop_formal`) decide the member over `{handler}` with `handler` classified
**`Missing`** (no contract in the derivation environment yet), so the call
refuses there (`callback-uncontracted`) and `drop_formal` gets no W4 summary;
the third (the reported run) classifies `handler` **`Identity`** and admits the
call. With no summary, the reported run never binds the formal `o` to an
entry resource, and `apply_resource_action` reads the release of an unbound
name as `UnknownAlias`. `drop_plain` and `drop_arms` are Safe because nothing in
their derivation depends on another unit function's summary; `drop_local` is
Safe because a local needs no entry binding.

The ordering gap: `derive_custody_summaries` (legacy_summary.rs) orders the
derivation by `direct_calls` plus `callback_route::callback_edges`, and
`callback_edges` adds edges only for rule 13u's `Invoked` / `Table` clauses,
and only to candidates the body names. A member invocation's candidates are
stores elsewhere in the unit (rule 12n), so no edge reaches them and the
caller may be derived before them.

**Hypothesis: accidental, not a soundness guard.** Nothing reads the formal as
possibly aliased by the closure candidate: the refusal is the
derivation-time `Missing` class of a candidate whose summary simply had not
been derived yet. The alias question the brief raises (could a candidate
alias the formal?) is answered by the candidate's own validated summary, the
same evidence the reported run and the tracked-local case already use: a
candidate that touches what it receives, or anything else, is not the
identity and refuses (`callback-effectful` / `callback-uncontracted`) in every
run.

## 3. The change (narrowing)

`callback_edges` also adds, for a body that calls a declaration with a
`resource_callback_member_invoked` clause, an edge to every function of the
unit's closed member set (`callback_member_closure::closure` over the unit's
facts, as `member_candidates` reads it); an open set adds none (the call
refuses anyway). Ordering only: no decision, class or verdict rule changes,
and the run's fold (rule 12n) is unchanged.

## 4. Predicted outcomes

* Fixture: `drop_formal` Unknown → **Safe** (no leaf); the tracked local,
  arms-only formal and plain-release controls stay Safe; a formal whose
  candidate releases what it receives (the formal itself), or releases a
  global that may alias the formal, stays **non-Safe**; an open member set
  stays `callback-member-open`.
* A same-unit caller of `drop_formal` handing it a tracked local reads it
  through the new summary (Safe), and its `contract_dependencies` name
  `c_op_destroy` transitively (rule 13f), so the run fold still governs it.
* Corpus / system / slice custody and the null property: **byte-identical**
  verdict rows (corpus 935 / 9 / 3,890, system 93 / 4 / 351, slice
  52 / 1 / 347 / 4 Pending; null corpus 1,113 / 1,435 / 2,355 / 18 Unsafe,
  system 84 / 125 / 245, slice 71 / 103 / 232). The ledger entry says no corpus
  row moves (every member invocation refuses before the release); new edges
  arise only in units whose member set is closed, and they only reorder
  derivation. Any movement is read by hand and attributed.
* Lean: a new module (definition of the member edges and the summary-time
  class; theorems that a closed candidate precedes its caller, that once every
  candidate is installed the formal reads as the local, that the formal is
  never Safe where the local is not, and that a non-identity candidate keeps
  the formal non-Safe). Audits 318 → 319.
* Ledger: `RETAINED-RELEASE-OF-A-FORMAL-READ-UNKNOWN` retired (its DISCHARGE:
  rule identified and narrowed, control on a formal, measured). 770 → 769
  unless a residual is registered.
