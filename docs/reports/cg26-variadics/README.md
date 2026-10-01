# CG26-VARIADICS: the `va_list` protocol and trailing actuals per position

Card `CG26-VARIADICS` of the [Clang gap card queue](../../plans/clang-gap-cards/README.md)
(wave 48, round 10), gap entries K08, K09 and K10, route G. It absorbs
`W44-VA-START-END-BUILTINS` and `W46-FCNTL-SETFD-ADDRESS-TAIL`. Base: local
`main` `b6a3dc42`, rebased onto `8e8ed41f` (CG17-LAYOUT-DERIVED-EXTENTS) and then
`06ad81cb` (CG20-NUMERIC-DOMAIN) and `b934c560` (CG25-RESOURCE-KINDS) after the
AFTER measurement.

- **Rule:** [gateway 11w](../../agents/gateway.md#constraint-11w), amending E20's
  uncontracted-builtin refusal for the four `va_*` builtins only and O04-TRACE's
  tail premises T1-T6 by T7 and T8.
- **Vocabulary:** `ensures normal resource_variadic_borrowed_tail();`
  ([contract spec C04.2](../../QCORE_CONTRACT_SPEC_r0_3.md#c042-parameter-modes),
  G05), a strengthened data tail; no provider declares it in this card.
- **Lean:** `QCore.Variadics`, `proof/AuditVariadics.lean` (19 theorems).
- **Prediction:** [PREDICTION.md](PREDICTION.md), frozen at `82fdad31` from the
  BEFORE census and before any AFTER binary existed. The implementation had
  been written before that census (recorded in the file).
- **Property published, and its scope.** The guarantee chosen is *custody under
  variadic calls and definitions*: a `va_*` builtin is custody-neutral only
  where C11 7.16's protocol is established on every path of the function, and a
  trailing actual is read only as far as its callee's declared tail states.
  Format agreement (K10) is **not** claimed: no format checker is built, and
  neither tail clause says anything about a format string.

## 1. What changed

**The protocol** (`va_list_protocol.rs`). From Clang's AST, a function's lists
are its `va_list` parameters (initially active, owing nothing; `va_start`,
`va_copy` into and `va_end` on them refuse) and the `va_list` locals of its
outermost block (initially uninitialised, owing `va_end` on every exit). Events:
`__builtin_va_start(L, P)` (P the last named parameter of a variadic
definition), `VAArgExpr`, `__builtin_va_copy(D, S)` (a read of S, a start of D),
`__builtin_va_end(L)`, `L` as a whole argument of another call (a pass; the list
is then indeterminate until `va_end`, 7.16p3) and `&L` of a local as a whole
argument (a lend, footnote 253: read like `va_arg`). Every other mention, two
events in one full expression, an event under `&&`/`||`/`?:`/a statement
expression/`sizeof`, a statement expression holding a jump, a computed `goto` or
a `longjmp` refuses by name. A control graph (sequence, `if`, the three loops,
`switch` with its labels, `break`, `continue`, `return`, `goto`/labels) is built;
the per-list sets of `ListState` (uninitialised, active, passed, ended) are
iterated to a fixed point; `decide` then checks exactly `QCore.Variadics.certify`,
list by list: the entry holds the initial state, every node whose set is
non-empty steps its events (`step`) and its result is contained in each
successor's set, and the exit's set is accepted (`exit_ok`).

**The lowering** (`legacy_variadics.rs`, asked after every older route of a
statement, only for an uncontracted `va_*` builtin's refusal). Under an
established protocol: `va_start`/`va_end`/`va_copy` and a discarded `va_arg` are
`NoOp` (a list is never bound); `x = va_arg(L, T)` / `T x = va_arg(L, T)` with an
arithmetic or enumeration `T` and a scalar local `x` is `ForgetScalar(x)`.
A pointer `va_arg` keeps a named refusal. Under a refused protocol the statement
keeps its refusal with the protocol's reason, so a missing `va_end`, a use after
`va_end` or after a pass, and a `va_copy` without its own `va_end` are never
`Safe`.

**Variadic definitions.** `typed_field_access::read_parameters` reads the named
parameters before a trailing `, ...` (`variadic_tail::named_parameters`; C11
6.7.6.3p9, the ellipsis declares no object). Before, every variadic definition's
lexical declaration reader refused, so every typed rule was off in it (found
while testing: `int f(int n, ...) { int x; x = n; … }` reported `unresolved
resource identity n`).

**Trailing actuals** (`variadic_tail.rs`, the second statement fallback). For a
call statement refused only by the tail refusal, to a callee with a declared
tail, after Clang's promoted type of every trailing argument (C11 6.5.2.2p7) is
arithmetic, enumeration or pointer, the statement is re-classified under the
`variadic_tail_widening` cell with every trailing position `Borrowed`
(`borrowed_tail_contract`), each trailing actual being T1-T6 data, an admitted
nested call, **T7** `&NAME` of a declared scalar that is neither a descriptor
name nor a protected formal (its scalar fact forgotten after the call), or —
only under `resource_variadic_borrowed_tail()` — **T8** a plain name, read as a
`Borrowed` place (the flow's liveness and alias checks apply). Under the data
tail an identity-carrying actual keeps O04-TRACE's refusal: the earlier cards'
adversarial rows (`o04_trace`, `prod_contracts`, `chained_tail`, `neighbours`)
are unchanged and pass.

**The clause** (`qcore_contract::contract_loader`): `resource_variadic_borrowed_tail()`
sets the data-tail flag and its own; at most one of the two per declaration
(`duplicate variadic data tail`); `variadic_borrowed_tails`,
`declared_variadic_borrowed_tails`; the inherited-declaration agreement compares
it.

**Shared items added (for the integrator).** `legacy_c.rs`: Parser fields
`variadic_tail_widening` (a `Cell<bool>`) and `variadic_borrowed_tails`; the tail
refusal arm asks `tail_widened` only under the cell; two hooks after rule 11i's
atomic hook (`va_list_statement`, `variadic_tail_statement`); the source scan of
`declared_variadic_tails` also matches the new spelling. `typed_field_access.rs`:
the one call to `variadic_tail::named_parameters`. Test support
`tests/support/clang_gap.rs`: `contract_sources()` (the old `contracts()` is
unchanged in effect). CG21's common builtin boundary must stay **after** the
`va_list_statement` hook (or exclude the four `__builtin_va_*` names).

## 2. Reproduced claims and corrections

| Claim (input) | BEFORE (executed) | Standing |
|---|---|---|
| K08: G validates declared data tails; T rejects variadic signatures | CG00 `k08_variadic_tails_are_data_only` | reproduces |
| K09: `VAArgExpr` unread by T; `va_*` uncontracted in G | CG00 `k09_va_list_operations_are_not_modelled` | reproduces |
| K10: format agreement not checked | CG00 `k10_format_agreement_is_not_checked` | reproduces (not claimed here) |
| W44-VA-START-END-BUILTINS: 127 leaves in 51 functions | corpus BEFORE: **124 leaves in 50 functions** (`va_end` 55, `va_start` 50, `va_arg` 17, `va_copy` 2) | **size correction** (the W44 base was older); `rd_kafka_set_thread_name` blocked by them alone reproduces |
| W46-FCNTL-SETFD-ADDRESS-TAIL: 1 leaf, 1 function | `rd_kafka_open_cb_generic` (`rdkafka_offset.c:138`), its only leaf | reproduces |

CG00's k09 control is flipped for route G (documented at the test); k08 and k10
are unchanged.

## 3. Measurements

Release `quirin-audit`, corpus arbiter `66910c10` `--jobs 4`, system route and
tail-pause slice (`scripts/measure.sh`; the corpus AFTER by `scripts/corpus-noparity.sh`).

| Run | binary | corpus | system | tail-pause slice |
|---|---|---|---|---|
| BEFORE, main `b6a3dc42` (`private/cg26va/before*`) | `fa2ac484…` | **629 / 10 / 4,195** `3d53a5d0…` | 44 / 2 / 188 `17b19316…` | 29 / 1 / 239 / 1 P `a2d9cf47…` |
| AFTER, tip on `b6a3dc42` (`after4`, `after-system`, `cg26va-after`) | `37a7be19…` | **631 / 10 / 4,193** `42c7df13…` | 44 / 2 / 188 `17b19316…` (byte-identical) | 29 / 1 / 239 / 1 P `3fe72160…` |

The corpus BEFORE reproduces the brief's baseline digest. The system and slice
totals reproduce; their digests differ from CG05's (`79404fed…`, `93cc660c…`)
because those reports embed the measuring worktree's path (8 occurrences in the
system report), not because of a verdict change.

**Three AFTER corpus attempts were discarded.** The first ran the BEFORE
binary (`scripts/run_pinned_audit.py` always runs `target/release/quirin-audit`
and ignores `measure.sh`'s `BINARY`; stopped, PID captured from its path). The
second and third died with `No space left on device` at the parity flush when
the shared disk filled (another card's builds); they measure the disk, not the
program. The fourth ran without the parity stream (the arbiter's result does
not depend on it) and is the measurement. The system and slice AFTER ran the
AFTER binary through `BINARY` and completed. Main then moved three times
after the AFTER: `8e8ed41f` (CG17-LAYOUT-DERIVED-EXTENTS) and `06ad81cb`
(CG20-NUMERIC-DOMAIN) left the corpus byte-identical at `3d53a5d0…` by their
own reports; `b934c560` (CG25-RESOURCE-KINDS) moved it to 630 / 10 / 4,194
(`5ade5aab…`). The AFTER was **not repeated on the combined tree** (the shared
lock serialises arbiter runs; each takes over an hour on this host). CG17's
hooks (VLA declarations, `container_of` values), CG20's (numeric, no custody
decision) and CG25's (kinded and in-place custody) take refusals this card's
two hooks do not (uncontracted `va_*` builtins, the tail refusal), so the
combined corpus is expected at 632 / 10 / 4,192; that is a prediction, and the
integrator's merged measurement is the check.

**Corpus movement** ([comparison](artifacts/compare-corpus.json),
[census](artifacts/census-corpus.json)): **+2 Unknown → Safe**, exactly the
prediction, no Safe → non-Safe move, no leak, violation or Unsafe added, 49
Unknown functions change leaves.

| Function | Move | Attribution |
|---|---|---|
| `rd_kafka_set_thread_name` | Unknown → Safe | protocol: its `va_start`/`va_end` were its only leaves |
| `rd_kafka_open_cb_generic` | Unknown → Safe | T7: `fcntl(fd, F_SETFD, FD_CLOEXEC, &on)` (W46) |

Leaf deltas: `__builtin_va_end` −53, `va_start` −49, `va_arg` −14, `va_copy` −1
(124 → 7 leaves, 50 → 4 functions); the variadic parameter reader removes
`unresolved resource identity` −17, `unmodeled increment or decrement` −14,
`unrecognized assignment destination` −14, `indirect memory access` −4, tail
refusals −7 (`fprintf` −3, `printf` −2, `snprintf` −2; their actuals pass T4
once the declarations are read), `sizeof` −2 and four others −1; the `fcntl`
tail −1. Added: 19 named refusals in 8 functions, every one classified by
reading:

| Function | Named refusal | Reading |
|---|---|---|
| `mk_vhost_set` (monkey `mk_lib.c:523`) | `va` may reach a function exit without `va_end` (4) | **correct**: `if (!value) { return -1; }` inside the `va_arg` loop returns without `va_end` (an upstream candidate, not a validated defect) |
| `rd_kafka_cgrp_rejoin` (`rdkafka_cgrp.c:1590`) | `ap` may be read … before `va_start` (2) | **correct**: `rd_kafka_cgrp_consumer_rejoin(rkcg, fmt, ap)` passes `ap` before `va_start(ap, fmt)` (an upstream candidate, not a validated defect) |
| `rd_kafka_mock_handle_PushTelemetry_decode_error` | `ap` may be read after … being passed (2) | precision: `rd_rkb_log`'s `do { … } while (0)` back edge is drawn, so the one pass is read as two (`CG26-VA-PROTOCOL-PRECISION`) |
| `rd_kafka_error_new_v` | `ap2` is not a plain local of the outermost block (2) | precision: an inner-block list (`CG26-VA-PROTOCOL-PRECISION`) |
| `rd_kafka_producev` | an operation under a conditional operator (6) | precision (`CG26-VA-PROTOCOL-PRECISION`) |
| `ut_topic_partitions`, `ut_populate_member_owned_partitions_generation`, `mk_config_set` | the `va_arg` value stored into a pointer (3) | by design: K09's pointer half (`CG26-POINTER-VA-ARG`) |

Still uncontracted: `rd_http_error_new`'s four (its `va_list ap2` is an
inner-block list) and three `va_arg` inside other expressions
(`cmt_histogram_buckets_create`, `rd_kafka_mock_push_request_errors`,
`rd_kafka_mock_partition_push_request_errors`), all in
`CG26-VA-PROTOCOL-PRECISION`. Tail refusals left: 201 leaves in 121 functions
(`fprintf` 126/78, `snprintf` 55/33, `mk_print` 17/9, `printf` 3/3), none
admissible under a data tail; `cb_debug_chunk_complete` is blocked by
`printf("…%s…", in_len, buf)` alone (`CG26-BORROWED-TAIL-DECLARATIONS`).

**System route:** byte-identical; its 11 tail statements (`flb_log_print` 10,
`snprintf` 1) carry live identities (`flb_debug("… %p", task)`,
[shapes](artifacts/shapes-system.json)) and wait on a borrowed-tail
declaration. **Tail-pause slice:** no status move; 4 functions lose 20 `va_*`
leaves (`va_end` −12, `va_start` −4, `va_arg` −4) and gain 4 pointer-`va_arg`
refusals (the loop-owner property setters' `value = va_arg(va, char *)`)
([comparison](artifacts/compare-slice.txt)).

**Prediction against outcome.** Predicted 631 / 10 / 4,193 with the two named
moves, ≥ 90 of 124 `va_*` leaves removed, no T8 movement, no system or slice
status move: all hold (117 of the 124 uncontracted `va_*` leaves gone, 19 named refusals in their place or beside them). The allowance of up to +3 further moves
was not used.

## 4. Tests and Lean

Rust: `tests/cg26_variadics.rs` (22: straight start/end, scalar `va_arg` in a
loop, a leak beside an established protocol, missing `va_end`, a `return`
between start and end, `va_copy` with and without its own end, use after
`va_end`, double `va_start`, a `goto` to the ending label, a second pass, a
parameter list, a pointer `va_arg`, conditional and escaping uses, the
`va_start` operand; the borrowed-tail logger with a live, a leaking and a
released identity, `&p` in the tail, the data tail's unchanged refusal;
`fcntl(fd, F_SETFD, FD_CLOEXEC)` with the production contract, the `&on` tail,
`&fd` and `&q`, the forgotten scalar; the clause's loading), unit tests in
`tests/unit/va_list_protocol.rs` (the table over all 2 × 16 × 5 inputs, the
fixed point on hand-built graphs, the Lean binding) and
`tests/unit/variadic_tail.rs`.

Lean `QCore.Variadics`: `step_sound` and `step_refuses` (the abstract step is
the pointwise lifting of the concrete protocol), `exit_sound`,
`step_vectors_hold` (bound to `STEP_VECTORS` by
`cg26_lean_step_vectors_match_the_rust_table`), `run_sound`,
`certified_invariant`, `certified_no_violation`, `certified_exit` (an accepted
graph has every reachable concrete state in its assignment, no reachable
violating step, and no local list reaching the exit unended), the refusal facts,
non-vacuity (`the_straight_protocol_is_certified`,
`the_skipping_protocol_is_never_certified`), and the tail
(`borrowed_tail_is_the_identity`, `a_scalar_address_demands_nothing`). The
graph's construction from the AST is trust boundary 28's premise; the
per-list decomposition is exact because no event reads another list's state.

## 5. Follow-ups

Retired: `CG26-VARIADICS`, `W44-VA-START-END-BUILTINS`,
`W46-FCNTL-SETFD-ADDRESS-TAIL`. Registered (sizes in
`scripts/update_ledger.py`): `CG26-VA-PROTOCOL-PRECISION`,
`CG26-POINTER-VA-ARG`, `CG26-BORROWED-TAIL-DECLARATIONS`,
`CG26-ROUTE-T-VARIADICS`.

GATE_SECTION
