# CG22-INDIRECT-CALLS-AND-CALLBACKS: candidate callee sets and the callback relation

Card `CG22-INDIRECT-CALLS-AND-CALLBACKS` of the
[Clang gap card queue](../../plans/clang-gap-cards/README.md) (wave 48, round 9;
entries K01 and K02, guarantee-gated). Base local `main` `b6a3dc42` (rounds 1–8
complete); main did not move during the card.

- **Published property and scope.** Custody (resource v1) of a call through a
  function pointer, on route G: such a call is `Safe` only when the analysed
  unit's own Clang AST closes the pointer's candidate set and every candidate's
  contract or validated summary makes the call `Safe`; an open set stays
  `Unknown`. The callback relation (K02) is decided in the core and the parity
  census; no capture vocabulary is published, so no registration is `Safe` by
  this card. Route T is unchanged.
- **Rule:** [gateway 11p](../../agents/gateway.md#constraint-11p).
- **Lean:** `QCore.IndirectCalls`, `proof/AuditIndirectCalls.lean` (21
  theorems), over `QCore.TypedefProjection`.
- **Prediction:** [PREDICTION.md](PREDICTION.md), frozen (commit `4831e1cb`)
  from the census runs before the AFTER binary was measured; the
  implementation existed, no AFTER run did.
- **Absorbed:** LOADER-ABI-INLINE-FNPTR-DECLARATOR (closed); EXTERN-CALL-F
  (function-pointer part) and W47-LOOP-OWNER-FIELD-KEYED-V-AND-F amended.

## 1. What changed

**Candidate sets** (`crates/qcore-c-gateway/src/indirect_calls.rs`,
`resolve`). A call whose callee is not a designator (through parentheses and
the decay, as `elaboration::direct_callee` reads it, or a builtin's
`BuiltinFnToFnPtr`) is resolved from Clang's AST:

| callee | candidate set |
|---|---|
| `f`, `(*f)`, `(&f)`, `(**f)` of a function | `{f}` |
| automatic local, address never taken, with an initializer | union over its initializer and every `local = E` in the body |
| a place of `const` function-pointer type read by `.` and `[ ]` from a named object with an initializer (rule 11h's constant table) | that member or element of the initializer; every element for a non-literal index |
| `c ? a : b`, `c` effect-free | union of the arms |
| a formal; a member or element read through a pointer anywhere on its base chain; a call's result; an explicit cast; a null definition (`0`, an array filler); an address-taken or uninitialised local; a mutable static; a candidate a declaration in the function shadows | **open** |

The flow-insensitive union is a superset of the reaching definitions when the
address is never taken and an initializer exists
(`a_local_holds_one_of_its_definitions`). A call through an indeterminate
pointer or an out-of-bounds table element is undefined; the set need not
cover it (stated premises `P-CG22-DEFINED-POINTER-VALUE` and
`P-CG22-TABLE-INDEX-IN-BOUNDS`, the `AOM-POISON-NO-UB` reading).

**The join, realised by the flow** (`legacy_indirect_calls.rs`). Once per
translation unit, right after rule 10c's normalisation and before the
reserved-spelling escape, every hold and withdrawal, the literal premises, W4's
derivation scope and every classifier position, a closed set is rewritten in
the token stream:

- a singleton's callee tokens become its candidate's name, in any position;
- a larger set's whole statement `S` (`CALL;`, `NAME = CALL;`, `return CALL;`)
  becomes `{ int c₁; …; if (c₁) S[f₁]; else … else S[fₙ]; }` over fresh,
  never-assigned scalars (`qcore_indirect_choice__N`), so the flow explores
  each candidate's arm and joins them: an arm that leaks makes the call
  `Unsafe`, an uncontracted candidate `Unknown`, all-`Safe` arms `Safe`
  (`join_over_covers_sound`);
- a resolved local whose every other reference is a rewritten callee has its
  definitions' values replaced by `0` (never read again; otherwise the lexical
  route reads `cb_t f = q_free;` as an alias of an unknown object `q_free`).

Each candidate is then held, withdrawn, restricted, summarised and classified
exactly as its direct call written in the source; the pass decides no
contract. A site is placed only when its callee's tokens occur on its line as
often as Clang has sites with those tokens there, all with one resolution, and
no conditional directive falls inside the edit. An open set or an unplaced site
changes nothing: the older refusal stands byte for byte. A lexical pre-filter
skips reading the AST for a unit whose bodies call only file-scope names,
keywords and builtins (cost only).

**Callbacks** (K02, `callback_trace`). Registration needs a live context and no
earlier teardown; an invocation needs the registration in force and the
context live; the context may be released only after unregistration; a trace
that ends registered is an escaped capture (`Unknown`).
`invocation_after_release_is_never_safe` and
`registration_after_teardown_is_never_safe` are proved for every trace. Route G
sees neither a registry's invocations nor an unregistration (the contract
language states no capture), so it publishes no registry: a registration
stays the call it is (an uncontracted registrar refuses), an invocation
through a resolved pointer applies its candidate's contract (a released
context is refused exactly as at a direct call: the K02 control), and the
relation classifies every registration in the parity census
(`position: "callback-registration"`).

**The inline declarator** (`legacy_declared_abi.rs`,
`inline_function_pointer_parameter`; absorbs
LOADER-ABI-INLINE-FNPTR-DECLARATOR). The declared-ABI reader reads a parameter
`RET (*NAME)(PARAMETERS)` or `RET (*)(PARAMETERS)` as the typedef reader's
`fnptr`, in exactly its shape; `(**f)`, `(*const f)` and a function returning a
function pointer stay unread (`ABI-FNPTR-DEPTH1`). `QCore.IndirectCalls`'
inline-declarator theorems reduce it to `QCore.TypedefProjection.project
.functionPointer 0`.

## 2. Cited claims, reproduced

| claim (supplied analysis, CG00 controls) | at `b6a3dc42` |
|---|---|
| K01: T admits only a direct designator; `(*f)(a)` is `indirect or unmodeled call` | reproduced (`clang_gap_phase4::k01_…`); route T unchanged |
| K01: G names `fp(p)` an uncontracted callee `fp`, refuses `s->cb()` at the indirect-access gate | reproduced; unchanged after (open sets) |
| K02: `reg(cb, p)` is an uncontracted call | reproduced; unchanged after |
| CG06: 14 value-connective refusals "under an indirect callee" (CG22) | **correction**: 15 distinct statements now; 8 are `__builtin_assume` (a builtin, `BuiltinFnToFnPtr`: CG21's boundary, not an indirect call), 7 genuine (`decode_interface->decode_error`, `rk_conf.connect_cb`), all open sets |
| the elaborator's `indirect call (CG22)` refusal (`FunctionToPointerDecay` anywhere) | **correction**: of its 212 distinct rows (elaboration, nested-call and one composed sibling), 162 are builtins (`__builtin_assume`, `__builtin_alloca`, `__builtin_clz*`, …), 47 hold no callee expression at all: the decay it refuses is a function designator passed as an actual (`rd_kafka_q_serve(…, rd_kafka_poll_cb, …)`, `rd_list_copy_to(…, …_copy_opaque, …)`: a callback registration, K02's shape; sampled, classified lexically), 3 are genuine indirect callees (`prop->set`, `prop->copy`) |
| the condition desugar's `IndirectCallee` (CG03-RETURN-OPERAND-RESIDUALS' and W43-RETURN-COMPOUND-CALL's "indirect callee (CG22)") | **correction**: of 40 distinct consultations, 19 are a cast's `) (` (`(int)(…)` in the `isspace`/`isprint` expansions, `(size_t)(…)`), not a callee; 21 genuine, all members through a pointer (nanopb's `stream->callback`, `rd_avl`'s `ravl_cmp`, SASL providers) |
| round 8: CG06's composable siblings refuse on the `rd_kafka_log0` "indirect gate" | the lexical indirect-*access* gate (`&rk->rk_conf` places, CG10 / CG05-CALL-ROOTED-PLACES), not an indirect call; re-measured in §4 |
| LOADER-ABI-INLINE-FNPTR-DECLARATOR: `rd_kafka_topic_partition_list_sort` withdrawn (`abi-unreadable-declaration`) | reproduced: 11 corpus leaves over 7 functions |

## 3. Census

Census-only build (`ADMIT = false`), byte-identical to BEFORE on all three
routes; distinct sites (function, line, callee tokens),
`artifacts/distinct-{corpus,system,slice}.json`, `artifacts/census-corpus.json`:

| | corpus | system | slice |
|---|---:|---:|---:|
| calls the elaborator's reading would call "not direct" | 5,568 | 2,892 | 3,109 |
| of which builtins | 4,996 | 2,460 | 2,524 |
| genuine indirect call sites | 572 | 432 | 585 |
| in a reported function | 235 (185 functions) | 12 (7) | 20 (12) |
| closed candidate set | **0** | **0** | **0** |

Corpus shapes: a member read through a pointer 220 (and 66 `p->a.f` chains the
census build counted as `.` members, through a pointer under the final
reader), a local defined by an explicit cast 172 (xxHash's `*_conv`), a formal
102, a mutable static's member or element 6, a local without an initializer 2.
System and slice: members through a pointer (362 / 513) and formals (70 / 72).
No single-name callee shares a contracted name.

Callback registrations: corpus 676 distinct (584 in reported functions), system
131, slice 157. Declared registrars: `msgpack_packer_init` ×2
(`flb_input_chunk_create`, `flb_input_chunk_map`: the packer captures `data`
behind a `Borrowed` formal) and `rd_kafka_topic_partition_list_sort` ×3
(synchronous comparator, nothing captured). The relation finds a named context
released later in the same body (no unregistration is visible) in 5 corpus, 1
system and 1 slice registrations, `flb_input_chunk_map`'s
`msgpack_packer_init` among them: a census class, no verdict
(`CG22-CALLBACK-REGISTRY-CONTRACT`).

## 4. Before and after

The arbiter's own command (`scripts/measure.sh`; corpus `66910c10`, `--jobs
4`), BEFORE on main `b6a3dc42`. Main then moved three times (CG17, CG20: both
byte-identical on all three routes by their own reports; CG25); the AFTER of
record is the tip rebased on main `06ad81cb`, whose base reports equal
BEFORE's, so the pair compares only this card.

| run | binary | corpus | system | slice |
|---|---|---|---|---|
| BEFORE, main `b6a3dc42` (`private/cg22/before`) | `1d06a383…` | 629 / 10 / 4,195 `3d53a5d0…` | 44 / 2 / 188 `d7aa010b…` | 29 / 1 / 239 / 1 P `0cdbef8b…` |
| census build (`ADMIT = false`) | `73ef3965…` | byte-identical | byte-identical | byte-identical |
| **AFTER, tip on `06ad81cb`** (`after6`) | `c58df810…` | **631 / 10 / 4,193** `eb1b0919…` | **byte-identical** | **byte-identical** |
| AFTER, tip on `b934c560` (`after7`) | AFTER7_BINARY | AFTER7_CORPUS | AFTER7_SYSTEM | AFTER7_SLICE |

System and slice reports embed the worktree path (8 strings), so their digests
are this worktree's; the totals equal CG05's `after3` row and the BEFORE/AFTER
pair is byte-identical within this worktree. Counts are compare.py's,
de-duplicated by (unit, function).

**Every move** (`artifacts/corpus-comparison.json`, `artifacts/corpus-changed.txt`),
all from the inline declarator; the candidate-set rewrite changed no token on
any route (572 corpus sites, every rewrite decision `none: open set`, the
same sites as the census):

- **Unknown → Safe** (2): `rd_kafka_topic_partition_list_sort_by_topic` and
  `rd_kafka_topic_partition_list_sort_by_topic_id` (rdkafka_partition.c),
  each losing its only leaf, the withdrawal of
  `rd_kafka_topic_partition_list_sort`. Predicted exactly.
- **Same status, leaves changed** (6): the 11 withdrawal leaves leave
  `rd_kafka_assignment_add` (−2), `_subtract` (−1),
  `rd_kafka_share_assignment_add` (−2), `_subtract` (−1) and `sortPartitions`
  (−3); `rd_kafka_topic_partition_list_has_duplicates` loses `call to
  uncontracted function rd_kafka_topic_partition_list_sort_by_topic` (the
  wrapper's validated summary) and one unresolved-identity leaf. Read: two
  unresolved-identity leaves appear (`rd_kafka_assignment_add`,
  `rd_kafka_share_assignment_add`: a later read of `partitions` the
  withdrawn call's refusal had poisoned first) and two leave
  (`rd_kafka_share_assignment_subtract`); all four functions stay Unknown on
  other leaves.
- No Safe → non-Safe move, no correction, no leak or violation added.
- Re-sized classes: the `rd_kafka_log0` value-connective statements are
  1,512 at BEFORE and AFTER (1,100 on an edge an older route refuses, 276 no
  unique Clang node, 109 a sibling composition, 20 an undecided operand, 2
  over the leaf cap; `scripts/log0.py`): unchanged, an indirect *access*
  class, not an indirect call.

**Prediction against outcome.** Every item held: no rewrite on any route; the
11 withdrawal leaves left; the two wrappers moved to Safe (the prediction's
upper bound); the other five stayed Unknown; system and slice byte-identical.
The prediction did not foresee the leaf exchange inside the four
`rdkafka_assignment.c` functions (+2 / −2 unresolved-identity leaves).

**Discarded attempts** (retained in `private/cg22/`, not evidence): `after`
(corpus killed by `No space left on device` in the parity sink when the shared
host disk filled; its system run completed byte-identical before the fill),
`after2`–`after5` (stopped at the disk floor or superseded by a rebase before
they ran).

**Cost.** The pass reads Clang's AST for every unit whose bodies call
something other than a file-scope name; the census build (no pre-filter)
took 92 minutes against BEFORE's 75 under a shared host; the AFTER corpus
took 9 minutes on an idle host. Wall time here measures the host, not the
pass; no per-unit timing was taken.

## 5. Follow-ups

Ledger 670 → 672 (`scripts/update_ledger.py`, applied to main's matrix):

- **Retired:** `CG22-INDIRECT-CALLS-AND-CALLBACKS` (every DISCHARGE clause
  holds: BEFORE reproduced, corrections recorded, PREDICTION frozen, AFTER
  attributed, absorbed entries closed or amended, Rust controls and audited
  Lean) and `LOADER-ABI-INLINE-FNPTR-DECLARATOR` (its discharge condition).
- **Amended:** `EXTERN-CALL-F` (the C gateway's function-pointer part is rule
  11p; the extern-call route's stays carried), `W47-LOOP-OWNER-FIELD-KEYED-V-AND-F`
  (its vtable dispatch is exactly the open member-through-pointer class: 220 /
  362 / 513 sites; instance sensitivity still needs configuration flow),
  `CG06-VALUE-CONNECTIVE-RESIDUALS` (the "indirect callee" class: 8 builtins,
  7 open; the `rd_kafka_log0` class is indirect access, unchanged),
  `CG03-RETURN-OPERAND-RESIDUALS` and `W43-RETURN-COMPOUND-CALL` (the
  desugar's `IndirectCallee`: 19 casts, 21 open).
- **Registered:** `CG22-OPEN-INDIRECT-CALLS` (572 / 432 / 585 open sites),
  `CG22-CALLBACK-REGISTRY-CONTRACT` (676 / 131 / 157 registrations; no capture
  vocabulary), `CG22-INLINE-FNPTR-LOCAL-DECLARATION` (1 corpus leaf),
  `CG22-DESIGNATOR-ACTUAL-ELABORATION` (the elaborator's 212-row label: 162
  builtins, 47 designator actuals, 3 genuine).

## 6. Gate

GATE

## Files

- `scripts/measure.sh` (BEFORE/AFTER, the arbiter's exact command),
  `scripts/census.sh` (the same commands with an explicit binary),
  `scripts/build_census.sh`, `scripts/census.py`, `scripts/distinct.py`,
  `scripts/keyword_callees.py`, `scripts/compare.py`, `scripts/gate.sh`,
  `scripts/update_ledger.py`.
- `artifacts/`: the census and comparison records.
