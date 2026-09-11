# `std.event` v1 — a checked source surface over the existing reactor models

Card `STD-EVENT` (wave 22). This document is the design record; the report of
what actually landed is `docs/reports/wave22-std-event/README.md`.

Written **before** the implementation, per the card brief, and left in place
unedited except where the implementation measured something the design assumed.
Every such correction is marked **measured**.

---

## 1. What this is not

**`std.event` v1 is not a binding to a Rust async runtime.** Not Tokio, not
Mio, not `async-std`, not `smol`. The decision and its grounds, recorded here so
that a later reader does not have to re-derive them:

1. **Rust is the checker, not the runtime.** Every executable route in this
   project emits self-contained C11 that is compiled by the target's own
   toolchain and run with no QCore library, no include path and no linked
   artifact (`crates/qcore-core/src/c11_threads_compat.h` is embedded into the
   emitted text for exactly this reason). A program whose event loop lived in a
   Rust crate would be the first emitted program that cannot be built from the
   emitted `.c` alone.
2. **Neither Tokio nor Mio has a stable C ABI.** They expose Rust generics,
   trait objects and `async fn` state machines. Binding them means writing and
   maintaining a hand-rolled `extern "C"` shim — which is a *new uncontracted
   native library*, i.e. level L0/L1 of `docs/design/UNSAFE_LINKAGE_V1.md`, and
   would put every verdict behind an unchecked-linkage waiver. That is a real
   route this project supports, but it is the *opposite* of what this card is
   for: the card exists to make an existing **checked** model reachable.
3. **Scheduler nondeterminism breaks the one-observation certificate.** Every
   executable route here compares **one** reference observation against **one**
   run of the emitted program (`interpret_*` against the compiled binary). A
   work-stealing multi-threaded scheduler does not produce one observation; it
   produces one of many admissible interleavings. Comparing against it needs the
   comparison-up-to-an-admitted-reordering capability that card `OBS-REORDERING`
   is scoped to build. Until that lands, a route that admits scheduler
   nondeterminism cannot publish a certificate at all.

None of the three is a statement that such a binding is impossible or
undesirable; they are the reasons it is **not this card**. §8 records it as a
follow-on with what would have to exist first.

## 2. What it is

Two halves, and they are independent.

### 2.1 The checked surface: the existing models, made reachable

`crates/qcore-core/src/event_reactor.rs` and
`crates/qcore-core/src/os_endpoint.rs` already carry a generation-indexed
callback lifecycle, a descriptor custody model and a target-qualified `close(2)`
post-state relation, with matching Lean in `proof/QCore/DescriptorCustody.lean`
and `proof/QCore/OsEndpointPostState.lean`. Until this card **no source program
could reach any of it**: the only producers of an `EventOperation` list were the
Libevent provider adapter and hand-written Rust tests. That is the recorded
**U3 source-unreachability blocker**.

v1's surface is the *honest projection* of what those models already express.
§4 is the census; §5 is the surface table. The rule the census imposes: a
statement is admitted only where the reactor model already decides its custody
consequence. Nothing is invented beside the model, and nothing the model refuses
is admitted by the surface.

### 2.2 The native half: an embedded `poll(2)` shim

The deterministic executable subset emits a self-contained C11 program with an
embedded compatibility header — `poll(2)` on POSIX, `WSAPoll` on Windows — on
the `c11_threads_compat.h` precedent: `include_str!`-ed into the emitted text,
no include path, no library, `-Werror`-clean.

Libevent (route (b) of the brief) is **not** bound in v1. §8.2 records why and
what it would take.

## 3. The determinism boundary — the hard part, stated first

The translation certificate compares **one** observation. A route that dispatches
on real readiness must therefore make "the" observation well-defined. v1 does it
with all three of the devices the brief names, each in its own place:

| Device | Where v1 uses it | Precedent |
|---|---|---|
| The program induces its own readiness | `wake D;` writes one byte into the peer of a program-created endpoint pair. No external peer, no timer, no signal. | self-pipe fixture |
| Declared **and verified** outcome | `dispatch ready E;` / `dispatch idle;` declare what the turn reports; the emitted C compares `poll`'s actual ready set against the declaration and exits non-zero on disagreement. | `succeeds INTEGER` (`source.extern-call.v1`, card `STD-IO-COMPOSE`) |
| Accept, check, do not execute | a `write` interest is checked in full and the module emits **no** C and publishes **no** reference observation. | `accept_check_only` (card `CT34-*`); the epoch route is already check-only |

And one thing is refused **by name**, with the ground stated in the message:

> **Two or more registrations ready in one dispatch turn.**
> `poll(2)` reports a *set*. The order in which a reactor runs the callbacks of
> one ready set is the implementation's, not one the program induced — the
> existing model says so itself:
> `event_reactor::explore_activation_orders` is documented "equal-priority
> events are deliberately nondeterministic". A certificate over one observation
> cannot cover a set with more than one member. This waits on the
> comparison-up-to-an-admitted-reordering capability (card `OBS-REORDERING`).
> **v1's boundary stands alone and does not depend on that card landing.**

The refusal is on the *state*, not on the spelling: a `dispatch ready E;`
written where the route's readiness ledger holds two outstanding wakes on
watched descriptors is refused, whichever event `E` names.

### 3.1 What a "readiness ledger" is, and what it is not

The route tracks, per program-created descriptor, how many `wake`s are
outstanding and un-drained. It is a **model of what this program did to
itself**, not a model of the kernel: the ledger says "this program wrote one
byte into the peer of `a` and has not read it back", from which — under the
premise in §3.2 — the route concludes that a level-triggered read registration
on `a` reports ready.

### 3.2 The premise, spelled as a premise (constraints 26–28)

> **`EVENT-V1-LEVEL-TRIGGERED-READINESS`.** A `poll(2)`/`WSAPoll` wait on a
> socket-pair endpoint with `POLLIN` requested reports `POLLIN` when, and only
> when, a byte written into its peer has not yet been read, given that this
> process is the sole holder of both endpoints and no other thread touches them.

That is a **declared premise about the platform**, exactly the kind
`event_reactor::DeclaredCloseFailure` already is, and it is **not a proof**. It
is what licenses the route to derive a reference observation at all. Two things
keep it honest:

* it is carried in the certificate as a named assumption, so a reader sees which
  platform claim a verdict rests on; and
* the emitted program **checks it at run time** rather than trusting it: the
  actual ready set is compared against the declaration and a disagreement is a
  non-zero exit, not a silently different trace. A false premise therefore
  produces a *failed run*, never a wrong `Safe`.

No durability, no ordering between the two endpoints' own byte streams, no
claim about `poll`'s behaviour on anything that is not a program-created socket
pair, and no claim about edge-triggered readiness is made or measured.

## 4. Census — what the existing models can already express

Read off `EventOperation<P>` and the checker in
`crates/qcore-core/src/event_reactor.rs`. "Reachable in v1" is this card's
answer, and every `no` has a ground.

| Model operation | What it decides | v1 |
|---|---|---|
| `CreateBase` | one reactor context under an accepted profile (`L09`) | **yes** — `event open;` |
| `FreeBase` | teardown; refuses while a dispatcher, a running callback, a live event or a live bufferevent remains (`L11`) | **yes** — `event close;` |
| `BeginDispatch` / `EndDispatch` | one dispatch turn; `EndDispatch` refuses unless quiescent (`L06`) | **yes** — bracketed around each `dispatch` |
| `Assign` | a registration: base, **descriptor**, flags, priority, callback summary; refuses a closed descriptor (`L03`), an invalid flag set, a trigger the flags do not permit, a dead argument (`L02`) | **yes** — `watch E D read;` |
| `Add` / `Reschedule` | make a registration pending; refuses on a closed descriptor | **yes** — with `Assign` |
| `Activate` | readiness arrives for a permitted reason; `cross_thread` needs a threading profile (`L04`) | **yes** — from `dispatch ready E;` |
| `CallbackEnter` / `CallbackExit` | run one callback; enter refuses unless the base is dispatching, nothing else is running, the argument is live and **this event has the minimum priority among the active ones** (`L06`) | **yes** — from `dispatch ready E;` |
| `Delete` | deregister; `Nonblocking` from inside a running callback | **yes** — `unwatch E;` (`Ordinary` only) |
| `FreeEvent` | free the registration; drops an `Owned` argument | **yes** — with `Delete` |
| `CreateSocketPair` | two distinct owned descriptors, or the failure branch with no custody | **yes** — `endpoints A B;` (success branch only) |
| `CloseSocket` | release; the failure branch consults the target-qualified relation | **yes** — `close D;` (success branch only) |
| `DropArgument` | retire a callback cookie no live registration references (`L02`) | via `FreeEvent` on the `Owned` cookie |
| `Finalize` | the finalize protocol for `EVLIST_FINALIZING` | no — needs `Nonblocking` delete, which needs a source form for *deleting from inside a callback*; v1 has no callback body |
| `GetSocketError` | a borrow of a descriptor | no — nothing in v1 needs it; the model already refuses it on a closed or poisoned number |
| `CreateBufferevent` … `FreeBufferevent`, `SetWatermark`, `BufferOutput`, `DrainOutput` | the whole byte-buffer tier | no — card `STD-BUFFER` owns the byte-buffer tier (boundary in the brief) |
| `Access` | payload access under callback authority, with conflict detection (`L05`) | no — v1 callbacks have no payload; the surface would need a shared-state form |
| `AssumeOrdering` | records an ordering assumption and refuses it (`L08`) | no — nothing in v1 assumes an order |
| `LoopRequest` | `event_loopbreak`/`loopexit`/`loopcontinue` custody (`L11`) | no — v1's turn structure is explicit, so there is nothing to request |
| declared close failure (`DeclaredCloseFailure`) | the target-qualified `close(2)` post-state | no — v1's `close` is the success branch; a failed close is a **run-time** disagreement, and v1 has no source form for a declared platform premise |

**The one thing the surface adds that the reactor does not model: byte flow.**
`wake D;` writes a byte, and the reactor has no operation for it. This is
recorded rather than hidden: `wake` projects to **no** `EventOperation`, it
moves only the route's own readiness ledger (§3.1), and the reactor's custody
answer for a program with and without it is identical. Card `STD-BUFFER` owns
the tier where bytes are modelled.

## 5. The surface

```text
module ::= import* entry
import ::= "import" "std.event" ";"
entry  ::= "fn" NAME "(" ")" "->" "i32"
           ["effects" "{" EFFECT {"," EFFECT} "}"] "{" stmt* "}"

stmt   ::= "event" "open" ";"
         | "event" "close" ";"
         | "endpoints" NAME NAME ";"
         | "watch" NAME NAME INTEREST ";"
         | "unwatch" NAME ";"
         | "wake" NAME ";"
         | "dispatch" "ready" NAME ";"
         | "dispatch" "idle" ";"
         | "close" NAME ";"
         | "return" ["-"] INTEGER ";"

INTEREST ::= "read" | "write"
EFFECT   ::= "io" | "alloc" | "mut"
```

There is deliberately **no `let`** and no scalar operand: v1 has no value
vocabulary at all. Every name in the grammar denotes an endpoint or a
registration, so no statement of this route can render, compute or carry a
value, and the only names it resolves are custody names. A route that needs to
print one composes with `source.std-io.v1` (§8.3).

| Statement | Reactor projection | v1 rule |
|---|---|---|
| `event open;` | `CreateBase { base: 0, profile: SingleThreadedPollV1 }` | at most one context per entry; before every other event statement |
| `event close;` | `FreeBase { base: 0 }` | last event statement; the reactor's own `L11` refuses a non-quiescent teardown |
| `endpoints a b;` | `CreateSocketPair { descriptors: Some((fd_a, fd_b)) }` | binds two names to the two owned endpoints; both must be `close`d |
| `watch e a read;` | `Assign { event: (e,1), descriptor: Some(fd_a), flags: READ, priority: 0, summary }` + `Add` | one registration per name; the cookie is `Owned` and retired by `unwatch` |
| `watch e a write;` | same with `flags: WRITE` | **check-only**: admitted and checked, but the module emits no C (§3) |
| `unwatch e;` | `Delete { mode: Ordinary }` + `FreeEvent` | every registration must be unwatched before `event close;` |
| `wake a;` | *(none)* | `a` must be a live endpoint; adds one outstanding wake on **the peer's** readiness |
| `dispatch ready e;` | `BeginDispatch` + `Activate { reason: Read }` + `CallbackEnter` + `CallbackExit` + `EndDispatch` | admitted only when exactly one watched descriptor is ready and `e` is its registration; the callback drains one byte |
| `dispatch idle;` | `BeginDispatch` + `EndDispatch` | admitted only when no watched descriptor is ready |
| `close a;` | `CloseSocket { succeeds: true, declared_failure: None }` | the reactor refuses a close under a live registration (`L03`) |

Priorities are all `0` in v1, which is what makes `CallbackEnter`'s
minimum-priority rule trivially satisfiable and is *also* why a two-member ready
set is refused rather than ordered: a priority order would be a source-declared
ordering premise, and v1 declares none.

## 6. Diagnostics — band assumption

A new semantic pass `Event`, owning **`Q051`–`Q055`** (spec §L21). The band
assumption, stated per the brief: `Q050` is the highest code allocated on the
base this card was cut from (`ComposedTasks`, card `COMPOSE-CONCURRENCY`), so
`Q051`+ is the next free range. Card `STD-BUFFER` of the same wave may also
claim from it. **A collision discovered at integration relocates the unshipped
band**, on the wave-3 `T07`/`T09` precedent that `PIPE-OPERATOR` and
`NATIVE-LIBRARY-CONTRACT-SLICE` both followed; the code-to-condition mapping
below is stable under such a shift.

| Code | Condition |
|---|---|
| `Q051` | the shape this profile models is violated (the generic presentation code of the pass) |
| `Q052` | a name that is not a live endpoint, registration or scalar of this entry |
| `Q053` | the reactor model refused the projected program — the `L00`–`L12` diagnostic, attributed to its statement |
| `Q054` | the **determinism boundary**: a dispatch turn whose declared readiness is not the one this program induced, including the by-name refusal of a two-member ready set |
| `Q055` | the declared `effects { … }` row does not admit the derived one |

`Q015` (a capability used without its file-local import) is reused for a missing
`import std.event;`, exactly as `STD-MODULES` reused it for the three IO
modules. No new import diagnostic code is minted.

## 7. Reserved words — the constraint-14 census

Eight new reserved words: `event`, `endpoints`, `watch`, `unwatch`, `wake`,
`dispatch`, `ready`, `idle`. Four existing ones are reused rather than
respelled: `open`, `close`, `read`, `write` (all already reserved — `open` and
`close` by the guard profile, `read`/`write` by the loan vocabulary).

The census over every `.qcore`/`.qc` fixture in the tree, and over every inline
source string in the Rust tests, found **no collision**: the four occurrences of
`event` in the contract corpus are a ghost-field name inside `contract { … }`
clauses (`ensures normal event == registered_event;`) and prose comments, never
a statement head and never a binding; `ready`, `endpoints` and `turn` occur only
in comments. Report §7 carries the counts.

The word choice is constrained by that census in one visible place: the context
pair is spelled `event open;` / `event close;` rather than `event context;` /
`event teardown;` precisely so that two fewer words are reserved.

## 8. Follow-ons

### 8.1 The other native half — Libevent (route (b))

Not landed. What was learned:

* Libevent is a pinned submodule with audited bodies, and the in-process
  first-party provider (`compiler/src/verifier_plugins/libevent/`) already
  produces `EventOperation` lists from it — so the *model* side is done.
* The gap is the **contract** side: binding `event_base_new`, `event_assign`,
  `event_add`, `event_base_loop`, `event_del`, `event_free` and
  `event_base_free` read-against-body faithfully is a new `.qcore` contract
  module, and a new module under `contract/qcore` moves the pinned per-file
  count tables in `compiler/tests/qcore_contract_corpus.rs` (invariant 5). The
  brief asks for corpus-neutrality, so v1 stays outside that directory.
* The identity would then have to flow through the L3 manifest machinery of
  `docs/design/NATIVE_LIBRARY_LINKAGE_V1.md` §L14.3 — which exists and works,
  and is the right home, but is a second card's worth of contract authoring.

### 8.2 A Rust-runtime binding

Requires, in order: (1) `OBS-REORDERING`'s comparison-up-to-an-admitted-
reordering, so a scheduler's interleaving can be compared at all; (2) a stable
`extern "C"` surface, which today means an uncontracted shim behind an
unchecked-linkage waiver (level L0); (3) a decision about what a `Safe` verdict
would even mean relative to a runtime nobody in this project checked. Recorded
as declined for v1 with the grounds in §1.

### 8.3 Named, smaller

* **Two-member ready sets**, once `OBS-REORDERING` lands. v1's boundary composes
  with it rather than blocking on it.
* **Write readiness executable**, rather than check-only — needs a way for the
  program to induce send-buffer pressure deterministically.
* **A callback body.** v1's callback drains one byte and does nothing else; a
  source-level callback body is where `Access`, `L05` conflict detection and the
  payload-authority half of the model become reachable.
* **A buffer payload** — card `STD-BUFFER`'s tier.
* **A declared close failure**, surfacing `DeclaredCloseFailure`'s closed
  vocabulary at the source level; the model half already exists and is proved.
* **`Finalize` / `Nonblocking` delete**, which need deletion from inside a
  callback.
