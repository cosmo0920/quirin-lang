# QUIRIN-SOCKET-V1 — a checked TCP and UDP socket surface in pure QCore source

**Card:** `QUIRIN-SOCKET-V1` (wave 24). **Route:** `source.socket.v1`, behind the
`std.socket` bundled module. **Band:** `Q080`–`Q089`, assigned by the integrator
before the card started; `Q075`–`Q079` belong to a sibling card of the same wave
and are untouched here.

**What shipped.** A checked, executable source-level socket surface for **both**
transports:

* **TCP** — `let S = tcp;`, `bind S;`, `listen S;`, `connect C L;`,
  `let S = accept L;`, `send C [BYTE, …];`, `recv S COUNT;`,
  `let B = recv S;`, `close S;`.
* **UDP** — `let U = udp;`, `bind U;`, `sendto U V [BYTE, …];`,
  `recvfrom V COUNT;`, `let B = recvfrom V;`, `close U;`.

Both lower to self-contained C11 through a new embedded three-arm shim, both run
sanitized on Linux with byte-compared output, and both have a published example
under `examples/sockets/`. Nothing here links a library, declares an extern
function or reads a manifest.

**Status language.** This card publishes no verdict about any pinned corpus unit
and moves no corpus number. It is corpus-neutral: no gateway, contract or
provider file is touched.

---

## 1. The problem this card had to solve, and how

A socket surface is the point at which external nondeterminism enters a
language. A `connect` waits on a peer; a `recv` waits on bytes that may never
come; a stream delivers a byte *sequence* whose segment boundaries nothing in
the program decides; a `close` can fail for reasons the documentation does not
resolve. The repository's standing rules (invariant 1, invariant 2, constraints
15a/15c) make "run it and see" unavailable as an answer: an executable route here
has to publish a reference observation that a native run reproduces *byte for
byte*, and anything the model cannot predict cannot be in it.

The design decision, from which everything else follows:

> **Every peer of every socket in an admitted program is another socket of the
> same program**, on the loopback interface, in one thread.

That is not a testing convenience grafted onto a general surface. It is what
makes the route's two determinism obligations *decidable*, and the card's
argument is that the honest v1 is the one whose guarantees are provable rather
than the one whose syntax looks most complete.

### 1.1 A receive never blocks on bytes that were never sent

The model carries, for every connected TCP socket, the exact byte stream an
earlier `send` of the same program committed to it, and for every bound UDP
socket the exact queue of datagrams an earlier `sendto` committed. A
`recv`/`recvfrom` the committed history does not already satisfy is **refused at
check time** (`Q085`) rather than deadlocking at run time. So is an `accept` with
nothing queued, and so is a second unaccepted connection on a depth-one accept
queue.

This is stronger than a timeout: the emitted program's bounded wait is a
**witness** of a decision the checker already made, in exactly the sense
`source.buffer.v1`'s emitted bounds comparison is a witness. If one ever fired,
that would be a defect in this checker.

Proved as `QCore.StdSocket.an_admitted_receive_takes_its_full_count` and
`a_receive_on_an_empty_stream_is_refused`.

### 1.2 What a receive yields is model-determined, and the run checks it

Because the model holds the committed bytes, it can say exactly which bytes each
receive takes. `SocketCertificate::receipts` publishes them, and the emitted
program **compares what it actually read against them**, exiting with status 88
on disagreement. The named premise `SOCKET-V1-STREAM-BYTE-ORDER` therefore
produces a *failed run* if it is false — never a differently shaped observation
and never a wrong verdict. This is the discipline `STD-EVENT` established for a
drained wake byte, applied to a payload.

Proved as `QCore.StdSocket.draining_returns_the_committed_bytes` and
`two_commits_are_returned_in_order`.

### 1.3 What is refused by name rather than absorbed

| Refusal | Why, and its code |
|---|---|
| An external peer | there is no spelled address anywhere; `connect C L` names a listening socket of this program. Follow-on `SOCKET-EXTERNAL-PEER` — a determinism change, not a syntax change |
| A receive the history does not satisfy | `Q085`, §1.1 |
| An accept with nothing queued | `Q085` |
| A second unaccepted connection | `Q085`; whether a backlogged connect completes before its accept is a kernel decision this model does not make (`SOCKET-BACKLOG-DEPTH`) |
| A send beyond the in-flight bound | `Q085`; beyond it a send may wait for a peer only this same thread could drain |
| A **stream** close with unread bytes | `Q085`; it resets the connection, and the peer observes that at a moment this program does not decide |
| A listener close with a queued connection | `Q085`, same ground |
| An interrupted call | the shim does **not** resume on a `-1` return; the run fails with a distinct status, because the retry count is not model-determined (`SOCKET-V1-EINTR-RESUME`) |
| A failed close | not retried — see §5 |

And one thing deliberately **admitted**, on stated grounds rather than for
uniformity: a **datagram** close with queued datagrams. The queue is discarded
locally, no packet is emitted, and nothing observable changes. The two closes are
decided differently because they are different
(`a_stream_close_with_undrained_bytes_refuses_and_a_datagram_close_does_not`).

### 1.4 Segment boundaries: resolved by declaration, not absorbed

TCP delivers a byte sequence; the placement of segment boundaries is a transport
decision this model does not predict. No statement, certificate field or
observation record mentions one. A receive names the **count** it takes, the shim
reassembles short reads to exactly that count, and the boundary is therefore
resolved by the declaration rather than silently absorbed.

A datagram is the opposite and is modelled as such: `recvfrom` takes exactly one
datagram and refuses at `Q084` when its declared count is not that datagram's
length. There is no truncation and no reassembly, because neither is what a
datagram is (`a_partial_datagram_is_refused`).

---

## 2. Decisions, argued

### 2.1 Naming: `std.socket`, not `std.net` — with the census

Constraint 14 requires a census before any new word. It ran over **412**
`.qcore`/`.qc` fixtures in the tree and **41,529** string literals in every Rust
source file, at whole-word granularity, with `//` comments stripped from the
QCore side.

| Candidate | Occurrences in QCore source position | Verdict |
|---|---|---|
| `tcp`, `udp`, `listen`, `connect`, `accept`, `recv`, `sendto`, `recvfrom` | **0** anywhere | reserved |
| `send` | 0 (3 occurrences, all inside `//` comments of the contract corpus) | reserved |
| `bind` | 0 (2 occurrences, all inside `//` comments) | reserved |
| `socket` | 0 in QCore source (5 in Rust diagnostic/profile strings) | **rejected as a keyword** — see below |
| `stream` | **15**, including `extern fn` **parameter** positions in the contract corpus | **rejected** |
| `endpoint`, `peer`, `datagram`, `net` | 0 / comments only | not needed |

`stream` is the finding worth recording: it is shielded from the reference
collector only because a parameter name is followed by `:`. That is too thin a
margin to build a keyword on, so it was not used — the same judgement `STD-BUFFER`
made about `buffer`, reached by the same measurement rather than by analogy.

**Both** `std.socket` and `std.net` census clean, so the module name was chosen
by argument, not forced:

* A module name in this language **claims exactly what it grants**. `std.buffer`
  is not `std.memory`; `std.string` is not `std.text`.
* What this module grants is the BSD socket vocabulary over loopback. `std.net`
  would name a capability space — addressing, name resolution, interfaces, TLS —
  of which v1 implements *none*.
* Choosing `std.net` now would force a later card supplying any of those to
  either overload the name or take a second one. `std.socket` leaves `std.net`
  available for exactly that, and `a_near_miss_socket_spelling_rejects` pins that
  `std.net` is **not** a registry entry today.

**Ten reserved words** were added: `tcp`, `udp`, `bind`, `listen`, `connect`,
`accept`, `send`, `recv`, `sendto`, `recvfrom`. Each is a clause word in a
statement or a `let` initialiser and never a reference, and each is measured
load-bearing: without reservation, name resolution collects it and the statement
refuses at `Q013`. `let`, `close` and `return` are **reused rather than
respelled**, which is the EXTERN-MODES discipline applied where it applies — the
release is `close NAME;`, a word `std.event` and the checked mutex-guard profile
already use at a statement head.

`close` rather than `drop`, deliberately, even though the operation *is*
`OwnershipOp::Drop`: `drop` is the language's word for relinquishing a value,
while a descriptor close is a system call whose failure the OS-endpoint relation
has documented answers for (§5). Naming them the same would hide that. This is
the one place this card departs from the `STD-BUFFER` precedent, and it departs
for a reason `STD-BUFFER` did not have.

### 2.2 Payload: declared byte lists, and why not a buffer or a string

`send NAME [BYTE, …];` carries a bracketed list of **operands**, each an integer
literal or a name this entry bound to a check-time-constant integer — the same
two forms `source.buffer.v1` and `source.string.v1` admit, for the same reason.
Neither form can denote a socket, so a payload byte can never be one, and every
byte this program puts on the wire is decided before emission. That is precisely
what lets the checker predict each receive and the run verify it (§1.2).

The alternatives were considered and both declined for v1:

* **A `std.buffer` payload** would be the natural next step, but a buffer is an
  owned extent-carrying resource and admitting it here would put two custody
  models in one entry. Route selection places this route **after** the buffer
  route, so a module that does both keeps the *buffer route's existing refusal* —
  measured, not asserted
  (`a_module_that_buffers_and_sockets_stays_with_the_buffer_route`). Named
  follow-on **`SOCKET-BUFFER-PAYLOAD`**.
* **A string payload** is what an HTTP card will want, and the string card runs
  in parallel. Nothing of this surface depends on it and nothing of it was
  imported. Named follow-on **`SOCKET-STRING-PAYLOAD`**, for the HTTP card to
  consume. This card creates **no dependency** on the parallel card: it touches
  no `source.string.v1` file and takes no code from one.

The `let BINDING = recv NAME;` value form binds the **one byte** the receive took
— directly the `let NAME = dispatch ready E;` precedent from
`COMPOSE-EVENT-LINES`. It is sound because the comparison immediately above it in
the emitted C has already refused to continue unless the byte read is that one.

### 2.3 Blocking: argued, not assumed

The card asked for this to be decided and argued rather than assumed. The
argument has three parts, and only the third is a premise:

1. **A receive cannot block indefinitely**, because §1.1 proves the bytes it
   takes are already committed. This is a theorem about the model, not about the
   platform.
2. **A connect and an accept cannot block**, because `connect` targets a
   *listening* socket of this same program whose accept queue the model knows is
   empty (depth one, second connect refused), so the handshake completes in the
   kernel without the accept running; and `accept` is admitted only where the
   model knows a connection is queued.
3. **A send cannot block**, and this is the one **premise**:
   `SOCKET-V1-LOOPBACK-BUFFERED-SEND`. It is made quantitative rather than
   hand-waved — `MAX_PAYLOAD = 1024` per send and `MAX_INFLIGHT = 4096`
   outstanding on one stream, both inside the smallest default socket buffer of
   all three target platforms (≥ 8 KiB), with `Q085` refusing anything beyond.
   Unlike `SOCKET-V1-STREAM-BYTE-ORDER` this premise is **not checkable by the
   run** — a program that blocked forever would produce no evidence at all — so
   it is converted into a **bounded failure** instead: every wait the emitted
   program performs polls with a finite 5000 ms timeout and a distinct non-zero
   status. A false premise is a failed run in finite time, never a hang.

That is the honest position: two of the three are theorems, the third is a named
premise with a bounded failure mode, and the report says which is which.

### 2.4 Ephemeral ports — a correctness requirement

Every `bind` is to **port 0** and the kernel's choice is read back with
`getsockname` only to hand this same program's `connect` or `sendto` an address.
No port is spellable in source, and none reaches a certificate field or an
observation record. Two concurrent runs of the same emitted program, and parallel
runs of the whole test suite, therefore cannot collide on a port. The card
identified this as a correctness requirement rather than a nicety, and
`two_runs_of_one_program_write_identical_bytes` holds it.

### 2.5 UDP source verification — beyond what was asked

A UDP socket here is bound but never connected, so the kernel would deliver a
datagram from *any* sender to the same port. The certificate's receipt therefore
carries `from`: the socket of this same program the model says sent the datagram.
The emitted program reads the source address `recvfrom` reports and compares it
field for field with that socket's own bound address, returning the payload-
mismatch result on disagreement. A stray datagram delivered to the same
short-lived ephemeral port fails the run rather than being read as this one
(`a_datagram_receipt_names_the_socket_the_model_says_sent_it`).

### 2.6 Effects: `Alloc` and `Io`, never `Mut`

Creation, `accept` and `close` derive `Alloc` — they open and close custody of a
kernel resource. Every transport call derives `Io`. **`Mut` is never derived**,
because this route mutates no object of the program: the bytes a send commits
leave it entirely and the bytes a receive takes arrive as new values. Stated over
the whole grammar as `QCore.StdSocket.no_statement_derives_mut`, so a widening
changes a theorem rather than slipping past it.

---

## 3. Custody: inherited, not reimplemented

A socket is an **owned descriptor**. Creation and `accept` acquire one on the
ownership footprint's own `descriptors` axis; `close` releases it. Every custody
refusal — a leak at normal exit, a double close, a use after close, a rebound
live name — is the **unedited** `qcore_core::ownership` checker's, reached
through `std_socket::custody_operations` exactly as `source.buffer.v1` reaches it.
This module builds the operation list and attributes the diagnostic; it decides
none of those rules and re-implements none of them.

`a_leak_refuses_and_the_refusal_comes_from_the_unedited_ownership_checker`
demonstrates it arriving as that checker's own `OwnershipError::NormalPathLeak`
rather than as a re-worded copy — the `STD-BUFFER` evidence pattern.

`SocketCertificate::peak_live_descriptors` is read out of the ownership state's
own `live()` footprint after each prefix, not counted here.

`Q083` is **this pass's own code and not a reuse of `StandardBuffer`'s `Q054`**,
on §L21's rule that a code may not be reused by a second semantic pass — even
where the inherited refusal is literally the same checker's. The same applies to
`Q087` against `Q057`.

---

## 4. Determinism of the observation (constraint 15c)

Constraint 15c asks a route to say **why** every record of its observation is
written by the entry's thread rather than to assume it. For this route the answer
is structural and short:

> A `source.socket.v1` program has **one thread of control and creates none.** It
> spawns no worker, registers no callback and runs no reactor. The only
> statements that write bytes to a stream are `source.std-io.v1`'s own `print`
> and `log`, executed in the entry's own body in program order.

Therefore every record's emitter is `Emitter::Parent`, no worker is ever live, the
`OBS-REORDERING` live-set derivation puts each record alone in its own window —
making it a **barrier** — and the declared `qcore.obs-reordering.v1` relation
`is_strict()`. The byte comparison this route's tests perform is that relation's
own degenerate case (`strict_iff_byte_equality`), not a convention beside it.
`the_declared_reordering_relation_is_strict` asserts exactly this, on both
streams, and additionally checks that comparing against the relation *is* byte
equality.

**The socket traffic writes no record at all.** A byte on a loopback stream is
not an observation: it reaches no stream the observation is read from, and the
only way one becomes visible is when a source `print` renders a value a receive
bound. That is the boundary `COMPOSE-EVENT-LINES` drew for a drained wake byte.

Nothing that is not model-determined reaches an observation: not a descriptor
number, not a port, not a segment boundary, not a timing.

---

## 5. The close discipline, derived from `os_endpoint` rather than invented

`std_socket::close_disposition` calls `qcore_core::os_endpoint::close_poststate`
and publishes its answer on **both** branches, per close, in the certificate.
Every premise is justified separately:

| Premise | Value | Why |
|---|---|---|
| `epoch_live` | granted | the inherited ownership checker refused the program otherwise |
| `sole_custodian` | granted | single-threaded; no fork, no `dup` |
| `result_observed` | granted | the shim compares the result against zero |
| `errno_observed` | **denied** | the shim does not read `errno`: no arm of this route's behaviour depends on which failure occurred, because every failure exits |
| `last_reference` | granted | nothing on this route creates a second reference to an open description |
| `no_record_locks` | granted | a socket cannot carry a POSIX record lock |

Target is `Target::Unpinned`: the emitted program runs on three platforms and
pins none, so the relation is read at the conservative join.

**The measured answer.** Success branch: `DescriptorState::Released`,
`DescriptionState::Released`. Failure branch: **`DescriptorState::Attempted`** —
a *poisoned* state carrying the conservative residual duty set — with **no
permitted recovery**. That is what forbids a retry, and the emitted shim
accordingly exits with status 89 rather than retrying a failed close.

The card expected `Indeterminate` here and the relation answered `Attempted`.
That is the honest answer and it names its own repair: `Attempted` means *this*
analysis cannot map the outcome, so reading `errno` and pinning the target would
move some arms to `Released`. Recorded as the named follow-on
**`SOCKET-V1-CLOSE-ERRNO`** — a real improvement rather than a gap, since nothing
on this route depends on which failure occurred. The finding is asserted in
`the_close_disposition_comes_from_the_os_endpoint_relation`, so a change to the
relation is a test failure rather than a silent drift.

---

## 6. Per-arm platform honesty

The shim `crates/qcore-core/src/qcore_socket_compat.h` has three arms, on the
shape `c11_threads_compat.h` and `qcore_event_poll_compat.h` established.

| Arm | Descriptor type | Close | Wait | Startup | Evidence in this card |
|---|---|---|---|---|---|
| `_WIN32` | `SOCKET` (unsigned handle, **not** a CRT fd) | `closesocket` | `WSAPoll` | `WSAStartup(2,2)`; `ws2_32.lib` named from inside the emitted file via `#pragma comment` under MSVC | **runtime-verified by follow-on `SOCKET-V1-WINDOWS-RUN`**; see `docs/reports/socket-v1-windows-run/README.md` |
| `__APPLE__` | `int` | `close(2)` | `poll(2)` | none | **compile-path-correct only.** Residual **`SOCKET-V1-APPLE-RUN`** |
| else (Linux, other POSIX) | `int` | `close(2)` | `poll(2)` | none | **runtime-verified**: emitted C built with `-std=c11 -Wall -Wextra -Werror -pedantic`, run under ASan/UBSan/LSan, stdout byte-compared with the reference observation |

The two POSIX arms select the *same* implementation by name. They are separate
arms because what each must declare to expose the BSD socket API under
`-std=c11 -pedantic` differs: glibc needs `_POSIX_C_SOURCE`, Apple's
`sys/cdefs.h` keys its visibility level off `_DARWIN_C_SOURCE`.

**The Windows SOCKET-vs-fd boundary, decided honestly** (constraint 15d's
spirit): the emitted program never converts a `SOCKET` to an `int`, never prints
one, and never compares one against a POSIX fd. Per-arm typedefs
(`qcore_socket_fd`, `qcore_socket_len`, `qcore_socket_ssize`,
`qcore_socket_addrlen`) contain every signature difference, and the **observation
carries neither**. AF_INET loopback is used on all three arms — the AF_UNIX
capability dance is `std.event`'s and is not needed here, which is why constraint
15d is cited for its *discipline* rather than reused as a mechanism.

**What the residuals mean.** This card originally left both platform runs at the
TRACE-STDIO-WIN precedent: written and compile-path-correct, without a native-run,
sanitizer or byte-comparison claim. Follow-on `SOCKET-V1-WINDOWS-RUN` closes the
Windows residual with native `cl.exe` TCP and UDP runs and byte comparisons. The
Apple residual remains: the macOS branch of the ASan options is present and
correct (LSan is Linux-only) but has not been exercised on macOS hardware.

---

## 7. Exact test names

All in `compiler/tests/std_socket.rs` unless stated. **49 tests, all passing.**

**Route selection and acceptance** — `a_stream_module_selects_the_socket_profile`,
`a_datagram_module_selects_the_socket_profile`,
`the_certificate_publishes_every_acquisition_and_every_close`,
`the_certificate_publishes_the_bytes_every_receive_takes`,
`a_datagram_receipt_names_the_socket_the_model_says_sent_it`,
`the_close_disposition_comes_from_the_os_endpoint_relation`,
`the_derived_effect_row_carries_alloc_and_io_and_never_mut`,
`a_module_that_declares_too_little_is_refused`,
`the_published_certificate_is_independently_rederivable`,
`the_reference_observation_separates_the_two_streams`,
`the_declared_reordering_relation_is_strict`,
`the_emitted_program_routes_every_line_through_the_single_io_route`,
`a_bound_integer_name_is_an_admitted_payload_byte`.

**Custody (inherited)** —
`a_leak_refuses_and_the_refusal_comes_from_the_unedited_ownership_checker`,
`a_double_close_refuses`, `a_use_after_close_refuses`.

**Determinism boundary (`Q085`)** —
`a_receive_the_history_does_not_satisfy_refuses`,
`an_accept_with_nothing_queued_refuses`,
`a_second_unaccepted_connection_refuses`,
`a_stream_close_with_undrained_bytes_refuses_and_a_datagram_close_does_not`,
`a_listener_close_with_an_unaccepted_connection_refuses`.

**Transport state machine (`Q082`)** — `listening_on_a_datagram_socket_refuses`,
`connecting_to_something_that_is_not_listening_refuses`,
`listening_before_binding_refuses`,
`the_two_transports_never_stand_in_for_each_other`,
`sending_on_an_unconnected_stream_socket_refuses`,
`a_datagram_to_an_unbound_socket_refuses`.

**Ranges (`Q084`)** —
`a_byte_outside_the_range_refuses_rather_than_truncating`,
`an_empty_payload_refuses`, `a_datagram_is_taken_whole_or_not_at_all`.

**Crossing and operands (`Q086`/`Q081`)** —
`a_socket_as_a_payload_byte_refuses_as_a_boundary_crossing`,
`a_borrowed_render_refuses_because_this_route_publishes_no_identity`,
`a_payload_byte_that_names_nothing_refuses`,
`naming_something_that_is_not_a_socket_refuses`.

**Imports (constraint 13)** — `socketing_without_the_socket_import_refuses`,
`the_io_import_alone_does_not_grant_a_socket`,
`the_socket_import_alone_does_not_grant_standard_output`,
`importing_the_module_in_another_file_does_not_grant_this_one_the_capability`,
`a_misspelled_module_refuses_as_an_unknown_module`,
`closing_is_not_a_gated_socket_use`.

**Route ordering (the seam)** —
`a_module_that_buffers_and_sockets_stays_with_the_buffer_route`,
`a_module_that_dispatches_and_sockets_stays_with_the_event_route`.

**Reserved words (constraint 14)** — `the_socket_spelling_is_not_this_cards_word`,
`every_reserved_head_is_load_bearing`.

**Published examples** — `the_published_examples_take_this_route`,
`the_readme_transcript_is_the_real_one`.

**Native, Linux/macOS only** —
`native::the_tcp_example_runs_sanitized_and_writes_the_reference_observation`,
`native::the_udp_example_runs_sanitized_and_writes_the_reference_observation`,
`native::two_runs_of_one_program_write_identical_bytes`.

---

## 8. Lean

New module `proof/QCore/StdSocket.lean`, registered in `proof/QCore.lean` and
audited by `proof/AuditStdSocket.lean` (placeholder-free; every closure is
`propext` / `Classical.choice` / `Quot.sound` only, confirmed by `AuditAll`).

| Group | Theorems |
|---|---|
| Custody lifecycle | `an_empty_ledger_holds_no_descriptors`, `an_acquisition_adds_exactly_one`, `a_release_removes_exactly_its_own`, `a_release_keeps_every_other_entry`, `a_closed_socket_is_not_held`, `transport_statements_leave_the_ledger_unchanged`, `an_accept_acquires_beside_its_listener` |
| No use after close | `a_closed_socket_is_not_held` (the statement form: a closed socket is not held, so every later statement naming it is a use-after-close the inherited checker refuses) |
| Non-blocking receive | `a_receive_on_an_empty_stream_is_refused`, `an_admitted_receive_takes_its_full_count`, `a_receive_takes_a_prefix`, `a_commit_never_shortens`, `a_receive_leaves_exactly_the_rest` |
| Declared-payload soundness | `draining_returns_the_committed_bytes`, `two_commits_are_returned_in_order` |
| Datagram half | `a_datagram_receive_on_an_empty_queue_is_refused`, `a_partial_datagram_is_refused`, `an_admitted_datagram_receive_takes_the_head`, `a_datagram_arrives_whole` |
| Effects | `a_creation_derives_alloc`, `a_close_derives_alloc`, `a_transfer_derives_io`, `a_binding_derives_nothing`, `no_statement_derives_mut` |
| Operand wall | `the_operand_forms_are_exactly_two`, `an_operand_is_never_a_custody_act` |
| Module gate | `the_socket_capability_needs_its_module`, `the_socket_module_grants_no_stream` |
| Registry (9th module) | eleven new `QCore.ModuleImports` theorems, incl. `a_near_miss_socket_spelling_rejects` (which pins that `std.net` is **not** a registry entry) |

`QCore.ModuleImports.Imports` grows from eight fields to nine — the same widening
`STRING-SLICE` made from seven to eight. Every existing theorem name and meaning
is preserved; the generic `admitted_capability_requires_its_module` argument is
instantiated once more, so the socket gate cannot drift from the eight that
precede it.

**What is not proved.** That the Rust lowering extracts these statements
faithfully from source tokens; that the platform's loopback transport is the FIFO
the model assumes (that is `SOCKET-V1-LOOPBACK-BUFFERED-SEND`, converted into a
bounded failure rather than proved away); or anything about a failed primitive —
the emitted program exits there, so a run either produces the modelled
observation or produces none. Partial correctness only, stated rather than proved
away.

---

## 9. The `driver.rs` seam

**Shared ground with the parallel string-io card. Flagged as requested.** Every
edit is additive and confined to five clearly delimited places in
`crates/qcore-verification/src/driver.rs`:

1. **Import line** — `std_socket, std_socket_c_emit` added to the existing
   `qcore_core::{…}` use list.
2. **`CheckedSourceModule`** — one new variant `StandardSocket(Box<CheckedSocketModule>)`,
   plus `| Self::StandardSocket(_)` appended to the eleven existing `=> None`
   arms (each was already a multi-variant `| Self::Event(_) => None,` line; the
   change is one added alternative per arm).
3. **One new struct** `CheckedSocketModule` and **one new accessor**
   `as_standard_socket`, placed beside their buffer/string siblings.
4. **`SourceExecutionProfile`** — one new variant `StandardSocketV1`; **one new
   `function_count` arm**.
5. **Route selection and `compile_socket_source`** — one `if` block inserted
   between the string route and the standard-IO route, and one function modelled
   line for line on `compile_buffer_source`.

A merge conflict with the sibling card is expected only in (2) and (4), where
both cards append to the same enums, and in the `=> None` arm lists. Both are
mechanical.

**Route ordering, argued.** The socket route is selected **after** every
concurrent route, the extern route, the event route, the buffer route and the
string route, and **before** the standard-IO route:

* after buffer/string, so a module that does both keeps *their* refusal
  (`SOCKET-BUFFER-PAYLOAD`, `SOCKET-STRING-PAYLOAD`) — measured;
* after event, so a module that dispatches and sockets keeps `source.event.v1`'s
  refusal (`SOCKET-EVENT-COMPOSE`) — measured;
* before standard-IO, so a module that receives a byte and prints it reaches this
  route, which models both — its line-producing statements are
  `source.std-io.v1`'s own, read by that route's reader and rendered by its
  emitter, over **scalars only**;
* a module that only prints fails the conjunction and falls through byte for
  byte.

---

## 10. Standing gate

Run at the card tip, Linux, with `$HOME/.elan/bin` on `PATH`.

| Step | Result |
|---|---|
| `git submodule update --init --recursive` | clean (needed once in this worktree; the first `cargo test --workspace` failed `libevent_submodule_matches_the_audited_revision` before it, which is invariant 5 working as designed and not a finding about this card) |
| `cargo test --workspace` | **3,825 passed, 0 failed, 8 ignored** across 203 test binaries |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit **0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit **0** |
| `python3 scripts/check_lean_audits.py` | **"Passed 100 Lean audits"** (was 99; this card adds `AuditStdSocket.lean`) |

Constraint 16's five load-sensitive provider/host rows all **passed in the full
run**, so there was nothing to re-verify in isolation. This card touches no
provider, host or package file, so none of them is reachable from it; the
`providers/` workspace was not rebuilt, and constraint 18 does not apply because
no manifest template and no `qcore-schemas` version changed.

**Corpus neutrality.** No gateway, contract or provider file is touched; the
pinned 185-unit corpus, its leak count (266) and the contract corpus (73 modules)
are unread and unmoved by this card.

---

## 11. Named follow-ons

| Name | Missing capability |
|---|---|
| `SOCKET-EXTERNAL-PEER` | a `connect` to a spelled address. A determinism change, not a syntax change: it removes §1.1's decidability and needs a different evidence story |
| `SOCKET-BUFFER-PAYLOAD` | a `send` whose payload is a `std.buffer` extent. Needs a borrowed extent crossing a boundary the ownership IR's borrow nodes cannot carry a fraction for |
| `SOCKET-STRING-PAYLOAD` | a `send` whose payload is a string view. For the HTTP card; owned jointly with whichever card owns text |
| `SOCKET-EVENT-COMPOSE` | registering a socket with `source.event.v1`'s reactor. Two custody models in one entry |
| `SOCKET-BACKLOG-DEPTH` | an accept queue deeper than one. Needs a model of whether a backlogged connect completes before its accept |
| `SOCKET-UDP-CONNECT` | a connected datagram socket |
| `SOCKET-V1-EINTR-RESUME` | resuming an interrupted call. Today a `-1` return fails the run, because the retry count is not model-determined |
| `SOCKET-V1-CLOSE-ERRNO` | reading `errno` and pinning the target on a failed close, moving some arms from the poisoned `Attempted` to `Released` (§5) |
| `SOCKET-V1-WINDOWS-RUN` | **closed** by `docs/reports/socket-v1-windows-run/README.md`: native Visual Studio `cl.exe` TCP/UDP runs and byte comparisons; no sanitizer claim |
| `SOCKET-V1-APPLE-RUN` | the same for the Apple arm |
| `SOCKET-V1-SHUTDOWN` | half-close (`shutdown(2)`), socket options, non-blocking mode, IPv6 |

## 12. Named premises

| Premise | Checked by the run? |
|---|---|
| `SOCKET-V1-STREAM-BYTE-ORDER` — the bytes a receive takes are the bytes an earlier send of this program committed, in order | **Yes.** The emitted program compares and exits with status 88 on disagreement |
| `SOCKET-V1-LOOPBACK-BUFFERED-SEND` — a bounded send with a bounded in-flight total completes without waiting for the peer to drain | **No**, and it cannot be: a blocked program produces no evidence. Converted into a bounded failure — every wait has a finite timeout and a distinct status |

## 13. Files

| Path | What |
|---|---|
| `crates/qcore-core/src/std_socket.rs` | the checked model: statements, state machine, committed history, certificate, `Q080`–`Q087` |
| `crates/qcore-core/src/std_socket_c_emit.rs` | reference execution, the strict observation, C11 emission, exit statuses 80–89 |
| `crates/qcore-core/src/qcore_socket_compat.h` | the embedded three-arm shim |
| `crates/qcore-frontend/src/source_socket.rs` | the exact source lowering and the route detector |
| `crates/qcore-core/src/module_imports.rs`, `crates/qcore-frontend/src/module_imports.rs` | `std.socket` and its `Q015` gate |
| `crates/qcore-frontend/src/frontend.rs` | `SemanticPass::StandardSocket`, the `Q080` presentation code, the ten reserved words |
| `crates/qcore-verification/src/driver.rs` | the seam of §9 |
| `compiler/tests/std_socket.rs` | the 49 tests of §7 |
| `examples/sockets/{tcp_echo,udp_echo}.qcore`, `examples/sockets/README.md` | the published examples and their verified transcripts |
| `proof/QCore/StdSocket.lean`, `proof/AuditStdSocket.lean`, `proof/QCore/ModuleImports.lean` | §8 |
| `docs/QCORE_SPEC_r0_3.md` §L21 | the `Q080`–`Q089` row |
| `AGENTS.md` | the diagnostics-namespace row and the constraint-14 census record |
