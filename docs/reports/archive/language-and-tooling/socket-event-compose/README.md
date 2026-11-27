# SOCKET-EVENT-COMPOSE — a checked socket as a reactor-registered event source

**Card:** `SOCKET-EVENT-COMPOSE` (wave 25), the socket card's own named
follow-on. **Route:** `source.socket.v1`, widened; no new route.
**Diagnostic codes allocated: zero.**

**What shipped.** A module importing `std.socket` and `std.event` may now open a
reactor context, **register a socket it owns** with it, receive dispatch for it,
and do its `accept`/`recv`/`recvfrom` in its own body. One published example,
`examples/events/socket_server.qcore`, runs under ASan/UBSan/LSan on Linux with
its stdout byte-compared against the compiler's own reference execution. 44 new
end-to-end tests, 32 audited Lean entries in a new `QCore.SocketEventCompose`.

**Status language.** This card publishes no verdict about any pinned corpus unit
and moves no corpus number. Corpus-neutral by construction (§11).

**`HTTP-V1-EVENT-COMPOSE` is deliberately *not* landed** and stays a refusal
(§4, §12).

---

## 1. The custody relation, decided and argued

The card asked this to be decided: *is registration a loan of the descriptor to
the reactor, or a custody-neutral interest declaration?* The answer is the first,
and it is the answer **both existing models were already shaped to give**.

> **A registration is a loan of the socket. The descriptor's custody never leaves
> the socket route's ownership ledger.**

### 1.1 The reactor borrows, and its own model says so

`source.event.v1` projects `endpoints A B;` onto `CreateSocketPair` and
`close D;` onto `CloseSocket`, because on that route the reactor's model *is*
where the descriptors live. This composition's projection contains **neither**.
Those two are exactly the operations by which `event_reactor` takes ownership of
a number: `descriptor_custody` has only two writers, `CreateSocketPair` and the
bufferevent tier's `CreateBufferevent` (which this composition does not reach at
all), and `CloseSocket` refuses any descriptor that map does not hold. A socket created by
`let s = tcp;` is not the reactor's to create and not the reactor's to close, so
the projection says neither.

What it does say is `Assign { descriptor: Some(d), … }`, and the model already
admits that for a number it did not create — its descriptor test at `Assign` is
*"not a number this trace closed"*, not *"a number this trace owns"*. The borrow
posture is already named in the model's own documentation, on the neighbouring
operation: **"A socket-error query *borrows* a descriptor."** This card uses the
posture that was there rather than adding one.

Consequence, stated so it is not read as a gap: the reactor's
`L03`-on-a-closed-descriptor arm is **unreachable from this composition**,
because the closes it would need to see are the ownership ledger's. The same
programs are refused — a `watch` after a `close` is a use-after-drop of the
socket name — so nothing is admitted that was not. Recorded as
`SOCKET-EVENT-L03-REACH`, on the `EXTERN-F1-STORE-REACH` and
`HTTP-V1-REDECLARE-REACH` precedent: a fail-closed guard on a public model API,
kept and exercised directly rather than deleted.

### 1.2 The loan is the ownership IR's own borrow node

`watch E S read;` builds an **`OwnershipOp::BorrowShared { owner: S, borrower: E,
body }`**, whose body is every ownership operation up to the matching
`unwatch E;`. That is the whole custody contribution of this card, and it is
three lines of construction rather than a rule. Everything a reader wants to know
about a registered socket is then the **unedited** cycle-3 checker's answer:

| Written inside a registration's span | The unedited checker's answer | Reported at |
|---|---|---|
| `close S;` | `OwnershipError::DropWhileBorrowed` | `Q083` |
| `bind S;`, `listen S;`, `connect … S;` | `OwnershipError::OwnerMutationWhileBorrowed` — these are the ownership IR's **writes** on this route, and a shared loan is outstanding | `Q083` |
| `send S …;`, `recv S …;`, `recvfrom S …;`, `let x = accept S;` | **admitted** — these are the IR's **reads**, and a shared loan admits reads | — |
| a registration whose name a live binding already holds | `OwnershipError::BindingAlreadyExists` | `Q083` |

One coarseness is inherited rather than introduced: `SocketError::Custody`
carries the statement index `program.statements.len()`, so every custody refusal
on this route — this card's included — is attributed to the **entry** rather than
to the statement that earned it. That is `QUIRIN-SOCKET-V1`'s attribution and it
is unchanged here; sharpening it is `SOCKET-CUSTODY-ATTRIBUTION`, and per
constraint 1 it is not a code change.

Two tests show the first two arriving as those error values rather than as
re-worded copies (`a_close_while_registered_refuses_and_the_refusal_is_the_unedited_ownership_checkers`,
`reconfiguring_a_registered_socket_refuses_because_it_is_an_ownership_write`).

**So "deregister before close" is a consequence, not a rule this card wrote** —
and it is enforced twice over by two models that were both already there: the
ownership checker refuses the drop, and the reactor's terminal sweep refuses an
event that was never `FreeEvent`d (`L07`). The card's brief offered
"refused or auto-deregistered"; auto-deregistration was never a candidate once
the relation was read off the models, because it would have had this card decide
a custody transition neither model performs.

### 1.3 The one thing the loan costs, stated

A registered socket cannot be re-configured. In practice that orders a composed
program as *induce, then register, then dispatch*: `connect client listener;`
must precede `watch inbound listener read;`, because `connect` writes the
listener. That is not a workaround — it is a checkable statement that the
readiness a turn declares was decided before the registration existed — but it is
a real constraint and it is named. `SOCKET-EVENT-CONFIGURE-UNDER-LOAN` is the
follow-on for a route that wants to re-configure a registered socket; it needs a
reason to believe the registration's declared interest survives the change, which
this card has no way to state.

### 1.4 Lexical spans, and the one narrowing

The ownership IR's borrow nodes carry a **body**: their own documentation says
they "delimit lexical loan lifetimes". So a registration's span is lexical and
registrations retire in the reverse of the order they are taken. A crossed pair —
`watch a; watch b; unwatch a; unwatch b;` — has no such body and is **refused by
name** at `Q080` rather than approximated, with the follow-on
`SOCKET-EVENT-CROSSED-SPANS` in the message. Proper nesting is admitted and
pinned in both directions (`crossed_registration_spans_refuse_by_name`,
`properly_nested_registration_spans_are_admitted`).

This is a narrowing relative to what a reactor could express, it refuses more
than it must (invariant 1's safe direction), and it is stated rather than hidden.

### 1.5 What the certificate publishes

`SocketCertificate` gains five fields: `descriptors` (each socket's **model**
identity — never a real descriptor number, which is not model-determined),
`registrations`, `turns` (each with the **watched set** it was checked against —
the poll set the emitted program builds), `reactor` (the reactor model's own
certificate, `None` for a program that registers nothing), and `executable`. All
five are part of the independent recheck, so none is a value the driver did not
recompute.

`peak_live_descriptors` is read from a **borrow-free** projection of the same
operation list — a loan carries no footprint, so erasing the borrow nodes leaves
exactly the operations that move the ledger. On a program with no registration
that erasure is the identity, which is why the number is byte-identically the one
it was before this card.

---

## 2. The readiness model: the self-pipe, generalised

`source.event.v1` needs `wake D;` because a socket *pair* has no other statement
that makes an endpoint readable. **This route has three**, so `wake` is absent
rather than reproduced, and so is `endpoints` (descriptors come from `tcp`, `udp`
and `accept`). A module spelling either is refused as a statement this route does
not model (`this_route_models_no_wake_and_no_endpoint_pair`).

| Readiness | Induced by | Ready when |
|---|---|---|
| **accept** readiness | `connect C L;` | the listener holds a connection no `accept` has taken |
| **data** readiness (stream) | `send C […];` | the socket holds committed bytes no `recv` has taken |
| datagram readiness | `sendto U V […];` | the bound socket holds a datagram no `recvfrom` has taken |

All three are read off the **committed history the socket model already carried**.
A `write` registration is never ready here for exactly the reason it is never
ready on the event route — this route induces no send-buffer pressure — so a
module carrying one is **check-only**: checked in full, emits no C, publishes no
reference observation. That is `source.event.v1`'s own `accept_check_only` arm,
inherited rather than re-decided.

### 2.1 A turn drains nothing, and that is the composition's point

`std.event`'s callback drains one byte because that route has no other statement
that could. Here the program's own `accept`, `recv` and `recvfrom` do the work,
in the entry's own body, where the socket model already checks them. A turn
therefore **observes** readiness and changes no committed history — which is
exactly what level-triggered readiness means, and is why a second
`dispatch ready E;` on an undrained socket is still admitted
(`a_turn_drains_nothing_so_a_second_turn_on_the_same_readiness_is_admitted`),
and why the `dispatch idle;` after an `accept` in the published example is
admitted: the accept took the queued connection.

### 2.2 The determinism boundary is `Q085`'s existing condition

A turn is admitted only when the declared readiness is the whole of what this
program's own committed history induced. A two-member ready set is refused by
name with the **same ground** `Q061` carries — `poll(2)` reports a set, and
`explore_activation_orders` is documented "equal-priority events are deliberately
nondeterministic" — and the refusal is on the **state**, not the spelling.

That is word for word `Q085`'s condition: *a statement whose completion is not
determined by this program's own committed history*. It needed no new code, which
is §3's subject.

---

## 3. Diagnostic economy: **zero new codes**

The card asked to exhaust the extension route first. It was exhausted: every
refusal a registered socket can earn maps onto a condition one of `Q080`–`Q087`
already names. **Nothing was proposed for allocation.**

### 3.1 The inventory

| Refusal | Code | The existing condition it lands on |
|---|---|---|
| a malformed `event`/`watch`/`unwatch`/`dispatch` statement | `Q080` | the flat entry shape this profile models |
| no context, two contexts, a context left open, a registration or turn without one | `Q080` | the same |
| a registration already retired; a registration set beyond the emitted poll set's bound | `Q080` | the same |
| **crossed registration spans** | `Q080` | the same (§1.4) |
| a reactor statement in a function this route never lowers | `Q080` | "a construct outside the entry" |
| a name at a registration position that names no registration | `Q081` | "a position of this route naming nothing of the right kind" — widened from *socket* to *socket or registration* |
| a registration on a socket in no state this program can make ready | `Q082` | the transport state machine's own refusal |
| a close, a re-configuration or a name collision under a live loan | `Q083` | the inherited, unedited **ownership** checker |
| **a reactor-model refusal (`L00`–`L12`)**, e.g. a registration never retired | `Q083` | the same condition, second model — §3.2 |
| a turn the committed history does not decide; a two-member ready set | `Q085` | the determinism boundary |
| a registration name at a payload byte or a rendering operand | `Q081` | "not a check-time-constant integer this entry bound" — the registration namespace is disjoint from the value namespace by construction, so `Q086`'s crossing condition (a **socket** where a scalar belongs) is untouched and stays exactly what it was |
| an undeclared derived effect | `Q087` | unchanged |

### 3.2 The one judgement call, argued

`Q083`'s condition was *"a refusal reported by the **inherited, unedited**
ownership checker — this variant carries that checker's own error value rather
than a re-worded copy of it."* The composition reaches that same condition
through a **second** inherited model. Two readings were available:

* **allocate a new code for "the reactor refused"** — which would split one
  condition across the diagnostic space in order to record *which* checker
  happened to speak; or
* **widen `Q083`'s condition** from "the ownership checker" to "an inherited,
  unedited custody model", keeping the carry-the-model's-own-value discipline
  intact.

The second, on §L21's own rule that **a code is allocated to a condition rather
than to a field**, and on the `NATIVE-LIBRARY-HEADER-DECORATION` precedent the
card names: *the same condition reached by a new route*. A reader who sees `Q083`
learns exactly what they learned before — *someone else's checker said no, and
here is what it said* — and the message names which model. `SocketError::Reactor`
carries `EventDiagnostic` verbatim, exactly as `SocketError::Custody` carries
`OwnershipError`.

This is recorded as a **deliberate widening of a published code's condition**,
not as a silent reuse: spec §L21's `Q080`–`Q089` row states it, and this report
states it. It changes no existing program's diagnostic.

### 3.3 Reserved words: **zero added**

Constraint 14's census is trivially satisfied and was still run. Every word of
the composition — `event`, `open`, `close`, `watch`, `unwatch`, `dispatch`,
`ready`, `idle`, `read`, `write` — is **already** globally reserved by
`STD-EVENT` and the profiles before it (`crates/qcore-frontend/src/frontend.rs`
`is_reserved_source_word`). No `.qcore` file anywhere can newly collide, and the
whole workspace gate is the measurement. `wake` and `endpoints` stay reserved and
stay unmodelled here.

---

## 4. Route selection: the seam, and what it deliberately does not move

The composed guard sits immediately **above** the event route and reaches
`compile_socket_source`:

```rust
if has_source_event && has_source_socket
    && !has_source_buffer && !has_source_string && !has_source_http
{ return compile_socket_source(…); }
```

The conjunction is deliberately narrow: **exactly these two special vocabularies
and no third.** A module that also buffers, strings or does HTTP declines the
guard and reaches the route it reached before this card, keeping that route's
existing refusal byte for byte (`EVENT-BUFFER-PAYLOAD`, `HTTP-V1-EVENT-COMPOSE`).
Every concurrent route and the extern route are above all of it, so a module that
dispatches, sockets and spawns still keeps its `Q600`. Four tests pin the four
directions: only-dispatch, only-socket, the buffer triple and the HTTP triple.

**One pinned test was rewritten**, and it is the one this card exists to retire:
`compiler/tests/std_socket.rs::a_module_that_dispatches_and_sockets_stays_with_the_event_route`
is now `…_reaches_this_route_and_is_now_admitted`. That is the
`COMPOSE-EVENT-LINES` precedent, which rewrote
`a_module_that_both_dispatches_and_prints_stays_with_this_route…` for the same
reason. No other existing test changed, and none was weakened.

### 4.1 `socket_statement` is shared, and that is flagged

The socket route's published statement reader is also called by the HTTP route
for statements HTTP does not own. Adding the five reactor arms to it means the
HTTP route *could* lower a `dispatch` — except that a module carrying one has
`has_source_event` true, so the composed guard claims it before HTTP selection is
reached. Pinned by `a_module_that_does_http_and_dispatches_keeps_an_existing_routes_refusal`.

---

## 5. Emission: there is no run-time registry, and that is the honest shape

`event open;`, `watch`, `unwatch` and `event close;` emit **nothing**. The
watched set is a fact the checker knows at every turn, so:

> **the reactor an emitted program runs *is* the declared registration set,
> materialised as the `poll` set at each dispatch.**

There is no registry to create, add to, delete from or free — which is also why
none of those four statements derives an effect. `a_dispatch_turn_derives_io_and_the_registration_statements_derive_nothing`
and `the_registration_statements_emit_no_call` pin both halves.

A turn emits:

```c
    {
        qcore_socket_fd set[] = { sk_listener, sk_server };
        int outcome = qcore_socket_dispatch(set, 2u, 1);
        if (outcome == 2) { return 91; }
        if (outcome != 0) { return 90; }
    }
```

**The declaration is checked, never trusted**, exactly as a receive's payload is:
`qcore_socket_dispatch` polls the watched set and returns `2` when the reported
ready set is not the declared one, so a kernel that disagrees produces a *failed
run* rather than a differently shaped observation. Two new exit statuses, 90
(the wait failed or expired) and 91 (the declaration did not hold), disjoint from
the socket route's 80–89 and the event route's 70–76.

An idle turn waits **zero** milliseconds rather than the bounded 5000: it claims
nothing is ready *now*, and under the checked history nothing this program
committed can make a watched socket ready while the program is between
statements, so waiting longer would test nothing.

### 5.1 Platform: measured, and no new per-arm primitive

The card asked this to be measured rather than assumed. The measurement:
`qcore_socket_dispatch` is the existing `qcore_socket_wait_readable` with
`nfds > 1`. It introduces **no platform** header, type, constant or call the shim
did not already use on every arm — `poll(2)` on the two POSIX arms and `WSAPoll`
on Windows, both already called with a one-element array, both already behind the
existing `qcore_socket_pollfd` typedef, both already under
`QCORE_SOCKET_TIMEOUT_MS`. Two details are this card's own and neither is
per-arm: a zero-length set is answered without calling either (WSAPoll refuses
`nfds == 0`), and the array is sized for a new **shim** constant
`QCORE_SOCKET_MAX_WATCHED` = 16 — a bound the checker refuses beyond, so a defect
in the checker overflows nothing, exactly as `QCORE_SOCKET_MAX_PAYLOAD` bounds a
payload.

So **no new platform capability is claimed**, and the Windows and Apple gap this
card records is the composed *programs* not having been run there rather than a
new primitive not having been exercised. Residuals `SOCKET-EVENT-WINDOWS-RUN`
and `SOCKET-EVENT-APPLE-RUN`, scoped exactly that way.

| Arm | This card's claim |
|---|---|
| `_WIN32` | **compile-path-correct only.** Residual `SOCKET-EVENT-WINDOWS-RUN`. No new primitive: `WSAPoll` with `nfds > 1` |
| `__APPLE__` | **compile-path-correct only.** Residual `SOCKET-EVENT-APPLE-RUN`. No new primitive: `poll(2)` with `nfds > 1` |
| else (Linux, other POSIX) | **runtime-verified**: emitted C built `-std=c11 -Wall -Wextra -Werror -pedantic`, run under ASan/UBSan/LSan, stdout byte-compared with the reference observation, twice |

The native tests are confined by `#[cfg(any(target_os = "linux", target_os = "macos"))]`
exactly as `std_socket.rs`'s are; the macOS ASan branch is present and correct
(LSan is Linux-only) and was not exercised on this card's hardware.

---

## 6. Determinism of the observation (standing constraint 15c)

Constraint 15c asks a route to say **why** every record of its observation is
written by the entry's thread. The composition adds no emitter, and the argument
is the conjunction of the two routes' own:

> A composed program has **one thread of control and creates none.** The socket
> half spawns no worker. The reactor half's one accepted profile
> (`SingleThreadedPollV1`) declares `threading_enabled = false`, under which the
> reactor refuses a cross-thread activation at `L04` — and **the callback body is
> empty**, so a turn runs on the entry's own thread and writes nothing at all.
> The only statements that write bytes to a stream are `source.std-io.v1`'s own
> `print` and `log`, executed in the entry's body in program order.

Therefore every record's emitter is `Emitter::Parent`, no worker is ever live,
the `OBS-REORDERING` live-set derivation puts each record alone in its own
window — a barrier — and the declared `qcore.obs-reordering.v1` relation
`is_strict()`. The byte comparison the native tests perform is that relation's
own degenerate case (`strict_iff_byte_equality`).
`the_declared_reordering_relation_is_strict` asserts it on both streams and
additionally checks that comparing against the relation **is** byte equality.
**No reordering is declared, because this composition needs none.**

**A dispatch turn writes no record at all**, and neither does a registration. A
turn reaches no stream the observation is read from — the boundary
`COMPOSE-EVENT-LINES` drew for a drained wake byte and `QUIRIN-SOCKET-V1` for a
payload (`a_dispatch_turn_writes_no_record`). Nothing that is not
model-determined reaches an observation: not a descriptor number, not a port, not
a segment boundary, not a timing, and not a `poll` result.

---

## 7. The executed example, and its comparison result

`examples/events/socket_server.qcore`, indexed on `examples/events/README.md`.
Its home is the events page rather than `examples/sockets/` because that is the
page a reader comes to for *"how do I wait on readiness"*, and because the user's
standing pipeline rule binds that family — every source line in it is a `|>`
pipeline (`the_published_example_is_spelled_with_pipelines`).

Shape: one entry; a listening socket and a client of the same program; **both**
readiness kinds; **two descriptors watched at once**, so a turn has to say which
one it reports; an `accept` and a `recv` in the entry's own body; two rendered
lines; LIFO deregistration before three closes.

```
event open → listener(bind, listen) → client(connect) → watch inbound listener
  → dispatch ready inbound → accept → watch traffic server
  → dispatch idle → send → dispatch ready traffic
  → recv, recv → two lines → unwatch traffic → unwatch inbound → event close
  → three closes
```

It writes `81\n0x00000051\n`.

**Byte-comparison result: identical.**
`native::the_server_example_runs_sanitized_and_writes_the_reference_observation`
compares the sanitized native run's stdout with the reference observation's
stdout projection, byte for byte, and passes.
`native::two_runs_of_the_server_example_write_identical_bytes` passes, so no
descriptor number, port or timing reaches the observation. The page's transcript
is read out of the README and compared with what the pipeline produces
(`the_readme_transcript_is_the_real_one`), so it cannot drift.

---

## 8. Exact test names

All in `compiler/tests/socket_event_compose.rs` unless stated. **44 tests, all
passing**, plus one rewritten pin in `compiler/tests/std_socket.rs`.

**Route selection (the seam)** — `a_composed_module_reaches_the_socket_route`,
`a_module_that_only_dispatches_stays_with_the_event_route`,
`a_module_that_only_sockets_stays_on_the_bare_socket_route`,
`a_module_that_buffers_and_dispatches_and_sockets_keeps_the_event_routes_refusal`,
`a_module_that_does_http_and_dispatches_keeps_an_existing_routes_refusal`,
`a_composed_module_needs_both_imports`; and in `compiler/tests/std_socket.rs`,
`a_module_that_dispatches_and_sockets_reaches_this_route_and_is_now_admitted`
(rewritten).

**The custody relation (the loan)** —
`a_close_while_registered_refuses_and_the_refusal_is_the_unedited_ownership_checkers`,
`reconfiguring_a_registered_socket_refuses_because_it_is_an_ownership_write`,
`transport_on_a_registered_socket_is_admitted_because_a_shared_loan_admits_reads`,
`a_registration_acquires_no_descriptor`,
`a_registration_never_retired_refuses_at_the_reactor_models_terminal_sweep`,
`a_registration_name_that_collides_with_a_socket_refuses`,
`crossed_registration_spans_refuse_by_name`,
`properly_nested_registration_spans_are_admitted`.

**The projection** — `the_projection_gives_the_reactor_no_descriptor_of_its_own`,
`a_program_that_registers_nothing_publishes_no_reactor_certificate`.

**The determinism boundary (`Q085`)** —
`a_turn_the_history_does_not_satisfy_refuses`,
`an_idle_turn_with_something_ready_refuses`,
`two_ready_registrations_admit_no_turn`,
`a_turn_that_names_the_wrong_registration_refuses`,
`a_turn_drains_nothing_so_a_second_turn_on_the_same_readiness_is_admitted`.

**Transport state and operands (`Q082`/`Q081`)** —
`registering_a_socket_nothing_can_make_ready_refuses`,
`a_name_at_a_registration_position_that_is_not_one_refuses`,
`a_registration_name_at_a_value_position_refuses_at_the_operand_code`.

**Shape (`Q080`)** — `a_registration_without_a_context_refuses`,
`a_context_left_open_refuses`, `two_contexts_refuse`,
`this_route_models_no_wake_and_no_endpoint_pair`,
`a_reactor_statement_in_a_helper_is_refused`.

**The check-only arm** —
`a_write_registration_is_checked_in_full_and_emits_nothing`.

**Effects, certificate, recheck, emission** —
`a_dispatch_turn_derives_io_and_the_registration_statements_derive_nothing`,
`a_module_that_declares_too_little_is_refused`,
`the_published_certificate_is_independently_rederivable`,
`the_certificate_publishes_the_watched_set_of_every_turn`,
`the_emitted_program_checks_every_declaration_rather_than_trusting_it`,
`the_registration_statements_emit_no_call`,
`the_emitted_program_routes_every_line_through_the_single_io_route`.

**Determinism of the observation (15c)** —
`the_declared_reordering_relation_is_strict`, `a_dispatch_turn_writes_no_record`.

**The published example** — `the_published_example_takes_this_route`,
`the_published_example_is_spelled_with_pipelines`,
`the_readme_transcript_is_the_real_one`.

**Native, Linux/macOS only** —
`native::the_server_example_runs_sanitized_and_writes_the_reference_observation`,
`native::two_runs_of_the_server_example_write_identical_bytes`.

---

## 9. Lean

New module `proof/QCore/SocketEventCompose.lean`, registered in `proof/QCore.lean`
and audited by `proof/AuditSocketEventCompose.lean` (**32** entries;
placeholder-free; every closure is `propext` / `Classical.choice` / `Quot.sound`
only).

| Group | Theorems |
|---|---|
| **Registration custody** (the card's first close requirement) | `a_registration_acquires_no_descriptor`, `retiring_a_registration_releases_no_descriptor`, `the_context_statements_move_no_custody`, `a_watched_socket_is_still_held` |
| **Dispatch neutrality** (the second) | `a_dispatch_turn_changes_no_custody`, `a_dispatch_turn_changes_no_loan`, `a_turn_drains_nothing` |
| **The three loan rules** | `watching_borrows_the_socket`, `a_close_while_registered_is_refused`, `reconfiguring_a_registered_socket_is_refused`, `a_transfer_on_a_registered_socket_is_admitted` |
| **Deregistration ordering** (the third) | `retiring_removes_exactly_its_own`, `retiring_keeps_every_other_loan`, `retiring_the_only_loan_admits_the_close` |
| Lexical spans | `properly_nested_spans_are_admitted`, `crossed_spans_are_refused`, `a_registration_never_retired_is_refused` |
| **Erasure** | `erasing_the_reactor_statements_leaves_the_ledger`, `erasing_a_composed_program_leaves_its_transport` |
| Determinism | `an_idle_turn_needs_an_empty_ready_set`, `a_declared_turn_names_the_one_ready_registration`, `two_ready_registrations_admit_no_turn`, `a_write_registration_is_never_ready`, `readiness_comes_only_from_the_programs_own_history` |
| Effects | `a_dispatch_turn_derives_io`, `the_registration_statements_derive_nothing`, `no_statement_derives_mut` |
| Module gate | `a_registration_needs_both_modules`, `the_socket_module_alone_grants_no_registration`, `the_event_module_alone_grants_no_registration`, `code_that_registers_nothing_needs_neither`, `the_transport_half_still_needs_its_module` |

**The erasure group is the one that makes the composition a composition.**
`erase` drops every statement this card adds, and the ledger's answer is
unchanged under it — so every custody theorem `QCore.StdSocket` proves about a
program is a theorem about the same program with registrations in it. That is
the shape `QCore.ComposeEventLines` used for a source line, reused for the same
reason.

**One deliberate non-widening.** `QCore.ModuleImports.Imports` was **not** grown
to an eleventh field. `std.event` was never in the Lean registry — the `STD-EVENT`
card did not add it — so adding it now would be that card's gap closed inside
this one's blast radius, at the cost of editing ten-positional structure literals
across two other cards' theorems. The module gate is instead stated here as the
conjunction it is, and the transport half is the registry's own theorem restated
rather than re-proved (`the_transport_half_still_needs_its_module`). Named
follow-on `EVENT-MODULE-REGISTRY-LEAN`.

**What is not proved.** That the Rust lowering extracts these statements
faithfully from source tokens; the reactor model's own lifecycle rules
(`QCore.DescriptorCustody` owns them, and the route hands them the projection
unedited); the transport half (`QCore.StdSocket`'s subject); or anything about a
failed primitive — the emitted program exits there. Partial correctness only.

---

## 10. Files

| Path | What |
|---|---|
| `crates/qcore-core/src/socket_event.rs` | **new**: the reactor projection, the custody argument in its header, `SocketRegistration`, `SocketDispatchTurn`, `MAX_WATCHED` |
| `crates/qcore-core/src/std_socket.rs` | the five statements, the walk, the loan in `custody_operations`, the readiness function, the five certificate fields, `SocketError::{Reactor, NotARegistration}` |
| `crates/qcore-core/src/std_socket_c_emit.rs` | the turn's emission and the two new statuses |
| `crates/qcore-core/src/qcore_socket_compat.h` | `qcore_socket_dispatch`, `QCORE_SOCKET_MAX_WATCHED` |
| `crates/qcore-core/src/std_event.rs` | `reactor_operation` published (one line), so the diagnostic→statement mapping is read rather than copied |
| `crates/qcore-frontend/src/source_socket.rs` | the five statement forms, `reactor_span`, the grammar and census headers |
| `crates/qcore-verification/src/driver.rs` | the seam of §4 and the check-only arm — **flagged**, §10.1 |
| `compiler/tests/socket_event_compose.rs` | the 44 tests of §8 |
| `compiler/tests/std_socket.rs` | the one rewritten pin |
| `examples/events/socket_server.qcore`, `examples/events/README.md` | the published example and its verified transcript |
| `proof/QCore/SocketEventCompose.lean`, `proof/AuditSocketEventCompose.lean`, `proof/QCore.lean` | §9 |
| `docs/QCORE_SPEC_r0_3.md` §L21, `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` §G07.11 | the amendment and the grammar |

### 10.1 The `driver.rs` seam, flagged as requested

Two edits, both additive and both confined:

1. **One route-selection `if` block**, inserted immediately above the event
   route's, calling the existing `compile_socket_source`. No new function, no new
   enum variant, no new `CheckedSourceModule` arm, no new
   `SourceExecutionProfile` — a composed module *is* a `StandardSocketV1` module.
2. **The check-only arm inside `compile_socket_source`**: the existing
   `interpret`/`emit` pair moved inside an `if certificate.executable`, modelled
   line for line on `compile_event_source`'s own. Every program without a `write`
   registration takes the `true` branch and is unchanged.

A merge conflict with a sibling card is expected only where both append to the
route-selection chain.

---

## 11. Corpus neutrality

By construction. The whole diff against the base (`a53fe2a4`) is **19 files**:
`crates/qcore-core` (6 — `socket_event.rs` new, `std_socket.rs`,
`std_socket_c_emit.rs`, `qcore_socket_compat.h`, one line of `std_event.rs`, one
line of `lib.rs`), `crates/qcore-frontend/src/source_socket.rs`,
`crates/qcore-verification/src/driver.rs`, two specification documents, three
Lean files, two test files, three example files (one new program, the events
page, one row of the index) and this report. `crates/qcore-c-gateway/`,
`crates/qcore-audit/`, `contract/`, `providers/`, `examples/threads`,
`examples/http` and every frozen archive appear **nowhere** in it. The pinned
185-unit artifacts, the leak count of **266** (constraint 7), the 73-module
contract pin and the CT manifest are main's, untouched and unread. Every existing
published example emits byte-identical C: the composition's statements are the
only ones whose absence changes nothing, and the borrow-free peak projection is
the identity on a program with no registration.

---

## 12. Named follow-ons

| Name | What is missing |
|---|---|
| `SOCKET-EVENT-WINDOWS-RUN`, `SOCKET-EVENT-APPLE-RUN` | a native run, sanitizer or byte-comparison claim for a **composed** program on those arms. No new per-arm primitive is emitted (§5.1), so this is about the programs and not about the shim |
| `SOCKET-EVENT-CROSSED-SPANS` | crossed registration spans. Needs a loan representation that is not the ownership IR's lexical borrow node (§1.4) |
| `SOCKET-EVENT-CONFIGURE-UNDER-LOAN` | re-configuring a registered socket. Needs a reason to believe the declared interest survives the change (§1.3) |
| `SOCKET-EVENT-WRITE-READY` | an **executable** write registration. Needs a way to induce send-buffer pressure deterministically — the same gap `source.event.v1` records |
| `SOCKET-EVENT-L03-REACH` | the reactor's `L03`-on-a-closed-descriptor arm is unreachable from this composition, because this route's closes are the ownership ledger's (§1.1). Kept as a fail-closed guard on a public model API, on the `EXTERN-F1-STORE-REACH` precedent |
| `EVENT-CALLBACK-BODY` | *(the event card's, unchanged)* a source-level callback body. It is what would put a record *inside* a turn, and §6's argument is shaped for it |
| `HTTP-V1-EVENT-COMPOSE` | **not landed, and still a refusal.** An HTTP module that dispatches declines the composed guard and keeps an existing route's refusal (§4, pinned). What it needs beyond this card: the HTTP route lowers to a socket program *before* the socket checker runs, so a `watch` on a socket the HTTP lowering synthesised has no source name to register — the composition has to decide whether a message exchange is one registrable thing or three. That is a design question this card did not answer |
| `SOCKET-BACKLOG-DEPTH` | *(the socket card's, unchanged)* an accept queue deeper than one. It is what a registered listener reporting several pending connections would need |
| `EVENT-MODULE-REGISTRY-LEAN` | `std.event` is not a row of `QCore.ModuleImports`. Adding it is the `STD-EVENT` card's gap, and closing it edits ten-positional literals across two other cards' theorems (§9) |
| `SOCKET-CUSTODY-ATTRIBUTION` | *(inherited from the socket card)* every `Q083` on this route is attributed to the entry rather than to the statement that earned it, because `SocketError::Custody` carries the statement count. Not a code change (constraint 1) |
| `SOCKET-EVENT-COMPOSE` | **retired by this card** (the socket card's follow-on) |

---

## 13. Standing gate

Run at the card tip, Linux, with `$HOME/.elan/bin` on `PATH`. Sibling agents were
building on the same host, so wall-clock timings are not comparable with a quiet
run.

| Step | Result |
|---|---|
| `git submodule update --init --recursive` | run in this fresh worktree — §13.1 |
| `cargo test --workspace --no-fail-fast` | **3,994 passed, 0 failed, 8 ignored** across 206 result lines, exit **0** |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit **0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit **0** |
| `python3 scripts/check_lean_audits.py` | **"Passed 104 Lean audits"** (103 → 104: `AuditSocketEventCompose.lean`), read from the **text** line per the script's own behaviour, no project axiom |
| `python3 -m unittest discover -s scripts/tests` | 193 tests, **1 expected failure** — §13.3 |

### 13.1 The submodule

`libevent_submodule_matches_the_audited_revision` is invariant 5's pin, and this
worktree started without the checkout. The gate's own first line is the repair;
the submodule was checked out at `d82464a277d0f42703702c4dfd9af6af38595a83`
before the whole-workspace run above.

### 13.2 The count, and what it is made of

**3,994 against the base's 3,950** (card `HTTP-QUIRIN-V1` §14's measured number),
and the difference is exactly this card's **44** new tests in
`compiler/tests/socket_event_compose.rs`: 3,950 + 44 = 3,994. The rewritten pin
in `compiler/tests/std_socket.rs` is one test before and after. **No existing
test was deleted or weakened**, which matters here because this card changed one
enum, one struct and one error type that the socket route's own tests read.

**No `P02-FLAKE` row tripped** (standing constraint 16): the complete run had
zero failing binaries, so nothing needed isolated re-verification. Three of the
five rows are in this workspace suite and each passed on the first attempt —
`vp04_vp07_vp08_process_lifecycle_and_payload_budgets`,
`a_confined_fixture_cannot_write_outside_the_admitted_paths` and
`memory_growth_beyond_the_declared_limit_is_bounded`. The other two live in the
separate `providers/` workspace, which this card neither touches nor ran: no
provider, schema, package template or protocol changed, so constraint 18's
rebuild is not owed and the 122-test provider checkpoint is **carried, not
rerun**. The host was carrying another agent's full gate throughout, with load
average between 39 and 51, so wall-clock timings are not comparable with a quiet
run.

### 13.3 One expected Python failure, left for the integrator

`test_the_derived_facts_are_recomputed_rather_than_carried` reports
`['14.0.0', 103] != ['14.0.0', 104]`. That is the derived-value tripwire working
as designed: this card adds one audited Lean module, and
`docs/reviews/r0_3/acceptance_matrix.json` — which this card may not edit —
carries the pre-card count. §14 item 3. The other 192 Python tests pass.

Measurement outputs are worktree-private under `measure/` (gitignored), never
under `target/` and never in the shared scratchpad.

---

## 14. Integrator reconciliation list

This card may not edit `AGENTS.md`, `DEVELOPMENT.md` or
`docs/reviews/r0_3/acceptance_matrix.json`. Four things are the integrator's.

1. **No new diagnostic code.** `AGENTS.md`'s diagnostics-namespace table needs
   only the `Q080`–`Q089` row's *description* widened, not a new row. The
   authority is spec §L21, which this card amended. Suggested addition to the
   existing row's text:

   > …and, since card `SOCKET-EVENT-COMPOSE`, the reactor composition: a checked
   > socket registered with `source.event.v1`'s reactor. That composition
   > allocates **no code** — a registration's refusals land on `Q080` (shape,
   > including crossed spans), `Q081` (a name at a registration position),
   > `Q082` (a socket in no state this program can make ready), `Q083` (whose
   > condition is widened from "the inherited ownership checker" to "an
   > inherited, unedited custody model", so a reactor `L00`–`L12` diagnostic
   > lands there carrying its own value), `Q085` (a turn the committed history
   > does not decide, including a two-member ready set) and `Q087`.

2. **A proposed standing constraint**, offered because it is the kind of rule the
   "when touching X, do not Y" inventory exists for:

   > **15g. A reactor registration is a loan, and the reactor owns no descriptor
   > of `source.socket.v1`.** A composed entry's projection carries **no**
   > `CreateSocketPair` and **no** `CloseSocket` — those are the operations by
   > which `event_reactor` owns a number — and `watch E S …;` is an
   > `OwnershipOp::BorrowShared` of `S` to `E` running to the matching
   > `unwatch E;`. Do not give the reactor a descriptor of this route: the three
   > rules a reader wants (a close while registered refuses, a
   > `bind`/`listen`/`connect` on a watched socket refuses, a `send`/`recv`/`accept`
   > on one is admitted) are the **unedited** ownership checker's drop, write and
   > read under a shared loan, and "deregister before close" is their consequence
   > rather than a rule. The loan's span is **lexical**, because the ownership
   > IR's borrow nodes are; a crossed pair is refused by name and must not be
   > approximated. A dispatch turn **drains nothing** — it observes
   > level-triggered readiness and the entry's own transport statements do the
   > work — and the four bookkeeping statements emit no call and derive no
   > effect, because the reactor an emitted program runs *is* the declared
   > registration set materialised as the `poll` set at each turn. `Q083`'s
   > condition is "an inherited, unedited custody model refused, and the variant
   > carries that model's own error value": do not split it per model.
   > `socket-event-compose`.

3. **The derived `lean_audits` count moves 103 → 104** for this card's one new
   `Audit*.lean`, so `scripts/tests`'s derived-value tripwire reports the stale
   matrix count until the derived section is regenerated (constraint 25).

4. **Public API changes in `qcore-core`**, all additive except one:
   * new module `socket_event` (`SocketRegistration`, `SocketDispatchTurn`,
     `RegistrationId`, `MAX_WATCHED`, the projection constructors);
   * `std_socket::SocketStatement` gains five variants — `ReactorOpen`,
     `ReactorClose`, `Watch`, `Unwatch`, `Dispatch`;
   * `std_socket::SocketCertificate` gains `descriptors`, `registrations`,
     `turns`, `reactor`, `executable`; `SocketError` gains `Reactor` and
     `NotARegistration`; `SocketRecheckFailure` gains five variants;
   * `std_socket_c_emit::{STATUS_DISPATCH, STATUS_READY}` are new;
   * `std_event::reactor_operation` is now `pub` (was private);
   * new in `qcore-frontend`: `source_socket::{EVENT_KEYWORD, WATCH_KEYWORD,
     UNWATCH_KEYWORD, DISPATCH_KEYWORD}`.
   * **Behavioural, and deliberate:** `compile_socket_source` may now return
     `c_source: None` and `extern_observation: None` for a check-only module.
     Only a `write` registration reaches that branch.

## 15. Draft increment paragraph

> A checked socket as a reactor-registered event source (SOCKET-EVENT-COMPOSE):
> a module importing `std.socket` and `std.event` may now open a reactor
> context, **register a socket it owns** with it, receive dispatch for it, and
> take its `accept`/`recv`/`recvfrom` in its own body — the socket card's own
> named follow-on, retired. It is not a new route: a module using both
> vocabularies is a `source.socket.v1` module, and one that also buffers,
> strings or does HTTP declines the composed guard and keeps the refusal it
> already had. **The custody relation is a loan, and both existing models were
> already shaped to give that answer.** The projection carries **no**
> `CreateSocketPair` and **no** `CloseSocket` — the two operations by which the
> reactor model owns a descriptor — and `watch E S read;` is an
> `OwnershipOp::BorrowShared` of `S` to `E`, so a close while registered is the
> unedited ownership checker's `DropWhileBorrowed`, a `bind`/`listen`/`connect`
> on a watched socket is its `OwnerMutationWhileBorrowed`, and a
> `send`/`recv`/`accept` is admitted because a shared loan admits reads.
> "Deregister before close" is therefore a **consequence** rather than a rule
> this card wrote, enforced twice over by the ownership checker's drop rule and
> the reactor's own `L07` terminal sweep. **Zero diagnostic codes were
> allocated and zero reserved words added**: every refusal lands on a condition
> `Q080`–`Q087` already names, with one deliberate widening — `Q083`'s condition
> moves from "the inherited ownership checker refused" to "an inherited,
> unedited **custody model** refused, carrying that model's own error value", so
> a reactor `L00`–`L12` diagnostic lands there (the
> `NATIVE-LIBRARY-HEADER-DECORATION` precedent). There is no `wake` and no
> `endpoints`, because this route's readiness is induced by its own `connect`,
> `send` and `sendto` — the self-pipe generalised to **accept** readiness and
> **data** readiness — and a turn **drains nothing**, which is what
> level-triggered readiness means and why a second turn on an undrained socket
> is still admitted. The emitted program has **no run-time registry**: the
> reactor it runs *is* the declared registration set, materialised as the `poll`
> set at each turn, which is why the four bookkeeping statements emit no call
> and derive no effect; a turn's declaration is **checked** against what `poll`
> reported and a disagreement is a distinct non-zero exit. The observation stays
> byte-for-byte on constraint 15c's own ground — the reactor profile is
> single-threaded and the callback body is empty, so every record is still the
> entry thread's — and no reordering is declared. One published example,
> `examples/events/socket_server.qcore`, watching two descriptors at once,
> sanitized-native on Linux with its stdout byte-compared and every line spelled
> as a `|>` pipeline; Windows and Apple stay compile-path-correct only
> (`SOCKET-EVENT-WINDOWS-RUN`, `SOCKET-EVENT-APPLE-RUN`), and the composition
> emits **no new per-arm primitive** — `poll`/`WSAPoll` with `nfds > 1`.
> Registration spans are **lexical**, because the ownership IR's borrow nodes
> are; a crossed pair is refused by name (`SOCKET-EVENT-CROSSED-SPANS`). **32**
> audited Lean entries in a new `QCore.SocketEventCompose`, including the
> erasure theorem that makes every custody theorem about a socket program a
> theorem about the same program with registrations in it. Corpus-neutral by
> construction. Evidence: `docs/reports/socket-event-compose/README.md`.
