# STD-EVENT — a checked event surface over the existing reactor models (wave 22)

Card brief: *"Land `import std.event;` exposing a minimal checked event
vocabulary over `os_endpoint` and the reactor/descriptor custody models, which
are today source-UNREACHABLE (the recorded U3 blocker). Census first: enumerate
what the models can already express; your surface is the honest projection of
it."*

Base commit: main `ec5535a9`. Design record, written before the implementation:
[`docs/design/STD_EVENT_V1.md`](../../design/STD_EVENT_V1.md). Example page:
[`examples/events/README.md`](../../../examples/events/README.md).

**What landed.** A new route `source.event.v1`, built the way `STD-MODULES`
built `source.std-io.v1`: lower, check, **recheck**, interpret, emit. The
custody half is not new and is not a second opinion — the projected program is
handed to the *existing* `event_reactor::check_event_program` unmodified, so a
refusal a reader sees at `Q053` carries that model's own `L00`–`L12` diagnostic
verbatim. The route's own contribution is one thing: the **determinism
boundary** that gives each dispatch turn one well-defined observation. One new
semantic pass, `Event`, owning `Q051`–`Q055`. An embedded three-arm readiness
shim (`poll(2)` on Linux and Apple, `WSAPoll` on Windows). 33 audited Lean
theorems and definitions in a new `QCore.StdEvent`.

**This card retires the U3 source-unreachability blocker** (§2.1), and
`STD-FMT-DESCRIPTOR`'s blocker moves accordingly — that card is not this one's,
and the unblocking is recorded here only.

**Mio/Tokio are not bound, deliberately** (§6).

---

## 1. Census — what the existing models can already express

Read off `EventOperation<P>` and the checker in
`crates/qcore-core/src/event_reactor.rs` (1,823 lines) and the `close(2)`
post-state relation in `crates/qcore-core/src/os_endpoint.rs` (1,642). The v1
column is this card's answer, and every `no` has a ground rather than a silence.

| Model operation | What it decides | v1 |
|---|---|---|
| `CreateBase` | one reactor context under an accepted profile (`L09`) | **yes** — `event open;` |
| `FreeBase` | teardown; refuses while a dispatcher, a running callback, a live event or a live bufferevent remains (`L11`) | **yes** — `event close;` |
| `BeginDispatch` / `EndDispatch` | one dispatch turn; `EndDispatch` refuses unless quiescent (`L06`) | **yes** — bracketed around each `dispatch` |
| `Assign` | a registration: base, **descriptor**, flags, priority, callback summary; refuses a closed descriptor (`L03`), an invalid flag set, a trigger the flags do not permit, a dead argument (`L02`) | **yes** — `watch E D INTEREST;` |
| `Add` / `Reschedule` | make a registration pending; refuses on a closed descriptor | **yes** — with `Assign` |
| `Activate` | readiness arrives for a permitted reason; `cross_thread` needs a threading profile (`L04`) | **yes** — from `dispatch ready E;` |
| `CallbackEnter` / `CallbackExit` | run one callback; enter refuses unless the base is dispatching, nothing else runs, the argument is live and this event has the **minimum priority among the active ones** (`L06`) | **yes** — from `dispatch ready E;` |
| `Delete` | deregister; `Nonblocking` from inside a running callback | **yes** — `unwatch E;` (`Ordinary` only) |
| `FreeEvent` | free the registration; drops an `Owned` argument | **yes** — with `Delete` |
| `CreateSocketPair` | two distinct owned descriptors, or the failure branch with no custody | **yes** — `endpoints A B;` (success branch) |
| `CloseSocket` | release; the failure branch consults the target-qualified relation | **yes** — `close D;` (success branch) |
| `DropArgument` | retire a callback cookie no live registration references (`L02`) | via `FreeEvent` on the `Owned` cookie |
| `Finalize` | the finalize protocol for `EVLIST_FINALIZING` | no — needs `Nonblocking` delete, which needs a source form for deleting *from inside a callback*; v1 has no callback body |
| `GetSocketError` | a borrow of a descriptor | no — nothing in v1 needs it; the model already refuses it on a closed or poisoned number |
| `CreateBufferevent` … `FreeBufferevent`, `SetWatermark`, `BufferOutput`, `DrainOutput` | the whole byte-buffer tier | no — card `STD-BUFFER` owns that tier (card boundary) |
| `Access` | payload access under callback authority, with conflict detection (`L05`) | no — v1 callbacks carry no payload; reachable only with a source-level callback body |
| `AssumeOrdering` | records an ordering assumption and refuses it (`L08`) | no — nothing in v1 assumes an order |
| `LoopRequest` | `event_loopbreak`/`loopexit`/`loopcontinue` custody (`L11`) | no — v1's turn structure is explicit, so there is nothing to request |
| `DeclaredCloseFailure` | the target-qualified `close(2)` post-state, as a declared platform premise | no — v1's `close` is the success branch; a failed close is a run-time disagreement here |

**The one thing the surface adds that the reactor does not model: byte flow.**
`wake D;` writes a byte, and the reactor has no operation for it. That is
recorded rather than hidden — `wake` projects to **no** `EventOperation`, and
the Lean theorem `erasing_the_wakes_leaves_the_projection_unchanged` is the
strong form: the reactor's answer for a program is identical to its answer for
the same program with every readiness induction removed.

## 2. The surface

```qcore
import std.event;

fn main() -> i32 effects { io } {
    event open;
    endpoints a b;
    watch reader a read;
    wake a;
    dispatch ready reader;
    dispatch idle;
    unwatch reader;
    close a;
    close b;
    event close;
    return 0;
}
```

That is `examples/events/self_pipe.qcore`, and it writes:

```
QCORE_EVENT_OPEN
QCORE_EVENT_ENDPOINTS 0 1
QCORE_EVENT_WATCH 0 0 READ
QCORE_EVENT_WAKE 0
QCORE_EVENT_DISPATCH 0
QCORE_EVENT_DISPATCH IDLE
QCORE_EVENT_UNWATCH 0
QCORE_EVENT_CLOSE 0
QCORE_EVENT_CLOSE 1
QCORE_EVENT_TEARDOWN
QCORE_EVENT_RESULT 0
```

| Statement | Reactor projection | v1 rule |
|---|---|---|
| `event open;` | `CreateBase { base: 0, profile: SingleThreadedPollV1 }` | one context per entry, before every other event statement |
| `event close;` | `FreeBase { base: 0 }` | after all of them; the reactor's `L11` refuses a non-quiescent teardown |
| `endpoints a b;` | `CreateSocketPair { Some((0, 1)) }` | binds two fresh names to the two owned endpoints; both must be closed |
| `watch e a read;` | `Assign { event: (0,1), descriptor: Some(0), flags: READ, priority: 0, summary }` + `Add` | the custody edge; the cookie is `Owned` and retired by `unwatch` |
| `watch e a write;` | the same with `flags: WRITE` | **check-only**: checked in full, emits no C |
| `unwatch e;` | `Delete { Ordinary }` + `FreeEvent` | every registration unwatched before `event close;` |
| `wake a;` | *(none)* | adds one undrained readiness on `a` |
| `dispatch ready e;` | `BeginDispatch` + `Activate { Read }` + `CallbackEnter` + `CallbackExit` + `EndDispatch` | exactly one watched descriptor ready, and `e` is its registration; the callback drains one byte |
| `dispatch idle;` | `BeginDispatch` + `EndDispatch` | no watched descriptor ready |
| `close a;` | `CloseSocket { succeeds: true }` | the reactor refuses a close under a live registration (`L03`) |

**Every registration carries priority `0`.** Not incidental: a priority order
would be a source-declared ordering premise, and v1 declares none — which is
exactly why a two-member ready set is *refused* rather than *ordered* (§3).

**There is no `let` and no scalar operand.** The grammar has no value vocabulary
at all, so every name it resolves is a custody name; that is why the derived
effect row is exactly `{Io}` and why `Alloc` and `Mut` are unreachable by
construction rather than by a rule.

### 2.1 The U3 unblocking, stated exactly

Before this card the only producers of an `EventOperation` list were the
in-process Libevent provider adapter and hand-written Rust tests; **no source
program could reach the reactor or descriptor-custody models at all**. That is
the recorded U3 source-unreachability blocker, and it is retired: a
`source.event.v1` module's custody verdict *is* `check_event_program`'s verdict
over the projection, and the end-to-end tests assert that a reactor refusal
reaches a source span carrying the model's own diagnostic name rather than a
paraphrase.

Two honest limits on that statement. The surface reaches **twelve of the
nineteen** operation families §1 lists (the eleven marked *yes* plus
`DropArgument`, reached through `FreeEvent`), not all of them; and reaching a
model is not
the same as exercising every rule in it — the bufferevent tier, the payload
access rules and the declared-close-failure vocabulary stay unreached, each with
a ground above.

`STD-FMT-DESCRIPTOR`'s blocker moves accordingly: the `borrow NAME` rendering it
wants needs a checked custody model that publishes a live identity, and this
route now is one. Scoping and landing that is **that card's**, not this one's;
it is recorded here only so the dependency is visible.

## 3. The determinism boundary, and its grounds

The translation certificate compares **one** observation. Three devices make
that well-defined, one thing is refused by name, and the platform claim under
all of it is spelled as a premise.

| Device | Where | Precedent |
|---|---|---|
| The program induces its own readiness | `wake D;` writes one byte into the peer of a pair **this program created**. No external peer, no timer, no signal. | self-pipe fixture |
| Declared **and verified** outcome | `dispatch ready E;` / `dispatch idle;` declare what the turn reports; the emitted C compares what `poll` actually reported against the declaration and exits non-zero on disagreement. | `succeeds INTEGER` (`STD-IO-COMPOSE`) |
| Accept, check, do not execute | a `write` interest is checked in full and the module emits **no** C and publishes **no** reference observation. | `accept_check_only` (CT34); the epoch route is already check-only |

### 3.1 What is refused by name

> **Two or more registrations ready in one dispatch turn** — `Q054`, with the
> ground in the message.
>
> `poll(2)` reports a *set*. The order a reactor runs one ready set's callbacks
> in is the implementation's, not one the program induced — **the existing model
> says so itself**: `event_reactor::explore_activation_orders` is documented
> "equal-priority events are deliberately nondeterministic". A certificate over
> one observation cannot cover a set with more than one member. Covering it
> waits on comparison up to an admitted reordering (card `OBS-REORDERING`).

**v1's boundary does not depend on that card landing.** It is a statement about
what this route admits, decided entirely inside this route's own ledger, and it
composes with a reordering-aware comparison later rather than blocking on one.
The refusal is on the **state**, not the spelling: a `dispatch ready E;` written
where two watched descriptors have undrained readiness is refused whichever
registration `E` names, and so is the idle declaration. Three Lean theorems say
exactly that (`two_ready_registrations_admit_no_turn`,
`…_admit_no_idle_turn`, and the three concrete `two_induced_readinesses_refuse_…`
checks).

### 3.2 The premise, spelled as a premise (standing constraints 26–28)

> **`EVENT-V1-LEVEL-TRIGGERED-READINESS`** — a `poll(2)`/`WSAPoll` wait on a
> socket-pair endpoint with `POLLIN` requested reports `POLLIN` when, and only
> when, a byte written into its peer has not yet been read, given that this
> process is the sole holder of both endpoints and no other thread touches them.

It is a **declared premise about the platform**, not a proof and not an
observation — the same kind `event_reactor::DeclaredCloseFailure` already is.
Two things keep it honest:

* it is carried in the certificate as a named assumption
  (`StdEventCertificate::premises`), so a reader sees which platform claim a
  verdict rests on, and a program that performs no wait carries **none** rather
  than a decorative one; and
* **the emitted program checks it rather than trusting it.** Each `dispatch`
  emits `qcore_event_wait`, which compares the reported ready set against the
  declared one and returns non-zero, and the emitted program then exits with a
  distinct status. A false premise therefore produces a *failed run*, never a
  differently-shaped trace and never a wrong verdict.

No durability, no ordering between the two endpoints' byte streams, no claim
about `poll` on anything that is not a program-created pair, and nothing about
edge-triggered readiness.

### 3.3 The readiness ledger is a model of this program, not of the kernel

The route tracks, per endpoint, how many `wake`s are outstanding and undrained.
It says "this program wrote one byte into the peer of `a` and has not read it
back" — a fact about the program — and only the premise above turns that into a
prediction about `poll`. The Lean theorems
`readiness_comes_only_from_a_wake` and `a_program_with_no_wake_admits_no_ready_turn`
are the formal content of that separation.

## 4. The native half — which one landed, and why

**Route (a), the embedded shim, in full — and it is now the required baseline
for all three platforms** (integrator amendment during this card). Route (b),
Libevent, is a named follow-on (§7.1).

`crates/qcore-core/src/qcore_event_poll_compat.h` is embedded into the emitted C
with `include_str!`, on the `c11_threads_compat.h` precedent, and has the same
three-arm shape that shim uses.

### 4.1 Arm-by-arm status

| Arm | Selected by | Wait / pair primitive | Descriptor type | Verification status |
|---|---|---|---|---|
| **Linux** and other POSIX | `#else` | `poll(2)` / `socketpair(AF_UNIX, SOCK_STREAM, 0)` | `int` | **Runtime-verified in this card's gate**: compiled `-std=c11 -Wall -Wextra -Werror -pedantic`, run under ASan/UBSan/**LSan**, stdout byte-compared with the reference observation, and separately byte-compared with the transcript `examples/events/README.md` publishes. |
| **Apple** | `__APPLE__` | the same POSIX pair, behind `_DARWIN_C_SOURCE` | `int` | **Compile path designed in; runtime not measured by this card.** The same workspace test (`the_emitted_program_reproduces_the_reference_observation`, `cfg(not(windows))`) runs it when the standing gate runs on macOS. Residual **`EVENT-V1-APPLE-RUN`**. |
| **Windows** | `_WIN32` | `WSAPoll` / loopback listen-connect-accept | `SOCKET` | **Compile path designed in; runtime not measured by this card.** `examples/events/build-windows.ps1 -Run` is its runner, on the `examples/threads/build-windows.ps1` shape, and it reads the expected trace **out of the README** rather than carrying a copy. Residual **`EVENT-V1-WINDOWS-RUN`**. |

The two residuals follow the `TRACE-STDIO-WIN-DIGEST` / `-WIN-RUN` precedent:
a named, recorded gap rather than a silence, and rather than a Linux-only green
gate implying a claim about the other two. The user's Windows host runs
validation batches that will exercise the Windows arm; that run is the residual's
discharge, and this card does not pre-announce its result.

### 4.2 Why `poll(2)` and not `kqueue`/`epoll`

`poll(2)` is the portable deterministic subset and is what both POSIX platforms
document for a socket-pair endpoint. v1's argument is about a **declared** ready
set of at most one member, for which neither interface's scalability buys
anything, and each would have made the Apple and Linux arms genuinely different
implementations rather than genuinely different declarations. A test asserts
neither appears in an emitted program.

### 4.3 The descriptor type, decided per arm and contained

The POSIX arms use `int` from `socketpair(2)`. The Windows arm uses `SOCKET` —
an unsigned handle, **not** a CRT file descriptor — because Windows has no
`socketpair(2)` and `WSAPoll` takes `SOCKET`s. Both are behind one
`qcore_event_fd` typedef, and:

**the trace carries neither.** Every identity an emitted program prints is the
model identity the **checker** assigned. A real descriptor number is not
model-determined — the same program run twice may get different numbers — so a
trace carrying one could not be compared against a reference at all. This is the
discipline `STD-FMT-BORROW` fixed for a borrowed render, and it is also why the
published transcript is byte-identical on all three arms, which is what makes
the Windows runner's README comparison meaningful.

### 4.4 The Windows subset, named rather than faked

Two differences, both stated in the header:

* the pair is a private **loopback TCP** connection rather than an AF_UNIX
  socket pair: still two-ended, still owned entirely by this process, same
  one-byte wake/drain semantics — different provenance;
* `WSAPoll`'s documented `POLLOUT`-on-connect defect is **unreachable** from an
  emitted v1 program, because a write interest makes a module check-only, so
  `QCORE_EVENT_POLLOUT` never reaches a program that runs. That is a consequence
  of §3's check-only arm rather than a Windows-specific narrowing.

`#pragma comment(lib, "ws2_32.lib")` keeps the Windows arm as self-contained as
the other two: no build command anywhere needs a library flag, and a test
asserts the PowerShell runner adds none.

## 5. Diagnostics and the band assumption

New semantic pass `Event`, owning **`Q051`–`Q055`** (spec §L21, added by this
card).

| Code | Condition |
|---|---|
| `Q051` | the flat entry shape this profile models, and the pass's generic presentation code |
| `Q052` | a name that is not a live endpoint or registration of this entry |
| `Q053` | the **reactor model** refused the projected program — its own `L00`–`L12` diagnostic, at the statement whose projection named it |
| `Q054` | the **determinism boundary**, including the by-name two-member refusal |
| `Q055` | the declared `effects { … }` row does not admit the derived one |

`Q015` is reused for a missing `import std.event;`, exactly as `STD-MODULES`
reuses it, with the message naming the module. No new import code is minted.

**Band assumption, stated as the brief asks.** `Q050` (`ComposedTasks`) is the
highest code allocated on `ec5535a9`, so `Q051`+ is the next free range. Card
`STD-BUFFER` of the same wave may also claim from it. **A collision discovered
at integration relocates this unshipped band**, on the wave-3 `T07`/`T09`
precedent that `PIPE-OPERATOR` and `NATIVE-LIBRARY-CONTRACT-SLICE` both
followed; the code-to-condition mapping above is stable under such a shift, and
`SemanticPass::Event`'s doc comment says so at the definition site.

## 6. The Mio/Tokio non-binding, recorded

`std.event` v1 is **not** a binding to a Rust async runtime — not Tokio, not
Mio, not `async-std`, not `smol`. Three grounds, recorded in
`docs/design/STD_EVENT_V1.md` §1 and summarised here:

1. **Rust is the checker, not the runtime.** Every executable route emits
   self-contained C11 built by the target's own toolchain with no QCore library
   and no include path. A program whose event loop lived in a Rust crate would
   be the first emitted program that cannot be built from the emitted `.c`
   alone.
2. **Neither has a stable C ABI.** They expose Rust generics, trait objects and
   `async fn` state machines. Binding them means a hand-rolled `extern "C"`
   shim — a *new uncontracted native library*, level L0/L1 of
   `docs/design/UNSAFE_LINKAGE_V1.md` — putting every verdict behind an
   unchecked-linkage waiver. That is the opposite of this card's purpose, which
   is to make an existing **checked** model reachable.
3. **Scheduler nondeterminism breaks the one-observation certificate.** A
   work-stealing multi-threaded scheduler does not produce one observation; it
   produces one of many admissible interleavings. Comparing against it needs the
   comparison-up-to-an-admitted-reordering capability `OBS-REORDERING` is scoped
   to build.

None of the three says such a binding is impossible or undesirable. §7.2 records
what would have to exist first.

## 7. Follow-ons

### 7.1 The other native half — Libevent (route (b)), not landed

What was learned:

* the **model** side is already done: the in-process first-party provider
  (`compiler/src/verifier_plugins/libevent/`) produces `EventOperation` lists
  from the pinned submodule's audited bodies;
* the gap is the **contract** side. Binding `event_base_new`, `event_assign`,
  `event_add`, `event_base_loop`, `event_del`, `event_free` and
  `event_base_free` read-against-body faithfully is a new `.qcore` module, and a
  new module under `contract/qcore` moves the pinned per-file count tables in
  `compiler/tests/qcore_contract_corpus.rs` (invariant 5). The brief asks for
  corpus-neutrality, so v1 stays outside that directory and **no pinned count
  moved**;
* the identity would then flow through the L3 manifest machinery of
  `NATIVE_LIBRARY_LINKAGE_V1.md` §L14.3, which exists and is the right home.

With the amendment, the three-arm shim is the **required baseline** and a
Libevent binding is additive rather than an alternative to it.

### 7.2 A Rust-runtime binding

Requires, in order: `OBS-REORDERING`'s comparison up to an admitted reordering;
a stable `extern "C"` surface, which today means an uncontracted shim behind an
unchecked-linkage waiver; and a decision about what `Safe` would mean relative to
a runtime nothing in this project checked.

### 7.3 Named, smaller

* **`EVENT-V1-WINDOWS-RUN`** and **`EVENT-V1-APPLE-RUN`** — the two unmeasured
  arms of §4.1.
* **Two-member ready sets**, once `OBS-REORDERING` lands. v1's boundary composes
  with it rather than blocking on it.
* **Write readiness executable** rather than check-only — needs a way to induce
  send-buffer pressure deterministically.
* **A callback body.** v1's callback drains one byte and does nothing else; a
  source-level body is where `Access`, `L05` conflict detection and the
  payload-authority half of the model become reachable.
* **A buffer payload** — card `STD-BUFFER`'s tier; v1's payloads are descriptors
  and nothing else.
* **A declared close failure**, surfacing `DeclaredCloseFailure`'s closed
  vocabulary at the source level. The model half exists and is proved.
* **`Finalize` / `Nonblocking` delete**, which need deletion from inside a
  callback.
* **Composing `source.event.v1` with `source.std-io.v1`.** A `print` inside a
  dispatch turn is exactly the shape a reordering-aware comparison would have to
  cover, which is why it is a follow-on and not a convenience.
* **`STD-FMT-DESCRIPTOR`** is unblocked by §2.1 but is not scoped here.

## 8. Reserved words — the standing-constraint-14 census

Eight added: `event`, `endpoints`, `watch`, `unwatch`, `wake`, `dispatch`,
`ready`, `idle`. Four existing words reused rather than respelled: `open`,
`close`, `read`, `write` — which is why the context pair is spelled
`event open;` / `event close;` rather than with two words of its own. Eight is
the same number `STD-MODULES` added.

Census over every `.qcore`/`.qc` file in the tree and every inline source string
in the Rust tests, at `ec5535a9` (before this card's own files):

| Word | Occurrences | What they are |
|---|---|---|
| `event` | 6 | 1 contract ghost field (`ensures normal event == registered_event;`), **2 extern-declaration parameter names** (`event: borrow<'call, mut, MonkeyEvent> use one`), 3 prose comments |
| `ready` | 2 | prose comments only (`sample_record.ready`) |
| `endpoints` | 1 | prose comment only |
| `watch`, `unwatch`, `wake`, `dispatch`, `idle` | 0 | — |

**No collision, and the one that looks like one is worth naming.** The two
`event` parameter names in `contract/qcore/monkey/wrapper_paths.qcore` are in a
*declaration parameter* position, which the declaration parser reads and
`is_reserved_source_word` never touches — that predicate governs
**statement-body name resolution** only. The contract ghost field is likewise
inside a `contract { … }` clause. The pinned contract-corpus tables are unmoved
and the full workspace gate is green, which is the measurement rather than the
argument.

Route selection reads the same detector as the import gate, over **statement
heads inside function bodies only**, so a contract module's ghost field named
`event` is invisible to both — the discipline `concurrency_span` already follows
for a ghost field named `thread`. `close` is deliberately **not** a selecting
keyword: it is an existing statement head of the checked mutex-guard profile,
and selecting on it would route a mutex module here.

## 9. What this card did not touch

Corpus-neutral by construction: no gateway, audit, contract-corpus or provider
change, no new module under `contract/qcore`, no pinned count moved, no frozen
archive edited. The leak pin stays 272 over the 185-unit corpus and the CT
mapping stays 33/50 — neither is a subject of this card.

Two shared files were extended rather than rewritten:
`crates/qcore-frontend/src/frontend.rs` gained one `StatementKind::Event`
variant (which records **only the names a statement binds**, because an endpoint
pair introduces two names in one statement and the existing single-binding tuple
cannot carry two), and `examples/README.md` gained one row.

## 10. The standing gate, as measured

| Gate step | Result |
|---|---|
| `cargo test --workspace` | **3,286 tests, 8 ignored, 0 failures** across 170 binaries |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **exit 0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **exit 0** |
| `python3 scripts/check_lean_audits.py` | **91/91 audits passed** (90 → 91: `AuditStdEvent.lean`) |
| `python3 -m unittest discover -s scripts/tests` | 193 tests, **1 failure** — see below |

Two things happened during the gate that are worth recording rather than
smoothing over.

**The confined-fixture flake tripped once and passed in isolation.** The full
run failed `a_confined_fixture_cannot_write_outside_the_admitted_paths`, which
is standing constraint 16's flake row 3 verbatim (`Io(Broken pipe)` under load).
Rerun on its own with `--test-threads=1`: **12 passed, 0 failed**. The machine
was carrying five sibling agents at the time, which is exactly the condition
that row names. Nothing was weakened.

**The submodule was uninitialised in this fresh worktree**, so
`libevent_submodule_matches_the_audited_revision` failed on the first run. That
is invariant 5's pin doing its job on an environment gap, not a code failure:
`git submodule update --init --recursive` (the gate's own first line) checked
out `d82464a2…` and the pin passes. Recorded so the first number above is read
as "after the documented one-time setup step".

**The one standing Python failure is an integrator reconciliation, not a
defect:** `test_the_derived_facts_are_recomputed_rather_than_carried` compares
the committed `docs/reviews/r0_3/acceptance_matrix.json` against the live tree,
and `validation.lean_audits` is a **derived** field that this card moves 90 → 91
by adding one `Audit*.lean`. That file is integrator-owned and outside this
card's boundary, so it is deliberately untouched here; regenerating its derived
section (constraint 25's `section_ownership` discipline — derived fields are
recomputed, integrator-supplied and carried sections are not clobbered) closes
it. The `lean_audit_files` list moves with it.

## 11. Evidence

| Artefact | Where |
|---|---|
| Design record (written first) | `docs/design/STD_EVENT_V1.md` |
| Checked model | `crates/qcore-core/src/std_event.rs` |
| Reference execution and C emission | `crates/qcore-core/src/std_event_c_emit.rs` |
| The three-arm embedded shim | `crates/qcore-core/src/qcore_event_poll_compat.h` |
| Source lowering | `crates/qcore-frontend/src/source_event.rs` |
| Route | `crates/qcore-verification/src/driver.rs::compile_event_source` |
| Lean | `proof/QCore/StdEvent.lean`, `proof/AuditStdEvent.lean` (33 entries) |
| Model tests | `crates/qcore-core/tests/unit/std_event.rs` (21) |
| End-to-end tests | `compiler/tests/std_event.rs` (26, one of them the sanitized native run) |
| Examples | `examples/events/` — two programs, the page, the Windows runner |
| Spec | `docs/QCORE_SPEC_r0_3.md` §L21 row `Q051`–`Q055`; `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` §G02k |
