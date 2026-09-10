# COMPOSE-CONCURRENCY — a module that spawns and prints (wave 21)

Card brief: *"The emitted C of the thread examples carries compiler-synthesized
debug prints, all on the single `QCORE_IO_WRITE` route, but the SOURCE cannot
spell a debug print of its own. Compose `source.std-io.v1` (and `std.logger`)
with the **checked task route**, so the four `examples/threads` programs can
express source-level logging — `first_result |> render |> log info`, prints
mirroring what the trace reports — beside the synthesized lines, on the same
single emission route."*

The last of the two pairs `STD-MODULES` named and `STD-IO-COMPOSE` deferred
("strictly larger than this card — a task worker is a separate checked model
with its own ledger"), carrying `STD-FMT-BUFFER-READ` with it since
`STD-FMT-BORROW`. Base commit: main `45f6c74c`.

**What landed.** A new route `source.std-io-tasks.v1`, built the way
`STD-IO-COMPOSE` built its own: the task half is the task route's **unedited**
lowering and the **unedited** `check_task_module`, the line half is
`source.std-io.v1`'s own grammar and its own line renderer, and one diagnostic
code, `Q050`, owns exactly the rules that exist because the two meet. All four
published examples now say something of their own, and all five of
`examples/threads`' tests are extended rather than weakened, with the byte
comparison now **per stream**.

**The v1 boundary is wider than the brief's conservative reading, and it is a
measurement rather than a choice.** A source line may stand **anywhere in the
parent's body**; a line in a **worker** body is refused by name at `Q050`. §3
gives the measurement: every write an emitted task program performs is performed
by the parent's thread, so every parent point is join-ordered and the brief's
"before any spawn, between joins, after all joins" is subsumed by it.

Three pinned decisions moved, each by name (§2). 31 audited Lean theorems in a
new `QCore.ComposedTasks`.

---

## 1. What is admitted, in one program

```qcore
import std.concurrency;
import std.io;
import std.logger;
import std.fmt;
concurrency checked;

fn worker(value: i32) -> i32 { return value; }

fn main() -> i32 effects { io, throws<SpawnError>, spawn, join, may_block } {
    let first_value = 42;
    let second_value = 7;
    thread scope workers {
        let first = spawn workers worker(copy first_value)?;
        match spawn workers worker(copy second_value) {
            Result::Ok { value: second } => {
                log info dec 2;                     // the creation decision
                let second_result = join move second?;
                let first_result = join move first?;
                first_result |> log info dec;       // the brief's sentence
                second_result |> log info dec;
                second_result |> print dec;         // mirroring the trace
                return second_result;
            }
            Result::Err { error: failure } => {
                log warn dec 1;
                let first_result = join move first?;
                first_result |> log warn dec;
                return first_result;
            }
        }
    }
}
```

That is `examples/threads/two_workers.qcore` after this card, and it writes:

```
stdout                        stderr
QCORE_TASK_SPAWN 1 0          [INFO] 2
QCORE_TASK_SPAWN 2 0          [INFO] 42
QCORE_TASK_RESULT 2 I32 7     [INFO] 7
QCORE_TASK_JOIN 2
QCORE_TASK_RESULT 1 I32 42
QCORE_TASK_JOIN 1
7                             <- the source's own `print`
QCORE_TASK_RESULT 0 I32 7
```

**No reserved word was added** (§7). **No synthesized line moved** (§6.1).

## 2. The three pinned decisions, moved by name

### 2.1 `TRACE-STDIO`'s "a source `print` in a threaded module is measured impossible"

That card measured the route split — `source.std-io.v1` selected last, so a
module that both spawns and prints stays with the more specific task model and
keeps its refusal — and pinned **both spellings** of the attempt in
`compiler/tests/thread_examples.rs::a_threaded_module_cannot_carry_a_source_print`,
"converting the reason from an omission into an asserted decision: if route
selection ever changes, that test fails rather than this report quietly becoming
false."

This card changes route selection, so **that test failed and was rewritten, not
deleted**, and the pin is now three tests rather than one, in the precedent's own
spirit:

| Test | What it now records |
|---|---|
| `a_threaded_module_carries_a_source_line_in_the_parent` | The **exact module** `TRACE-STDIO` wrote — `io` in the effect row, `first_value \|> print dec;` beside a `spawn` — compiles, on `ComposedStdIoTasksV1`. |
| `a_worker_body_cannot_carry_a_source_line` | What is refused *instead*, at `Q050`, with the interleaving ground asserted in the message. |
| `a_spawning_module_that_does_not_compose_keeps_the_task_routes_refusals` | The measurement's *other* half, still true: a spawning module that declares `io` and writes nothing is not composing, and keeps `Q020` byte for byte. |

`TRACE-STDIO` §4.1's table therefore reads, after this card:

| Attempt | `TRACE-STDIO` | Now |
|---|---|---|
| `effects { io, … }` + `first_value \|> print dec;` in the parent | `Q020` | **accepted**, `ComposedStdIoTasksV1` |
| the same without `io` in the row | `Q500` | `Q050` — the composed effect rule, naming `io` |
| `effects { io, … }` with **no** line anywhere | `Q020` | `Q020`, unchanged |
| a line inside a **worker** body | (not measured) | `Q050`, the placement rule |

And §4.1's conclusion — "all 4 of 4 published examples are verification
machinery … by measurement rather than by choice" — is superseded: 4 of 4 now
carry source lines too, and §4 of this report is the per-example disposition.
`TRACE-STDIO`'s §4.1 is **not edited**: it was true of the tree it measured, and
this report supersedes it rather than rewriting an earlier card's archive
(standing constraint 21).

### 2.2 The route-selection exclusion `STD-IO-COMPOSE` pinned

`STD-IO-COMPOSE` §4's decision table ends with three rows — prints **and**
spawns, extern-calls **and** spawns, all three — each "task route, refuses
`Q600`; selected earlier; unchanged", and its selector comment recorded them as
a deliberate exclusion. **This card lifts exactly one of the three and pins the
other two, each decided honestly rather than by convenience.**

Selection order, after this card:

```
composed epoch → epoch → mutex → COMPOSED TASKS → tasks
                                → composed std-io/extern → extern → standard IO → sequential
```

| Module uses | Route | Why |
|---|---|---|
| spawns only | task route | the conjunction is false; falls through **byte for byte** |
| `print`/`log`/`flush` only | `StandardIoV1` | same, in the other direction |
| imports `std.io`/`std.logger`/`std.fmt` but writes no line, and spawns | task route | importing is not using — `has_source_std_io` was already exact, and the emitted C is byte-identical (pinned) |
| **prints and spawns** | `ComposedStdIoTasksV1` | **this card** |
| **extern-calls and spawns** | task route, `Q600` | **not lifted.** The extern composition is a straight-line union of two models whose rendered numbers are *declared at the site*; putting it beside a branching concurrent model whose numbers are *computed* is a third join with its own argument to make, and neither existing join states it. Pinned. |
| **prints and extern-calls and spawns** | task route, `Q600` | **not lifted**, for the same reason, and pinned separately so the two exclusions cannot be read as one |
| an epoch scope, or a mutex generation, beside a line | that route's own refusal | selected earlier; unchanged |

The selector is the conjunction
`has_source_tasks ∧ has_source_std_io ∧ ¬has_source_extern` and nothing else, so
the last two rows are structural rather than tested into place — and the tests
pin them anyway (`extern_calls_beside_a_spawn_keep_the_task_routes_refusal`).

**"A module using one feature set keeps its route byte for byte" is structural
here too, and more strongly than in `STD-IO-COMPOSE`.** That card gave both
lowerings a skip list whose empty case *is* the old entry point. This one has no
second lowering at all: `lower_source_tasks` is `lower(…, composing = false)`,
`emit_task_c11` is `emit_task_c11_with_source_lines(…, &[])`,
`execute_task_module_with_worker_allocations` is `execute_with_source_lines(…, &[])`,
and `compile_task_source` is `task_route(…, composing = false)`. There is
nothing for a task-route module to drift onto. The observable consequence is
pinned anyway: `importing_is_not_using_and_the_task_route_is_kept_byte_for_byte`
compiles the same program with and without the three imports and asserts the
emitted C is equal.

### 2.3 Observation semantics under threads

`STD-MODULES` established the per-stream byte comparison, and it is well-defined
on every earlier route because emission there is single-threaded. Under threads
that is the question this card had to answer, and the answer is a measurement of
`crates/qcore-core/src/task_c_emit.rs`, not a policy:

* `qcore_buffer_event` writes **immediately** only when `context->identity == 0`
  — the parent. A worker appends to its own **private** `events` array
  (`:509-518`).
* `qcore_task_collect` replays that array, **on the parent's thread**, after a
  successful `thrd_join` (`:598-611`).
* `qcore_task_start`, `qcore_task_collect` and `qcore_task_print_result` are
  called from the parent, and a worker never calls them: the source lowering
  refuses a `spawn` inside a worker ("spawned workers must be first-order leaf
  functions in this task profile").

So **each stream of an emitted task program is written by exactly one thread, in
the parent's own statement order.** A source line placed at *any* point of the
parent body therefore has a determined position relative to every other line —
which means the brief's conservative reading ("the parent scope before any
spawn, between joins, and after all joins") is not a restriction this card needs
to impose: **every parent point is join-ordered**, because a worker's output does
not exist as bytes until its parent replays it at the join. The wider admission
is the honest one, and the narrower one is a corollary rather than a rule
(`a_line_before_any_spawn_is_admitted` pins the narrow reading's own cases too).

A line inside a **worker** body is refused by name
(`ComposedTaskRule::SiteOutsideEntry`, `Q050`), with the ground in the message:
it would be written by that worker's own thread, concurrently with the parent's
writes and with its siblings', and no checked model in this repository orders
them, so the per-stream comparison would compare against something that is not a
function of the program. That is the boundary, and `shared_readers.qcore`'s
worker carries a comment where such a line would go.

**The future path is named, not built.** Buffering worker lines the way the
trace events are buffered would need either F02's
comparison-up-to-an-admitted-reordering treatment
(`crates/qcore-core/src/task_trace_relation.rs`) or `STD-LOGGER-ASYNC`'s
declared queue policy. Neither is cheap here — the F02 relation is over the
*model* trace domain and would have to be extended to carry source lines with a
partial order the checker derives — so it is follow-on `COMPOSE-WORKER-LINES`
(§9) rather than scope creep.

### 2.4 `STD-LOGGER-ASYNC`'s blocker, re-examined against today's ledger

**It has not moved, and this card adds a second, independent reason.**

`STD-MODULES` §8.1 recorded two findings. Both are re-measured on this tree:

1. **The task ledger still has no post-creation transfer.** Its complete public
   operation set in `crates/qcore-core/src/scoped_tasks.rs` is `acquire`,
   `use_resource`, `read_resource`, `drop_resource`, `enter_scope`, `exit_scope`,
   `reserve`, `reserve_with_loans`, `rollback`, `commit`, `complete`, `join`,
   `fail_join` — the same list, unchanged, plus read-only accessors. Every
   parent→child custody edge is still established at creation. There is still no
   `send`, `enqueue` or transfer-to-a-running-child.
2. **The checked mutex backend still emits one thread** — its own module header
   still says so (`mutex_c_emit.rs:4`, `:84`).

What this card adds: the composition now *demonstrates* the second half of that
follow-on's requirement. `STD-MODULES` §8.2 wrote that when the async logger is
taken up, "the ordering comparison will need the F02 task-trace-relation
treatment (comparison up to an admitted reordering) or the feature is not
v1-honest." This card measured exactly that wall from the other side: a
**worker-side line** is refused for want of that ordering model, and the refusal
is the same one an asynchronous logger would hit at its queue. So
`STD-LOGGER-ASYNC` and `COMPOSE-WORKER-LINES` are the same missing capability
seen from two directions, and the report records them as such.

## 3. The composition, stated exactly

### 3.1 It is the task program plus placements, and here is what that buys

```rust
pub struct ComposedTaskProgram {
    pub tasks: TaskModule,        // checked by check_task_module, unedited
    pub sites: Vec<TaskIoSite>,   // (block, index, StdIoEvent, operand slot)
    pub declares_io: bool,
}
```

`check_composed_task_program` runs `check_task_module` **first and unedited**, so
composing can never admit a task program the task route refuses; only then does
it state its own rules. A composed module therefore gets:

* every task-route refusal, from the task checker, at the task pass's own
  `Q600` — the custody transfer discipline, the share ledger, the join
  discipline, the leaf-worker rule, the all-path check, everything;
* every `source.std-io.v1` **shape** refusal, at that route's own `Q030`,
  because the three line-producing statements are read by `source_std_io`'s own
  parser (`std_io_statement`), extracted from the lowering in this card so that
  a shape one route accepts and the other refuses is not expressible;
* the **line bytes** from `std_io::rendered_line`, extracted from
  `check_std_io_program` in this card for the same reason: one place decides
  what a line looks like, so "a composed program's source lines are
  `source.std-io.v1`'s lines" is a statement about code;
* the **C statement** from `std_io_c_emit::event_statement`, the same function
  the standard-IO route and the extern composition already use;
* the **effect rule** from `std_io::derived_effects`, applied to the composed
  projection.

Nothing in `composed_tasks.rs` re-implements a custody rule, a share rule, a
join rule or a rendering rule.

### 3.2 The liveness question is asked of the task checker, in its own language

This is the part with an argument in it. A rendering operand may name an `i32`
the task model bound; whether that name may be **read at that position** is a
custody/liveness question, and the composition must not answer it.

It does not. `readable_probe` inserts an ordinary
`BindScalar { slot: fresh, value: ScalarOperand::Slot(s) }` — a copy of an
immutable scalar, an operation the task model already has and which moves no
custody — at **exactly** the rendering's position, and re-runs
`check_task_module`. If that module checks, the copy is admitted on every path
that reaches the site, and a rendering reads no more than the copy does. If it
does not, the rendering is refused (`ComposedTaskRule::OperandNotReadable`).

Two consequences worth stating:

* **The must-fact bar of `BORROW-LIVENESS` is met by construction**, not by a
  meet this module computes: `check_task_module` is an all-path checker, so
  "readable here" is its answer over every path, not a may-fact.
* **A widening of the task model widens this automatically and correctly.** If
  the task model ever admits copying a scalar in a place it does not today, the
  probe follows it; if it ever refuses one, the probe refuses with it. There is
  no second liveness rule to keep in step.

The frontend's type rule runs first and gives the better message — `` `NAME` is
not an `i32` this entry bound `` — and the probe is the checked precondition
behind it. The frontend also refuses a worker-body line before a site is built,
which makes `SiteOutsideEntry` unreachable from source; it is kept and pinned
directly against the model (`crates/qcore-core/tests/unit/composed_tasks.rs`) on
the `STD-IO-COMPOSE` §6.1 precedent.

### 3.3 The custody discipline

`STD-MODULES` made it structural — the operand type has no constructor that
could denote a resource — and this route keeps that, over a different value
namespace. `TaskIoSite::operand` is an `Option<SlotId>` and the site's event is
a `StdIoEvent`; a name resolves through the task lowering's environment and is
admitted only at `ValueType::Scalar(Scalar::I32)`. An owned Buffer, a read loan,
a task handle and an undiscriminated `Result` are each refused at `Q050`, and
pinned. So a rendering cannot consume, release, lend or retain custody of
anything, and the Lean side proves it as an equation rather than asserting it:
the ledger a composed run leaves is *exactly* the one the same program leaves
with every line erased.

**`borrow NAME` is refused on this route**, deliberately and by name.
`STD-FMT-BORROW` admits one on `source.std-io-extern.v1` because that model
publishes a ledger (`live_after`) the join can read; the task ledger's
parent/child custody edges, share arithmetic and join discipline are a different
question — a resource can be live in the parent while lent to two readers, or
held by a child until its join — and admitting a borrowed render across a task
boundary needs its own custody argument. Follow-on `COMPOSE-TASK-BORROW` (§9).

### 3.4 Emission and reference execution

`emit_task_c11_with_source_lines` places `event_statement`'s output immediately
before the instruction the site names, in the **entry function only**. The
operand expression is `s_N.value.scalar` — the object the task backend already
declared for that slot, never a fresh copy — which is the same discipline
`STD-IO-COMPOSE` §5.4 applied to `vN`.

`execute_with_source_lines` is the reference side. It records
`(position, ObservedLine)` where `position` is the index into the task
observation's own event list at the moment the line is written, and
`ComposedTaskObservation::projections()` splices. The two halves are kept
**apart** rather than merged, so "the task evidence is the task route's" is a
fact about the data: `ComposedTaskObservation::tasks` is exactly what
`execute_task_module` would have produced for the same module, and
`render_task_observation` of it is exactly standard output with this route's own
lines removed. That equation is checked over all four published examples
(`removing_the_source_lines_leaves_exactly_the_task_rendering`) and proved in
general in Lean (`erasing_the_source_lines_leaves_the_task_trace`).

## 4. The four examples — per-example disposition

All four take `ComposedStdIoTasksV1` after this card. All four compile under
`-Wall -Wextra -Werror`, run clean under ASan/UBSan (LSan on Linux), and
byte-match the composed reference **per stream**.

| Example | Source lines added | stdout | stderr (successful run) | Notes |
|---|---|---|---|---|
| `two_workers` | `log info dec 2;` (the creation decision), `first_result \|> log info dec;`, `second_result \|> log info dec;`, `second_result \|> print dec;`, and `log warn dec 1;` + `first_result \|> log warn dec;` on the refused-creation arm | **gains one line**, the bare `7`, between `QCORE_TASK_JOIN 1` and `QCORE_TASK_RESULT 0 I32 7` | `[INFO] 2`, `[INFO] 42`, `[INFO] 7` | The base case and the README transcript. The only example whose stdout moves. |
| `owned_buffer` | `local \|> log info dec;` (the parent's own buffer, while the child owns the other), `value \|> log info dec;` **post-join**, plus `log warn`/`log fatal` on the two failure arms | **byte-identical** | `[INFO] 7`, `[INFO] 42` | **`STD-FMT-BUFFER-READ` landed** — see §5. |
| `core_context` | `value \|> log info dec;` on the success arm; `log warn dec 0;` + `value \|> log warn dec;` on the **creation-rollback** arm | **byte-identical** | `[INFO] 42` on the successful run; `[WARN] 0`, `[WARN] 42` on the injected-failure run | Exercises a line on a branch the successful run never reaches, pinned in `owned_source_tasks.rs` in both directions. |
| `shared_readers` | `during \|> log info dec;` (between the spawns and the joins, while both readers hold shares), `a \|> log info dec;` **between the two joins**, `b \|> log info dec;`, `after \|> log info dec;` after write access is restored, plus the two failure arms | **byte-identical** | `[INFO] 42` ×3, `[INFO] 7` | The sharpest case. Worker-side rendering is out of v1 and the **refusal is pinned by name** (`a_line_in_a_worker_body_refuses`), with a comment in the worker where such a line would go. |

Nothing was skipped and nothing kept its unextended form: the v1 boundary reaches
every one of the four, in the parent.

## 5. `STD-FMT-BUFFER-READ`

`STD-FMT-BORROW`'s census recorded `qcore_buffer_read` as the census's one
**model-computed** live-resource read (its row C1), unreachable from a rendering
"only because it lives on the task/epoch routes, so this is a *route* problem: it
is strictly inside `COMPOSE-CONCURRENCY`, and lands with it or not at all."

**It lands.** `owned_buffer.qcore` renders, post-join, the number
`qcore_buffer_read` computed out of the buffer the worker returned:

```qcore
let returned = join move handle?;
let value = qcore_buffer_read(returned);
value |> log info dec;
```

and its sanitized native run's stderr byte-matches the reference's stderr
projection, with the reference having *computed* that number from the modelled
buffer's payload. That is a strictly stronger thing than `STD-IO-COMPOSE`'s
declared `succeeds INTEGER`: no outcome is declared anywhere, and no runtime
comparison guards it — the model computes the value the program will read,
because on this route it models the buffer's contents.

**What did not land, stated exactly.** The brief's spelling was
`buffer |> read |> render |> log`, and there is no `read` pipeline stage: the
buffer read is `qcore_buffer_read(NAME)`, a task-route intrinsic with its own
`let`, and giving the pipe a stage for it is a pipe-grammar change (the sink
stage's selector-word rule, `PIPE-BORROW-SINK`'s neighbourhood), not a rendering
change. What is admitted is the two-statement form above and its pipe tail
`value |> log info dec`. Follow-on `PIPE-INTRINSIC-STAGE` (§9).

## 6. What did not move

### 6.1 No synthesized line moved — pinned, not argued

`the_composed_emission_is_the_task_emission_with_lines_inserted` compiles the
same task program with and without three source lines and asserts that the
composed emitted C, with exactly those three statements removed, is
**byte-identical** to the task route's. So:

* the **F02 emission digests** (`compiler/tests/f02_trace_relation.rs`,
  `compiler/tests/windows_threads.rs`) do not move — their fixtures import no
  `std.io` and stay on the task route, and the backend they travel is the same
  code path with an empty site list;
* the **CT fixtures**' `emitted_c_contains`, `reference_observation`,
  `stdout_contains` and `stdout_order` do not move;
* the four examples' **stdout** is byte-identical for three of the four, and the
  fourth gains one line the source asked for (§4).

The Lean side proves the general statement rather than the instance:
`the_trace_is_a_function_of_the_task_steps_alone` and
`erasing_the_source_lines_leaves_the_task_trace`.

### 6.2 The corpus

**Corpus-neutral by construction.** No file under `crates/qcore-c-gateway/`,
`crates/qcore-audit/`, `contract/qcore/` or `providers/` is touched. The
contract-corpus pin (69 modules), the 185-unit artifacts, the leak count of
**272** and the CT manifest are main's, and none of them moves for anything here.
The whole diff is:

```
crates/qcore-core/src/composed_tasks.rs             (new: the join, the probe, the certificate)
crates/qcore-core/src/std_io.rs                     (rendered_line extracted; behaviour byte-identical)
crates/qcore-core/src/task_c_emit.rs                (the placed statements; empty list = the old entry point)
crates/qcore-core/src/task_execution/reference.rs   (render_task_event extracted; the widened run)
crates/qcore-core/src/lib.rs
crates/qcore-frontend/src/source_composed_tasks.rs  (new: the selector, and only that)
crates/qcore-frontend/src/source_std_io.rs          (std_io_event/std_io_statement extracted)
crates/qcore-frontend/src/source_tasks.rs           (the `io` effect, the placed lines, the composing flag)
crates/qcore-frontend/src/frontend.rs               (the ComposedTasks pass)
crates/qcore-frontend/src/lib.rs
crates/qcore-verification/src/driver.rs             (the route, the profile, the checked variant)
crates/qcore-core/tests/unit/composed_tasks.rs      (new)
compiler/tests/task_io_compose.rs                   (new)
compiler/tests/thread_examples.rs, owned_source_tasks.rs, module_imports.rs
examples/threads/{two_workers,owned_buffer,shared_readers,core_context}.qcore, README.md
docs/QCORE_SPEC_r0_3.md, docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md
proof/QCore/ComposedTasks.lean, proof/AuditComposedTasks.lean, proof/QCore.lean
docs/reports/wave21-compose-concurrency/README.md
```

## 7. Reserved words — the constraint-14 census

**Words added: 0.** The three line-producing statement heads (`print`, `log`,
`flush`), the two renderings (`dec`, `hex`) and the three levels (`info`, `warn`,
`fatal`) were all reserved by `STD-MODULES`, and `io` is an effect name inside an
`effects { … }` row, which is not an identifier position — the task route's
effect parser already read `alloc`, `spawn`, `join`, `may_block` and `throws`
there and this card adds one arm to that match, gated on the composed route.
Since no word is newly reserved, no `.qcore` file anywhere can newly collide, and
no fixture surface moves. The census `STD-MODULES` §5 established therefore did
not need re-running for a new word; it was re-run for `io` in the effect position
and found no source-side occurrence outside an effect row.

## 8. Deviations and boundaries, found by measurement

1. **The v1 boundary is wider than the brief's.** The brief proposed admitting
   lines at "join-ordered points — the parent scope before any spawn, between
   joins, and after all joins". Measured (§2.3), *every* parent point is
   join-ordered, so the admission is the whole parent body and the narrower
   reading is a corollary. Recorded as a deviation because it is one, and because
   the wider rule is the one a reader must check: it rests on the claim that
   worker output is buffered and replayed at the join, which is stated in the
   emitted runtime, pinned by the tests, and proved in the model
   (`the_parents_own_emissions_do_not_depend_on_the_workers`).
2. **`check_std_io_program` could not be run unmodified, and that is the
   structural difference from `STD-IO-COMPOSE`.** That checker resolves a
   rendering operand through `StdIoProgram::scalars`, a **static** name→value
   map, because on that route every number is declared at the site. On this route
   the number is *computed by the task model* at the rendering's own step — the
   value a `qcore_buffer_read` read, the outcome a `join` collected — and is not
   a static binding at all. Rather than fake one, the composed projection's
   `scalars` map is deliberately **empty** (pinned:
   `the_projection_carries_no_static_scalar_environment`), `derived_effects` is
   called on that projection because the effect rule reads only event kinds, and
   the line bytes come from the extracted `rendered_line`. So the reuse is at the
   level of the *decisions* — the grammar, the line bytes, the C statement, the
   effect rule — rather than of one function call, and each of the four is a
   single shared definition rather than a second implementation.
3. **`SiteOutsideEntry` and `SiteOutOfRange` are unreachable from source.** The
   lowering refuses a worker line with its own message before a site is built,
   and it can only place a site at a position of the block it is building. Both
   arms are kept as fail-closed backstops and pinned directly against the model,
   on the `STD-IO-COMPOSE` §6.1 precedent: if the reachability ever changes, a
   test says so rather than an arm quietly passing.
4. **`OperandNotReadable` is likewise a backstop from source**, because the
   frontend's lexical scoping refuses a name out of scope at `Q013` first, and an
   `i32` slot in the task model is single-assignment and never consumed. It is
   pinned at the model with a real instance — a line placed *before* its binding,
   where the probe's `check_task_module` refuses — so the mechanism is exercised
   rather than asserted.
5. **A rendering before its binding refuses at `Q013`, not at the join.** The
   fourth time this measurement is recorded (`PIPE-SCALAR-RESULT` §2.3,
   `STD-IO-COMPOSE` §6.2, `STD-FMT-BORROW` §7.1) and handled the same way: name
   resolution runs over the parsed body before any lowering. Pinned, with the
   join's own rule kept beneath it.
6. **`as_scoped_tasks()` answers `None` for a composed module**, on the
   `ComposedEpochTasks` precedent: its task half is one half of a checked pair,
   and a consumer asks `as_composed_tasks()` and takes `tasks()` from it, with
   the placed lines in view beside it. Three test files were updated for that,
   and `core_context.qcore`'s two existing tests
   (`owned_source_tasks.rs`, `module_imports.rs`) pass unchanged otherwise —
   their stdout comparisons are untouched, because that example's lines are all
   on stderr.
7. **`check_task_module` is run more than once per composed module**, once for
   the program and once per rendering operand for the probe. That is a
   deliberate cost: it is what makes the liveness answer the task checker's. It
   is bounded by the number of rendered names, and the pinned corpus is not
   affected (this is a source route).

## 9. Named follow-ons

| Name | What it is, and what it needs |
|---|---|
| `COMPOSE-WORKER-LINES` | a line inside a **worker** body. Needs an ordering model for two writers: either F02's comparison-up-to-an-admitted-reordering extended to carry source lines, or the worker-buffer treatment the trace events already get, with the checker deriving the partial order. It is the same missing capability as `STD-LOGGER-ASYNC`'s ordering half (§2.4). |
| `COMPOSE-TASK-BORROW` | `print RENDERING borrow NAME;` on this route. Needs the task ledger to publish a per-step liveness snapshot the join can read, the way `ExternCallCertificate::live_after` does — and a custody argument for a resource that can be lent to two readers or held by a child until its join, which the extern route's straight-line ledger never had to make. |
| `PIPE-INTRINSIC-STAGE` | the brief's literal `buffer \|> read \|> render \|> log`. Needs a pipeline stage for a task intrinsic, which is a pipe-grammar change (§5). |
| `COMPOSE-TASK-EXTERN` | the third exclusion: a module that extern-calls **and** spawns, with or without lines. Two checked models beside a branching concurrent one, and a third join. |
| `COMPOSE-EPOCH-LINES`, `COMPOSE-MUTEX-LINES` | the same composition for the epoch and mutex routes, each of which is selected earlier and keeps its refusal today. The mutex backend emits one thread, so its case is the easier one. |
| `STD-LOGGER-ASYNC` | *(unchanged, blocker re-measured — §2.4)* |
| `TRACE-STDIO-WIN-RUN` | *(unchanged)* `build-windows.ps1 -Run` checks exit codes only. It now has **more** to check: the four examples write on stderr as well. |
| `STD-FMT-BUFFER-READ` | **retired** by this card (§5) for the statement form; the pipeline spelling moves to `PIPE-INTRINSIC-STAGE`. |
| `COMPOSE-CONCURRENCY` | **retired** by this card for the print-and-spawn pair. |
| `EXTERN-SCALAR-OUTCOME` *(trace half)*, `COMPOSE-RENDER-WIDTH`, `EXTERN-SCALAR-WIDTH`, `STD-FMT-DIAGNOSTIC`, `STD-FMT-EXTENT`, `STD-FMT-DESCRIPTOR`, `STD-FMT-ADDRESS`, `PIPE-BORROW-SINK`, `SEQ-SCALAR-RESULT`, `PIPE-TAP`, `PIPE-ASSIGN`, `PIPE-BINDER-SPAN`, `PIPE-STMT-STAGES`, `PIPE-STAGE-ARITY` | *(unchanged)* |

## 10. Evidence

### 10.1 Tests

| Target | Count |
|---|---|
| `compiler/tests/task_io_compose.rs` | **16 new**, including one sanitized native run compared per stream |
| `crates/qcore-core/tests/unit/composed_tasks.rs` | **14 new**, against the model directly |
| `compiler/tests/thread_examples.rs` | **5 → 8**, all extended, none weakened |
| `compiler/tests/owned_source_tasks.rs` | 14, one extended with the rollback-arm assertion |
| `compiler/tests/module_imports.rs` | 3, unchanged behaviour |
| `compiler/tests/std_modules.rs`, `std_io_compose.rs`, `extern_calls.rs`, `pipe_operator.rs`, `source_tasks.rs`, `f02_trace_relation.rs`, `scoped_read_loans.rs`, `scoped_read_shares.rs`, `error_payload_tasks.rs` | unchanged and unmoved |

`examples/threads`' five checks, each **extended**:

| `TRACE-STDIO` | after this card |
|---|---|
| single emission route, asserted non-vacuously for all four | kept, and non-vacuous in **both** directions: each example writes through `QCORE_IO_WRITE(stdout,` *and* `QCORE_IO_WRITE(stderr,` |
| `-Wall -Wextra -Werror` + ASan/UBSan/LSan for all four | kept, unchanged |
| stdout byte-compared with the reference | **widened to per stream**: stdout against the composed reference's stdout projection, stderr against its stderr projection |
| README transcript read out of the file and compared | kept, now **two** marked blocks, one per stream, with the shape check widened |
| a source `print` is impossible in a threaded module (2 pins) | **moved by name** into three tests (§2.1) |
| — | **new**: removing the source lines from stdout leaves exactly `render_task_observation` of the task projection, for all four |

### 10.2 Lean

New module `proof/QCore/ComposedTasks.lean`, audited by the new
`proof/AuditComposedTasks.lean`, registered in `proof/QCore.lean` so
`AuditAll.lean`'s whole-project axiom closure covers it. **31 audited theorems**,
no new axiom, no placeholder, no `native_decide`. No existing `QCore/*.lean` or
`Audit*.lean` file was edited — the one-line import in `proof/QCore.lean` is the
only change to an existing proof file.

It imports `QCore.StdIo` and *reuses* that module's `Rendering`, `Level`,
`Stream` and `Effect`, so the line vocabulary is `source.std-io.v1`'s rather
than a copy of it, and `the_streams_agree_with_the_std_io_route` states that as a
theorem: this route's `publishes` and `QCore.StdIo.linesOn` are the same
decision.

The card's central claims are proved rather than asserted:

* `the_ledger_is_a_function_of_the_task_steps_alone`,
  `a_line_leaves_the_ledger_unchanged`,
  `a_program_of_lines_leaves_the_ledger_unchanged` and
  `the_composed_ledger_is_the_task_projections_ledger` — composing added no
  custody path, as a statement about the model;
* `the_trace_is_a_function_of_the_task_steps_alone` and
  `erasing_the_source_lines_leaves_the_task_trace` — no synthesized trace byte
  moves, in general rather than on the four examples;
* `the_parents_own_emissions_do_not_depend_on_the_workers`, with
  `the_logger_stream_does_not_depend_on_the_workers`,
  `the_ledger_does_not_depend_on_the_workers` and
  `only_a_join_replays_a_workers_trace` — **the interleaving argument**: a
  worker's buffered trace enters the observation only at that worker's join, so
  the parent's own emissions are the same list whatever the workers wrote. That
  is why a line at any parent point is comparable, and its absence is why a
  worker line is not;
* `a_line_in_a_worker_has_no_admitted_placement`,
  `a_line_in_the_parent_is_admitted`, `the_placements_are_exactly_these` — the
  boundary, with the surface as one list so a future widening changes a theorem;
* `print_never_writes_stderr`, `log_never_writes_stdout`,
  `a_barrier_publishes_no_line`, `a_resolved_print_publishes_exactly_one_line`,
  `an_unresolved_operand_publishes_nothing` — stream separation, with **no**
  cross-stream order claimed anywhere;
* `an_operand_is_a_literal_or_a_scalar_name`,
  `a_rendering_reads_only_a_bound_name` — the effect discipline as a type;
* `writing_requires_io`, `a_barrier_still_carries_io`,
  `no_line_carries_alloc_or_mut`, `a_program_of_no_lines_carries_nothing` — the
  effect row, derived by `QCore.StdIo.derivedEffects` over the projection.

One `private` helper lemma, `filter_replicate`, is not listed individually,
following the convention `QCore.ComposedStdIo` established
(`STD-IO-COMPOSE` §7.5): it is covered by `AuditAll.lean`'s whole-project axiom
closure and transitively by every public theorem that depends on it.

**Not proved**, stated in the module header: that the Rust lowering extracts
these steps faithfully from source tokens, that the emitted C performs them
(both tested Rust boundaries, on the `ConcurrencyPrelude` precedent), the digits
a rendering publishes (the *number* is modeled; the digits are compared against a
native run), and any order between the two streams.

### 10.3 Native evidence

Five sanitized runs on x86_64 Linux, Clang, with
`-std=c11 -pthread -Wall -Wextra -Werror -g -O0 -fsanitize=address,undefined
-fno-omit-frame-pointer`, `detect_leaks=1`, `halt_on_error=1`: the four published
examples and `task_io_compose.rs`'s own fixture (which additionally passes
`-pedantic`). Each compares **stdout against the composed reference's stdout
projection and stderr against its stderr projection**. No cross-stream order is
asserted anywhere.

Three of the five are **positive** leak controls: `owned_buffer`,
`shared_readers` and `core_context` each allocate, transfer and release a real
buffer while rendering numbers read out of it, so had a rendering moved,
duplicated or discharged custody, LSan or the double-free detector would have
said so. Apple ASan provides no LSan, so leak detection stays a Linux-only
observation.

## 11. Gate

| Gate step | Result |
|---|---|
| `git submodule update --init --recursive` | run in this fresh worktree before the closing measurement |
| `cargo test --workspace --no-fail-fast` | **3,296 passed, 0 failed, 8 ignored**, doctests included, exit **0** — see §11.1 |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean (one of this card's own lints found and fixed at the source, not suppressed) |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit **0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit **0** |
| `python3 scripts/check_lean_audits.py` | **89/89**, no new axiom (88 → 89 for this card's new audit file) |
| `python3 -m unittest discover -s scripts/tests` | 193 tests, **1 expected failure** — see §11.2 |
| `cargo test -p qcore-compiler --test thread_examples` | 8 pass, four sanitized native runs |
| `cargo test -p qcore-compiler --test task_io_compose` | 16 pass, one sanitized native run |

Measurement outputs are worktree-private under `measure/` (gitignored), never
under `target/` and never in the shared scratchpad.

### 11.1 The workspace run

One run, no retries, doctests included: **3,296 passed, 0 failed, 8 ignored**
across 188 targets, exit **0**. **None of the four `P02-FLAKE` rows (standing
constraint 16) tripped** — the `RLIMIT_NPROC` conformance test, the
process-lifecycle budget test, the confined-fixture write test and
`real_packages_preserve_unknown_and_reject_mutations` all passed on the first
attempt, so nothing here rests on a re-run and no assertion was weakened.

For the integrator: a sibling worktree was building and testing on the same
machine throughout, so wall-clock timings here are not comparable with a quiet
run; none of the results depends on them.

**One line was added after that run started, and is named rather than buried.**
`crates/qcorec/src/main.rs` gained the composed profile's CLI label — one match
arm printing a description string — while the workspace run was in flight. It
was an edit to a source file only; no `cargo` invocation was made until the run
had finished, so nothing rebuilt underneath it (the hazard `STD-IO-COMPOSE`
§9.1 recorded). The final tree was then re-measured for that crate:
`cargo test -p qcorec --no-fail-fast` passes (7 + 7 + 0 across its three
targets), `cargo fmt --all -- --check` is clean, `cargo clippy --all-targets --
-D warnings` is clean, and both probes exit 0 on the final tree. No test asserts
that label's text.

### 11.2 One expected failure, left for the integrator

`scripts/tests` reports
`test_the_derived_facts_are_recomputed_rather_than_carried` failing with
`lean_audits: 88 != 89`. That is the derived-value tripwire working exactly as
designed: this card adds one audited Lean module and
`docs/reviews/r0_3/acceptance_matrix.json` carries the pre-card count. The base
commit `45f6c74c` was self-consistent (88 audit files, matrix value 88), so the
delta is entirely this card's and is not a pre-existing failure. **This card's
brief forbids editing acceptance files**, so the matrix is deliberately left
stale for the wave integrator to regenerate — the same handling every wave's
derived sections receive.

### 11.3 Integrator reconciliation list

* **One diagnostic code allocated, `Q050`**, under a new `ComposedTasks`
  semantic pass, through the §L21 process. It is the first free code above the
  native-library second band `Q047`–`Q049`; `Q051`+ are free as far as this card
  is concerned.
* `docs/QCORE_SPEC_r0_3.md` §L21 gains one row (`Q050`);
  `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` gains §G02j, in the same change.
* `CheckedSourceModule` gains one variant, `ComposedTasks`, and one accessor,
  `as_composed_tasks()`. Every existing exhaustive match answers `None` for it —
  including `as_scoped_tasks()`, on the `ComposedEpochTasks` precedent (§8.6).
* `SourceExecutionProfile` gains one variant, `ComposedStdIoTasksV1`.
* `qcore_core::std_io::rendered_line` and
  `qcore_core::task_execution::render_task_event` are **extracted**, not new
  behaviour: both are byte-identical factorings of code that already existed,
  and their callers are unchanged.
* `qcore_frontend::source_std_io::std_io_statement` is likewise extracted; the
  three arms of `lower_source_std_io_skipping` now call it, with the same
  messages.
* `emit_task_c11`, `execute_task_module_with_worker_allocations` and
  `lower_source_tasks` keep their signatures and are each defined as the widened
  function with an empty site list.
* `docs/reviews/r0_3/acceptance_matrix.json` needs its derived `lean_audits`
  (88 → 89) and `lean_audit_files` regenerated. Nothing else in it moves for this
  card.
* `AGENTS.md`'s headline Lean-audit number is stale on main (it says 80; main
  `45f6c74c` has 88, and this card makes 89). Recorded here rather than fixed,
  because `AGENTS.md` is outside this card's footprint.
* **A new standing constraint is proposed** for the "Concurrency source routes"
  section: *a source line on the composed task route stands in the parent's body
  and nowhere else, because every write an emitted task program performs is
  performed by the parent's thread — a worker's trace events are buffered in its
  own private array and replayed by the parent at the join. Admitting a
  worker-side line needs an ordering model (F02's comparison up to an admitted
  reordering, or `STD-LOGGER-ASYNC`'s declared queue), not a widened rule.*
  `wave21-compose-concurrency`.

## 12. Draft increment paragraph

> Composing standard IO with the checked task route (COMPOSE-CONCURRENCY): a
> module may now both spawn and print, on the new route
> `source.std-io-tasks.v1` — the second and last of the two pairs `STD-MODULES`
> named. The task half is the task route's **unedited** lowering and the
> **unedited** `check_task_module`, run first so composing can never admit a task
> program that route refuses; the line half is `source.std-io.v1`'s own grammar,
> its own line renderer, its own C statement builder and its own effect rule,
> each extracted to a single definition both routes call rather than
> reimplemented. There is no second lowering, backend, reference interpreter or
> route function at all: each is the composed one with an **empty** site list, so
> a module using one feature set keeps its route byte for byte by construction.
> **The v1 boundary is a measurement, not a policy.** Every write an emitted task
> program performs is performed by the parent's thread — a worker's trace events
> go to that worker's private array and are replayed by the parent inside the
> join — so a source line placed at *any* point of the parent body has a
> determined position, and the conservative "before any spawn, between joins,
> after all joins" reading is subsumed rather than imposed. A line inside a
> **worker** body is refused by name at `Q050` with that ground stated: it would
> be written by that worker's own thread and no checked model here orders them,
> which is the same missing capability `STD-LOGGER-ASYNC`'s ordering half needs
> (its ledger blocker was re-measured and has **not** moved — the task ledger
> still has no transfer-to-a-running-child). Whether a rendering may read a name
> at its position is **asked of the task checker in the task model's own
> language**: an ordinary `BindScalar` copy of the scalar is inserted at exactly
> that position and `check_task_module` re-run, so the composition owns no
> liveness rule and `BORROW-LIVENESS`'s must-fact bar is met by that checker's
> all-path check. One code, `Q050`, under a new `ComposedTasks` pass through the
> §L21 process, owns placement, position, operand and the effect row and nothing
> else; **no reserved word was added** (`io` is an effect name, gated on this
> route, and still refuses at `Q020` off it). Three pinned decisions moved
> deliberately: `TRACE-STDIO`'s "a source `print` in a threaded module is
> measured impossible" — the exact module that card wrote now compiles, and its
> two-spelling pin is rewritten into three tests recording what is true, what is
> refused instead, and the half of the old measurement that still holds; the
> route-selection exclusion `STD-IO-COMPOSE` pinned — one of its three rows is
> lifted and the other two (extern+spawn, print+extern+spawn) are decided
> honestly as *not* lifted and pinned; and the per-stream observation semantics,
> now well-defined under threads for the reason above. **`STD-FMT-BUFFER-READ`
> lands**: `owned_buffer.qcore` renders, post-join, the number
> `qcore_buffer_read` computed out of the returned buffer, with no declared
> outcome and no runtime guard, because on this route the model computes it. All
> four published examples say something of their own and all five of
> `examples/threads`' checks are extended — the byte comparison is now per
> stream, the README carries one marked transcript per stream, and a new check
> asserts that removing the source lines from standard output leaves *exactly*
> the task rendering those examples produced before. No synthesized line moved,
> pinned by comparing the composed emission with the task emission statement for
> statement, so the F02 digests, the CT fixtures and every pinned trace byte are
> untouched. 33 new tests (16 route/refusal, 14 against the model, 3 more over
> the published examples), five sanitized ASan/UBSan/LSan runs (three of them
> positive leak controls), 31 audited Lean theorems in a new `QCore.ComposedTasks`
> that reuses `QCore.StdIo`'s vocabulary, 89/89 audits and no new axiom.
> Corpus-neutral by construction. Evidence:
> `docs/reports/wave21-compose-concurrency/README.md`.
