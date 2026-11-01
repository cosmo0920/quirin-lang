# MK-EVENT-CELL-ARRAY-SLOTS

Card authority: ledger entry `MK-EVENT-CELL-ARRAY-SLOTS`
(`docs/reviews/r0_3/acceptance_matrix.json` `named_followups`, registered by
[MK-EVENT-DESCRIPTOR-QUANT](../mk-event-descriptor-quant/README.md)). Its
DISCHARGE reads: "the gate admits a constant-index element of an integer
array member at a withheld declaration's borrowed position (as
W44-INDIRECT-CALL-ARGUMENT-ACCESS admits a typed member read), with fixtures,
measured". Base `e589ee5a7`.

The prediction was committed before any code (`ba1fb8d7c`) and amended before
any measurement: [PREDICTION.md](PREDICTION.md). The vocabulary is the
approved one (§C04.6.2; USER DECISION 2026-11-01, "I approve the changes of
D1-D5."). No clause spelling is added.

## What changed

Gateway rule 38a ([rule text](../../agents/rules/38a.md)) lives entirely in
`crates/qcore-c-gateway/src/legacy_call_argument_access.rs`. Both of its
parts apply only at a withheld
declaration (rule 12t) that carries a descriptor-cell state:
`mk_event_timeout_create`, `mk_event_timeout_destroy`,
`mk_event_channel_create` and `mk_event_channel_destroy`.

1. **The element actual** (`element_read`). It is asked at a `Borrowed`
   position after every older route refused. Its forms are `B->F[K]`,
   `S.F[K]`, `&B->F[K]` and `&S.F[K]`, where:
   - the base is typed as W44 types the base of `B->F`;
   - `F` is an `INTEGER-TYPE F[N]` member of a complete, single-definition,
     non-union record (`integer_array_length`: one declarator, one
     dimension, a literal length, no `*`, no `volatile`/`_Atomic`). It reads
     the record's definition from the unit's tokens after the declaration
     reader's completeness check. It is not a class of
     `typed_field_access.rs`, because
     `docs/reports/libc-reading-site-evidence/bindings.json` pins that file's
     bytes whole and frozen replay proofs carry the manifest's digest. The
     first card gate failed on that pin, and the class was moved here;
   - `K` is a canonical decimal literal (`canonical_decimal`) with `K < N`.

   The actual is handed to W44 as the member read of its base, with the same
   bases and names. So `B`'s typed read (`IndirectScalarAccess`), the second
   obligation (`ArgumentReads`) and the accusation of a released base apply
   unchanged.
2. **The minted name** (`AdmittedArgument::custody_contract`). Rule 12t
   classifies a withheld call a second time, under
   `qcore_retained__<callee>`. W44's probe found its admitted actuals only
   under the plain name. They left the gate in the outer classification, and
   the inner one refused every actual that no older route exempts.
   - This is why the element alone was not the cause. Every Fluent Bit
     channel call also hands a loop member read through a base its body
     binds: `sched->evl`, `config->evl`, `thi->evl`.
   - Under the rule-38 tests' caller formals, an older route had exempted
     those actuals in the inner pass.
   - The minted name is now matched, but only for its own callee and only at
     a cell-bearing declaration.

The obligations are lowered before the call exactly as before. Only the inner
gate's exemption is new.

**Narrower than the DISCHARGE text.** That text says "a withheld
declaration". The admission covers only cell-bearing withheld declarations.
Matching the minted name at every withheld declaration would admit W44 reads
at `mk_event_add`, the collector setters, msgpack and librdkafka calls. That
is a separate change with its own rows, so it is registered as the follow-up
`RETAINING-CALL-MINTED-NAME-ACTUALS`. The card's goal, rule 38 deciding Fluent
Bit's channel slots, needs only the cell-bearing calls.

**Lean.** `QCore.CellArraySlots`, audited by `AuditCellArraySlots.lean`,
holds 33 theorems. It uses only `propext` and `Quot.sound`, with no `sorry`
and no project axiom. It covers:

- **the admission predicate:**
  - an admitted element is in bounds, at a `Borrowed` position and at a
    cell-bearing withheld declaration;
  - each refused class never admits: a custody position, a data tail, a
    variable, negative, non-canonical or out-of-bounds index, a pointer
    member, a non-integer element, no cells, a function-map callee, an
    untyped base;
- **the soundness law relative to `QCore.CallArgumentAccess`:**
  - `an_admitted_element_reads_as_the_member_read`;
  - therefore it is a W44 candidate whose base is obligated;
  - a released base refuses and is accused (W44's theorem, reused);
  - a statement whose only actual is a refused element keeps the refusal;
- **the minted name:** it matches only its own callee, and never without
  cells;
- **the switch:** off refuses every cell call, a refused element never
  reaches rule 38, and off never decides what on does not;
- **non-vacuity:** `&sched->ch_events[0]` and `[1]` are admitted, `[2]` and
  the same element at `mk_event_add` are refused, and the creation statement
  lowers to the reads and then the call, adding no diagnostic over a live
  base.

**Tests.** `crates/qcore-c-gateway/tests/cell_array_slots.rs` holds 16
tests. Each asserts the complete diagnostic vector, and every `Safe`
certificate is rechecked. The fixtures use the provider's own
`monkey/event_loop.qv` with a record this body allocates:

- a channel created into `&c->ch[0]`, `&c->ch[1]` and destroyed through
  `c->ch[0]`, `c->ch[1]` is `Safe`;
- with the ends swapped, rule 38 refuses: the cells escape and the owner's
  release refuses;
- `flb_sched_destroy`'s shape, over a caller's record and its loop member,
  reaches rule 38 and refuses on its guard;
- an element read after `ev_free(c)` is `Unsafe` (`use_after_release`);
- the gate refusal is kept for:
  - a variable index;
  - an out-of-bounds index;
  - a negative index;
  - an octal index;
  - a pointer member;
  - `double` and plain-`char` elements;
  - a consumed position of a cell-bearing declaration;
  - a function-map callee;
  - `mk_event_add`, which is withheld but carries no cells;
- with `QCORE_GATEWAY_RULES_OFF=38`, the admitted cycle refuses with "rule 38
  is switched off" and no gate refusal;
- a record passed whole beside its member address never keeps the gate's
  refusal (`MK-EVENT-CELL-WHOLE-AND-MEMBER-ACTUALS`, below). The shapes are
  `c` at one call with `&c->event` at another, and Fluent Bit's `c` whole
  beside `&c->ch[0]` in one call. The assertions do not depend on rule 38's
  wording, so they hold before and after MK-EVENT-FIRST-MEMBER-CELL.

The unit test `element_reads_are_recognised_in_bounds_only` covers the
recogniser and the member class. At base `e589ee5a7`, every one of these
fixtures was the gate refusal (measured in the worktree before the change).

## The channel operations that now reach rule 38

The bodies were read at the three analysed revisions: 66910c10 (corpus),
046accf6 (system) and 34155955 (slice). At 046accf6 and 34155955 the
enclosing functions are byte-identical. At 66910c10 the same calls sit at
other lines. The calls with element actuals in the analysed rows are listed
below. The table is the reading of those bodies under the rule, not a
measurement; the integrator's measured leaves are the evidence.

- **The gate.** Each call is expected to pass the gate. Its records'
  definitions carry no nested brace, bitfield or initialiser in the header
  text, which are `record_body_refusal`'s triggers. Each slot member is
  `flb_pipefd_t f[2]`, which is `int` on Linux.
- **The row.** Each row is expected to stay `Unknown`, for the reason given.

| Function (unit) | Calls | What rule 38 and the row then say |
|---|---|---|
| `flb_sched_create` (`flb_scheduler.c`) | channel_create :794 | The channel is created into `sched`, which this body allocates and returns, so its cells escape at the exit. The failure arm calls the uncontracted `flb_sched_destroy`. |
| `flb_sched_destroy` (`flb_scheduler.c`) | channel_destroy :856, timeout_destroy :843 | The guard `sched->event.fd == sched->ch_events[0]` is undecided: the creation is in `flb_sched_create`. |
| `flb_engine_start` (`flb_engine.c`) | channel_create :1168, :1237, :1246 | The creations go into the caller's `config` and escape at the exit. The `ret == -1` tests are not the class's `== 0` / `!= 0` witness test. |
| `flb_engine_shutdown` (`flb_engine.c`) | channel_destroy :1711, :1718 | The guards are undecided: the creations are in `flb_engine_start`. |
| `flb_output_init_all` (`flb_output.c`) | channel_create :1545 | The `data` actual `ins` is a record whose first member is the event: rule 38 (1), `MK-EVENT-FIRST-MEMBER-CELL`. |
| `input_instance_channel_events_init` (`flb_input.c`) | channel_create :1380 | The same `ins` case, and `flb_input_event_loop_get` is uncontracted. |
| `input_thread_instance_create` (`flb_input_thread.c`) | channel_create :262, :275, :287 | The creations go into `thi`, which this body allocates and returns. The failure arms call the uncontracted `input_thread_instance_destroy`. |
| `input_thread_instance_destroy` (`flb_input_thread.c`) | channel_destroy :197 | The guard is undecided. The other three channels' ends are closed with `close` at the custody position and keep their refusal. |
| `flb_create` (`flb_lib.c`) | channel_create :209 | The `data` actual `ctx->event_channel` is a pointer read, not a key. |

`output_thread`, the output-thread pool setup and `downstream_worker_thread`
carry the same shapes, but they are in no measured row.

Every row stays `Unknown` because of cross-function lifetime
(`FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME`), first-member actuals
(`MK-EVENT-FIRST-MEMBER-CELL`), and the rows' unrelated leaves. The rows are
listed in PREDICTION.md with one leaf each that stays.

## Measurement

The integrator runs the measurement from the main checkout. The worktree's
isolation guard refuses command lines naming the prepared inputs there. The
command is in the card's reply, and its record is added here at landing.

## Card gate

The gate ran as `python3 scripts/gate_scope.py --base e589ee5a7 --out
private/gate-2` on `6066c25f0`, the last code commit. Only this report, the
amendments file and PREDICTION.md changed after it. The scope was the full
gate, because the diff reaches `qcore-c-gateway`: every Lean audit, no
provider gate.

| Step | Exit | Counts |
|---|---|---|
| submodules | 0 | 1 checked |
| fmt | 0 | |
| clippy | 0 | |
| Rust suite | 0 | 591 units; 9,011 passed, 0 failed, 15 ignored, 105 filtered |
| `reevaluation_probe` | 0 | |
| `reevaluation_followup` | 0 | |
| Lean audits | 0 | 400 |
| Python suite | **1** | 577 tests, 1 failure |

**The Python failure.** The one failure is
`test_build_matrix_sections.test_the_derived_facts_are_recomputed_rather_than_carried`.
The acceptance matrix's derived `lean_audits` is 399, and the tree now gives
400, the new `AuditCellArraySlots`. The integrator regenerates the matrix at
landing.

**The first gate run** (`private/gate` on `7dc756613`) exited 1. The Rust
suite had 4 failures and the Python suite had the same 1 failure. The four
Rust failures were `libc_reading_site_evidence` (3) and `reading_site_domain`
(1): "artifact reference mismatch: crates/qcore-c-gateway/src/typed_field_access.rs".
The member class was moved out of that pinned file (`6066c25f0`), and both
binaries pass in the second run.

**Logs.** The gate logs are under `private/gate-2/` in the worktree. The
console logs are outside it, at
`/tmp/claude-1000/-media-Data-GitHub-quirin-lang--claude-worktrees-launch-cards-clean-worktrees-5365ff/6fcd7fbd-1cae-4a40-a37d-5bce071c02c6/scratchpad/cell-array-slots-gate-2-console.log`
and `…/scratchpad/gate-console.log` for the first run. The second file also
carries another card's lines, because the scratchpad is shared.

## Observations and follow-ups

- **`MK-EVENT-CELL-WHOLE-AND-MEMBER-ACTUALS`** (registered by
  MK-EVENT-FIRST-MEMBER-CELL, not on this card's base). Take a function that
  passes a caller's record `c` whole to one cell call. A member address
  `&c->event` at another cell call kept the access gate's refusal. That
  refusal is this card's minted-name gap.
  - Measured in the worktree on the whole-then-member fixture: with rule
    38a's match switched off, both orders give `indirect memory access is
    outside the resource projection`. With the match on, neither does, and
    rule 38 answers.
  - The fix is inside rule 38a's scope by construction, so no channel row
    still refuses because of it. Fluent Bit's `ins` whole beside
    `&ins->ch_events[0]` passes the gate. On this base, rule 38 then refuses
    `ins` until the first-member reading lands.
  - The entry is not in this base's matrix, so these amendments cannot
    retire it. **The integrator may retire it at landing**, citing this
    section and the test
    `a_record_passed_whole_never_keeps_the_gate_at_its_member_address`.

- **`RETAINING-CALL-MINTED-NAME-ACTUALS`** (registered). At every withheld
  declaration without cells, the W44 admission of
  FB-EVENT-LOOP-RETENTION-AND-REGISTRATION (`RETAINING-CALL-MEMBER-ACTUAL`)
  takes effect only for actuals an older route exempts again in rule 12t's
  inner classification. Examples are `mk_event_add(loop, c->ch[0], …)` and
  `flb_input_set_collector_event(in, cb, ctx->ch_manager[0], config)` in
  `in_tail_init`.
- **A parenthesised base.** `&(c)->ch[0]` leaves the gate, but rule 38 does
  not key it (`descriptor-cell-undecided`). This refuses, and Fluent Bit does
  not spell it, so it is recorded here only.
