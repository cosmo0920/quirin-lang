# COMPOSE-EVENT-LINES — source lines in an event program, and why the comparison stays strict (wave 23)

Card brief: *"Compose `source.event.v1` with the source line vocabulary so event
programs can carry source-level output lines."* With the user's own acceptance
rule, quoted verbatim: **"use pipe operator instead of debug prints on example
for std.event."**

Base commit: main `52e3889c` (the cross-platform/ARM64 observation record).
Route: `source.event.v1`, card `STD-EVENT`
(`docs/reports/wave22-std-event/README.md`). Comparator: `qcore.obs-reordering.v1`,
card `OBS-REORDERING` (`docs/reports/wave22-obs-reordering/README.md`).

**What landed.** An event entry may now carry `print`, `log` and `flush`, and
one value form — `let NAME = dispatch ready E;`, the byte that turn's callback
drained off the endpoint. `wake D BYTE;` declares which byte it delivers. One
new diagnostic code, **`Q065`**, the `Event` pass's second band. One new
published example, `examples/events/wake_payload.qcore`, whose every line is a
`|>` pipeline. 29 audited Lean theorems and definitions in a new
`QCore.ComposeEventLines`.

**The determinism boundary is untouched, and that is stated as a theorem rather
than as a promise**: a line erases to no reactor operation and no ledger
transition, so `Q061`'s two-member refusal, the declared-readiness rule and the
`write`-interest check-only arm answer exactly as they did (§3).

**No reordering is declared, because this route needs none.** §4 is the brief's
"generalise the live-set walk" half, and its answer is a measurement about the
reactor profile: every record an event program writes is written by the entry's
own thread, so every window is a barrier and the declared relation is *strict* —
the degenerate case of `OBS-REORDERING`'s capability, which is exactly how that
card says its own comparator relates to the strict one.

**Corpus-neutral by construction** (§9).

---

## 1. The spelling, and the user's rule

The idiomatic target the brief names is *read the byte a wake delivered off the
endpoint and pipe it*. That is `examples/events/wake_payload.qcore`:

```qcore
    wake a 72;
    wake a 105;
    let first = dispatch ready reader;
    first |> log info dec;
    let second = dispatch ready reader;
    second |> log info hex;
```

Two things about the spelling, both recorded rather than smoothed over.

**`value |> render |> log info` is not spellable, and the honest chain is two
stages.** `render` is not a stage; it is a *selector word* belonging to the
sink, so it collapses into `log info dec`. This is not new to this card —
`PIPE-CHAIN` §3 and `PIPE-SCALAR-RESULT` measured it, and
`PIPE-INTRINSIC-STAGE` §1.1 recorded the same collapse for the brief's earlier
`buffer |> read |> render |> log`. The landed spelling is `first |> log info dec;`.

**`dispatch` is keyword-shaped, so the turn itself is not a stage.**
`PIPE-INTRINSIC-STAGE`'s census sorted every model intrinsic into call-shaped
(a pipeline stage) and keyword-shaped (not one, proved as an inequality). A
dispatch turn is keyword-shaped, so the binding is spelled with the language's
own `let` — the same shape `source.buffer.v1` uses for a byte read — and the
value flows into the pipeline from there.

**The rule is about the examples and costs the route nothing.**
`the_piped_line_and_its_hand_written_twin_are_the_same_program` compiles both
spellings and compares the emitted C **byte for byte** and the reference
observation field for field. A statement-form line still compiles, because the
desugaring produces one; no published example writes one.

## 2. What the composition is

It is *inside* `source.event.v1`, not a route beside it — the `source.buffer.v1`
shape, whose line-producing statements are likewise its own route's. Three
additions, and nothing else:

| Form | What it is |
|---|---|
| `print RENDERING OPERAND;`, `log LEVEL RENDERING OPERAND;`, `flush;` | `source.std-io.v1`'s statements, read by **its** reader (`source_std_io::std_io_statement`) and rendered by **its** emitter (`std_io_c_emit::event_statement`). A shape one route accepts and the other refuses is not expressible. |
| `let NAME = dispatch ready E;` | The **same turn** `dispatch ready E;` is — same projection, same determinism decision, same trace record — naming the byte the callback drained. |
| `wake D BYTE;` | The byte that wake delivers. `wake D;` delivers `DEFAULT_WAKE_BYTE` (`'q'`), which is what the shim has always written. |

**The value vocabulary is exactly one form.** Before this card the route had
none at all: every name it resolved was a custody name. It now has this one, and
a second `let` initialiser has no spelling — `let v = 7;` and
`let v = dispatch idle;` both refuse at `Q065`
(`an_idle_turn_binds_nothing_and_no_other_let_has_a_spelling`).

**The custody/value wall stands.** An endpoint or registration name at a
rendering operand is refused at `Q065` rather than rendered as a number
(`rendering_a_custody_name_refuses_at_this_passes_own_code`), and a
`borrow NAME` is refused with the reason: this route publishes no live custody
identity for a render, which is card `STD-FMT-DESCRIPTOR`'s subject and not this
one's.

### 2.1 The payload, and why the trace does not carry it

The readiness ledger was a count per endpoint; it is now a **queue of bytes**.
A ready turn drains the oldest undrained one, so the value a turn binds is the
byte its own wake delivered — 72 then 105 in the example, and 105 then 72 if the
two wakes are exchanged (`exchanging_the_two_wakes_exchanges_the_two_bound_bytes`).

**The payload is absent from the trace, deliberately.** A `wake` projects to no
reactor operation at all — that is `STD-EVENT`'s own
`erasing_the_wakes_leaves_the_projection_unchanged` — so its byte is not a
reactor fact. It is a *value*, and a value reaches an observation only where a
source line renders it. The consequence is measurable and measured: every
transcript this route published before this card is byte-identical after it
(`the_self_pipe_example_is_byte_identical_after_this_card`,
`a_line_writes_no_trace_record_and_a_payload_reaches_no_trace`).

### 2.2 The second premise, checked rather than trusted

> **`EVENT-V1-STREAM-BYTE-ORDER`** — a connected stream endpoint pair delivers
> the bytes written into one end to a reader of the other end in the order they
> were written, none lost and none invented, given that this process is the sole
> holder of both endpoints and no other thread touches them.

It is carried **only** by a program that names a drained byte
(`the_byte_order_premise_is_carried_only_where_a_byte_is_named`) — a program
that drains without naming rests on nothing about the value, the same discipline
that keeps `EVENT-V1-LEVEL-TRIGGERED-READINESS` off a program which performs no
wait. And it is checked exactly as the readiness premise is: the emitted program
compares the byte it read against the one the model predicted and returns
`STATUS_PAYLOAD` (76) on disagreement. A false premise is a **failed run**,
never a rendered line no model computed.

## 3. Nothing of `Q058`–`Q062` moves

The whole question a reader should ask of a composition is whether the composed
route's own answers change. They do not, and the argument is not "we were
careful":

* **`erase`** maps a composed program to an event program by dropping every line
  and every payload. `erasing_the_lines_leaves_the_reactor_projection` says the
  reactor projection is unchanged under it, and
  `erasing_the_lines_leaves_the_run_unchanged` says the determinism ledger's own
  run is. So every theorem `QCore.StdEvent` proves about a program is a theorem
  about the same program with lines in it. This is the shape `STD-EVENT` used
  for `wake`, reused for the same reason.
* `two_ready_registrations_admit_no_turn` is restated over the composed
  statement type and takes the binding as a parameter: a `let` buys no turn
  (`a_binding_admits_exactly_what_the_bare_turn_admits`).
* The Rust side pins the same thing twice, with and without a line in the
  program (`the_determinism_boundary_is_unmoved_by_a_line`,
  `two_ready_registrations_still_refuse_with_a_line_present`).
* The `write`-interest arm is unchanged: a check-only module publishes no C and
  no observation, lines or not.

## 4. The comparison — the brief's live-set walk, generalised and then measured

The brief asked for `OBS-REORDERING` §1.2's live-set walk generalised to the
event model's regions, *not* a new comparator, and warned against declaring
coverage the route does not need. Both halves are honoured.

**The walk.** `std_event_c_emit::observe_event_program` is the same walk: a
record written while no other thread of control is live takes a window of its
own and commutes with nothing. What differs is the region — for tasks it is the
interval between a spawn and its collect; here it is a **dispatch turn**, and
the thread a turn makes live is the one running its callback.

**The measurement.** Which emitter that is, is read off the profile rather than
asserted: `concurrent_emitters(profile)` consults
`ReactorProfile::threading_enabled`, and v1's one accepted profile
(`SingleThreadedPollV1`) declares it `false` — under which the reactor refuses a
cross-thread activation at `L04`. The live set is therefore empty at every point
of every v1 program.

**The consequence, and the honest conclusion.** Every record is a barrier, so
the declared relation is strict, so comparing a native run against it **is** byte
equality — `strict_iff_byte_equality`, which `OBS-REORDERING` proved in general
and which this route is a case of. **This card declares no reordering**, and the
existing strict comparison is not replaced; it is shown to be the right one.
Pinned three ways:

| Where | What |
|---|---|
| `every_record_of_an_event_program_is_written_by_the_entrys_own_thread` | every derived record's emitter is `Emitter::Parent` |
| `the_declared_relation_is_strict_on_both_streams` | `is_strict()` on both streams, every window a barrier, the declared text equal to the strict projection, and a transposition of two records refused |
| `QCore.ComposeEventLines.the_event_relation_is_strict` / `…_is_admitted_exactly_when_it_is_byte_identical` | the same two facts as theorems, via `barriers_are_strict` and `strict_iff_byte_equality` |

v1 has no source-level callback body, so no line can stand *inside* a turn
today. The region is walked rather than assumed away precisely so that a route
which gives a callback a body inherits the structure instead of inventing one
(follow-on `EVENT-CALLBACK-BODY`).

## 5. The observation is now what the program writes

One deliberate change to an existing published value, and it is a correction
the quiet-emission amendment implies.

Before: an executable module's reference observation was the whole event trace
as `out:` records, regardless of whether the program wrote any of it. After the
amendment that over-claimed — without `log_level` the retained trace reaches the
null sink and the program writes nothing.

Now: the observation is **the bytes the program writes, per stream** — the
source-authored lines always, and the retained trace exactly where `log_level`
installs the hosted stdout adapter. The retained trace events are unchanged
either way: they are in the certificate and in the emitted program's typed
logger calls, which is where the amendment says compiler observations belong.

Consequences, all measured:

* the two published `STD-EVENT` programs are byte-identical, because
  `self_pipe.qcore` selects `log_level trace;`
  (`the_reference_observation_is_the_whole_event_trace`, unedited, still passes);
* a quiet composed program's observation is exactly its lines
  (`a_quiet_composed_program_writes_its_lines_and_nothing_else`), and its emitted
  C has exactly one stdio route — `io_emit::direct_stdio_calls` is empty;
* the native run now compares **both** streams rather than stdout alone
  (§7).

## 6. Refusals, and three things that are not where a reader would guess

Every refusal is pinned in both directions. Three are recorded as measurements
rather than as design, because that is what they are.

| Shape | Code | Where decided |
|---|---|---|
| a rendering operand that is a custody name | `Q065` | this route |
| `borrow NAME` at a rendering | `Q065` | this route, naming `STD-FMT-DESCRIPTOR` |
| a `let` that is not the one binding form | `Q065` | this route |
| a line-producing statement `source.std-io.v1` refuses | `Q065` | its reader's message, this pass's code |
| `wake D 256;` | `Q058` | this route — refused, never truncated |
| a line in a function this route never lowers | `Q058` | this route (see below) |
| two ready registrations, with or without a line | `Q061` | unchanged |
| a `print` without `import std.io;` | `Q015` | the import gate, before route selection |
| **a rendering operand naming nothing at all** | `Q013` | name resolution, *generically* |
| **a `let` shadowing an endpoint** | `Q012` | name resolution, *generically* |
| an owned buffer in an event module | `Q058` | this route (§8.1) |

**`Q013` for an unbound operand** is the `STD-FMT-DIAGNOSTIC` residual, recorded
by `STD-MODULES` for its own route and unchanged here: the refusal is correct,
only its specificity is the gap
(`rendering_a_name_nothing_binds_refuses_at_name_resolution`).

**`Q012` for a shadowing binding** means the model's own
`NameAlreadyBound` gate is **unreachable from source**: a `let` is an ordinary
binding to the general resolver and so is an endpoint name, so the duplicate is
refused first. The gate is kept and exercised directly
(`a_binding_that_collides_with_a_custody_name_refuses_at_the_model`) for the
reason standing constraint 17 keeps `EXTERN-F1`'s `resource_stored`: it guards a
public model API a composing route could drive. The unreachability is recorded,
not hidden.

**A line outside the entry** is newly refused. The route models the entry, and
before this card a `print` in a helper of an event module was refused only
because the whole route refused `print`; admitting lines without extending the
helper scan would have made it neither emitted nor refused. That is the one
fail-closed hole the composition could have opened, and it is closed
(`a_line_in_a_function_this_route_never_lowers_is_refused`).

## 7. The native half

`the_composed_program_reproduces_both_streams` compiles the emitted C with
`-std=c11 -Wall -Wextra -Werror -pedantic -g -O0 -fsanitize=address,undefined`
under `detect_leaks=1:halt_on_error=1`, runs it, and compares **stdout and
stderr separately** against the reference observation. A non-zero exit is a
sanitizer finding, a ready set that disagreed with the declaration, **or** a
drained byte that disagreed with the model — all three are failures.

`the_emitted_program_verifies_the_byte_it_drained` pins the three emitted lines
that make that true: the drain into `&qcore_payload`, the two
`!= (unsigned char) 72u` / `105u` comparisons returning 76, and the wake carrying
its declared byte.

**Platform status is unchanged and no new claim is made.** The shim's two
touched primitives (`qcore_event_wake` gains a payload parameter,
`qcore_event_drain` an out-parameter) are inside the existing three-arm
structure and compile on every arm by construction, but **this card ran the
Linux arm only**. `EVENT-V1-APPLE-RUN` stays. The Windows arm's own tests
(`compiler/tests/std_event.rs`, 27 tests including the AF_UNIX-required, forced
loopback and no-`afunix.h` builds) are unedited and cover the changed shim
signature when the gate runs on Windows; this card did not run them, and claims
nothing about that architecture. The published Windows runner is untouched.

## 8. Deviations from the brief, each argued

### 8.1 "event + buffer + stdio" is two compositions, not one

The brief names buffer as part of the idiomatic target. Measured: a module that
both dispatches and buffers routes **here** — route selection puts
`source.event.v1` before `source.buffer.v1` — and this route keeps its refusal
for the buffer statements (`an_owned_buffer_does_not_cross_into_an_event_module`).

That is the same principle the extern and standard-IO routes state about each
other, and admitting it would be a real card rather than a widening: one entry
would carry **two custody models**, the reactor's registration/descriptor ledger
and the ownership footprint the buffer route inherits, and the one question worth
answering — what does it mean to drain *into* an owned extent — is a payload
transport neither model has today. Named follow-on `EVENT-BUFFER-PAYLOAD`.

What the brief's sentence asked for concretely — *the byte a wake delivered,
piped into a rendered line* — landed in full, with the byte coming off the
endpoint into the value namespace directly. No extent is transported, so none is
discarded.

### 8.2 The reference observation's meaning changed

§5. It is a change to a published value, made deliberately and for a stated
reason, and every transcript a program actually writes is unmoved.

### 8.3 `wake` gained an operand the brief did not ask for

Without it every drained byte is `'q'` and the value half of the composition
demonstrates nothing: one constant, rendered. With it the FIFO structure of the
ledger is observable and testable, the premise it rests on is nameable, and the
run-time check has something to fail on. The operand is optional and the default
is the byte the shim already wrote, so nothing published moved.

### 8.4 No new pipe stage, and no edit to the pipe tables

Card `PIPE-DROP-STAGE` owns the stage/sink tables this wave. This card uses only
stages that exist on main: the sink stage `log LEVEL RENDERING`. `pipe.rs` and
the stage tables are untouched.

## 9. What this card did not touch

Corpus-neutral by construction: no file under `crates/qcore-c-gateway/`,
`crates/qcore-audit/`, `contract/qcore/` or `providers/`. The contract-corpus pin
(72 modules), the 185-unit artifacts, the leak count of **272** and the CT
manifest are main's. No frozen archive was edited, no pinned count moved,
`examples/threads` is untouched, and `AGENTS.md`, `DEVELOPMENT.md` and the
acceptance matrix are the integrator's.

Two shared files were extended rather than rewritten:
`crates/qcore-core/src/std_io.rs` is **unedited** (its `StdIoEvent`,
`rendered_line` and operand vocabulary are used as they are), and
`crates/qcore-core/src/qcore_event_poll_compat.h` gained one parameter on each
of two functions.

## 10. Reserved words — the standing-constraint-14 census

**Words added: 0.** The composition spells itself entirely in words the
resolver already reserves: `let` (the language's own binding word, reserved
since the beginning), `print`, `log`, `flush`, `dec`, `hex` and the six levels
(`STD-MODULES` and the quiet-emission amendment), and `dispatch`/`ready`
(`STD-EVENT`). The payload is an integer literal, which is not a word at all. No
`.qcore` file anywhere can newly collide, and the whole workspace gate is the
measurement rather than the argument.

## 11. Evidence

| Artefact | Where |
|---|---|
| Checked model | `crates/qcore-core/src/std_event.rs` (payload queue, value namespace, lines, `Q065`) |
| Observation, relation and emission | `crates/qcore-core/src/std_event_c_emit.rs` (`observe_event_program`, `EventObservation`, `concurrent_emitters`) |
| Shim | `crates/qcore-core/src/qcore_event_poll_compat.h` (`qcore_event_wake` payload, `qcore_event_drain` out-parameter) |
| Source lowering | `crates/qcore-frontend/src/source_event.rs` (`binding_statement`, the line heads) |
| Lean | `proof/QCore/ComposeEventLines.lean`, `proof/AuditComposeEventLines.lean` (29 entries) |
| Model tests | `crates/qcore-core/tests/unit/std_event.rs` (21 → **32**) |
| End-to-end tests | `compiler/tests/compose_event_lines.rs` (**22**, one of them the sanitized native run) |
| Rewritten pin | `compiler/tests/std_event.rs::a_module_that_both_dispatches_and_prints_stays_with_this_route_and_is_now_admitted` |
| Example | `examples/events/wake_payload.qcore` and its section of `examples/events/README.md` |
| Spec | `docs/QCORE_SPEC_r0_3.md` §L21 row `Q065`; `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` §G02k |

## 12. Gate

Run in this worktree on main `52e3889c` plus this card. Sibling agents were
building and testing on the same host throughout, so wall-clock timings are not
comparable with a quiet run.

| Step | Result |
|---|---|
| `cargo test --workspace` | GATE_WORKSPACE |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | GATE_PROBE |
| `cargo run -p qcore-compiler --example reevaluation_followup` | GATE_FOLLOWUP |
| `python3 scripts/check_lean_audits.py` | GATE_LEAN |
| `python3 -m unittest discover -s scripts/tests` | GATE_PYTHON |

Measurement outputs are worktree-private under `measure/` (gitignored), never
under `target/` and never in the shared scratchpad.

## 13. Named follow-ons

| Name | What it is, and what it needs |
|---|---|
| `EVENT-BUFFER-PAYLOAD` | draining into an owned extent rather than into a scalar. Needs one entry to carry two custody models and a payload transport neither has (§8.1). |
| `EVENT-CALLBACK-BODY` | a source-level callback body. It is what would put a record *inside* a dispatch turn, and it is where the reactor's `Access`, `L05` conflict detection and payload-authority rules become reachable. §4's walk is already shaped for it. |
| `STD-FMT-DESCRIPTOR` | `borrow NAME` rendering an endpoint's or registration's live model identity. Unblocked by `STD-EVENT` §2.1 and refused by name here; still its own card. |
| `EVENT-V1-WINDOWS-RUN`, `EVENT-V1-APPLE-RUN` | *(unchanged)* the two arms this gate does not run. The Windows arm's own suite is unedited and covers the changed shim signature when a Windows gate runs it. |
| `STD-FMT-DIAGNOSTIC` | *(unchanged)* an operand naming nothing refuses generically at `Q013`. §6. |
| `EVENT-LINE-NAME-COLLISION` | making the model's `NameAlreadyBound` gate reachable from source, or deciding it should not be. Today `Q012` answers first. §6. |
| `COMPOSE-EVENT-EXTERN` | an event module that also calls a declared extern function. Route selection puts it with the extern route today, which keeps its own refusal. |
| `PIPE-DROP-STAGE`, `PIPE-INTRINSIC-EXAMPLE` | *(unchanged, other cards')* |
| `COMPOSE-EVENT-LINES` | **retired by this card.** |

## 14. Draft increment paragraph

> Source lines in an event program, and the comparison that stays strict
> (COMPOSE-EVENT-LINES): a `source.event.v1` entry may now carry `print`, `log`
> and `flush` — `source.std-io.v1`'s own statements, read by its reader and
> rendered by its emitter — together with the one value form this route has,
> `let NAME = dispatch ready E;`, which is the **same dispatch turn**
> `dispatch ready E;` is and names the byte that turn's callback drained off the
> endpoint; `wake D BYTE;` declares which byte it delivers, and the readiness
> ledger became a queue so the value a turn binds is the byte its own wake
> wrote. **Nothing of `Q058`–`Q062` moves**, and that is a theorem rather than a
> promise: a line erases to no reactor operation and no ledger transition, so
> `Q061`'s two-member refusal, the declared-readiness rule and the
> `write`-interest check-only arm answer exactly as before. The **payload is
> absent from the trace** — a `wake` projects to no reactor operation, so its
> byte is a value and not a reactor fact — which is why every transcript this
> route published is byte-identical after the card. A second named platform
> premise, `EVENT-V1-STREAM-BYTE-ORDER`, is carried only by a program that names
> a drained byte and is **checked** by the emitted program, which exits 76 on a
> byte the model did not predict. The reference observation is now the bytes the
> program writes, per stream: the source lines always, the retained trace exactly
> where `log_level` installs the hosted adapter — the quiet-emission amendment's
> own boundary, applied to a value that previously over-claimed. **No reordering
> is declared, because none is needed**: card `OBS-REORDERING`'s live-set walk,
> generalised to this model's regions (a dispatch turn), reads its emitters off
> the reactor profile, and v1's one profile is single-threaded, so every record
> is written by the entry's own thread, every window is a barrier and the
> declared relation is strict — the degenerate case `strict_iff_byte_equality`
> names, so the existing byte comparison is not replaced but shown to be the
> right one. One new code, `Q065`, the `Event` pass's second band, for the
> composition's own refusals: a custody name at a rendering operand, a `borrow`
> render (`STD-FMT-DESCRIPTOR`), a `let` that is not the one binding form, and a
> line shape `source.std-io.v1` refuses. **No reserved word was added.** The
> user's acceptance rule is honoured literally — every line in `examples/events`
> is a `|>` pipeline, and the piped and hand-written spellings are pinned to emit
> byte-identical C; the brief's `value |> render |> log info` is spelled
> `value |> log info dec` because `render` is a sink selector word, the same
> collapse `PIPE-INTRINSIC-STAGE` recorded. The brief's "event + buffer" half is
> **two compositions and not one**: an owned buffer still does not cross into an
> event module, measured and pinned, because one entry would then carry two
> custody models (`EVENT-BUFFER-PAYLOAD`). One new published example,
> `wake_payload.qcore`, sanitized-native with **both** streams compared. 29
> audited Lean theorems in a new `QCore.ComposeEventLines`, no new axiom.
> Corpus-neutral by construction. Evidence:
> `docs/reports/compose-event-lines/README.md`.
