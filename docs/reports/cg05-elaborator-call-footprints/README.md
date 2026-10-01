# CG05-ELABORATOR-CALL-FOOTPRINTS: the elaborator's call events with their contract footprints

Follow-ups `CG05-ELABORATOR-CALL-FOOTPRINTS` and
`CG05-FOOTPRINT-ADDRESS-CONSERVATISM` of the
[Clang gap card queue](../../plans/clang-gap-cards/README.md) (wave 48, round 8,
the composition batch). Base `d4df096f` (local `main`, round 7 complete), rebased
onto main `0a22a998` (the MSVC C ABI reader) and then onto `cfb5d8e1` (CG06's
sibling-effect part, rule 11m), and re-measured on each.

- **Rule:** [gateway 11n](../../agents/gateway.md#constraint-11n), amending rule
  10s's call refusal; rules 10y and 11k unchanged.
- **Lean:** `QCore.ElaboratorCallFootprints`, `proof/AuditElaboratorCallFootprints.lean`
  (15 theorems), over `QCore.NestedCallEffects`.
- **Prediction:** frozen in [PREDICTION.md](PREDICTION.md) (commit `7b1b7ffd`
  after the rebases, `d5a52c96` before them), before the implementation commit and
  before any test or AFTER run.

## 1. What changed

**Footprints on the elaborator's call events.** `Context::call_footprint`
(default: none) lets a route give each call event its contract footprint. Route
G's statement route reads it with `nested_call_effects::call_footprint`, the same
reading rule 10y applies to a call of its tree (contract in force, `Borrowed`
identities read, every other custody formal written, `&x` and array decays a
write of the object, an owned result a fresh identity keyed to the event). Every
other event gets `event_footprint`'s cells: a named object's storage, plus an
identity for a pointer value; a path's root object, plus an identity through a
pointer or for a pointer value; a lifetime event writes its object; control
events touch nothing.

**The order check.** `elaborate` asks the new
`EvalGraph::check_with_footprints`: a pair `commutes` leaves as a call pair
commutes only when both events carry a footprint and `footprints_commute`
holds; every other pair is CG18's `check_with`, in the same order. With no call
footprint the map is empty and the check *is* `check_with`
(`an_empty_map_is_check_with`, every pair of five event kinds). `check`,
`check_with` and `check_footprints` are unchanged.

**The lowering** (`legacy_elaborator_calls.rs`). A refused piece whose graph
holds calls is lowered in event order: reads and writes as rule 10s lowers them,
each call through `Parser::instantiate_call` with use `Tested` (a consumed value
owns nothing), scalar actuals at `Borrowed` formals as data, pointer actuals by
the classifier's substitution. The contract applied must equal the one the
footprint was read from. A call with no footprint, or with effectful actuals
(CG03's), keeps its refusal.

**Two supporting corrections.**

- `aliasing` (CG02's, shared with rule 10y) now counts `&s.m`, `&a[i]` and an
  array decay (other than a subscript base) as taking the root object's
  address. Before, `f(buf)` left an array `buf` "never address-taken", which
  breaks `overlap_sound`'s `unaliasedIdentity` premise for `Cell::Object(buf)`.
  Only an aggregate's set changes; no nested admission was withdrawn (18 → 18).
- The node match strips callee parentheses as rule 10c's token normalisation
  does, when no exact match exists: `(void)(q_free)(p);` and `(void)
  q_free(p);` are decided alike.

**Shared items added (for the integrator).** `elaboration.rs`:
`Context::call_footprint`, `event_footprint`, `EvalGraph::check_with_footprints`,
`Refusal::CallLowering`, `Elaboration::{call_footprints, call_nodes}` (the only
construction site is `elaborate`), the `Elaborator` fields `call_footprints`,
`call_nodes`, `scalar_reads`. `nested_call_effects.rs`: `call_footprint`.
`legacy_parenthesised_callee.rs`: `strip_callee_parentheses`.
`legacy_elaboration.rs`: `GContext::calls`, `elaboration_context_with_calls`,
`aliasing` (body corrected), the stripped fallback in `elaboration_node`, and
two hook lines in `lower_step`. `legacy_c.rs`: the module declaration. No
existing variant or function changes meaning. `value_connective.rs` and
`legacy_value_connective.rs` (CG06's) are untouched: `leaf_events` keeps the
plain context.

## 2. The cited claims

| Claim | Reproduced? |
|---|---|
| 98 elaborator consultations over 33 functions refused as a call unsequenced with another event | **No, at this base: 621 rows over 411 functions** carry that refusal (619 statements, 2 returns). The compound family the ledger describes is 118 / 39 (it grew); the other rows are stores of calls (276), call statements (170), declarations (52) and 5 others that later cards (CG08 consults every statement holding a cast) route to the elaborator. Correction to the input, recorded. |
| `len += rd_kafka_AclBinding_request_size(acl, v)`, `not_in_group \|= rd_kafka_cgrp_leave_maybe(rkcg)`, `score += abs(...)` | **Yes**, each present, and each still refused: none of the three calls has a footprint. `rd_kafka_AclBinding_request_size` and `rd_kafka_cgrp_leave_maybe` carry their own `call to uncontracted function` leaf; `abs`'s line carries an indirect-access leaf (`sizes[next]`). |
| 25 address-conservatism consultations, 24 printf-family in librdkafka unit tests | **Yes, exactly:** 25 over 14 functions; `rd_kafka_msgq_len` 12, `rd_list_cnt` 7, `rd_buf_len` 3, `rd_kafka_msgq_size` 2, `localtime_r` 1. |

## 3. Measurements

`scripts/measure.sh` (corpus `66910c10` `--jobs 4`, system, tail-pause slice);
`scripts/archive.sh`; [artifacts/](artifacts/).

| Run | Binary | Corpus | System | Slice |
|---|---|---|---|---|
| BEFORE, base `d4df096f` (`private/cg05ef/before`) | `8bed2ee9…` | 628 / 10 / 4,196 `3b09309a…` | 44 / 2 / 188 `3e88dd81…` | 29 / 1 / 239 / 1 P `8fbf8ffe…` |
| BEFORE, main `0a22a998` (`before2`) | `3d88c503…` | **byte-identical** | byte-identical | byte-identical |
| AFTER, tip on `0a22a998` (`after2`) | `ead22004…` | 629 / 10 / 4,195 `36941e4c…` | 44 / 2 / 188 `79404fed…` | 29 / 1 / 239 / 1 P `93cc660c…` |
| BEFORE, main `cfb5d8e1` (`before3`) | `783dec5e…` | 628 / 10 / 4,196 `f9b3420e…` | `3e88dd81…` (same) | `8fbf8ffe…` (same) |
| **AFTER, tip on `cfb5d8e1` (`after3`)** | `8acb3541…` | **629 / 10 / 4,195** `3d53a5d0…` | 44 / 2 / 188 `79404fed…` | 29 / 1 / 239 / 1 P `93cc660c…` |

The published figures are the last pair. The AFTER at the pre-rebase tip
`2c8de046` (`after1`) gave `after2`'s corpus file byte for byte; CG06's
landing changes the corpus file (`3b09309a…` → `f9b3420e…`) but nothing this
card decides: the changed functions, leaf deltas, admitted rows and
populations are identical on all three mains. The artifacts are from
`before3`/`after3`. All exits 0. Sidecars (`*.contracts.json`,
`*.chunk-contracts.json`, `*.source-effects.json`, `monkey-source.json`)
byte-identical.

**The move.** `rdkafka_request.c::rd_kafkap_leader_discovery_tmpabuf_add_alloc_topic`,
Unknown → Safe: `(tbuf)->size += ((strlen(topic_name) + 1) + 7) & ~7` (the
`rd_tmpabuf_add_alloc` macro). The read of `tbuf->size` (its root object and an
identity) and `strlen` (borrows `topic_name`'s identity) only read; the write of
`tbuf->size` follows the call. It was the function's one leaf.

**Changed diagnostics** ([corpus-changed.json](artifacts/corpus-changed.json)):
10 corpus functions, unsupported −12 (indirect access −10, formal-embedded −2),
`unknown_alias` +12. 18 elaborator rows newly admitted over 9 functions
([newly-admitted.txt](artifacts/newly-admitted.txt)): stores of a contracted
scalar call into a path (`md->brokers[i].id = (int32_t) __bswap_32(_v)` ×6,
`…->as_int/as_double = … cmt_metric_get_uint64_value(sample)` ×4, `conf->created
= time(NULL)` ×3, `local_upper_bounds[i] = pow(…)` ×2), two `memcpy` statements
and the move. The `unknown_alias` leaves are those stores reaching the flow
with a base the flow does not bind; none is a leak or a violation. No row lost
its admission; 20 refused rows changed their text (a later call of the
statement is now the first refused, or the application point declines an owned
`rd_malloc`/`rd_calloc`/`calloc` result in a consumed value, 9).

**System and slice:** no status move; `flb_sched_request_create`,
`flb_sched_retry_now` (system) and `flb_tail_file_rotated`,
`flb_tail_fs_stat_add` (slice) lose an indirect-access leaf at `x->m =
time(NULL)`.

**Populations after** ([census-after3.json](artifacts/census-after3.json)):
603 rows over 400 functions keep `call … unsequenced with another event`, each
a call with no footprint by construction; 515 have the callee's own `call to
uncontracted function` leaf at their line. No corpus elaborator row is refused
as `contract footprints do not commute`. The address population is 25 / 14,
unchanged.

**No correction:** no Safe → non-Safe, no Unknown → Unsafe, no leak or violation
added.

## 4. Prediction against outcome

| Prediction | Outcome |
|---|---|
| BEFORE reproduces the baseline | **Held** (and on `0a22a998`, byte for byte). |
| 0 Safe moves, upper bound 3 | **1 move, inside the bound, but the reading was wrong:** I expected `(tbuf)->size += (strlen(…) …)` to stay refused as a destination read through a pointer unordered with a call; both events only read, and two reads commute. |
| No Safe → non-Safe, no Unknown → Unsafe | **Held.** |
| At most 132 rows change; ≤ 423 functions change diagnostics | **Held:** 18 admitted, 20 refusals rephrased; 10 functions. |
| Address population stays 25 / 14 | **Held.** |
| `aliasing` correction moves no function | **Held:** no nested admission withdrawn (18 → 18). |
| System and slice move no status; sidecars identical | **Held** (2 functions each change leaves). |

## 5. The address boundary (option (c))

`fprintf(stderr, …, rd_kafka_msgq_len(&q))` stays refused. Option (b) is not
sound from the flow: `resource_borrowed` "is not the full QCore
lifetime-checked borrow type" (contract spec §C04.2), so a callee given `&q`
may retain it, and nothing proves a file-scope pointer never receives an
automatic's address. Option (a) would be a new program premise named in every
using verdict, and no function would move with it (each of the 14 has other
blocking leaves). Lean states why the refusal is needed without a premise:
`aliased_den_meets_the_premises` builds a denotation meeting `overlap_sound`'s
premises in which the identity *is* the local's storage, and
`the_address_boundary_is_necessary` shows the pair is refused and the two
orders observe different values there.

## 6. Tests and flips

- **New:** `tests/cg05_elaborator_call_footprints.rs` (5 tests): `x +=
  r_release_code(p)` Safe with the release applied once (a second release is
  caught, a borrow leaks), every compound operator; `*q += r_release_code(q)`
  refused; an uncontracted `x += opaque()` and an undeclared callee refused; an
  address-taken destination refused; an owned result in a consumed value and
  two releases of possibly-aliased names refused.
  `tests/unit/elaborator_call_footprints.rs` (3): the Lean vectors bound to
  `check_with_footprints` and `event_footprint` both ways; the empty-map
  equivalence; Clang-read footprints admitting `x += r_release_code(p)` and
  refusing `*q += r_release_code(q)` with `CallFootprintConflict`.
- **Flipped deliberately:**
  - `indirect_scalar_access::a_call_in_the_statement_stays_refused` →
    `a_call_in_the_statement_is_applied_through_its_footprint`: `p[0] = (char)
    x_len(s)` is Safe (`x_len` borrows `s`; the store follows the call), and so
    is `p[0] = (char) x_len(p)`.
  - `parenthesised_callee::nested_parentheses_and_a_cast_to_void_are_normalised`:
    `(void) q_free(p)` and `(void)(q_free)(p)` were both refused as `resource
    call with a formal embedded in an unmodeled expression`; both are now Safe,
    and still decided alike.
- CG02's and CG05's own refusal pins (`cg02_unordered_calls_are_refused_for_cg05`,
  the `ORDER_VECTORS`) use contexts with no call footprint and stand unchanged.

## 7. Lean

`QCore.ElaboratorCallFootprints`, 15 theorems, no placeholders, no project
axioms: `a_footprintless_call_is_refused`, `empty_map_is_check_with` (against
`QCore.ElaborationCore.call_never_commutes`), `callPair_symm`,
`a_scalar_destination_read_commutes_with_a_call`, `a_disjoint_read_observes_alike`,
`the_compound_assignment_runs_alike`,
`a_read_through_the_written_identity_is_refused`,
`an_address_taken_destination_is_refused`, `two_identity_writes_are_refused`,
`aliased_den_sound`, `aliased_den_meets_the_premises`,
`a_local_write_respects_its_footprint`, `the_address_boundary_is_necessary`,
`footprint_vectors_hold`, `footprint_vectors_count`. A new module rather than an
edit of `QCore.ElaborationCore` / `QCore.NestedCallEffects`, so the concurrent
CG06 card's Lean is not touched. Premises stated, not proved: faithful Clang
extraction and node matching (trust boundary 28); a call's effect on the
abstract state is its contract's (constraint 4).

## 8. Follow-ups

Ledger 667 → 666 ([scripts/update_ledger.py](scripts/update_ledger.py)).
Retired `CG05-ELABORATOR-CALL-FOOTPRINTS` (every DISCHARGE clause holds) and
`CG05-FOOTPRINT-ADDRESS-CONSERVATISM` (option (c), measured). Registered
`CG05-ELABORATOR-FOOTPRINTLESS-CALLS` (603 / 400; per-callee contracts or
summaries; returns and conditions are not lowered by this route).

## 9. Gate

At tip `1cdf20a4` on main `cfb5d8e1`, with [scripts/gate.sh](scripts/gate.sh);
exits in [gate/](gate/), record [gate.json](gate.json), logs
`private/cg05ef/gate2/`.

| Step | Command | Exit | Count |
|---|---|---|---|
| submodule | submodule update, `--init --recursive` | 0 | |
| tests | `python3 scripts/run_test_binaries.py` | 0 | 410 units, 423 summaries, **7,171 passed**, 0 failed, 11 ignored (`private/test-binaries/20261001T121514Z-1042180`) |
| fmt | `cargo fmt --all -- --check` | 0 | |
| clippy | `cargo clippy --all-targets -- -D warnings` | 0 | |
| probe | `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | |
| follow-up probe | `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | |
| Lean | `python3 scripts/check_lean_audits.py` | 0 | **263 audits** (262 on main + `AuditElaboratorCallFootprints.lean`) |
| Python | `python3 -m unittest discover -s scripts/tests` | 1, then **0** | 365; the one failure was the matrix's derived `lean_audits` (262 → 263), cleared by `build_matrix.py . --gate gate.json` (exit 0) |
| docs index | `python3 scripts/build_docs_index.py` | 0 | |

`cargo check --workspace --all-targets` exited 0 on each rebased tree. An
earlier gate at `4674423f` on `0a22a998` (`private/cg05ef/gate`) failed clippy
on two findings in this card's code, fixed in the next commit; its tests
passed 7,162 / 0. `providers/` is unchanged, so the provider gate is not due.
Clang 21.1.8.
