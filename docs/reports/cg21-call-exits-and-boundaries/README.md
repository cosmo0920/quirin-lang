# CG21-CALL-EXITS-AND-BOUNDARIES

Wave 48, round 9, of the [Clang gap queue](../../plans/clang-gap-cards/README.md#phase-4-vocabulary-numeric-and-weak-memory-extensions-v-by-guarantee)
(entries C09–C13, E20, I06; absorbs `W43-LIBC-NORETURN`). Route G. Gateway
rule [11o](../../agents/gateway.md#constraint-11o); Lean `QCore.CallExits`
([`AuditCallExits.lean`](../../../proof/AuditCallExits.lean)). Measured on base `b6a3dc42`; rebased onto main `f6336c9d` (CG28-WEAK-MEMORY-EVENTS on `f0bbc0eb`) without a re-measurement. CG17-LAYOUT-DERIVED-EXTENTS and CG20-NUMERIC-DOMAIN left the corpus, system and slice custody reports byte-identical to `b6a3dc42`'s; CG25-RESOURCE-KINDS moved the corpus to 630 / 10 / 4,194 and CG26-VARIADICS models the `__builtin_va_*` builtins, so the combined figure on `f6336c9d` is the integrator's arbiter run to take, and this card's attribution is against `b6a3dc42`. On the rebase `__builtin_va_arg` left `UNDECLARABLE_C_NAMES` (CG26's `va_list` route reads it; its uncontracted wording is CG26's), so the 17 corpus and 4 slice `__builtin_va_arg` renames measured below no longer occur on this tip.

**Published property and scope** (Phase 4 opens a V card only for a chosen
guarantee): resource custody on route G, with a call statement's
**terminating exit** as a path end. The published guarantee is unchanged —
every acquired resource is released or handed on at every *return* of the
function — and a path that ends at a declared terminating call has no return.
No guarantee is claimed for a non-local exit, a second return, inline
assembly, SEH, a `cleanup` scope exit or a computed `goto`: each is refused
by name.

**Result.** Corpus **629 / 10 / 4,195 → 641 / 10 / 4,183** (`3d53a5d0…` →
`c9cdfb1c…`): **12 Unknown → Safe**, no other move, no correction, no leak or
violation added. Every one of the 1,265 `__assert_fail` / `abort` / `exit`
leaves is gone (1,340 statement path ends, every one resolved `noreturn`; no
declaration was refused at a call). System route and tail-pause slice: no
status move, the predicted renames only. Ledger 672 → 674 (on main `f6336c9d`):
`CG21-CALL-EXITS-AND-BOUNDARIES` and `W43-LIBC-NORETURN` retired, four
residuals registered.

## 1. What changed

**The no-return exit is a declaration, and the attribute must agree.** A
call statement ends its path (a CFG node with no successor that is not an
exit: nothing after it is reached, it contributes no exit state, and its own
contract is still applied, so its actuals' preconditions are checked) only
when both hold:

1. **The contract** carries the new v1 clause `ensures normal
   resource_no_return();` (`crates/qcore-contract/src/contract_loader.rs`;
   contract specification [C04.3.3](../../QCORE_CONTRACT_SPEC_r0_3.md#c0433-the-no-return-exit)).
   It says the call has no normal exit *and* that the exit it takes is
   program termination: no frame of the calling thread resumes. The loader
   admits it only on a `unit` result with `resource_return_none()` and every
   parameter `resource_borrowed`. Like the variadic data tail it is not on
   the custody record; it is rederived from the retained trusted texts
   (`no_return_callees`, `declared_no_return_callees`), so an inferred or
   proposed contract never carries it.
2. **The Clang declaration** the call resolves to is `noreturn`
   (`call_exits::resolve`): every `DeclRefExpr` naming the callee in the body
   refers to a `FunctionDecl` whose type ends in `__attribute__((noreturn))`
   (a spelling holding a pointer declarator is not read) or which carries
   `NoReturnAttr` / `C11NoReturnAttr` / `CXX11NoReturnAttr`, inherited
   included (the unit's file-scope ids are read once into the AST index,
   block-scope ones from the body); no object of that name is declared in the
   body, and the references agree. Otherwise the declared call is refused:
   `no-return contract of `f` not established at this call: …`.

**Why both.** The brief allowed either source. The attribute alone is not
sound for custody: `longjmp`, `siglongjmp` and `pthread_exit` are
`noreturn` too, and at a non-local exit the frame's live resources are
abandoned while the program goes on, so a path end there hides a leak
(`QCore.CallExits.nonlocal_path_end_hides_leak`; the Rust control
`longjmp_across_a_live_resource_is_refused`). A user `noreturn` function may
itself `longjmp` (libpng's error handlers do). The exit's *kind* is therefore
a declaration — a trusted assumption about the C library, in
`quirin-provider-libc` — and a declaration the call's own C declaration does
not establish is refused, never applied as a path end. Hard invariant 1
decides it: an unproven call effect (here, the exit kind) is `Unknown`.

**Declared** (`providers/quirin-provider-libc/contracts/libc/stdlib.qv`,
flexible, pinned by `scripts/pin.sh` at `66910c10` and `046accf6`, `pin
--check` exit 0 at both, three lock rows): `abort()`, `exit(status)` and
glibc's `__assert_fail(assertion, file, line, function)`. `__assert_fail`
lives in `stdlib.qv` beside `abort` (C17 7.2.1.1: a failed `assert` calls
`abort`), so every selection that reads the module reads it; a new
`assert.qv` would have needed every profile's selection changed.

**Positions.** The path end is taken where the call is a statement (an
expression statement whose classification is the declared `ContractCall`
with no destination; inner statements of a lowered statement expression,
glibc's `assert` among them, are statements). In every other position (a
`?:` arm, an `&&`/`||` operand, a comma operand of a value, a condition, a
`return` operand) the declared call is applied as an ordinary borrowing call
whose continuation C does not have: an over-approximation of the paths, so
never a wrong `Safe` (a diagnostic on that continuation can only be a
provisional `Unsafe` or an `Unknown` C does not reach), and never a path end.
That keeps every
position decided by the one application point; a precise operand-position
path end is `CG21-NORETURN-OPERAND-POSITIONS`.

**Undeclarable names** (`contract_loader::UNDECLARABLE_C_NAMES`): the Clang
extension nodes spelled like calls (`__builtin_choose_expr` → ChooseExpr,
`__builtin_bit_cast`, `__builtin_offsetof`, `__builtin_types_compatible_p`,
`__builtin_convertvector`, `__builtin_shufflevector`,
`__builtin_FILE`/`FUNCTION`/`LINE`/`COLUMN`), the returns-twice calls
(`setjmp`, `_setjmp`, `__setjmp`, `sigsetjmp`, `__sigsetjmp`, `savectx`,
`vfork`, `getcontext`) and the non-local exits (`longjmp`, `_longjmp`,
`siglongjmp`, `__longjmp_chk`, `setcontext`, `swapcontext`, `pthread_exit`,
`thrd_exit`). The loader rejects a declaration of any of them (an ordinary
contract would give the node or the call an ordinary continuation and hide
the edge or the abandoned frame), so an ordinary function contract is never
applied to a non-call node. The gateway names an uncontracted call to one
(`` `longjmp` is a non-local exit, which no resource contract or route
models``), with the same writes as before: only the wording changes. A
builtin Clang parses as a call (`__builtin_expect`, `__builtin_memcpy`) stays
an ordinary call.

**Boundaries asked first** (`call_exits::statement_boundary`, the hook at the
head of `expression_statement`; `goto *` in the `goto` arm): a statement
holding `asm`/`__asm__` (inline assembly, an `asm` label or register
binding), `asm goto`, `__try`/`__except`/`__finally`/`__leave`, a
`cleanup`/`__cleanup__` attribute or a label address `&&label` is an
`Unknown` with the boundary's name and unbounded writes; a computed `goto
*E;` likewise (it used to fail the parse of the whole function). They are
asked before every older route: an older route admitting such a statement
would drop the attribute's scope-exit call or the assembly's effects.
Assembly used to read as a call to an uncontracted function named after its
first keyword (`volatile`, `__asm__`, `goto`), a misnamed refusal (CG00's
correction to C12); a `cleanup` local read as the indirect-access refusal.

## 2. Reproduction of the cited claims

Every cited entry was reproduced as an executed control before the change
(CG00's `clang_gap_phase4.rs`, which this card now flips to the named
refusals; the BEFORE wording stays in each test's comment):

| Entry | Input's claim | Reproduced | Note |
|---|---|---|---|
| C09 computed goto | G covers direct goto only; T lacks `IndirectGotoStmt`/`AddrLabelExpr` | yes | **Correction**: in G a `goto *` failed the parse of the whole function (`<function not projected>`); now a named refusal in that function |
| C10 setjmp/longjmp | no returns-twice or cross-frame edge | yes (`if (setjmp(b))` refused as a branch condition) | 0 corpus leaves |
| C11 cleanup | T rejects; G has no `CleanupAttr` lowering | yes | CG00's correction stands: G refused through IND, not by name; now named |
| C12 asm / asm goto / SEH | T rejects `GCCAsmStmt`; general allowlists do not lower SEH | yes | CG00's correction stands: G read `__asm__ volatile` as an uncontracted call to `volatile`; now named |
| C13 noreturn | T handles terminal statements only; G has no noreturn | yes | T unchanged |
| E20 builtins/extension nodes | non-call AST kinds are not contracted calls | yes | **Correction** (CG00's): G read `__builtin_choose_expr`, `__builtin_offsetof`, `__builtin_va_arg`, `__builtin_bit_cast` as uncontracted calls, so a contract of that name would have applied; the loader now rejects one |
| I06 extension nodes | not contracted calls | yes | OpenMP pragmas stay inert without `-fopenmp` (CG00) |

Measured lever (`scripts/census.py`, artifacts/census-before.json):
W43-LIBC-NORETURN's "exit blocks 13 functions and abort 10" and "together 33
leaves" reproduce (13 / 10; 22 + 11). CG07's "1,232 `__assert_fail` leaves on
533 functions" reproduces; its "10 Unknown functions blocked by it alone" is
**11** at this base (`snappy.c::get_hash_table` joined), and the three callees
together block 13. The brief's system and slice totals reproduce (44 / 2 / 188;
29 / 1 / 239 / 1 Pending); their raw digests do not, because a route report
embeds the tree's path (review-hold module paths and the per-unit
`resource_contract_sources` / `inventory_sha256` digests). The BEFORE routes
ran over an extraction of the base tree (`scripts/measure-routes.sh`: the
`current` route reads the contract modules of the tree it runs in, so the
BEFORE binary cannot read this card's `stdlib.qv`); with those fields
normalised (`scripts/identity.py`) the slice BEFORE equals CG05's `after3`
slice exactly (`c2c3b067…`). The frozen PREDICTION.md's note that only the
review-hold paths differ was incomplete; the two digests are the rest.

## 3. Prediction against outcome

| Prediction (frozen) | Outcome |
|---|---|
| ≥ 1,200 of the 1,232 `__assert_fail` leaves go; any left in an operand position | all 1,232 go, and all 22 `exit` and 11 `abort`: 1,265 BEFORE sites, 0 outside a statement (`scripts/residuals.py`) |
| Unknown → Safe for the 13 sole-blocked functions (upper bound), cascade possible | **12**. `snappy.c::get_hash_table` does not move: its `unresolved resource identity `table`` leaf (an `unknown_alias`, which the census did not count as a blocker) remains. A census flaw in the prediction, not a route correction. No cascade move. |
| No Safe → non-Safe; no hidden leak unhidden | none; no leak or violation diagnostic added anywhere |
| offsetof (39), va_arg (17), `__asm__` (6) renamed, no move | as predicted, plus one `braced initializer` leaf renamed to the assembly boundary (Valgrind's client request in `mk_http_thread_create`, a statement expression holding `__asm__`) |
| System: the `exit` leaf goes, 3 offsetof renamed, no move | as predicted, plus one `braced initializer` leaf renamed to the assembly boundary (`flb_sched_timer_coro_create`) |
| Slice: 2 offsetof and 4 va_arg renamed, no move | exactly |

## 4. Measurement

Corpus arbiter `66910c10`, `--jobs 4`, release binaries `51839dc8…` (base)
and `cc8d27bb…` (`83ad877c`); system route and tail-pause slice as in
`scripts/measure-routes.sh` (digests in `artifacts/digests.txt`).

| Run | Corpus | System | Slice |
|---|---|---|---|
| BEFORE, base `b6a3dc42` | 629 / 10 / 4,195 `3d53a5d0…` | 44 / 2 / 188 `6e65757c…` | 29 / 1 / 239 / 1 P `499081be…` |
| AFTER, `83ad877c` | **641 / 10 / 4,183** `c9cdfb1c…` | 44 / 2 / 188 `e2408f8b…` | 29 / 1 / 239 / 1 P `61a89016…` |

**The 12 moves** (Unknown → Safe, `artifacts/corpus-comparison.json`), each
by its own path end alone: `mk_header_set_http_status` (`abort`; its
`unknown_alias` went with it: the aborting arm no longer joins),
`mk_socket_reset` (`exit`), and `__assert_fail` in `isFullyBalanced0`,
`rd_kafka_cgrp_clear_wait_resp`, `rd_kafka_cgrp_set_wait_resp`,
`rd_kafka_confval_get_int`, `rd_list_set_cnt`, `get_u32_at_offset` and
tinycthread_extra.c's `rwlock_rdlock`, `rwlock_rdunlock`, `rwlock_wrlock`,
`rwlock_wrunlock`.

**Every changed function** (594; `scripts/families.py`,
`artifacts/corpus-families.json`): besides the 12, 582 keep their status —
494 lose only path-end leaves, 36 also an `unresolved resource identity` (a
null or alias fact the aborting arm used to weaken at the join: after
`assert(p)` only the arm where `p` holds flows on), 33 only renames, 4 path
ends and renames, 6 a callee leaf of a newly Safe function (cascade:
`rd_kafka_cgrp_set_wait_resp`, `…clear_wait_resp`, `mk_socket_reset`,
`get_u32_at_offset` now summarised), 4 Unsafe functions lose a path-end leaf
and stay Unsafe. Six refusals not of those families disappear with them, all
removals: `compress_fragment`'s do-while condition (it calls the now-Safe
`get_u32_at_offset`), `mk_vhost_init`'s `nullable resource dir` (`if (!dir) {
… exit(EXIT_FAILURE); }`), the `mk_http_thread_create` rename, and three
state-dependent refusals (a variadic tail actual in `unittest_rd_crc32c`, an
indirect access in `rd_kafka_mock_cgrp_classic_elect_leader`, a loop
condition in `unittest_ssl`) in bodies whose assertion arms now end; those
three were classified by family, not traced one by one. No diagnostic of any kind is added except the
renames.

**Consultations** (`scripts/parity_census.py`,
`artifacts/parity-census-after.json`; de-duplicated rows, a header body
counted per unit): 1,519 statement path ends (`__assert_fail` 1,479 over 751
bodies, `abort` 18 / 14, `exit` 22 / 13), every one resolved `NoReturn`;
inline assembly 57 boundary rows over 53 bodies; no `cleanup`, SEH, computed
`goto`, label address or `asm goto` row in the corpus, the system route or
the slice. The census rows resolve one uncontracted callee to a `noreturn`
declaration: librdkafka's `rd_kafka_crash` (92 rows, 50 leaves on 40
functions, none blocked by it alone), `CG21-UNDECLARED-NORETURN-CALLEES`.

**Published property (projection coverage is not safety coverage).** The
twelve are `Safe` for custody on route G under the libc provider's declared
exits (a trusted assumption, constraint 28: `abort`, `exit` and
`__assert_fail` terminate the program) and the Clang reading of `noreturn`
(trust boundary 28). Nothing about the asserted conditions is claimed: a path
end discharges no obligation, it removes a path the program does not
continue.

## 5. Axes

| Axis (§8) | Covered | Stays refused (named) |
|---|---|---|
| Exit kind | terminating (declared + `noreturn`) | non-local, returns-twice, attribute-only, declaration-only |
| Position | statement (incl. statement-expression inner statements, glibc `assert`) | operand positions keep the continuation (sound, not a path end) |
| Callee identity | direct call resolved by Clang, agreeing references | shadowed, unresolved, disagreeing redeclarations, indirect |
| Preconditions | the contract's modes at the path-end node (liveness of borrowed actuals) | — |
| Boundaries | — | asm, asm goto, SEH, `cleanup`, computed `goto`, `&&label`, extension nodes |
| Route | G | T unchanged (its terminal-statement `NoReturnCall` stands) |

## 6. Lean

`QCore.CallExits` (`proof/QCore/CallExits.lean`), audited by
`AuditCallExits.lean` (12 theorems; axioms at most `propext`, `Quot.sound`):
`decision_vectors_hold`, `path_end_iff` (a path end exactly when contracted,
declared and resolved `noreturn`), `attribute_alone_not_path_end`,
`declaration_alone_not_path_end`, `path_end_no_exit`,
`path_end_rest_unchecked`, `path_end_checks_pre`, `refused_no_claim`,
`model_sound` (under the two premises — a no-return declaration is a
terminating exit; a non-local exit has no contract — a leak-free model
outcome is a leak-free run, over straight paths with acquisitions, releases
and calls of every kind), `nonlocal_path_end_hides_leak` (why the exit kind
must be declared), `boundaries_refused`, `boundary_table_refused`. Bound to
Rust by `call_exits::tests::cg21_lean_vectors_match` (`decisionVectors` =
`DECISION_VECTORS`, `boundaryTable` = `BOUNDARIES`). Not modelled: the
Clang-JSON reading (trust boundary 28) and the CFG builder's node shape (the
route-G controls cover it). No `cleanup` theorem: the card refuses `cleanup`,
so a "runs on every exit" theorem would describe nothing the Rust does.

## 7. Controls

`crates/qcore-c-gateway/tests/cg21_call_exits.rs` (10 tests, the production
`stdlib.qv` loaded byte for byte): glibc's expanded `assert(q); use(q)`
decided `Safe` (BEFORE contracts: the `__assert_fail` leaf); `exit` with a
live resource `Safe`, a leak on a returning path still `Unsafe`, a body whose
every path aborts `Safe`; a double release after a path end unreachable (the
same body with an ordinary call keeps it); a released actual at a no-return
call not `Safe`; a declared callee without the attribute, an attribute-only
callee, a merely named callee and a shadowed name all refused; `longjmp`
across a live resource and `_setjmp` refused by name, and declarations of
`longjmp`, `_setjmp`, `pthread_exit`, `__builtin_choose_expr` rejected by the
loader; `cleanup` (releasing on early return, and a scalar) refused by name;
assembly, `asm goto` and `__builtin_choose_expr` named; an operand position
keeps its continuation. Unit tests (`tests/unit/call_exits.rs`): the
decision table, the Lean binding, the boundary reader (split `& &` label
addresses against logical `&&`), the AST resolution. CG00's phase-3/4
controls (C09, C11, C12, E18, E20, K09) now assert the named refusals, their
BEFORE wording kept in comments, and the gateway unit control
`unrestricted_projection_retains_successful_siblings` takes a `do` without its
`while` as its parse failure (it used a computed `goto *`, which is now a
refusal inside its own function).

**Not met from the brief's evidence list**: "cleanup releasing on early
return Safe". A `cleanup` function receives `&p` (`void **`); releasing
through a pointer-to-pointer is outside v1 (only `resource_out` stands at
`ptr<mut, ptr<mut, u8>>`), and route G has no block scopes to place a
scope-exit call, so the card refuses the attribute by name rather than model
half of it. Registered as `CG21-CLEANUP-SCOPE-EXITS`.

## 8. Gate

Shared host: six, then eleven, cards at load 50–110, the disk at its 40 GB
floor. Final gate at `94bd56fb` on main `f0bbc0eb` (the tip was then rebased onto `f6336c9d`, CG28, with only `cargo check --workspace --all-targets`, exit 0: the matrix's derived `lean_audits` is CG28's 268 against 269 audit files, which the integrator's regeneration with its gate settles), run under the shared heavy lock (`scripts/gate-locked.sh`; the submodule step first, by hand); logs in this worktree's `private/cg21/gate7`; exits copied to
[`gate/`](gate/); summary in [`gate.json`](gate.json).

| Step | Command | Exit | Count / note |
|---|---|---|---|
| submodule | `git submodule update --init --recursive` | 0 | |
| fmt | `cargo fmt --all -- --check` | 0 | |
| tests | `python3 scripts/run_test_binaries.py` | **0** | 418 units, 431 summaries, **7,272 passed**, 0 failed, 11 ignored (`private/test-binaries/20261001T201309Z-1065311`) |
| clippy | `cargo clippy --locked --all-targets -- -D warnings` | 0 | |
| probe | `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | |
| follow-up probe | `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | |
| Lean | `python3 scripts/check_lean_audits.py` | 0 | **268 audits** (267 on main `f0bbc0eb` + `AuditCallExits.lean`) |
| Python | `python3 -m unittest discover -s scripts/tests` | 1, then **0** | 365; the one failure was the matrix's derived `lean_audits` (267 → 268), cleared by `build_matrix.py . --gate gate.json` |
| docs index | `python3 scripts/build_docs_index.py --check` | 0 | |
| provider check | `cargo check --manifest-path providers/Cargo.toml --workspace --locked --offline` | 0 | |
| libc provider tests | `cargo test --manifest-path providers/Cargo.toml -p quirin-provider-libc --locked --offline` | 0 | 2 passed |
| contract pins | `cargo test -p qcore-compiler --test contract_loader --test qcore_contract_corpus` | 0 | 10 + 4 passed (v1 declarations 259 → 262 on `f0bbc0eb`, 255 → 258 at the base) |
| lock | `scripts/pin.sh` (`quirin-audit pin` at `66910c10` and `046accf6`, then `pin --check` at both) | 0, 0, 0, 0 | three flexible libc rows; every crate 0 not in force, 0 stale |

Not run: `providers/package.py build` / `verify` (constraint 18) and the full
provider workspace tests. Only `quirin-provider-libc`'s contract text changed;
that crate is data only and is not packaged (the packages are the
`qcore-provider-*` executables). Owed to the integrator if the provider gate
is required for a contract-only change; `provider_tests` in `gate.json` is
carried (129).

Earlier attempts, retained and not used as evidence of a pass: the first full
suite at `6db1ce32` (`private/test-binaries/20261001T162038Z-1915279`): 411
units, 424 summaries, 7,184 passed, **2 failed**, 11 ignored, exit 101 — this
card's `legacy_c::tests::unrestricted_projection_retains_successful_siblings`
(it used a computed `goto *` as its parse failure; fixed in `400d9dfa`, the
test then passed alone) and constraint 16's
`cycle20_verifier_plugins::vp04_vp07_vp08_process_lifecycle_and_payload_budgets`
(`elapsed < 500 ms` under load ~55; it failed again alone on the loaded host,
untouched by this card and not weakened). A second full gate at `8d8d7587`
(`private/cg21/gate2`) ran out of disk while building the tests (`No space
left on device`, an `ld` bus error): it measured the disk, not the program;
its other steps are the exits above.

## 9. Follow-ups

Retired: **`CG21-CALL-EXITS-AND-BOUNDARIES`** (every DISCHARGE clause
holds: the cited refusals reproduced with the corrections above, PREDICTION.md
frozen before the AFTER, every moving function attributed, the absorbed entry
closed, Rust controls and audited Lean) and **`W43-LIBC-NORETURN`** (its
DISCHARGE: a v1 no-return clause the C gateway models as a path end, matching
Lean, then the declarations; re-measured: `exit` 22 leaves / 13 functions and
`abort` 11 / 10 at BEFORE, 0 / 0 at AFTER). Registered (sizes from the AFTER,
`scripts/update_ledger.py`):

- `CG21-UNDECLARED-NORETURN-CALLEES` — `rd_kafka_crash`: 92 consultations,
  50 leaves on 40 functions, 0 blocked by it alone; a reviewed
  `resource_no_return()` in its owner crate or a derived no-return summary.
- `CG21-NORETURN-OPERAND-POSITIONS` — 0 corpus sites; the path end in
  condition and value-connective plans.
- `CG21-CLEANUP-SCOPE-EXITS` — 0 corpus, system and slice sites; the brief's
  unmet evidence item.
- `CG21-OPAQUE-BOUNDARIES` — inline assembly 57 consultations, 7 corpus
  leaves on 3 functions and 1 on the system route; asm goto, SEH, computed
  `goto`, label addresses and setjmp/longjmp 0.

Out of scope and not tracked here: route T (its `NoReturnCall` for a
terminal statement is unchanged).
