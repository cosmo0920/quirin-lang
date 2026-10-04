# QUANT-INDEX-CHECKS

Wave 48, round 19. The user's request (2026-10-04): a `quant` constraint over an
index, `quant Idx #i …`, with `==`, `<`, `>`, `>=`, `<=` comparisons, in the
contract language. Gateway rule [12l](../../agents/gateway.md#constraint-12l);
contract specification
[C09.2](../../QCORE_CONTRACT_SPEC_r0_3.md#c092-quantified-index-checks-quant-checks);
grammar [G05c](../../QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05c-quantified-index-checks).
Lean: `QCore.IndexQuant` ([`proof/QCore/IndexQuant.lean`](../../../proof/QCore/IndexQuant.lean),
audited in `proof/AuditIndexQuant.lean`, 36 theorems), extending
`QCore.IndexExtent`. Base: local main `0ebe77f86`. Prediction frozen in
[PREDICTION.md](PREDICTION.md) (`b43bc226b`) before implementation code.

**Published property and scope.** CG16's `c.bounds.index-extent.v1` (rule 11s),
with one more declared fact a person can state about a pointer formal: which
indices the function uses through it. It is bounds vocabulary (`requires
bounds_contract_v1;`), not resource v1, which rejects it. Custody is unchanged.

## 1. Design

The user's two decisions, as asked and answered:

1. **Universal over a range.** `quant checks map, Idx #i: 0 <= #i && #i < 24;`
   says every index the function reads or writes through `map` satisfies the
   predicate. Ranges are written with `&&`; `||` is admitted and read
   literally, so the user's own spelling `#i < u || l < #i` (with `l < u`)
   admits every index and is rejected as unbounded rather than reinterpreted.
2. **A collocated check, not an `ensures`.** The clause has its own
   introducer, `quant checks`, names the formal it constrains as its subject
   (one clause per formal, like `bounds_extent(P, …)`), and carries no exit
   selector; `quant normal …` (C03) keeps its structural meaning. The
   parameter-collocated `[| quant Idx #i: … |]`, overloading `quant normal`, and
   a `bounds_index_where(P, …)` predicate call were rejected; C09.2 records why
   (in short: a predicate inside the parameter type text would break the
   parameter splitter's angle-bracket depth and every declaration reader that
   reconstructs signatures, and G05b's predicate calls have no expression
   grammar).

Comparisons are over the mathematical integers; the index counts elements of
the C pointee type; bound terms are canonical literals, a scalar formal's entry
value, `+`/`-` of those (one formal, coefficient one); everything else is
refused by a named `quant-check-…` diagnostic and the whole module with it
(G05c lists them).

## 2. What changed

| Layer | Item | Owns |
|---|---|---|
| frontend | `SourceContractClauseKind::QuantCheck` (shared item) | the structural parser reads `quant checks …;` as its own clause kind (a parse error before this card); `contract_inclusion` and `contract_views` render it, resource v1 rejects it |
| core | [`qcore_core::index_quant`](../../../crates/qcore-core/src/index_quant.rs) (new) | `QuantPredicate`, `BoundTerm`, `Relation`; the exact interval `reading`; `admitted` (empty, unbounded, bounded merged union); `syntactically_bounded`; `symbolic_prefix`; `index_admitted` (inside, outside, undecided); `shifted_within`; `both_in_bounds` |
| contract | [`qcore_contract::bounds_quant`](../../../crates/qcore-contract/src/bounds_quant.rs) (new), hook in `bounds_contract.rs` | the total-or-reject reader and its diagnostics; `BoundsClause::QuantCheck` (shared item), digested with the declaration; a pointer states an extent or a quant check |
| gateway | [`index_extent/quant_checks.rs`](../../../crates/qcore-c-gateway/src/index_extent/quant_checks.rs) (new), thin hooks in `index_extent.rs` | seeding the checked formal, the body-side decision before the extent decision, the lower-bound-only downgrade, the caller obligation (also before a callee's return facts are used), the escape check, phase-one refutation |
| example | [`quant_index_probe`](../../../crates/qcore-c-gateway/examples/quant_index_probe.rs) | the API route for the chunkio probe |
| Lean | `QCore.IndexQuant` (new) | the predicate language, its exact reading, the admitted set, the body/call/escape decisions, 36 audited theorems, three shared-vector tables |

No custody code changed: no hook in `legacy_c.rs`, `typed_source.rs` or any
custody route, and the audit routes still run the bounds consumer with an empty
bounds environment (`CG16-BOUNDS-CONTRACT-TRANSPORT`).
`BOUNDS_MODEL_VERSION` is unchanged: every decision for an existing declaration
is unchanged (a function with no quant check takes none of the new paths), and
a declaration carrying a quant check has a new text digest, so recorded
evidence cannot be reused across the change.

## 3. Semantics, fail-closed

* **Body.** The formal enters as a *quant object* with the extent its callers
  must cover. An element access, dereference or `->` through it (directly,
  through a copy, or through `P + k`) is decided against the admitted set
  first: inside, the ordinary extent decision follows (held declarations make
  it Pending); outside, the row keeps `bounds-contract-refuted-by-body`, the
  access records `bounds-quant-check-refuted`, and the declaration is withdrawn
  from the unit's callers; undecided, `bounds-quant-check-undecided`. Never in
  bounds on an undecided index.
* **Lower bound only.** The check says which indices are used, not how large
  the object is, so out of bounds against the seeded extent (a formation
  beyond it, a callee requiring more) is `bounds-range-not-contained`: a quant
  check never accuses.
* **Callers.** The element accesses at the least and greatest admitted index
  under the actuals' values; the prefix `[0, f)` uses rule 11s's symbol cover.
  A violation is `Unsafe` only when definitely reached under in-force premises
  (rule 11s(6)); an empty set under the actuals is
  `bounds-inconsistent-premises`; any other unread case Unknown.
* **Escape.** A quant object passed to a contracted callee is decided inside
  only when the callee's constant check, shifted by the constant offset, lies
  inside the caller's set; otherwise `bounds-quant-check-escapes`, never a
  refutation (an uncontracted callee was already Unknown).
* **No fact.** Any symbolic form other than `[0, f)`:
  `bounds-quant-check-no-fact`, the formal carries no extent, its accesses are
  `bounds-missing-extent`.

## 4. Prediction against outcome

Binaries: BEFORE `1e028e2f…` built at `0ebe77f86` (checked out for the build),
AFTER `a9c41374…` built at `f04d18fc4` (the final code), each copied to
`private/qic/bin/` and checked against every run's `run.json` `binary_sha256`.
Scripts: [`scripts/`](scripts/) (`measure-binary.sh`, `measure-bounds.sh`,
`identity.py`); digests and the identity table in [`artifacts/`](artifacts/).

| Route | BEFORE | AFTER | Prediction |
|---|---|---|---|
| Corpus arbiter (custody) | 756 / 9 / 4,069, `ce57d194…` | identical, `ce57d194…` | identical: held |
| System route | 85 / 4 / 359, `878b06d4…` | identical | identical: held |
| Tail-pause slice | 45 / 4 Pending / 354 / 1, `0fcc81a6…` | identical | identical: held |
| Typed-path parity (corpus / system / slice) | 669,570 / 141,186 / 50,426 rows | equal multisets | identical: held |
| Corpus with `QCORE_BOUNDS_PROPERTY` | bounds rows `822a7633…`: 162 Safe / 0 Pending / 0 Unsafe / 4,759 Unknown | byte-identical rows and custody report | identical: held |

Every compared file is identical after erasing the run's own directory name
and the system routes' preparation durations (`artifacts/identity-routes.json`,
20 files). No function moved on any route; there is nothing to attribute.

The bounds tally on this main matches the STATUS figures measured on
`b6a3dc42` (162 / 0 / 0 / 4,759); the combined row is now **111 Safe / 4,801
Unknown / 9 Unsafe** (it was 111 / 4,800 / 10), following custody's own move
from 10 to 9 provisional Unsafe since then, not this card
(`SUP03-BOUNDS-TALLY-ON-INTEGRATED-MAIN` owns that re-measurement).

**The bounds property where it can be selected today.** No audit route loads a
bounds module, so the only route is the API. The probe
([`probe/`](probe/), `quant_index_probe` example) analyses chunkio's header
functions from the pinned corpus source (read only, through the include path)
under two **test-only** declarations, review-held as written (they are not
provider contracts and nobody reviewed them), and again with the markers
removed. `probe/run.sh` writes the declarations into `private/qic/` rather than
tracking a `.qv` file: CG16's census
`the_reserved_spellings_are_unused_by_tracked_contracts` keeps every tracked
contract module free of bounds vocabulary, and the first gate (g1) failed on
exactly that when the probe was a tracked `chunkio-held.qv` (section 6).

| Function | Held | Resolved |
|---|---|---|
| `cio_file_st_get_meta_len` (`map[22]`, `map[23]`; check `0 <= #i && #i < 24`) | Pending | Safe |
| `cio_file_st_set_meta_len` (writes `map[22]`, `map[23]`; check `22 <= #i && #i <= 23`) | Pending | Safe |
| `cio_file_st_get_hash` (`map + 2`, no declaration) | Unknown, missing extent | Unknown |
| `cio_file_st_get_content` (passes its own `map`, no extent) | Unknown | Unknown, the callee's obligation unestablished |
| `probe_full_header` (written for the card, `char header[24]`) | Pending | Safe |
| `probe_short_header` (written for the card, `char header[16]`) | Unknown, `bounds-held-premise-violation` | Unsafe (a provisional violation of a test-only declaration in the card's own fixture, not an upstream finding) |

The prediction's controls all held as predicted (the Rust tests
`crates/qcore-c-gateway/tests/quant_index_checks.rs`, 12;
`crates/qcore-contract/tests/bounds_quant.rs`, 11;
`crates/qcore-core/tests/index_quant.rs`, 7). One reading was refined while
writing them, not a prediction miss: a checked formal passed on to a callee
without a constant check is an undecided escape, never a refutation (the
callee need not use every index it admits).

## 5. Follow-ups

* `QUANT-INDEX-CHECKS` itself was never registered in `named_followups`; it
  lands with its scope complete, so its retirement is a no-op, described as
  such.
* Amended: `CG16-BOUNDS-CONTRACT-TRANSPORT` (quant checks travel with bounds
  modules; the chunkio declarations it would enable, measured here through the
  API) and `CG16-BOUNDS-SYMBOLIC-RANGES` (symbolic quant forms beyond `[0, f)`
  and escapes through non-constant callee checks). Follow-ups stay **705**.

## 6. Gate

Standing gate `g2` (`scripts/gate.sh`, every step under the shared lock, logs
`private/qic/gate-g2/`, exits copied to [`gate/`](gate/), summary in
[`gate.json`](gate.json)), every step at `5ef8ef181`:

| Step | Command | Exit | Count |
|---|---|---|---|
| 00 | `cargo check --workspace --all-targets` | 0 | |
| 01 | submodule update `--init --recursive` | 0 | |
| 02 | `cargo fmt --all -- --check` | 0 | |
| 03 | `cargo clippy --locked --all-targets -- -D warnings` | 0 | |
| 04 | `python3 scripts/run_test_binaries.py` | 0 | 444 units, 7,595 passed, 0 failed, 11 ignored (`private/qic/tests-g2/`) |
| 05 | `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | |
| 06 | `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | |
| 07 | `python3 scripts/check_lean_audits.py` | 0 | 288 audits (287 → 288) |
| 08 | `python3 -m unittest discover -s scripts/tests` | 1 | 365 run, 1 failure: the committed matrix still carried `lean_audits` 287 |
| 09a | `build_matrix.py . --gate gate.json` | 0 | matrix regenerated (288 audits, `AuditIndexQuant.lean`) |
| 09b | `python3 -m unittest discover -s scripts/tests` | 0 | 365 OK |
| 09, 10b, 10c | `build_docs_index.py`, `--check` | 0 | |

The first gate (`g1`, at `8c87bd664`) failed step 04 with 7,594 passed and 1
failed, the CG16 census `the_reserved_spellings_are_unused_by_tracked_contracts`
finding the probe's tracked `chunkio-held.qv`; the declarations moved into
`probe/run.sh` (`5ef8ef181`) and that gate was stopped. `providers/` is
unchanged, so the provider gate was not run. Measurement binaries were built in
`private/qic/target-rel`; `cargo clean --profile dev` ran after the tests.
