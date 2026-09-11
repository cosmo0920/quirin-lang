# HTTP-QUIRIN-V1 — an executed HTTP/1.1 subset in pure Quirin, with the boundary named

**Card:** `HTTP-QUIRIN-V1` (wave 24, phase 3 — the wave's milestone head).
**Route:** `source.http.v1`, behind the `std.http` bundled module.
**Band:** `Q090`–`Q096`, seven codes contiguous from `Q090` of the pre-assigned
`Q090`–`Q099`; `Q097`–`Q099` returned to the pool.

User-directed, in two statements: *"The first wave of HTTP implementation should
implement HTTP/1.1 spec so it can achieve with only quirin qcore sources and the
latter C11 emitter"* and *"HTTP quirin v1 should be executed instead of HTTP
quoted something."*

**What shipped.** A checked, **executed** HTTP/1.1 subset: a source surface that
declares requests and responses, has their exact wire bytes constructed and
checked against RFC 9112 at compile time, and moves them over
`source.socket.v1`'s loopback transport — which it reuses unedited. Two published
programs under `examples/http/`, both of which **run** under ASan/UBSan/LSan with
their output byte-compared against the compiler's own reference execution. Zero
external C libraries: no extern declaration, no manifest, no linked artifact.

**The claim vocabulary, fixed here and used everywhere:** *an executed HTTP/1.1
subset with the boundary named.* Never "HTTP/1.1 conformance". §5 is the subset
table and §11 the named follow-ons.

**Status language.** This card publishes no verdict about any pinned corpus unit
and moves no corpus number. Corpus-neutral by construction (§12).

---

## 1. The problem, and the move that solves it

An HTTP implementation in a language with no computed text looks impossible at
first reading, and the design record
`docs/design/STD_HTTP_BINDING_V1.md` §2(a) said so: it listed a string tier,
loops over buffers, bulk operations, growth, recoverable allocation failure,
sub-ranges and concurrency at the boundary as prerequisites, and recommended
binding a C library instead.

**That list is the prerequisite list for *parsing* HTTP, not for *constructing
and checking* it**, and the distinction is the whole card:

> Every component of a declared message is a quoted literal, so its bytes — and
> therefore its **length** — are known at check time. A protocol rule about those
> bytes is then a rule a compiler can decide.

That turns the missing capability into an asset. A declared `Content-Length` that
is not the content's byte count is a **compile-time refusal** here — and it is a
refusal a contracted binding to a C library could not offer at all, because the
library computes the length at run time and a contract's only available posture
toward it is a premise. The design doc's §0 now records the decision and what it
changed about §2(a); its recommendation and grounds are left unedited, because a
decision record that rewrote its own reasoning after the fact would be worthless.

---

## 2. The three things this card actually had to build

### 2.1 The transport composition: `SOCKET-STRING-PAYLOAD`, landed

The socket card's named follow-on. A **string view as a transfer payload**,
structurally rather than as a second operand form:

```rust
pub enum SocketPayload { Bytes(Vec<SocketOperand>), Text(TextPayload) }
pub struct TextPayload { pub symbol: Option<String>, pub bytes: Vec<u8> }
```

A `SocketOperand` denotes one byte; a view denotes an **extent**, which is what
`source.string.v1`'s `{ptr, len}` is for, so the payload is the extent read at
the transfer site. The bytes are the view's exact decoded UTF-8 payload — no
terminator, no re-encoding, no escaping — which is amendment §7's observation
rule applied to a wire instead of to a stream.

**No custody crosses**, and that is a consequence rather than a decision: a
string is a custody-free view of storage it does not own (amendment §3.6,
"creating a view neither creates nor discharges that custody"), so a text payload
adds no ownership operation. The socket keeps every custody rule it had; the view
contributes none, because it has none.

**`\r` is in the escape set** (`decode_string_literal`: `b'r' => 13`), verified
rather than assumed, so a CRLF is spellable and
`send C "GET / HTTP/1.1\r\nHost: h\r\n\r\n";` is a program.

**The receive half is the interesting one.** `recv S "…";` declares the bytes it
takes, and the model *already knows* which bytes a receive takes — they are the
bytes an earlier send of this same program committed. So the declaration is
compared against that answer **at check time**, and a disagreement is a refusal
(`Q084`, `SocketError::PayloadDisagrees`). The emitted program still compares
what it actually read against the certificate's receipt, so a declared payload is
checked twice by two different means and neither is trusted.

The `symbol` field is what makes a composed program honest in the emitted C:
`None` emits the bytes inline at the site (the bare socket route's form),
`Some(name)` names a file-scope object a composing route defined once. That is
why `send client rq;` and `recv server rq;` both name `qcore_http_rq` — "the
server verifies exactly what the client sent" is visible in the generated file
rather than something a reader must check by comparing two arrays.

### 2.2 The HTTP vocabulary, and the conformance facts it decides

```qcore
let rq = request "GET" "/index.html" "HTTP/1.1";
header rq "Host" "127.0.0.1";
let rs = response "HTTP/1.1" 200 "OK";
header rs "Content-Length" "6";
content rs "hello\n";
send client rq;   recv server rq;
send server rs;   recv client rs;
```

**The version is written and checked, never supplied.** A reader of an admitted
program can see which protocol it speaks, and `HTTP/1.0` refuses by name rather
than being silently upgraded to the one version this route models. The same
principle runs through the surface: nothing the compiler could have filled in for
the author is filled in, because a checked value the author did not write is a
value the author cannot be refused for.

### 2.3 The framing, which is constructed and not spelled

No source program writes a `CRLF`. `head_bytes` writes them — one after the start
line, one after each field, one to end the head — and **no admitted component may
carry a `CR` or an `LF`** (`Q092`). Therefore every CRLF in an admitted message
head is one the compiler put there, and **header injection is not expressible on
this route** rather than merely not intended. That is the card's other signature
property, and it is the one with real security content.

---

## 3. Every static conformance check, with its refusing test

All in `compiler/tests/std_http.rs`. **67 tests, all passing.**

| # | The RFC fact | Code | Refusing test |
|---|---|---|---|
| 1 | the version is exactly `HTTP/1.1` (RFC 9112 §2.3) | `Q091` | `a_version_that_is_not_http_1_1_refuses` |
| 2 | a method is a `token` (RFC 9110 §5.6.2) | `Q091` | `a_method_that_is_not_a_token_refuses` |
| 3 | the method token is **case-sensitive** (RFC 9110 §9.1), so `get` is not `GET` | `Q091` | `a_registered_method_in_the_wrong_case_refuses` |
| 4 | a request target is origin-form (RFC 9112 §3.2.1) | `Q091` | `a_request_target_that_is_not_origin_form_refuses` (4 shapes) |
| 5 | a status code is three digits in `100..=599` (RFC 9112 §4) | `Q091` | `a_status_code_outside_the_registered_range_refuses` (3 values) |
| 6 | a reason phrase carries no framing or control byte | `Q091` | `a_reason_phrase_carrying_a_framing_byte_refuses` |
| 7 | a field name is a `token` | `Q092` | `a_field_name_that_is_not_a_token_refuses` |
| 8 | **no component carries `CR` or `LF`** — the injection wall | `Q092` | `a_field_value_carrying_a_framing_byte_refuses` (3 shapes, incl. `a\r\nX-Injected: 1`) |
| 9 | a field value is ASCII field-content; `obs-text` refused (stricter than RFC 9110 §5.5, stated) | `Q092` | `a_non_ascii_field_value_refuses` |
| 10 | a field value has no leading/trailing `OWS` (RFC 9110 §5.5 strips it, so v1 refuses rather than dropping bytes) | `Q092` | `a_field_value_with_surrounding_whitespace_refuses` (3 shapes) |
| 11 | **a request carries exactly one `Host`** (RFC 9112 §3.2) | `Q092` | `a_request_without_a_host_field_refuses`, `a_request_with_two_host_fields_refuses` |
| 12 | …and a response needs none, because §3.2 is about requests | — | `a_response_needs_no_host_field` (an *admission*) |
| 13 | a `Connection` option v1's transport honours | `Q092` | `a_connection_option_v1_does_not_honour_refuses` |
| 14 | **a declared `Content-Length` equals the content's byte count** | `Q093` | `a_content_length_that_is_not_the_content_length_refuses` (3 pairs) |
| 15 | …and the matching one is admitted, zero-byte content included | — | `the_matching_content_length_is_admitted` |
| 16 | content with no declared length is unframed (RFC 9112 §6.3) | `Q093` | `content_without_a_declared_length_refuses` |
| 17 | a declared length on a message with no content must be `0` | `Q093` | `a_declared_length_on_a_message_with_no_content_must_be_zero` |
| 18 | no transfer coding is modelled | `Q093` | `a_transfer_coding_refuses` |
| 19 | a duplicate `Content-Length` (RFC 9112 §6.3) | `Q093` | `two_content_length_fields_refuse` |
| 20 | a `Content-Length` is `1*DIGIT` | `Q093` | `a_content_length_that_is_not_decimal_refuses` (4 spellings) |
| 21 | a message is complete before it is transferred | `Q090` | `a_header_after_the_message_was_sent_refuses`, `content_set_twice_refuses` |
| 22 | a message declared and **never transferred** is checked exactly as hard | `Q091`+`Q092` | `a_declared_message_never_transferred_is_checked_exactly_as_hard` |

Plus the framing shape itself, checked against a real certificate rather than a
model alone:
`the_wire_bytes_are_the_head_then_the_content_and_the_head_ends_in_an_empty_line`
asserts the wire is head-then-content, the head ends `\r\n\r\n`, and the head
carries exactly `1 + fields + 1` CRLFs.

### 3.1 Two refusals that are v1 narrowings, named as such

Both refuse programs RFC 9112 would admit, which is always safe (invariant 1) and
is stated rather than hidden:

* **`HTTP-V1-METHOD-CASE`** (#3). `token` admits `get`; RFC 9110 §9.1 makes the
  method case-sensitive, so `get` is a method no origin server implements. v1
  refuses the eight common spellings in the wrong case and names the repair,
  rather than emitting a request nothing will answer.
* **`HTTP-V1-OBS-TEXT`** (#9). RFC 9110 §5.5 admits `%x80-FF` as opaque
  `obs-text`. A route whose entire evidence is a byte comparison declines to put
  bytes on a wire whose meaning it cannot state.

---

## 4. The architecture, and the first real use of `merge`

### 4.1 The check is a composition, not a second checker

`check_http_program` has two halves and the second is not this card's:

1. **`lower_transport`** — validate every message against RFC 9112, construct its
   wire bytes, and rewrite each statement into `source.socket.v1`'s own. This is
   the **whole** of what the HTTP vocabulary does.
2. **`std_socket::check_socket_program`** over the result, **unedited**. Custody,
   the transport state machine, the determinism boundary and the declared effect
   row are all that checker's answers, re-attributed to the HTTP statement a
   reader wrote through an `attribution` index map.

`Q095` therefore carries `SocketError` itself rather than a re-worded copy — the
same evidence shape `SocketError::Custody` uses for `OwnershipError`, one level
up. `a_leak_refuses_and_the_refusal_comes_from_the_inherited_socket_checker`
demonstrates it arriving that way.

The one socket error mapped differently is `EffectNotDeclared`, which becomes
`Q096`: it is about a row a reader of *this* route wrote.

### 4.2 The lowering reads socket statements with the socket route's own reader

This card refactored `source_socket.rs` to publish
`socket_statement(parsed, window, span) -> SocketLowering`, the **one place** that
route's statement grammar lives, and `lower_source_socket` now calls it. The HTTP
lowering calls the same function for every statement it does not own, so a
transport statement this composition admits is byte for byte the statement the
socket route would have produced, and **a shape one accepts and the other refuses
is not expressible**. That is the discipline `source_std_io::std_io_statement`
established and the socket route already followed for its own line statements.

Refusals come back as `(code, message)` pairs carrying no span and no pass: the
caller owns attribution, so the HTTP route re-attributes them to `Q090` without
the socket grammar having to know who is reading it.

### 4.3 Emission: `C-EMIT-COMPOSED-VIA-MERGE`, used as designed

Card `C-EMIT-MAIN-UNIFY` built the fragment/entry architecture and named the
follow-on `C-EMIT-COMPOSED-VIA-MERGE`, cautioning that converting the *existing*
composed route to `merge` would move pinned bytes. **This route is new — no
pinned bytes exist for it — so it uses the mechanism as designed, and is the
first consumer to do so.** Nothing was forked.

| Contributor | Fragments |
|---|---|
| `std_http_c_emit::preamble(cert)` | `http.banner` (Banner) · one `http.message.<binding>` (FileScope) per **transferred** message |
| `std_socket_c_emit::preamble()` | `socket.compat` (Compat) · `socket.includes` (Includes) · `qcore_io.prelude` (Prelude) |

`c_program::merge(&[&socket, &http])` concatenates, deduplicates and orders by
section; `std_socket_c_emit::entry_parts` (new, published by this card) supplies
the one entry's prologue and body; `c_program::assemble` writes the single entry
point. Measured:

* `the_composed_program_has_exactly_one_entry_point` — `entry_definitions` = 1;
* `the_merge_gives_the_composed_program_one_copy_of_the_shared_prelude` — one
  `#define QCORE_IO_WRITE(`, one `fprintf(`, and the compatibility header above
  every include, which is what `merge`'s section ordering is *for* (the HTTP
  route contributes no include of its own, so insertion order would not have
  placed it);
* `the_emitted_program_routes_every_line_through_the_single_io_route` —
  `direct_stdio_calls` empty.

**A declared message no statement transfers contributes no fragment.** It is
checked exactly as hard, at the end of the program; emitting an unused file-scope
array would not survive `-Wall -Wextra -Werror`. The `transferred` flag on
`HttpMessageRecord` carries that decision where a reader can see it.

The archived emitted C for `exchange.qcore` is
`artifacts/exchange.c` (494 lines, one `int main(`).

### 4.4 The `driver.rs` seam

Additive and confined: one `CheckedSourceModule` variant (appended to eleven
existing multi-variant `=> None` arms), one `CheckedHttpModule` struct, one
`as_standard_http` accessor, one `function_count` arm, one
`SourceExecutionProfile` variant, one route-selection `if` block, and one
`compile_http_source` modelled line for line on `compile_socket_source`.

**Route ordering, argued.** Selected after the buffer route and **before both
routes it composes**:

* after buffer, so a module that buffers and does HTTP keeps the buffer route's
  refusal (`HTTP-V1-BUFFER-CONTENT`) — measured by
  `a_module_that_buffers_and_does_http_stays_with_the_buffer_route`;
* before string, so a module that declares a message and binds a payload reaches
  this route, which models the binding; a module that only slices falls through
  byte for byte;
* before socket, so a module that declares a message and sends it reaches this
  route; a module that only sockets falls through — measured by
  `a_module_that_only_sockets_stays_with_the_socket_route`, which is the
  published `raw_request.qcore`;
* after event and extern and every concurrent route, so those keep their
  refusals (`HTTP-V1-EVENT-COMPOSE`, `Q600`).

---

## 5. The subset table: what of RFC 9112 is in, and what is out

| RFC 9112 feature | In v1? | Why / who owns it |
|---|---|---|
| request-line (§3): method, request-target, version | **in** | origin-form targets only |
| status-line (§4): version, status code, reason phrase | **in** | `100..=599` |
| header fields (RFC 9110 §5) | **in** | ASCII field-content; `obs-text` out (`HTTP-V1-OBS-TEXT`) |
| CRLF framing and the empty line ending the head (§2.1) | **in** | constructed, not spelled — §2.3 |
| `Content-Length` framing (§6.2) | **in** | and **checked against the content**, §3 #14 |
| a fixed literal content | **in** | a literal only |
| the required `Host` field (§3.2) | **in** | exactly one, §3 #11 |
| `Connection: close` (§9.6) | **in** | any other option refused |
| a **computed** content or header value | **out** | `STRING-SLICE-CONCAT` — v1 has no computed text at all |
| parsing a message out of unknown input | **out** | `HTTP-V1-PARSE` — needs a varying-value tier no route has |
| chunked transfer coding (§7.1) | **out** | `HTTP-V1-CHUNKED`; any `Transfer-Encoding` refuses |
| persistent connections, pipelining (§9.3) | **out** | `HTTP-V1-PERSISTENT` — the transport closes after one exchange |
| `100 (Continue)` (§9.5 / RFC 9110 §10.1.1) | **out** | `HTTP-V1-EXPECT-CONTINUE` |
| an external peer | **out** | `SOCKET-EXTERNAL-PEER` — a determinism change |
| TLS | **out** | `HTTP-V1-TLS` |
| absolute-form, authority-form, asterisk-form targets (§3.2) | **out** | `HTTP-V1-TARGET-FORMS` |
| trailers (§7.1.2) | **out** | there is no chunked coding to carry them |
| a message over 1024 bytes | **out** | `HTTP-V1-MESSAGE-BOUND` — the transport bound |
| upgrade, `CONNECT` tunnelling | **out** | not modelled, unowned |

---

## 6. Determinism (standing constraint 15c)

Constraint 15c asks a route to say **why** every record of its observation is
written by the entry's thread. The answer here is inherited whole and is short:

> A `source.http.v1` program has **one thread of control and creates none.** It
> spawns no worker, registers no callback and runs no reactor. The HTTP
> vocabulary adds no statement that waits. The only statements that write bytes
> to a stream are `source.std-io.v1`'s own `print` and `log`, executed in the
> entry's own body in program order.

Therefore every record's emitter is `Emitter::Parent`, no worker is ever live,
the `OBS-REORDERING` live-set derivation puts each record alone in its own window
— making it a barrier — and the declared `qcore.obs-reordering.v1` relation is
`is_strict()`. The byte comparison the native tests perform is that relation's
own degenerate case (`strict_iff_byte_equality`), not a convention beside it.
`the_declared_reordering_relation_is_strict` asserts exactly this, on both
streams, and additionally checks that comparing against the relation **is** byte
equality.

**HTTP traffic writes no record at all.** A request on a loopback stream is not
an observation: it reaches no stream the observation is read from. That is the
boundary `COMPOSE-EVENT-LINES` drew for a drained wake byte, and
`QUIRIN-SOCKET-V1` drew for a payload.

---

## 7. The executed evidence

Two published programs, `examples/http/`, both admitted, emitted, compiled under
`-std=c11 -Wall -Wextra -Werror -pedantic` and run under ASan/UBSan/LSan:

| Program | Route | Shape | Transcript |
|---|---|---|---|
| `exchange.qcore` | `source.http.v1` | one entry, loopback client+server pair, declared request and response, both verified on receipt, three closes | `200\n6\n` |
| `raw_request.qcore` | `source.socket.v1` | the same kind of bytes written by hand as string payloads | `204\n` |

**Byte-comparison result: identical.** `native::the_exchange_example_runs_sanitized_and_writes_the_reference_observation`
and its `raw` sibling compare the native run's stdout with the reference
observation's stdout projection, byte for byte, and both pass.
`native::two_runs_of_one_program_write_identical_bytes` passes, so no descriptor
number, port or timing reaches the observation. The README's transcripts **and
its published wire bytes** are read out of the file and compared with what the
pipeline produces (`the_readme_transcript_is_the_real_one`,
`the_readme_wire_transcript_is_the_real_one`), so the page cannot drift.

**The contrast is the point of publishing two.** `raw_request.qcore` puts the
same kind of bytes on the wire and the compiler checks only that the receive is
satisfiable and that the declared bytes are the committed ones. It does not know
the payload is HTTP: no `Host` requirement, no `Content-Length` relation, no
notice of a CRLF in the wrong place. A payload a program *writes* is bytes; a
message a program *declares* is checked.

**v1 renders scalars.** A rendered text line inside this route's transport body
would need `source.string.v1`'s own text emission there — `io_emit::TEXT_PRELUDE`
and a `StdIoEvent`/`SocketStatement` widening that would reach every route
consuming those enums, which is precisely the widening `STRING-SLICE-IO` §5
declined to make. Named `HTTP-V1-TEXT-TRANSCRIPT` rather than forced.

---

## 8. Per-arm platform honesty

The transport shim is `QUIRIN-SOCKET-V1`'s, unchanged, and so are its arms.

| Arm | This card's claim |
|---|---|
| `_WIN32` | **compile-path-correct only.** Residual **`HTTP-V1-WINDOWS-RUN`** |
| `__APPLE__` | **compile-path-correct only.** Residual **`HTTP-V1-APPLE-RUN`** |
| else (Linux, other POSIX) | **runtime-verified**: emitted C built under `-Werror`-class flags, run under ASan/UBSan/LSan, stdout byte-compared with the reference observation |

The native tests are confined by `#[cfg(any(target_os = "linux", target_os = "macos"))]`
exactly as `std_socket.rs`'s are, and the macOS branch of the ASan options is
present and correct (LSan is Linux-only) but was not exercised on this card's
hardware. The emitted C carries no Windows-specific text of this route's own: the
message arrays are `static const unsigned char`, and `ws2_32.lib` is named from
inside the socket shim as before. This is the `SOCKET-V1-WINDOWS-RUN` /
`TRACE-STDIO-WIN` precedent.

---

## 9. Keyword census (standing constraint 14): **four words, and one rejection that matters**

Ran over **416** `.qcore`/`.qc` fixtures and **43,239** Rust string literals, at
whole-word granularity with `//` comments stripped from the QCore side.
Reproduction script and output: `artifacts/keyword_census.sh`,
`artifacts/keyword_census.txt`.

| Candidate | QCore raw | QCore **source** | Rust strings | Verdict |
|---|---|---|---|---|
| `request` | 0 | **0** | 43 | **reserved** |
| `response` | 1 | **0** | 7 | **reserved** |
| `header` | 8 | **0** | 93 | **reserved** |
| `content` | 3 | **0** | 5 | **reserved** |
| `http`, `method`, `trailer` | 0 | 0 | 0–0 | clean, not needed |
| `version`, `status`, `reason`, `target` | 19/9/4/3 | **0** | — | clean, not needed |
| **`body`** | 93 | **8** | 306 | **REJECTED** |
| `message` | 2 | **2** | 57 | **rejected** |
| `payload` | 56 | **49** | 114 | **rejected** |
| `field`, `head` | 24/19 | 5/6 | — | rejected |

**`body` is the finding worth recording.** It is the obvious word for a message
body, and it is a *binding name* in two shipped fixtures —
`examples/strings/status_line.qcore` and
`compiler/tests/fixtures/std_string/text.qcore` — referenced by `print body;`,
`body |> print;` and `len(body)`. Reserving it would have refused programs that
ship today. RFC 9110 §6.4 renamed that concept **content** ("the content of a
message"), so the word the census admits is also the specification's current
one — the census and the standard agreed, which is the pleasant case rather than
the usual one. This is the same measured judgement `STD-BUFFER` made about
`buffer` and `QUIRIN-SOCKET-V1` made about `stream`.

`the_body_spelling_is_not_this_cards_word` compiles a module that binds `body`,
prints it and measures it — the test that would fail if a later card reserved it.

`send`, `recv`, `let`, `close` and the standard-IO line words are **reused rather
than respelled**, which is why this card adds four words and not six. The
`EXTERN-MODES` discipline applied where it applies.

---

## 10. The import-composition rule, decided and argued (constraint 13)

The card asked for the exact rule to be decided per the `Q014`/`Q015` discipline.
`std.http` grants the message vocabulary and **nothing else**, in three separate
statements each with its own test:

1. **It grants no transport.** A message is a value; putting one on a wire is
   `std.socket`'s `send`/`recv`. A program that actually speaks HTTP names both.
   `the_http_import_alone_does_not_grant_a_socket`,
   `the_socket_import_alone_does_not_grant_a_message`.
2. **It grants no text — and this is the one that had to be argued.** A quoted
   token at a method, a target, a field name, a field value or a content position
   is *this module's own syntax* and constructs no `string` value: nothing in the
   language can bind it, slice it, measure it or render it.
   `STRING_SLICE_V1.md` §2 excludes exactly that case, in as many words —
   *"Recognizing quoted tokens for structural syntax or existing declaration
   metadata … does not construct a string value and MUST NOT acquire this new
   import requirement."* So a pure HTTP program imports three modules, not four:
   `a_declared_message_needs_no_string_module`, which asserts the source does not
   even contain the string `std.string`.
3. **And the converse holds.** `let s = "…";` **does** construct a string value a
   later statement names, so it takes the string module's own gate exactly as a
   rendered literal does: `a_bound_view_needs_the_string_module` pins both
   directions. The same rule gates a socket text payload
   (`a_text_payload_needs_the_string_module`), while a byte-list payload still
   needs nothing new (`a_byte_list_payload_still_needs_no_string_module`).

Cross-file isolation is unchanged and pinned
(`importing_the_module_in_another_file_does_not_grant_this_one_the_capability`).

The rule is stated at the registry in Lean too — `admitsHttp` beside
`admitsSocket` and `admitsString`, with independence proved in both directions.

---

## 11. Named follow-ons

| Name | What is missing, and its recorded blocker |
|---|---|
| `STRING-SLICE-CONCAT` | *(another card's, inherited)* a `Content-Length` of a **computed** value. v1 has no computed text at all; it needs a building operation, which needs storage a program owns — `STRING-SLICE-BUFFER-BORROW`'s question first. Named by `STRING-SLICE-IO` as this card's next obstacle, and it was |
| `HTTP-V1-PARSE` | reading a message out of **unknown** input. Received bytes are verified against a declared expectation; parsing needs a loop over an index the checker cannot fix, which is the varying-value tier no route in this language has |
| `HTTP-V1-CHUNKED` | chunked transfer coding and trailers. Any `Transfer-Encoding` refuses at `Q093` |
| `HTTP-V1-PERSISTENT` | persistent connections and pipelining. The emitted exchange closes after one message pair, so any `Connection` option but `close` would be a claim the transport does not honour |
| `HTTP-V1-EXPECT-CONTINUE` | `100 (Continue)`: a two-response exchange whose second depends on the first, which the declared-transfer model does not express |
| `SOCKET-EXTERNAL-PEER` | *(the socket card's, unchanged)* a peer outside this program. A determinism change, not a syntax change |
| `HTTP-V1-TLS` | transport security. No cryptography and no library to borrow one from |
| `HTTP-V1-TARGET-FORMS` | absolute-form, authority-form and asterisk-form request targets (RFC 9112 §3.2) |
| `HTTP-V1-BUFFER-CONTENT` | a content that is a `std.buffer` extent. A module that buffers stays with the buffer route |
| `HTTP-V1-OBS-TEXT` | `obs-text` (`%x80-FF`) in a field value. A stated v1 narrowing (§3.1) |
| `HTTP-V1-METHOD-CASE` | admitting a `token` method whose spelling is a registered method in another case. The other stated narrowing (§3.1) |
| `HTTP-V1-MESSAGE-BOUND` | a message over 1024 bytes. The transport bounds one transfer so a send provably never blocks |
| `HTTP-V1-STRING-COMPOSE` | `len`, a slice or a rendered text line beside HTTP. This route models a bound view as a payload and nothing else of `source.string.v1` |
| `HTTP-V1-TEXT-TRANSCRIPT` | a rendered **text** line in this route's transport body. Needs `io_emit::TEXT_PRELUDE` inside the socket backend's entry, which needs a `StdIoEvent`/`SocketStatement` widening reaching every consumer — the widening `STRING-SLICE-IO` §5 declined |
| `HTTP-V1-EVENT-COMPOSE` | composition with `source.event.v1`. A module that dispatches stays with the event route |
| `HTTP-V1-REDECLARE-REACH` | `Q090`'s redeclaration arm is unreachable from a single source module — name resolution refuses a duplicate binding at `Q012` first. Kept as a fail-closed guard on the model, on the `EXTERN-F1-STORE-REACH` precedent, and pinned by `a_message_declared_twice_refuses` |
| `STD-FMT-DIAGNOSTIC` | *(unchanged, another card's)* an operand naming **nothing** refuses generically at `Q013` before this route runs. Correct; only its specificity is the gap. Both directions pinned (`an_operand_naming_nothing_at_all_keeps_the_generic_refusal`) |
| `SOCKET-TEXT-BINDING` | on the **bare** socket route a text payload must be a literal written at the site; a `let`-bound view there is claimed by the string route. The HTTP route models the bound form, so the gap is the socket route's alone |
| `HTTP-V1-EXAMPLE-SUITE` | `examples/http/` is verified by this card's own test file rather than by `compiler/tests/examples_suite.rs`, whose list is explicit rather than a directory scan. The same gap `examples/sockets/` and `examples/strings/` have (`STRING-SLICE-IO-EXAMPLE-SUITE`); folding all three in is mechanical and was kept out of this card's blast radius |
| `HTTP-V1-WINDOWS-RUN`, `HTTP-V1-APPLE-RUN` | a native run, sanitizer or byte-comparison claim for those arms (§8) |
| `SOCKET-STRING-PAYLOAD` | **retired by this card** |
| `C-EMIT-COMPOSED-VIA-MERGE` | *not* retired: this card is `merge`'s first consumer, but `composed_std_io_c_emit` still spells its own list inline and converting it still moves bytes |
| `STD-HTTP-NATIVE-ALTERNATIVE` | *inverted*: strategy (a) is now the implemented route and a contracted binding is the open alternative (design doc §0) |

---

## 12. Corpus neutrality

By construction. The diff touches `crates/qcore-core` (5 files), `crates/qcore-frontend`
(4), `crates/qcore-verification/src/driver.rs`, three documents, seven Lean files,
one test file, one example directory and this report.
`crates/qcore-c-gateway/`, `crates/qcore-audit/`, `contract/`, `providers/`,
`examples/threads` and every frozen archive appear **nowhere** in it. The pinned
185-unit artifacts, the leak count of **272** (constraint 7), the 72-module
contract pin and the CT manifest are main's, untouched and unread.

---

## 13. Evidence

| Artefact | Where |
|---|---|
| Checked model | `crates/qcore-core/src/std_http.rs` (`HttpMessage`, `admit_start`, `admit_field`, `admit_framing`, `head_bytes`, `lower_transport`, `Q090`–`Q096`) |
| Emission | `crates/qcore-core/src/std_http_c_emit.rs` (the `merge` composition, the message fragments) |
| Transport extension | `crates/qcore-core/src/std_socket.rs` (`SocketPayload`, `TextPayload`, `PayloadDisagrees`), `crates/qcore-core/src/std_socket_c_emit.rs` (`entry_parts`, named payload references) |
| Source lowering | `crates/qcore-frontend/src/source_http.rs`; `crates/qcore-frontend/src/source_socket.rs` (`socket_statement`, `SocketLowering`, the text payload forms) |
| Module registry | `crates/qcore-core/src/module_imports.rs`, `crates/qcore-frontend/src/module_imports.rs` |
| Pass and keywords | `crates/qcore-frontend/src/frontend.rs` (`SemanticPass::StandardHttp`, the `Q090` presentation code, the four reserved words) |
| Driver seam | `crates/qcore-verification/src/driver.rs` (§4.4) |
| Tests | `compiler/tests/std_http.rs` (**67**, three of them sanitized native runs) |
| Examples | `examples/http/{exchange,raw_request}.qcore`, `examples/http/README.md`, indexed in `examples/README.md` |
| Lean | `proof/QCore/StdHttp.lean`, `proof/AuditStdHttp.lean`, `proof/QCore/ModuleImports.lean` (nine modules → ten) |
| Spec | `docs/QCORE_SPEC_r0_3.md` §L21 row `Q090`–`Q096` |
| Grammar | `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` §G07.10 |
| Design record | `docs/design/STD_HTTP_BINDING_V1.md` §0 (the decision, and what it changed about §2(a)) |
| Artifacts | `artifacts/keyword_census.{sh,txt}`, `artifacts/exchange.c` |

### 13.1 What the Lean proves, and what it does not

`QCore.StdHttp`, registered in `proof/QCore.lean` and audited by
`proof/AuditStdHttp.lean`. Placeholder-free; every closure is `propext` /
`Classical.choice` / `Quot.sound` only.

| Group | Theorems |
|---|---|
| The injection wall | `no_framing_byte_is_a_field_byte`, `no_framing_byte_is_a_tchar`, `an_admitted_component_carries_no_framing_byte`, `an_admitted_token_carries_no_framing_byte`, `an_admitted_field_carries_no_framing_byte` |
| The message grammar | `the_version_is_exactly_http_1_1`, `a_status_outside_the_registered_range_is_refused` |
| Framing invariants | `the_wire_is_the_head_then_the_content`, `a_message_with_no_content_is_its_head`, `the_head_is_the_start_line_then_the_fields_then_the_empty_line`, `every_field_line_ends_in_crlf`, `the_field_lines_are_crlf_terminated`, `the_head_ends_in_an_empty_line`, `a_fieldless_head_is_the_start_line_and_an_empty_line`, `one_more_field_adds_exactly_one_line` |
| **The `Content-Length` equality** | `a_declared_length_is_the_content_length` and its converse `a_length_that_is_not_the_content_length_is_refused`; `content_without_a_declared_length_is_refused`, `no_content_and_no_length_is_admitted`, `a_length_on_an_empty_message_must_be_zero` |
| Effects | `no_http_statement_derives_an_effect`, `the_statement_forms_are_exactly_three` |
| Module gate | `the_http_capability_needs_its_module`, `the_http_module_grants_neither_transport_nor_text` |
| Registry (10th module) | twelve new `QCore.ModuleImports` theorems, incl. `a_near_miss_http_spelling_rejects` (pinning that `std.web` and `std.https` are not entries) |

The equality is proved **in both directions** deliberately: one direction says an
admitted declaration *is* the length, the other says a declaration that is not the
length is refused. Only the pair makes the check a gate rather than a description.

`QCore.ModuleImports.Imports` grows from nine fields to ten — the same widening
`QUIRIN-SOCKET-V1` made from eight to nine. Every existing theorem name and
meaning is preserved; the generic `admitted_capability_requires_its_module`
argument is instantiated once more, so the HTTP gate cannot drift from the nine
that precede it.

**What is not proved.** That the Rust lowering extracts these messages faithfully
from source tokens; that the transport delivers them (`QCore.StdSocket`'s
subject, inherited unedited); or that the subset modelled *is* HTTP/1.1 — it is a
bounded subset, and everything outside it is named in §5 and §11 rather than
proved away. Partial correctness only, stated rather than proved away.

---

## 14. Standing gate

Run at the card tip, Linux, with `$HOME/.elan/bin` on `PATH`.

| Step | Result |
|---|---|
| `git submodule update --init --recursive` | run in this fresh worktree — see §14.1 |
| `cargo test --workspace --no-fail-fast` | **3,950 passed, 0 failed, 8 ignored** across 203 result lines, exit **0** |
| `cargo fmt --all -- --check` | exit **0** |
| `cargo clippy --all-targets -- -D warnings` | exit **0** |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit **0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit **0** |
| `python3 scripts/check_lean_audits.py` | **"Passed 103 Lean audits"** (102 → 103: `AuditStdHttp.lean`), no project axiom |
| `python3 -m unittest discover -s scripts/tests` | 193 tests, **1 pre-existing failure** — §15 item 3 |

### 14.1 The first run stopped at the submodule pin

`libevent_submodule_matches_the_audited_revision` failed on the first
`cargo test --workspace` of this fresh worktree, and four
`native_library_header` rows failed for the same missing checkout. That is
invariant 5's pin doing its job on an **environment gap** rather than a code
failure — the same thing cards `STD-EVENT`, `COMPOSE-EVENT-LINES` and
`STRING-SLICE-IO` recorded in their own fresh worktrees — and the gate's own
first line is the repair. The submodule was checked out at
`d82464a277d0f42703702c4dfd9af6af38595a83` and the whole-workspace run redone.

### 14.2 The count, and what it is made of

**3,950 against the base's 3,883** (card `C-EMIT-MAIN-UNIFY` §8's measured
number), and the difference is exactly this card's **67** new end-to-end tests in
`compiler/tests/std_http.rs`: 3,883 + 67 = 3,950. **No existing test was
deleted, rewritten or weakened**, which matters here because this card changed
two enums the socket route's own tests read.

**No `P02-FLAKE` row tripped** (standing constraint 16). Three of the five rows
are in this workspace suite and each passed on the first attempt of the complete
run: `vp04_vp07_vp08_process_lifecycle_and_payload_budgets`,
`a_confined_fixture_cannot_write_outside_the_admitted_paths` and
`memory_growth_beyond_the_declared_limit_is_bounded`. Nothing needed isolated
re-verification. The other two live in the separate `providers/` workspace, which
this card neither touches nor ran: no provider, schema, package template or
protocol changed, so constraint 18's rebuild is not owed and the 122-test
provider checkpoint is **carried, not rerun**.

The Lean leg is read from the **text** line rather than the exit code, per the
script's own behaviour: `scripts/check_lean_audits.py` reports a missing `lake`
and exits 0 when `$HOME/.elan/bin` is not on `PATH`. That is the environment note
cards `STRING-SLICE` and `STRING-SLICE-IO` recorded, and it recurred.

Measurement outputs are worktree-private under `measure/` (gitignored), never
under `target/` and never in the shared scratchpad.

---

## 15. Integrator reconciliation list

This card may not edit `AGENTS.md`, `DEVELOPMENT.md` or
`docs/reviews/r0_3/acceptance_matrix.json`. Five things are the integrator's.

1. **Seven new diagnostic codes.** `AGENTS.md`'s diagnostics-namespace table
   needs a new row (authority is spec §L21, which this card updated):

   > `Q090`–`Q096` | `StandardHttp` | `source.http.v1`: the checked HTTP/1.1
   > message subset — `Q090` shape, `Q091` the start line, `Q092` header fields
   > (including the required `Host` and the `CR`/`LF` exclusion that makes the
   > constructed framing unforgeable), `Q093` framing (of which the
   > `Content-Length`/content-length equality is the card's signature check),
   > `Q094` a name at a message position that is not one, `Q095` the inherited
   > unedited `source.socket.v1` refusal, `Q096` an undeclared derived effect
   > (`HTTP-QUIRIN-V1`)

   `Q097`–`Q099` are **unused and returned to the pool**.

2. **A proposed standing constraint**, offered because it is the kind of rule the
   "when touching X, do not Y" inventory exists for:

   > **15f. An HTTP message's framing is constructed, and its `Content-Length` is
   > decided.** `source.http.v1` writes every CRLF itself — one after the start
   > line, one after each field, one to end the head — and **no admitted
   > component may carry a `CR` or an `LF`**. Do not relax that exclusion: it is
   > what makes "every CRLF in an admitted head is one the compiler wrote" true,
   > and header injection unexpressible rather than merely unintended. A declared
   > `Content-Length` must **equal** the literal content's byte count, decided at
   > check time because a literal's length is known there; do not turn that
   > equality into a premise, and do not admit a content whose length the route
   > cannot compute. The transport half is `source.socket.v1`'s **unedited**
   > checker over a program this route lowers to, and its refusals are
   > re-attributed at `Q095` carrying that checker's own error value — do not
   > re-implement a custody, transport-state or determinism rule here. The
   > composition's emission goes through `c_program::merge` + one
   > `c_program::assemble`; do not fork a whole-program backend. `std.http`
   > grants **no** transport and **no** text: a quoted token at a message
   > component is this module's syntax and constructs no `string` value
   > (`STRING_SLICE_V1.md` §2), while a `let`-bound one does and takes
   > `std.string`'s gate. `http-quirin-v1`.

3. **The derived `lean_audits` count moves 102 → 103** for this card's one new
   `Audit*.lean`, so `scripts/tests`'s derived-value tripwire reports the stale
   matrix count until the derived section is regenerated. The same one-line
   reconciliation every wave's new audit file receives (constraint 25).

   **The drift predates this card by three.** `docs/reviews/r0_3/acceptance_matrix.json`
   carries `lean_audits: 99`, and the base this worktree was cut from already had
   **102** audit files (card `C-EMIT-MAIN-UNIFY` §8 measured that), so
   `test_the_derived_facts_are_recomputed_rather_than_carried` was failing
   `99 != 102` before this card existed and now fails `99 != 103`. One
   regeneration reconciles all four. Recorded rather than absorbed, because a
   tripwire that was already red is a different fact from one this card turned
   red.

4. **Public API changes**, all additive except two:
   * new in `qcore-core`: the modules `std_http` and `std_http_c_emit`;
     `module_imports::HTTP_MODULE` and `ModuleImports::http`;
     `std_socket::{SocketPayload, TextPayload}`;
     `std_socket_c_emit::entry_parts`.
   * **breaking**, and deliberately so: `std_socket::SocketStatement`'s `Send`
     and `SendTo` carry a `SocketPayload` rather than a `Vec<SocketOperand>`,
     and `Recv`/`RecvFrom` gain an `expect` field. A payload is now a kind of
     thing rather than a list, which is the change the follow-on asked for; the
     alternative (two more variants) would have left two spellings of one
     concept.
   * new in `qcore-frontend`: the module `source_http`;
     `source_socket::{socket_statement, SocketLowering, Refusals, text_payload_span}`;
     `SocketUse::TextPayload`; `SemanticPass::StandardHttp`.
   * new in `qcore-verification`: `CheckedSourceModule::StandardHttp`,
     `CheckedHttpModule`, `as_standard_http`,
     `SourceExecutionProfile::StandardHttpV1`.

5. **`examples/README.md` gained two rows**, not one: the `sockets/` family was
   never indexed by its own card, and indexing `http/` beside a missing
   `sockets/` would have read as an omission. Purely additive; flagged because it
   is another card's directory.
