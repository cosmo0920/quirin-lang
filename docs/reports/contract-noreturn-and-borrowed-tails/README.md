# CONTRACT-NORETURN-AND-BORROWED-TAILS

Wave 48, round 15. Contracts only: no route, rule or Lean module changes.
Two registered follow-ups are discharged: `CG21-UNDECLARED-NORETURN-CALLEES`
(gateway rule [11o](../../agents/gateway.md#constraint-11o)) and
`CG26-BORROWED-TAIL-DECLARATIONS` (rule [11w](../../agents/gateway.md#constraint-11w)).
Base: local `main` `ca3576dd4` (main did not move during the card).
Prediction frozen at `6279398f7`, before any contract was edited
([PREDICTION.md](PREDICTION.md)).

**Published property and scope.** Resource custody on route G, unchanged:
every acquired resource is released or handed on at every return. Two kinds
of trusted declaration are added. A call to `rd_kafka_crash` is a
terminating exit, so a call statement to it ends its path (rule 11o, which
also requires the Clang `noreturn` declaration). The trailing arguments of
five variadic callees are borrowed, so a tracked pointer passed there is read
as a borrowed place (rule 11w, T8). Nothing is claimed about format strings
or the asserted conditions.

**Result.** Corpus **648 / 9 / 4,177 → 649 / 9 / 4,176** (`bcc98d2d…` →
`450a9e54…`). The one move is `cb_debug_chunk_complete`, Unknown → Safe, the
predicted mover. There is no Safe → non-Safe move.

Leaves:

- The 92 `rd_kafka_crash` call statements CG21 counted end their path.
  Every one resolves `noreturn`.
- 34 more crash statements, in 26 functions, used to be refused at their
  member-chain `rk` actual. They are now admitted as ordinary calls that keep
  their continuation. That is sound, but it is not a path end.
- No function is freed by `rd_kafka_crash`.
- 157 of the 197 corpus tail refusals go.
- One leak is added inside a unit test that was already provisional Unsafe.
  By reading it is a real leak on the test's failure arm.

System and slice: no status move. 14 and 8 tail leaves go.

Ledger 696 → 696: both follow-ups are retired, and two residuals are
registered.

## 1. What changed

**`rd_kafka_crash`** (`providers/qcore-provider-librdkafka/contracts/librdkafka/memory.qv`).
It is the target of `rd_kafka_assert`. The declaration:

```qcore
extern fn rd_kafka_crash(file: ptr<const, u8>, line: i32, function: ptr<const, u8>, rk: ptr<mut, u8>, reason: ptr<const, u8>) -> unit
contract { … every argument resource_borrowed …; resource_return_none(); resource_no_return(); };
```

The body was read at `rdkafka.c:6640-6649`, which is identical at
`66910c10`, `046accf6` and the slice export `90978ac6`
(`scripts/compare_revisions.py`). On every path it:

1. prints to `stderr`;
2. calls `rd_kafka_dump0(stderr, rk, 0)` when `rk` is not NULL. That dump
   only reads: no lock at `locks` 0, and one `mtx_lock`/`mtx_unlock` pair
   inside `rd_kafka_curr_msgs_get`;
3. calls `abort()`.

No path leaves through `longjmp`, `pthread_exit` or a similar non-local exit.

The declaration (`rdkafka_int.h:69`) and the definition both carry
`RD_NORETURN`, which is `__attribute__((noreturn))` (`rdposix.h:61`). Rule
11o needs both the declaration and the attribute. The controls show that the
declaration alone is refused, and a call without the attribute is refused.

The declaration sits in `memory.qv`, the module where W46 and W47 added
librdkafka callees, so every selection that reads librdkafka's contracts
reads it. CG21's census found no other uncontracted callee that resolves
`noreturn` on any route.

**Borrowed tails.** `ensures normal resource_variadic_data_tail();` becomes
`ensures normal resource_variadic_borrowed_tail();` for these callees:

| Callee | Crate | Reading |
|---|---|---|
| `printf`, `fprintf`, `snprintf` | `quirin-provider-libc` (`libc/stdio.qv`, flexible) | C17 7.21.6.1/.3/.5: `%s`/`%p` read, `%n` stores through an `int *`, everything else reads a value; nothing of an argument outlives the call but the rendered text |
| `mk_print` | `qcore-provider-libevent` (`monkey/logging.qv`) | `mk_utils.c:199-261`, identical at both revisions: `va_start` → `vprintf(format, args)` → `va_end` |
| `flb_log_print` | `quirin-provider-fluent-bit` (`fluent_bit/logging.qv`) | `src/flb_log.c:1179-1300`, identical at both revisions: the list reaches only `vsnprintf` into the local `body` |

In each case no trailing pointer is released, retained or stored.
`sprintf` keeps the data tail: no analysed call of it has a tail actual that
the data tail refuses. `fcntl`, `open` and `prctl` are untouched; their tail
values depend on the command. No other callee with a declared tail has a
tail leaf on any route.

**`%n`.** The brief asked how the clause treats `%n`, which writes through an
`int *`. The borrowed tail reads that actual as a `Borrowed` place. That place
takes and transfers nothing, exactly as `snprintf`'s written-through `s`. A
borrowed tail could certify something false through `%n` only if a custody
decision rested on the count it rewrites. The control
`a_percent_n_write_through_an_aliased_scalar_is_never_certified` shows none
does, against the fixture borrower and against all five production callees:

- `&x` (T7) forgets `x` after the call, so the leak on the `x != 0` arm is
  reported.
- `px = &x` is itself refused by the projection.
- `*cnt` of a pointer formal is not a fact the projection holds.
- Without the branch on the count, the `&x` and formal shapes are admitted.

No `%n` conversion occurs in the audited printf-family call sites. So the
libc change is not refused. `stdio.qv`'s comment now says this, and also that
format/argument mismatch is undefined behaviour no custody statement covers.

**Pinning.** [`scripts/pin.sh`](scripts/pin.sh) ran `quirin-audit pin` for
the six functions at the read-only `66910c10` source and at this card's own
fresh `046accf6` clone (`scripts/prepare-attempt.sh attempt-1`, exit 0, under
`private/w45-fb/contract-noreturn-and-borrowed-tails/`). Every pin step exits
0.

`pin --check`:

- **Per crate** (libc, Fluent Bit and librdkafka whole; `mk_print` in
  libevent) exits **0** at both revisions.
- **Whole lock** exits 1 at both. The only line out of force is the
  tip-tracked `mk_vhost_set` (CONTRACT-PIN-TIP-MODE). By design its Fluent
  Bit-vendored sites differ from the Monkey tip, and
  MONKEY-VHOST-PROPERTY-CUSTODY's `artifacts/pin/check-*.exit` record the
  same 1.

[Logs](artifacts/pin/). `providers/contracts.py sync` then `check`: exit 0.

**Deliberate pin updates:**

- `compiler/tests/contract_loader.rs`: v1 declarations 264 → 265, and
  `rd_kafka_crash` joins the neutral-mode table;
- `compiler/tests/qcore_contract_corpus.rs`: `("librdkafka/memory.qv",
  "rd_kafka_crash", 8)`;
- the hand-written librdkafka twin
  (`compiler/tests/fixtures/contract_inherit_qcore_provider/librdkafka_memory_hand_written.qv`)
  gains the declaration.

Replacing a tail clause changes no clause count.

**Rule text.** No rule was added. Rule 11o's list of premises names
`rd_kafka_crash`, and rule 11w's names the callees that now declare the
borrowed tail. The contract specification is unchanged: no clause's reading
changed.

## 2. Reproduced claims

| Claim (registered) | BEFORE on `ca3576dd4` | Standing |
|---|---|---|
| `rd_kafka_crash`: 92 statement consultations, 50 leaves on 40 functions, none alone (CG21) | 92 / 50 / 40 / 0 | reproduces |
| `cb_debug_chunk_complete` blocked by `printf`'s tail alone (CG26) | yes | reproduces |
| tail leaves `fprintf` 126 / 78, `snprintf` 55 / 33, `mk_print` 17 / 9, `printf` 3 / 3 (CG26 at `b6a3dc42`) | 124 / 77, 55 / 33, 15 / 8, 3 / 3 | re-measured at this base; the statement counts (120, 55, 20, 4) are unchanged, so later cards removed the leaves of two functions, not statements; not a correction |
| system: 10 `flb_log_print` and 1 `snprintf` tail statements (CG26) | 10 and 1 (16 and 2 leaves over the configurations' rows) | reproduces |

The BEFORE totals equal the brief's:

| Route | Totals | Digest |
|---|---|---|
| corpus | 648 / 9 / 4,177 | `bcc98d2d…` |
| system | 44 / 2 / 188 | |
| slice | 29 / 1 / 238 / 2 Pending | |

The system and slice raw digests embed the worktree path
([totals](artifacts/totals-before.txt), merged by
`scripts/merged_totals.py`).

## 3. Prediction against outcome

| Prediction (frozen) | Outcome |
|---|---|
| Corpus: exactly one move, `cb_debug_chunk_complete` Unknown → Safe; at most 2 more; no Safe → non-Safe | **exactly that one**; none of the allowance used |
| All 50 `rd_kafka_crash` leaves go; every one of the 92 statements a `noreturn` path end; no refusal at a call | exactly: 50 go; **92** statement path ends (`scripts/path_ends.py`), all `PathEnd`/`NoReturn`, in 45 bodies (the 40 functions, and 5 header inline bodies with no reported row); 0 `no-return contract … not established` |
| Other leaves of the 40 functions may go; at most a refused `rk` added | **Census flaw.** 26 more functions move. Their 34 crash statements used to be refused at a member-chain actual (`(rkb->rkb_rk)`, an `indirect memory access`) before the uncontracted call was reached, so the census never saw them. With the contract in force, rule 10k(3)'s typed call-actual reader admits them. Rule 11o applies a declared call reached outside its statement hook as an ordinary borrowing call with its continuation. That over-approximates C's paths and is never a path end (`CG21-NORETURN-TYPED-ACTUAL-STATEMENTS`). Over the 66 functions: −38 indirect accesses, −7 and +21 unresolved identities (the chain roots `rkb`, `rktp`, `rkcg`, `rkt`, `msetw`). Every leaf is attributed to an `rd_kafka_crash` line by `scripts/attribute_other.py`, except in 2 functions. There, the next statement (`rkb->rkb_fetching = 0`, `rktp->rktp_committing_pos = …`) also loses its unresolved identity, because its root no longer passes a refused statement (rule 11d) |
| `rd_kafka_crash`'s own row stays Unknown, not refuted | Unknown (`rd_kafka_dump0` uncontracted); no refutation |
| Tail leaves: 140–185 of 197 go | **157** (`fprintf` 97, `snprintf` 44, `mk_print` 15, `printf` 1); 40 left |
| No leak or violation added (at most a released-identity candidate) | **One leak added**, at `:1589` in `ut_sasl_oauthbearer_oidc_assertion_parse_from_file`, already provisional Unsafe (§4) |
| System: no move; the T8- and chain-shaped `flb_log_print` statements lose their leaves | no move; 14 of 16 leaves go; `flb_coro_destroy`'s pointer arithmetic keeps 2, the `snprintf` cast keeps 2 |
| Slice: no move; `flb_lib_config_file`'s and `do_start`'s leaves go, `flb_coro_destroy` keeps its | no move; all 8 tail leaves go. The prediction's "keeps its" was empty: `flb_coro_destroy` held no tail leaf in the slice report |
| Round-12 tip probe unchanged | unchanged (§4) |

## 4. Measurement and attribution

Release `quirin-audit`, built in this worktree: `495d1e8e…` BEFORE,
`d935d74a…` AFTER (the contract texts are embedded). Corpus arbiter
`66910c10` `--jobs 4`; system route and tail-pause slice
([`scripts/measure.sh`](scripts/measure.sh)).

| Run | Corpus | System | Slice |
|---|---|---|---|
| BEFORE | 648 / 9 / 4,177 `bcc98d2d…` | 44 / 2 / 188 (normalised `36e40330…`) | 29 / 1 / 238 / 2 P (normalised `17b1cfa1…`) |
| AFTER | **649 / 9 / 4,176** `450a9e54…` | 44 / 2 / 188 (normalised `f96c3132…`) | 29 / 1 / 238 / 2 P (normalised `f3a368f4…`) |

**Every moving function** ([corpus](artifacts/after/compare-corpus.json),
[system](artifacts/after/compare-system.json),
[slice](artifacts/after/compare-slice.json), by `scripts/compare.py`):

- Corpus, 165 functions:
  - 1 Unknown → Safe (`printf`);
  - 66 Unknown → Unknown by `rd_kafka_crash` (40 that lost its leaf to a
    path end, and the 26 above);
  - 64 by `fprintf`, 25 by `snprintf`, 1 by both, and 8 by `mk_print`;
  - 1 Unsafe → Unsafe by `fprintf`.
- System, 12 rows: `flb_log_print`.
- Slice, 4 rows: 2 `flb_log_print` and 2 `fprintf`.

**`cb_debug_chunk_complete`** (`mk_server/mk_http_parser.c`).
`printf("…%zu…'%s'…", in_len, buf)` receives `buf`, a live `mk_mem_alloc`
buffer, which `mk_mem_free` releases after the call. T8 reads `buf` as a
borrowed place, so the body's only leaf goes. The result is `Safe` relative to
`printf`'s declaration (a trusted libc premise, constraint 28) and the other
premises it already rested on.

**The added leak, classified by reading.** It is in
`ut_sasl_oauthbearer_oidc_assertion_parse_from_file`
(`rdkafka_sasl_oauthbearer_oidc.c:1574`), which was already provisional
Unsafe on the `:1599` and `:1603` leaks that W47-CALLEE-CLOSURE-R2
classified. The new leak, acquired at `:1589`, is real:

```c
result = rd_kafka_oidc_assertion_read_from_file(tempfile_path);   // owned, nullable
RD_UT_ASSERT(result != NULL, …);
RD_UT_ASSERT(!strcmp(result, test_jwt), "… '%s', got '%s'", test_jwt, result);
```

`RD_UT_ASSERT` (`rdunittest.h:93-106`) prints through `fprintf` and returns 1
on failure. It does not release `result`, unless
`rd_unittest_assert_on_failure` aborts first. Before, that failure arm was
refused at the `fprintf` that receives `result` in its tail. Now the arm is
read, and `result` leaks on it. This is a test-code candidate on a failure
path, never an upstream defect claim. The status does not move.

**Not moved by the tails.** 40 corpus tail leaves remain in 23 functions,
none blocked by them alone ([census](artifacts/census-after-corpus.json)).
They come from 34 statements:

- 19 whose trailing actuals are plain names, but whose function's lexical
  declarations are unreadable (`lexical_readable: false` in their parity
  rows), which turns T8 off;
- 14 with a cast, arithmetic or a conditional actual;
- 1 assignment form.

8 of the 9 assignment-form `x = snprintf(…)` statements were admitted. The
system keeps 2 `flb_log_print` leaves (`flb_coro_destroy`'s
`((char *) coro) + sizeof(...)`) and 2 `snprintf` leaves (`(void *) src`).
These are registered as `CG26-TAIL-RESIDUAL-ACTUALS`.

**Round-12 probe** (`scripts/tip-probe.sh`, the integrator's
`tip-contracts.sh` on a private copy of the probe tree; [rows](artifacts/tip-probe/rows.json)):

| Copy | Function | BEFORE | AFTER |
|---|---|---|---|
| tip (`21a2c190`) | `mk_vhost_set` | Unknown: uncontracted `mk_vhost_set_property` (`:533`) | the same |
| tip (`21a2c190`) | `mk_vhost_set_property` | Unknown: two indirect accesses | the same |
| master | `mk_vhost_set` | Unknown: the `va_end` protocol refusals and `mk_vhost_set_property` | the same |
| master | `mk_vhost_set_property` | Unknown: two indirect accesses | the same |

No function in either report moves: neither body calls a callee this card
declares. The reports differ only in the embedded contract digests.

## 5. Controls and Lean

[`crates/qcore-c-gateway/tests/contract_noreturn_borrowed_tails.rs`](../../../crates/qcore-c-gateway/tests/contract_noreturn_borrowed_tails.rs)
has 8 tests over the production modules, loaded byte for byte, beside the
CG00 fixture contracts:

- **The clauses as loaded.** The five callees carry the borrowed tail, which
  implies the data tail. `sprintf` carries the data tail only.
  `rd_kafka_crash` is a no-return callee.
- **`rd_kafka_crash` ends its path.** Two shapes are `Safe`:
  - `rd_kafka_assert` as the preprocessor writes it, over a nullable
    acquisition, with the non-null release after it;
  - a release before the crash on its arm and a release after it.

  With the data-tail texts and no `rd_kafka_crash` declaration, both are the
  base's uncontracted call.
- **Its `noreturn` declaration is needed.** A prototype without the
  attribute is refused with `no-return contract … not established`, and is
  never `Safe`.
- **A released `rk` at the crash is never `Safe`.**
- **A tracked `char *` in each callee's tail is borrowed.** The call is
  `Safe`, and the same body under the data tail keeps the tail refusal.
- **Dropping it leaks, and a released one is never `Safe`.** This is C04.2's
  caller-side rule.
- **`sprintf` keeps the tail refusal.**
- **`%n` (§1).**

A non-null `q_alloc` before the assertion makes the crash arm dead. Without
the declaration, that body then hits the gateway's known hard error for a
dead uncontracted call (`retains an unknown instruction`,
[remaining-c03-nfr §5](../remaining-c03-nfr/README.md)). That is why the
assertion control uses the nullable acquisition.

**Lean.** No new module: no clause has a new reading. Rule 11o's
`QCore.CallExits` and rule 11w's `QCore.Variadics`
(`borrowed_tail_is_the_identity`) already model both clauses. Audits stay at
282.

## 6. Gate

GATE-PENDING

## 7. Follow-ups

**Retired** (`scripts/update_ledger.py`; 696 → 696 with the two
registrations):

- **`CG21-UNDECLARED-NORETURN-CALLEES`.** Its DISCHARGE is a reviewed
  `resource_no_return()` declaration in the owner crate, read against the
  body at every audited revision and measured. That holds: 50 BEFORE leaves
  and 0 AFTER, and 92 path ends. No function was blocked by the callee alone,
  and none moved.
- **`CG26-BORROWED-TAIL-DECLARATIONS`.** Its DISCHARGE is, per callee, a
  reviewed provider declaration in libc, Fluent Bit and Monkey, with the pins
  and the provider gate, measured. That holds for all five callees.

**Registered:**

- `CG26-TAIL-RESIDUAL-ACTUALS`, with the sizes in §4.
- `CG21-NORETURN-TYPED-ACTUAL-STATEMENTS`: the 34 crash statements in 26
  functions that keep their continuation (§3).

**Not registered:** the dead-uncontracted-call hard error seen while writing
the controls. It is already recorded as a pre-existing boundary in
remaining-c03-nfr §5.
