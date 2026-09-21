# How `std.http` should exist — revision 0.3 decision record

> **Status: the choice reserved to the user was exercised, and it was
> strategy (a).** §1 below recommended (b), a contracted native-library binding.
> The user directed the opposite for the first wave, in two statements recorded
> verbatim in §0: *"The first wave of HTTP implementation should implement
> HTTP/1.1 spec so it can achieve with only quirin qcore sources and the latter
> C11 emitter"* and *"HTTP quirin v1 should be executed instead of HTTP quoted
> something."* Card `HTTP-QUIRIN-V1` (wave 24) implemented that route.
> Everything below §1 is **unedited**: the recommendation, its grounds and its
> counter-argument stand as the record of what was argued before the decision,
> because a decision record that rewrote its own reasoning after the fact would
> be worthless. §0 is the only addition.
>
> Authored by card `NATIVE-LIBRARY-SOURCE-BINDING` (wave 23) as its second,
> design-only deliverable; §0 added by card `HTTP-QUIRIN-V1`.

## 0. The decision, and what it changed about §2(a)

**Decided: strategy (a), for the first wave.** `std.http` exists as a checked
source surface in Quirin, lowered to self-contained C11 by the existing backend,
linking no library and declaring no extern function. Evidence:
`docs/reports/http-quirin-v1/README.md`; the published programs are under
`examples/http/`, and both **run** sanitized with their output byte-compared
against the compiler's own reference execution.

**§2(a)'s objection list was right about the capabilities and wrong about what
they gate.** That list — a string tier, loops over buffers, bulk operations,
growth, recoverable allocation failure, sub-ranges, concurrency at the boundary —
is the prerequisite list for *parsing arbitrary HTTP*. It is not the prerequisite
list for *constructing and checking* HTTP messages, and the distinction is what
made the first wave possible:

* a **parser** needs a varying-index tier, because it reads bytes whose values
  it does not know. `HTTP-QUIRIN-V1` has none and claims none: received bytes are
  **verified against a declared expectation**, and reading a message out of
  unknown input is the named follow-on `HTTP-V1-PARSE`. §2(a)'s first four
  bullets gate exactly that follow-on and nothing this card shipped.
* a **builder** needs only literals — and it turns out to buy something strategy
  (b) cannot offer at all. Because every component is a literal, its byte length
  is known at check time, so a declared `Content-Length` that is not the
  content's length is a **compile-time refusal**. A contracted binding to a C
  library could not state that: the library computes the length at run time, and
  the contract's only available posture toward it is a premise.
* the **string tier** §2(a) called missing landed for its own reasons while this
  document sat (`STRING_SLICE_V1.md` §§1–4, §7, cards `STRING-SLICE` and
  `STRING-SLICE-IO`), and `QUIRIN-SOCKET-V1` supplied the transport the same
  wave. §4's prerequisite table was current when written and was overtaken.

**What §3's grounds look like after the fact.** Ground 3 — "it is honest about
what is proved" — was the strongest argument for (b) and it survives inverted:
under (a) the verified fraction is *small but exactly stated*, and the card's
claim vocabulary is "an executed HTTP/1.1 subset with the boundary named", never
"HTTP/1.1 conformance". Ground 4's failure mode is likewise better rather than
worse: there is no substituted artifact to violate a premise about, because there
is no artifact. Ground 1 is unaffected — the L3 apparatus stays available and
unused by this route — and grounds 2 and 5 describe work a later `std.http`
increment may still want.

**What is still true of (b).** Everything §5 lists remains missing, and a
production HTTP client or server is not reachable from (a) alone: no chunked
coding, no persistent connections, no external peer, no TLS. §8's follow-ons are
therefore **not retired**; `STD-HTTP-NATIVE-ALTERNATIVE` is the one that inverts,
since strategy (a) is now the implemented route and a contracted binding is the
open alternative. The comparison should be re-run rather than assumed settled —
which is what that follow-on already said, in the other direction.

## 1. Decision

`std.http` should exist as a **contracted native-library binding of an existing
C HTTP library** (strategy (b) below), not as HTTP re-implemented in QCore
(strategy (a)).

The recommendation is the integrator's standing one and it is recorded here so
that a future card inherits an argument rather than a preference. The choice
itself is the user's: nothing in this document is a commitment, and a decision
for (a) would not contradict anything already landed — it would simply be a much
larger programme with a different set of prerequisites.

`NATIVE_LIBRARY_LINKAGE_V1.md` §6 already says `std.http` is "a direction, not a
v1 deliverable", and §12 stages it as `(d+)`. This document is that staging made
explicit: it says *which* direction, *why*, and *what is still missing*.

## 2. The two candidates

### (a) Native QCore HTTP

Implement request parsing, header handling, chunked transfer encoding, body
framing and connection management in QCore source, lowered to C11 by the
existing backend.

What it would need, none of which exists today:

* **a string tier.** `STRING_SLICE_V1.md` is proposed, not implemented; its own
  follow-ons `STRING-SLICE-LEX-TYPE` and `STRING-SLICE-EXEC-C11` are the whole of
  "QCore can hold text at all". HTTP is a text protocol at its boundaries.
* **loops over buffers.** `source.buffer.v1` (card `STD-BUFFER`, wave 22) is a
  *flat* route: every index is a check-time constant, refused at `Q052`
  otherwise, and `STD-BUFFER-VARYING-INDEX` names the missing capability as a
  relational domain over indices. A parser is a loop over a varying index by
  definition.
* **bulk operations.** `STD-BUFFER-COPY` names the missing loop or bulk
  statement; a protocol implementation copies constantly.
* **growth.** `STD-BUFFER-GROW` names the missing custody shape (an acquisition
  that replaces a live one). A body of unknown length needs it.
* **recoverable allocation failure**, `STD-BUFFER-ALLOC-FAILURE`.
* **sub-ranges.** `STD-BUFFER-SLICE` is **retired in part** by card
  `CT07-SLICE-PERMISSIONS`: `borrow_utf8(NAME, LOW, HIGH)` lends a read
  sub-range, and the blocker recorded here — a footprint fraction in the
  ownership IR's borrow nodes — was measured not to be what that direction
  needed. Every header field is a sub-range, and a *read* of one is now
  expressible; what is not is a **write** lend
  (`SLICE-PERMISSIONS-WRITE-LEND`), and a parser reads its sub-ranges at
  varying indices anyway, which is the bullet above.
* **concurrency at the boundary.** Extern calls stay refused in concurrent
  contexts (`Q026`-class), and a server is concurrent.

Each of those is an existing named follow-on with its own owner and its own
missing capability. Sequenced, they are a multi-wave programme whose deliverable
would be *a second implementation of something the world already has*, checked
against nothing but itself.

### (b) A contracted native-library binding

Bind an existing, audited C HTTP library through the L3 manifest route:
declared `.qcore` contracts in the unchanged C04 vocabulary, one
`source.native-library.v1` manifest binding each exported symbol to its contract
and to a named build, the exported header read and compared, and the source file
naming the library it binds.

`std.http` then *is* a contract corpus plus a set of custody modes for a real
library, and what QCore verifies is the **custody protocol at the boundary** —
who owns a connection, a request, a response body; when each is released; which
call may fail and what its failure arm must dispose of. That is exactly what the
linkage spectrum was built for, and exactly what the C implementation's own type
system cannot state.

## 3. Why (b) — the grounds

1. **It is what the linkage spectrum exists for.** `UNSAFE_LINKAGE_V1.md` §2
   defines a spectrum from a bare waiver (L1) to a manifest-bound declared
   contract (L3); `NATIVE_LIBRARY_LINKAGE_V1.md` §1 states the "short-circuit"
   in as many words — "the mechanism … lives in audited native code, and QCore
   verifies the *custody protocol* at the boundary rather than re-deriving the
   mechanism from generated C11". An HTTP library is the canonical instance of
   that sentence. Building (a) instead would leave the whole L3 apparatus
   unused for the one capability it was designed around.
2. **The Fluent Bit heritage.** This repository's entire existing-C corpus is a
   C data pipeline: 185 units of Fluent Bit, with contracts already written for
   `flb_*`, `cio_*`, `mk_*`, `rd_*` and `msgpack_*` families. The custody
   vocabulary, the W4 summary inference, the declared-ABI agreement gate and the
   contract-corpus conventions were all grown against real C library surfaces of
   exactly this shape. An HTTP binding is the *same activity* the repository has
   been doing for twenty cycles, applied to a library the language ships rather
   than to one it audits.
3. **It is honest about what is proved.** Under (b), `std.http` publishes a
   declared-contract premise, an artifact premise naming `library@hash`, and the
   four ABI premise fields — and claims nothing about the library's
   correctness (hard invariant 2). Under (a), a reader would reasonably expect
   the protocol implementation itself to be verified, and the parts that matter
   most (header parsing, framing) are precisely the parts the flat route cannot
   express, so the verified fraction would be small and the claim easy to
   overstate.
4. **The failure mode is bounded and visible.** A substituted library build
   under (b) is a *premise violation against a named artifact*, detectable
   because the certificate says which build was obligated. A bug in a
   hand-written QCore HTTP parser under (a) is a bug in this repository.
5. **It composes with what is next.** `std.event` v1 (card `STD-EVENT`, wave 22)
   already models reactor and descriptor custody — watch, unwatch, dispatch,
   close — which is the *other* half of an HTTP client or server. A contracted
   library binding sits beside it; a native implementation would have to
   re-derive it.

The honest counter-argument, recorded rather than suppressed: (b) makes the
language's most visible standard module a **trusted declaration about somebody
else's code**. That is a real cost and it must be stated wherever `std.http` is
documented, in the same words the extern-call family already uses — every
downstream verdict is valid *relative to* the declarations, and the library's
conformance to its own contracts is a premise, never a proof.

## 4. Prerequisites that have landed

| Piece | Where | What it supplies to (b) |
|---|---|---|
| Manifest format, loader, artifact identity, ABI premises | `UNSAFE-LINKAGE-ARTIFACT` (wave 19), `crates/qcore-core/src/native_library.rs` | the trust object: a build named on the certificate |
| Contract slice binding, both directions, closed allocator custody | `NATIVE-LIBRARY-CONTRACT-SLICE` (wave 20) | where a library's **pointer** symbols belong, and the check that a slice which can acquire can also discharge |
| Exported-header reading and agreement | `NATIVE-LIBRARY-HEADER` (wave 22) | the manifest's transcription checked against the library's real `.h`, with a measured digest |
| Source-file-scoped binding | **`NATIVE-LIBRARY-SOURCE-BINDING`** (wave 23, this card) | `extern library NAME;` — a source file names the build its extern surface resolves against, and a symbol of an unbound manifest refuses |
| Owned extent-carrying byte buffer | `STD-BUFFER` (wave 22), `source.buffer.v1` | the source-level value an HTTP body would be, once it can cross a boundary |
| Checked event vocabulary | `STD-EVENT` (wave 22), `source.event.v1` | reactor and descriptor custody — the transport half |
| String views (**design only**) | `STRING_SLICE_V1.md` | the proposed shape of text; no implementation |
| Declared-contract corpus conventions | the canonical contract corpus, 72 modules | how a library's custody modes are written down |

## 5. Named gaps

These are the reasons (b) is not startable today. Each names its missing
capability rather than an intention.

| Gap | What is missing |
|---|---|
| `STD-BUFFER-EXTERN` | passing a `source.buffer.v1` buffer to a declared extern callee. A C04 pointer is an extentless `u8` projection, so the crossing would discard the extent the route carries and replace it with a premise. Every HTTP read and write is this crossing |
| `STD-BUFFER-ABI-EXTENT` | the C04 half of the same, stated separately because it is a **contract-loader** change: the vocabulary itself must carry an extent. This is `NATIVE_LIBRARY_LINKAGE_V1.md`'s stage (b), and it is the single largest prerequisite |
| `STRING-SLICE-ABI` | an extent- and lifetime-carrying extern/native-library ABI for string views. `STRING_SLICE_V1.md` §6 refuses the crossing outright until a separate ABI profile specifies aggregate layout, target `size_t`, provenance, lifetime and callee retention. HTTP headers are strings, so this is not optional for a usable surface |
| `NATIVE-LIBRARY-HEADER-DECORATION` | export-decoration macros, the census's single dominant unreadable-header reason. Every real HTTP library's header is decorated (`CURL_EXTERN`, `MHD_EXTERN`, `NGHTTP2_EXTERN`, …), so today the header level would be unusable against one. The obvious repair — a manifest-declared erasure list — converts a reader boundary into an unchecked premise about the header's own text, which is what the level exists to remove; a successor must argue it |
| `NATIVE-LIBRARY-HEADER-INCLUDE` | following `#include` into a bounded, manifest-declared set of further texts. An HTTP library's exported types live across several headers |
| `NATIVE-LIBRARY-HEADER-FNPTR` | a manifest spelling for `fnptr`. Every C HTTP library's API is callback-shaped; this needs `AbiType` widened, which is a change to what may cross a call boundary |
| `EXTERN-MODES-SENTINEL` | sentinel/errno adapters. A C HTTP call reports failure by return code, and the adapter vocabulary admits only the nullable-owned and reallocated shapes today |
| Struct transport | requests and responses are not `u8` buffers. Named as out of scope by `NATIVE_LIBRARY_LINKAGE_V1.md` §10 and unowned |
| Extern calls in concurrent contexts | `Q026`-class refusal, unchanged. A server is concurrent; a one-request-at-a-time client is not, so this bounds the *first* deliverable rather than blocking it |
| **Candidate library selection** | no library is chosen, and choosing one is not this document's act — see §6 |

## 6. Candidate libraries and selection criteria

**No library is selected here, and this document does not rank them.** What it
records is the criteria a selection must satisfy, so that the eventual choice can
be argued against a written standard rather than a preference. Plausible
candidates all sit in the C ecosystem this repository already audits — an
embeddable server library, a client transfer library, an HTTP/2 framing library,
or a header-only parser — and each trades differently against the criteria below.

| Criterion | Why it decides |
|---|---|
| **A closed allocator custody story** | design §4.4's `allocator_custody closed` is a structural check since wave 20: every allocation the library hands out must be released only through the library's own declared entry points. A library that hands out memory the caller frees with `free` cannot state that premise honestly |
| **A small, stable exported surface** | the manifest binding is **total in both directions** (§4.1): every contract in the slice is bound by an entry and vice versa. A sprawling or churning API makes the slice a maintenance surface and every upstream release a re-pin |
| **Readable exported headers** | until `NATIVE-LIBRARY-HEADER-DECORATION` lands, a decorated header is unreadable at `Q063`. A library whose public header is plain C11 declarations is the only one bindable today |
| **Scalar-and-pointer signatures** | the C04 ABI projection admits fixed-width scalars, `uint8_t *`, `const uint8_t *` and `uint8_t **`. Struct-by-value and function-pointer parameters are refused until their own follow-ons land |
| **No unwinding, no threads of its own** | `unwinding abort` and `thread_posture single_caller` are the only admitted v1 values. A library that starts its own threads cannot state the second |
| **A licence compatible with bundling** | `std.http` would ship the contracts and the manifest; whether it ships the artifact is a packaging decision with its own consequences |
| **An auditable build** | the artifact identity is a *stated pin*, so the value of `library@hash` depends entirely on whether a builder can reproduce the named build |
| **Prior art in this repository's corpus** | a library already reachable from the Fluent Bit corpus would let the first contracts be validated against real analysed C rather than against a fixture prelude — constraint 9's warning (`NEIGHBOURS-FIXTURE-PROXY`) applies with full force here |

A selection card must publish its comparison against this table, and must say
which criteria the chosen library *fails* and what the consequence is.

## 7. What a first deliverable would look like

Recorded so that "strategy (b)" is not mistaken for "all of HTTP".

The first shippable increment under (b) is **not** a usable HTTP module. It is
a bounded client surface — open a connection, issue one request with no body,
read a response into an owned buffer, close — with:

* one `.qcore` contract module for the library's slice, in the unchanged C04
  vocabulary, every clause `ContractTrust::Declared`;
* one `source.native-library.v1` manifest binding each symbol to that module and
  to a named build, with the exported header bound and checked;
* `extern library NAME;` in every source file that uses it (this card's form);
* the `std.http` registry name, through constraint 14's census and §L21's
  diagnostic allocation process, gated exactly as `std.buffer` and `std.event`
  are — the capability is unavailable without the file's own import;
* a native harness linking the real library under ASan/UBSan, on the
  `compiler/tests/extern_calls.rs` model, so the declared modes are exercised
  against an implementation rather than only against a fixture.

Everything beyond that — bodies, headers as text, streaming, TLS, servers,
HTTP/2 — waits on §5.

## 8. Named follow-ons

* `STD-HTTP-LIBRARY-SELECTION`: choose a candidate against §6's criteria and
  publish the comparison, including the criteria it fails.
* `STD-HTTP-CONTRACT-SLICE`: the `.qcore` contract module and manifest for the
  chosen library's bounded first surface.
* `STD-HTTP-REGISTRY-NAME`: the `std.http` bundled-module name, its constraint-14
  census and its §L21 diagnostic allocation.
* `STD-HTTP-NATIVE-HARNESS`: the sanitized native harness linking the real
  library.
* `STD-HTTP-EVENT-COMPOSE`: composition with `source.event.v1`, once a
  transport-owning surface exists.
* `STD-HTTP-NATIVE-ALTERNATIVE`: the strategy-(a) record, kept open rather than
  closed. If the string tier and the buffer loops land for their own reasons, the
  comparison should be re-run rather than assumed settled.

## 9. Relation to other documents

`NATIVE_LIBRARY_LINKAGE_V1.md` §6 and §12 stage `std.http` and are unchanged by
this document; §6's list of HTTP prerequisites is the ancestor of §5 here.
`UNSAFE_LINKAGE_V1.md` §2 defines the spectrum this recommendation sits at the
far end of. `STRING_SLICE_V1.md` §6 and its `STRING-SLICE-ABI` follow-on own the
text-crossing half of §5. `STD_BUFFER_V1.md` and the `STD-BUFFER-EXTERN` /
`STD-BUFFER-ABI-EXTENT` follow-ons own the byte-crossing half. `STD_EVENT_V1.md`
owns the transport half. None of those is edited here.
