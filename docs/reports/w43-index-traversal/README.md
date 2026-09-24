# W43-INDEX-TRAVERSAL: index and pointer traversal

Card W43-INDEX-TRAVERSAL, 2026-09-25, on base `0a8f59ac`: local `main`
`ec70709a` with the finished W43-INDIRECT-ACCESS-GENERAL card (`6bb838c8`)
merged, whose typed scalar-access rule it builds on. The forecast was frozen
before the admission was committed or measured ([PREDICTION.md](PREDICTION.md),
commit `1ea2f7f0`); the admission is commit `44d7eb5f`.

Code that walks a buffer or a string by index (`s[i]`, `i++`) or by pointer
increment (`p++`, `p += k`, `*p++ = c`) used to block every verdict it
touched. This card makes such walks decidable **for the resource property
only** — custody, use after release and invalid release. **No bounds or
spatial-safety claim is made.** The verdict's scope string and spec L18 say so.

## Headline

* **Rules** ([§2](#2-the-rules)): a refused controlling expression whose every
  access is typed lowers to a typed read with each base's dereference
  obligation; `p++`, `p += e`, `p = q ± e` lower to a pointer step that is
  *derived* from the source's binding with an interior extent; `*p++ = c`,
  `c = *p++` and `buf[i++] = c` lower to the access over the old values followed
  by the step; a declared scalar's branch fact joined with no fact is no fact;
  a copy of an unbound local pointer is unbound.
* **Custody.** A stepped pointer owns nothing. Releasing one that moved
  forward by a positive literal is an `invalid_release` candidate; releasing
  one that may sit at its object's start is refused; using one after its
  base's release is a `use_after_release` candidate; stepping in place the
  only identity of a live object refuses (`p = malloc(n); p++; free(p - 1);`
  is never credited or accused as a leak).
* **A soundness repair in the merged base.** W43-INDIRECT-ACCESS-GENERAL's
  typed access admitted `out[m++] = c` without forgetting `m`, so
  `int m = 0; out[m++] = 'a'; if (m) return;` kept `m == 0`, pruned the
  `return`, and certified a body that leaks. The index may no longer write;
  the stepped-access shape re-admits `B[X++]` with the forget
  ([§2.5](#25-the-repair)). The arbiter measured no corpus verdict resting on
  the hole.
* **Lean.** `QCore.IndexTraversal`, 51 theorems, audited by
  `AuditIndexTraversal.lean`: the derivation lattice (commutative,
  associative, idempotent over every derivation, the pre-card answers kept),
  a concrete-offset semantics under which the step and the join are sound and
  **a post-fixed point of a loop header covers every iteration**, the custody
  rules, the index-fact join, and the refusals. No placeholders, no project
  axioms.
* **Arbiter.** BEFORE reproduces the W43-INDIRECT-ACCESS-GENERAL diagnostics
  byte for byte; both census runs reproduce their plain runs byte for byte.
  AFTER removes **1,479** leaves and adds **130**, every one attributed
  ([§4](#4-arbiter)); `attribution_complete: true`.
* **Moves: 11 functions Unknown → Safe** (all 11 forecast; 2 forecast
  functions stay Unknown). **0 unit moves. 0 violation candidates. Leak pin
  271 unchanged.**
* **Ledger 307 → 318** on the card branch, **313 → 324** after merging main (`0a07cb57`): `W43-INDIRECT-POINTER-INCREMENT` retired against its
  own discharge condition; twelve residuals registered, one per refused shape
  with its size ([§6](#6-ledger)).

| Measure (Unknown functions for families) | BEFORE | AFTER | Delta |
|---|---:|---:|---:|
| Functions Safe / provisional Unsafe / Unknown | 269 / 84 / 4,481 | **280 / 84 / 4,470** | +11 / 0 / −11 |
| Units Safe / provisional Unsafe / Unknown / data-only | 3 / 42 / 136 / 4 | 3 / 42 / 136 / 4 | 0 |
| `unsupported` / `unknown_alias` (all statuses) | 47,832 / 4,727 | **46,889 / 4,321** | −943 / −406 |
| `leak` / `summary_conflict` / violations | 271 / 1 / 0 | 271 / 1 / 0 | 0 |
| branch condition: leaves / functions / sole | 8,712 / 2,408 / 50 | 8,290 / 2,321 / 49 | −422 / −87 |
| for-loop condition | 927 / 602 / 1 | 839 / 537 / 1 | −88 / −65 |
| loop condition | 318 / 203 / 3 | 294 / 187 / 5 | −24 / −16 |
| switch expression | 96 / 90 / 1 | 78 / 75 / 1 | −18 / −15 |
| do-while condition | 31 / 29 / 0 | 29 / 27 / 0 | −2 / −2 |
| unmodeled increment or decrement | 678 / 345 / 1 | 529 / 301 / 1 | −149 / −44 |
| unrecognized assignment destination | 772 / 385 / 2 | 668 / 360 / 3 | −104 / −25 |
| unsupported assignment expression | 1,645 / 569 / 5 | 1,606 / 561 / 10 | −39 / −8 |
| formal-mutation gate | 118 / 69 / 0 | 104 / 62 / 1 | −14 / −7 |
| unresolved resource identity | 4,512 / 1,446 / 5 | 4,115 / 1,227 / 2 | −397 / −219 |

Family rows come from [blocker-census.txt](blocker-census.txt) and its
predecessor, W43-INDIRECT-ACCESS-GENERAL's, which the BEFORE run reproduces.
A sole-blocked count that rises is a function whose other families cleared.

## 1. Census

### 1.1 Instrumentation (commit `42ee628f`)

The `indirect-census` feature gains a **traversal stream**
(`QCORE_TRAVERSAL_CENSUS`, `crates/qcore-c-gateway/src/traversal_census.rs`),
written after the classifier decided:

* every refused controlling expression (`if`, `while`, `for`, `do`-`while`,
  `switch`), with the typed-condition recogniser's answer;
* every refused statement and `for` clause, every copy between two typed
  pointer names or two declared scalars, and every typed access admitted with
  a writing index, with the pointer-step and stepped-access recognisers'
  answers;
* each function's declared scalar and single-level pointer names.

The recognisers (`index_traversal.rs`, `typed_scalar_condition`,
`is_scalar_value_expression`, `FunctionTypes::pointer_kind`) were committed
with the census and read by no rule until `44d7eb5f`, so the census answers
are the rule's own. The census run reproduces **all five** BEFORE artifacts
byte for byte (inertness); the AFTER census reproduces AFTER's diagnostics.
The streams (71 MB and larger, with the analyzer's repeated passes) are not
committed.

### 1.2 Forecast

[census/predict.txt](census/predict.txt): condition 580 leaves, step 302,
gated step 32 (an upper bound: formals keep the gate), scalar/copy alias 698
(a name heuristic); at most **13** functions Unknown → Safe, 8 from the lexical
classes alone. The function count is small because traversal functions in this
corpus carry other blockers — calls, member chains, list macros — that no rule
here touches.

## 2. The rules

### 2.1 Typed condition read

`Parser::condition_access`: a controlling expression the existing routes
refused (not a supported neutral expression, no isolated contracted call, no
null test), with no call and no brace, whose every memory access
`typed_scalar_condition` admits, lowers to
`LegacyInstruction::IndirectScalarAccess { bases, alias_reads, destination: None, frame: None }`
at the header — the typed read of W43-INDIRECT-ACCESS-GENERAL, which changes
no binding. The condition walker is the statement value walker with one
widening only a condition may have: a declared single-level pointer name and
the null constant `(void *)0` are whole operands, so `p < end && *p != ':'`
and `p != 0 && *p` read like the lexical predicate's `p < end`. No increment,
assignment, unary `&` or comma is admitted, and at least one access is
required. Each base keeps its dereference obligation **even where a
short-circuit could skip the operand** (requiring an unneeded obligation only
refuses more). A base released on a reaching path is refused and accused
(`use_after_release`) through the existing settled-state pass.

### 2.2 Pointer step

`index_traversal::pointer_step` recognises `P++`, `++P`, `P--`, `--P`,
`P += E`, `P -= E`, `P = Q + E`, `P = Q - E` and the declaration
`T * P = Q ± E`, with `P` a declared `SCALAR *` or `struct T *`, `Q` any
declared single-level pointer object (`SCALAR [N]` too) and `E` a scalar value
with no memory access. It is consulted after the typed access route, for the
same refusals, and — for a local's step only — after the formal-mutation gate
(a formal's step keeps the gate, as the brief directs).

`LegacyInstruction::PointerStep { destination, source, offset }` in
`transfer`:

* an unbound `source` gives an unbound `destination` (arithmetic on a pointer
  into an object no path bound stays there);
* otherwise the destination takes `source`'s targets, nullness and
  unknownness, with `Derivation::step`: interior extent, `Stepped::Forward`
  for a positive literal forward step, `Stepped::Any` otherwise;
* no status, marker, containment or field link changes;
* a step **in place** of an identity binding whose live target no other
  identity binding, field link or containment names refuses (`unknown_alias`)
  and leaves the object's fate unresolved;
* a step into a static-storage destination publishes the object's interior and
  is refused as a copy is; a step into a sanctioned out slot's pointer is the
  existing out-parameter reassignment refusal.

`Derivation::Derived` gains `stepped: Stepped { No, Forward, Any }`. `No` is
every derivation W43-LIBC-RESULT-ALIAS made and behaves exactly as before. The
join adds one arm and keeps every other: an identity joined with a *stepped*
interior pointer is `Any` — what a loop header sees for
`for (q = p; *q; q++)` — while an identity and an unstepped derived result,
and a stepped and an unstepped one, still join to `Mixed`. The lattice is
proved a lattice in Lean (§3). `derived_release_is_accusable` excludes `Any`.

### 2.3 Stepped access

`index_traversal::stepped_access` rewrites `* P ++` / `* ( P ++ )` (and `--`)
to `* P` and `[ X ++ ]` / `[ ++ X ]` (and `--`) to `[ X ]`, requires the
rewritten statement to be a typed scalar statement, and refuses when a stepped
name appears anywhere else in the statement or is stepped twice (an
unsequenced access, C11 6.5p2) or is the scalar destination. The statement
lowers to a block: the typed access over the old values (with its frame), then
each `PointerStep` or `ForgetScalar` — the order C sequences them in.

### 2.4 Index facts and copies

* `AbstractState::join` takes the function's declared scalars
  (`LegacyCfg::traversal`, read from the same token stream as every typed
  premise, so W4 and certificate rederivation reproduce it). A declared scalar
  bound on one path with a target-free binding and unbound on the other joins
  to **unbound** — no fact — rather than the unknown, possibly-null pointer
  the generic arm makes of it. A scalar binding that carries a target keeps
  the generic arm.
* `Alias` between two declared scalars, or between two declared pointer
  objects that are not formals, with an unbound source, leaves the destination
  unbound and reports nothing. A **formal's** copy keeps its unresolved alias:
  W4 summary derivation binds exactly the formals an analysis reports
  (`bind_unresolved_formals`), and removing that signal was measured to stop it
  (`may_alias_boundary::a_guard_still_discharges_two_locals_that_are_not_a_definite_alias`).
  A walk over a caller's string therefore certifies through the W4 summary,
  which seeds the formal `Owned` (`W43-INDEX-TRAVERSAL-FORMAL-COPY`).
* `EscapeIfOwned` of a declared scalar with a target-free binding escapes
  nothing, and `observe_exit` reads it as returning nothing.

### 2.5 The repair

`typed_access`'s subscript premise used `scalar_statement_accesses`, which
admits `+`, `-` and `=` as punctuation, so `out[m++] = c` and
`out[m += 1] = c` were typed accesses with `m`'s old fact kept. The index now
refuses any `++`, `--` or assignment; `stepped_access` re-admits `B[X++]` with
the forget. The census found 23 such statements in 11 functions, all
`B[X++]`: every one is re-admitted, and no corpus verdict moved because of the
repair alone. `ut_write_varint` (`buf[i++] = …; return i;`) moved Unknown → Safe
*because* `i` is now forgotten rather than kept at zero.

### 2.6 Scope (invariant 2)

`crates/qcore-audit/src/report.rs`'s scope string gains: "an index or pointer
traversal (a stepped pointer, a moved index, a typed read in a loop
condition) decides custody, use after release and invalid release only: no
bounds claim". Spec L18 gains the rule paragraph. An index or offset is never
evaluated beyond whether a step is a positive literal.

### 2.7 Tests and pins

`crates/qcore-c-gateway/tests/index_traversal.rs` (19 tests, each asserting
the complete diagnostic vector and rechecking every `Safe` certificate): walks
over copies of formals and locals, every step form, every condition position,
counters and scalar copies, walks over a live allocation with the base
released, the unguarded nullable base, the lost-base refusal, the
`invalid_release` candidate, the refused `Any` release, `use_after_release`
through a store and through a condition, the stepped/declared `Mixed` join,
the protected formal's gate, conditions with a call or a write, the repaired
index write, and the refused return of an interior pointer. The module's unit
tests (`index_traversal.rs`, 4) pin the recognisers.

Nine existing pins move deliberately, each explained where it stands:

| Pin | Before | After, and why |
|---|---|---|
| `unit/legacy_c.rs` increment family, `char *p = q_alloc(); p++; q_free(p);` | Unknown, `Unsupported` | Unsafe: `unknown_alias` (lost base) + `invalid_release` (the release of `base + 1`) |
| `unit/legacy_c.rs` C11 memory read, `if (*p)` after `q_free(p)` | Unknown | Unsafe `use_after_release` (the condition read's accusation) |
| `indirect_scalar_access.rs` `*p++ = c` through a local | Unknown | Safe (stepped access; `p = buf` copies an unbound array) |
| `neutral_expression_positions.rs` bound base `c->hits` | branch refusal | `unknown_alias c` (the read's obligation refuses the consumed base) |
| `neutral_expression_positions.rs` `for (; table[n]; )` | Unknown | Safe; new test `subscript_condition_certifies` |
| `o04_b08.rs` `b->flag == 0` on a live guarded base | branch refusal | Safe (moved out of the refused table) |
| `reach_gate_positions.rs` read through a released base | branch refusal | `unknown_alias n` + `use_after_release` |
| `scalar_route_exit_edge.rs` body writing the deciding scalar | `unknown_alias flag` | no fact after the join for modeled writes (both edges still propagate, nothing retained, the leak still reported); the refused write keeps the Unknown arm |
| `c03_neutral_frame_recursion_probe.rs`, `neutral_frame_recursion.rs` pointer-field `seek` | Unknown (its branch read) | Safe: the design's §6.4 dependency (the read was the only gate); the heap-link teardown stays refused (tests renamed) |

## 3. Lean parity

[`QCore.IndexTraversal`](../../../proof/QCore/IndexTraversal.lean), audited by
[`AuditIndexTraversal.lean`](../../../proof/AuditIndexTraversal.lean), 51
theorems:

* **the lattice:** `join_comm`, `join_assoc`, `join_idem`, `mixed_absorbs`
  (each checked over every derivation by `decide`),
  `an_unstepped_derived_result_still_joins_the_identity_to_mixed`,
  `the_header_joins_the_identity_and_its_step_to_any`,
  `stepped_and_unstepped_families_stay_apart`;
* **interior derivation by increment:** `a_step_is_never_the_identity`,
  `a_step_of_a_non_mixed_binding_is_interior`,
  `a_backward_step_may_be_at_the_start`, `const_dropped_survives_a_step`;
* **soundness against concrete offsets** (natural numbers under the
  no-undefined-behaviour reading): `join_admits_left/right`, `le_admits`,
  `a_forward_step_is_sound`, `an_unknown_step_is_sound`, `a_move_is_sound`,
  and the loop fixed point: `every_iteration_from_the_header_is_covered`,
  `a_post_fixed_point_covers_every_iteration`,
  `the_walk_header_is_a_post_fixed_point`;
* **release:** `releasing_a_stepped_pointer_never_releases_its_base`,
  `a_forward_stepped_release_is_accusable_and_never_at_the_start`,
  `a_pointer_that_may_be_at_the_start_is_never_accused`,
  `the_walk_header_is_never_accused`,
  `a_mixed_binding_is_never_accused_or_credited`;
* **stepping in place:** `a_step_changes_no_status`,
  `a_step_keeps_its_targets`, `an_unbound_pointer_stays_unbound`,
  `losing_the_only_name_of_a_live_object_refuses`,
  `a_step_that_loses_nothing_adds_no_diagnostic`, and the witnesses
  `stepping_the_only_name_refuses`, `stepping_a_copy_is_admitted`;
* **index facts:** `a_one_sided_scalar_fact_joins_to_no_fact`,
  `the_scalar_join_is_sound`, `no_fact_admits_every_count`;
* **copies:** `an_unbound_typed_copy_is_unbound_and_admitted`,
  `an_untyped_unbound_copy_keeps_its_refusal`, `a_bound_copy_is_unchanged`;
* **conditions and stepped accesses:** `a_typed_condition_changes_no_binding`
  and `a_typed_condition_keeps_its_obligations` (over
  `QCore.IndirectScalarAccess`), `an_unsequenced_stepped_name_refuses`,
  `a_double_step_refuses`, `an_index_write_outside_the_shape_refuses`,
  `the_access_precedes_its_steps`, `a_full_stepped_candidate_is_admitted`.

The premises stated, not proved: the no-undefined-behaviour reading (a
pointer stays inside its object, so an offset is a natural number and a
backward step never leaves the object), the frame of constraint 10a, and the
Rust–Lean correspondence and C extraction premises of trust boundary 28.

## 4. Arbiter

### 4.1 Runs

Runner [`scripts/run_audit.py`](scripts/run_audit.py) (the indirect card's,
plus `QCORE_TRAVERSAL_CENSUS`), same pin, Clang and ChunkIO package as the
baseline.

| Run | Binary (sha256) | Tree | Exit | Record |
|---|---|---|---:|---|
| BEFORE | `db895e6e…` | merged base `0a8f59ac` | 0 | [before-run.json](before-run.json), [before-identity.json](before-identity.json) |
| CENSUS | `e9f840b1…` | `42ee628f` + feature | 0 | [census-run.json](census-run.json) |
| AFTER | `2ae54a0e…` | `44d7eb5f` | 0 | [after-run.json](after-run.json), [artifacts/](artifacts/) |
| CENSUS (after) | `ec36dff0…` | `44d7eb5f` + feature | 0 | [census-after-run.json](census-after-run.json) |

* BEFORE against the W43-INDIRECT-ACCESS-GENERAL baseline: diagnostics,
  chunk-contracts and source byte-identical; `contracts` differs by main's
  verdict-neutral contract records (the CONTRACT-INHERIT-* inclusion records
  and `crate:module` keys) and the checkout path, `source-effects` by the
  checkout-path digests, as the brief lists.
* CENSUS against BEFORE: all five artifacts byte-identical.
* AFTER against BEFORE (same checkout): chunk-contracts, contracts,
  source-effects and source byte-identical; only the diagnostics move, by the
  leaves below and the scope string. CENSUS (after) against AFTER: the
  diagnostics are byte-identical.
* A first census run died of a full shared disk (other worktrees); it is set
  aside privately as `aborted-census-enospc` and is not evidence.

### 4.2 Attribution

[`scripts/attribute.py`](scripts/attribute.py) →
[attribution.txt](attribution.txt), [attribution.json.gz](attribution.json.gz);
`attribution_complete: true`.

| Class | Leaves | What it is |
|---|---:|---|
| removed: rule:condition | **614** | a refused condition whose every census record the typed-condition recogniser admits (branch 422 + for 88 + loop 24 + switch 18 + do-while 2, in all statuses; 34 are `S.F`-only conditions the frozen forecast missed, §4.4) |
| removed: rule:step | **302** | increment / destination / assignment refusals of recognised steps and stepped accesses |
| removed: rule:gated-step | 16 | the formal-mutation gate on a recognised step of a local |
| removed: rule:scalar-alias | **517** | `unknown_alias` of a declared scalar (the join, the copy, the return) or of a recognised copy's source |
| removed: summary | 10 | `call to uncontracted function F` where `F` moved to Safe and gained a validated W4 summary: `ut_write_varint` ×6, `char_lookup` ×2, `decode_hex_digit`, `mk_server_lib_notify_event_loop_break` |
| removed: downstream | 20 | binding precision: a refusal that poisoned every binding became a framed or poison-free modeled statement, so later uses of `output_pointer` (cJSON `print_array`/`print_object`), `tail` (`rd_murmur2`), `left` (`mk_string_trim`), `buf`'s nullness (`mk_string_html_escape`) and similar resolve |
| added: exchange | **127** | `unknown_alias` at the coordinate of a removed rule leaf: the read's or step's obligation refusing a base that is bound, possibly null or unresolved (`buf` 30, `s` 16, …) |
| added: exchange:scalar-join | 3 | `unknown_alias` of a declared scalar elsewhere: the join left no binding, and a pre-existing generic copy of it into a place the rule does not type (`mdi->brokers[i].id = i` names `id`) meets an unbound source. Refusing direction |
| added: violation | **0** | |

Separated per `POISON-UNMASK-ON-ADMISSION`: 20 removals come from binding
precision and 10 from new summaries; no removal came from an edge becoming
infeasible (no pruned node was found in the moved functions). The repair of
§2.5 moved no leaf by itself.

### 4.3 Movers

Each mover is **Safe relative to the declared premises** (invariant 2's
reading, not a proof of the C), and nothing in it is a bounds claim.

| Function | Unit | BEFORE blocker | Shape |
|---|---|---|---|
| `rd_string_hash` | `rdstring.c` | for-loop condition `str[i] != '\0'` | index scan of a caller's string |
| `str_searchr`, `char_lookup`, `reverse_char_lookup` | `mk_http_parser.c` | branch `buf[i] == c` in a loop | index scans |
| `decode_hex_digit` | `ctr_id.c` | branches `*digit >= '0'` | reads through a formal |
| `cio_file_native_filename_check` | `cio_file_unix.c` | branch condition read | typed read |
| `rd_kafka_q_check_yield` | `rdkafka_queue.c` | branch condition read | typed read through a record pointer |
| `mk_server_lib_notify_event_loop_break` | `mk_server.c` | branch condition read | typed read |
| `bitLen`, `mk_kernel_features` | `rdhdrhistogram.c`, `mk_kernel.c` | `unknown_alias` of a counter | index-fact join |
| `ut_write_varint` | `rdunittest_msgset_errors.c` | `unknown_alias i` | `buf[i++] = …; return i;` (the repaired shape, now forgetting `i`) |

### 4.4 Prediction against outcome

| Forecast | Measured | |
|---|---|---|
| condition 580 leaves | 614 | miss +34: the forecast tested the census's `typed_bases` for truthiness, and a condition whose only accesses are `S.F` has the empty base list; `attribute.py` tests `is not None` |
| step 302 | 302 | hit |
| gated step ≤ 32 | 16 | within the bound (formals keep the gate) |
| scalar/copy alias ≈ 698 | 517 | heuristic over names; overestimate |
| ≤ 13 Unknown → Safe, named | 11, all named | 2 misses: `gf2_matrix_times` steps a formal (`mat++`, the gate), `cio_chunk_is_file`'s `type` is not a join artifact |
| 0 unit moves | 0 | hit |
| 0 violation candidates | 0 | hit |
| leak 271 unchanged | 271 | hit |
| R6 moves no leaf by itself | none | hit |

The frozen file is not edited; these corrections live here.

## 5. Validation

Measured in this worktree (`agent-a3257f6ea7e13d982`) by
[`scripts/run_gate.py`](scripts/run_gate.py) — the indirect card's runner with
`CARGO_INCREMENTAL=0` — on the implementation commit `44d7eb5f`; later commits
change documents, the ledger, report scripts and evidence only. Logs are
worktree-private under `private/gate/` and `private/provider-gate/`. The
generator input is [gate.json](gate.json), with the
[Clang recording](clang-version.txt) taken in this checkout beside it.

GATE-TABLE

## 6. Ledger

**Retired** ([retired-followups.json](retired-followups.json)):
`W43-INDIRECT-POINTER-INCREMENT`, against its own discharge condition (pointer
arithmetic modelled as an interior derivation of the same sites, with
DerivedResult's release rules, Lean parity and a measured re-pin). Its class
shrinks from 335 / 172 to 257 / 159 under its own classifier
([indirect-residual-classes.txt](indirect-residual-classes.txt)); the
remainder is re-registered narrower.

**Registered** ([followups.json](followups.json)), sized at the AFTER census
([residuals.txt](residuals.txt); leaves / Unknown functions / sole):

| Id | Size | What stays refused |
|---|---|---|
| `W43-INDEX-TRAVERSAL-CONDITION-CALL` | 4,419 / 1,603 / 28 | a condition carrying a call no route isolates |
| `W43-INDEX-TRAVERSAL-CONDITION-UNTYPED` | 2,616 / 1,154 / 28 | a condition access the reader does not type |
| `W43-INDEX-TRAVERSAL-CONDITION-CHAIN` | 1,248 / 632 / 1 | member chains in a condition |
| `W43-INDEX-TRAVERSAL-CONDITION-ADDRESS` | 976 / 594 / 1 | `&` in a condition (the list-iteration macros) |
| `W43-INDEX-TRAVERSAL-CONDITION-WRITE` | 271 / 166 / 0 | `while (*p++)`, `while (n--)` |
| `W43-INDEX-TRAVERSAL-STEP-COMPUTED` | 102 / 63 / 1 | an access, call or cast in a step |
| `W43-INDEX-TRAVERSAL-STEP-UNTYPED` | 761 leaves | steps in unread functions or of untyped names |
| `W43-INDEX-TRAVERSAL-FORMAL-STEP` | 48 / 37 / 1 | `s++` on a protected formal (the gate) |
| `W43-INDEX-TRAVERSAL-COMPUTED-POINTER-ACCESS` | 257 / 159 / 0 | `*(p + i)`, the rest of the retired class |
| `W43-INDEX-TRAVERSAL-ACCUSING-UNEXERCISED-IN-CORPUS` | — | the candidate arms reach no corpus function |
| `W43-INDEX-TRAVERSAL-DESCRIPTOR-PREDICATE` | — | the scalar join must consult the descriptor card's predicate |
| `W43-INDEX-TRAVERSAL-FORMAL-COPY` | — | a formal's copy keeps its refusal for W4 |

Noted, not retired: `W43-INDIRECT-OTHER-STATEMENTS` (the traversal stream
records `for` clauses and conditions, but its own class is not split here);
`W43-INDIRECT-PROTECTED-FORMAL-INCREMENT` (pointee increments, untouched);
`POISON-UNMASK-ON-ADMISSION` (measured for this admission, §4.2).

Follow-ups go from **307 to 318** on the card branch, and from **313 to 324** after main `0a07cb57` (the indirect card's integration and three contract cards) was merged; the entry-level merge kept both sides' entries. `corpus.cycle19_raw_units` points at this
report's artifacts; its previous contents are in
[previous-corpus-row.json](previous-corpus-row.json).

## 7. For the integrator

* **Proposed gateway amendment**, a constraint 5i beside the indirect card's
  proposed 5h, in these words:

  > W43-INDEX-TRAVERSAL: a refused controlling expression with no call whose
  > every access is a typed scalar access (pointer names and the null
  > constant as whole operands only) lowers to the typed read, each base's
  > obligation required even under short-circuit. `p++`, `p += e`,
  > `p = q ± e` over declared single-level pointers lower to `PointerStep`,
  > interior-derived from the source's binding (`Stepped::Forward` for a
  > positive literal, `Any` otherwise or when joined with the base identity);
  > it owns nothing, a forward-stepped release is an `invalid_release`
  > candidate, an `Any` release refuses, a use after the base's release is a
  > `use_after_release` candidate, and a step in place of a live object's
  > only identity refuses. `*p++`/`B[X++]` lower to the access then the step
  > or forget. A declared scalar's target-free fact joined with no fact is no
  > fact. A local pointer's copy of an unbound local is unbound; a formal's
  > copy keeps its refusal. Typed indices may not write. No bounds claim.

* **The repaired hole** (§2.5) is in W43-INDIRECT-ACCESS-GENERAL's rule; the
  merge carries the repair, so no integration step can reintroduce it.
* **Files likely to conflict:**
  * `crates/qcore-c-gateway/src/legacy_c.rs`: `LegacyInstruction`
    (`PointerStep`), `StepOffset`, `TraversalNames`, `LegacyCfg::traversal`,
    `Statement` (`condition_access`), `Parser::classify_simple`,
    `condition_access`, `stepped_access`, `CfgBuilder::compile`,
    `branch_instruction`, `Derivation` (`Stepped`), `AbstractState::join`
    (signature), `transfer` (the `Alias`, `EscapeIfOwned`, `PointerStep` and
    publication arms), `observe_exit` (signature), `derive_pointer`,
    `derived_release_is_accusable`, `build_function_cfg`;
  * `crates/qcore-c-gateway/src/typed_field_access.rs` (`PointerKind`,
    `pointer_kind`, `traversal_names`, the condition walker, the subscript
    repair), `index_traversal.rs`, `traversal_census.rs`,
    `indirect_census.rs` (`unit_seq`), `lib.rs`, `legacy_recursion.rs`
    (`combine`), `legacy_joined_provenance.rs` (test literals);
  * the concurrent **W43-INDEXED-RESOURCE-COLLECTIONS** card shares the loop
    machinery: nothing here adds a loop-specific pass, the fixed point is the
    existing work list, and no collection custody is modelled;
  * the concurrent **W43-DESCRIPTOR-RESOURCE-TRACKING** card: see
    `W43-INDEX-TRAVERSAL-DESCRIPTOR-PREDICATE`;
  * `crates/qcore-audit/src/report.rs` (the scope string changes every
    future diagnostics artifact), `compiler/examples/alias_aggregate_probe.rs`,
    `proof/QCore.lean`, `docs/QCORE_SPEC_r0_3.md` L18;
  * nine gateway test files with deliberate pin moves (§2.7);
  * the shared records: `docs/reviews/r0_3/acceptance_matrix.json`,
    `docs/STATUS.md`, `docs/development/increments/wave-43.md`,
    `docs/reports/INDEX.md`.

## Reproduction

```sh
python3 docs/reports/w43-index-traversal/scripts/run_audit.py <before> 0a8f59ac --binary <base binary>
python3 docs/reports/w43-index-traversal/scripts/run_audit.py <census> 42ee628f+census --census --binary <census binary>
python3 docs/reports/w43-index-traversal/scripts/predict.py <census> predict.json predict.txt
python3 docs/reports/w43-index-traversal/scripts/run_audit.py <after> 44d7eb5f --binary <after binary>
python3 docs/reports/w43-index-traversal/scripts/run_audit.py <census-after> 44d7eb5f+census --census --binary <after census binary>
python3 docs/reports/w43-index-traversal/scripts/attribute.py <before> <after> <census> attribution.json attribution.txt
python3 docs/reports/w43-index-traversal/scripts/residuals.py <census-after> residuals.json residuals.txt
python3 docs/reports/w43-indirect-access-general/scripts/residuals.py <census-after> classes.json classes.txt
python3 docs/reports/w43-libc-contracts/scripts/archive.py <after> docs/reports/w43-index-traversal/artifacts
python3 docs/plans/cycle19-custody-cards/blocker_census.py docs/reports/w43-index-traversal/artifacts
python3 docs/reports/w43-index-traversal/scripts/update_ledger.py
python3 docs/reports/w43-index-traversal/scripts/run_gate.py <gate-dir> submodules clang test fmt clippy probe followup lean census
```
