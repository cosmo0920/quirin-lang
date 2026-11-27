# SOCKET-INET-ADDRESSES — declared IPv4 and IPv6 addresses with ports

**Card:** `SOCKET-INET-ADDRESSES` (wave 26). **Route:** `source.socket.v1`,
behind the `std.socket` bundled module. **Parent:** `QUIRIN-SOCKET-V1`.
**Diagnostic codes allocated: zero.**

**User directive, quoted:** *"the current status of socket is only for local loop
back so we need to bind IPv4 and IPv6 addresses with ports for outbound and
inbound transportations."*

**What shipped.** A program may now write its addresses down:

* **Inbound** — `bind S 127.0.0.1:8080;` and `bind S [::1]:8080;` bind the
  address the program declares, rather than IPv4 loopback on a port only the
  kernel knows.
* **Outbound** — `connect C 127.0.0.1:8080;` and
  `sendto U 127.0.0.1:9000 [BYTE, …];` name their target by that address.
* A port is an integer literal **or a name this entry bound to a
  check-time-constant integer**, so one number can be read at both ends.

Both published examples run natively, sanitized, byte-compared on Linux; the
Windows arm is wired through the established MSVC harness from the start and the
Apple arm is compile-path-correct. The card is **corpus-neutral**: no gateway,
contract, audit or provider file is touched, and the pinned 185-unit corpus, its
leak count and the contract corpus are unread and unmoved.

**Status language.** This card publishes no verdict about any pinned corpus unit
and moves no corpus number.

---

## 1. The one decision everything else follows from

> **A declared address is not a spelled peer.** A target address is resolved, at
> check time, against the addresses **this same program's own `bind` statements
> declared**. An address no bind of this program answers to is refused by name.

That is why the parent card's determinism argument survives intact. `connect`
still names a listening socket of this program — it may now do so by address
instead of by name — so every `accept`, every `recv` and every connection
completion stays decided by the program's own committed history, and
`SOCKET-EXTERNAL-PEER` stays exactly where it was.

What the declared form *adds* is real and is the user's request: the emitted
program genuinely `bind`s the address written down and genuinely `connect`s to
the address written down, instead of reading a peer's address back with
`getsockname`. Inbound and outbound both name the endpoint.

**Where the external-peer refusal lives, exactly.** In
`qcore_core::std_socket::resolve_addresses`, as the `AddressOutcome::NoSuchEndpoint`
arm, produced when no entry of the program's own bind list carries the resolved
address; `address_error` maps it to `SocketError::WouldBlock` (`Q085`), the
determinism-boundary code, with `SOCKET-EXTERNAL-PEER` written in the message.
It is `Q085` rather than a shape code because that *is* the condition: a connect
to something outside the program is a statement whose completion this program's
own history does not determine. Pinned by
`an_address_no_bind_of_this_program_declares_refuses_as_an_external_peer`, and
proved as a property of the model by
`QCore.SocketInetAddresses.an_undeclared_address_reaches_nothing` and
`reaching_names_a_bind_of_this_program`.

---

## 2. Grammar, and the census (constraint 14)

```text
stmt ::= …
       | "bind" NAME ";"                       -- unchanged: IPv4 loopback, kernel port
       | "bind" NAME ADDRESS ";"               -- NEW
       | "connect" NAME NAME ";"               -- unchanged
       | "connect" NAME ADDRESS ";"            -- NEW
       | "sendto" NAME NAME PAYLOAD ";"        -- unchanged
       | "sendto" NAME ADDRESS PAYLOAD ";"     -- NEW
ADDRESS ::= IPV4 ":" PORT | "[" IPV6 "]" ":" PORT
PORT    ::= INTEGER | NAME                     -- a check-time-constant binding
```

### 2.1 Zero reserved words, zero lexer changes

`SOCKET-INET-ADDRESSES` adds **no** reserved word and **changes no lexer rule**.
An address is read back out of the tokens the existing lexer already produces.
The census is a measurement of that lexer, run rather than assumed:

| Spelling | Lexes as (measured) |
|---|---|
| `127.0.0.1:8080` | `Float("127.0") Dot Float("0.1") Colon Integer("8080")` |
| `0.0.0.0:0` | `Float("0.0") Dot Float("0.0") Colon Integer("0")` |
| `192.168.1.1:65535;` | `Float("192.168") Dot Float("1.1") Colon Integer("65535") Semicolon` |
| `[::1]:8080` | `LeftBracket ColonColon Integer("1") RightBracket Colon Integer("8080")` |
| `[::]:1` | `LeftBracket ColonColon RightBracket Colon Integer("1")` |
| `[2001:db8::1]:80` | `LeftBracket Integer("2001") Colon Word("db8") ColonColon Integer("1") RightBracket Colon Integer("80")` |
| `[fe80::0e12]:1` | `LeftBracket Word("fe80") ColonColon Integer("0") Word("e12") RightBracket Colon Integer("1")` |
| `10.0.0.1:PORT` | `Float("10.0") Dot Float("0.1") Colon Word("PORT")` |

Three findings, and all three are why no lexer change is needed:

1. **A dotted quad is always three tokens.** `127.0` is a *float* spelling
   (`digit+ '.' digit+`), and the second `.` cannot begin one because what
   follows is another digit run already consumed — so every dotted quad lexes as
   `Float Dot Float`, whatever its components. `TokenKind::Float` is produced by
   the lexer and **consumed nowhere else in the workspace** (measured: one
   construction site, zero readers), so claiming it here collides with nothing.
2. **`::` is already one atomic token** (`Symbol::ColonColon`), and `[` `]` `:`
   `.` `%` are all existing symbols.
3. **The `[| |]` and `..` golden surfaces are untouched.** `[|` wins over `[` by
   the lexer's fixed width-two longest match, and an address never begins `[|`;
   `0..3` still lexes as `Integer Dot Dot Integer` (LOOP-V1's range), and an
   address is read at no range position. *Seam note for the integrator:* the
   parallel `LOOP-UNROLL-BUDGET` card owns the loop pass and the `[| |]`
   attribute slot. This card touches neither, and its address reader is invoked
   only at three statement heads (`bind`, `connect`, `sendto`) at one token
   position each — but if that card adds `#` or `where` tokens, the one place to
   re-read is `source_socket::token_text`, whose set of address-bearing tokens is
   a closed list.

### 2.2 How an address position is told from a name position

Without a keyword, and with one fact:

> **Every address of this route spells a port.** So the token run at the target
> position carries a `:`, and a socket name never does.

`source_socket::address_run` takes the maximal run of **lexically adjacent**
address-bearing tokens at the position and returns it only if it contains a `:`.
Adjacency — the previous token's `span.end` being exactly this one's
`span.start` — is what makes an address one lexeme: `127.0.0.1 : 8080` is
**not** an address and refuses (`a_spaced_address_is_not_an_address`). Adjacency
is also what stops the run at the statement's `;`, at a payload's `[`, and at a
socket name written beside it.

### 2.3 The one name-resolution exclusion, and its census

The host half of an address is spelled with words — a hexadecimal IPv6 group like
`fe80` lexes exactly as an identifier does, and so does every label of a hostname
— so `lexical_references` would collect them as *referenced names* and refuse at
`Q013`. **Measured before the fix:** `bind listener [fe80::1]:8080;` gave
`Q013 unknown name 'fe80'`.

The answer is the `len` / `concat` / `borrow_utf8` discipline — narrow the rule
rather than reserve a word — and the rule here is not a shape heuristic but **the
address reader itself**. `source_socket::address_host_span(window)` returns the
token index range of a statement's address **host**, at exactly the three
statement heads and the one token position the lowering reads an address from.
Two properties matter:

* **The port is deliberately outside the range.** A port may be a name, and it is
  a genuine reference that must still resolve. `let port = 49301;
  bind s 127.0.0.1:port;` works; `bind s 127.0.0.1:missing;` still refuses at
  `Q013` (`a_port_that_names_nothing_refuses`).
* **A hostname now reaches this route's own reader**, and is refused there by name
  with `SOCKET-DNS` in the message, instead of being reported as an unknown name.

**Census, run by the test rather than by hand.**
`this_card_reserves_no_word_and_its_address_rule_collides_with_nothing` lexes
every `.qcore`/`.qc` file in the repository (**424** files) and asks
`address_host_span` at every token offset of every one. It claims a span in
**exactly two files** — this card's own `examples/sockets/inet_v4_echo.qcore` and
`examples/sockets/inet_v6_echo.qcore` — and nowhere else. The assertion is an
equality against that two-element list, so a later fixture that trips the rule is
a test failure rather than a silent behaviour change.

`std.net` remains **not** a registry entry, and the module stays `std.socket`:
this card supplies addressing, which `QUIRIN-SOCKET-V1` §2.1 named as one of the
things `std.net` would have claimed — and it supplies it *inside* the socket
vocabulary, as an argument to statements that already existed, rather than as a
new capability space. That is the reason the parent card's naming argument still
holds after this card rather than in spite of it.

---

## 3. The admitted IPv6 textual subset, decided

`qcore_core::socket_address::parse_ipv6` admits **RFC 4291 §2.2 forms 1 and 2**:
eight colon-separated groups of one to four hexadecimal digits, with at most one
`::` standing for **one or more** all-zero groups. Case is immaterial, because
comparison is on the parsed bytes and never on the text — `::1`,
`0:0:0:0:0:0:0:1` and `FE80::1`/`fe80::1` are handled as RFC 4291 says they
should be (`a_compressed_and_an_uncompressed_spelling_are_one_address`).

Two forms are refused **by their own names**, because each is a capability
decision rather than a typo:

| Refused | Why, and its residual |
|---|---|
| **Form 3, embedded IPv4** (`::ffff:127.0.0.1`) | It denotes a v4 peer reached through a v6 socket — precisely the dual-stack shape §4's `IPV6_V6ONLY` decision declines to model. Every v6 socket an admitted program creates sets `IPV6_V6ONLY`, so such an address could never be reached; admitting the spelling would name a capability the emitted program does not have. `SOCKET-INET-V6-EMBEDDED-V4` |
| **A zone identifier** (`fe80::1%eth0`) | A zone names an interface of the **running host**, not a constant this checker can evaluate — the same ground a hostname is refused on. `SOCKET-INET-V6-ZONE` |

`%` is deliberately in the address-token set so that a zone is *seen* and refused
by its own name rather than truncating the run into a generic shape error. One
honest wrinkle, recorded: a zone spelled as a **word** (`%eth0`) additionally
reaches name resolution first and refuses at `Q013`, because `eth0` is outside the
host span. A numerically spelled zone (`%2`) reaches the named refusal, and the
test uses that spelling.

**IPv4 components carry no leading zero.** `127.0.0.01` is refused rather than
read: glibc's `inet_pton` refuses it and other platforms have historically read it
as octal, and this route runs on three platforms and pins none — so the one
spelling every platform agrees about is the only one admitted, and the refusal
says so (`a_leading_zero_component_refuses_rather_than_being_read`).

**An unbracketed IPv6 host asks for its brackets** rather than being split at a
guess: inside `[…]` every `:` belongs to the host and the separator is the one
after `]`, and that is what makes `split_host_and_port` total.

---

## 4. The `IPV6_V6ONLY` decision, per arm

**Decision: set explicitly to `1`, on every arm, immediately after creation and
before any bind. A `setsockopt` failure fails socket creation.**

| Arm | Platform default | What the emitted program does | Evidence |
|---|---|---|---|
| `_WIN32` (Winsock) | **1** (on) | `setsockopt(IPPROTO_IPV6, IPV6_V6ONLY, 1)` | compile-path-correct; harness wired, residual `SOCKET-INET-WINDOWS-RUN` |
| `__APPLE__` | 0 (off) | the same | compile-path-correct; residual `SOCKET-INET-APPLE-RUN` |
| else (Linux/POSIX) | 0 (off, `net.ipv6.bindv6only`) | the same | **runtime-verified**: sanitized native run, byte-compared |

The setsockopt call is written **once**, outside the three `#if` arms, so "the
same value on every arm" is true by construction rather than by three agreeing
copies. It is also proved: `QCore.SocketInetAddresses.v6only` is a function from
the three arms, and
`the_v6only_decision_is_made_explicitly_on_every_arm` /
`the_v6only_decision_does_not_vary_by_arm` make a change to any single arm a
*failed proof*.

**Why 1 rather than 0.** The model keeps the two address families **disjoint** —
`::` and `0.0.0.0` are different addresses, and a socket is created in exactly one
family. With `IPV6_V6ONLY` off, a v6 socket bound to `::` would also receive v4
traffic; then a program binding both `::` and `0.0.0.0` would collide at run time
in a way check time did not predict, and a datagram's declared source would stop
being verifiable. Setting the option makes the C agree with the model. Inheriting
it would make the same program bind different things on Linux and on Windows,
which is the class of silent platform divergence this project refuses.

### 4.1 A host without IPv6, on constraint 15d's shape

Compiling is not evidence that the host *has* IPv6. `qcore_socket_create_family`
asks the running kernel for the socket and answers with **three** results:

* `0` — a socket in the declared family;
* `2` — the host **explicitly** does not support this family
  (`EAFNOSUPPORT` / `EPROTONOSUPPORT` / `EPFNOSUPPORT`, or their `WSAE…`
  equivalents). The emitted program exits **92** (`STATUS_IPV6_UNAVAILABLE`)
  having written nothing;
* `1` — every other failure, which refuses at `STATUS_CREATE` (81).

**Nothing falls back to IPv4.** A fallback would bind a different address from the
one the program declared, which is the one thing a declared address exists to
prevent, and the emitted entry of a declared-IPv6 program contains no AF_INET
creation at all (`a_host_without_ipv6_is_a_recorded_refusal_and_never_a_fallback`).
Proved as `creation_never_changes_the_family`.

This is also the **one** place in the shim that reads `errno`, and it is
deliberate: §5 of the parent card records that the close discipline does *not*
read `errno` because no arm of its behaviour depends on which failure occurred.
Here two arms do — a recorded "no IPv6 on this host" and a refusal — so the
premise that made `errno_observed: false` right there makes reading it right here.

**The native test detects and records the skip.**
`native::the_declared_ipv6_example_runs_sanitized_or_records_that_this_host_has_no_ipv6`
returns the exit status rather than asserting it: on 92 with empty stdout it
prints a recorded skip and returns; on anything else it demands 0 and a byte-exact
match. On the Linux machine this card was measured on, IPv6 **was** available and
the row ran the full byte comparison.

---

## 5. Inbound and outbound: the example shapes, and what ran

Two published examples, both under `examples/sockets/`, both indexed in that
directory's README with pinned transcripts.

| Example | Shape | Transcript |
|---|---|---|
| `inet_v4_echo.qcore` | `let port = 49301;` · `bind listener 127.0.0.1:port;` · `listen` · `connect client 127.0.0.1:port;` · `accept` · five-byte send, reversed echo, both `recv` forms · three closes | `101\n0x00000065\n` |
| `inet_v6_echo.qcore` | the same over `[::1]:49302` | `101\n0x00000065\n` |

Both spell **inbound** (a declared bind) and **outbound** (a declared connect) in
one program, which is the directive's two halves in one artefact. The v4 example
also demonstrates the check-time-constant port binding: one number, read by the
bind and by the connect, so they cannot drift.

**Byte-comparison results.** Both compile under
`-std=c11 -Wall -Wextra -Werror -pedantic` with ASan/UBSan/LSan and their stdout
matches the reference observation exactly, with empty stderr. The v4 row runs the
program **twice inside one test** and compares the two runs: byte-identical, and
the second run binds successfully inside the first run's `TIME_WAIT` (see §6). A
declared-address **UDP** pair (`49340` → `49341`) runs sanitized too.

### 5.1 Emission byte-identity for everything that came before

The card's claim, stated precisely and measured two ways:

* **The emitted statement sequence of a program that declares no address is
  unchanged.** `the_implicit_forms_emit_exactly_the_statements_they_always_did`
  pins that `tcp_echo` and `udp_echo` still emit
  `qcore_socket_create(0, &sk_listener)`,
  `qcore_socket_bind_loopback(sk_listener)` and
  `qcore_socket_connect(sk_client, sk_listener)`, and that their entry bodies
  contain **none** of the seven declared-address entry points and no `host[]`
  array. The mechanism is that a socket which declares no family is created in
  `Family::V4`, and the V4 arms of create/bind/connect/sendto/recvfrom are the
  *original* functions, untouched.
* **The embedded shim grew by addition only, in code.**
  `git diff --numstat crates/qcore-core/src/qcore_socket_compat.h` = **326
  insertions, 9 deletions**, and every one of the nine deleted lines is inside the
  file's opening **comment block** — two paragraphs that said the route had only a
  kernel-chosen port and no options, reworded because it now has both. No existing
  function body, signature, `#include` list or `#if` arm was edited. (The embedded shim is part of every emitted socket program, so its text
  does change those files — exactly as `SOCKET-EVENT-COMPOSE` changed them when it
  added `qcore_socket_dispatch`. The invariant that is preserved, and the one that
  matters, is that no previously-emitted *statement* changed.)

All of `std_socket` (**49**), `socket_event_compose` (**44**), `std_http`
(**67**), `string_slice_io` (**22**) and `compose_event_lines` (**37**) pass
**unchanged** — no test edited, no expectation moved.

---

## 6. The port and test discipline, argued

A declared fixed port is the one thing in this card that can collide with
something outside the program, and the card does not pretend otherwise.

**What is decided at check time.** Two `bind` statements of **one program**
declaring the same non-ephemeral address are refused at `Q082`, because the
checker knows both (`two_binds_of_one_declared_address_refuse_at_check_time`).
Two binds of an **ephemeral** address are admitted, because the kernel picks a
different port for each (`two_binds_of_one_ephemeral_address_stay_admitted`).

**What is decided at run time, loudly.** A collision with a live socket outside
this program fails the emitted program with `STATUS_BIND` (82) — a bounded,
distinctly-statused failure, never a different observation and never a hang.

**The `SO_REUSEADDR` decision, per arm.** This card sets one property:

> A declared bind **succeeds when no live socket holds the address, and fails when
> one does.**

That is not the default on either platform family, and the two need **opposite**
options to reach it — the same shape as `_POSIX_C_SOURCE` versus
`_DARWIN_C_SOURCE`, and for the same reason:

| Arm | Option | Why |
|---|---|---|
| POSIX | `SO_REUSEADDR` **set** | Without it a listener cannot rebind an address while a connection *this program already closed* sits in `TIME_WAIT`. That is not a live holder and must not fail — measured: the second sanitized run of `inet_v4_echo` exited **82** before this was set. `SO_REUSEADDR` permits exactly that rebind and still refuses a live duplicate. |
| Winsock | **not set** | There `SO_REUSEADDR` means something else: it permits a *second live socket* on the same address, which is address hijacking and the opposite of the decided property. The default already refuses a live duplicate. |

`SO_REUSEPORT` is set **nowhere**. `qcore_socket_bind_loopback` — the implicit
form — is untouched, so no pre-existing program's emission changes.

**The test discipline, decided.** The card's brief offered "examples may spell
fixed ports only if their tests rebind to ephemeral, or spell port 0". Neither is
available: connect-by-address *requires* a non-ephemeral port, which is the
capability the card exists to demonstrate, and rebinding in the test would mean
the published example is not the thing that ran. So:

1. Every published fixed-port example uses a **distinct** port from the IANA
   dynamic/private range (49301, 49302), which no registered service claims.
2. Every **check-time** fixture in the suite uses its own distinct port too
   (49310–49341), so no two rows of the suite contend.
3. The two native runs of the v4 example are **one test**, not two, precisely so
   that two rows cannot contend for 49301 concurrently.
4. A collision is a **loud failure**, never a wrong observation.
5. Every pre-existing example and every fixture of `std_socket.rs` keeps port 0,
   which stays the collision-free spelling and the default.

What this does **not** achieve: two concurrent runs of the whole test suite on
one host would contend for 49301/49302. That cannot be made collision-free by
construction while a fixed port is spelled, and it is recorded as
**`SOCKET-INET-FIXED-PORT-COLLISION`** rather than papered over — it fails loudly
if it happens.

---

## 7. Address failure, represented or refused by name

The card asked for address failures to be decided per the parent card's
bounded-failure model. They are, and each is placed deliberately:

| Outcome | Where it is decided | How |
|---|---|---|
| Bind to an address a **second socket of this program** declares | check time | `Q082`, by name |
| Bind to an address something **outside** the program holds | run time | `STATUS_BIND` (82), bounded, distinct, nothing retried |
| Connect to an address **no bind of this program** declares | check time | `Q085`, the external-peer refusal |
| Connect refused by the peer | **unreachable** | the checker proved a socket of this program is listening there; a refusal at run time is `STATUS_CONNECT` (84), which is evidence about the platform |
| A **family mismatch** between socket and endpoint | check time | `Q082`, by name |
| A **port** outside `0..=65535` | check time | `Q084` (see §8) |
| A target naming a **kernel-chosen** port | check time | `Q085` — an ephemeral port is not known at check time and so is not a name |
| IPv6 unavailable on the host | run time | `STATUS_IPV6_UNAVAILABLE` (92), recorded, never a fallback |

Nothing retries anywhere. Nothing falls back anywhere.

---

## 8. Diagnostics: zero codes allocated, one condition widened

Per the `Q-NAMESPACE-DECISION` ice box, this card allocates **no** `Q` code. Every
refusal extends an existing `Q080`–`Q089` condition, on the
`SOCKET-EVENT-COMPOSE` / `NATIVE-LIBRARY-HEADER-DECORATION` precedent.

| Condition | Code | What this card adds to it |
|---|---|---|
| the statement shape this profile models | `Q080` | a spelling that is not an address: a hostname, an unbracketed IPv6 host, the embedded-IPv4 form, a zone identifier, a leading-zero IPv4 component, a malformed port, a spaced address |
| an operand that is not a check-time constant this entry bound | `Q081` | a port naming something this entry bound to no integer |
| the transport state machine's own refusal | `Q082` | a family mismatch between a socket and its endpoint; a second bind of one declared address inside one program |
| an inherited, unedited custody model refused | `Q083` | *(unchanged; but see §9 — the loan now sees a listener reached by address)* |
| **a declared numeric operand outside the range this position models** | `Q084` | **widened**: from a byte, a payload length and a receive count to those **and a port** (`0..=65535`) |
| the determinism boundary | `Q085` | a target address no bind of this program declares (the external-peer refusal); a target naming a kernel-chosen port |
| a socket at a position it may not cross | `Q086` | a socket at a **port** position |

The `Q084` widening is the only condition change, and it is stated in
`docs/QCORE_SPEC_r0_3.md` §L21 and in `AGENTS.md`'s namespace table rather than
taken silently. A port written as a **literal** never reaches the core's arm: the
source reader refuses an out-of-range literal where it reads it, because at that
point the text is all it has. The core arm exists for the form the reader cannot
decide — a check-time-constant binding
(`a_port_binding_out_of_range_refuses_at_the_same_code`).

---

## 9. Model: one resolution, read by both halves

The transport check and the **custody** operation list must name the *same*
listener, or constraint 15g's loan could be evaded by spelling a socket
differently: a `connect` on a `watch`ed socket refuses because it is an ownership
*write* of that socket, and a connect that reached the listener by address without
the operation list noticing would launder that refusal away.

So `qcore_core::std_socket::resolve_addresses` is **one** total, diagnostic-free
forward walk producing:

* `outcomes` — one `AddressOutcome` per `bind`/`connect`/`sendto`, by statement
  index, including the forms that spell no address;
* `families` — the address family every socket that declares one is created in;
* `locals` — the address every bound socket holds.

`custody_operations` and `check_socket_program` both read it — the first for the
listener a `connect` writes and the target a `sendto` reads, the second for every
refusal. `the_loan_sees_the_same_listener_through_an_address` pins the
consequence: `watch reg listener read;` followed by
`connect client 127.0.0.1:49326;` is refused at `Q083` by the unedited ownership
checker, exactly as the by-name spelling is.

**Family assignment, stated.** A socket's family is decided **once** and never
re-decided: by the address its own `bind` declared, or — for a connector or a
datagram sender that declares none — by the peer it first reaches; an accepted
socket inherits its listener's. A socket that neither binds nor reaches a peer
declares no family and is created in IPv4, which is the family every program
written before this card already had
(`a_socket_that_declares_no_family_is_created_in_ipv4`).

**The certificate grows by two fields**, both rechecked field-for-field by
`recheck_socket_certificate`: `families` (one row per acquisition, in program
order) and `declared_addresses` (`(event, ResolvedAddress)` for every declared
position). A program that declares no address publishes an empty
`declared_addresses` and an all-IPv4 `families`.

**One parse, and the emitted program carries octets.** The address text is parsed
once, in `qcore_core::socket_address`, and the emitted C carries the wire bytes —
`const unsigned char host[] = { 127u, 0u, 0u, 1u };`. **No emitted program calls
`inet_pton`**, so no second parser can read one spelling differently from the
first (`a_declared_address_is_emitted_as_bytes_and_never_as_text`). This is the
rule a payload byte already follows, applied to an address. A port follows the
same rule as a payload operand: a literal is written as the number the checker
read, a bound name as that binding's own C expression.

---

## 10. Exact test names

**`compiler/tests/std_socket_inet.rs` — 36 tests, all passing.**

*Acceptance* — `a_declared_ipv4_endpoint_is_bound_and_reached`,
`a_declared_ipv6_endpoint_is_bound_and_reached`,
`a_hexadecimal_ipv6_group_is_an_address_and_not_an_unknown_name`,
`a_port_binding_is_read_at_both_ends`,
`a_declared_address_still_admits_a_kernel_chosen_port`,
`a_datagram_reaches_a_declared_address`.

*Certificate* — `the_certificate_publishes_the_family_of_every_socket`,
`a_socket_that_declares_no_family_is_created_in_ipv4`,
`the_certificate_publishes_every_declared_address_with_its_resolved_port`,
`the_published_certificate_is_independently_rederivable`.

*Emission* — `the_implicit_forms_emit_exactly_the_statements_they_always_did`,
`a_declared_address_is_emitted_as_bytes_and_never_as_text`,
`an_ipv6_program_creates_its_sockets_in_the_declared_family`,
`every_ipv6_socket_sets_v6only_explicitly_and_a_failure_to_set_it_fails_creation`,
`a_host_without_ipv6_is_a_recorded_refusal_and_never_a_fallback`.

*The determinism boundary* —
`an_address_no_bind_of_this_program_declares_refuses_as_an_external_peer`,
`an_ephemeral_port_is_not_an_endpoint_name`,
`the_loan_sees_the_same_listener_through_an_address`.

*Shapes* — `a_hostname_refuses_by_name_because_there_is_no_resolution`,
`an_ipv6_address_without_brackets_asks_for_them`,
`the_embedded_ipv4_form_and_a_zone_identifier_refuse_by_their_own_names`,
`a_leading_zero_component_refuses_rather_than_being_read`,
`a_spaced_address_is_not_an_address`.

*Ports* — `a_port_literal_out_of_range_refuses`,
`a_port_binding_out_of_range_refuses_at_the_same_code`,
`a_port_that_names_nothing_refuses`,
`a_socket_at_a_port_position_refuses_as_a_boundary_crossing`.

*Families and collisions* —
`the_two_address_families_do_not_stand_in_for_each_other`,
`two_binds_of_one_declared_address_refuse_at_check_time`,
`two_binds_of_one_ephemeral_address_stay_admitted`.

*Census (constraint 14)* —
`this_card_reserves_no_word_and_its_address_rule_collides_with_nothing`.

*Published examples* — `the_published_examples_take_this_route`,
`the_readme_transcript_is_the_real_one`.

*Native, Linux/macOS only* —
`native::the_declared_ipv4_example_runs_sanitized_twice_and_writes_the_reference_observation`,
`native::the_declared_ipv6_example_runs_sanitized_or_records_that_this_host_has_no_ipv6`,
`native::a_declared_datagram_address_runs_sanitized`.

*Native, Windows only (wired, awaiting a run)* —
`native_windows::the_declared_ipv4_example_runs_under_cl_and_writes_the_reference_observation`,
`native_windows::the_declared_ipv6_example_runs_under_cl_or_records_that_this_host_has_no_ipv6`.

**`crates/qcore-core/src/socket_address.rs` — 11 unit tests** covering the parser
directly: `a_dotted_quad_parses_to_its_octets`,
`a_leading_zero_component_is_refused_rather_than_read`,
`a_hostname_is_refused_as_a_name_and_not_as_a_malformed_address`,
`the_two_degenerate_ipv6_forms_parse`,
`a_compressed_and_an_uncompressed_spelling_are_one_address`,
`the_embedded_ipv4_form_and_a_zone_are_refused_by_their_own_names`,
`an_elision_stands_for_at_least_one_group`,
`an_endpoint_carries_a_host_and_a_port`,
`a_bare_ipv6_host_asks_for_its_brackets`,
`a_port_out_of_range_refuses_rather_than_wrapping`,
`the_canonical_rendering_never_compresses`.

---

## 11. Lean

New module `proof/QCore/SocketInetAddresses.lean`, registered in
`proof/QCore.lean` and audited by `proof/AuditSocketInetAddresses.lean`
(placeholder-free; every closure is `propext` / `Classical.choice` / `Quot.sound`
only, confirmed by `AuditAll`). Lean audits go from **107** to **108**.

| Group | Theorems |
|---|---|
| Address admission is total | `every_address_has_exactly_one_family`, `the_two_families_are_disjoint`, `an_implicit_bind_is_ipv4_loopback_on_a_kernel_chosen_port` |
| Resolution refuses outward, never inward | `an_ephemeral_address_reaches_nothing`, `no_bind_reaches_nothing`, `reaching_names_a_bind_of_this_program`, `a_declared_bind_is_reachable_by_its_address`, `an_undeclared_address_reaches_nothing`, `an_implicit_program_is_unreachable_by_address` |
| The families never stand in for each other | `a_family_agrees_with_itself`, `the_families_never_stand_in_for_each_other`, `agreement_is_equality` |
| The `IPV6_V6ONLY` decision, as a stated premise per arm | `the_v6only_decision_is_made_explicitly_on_every_arm`, `the_v6only_decision_does_not_vary_by_arm` |
| A missing family is recorded, never replaced | `creation_never_changes_the_family`, `a_missing_ipv6_is_recorded`, `an_unsupported_family_yields_no_socket` |
| A collision inside one program is decided | `two_ephemeral_binds_never_collide`, `a_repeated_declared_address_collides` |

Two are worth naming. `creation_never_changes_the_family` is the no-fallback
theorem: creation never yields a socket in a family other than the one asked for.
`an_implicit_program_is_unreachable_by_address` is why the card is safe for
everything that came before: a program that declares only implicit locals declares
only ephemeral addresses, and an ephemeral address is not a name, so **no address
can reach any of its sockets**.

**What is not proved.** That the Rust reader extracts an address faithfully from
source tokens; that a platform binds what the emitted program asks it to; or
anything about a failed primitive — the emitted program exits there. Partial
correctness only, stated rather than proved away.

---

## 12. Standing gate

Run at the card tip, Linux, with `$HOME/.elan/bin` on `PATH`.

| Step | Result |
|---|---|
| `git submodule update --init --recursive` | clean (needed once in this worktree; the first `cargo test --workspace` failed `libevent_submodule_matches_the_audited_revision` before it, which is invariant 5 working as designed and not a finding about this card) |
| `cargo test --workspace` | **4,136 passed, 0 failed, 8 ignored** across 205 test targets |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit **0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit **0** |
| `python3 scripts/check_lean_audits.py` | **"Passed 108 Lean audits"** (was 107; this card adds `AuditSocketInetAddresses.lean`) |

Constraint 16's five load-sensitive provider/host rows all **passed in the full
run**, so there was nothing to re-verify in isolation. This card touches no
provider, host or package file, so none of them is reachable from it and constraint 18 does not apply —
no manifest template and no `qcore-schemas` version changed. The `providers/`
workspace was not rebuilt.

**Corpus neutrality.** No gateway, contract, audit or provider file is touched;
the pinned 185-unit corpus, its leak count (266) and the contract corpus (73
modules) are unread and unmoved by this card.

---

## 13. Named follow-ons

| Name | Missing capability |
|---|---|
| `SOCKET-DNS` | resolving a hostname. Out of scope **by construction**: a resolved name is a value the checker cannot compute, and everything this route puts on the wire is decided before emission |
| `SOCKET-EXTERNAL-PEER` | reaching an endpoint outside the program. **Unchanged** by this card — a declared address is resolved against this program's own binds |
| `SOCKET-INET-V6-EMBEDDED-V4` | the `::ffff:1.2.3.4` IPv6 form. It presumes the dual-stack behaviour `IPV6_V6ONLY` excludes |
| `SOCKET-INET-V6-ZONE` | a zone identifier. A zone names an interface of the running host |
| `SOCKET-INET-FIXED-PORT-COLLISION` | a fixed port cannot be collision-free by construction. Two concurrent runs of the whole suite on one host would contend for 49301/49302; the run fails loudly rather than silently |
| `SOCKET-INET-WINDOWS-RUN` | native `cl.exe` execution and byte comparison of the two declared-address examples. The rows are **written and wired** through `tests/support/windows_c.rs::run_msvc`; the discharge condition is those two test names passing |
| `SOCKET-INET-APPLE-RUN` | the same for the Apple arm |
| `SOCKET-INET-BIND-ANY` | `0.0.0.0` and `::` parse, bind and are refused nothing — but no published example binds a wildcard, because this route's peers are all in-program and a wildcard bind buys such a program nothing. `IpAddress::is_unspecified` exists for the card that wants one |
| `SOCKET-INET-SOURCE-ADDRESS` | binding a *connector*'s local address before connecting. Admitted by the grammar today (`bind C ADDR:0; connect C …;`), but unexercised and unpublished |
| `SOCKET-INET-V6-SCOPE` | link-local addressing generally. `fe80::…` parses, but reaching a link-local peer needs the zone this card refuses |

## 14. Files

| Path | What |
|---|---|
| `crates/qcore-core/src/socket_address.rs` | **new** — families, addresses, ports, endpoints, the parser and its refusal vocabulary, 11 unit tests |
| `crates/qcore-core/src/std_socket.rs` | the statement shapes, `resolve_addresses`, the refusal mapping, the two new certificate fields and their recheck |
| `crates/qcore-core/src/std_socket_c_emit.rs` | family-aware creation, declared bind/connect/sendto, the IPv6-unavailable status |
| `crates/qcore-core/src/qcore_socket_compat.h` | **+326 / −9**, the nine all inside the opening comment block — the declared-address arms, `IPV6_V6ONLY`, the family probe, the per-arm `SO_REUSEADDR` decision |
| `crates/qcore-frontend/src/source_socket.rs` | `token_text`, `address_run`, `lower_address`, `address_host_span`, and the three statement readers |
| `crates/qcore-frontend/src/frontend.rs` | the one name-resolution exclusion (§2.3), 9 added lines |
| `compiler/tests/std_socket_inet.rs` | **new** — the 36 tests of §10 |
| `examples/sockets/inet_v4_echo.qcore`, `examples/sockets/inet_v6_echo.qcore`, `examples/sockets/README.md` | **new** examples and their verified transcripts |
| `proof/QCore/SocketInetAddresses.lean`, `proof/AuditSocketInetAddresses.lean`, `proof/QCore.lean` | §11 |
| `docs/QCORE_SPEC_r0_3.md` §L21 | the `Q080`–`Q089` row's second amendment |
| `AGENTS.md` | the constraint-14 census record, the namespace-table row, and standing constraint 15i |
