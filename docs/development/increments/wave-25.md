# Wave 25 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

<!-- migrated-section:start -->
Wave 25 close (2026-09-12): five cards merged one at a time under the
failure-gated chain, each under its own green gate, beside one closure the user
validated on their own Windows host. `WINDOWS-NATIVE-SOCKETS-HARNESS` armed the
wave-24 native suites for MSVC — `std_socket`, `std_http` and the string
rendering runs, whose native modules had been `cfg(any(linux, macos))` and so
did not exist on Windows at all — through the established
`tests/support/windows_c.rs` harness, with the Unix arms byte-untouched in
behaviour and the Windows arms type-checked and linted from Linux against
`x86_64-pc-windows-msvc`, the check proved live by a deliberate error inserted
and removed. The card **discharges nothing by itself**: it makes
`HTTP-V1-WINDOWS-RUN` and `STRING-SLICE-IO-WINDOWS-RUN` dischargeable by exact
named test rows, and its Windows arm claims the **byte-comparison** half only —
MSVC ships no ASan/UBSan in this harness, so leak detection stays a Linux
observation and Apple ASan/UBSan an Apple one. Its socket half was **superseded
at integration**: while it was in flight the user's own host landed `360d9464`
with a validated `native_windows` module and a `windows_c.rs` extension, and
real `cl.exe` runs of both published socket examples against Winsock loopback —
Visual Studio 18 Community, MSVC 19.51.36248, stdout byte-identical to the
reference observation, stderr empty, two runs of one program identical, the
complete 49-test `std_socket` target green — so `SOCKET-V1-WINDOWS-RUN` is
**discharged on the user's validated evidence** (byte comparison; the sanitizer
half stays a Unix observation) and the merged tree carries the user's wiring for
that file, the card standing for `std_http.rs` and for the `string_slice_io`
CRLF transcript repair. New residual `WINDOWS-RUN-TEST-NAME-SANITIZED`: four
tests keep `runs_sanitized` in their names while their Windows arm makes no
sanitizer claim, recorded rather than renamed because two frozen reports cite
those exact paths (constraint 21). `SOCKET-EVENT-COMPOSE` made a checked socket
a reactor-registered event source — the socket card's own named follow-on,
retired — and the custody relation is a **loan** both existing models were
already shaped to give: the projection carries no `CreateSocketPair` and no
`CloseSocket`, `watch E S read;` is an `OwnershipOp::BorrowShared` running to
its `unwatch`, so a close while registered is the unedited ownership checker's
drop rule and "deregister before close" is a **consequence** rather than a rule
the card wrote. **Zero diagnostic codes allocated and zero reserved words
added**, with one deliberate, argued widening — `Q083`'s condition moves from
"the inherited ownership checker refused" to "an inherited, unedited **custody
model** refused, carrying that model's own error value", the
`NATIVE-LIBRARY-HEADER-DECORATION` precedent — and a dispatch turn **drains
nothing**, which is what level-triggered readiness means. The emitted program
has no run-time registry: the reactor it runs *is* the declared registration
set materialised as the `poll` set at each turn. One published example,
`examples/events/socket_server.qcore`, watching two descriptors, sanitized on
Linux with its stdout byte-compared; Windows and Apple stay compile-path-correct
only (`SOCKET-EVENT-WINDOWS-RUN`, `SOCKET-EVENT-APPLE-RUN`), and
`HTTP-V1-EVENT-COMPOSE` is deliberately **not** landed and stays a refusal.
`LOOP-V1` landed `source.loop.v1` — `for NAME in 0..BOUND { … }`, admitted on
`source.buffer.v1` only — whose checking route is **exact bounded unrolling and
deliberately not a fixed point, and the reason is not cost**: a route that
publishes exact values and validates them by byte comparison against a native
run would find a widening over a back edge replacing "byte 3 is 65" with "byte 3
is some byte", which makes the route's evidence unwritable rather than the loop
cheaper. The per-iteration custody discipline was almost entirely already there,
so `Q099` exists to make the rule hold at a bound of **one**; the emitted C is a
**real `for`**, which is what makes the native run a check of the unrolling
rather than a restatement of it. **One reserved word** (`for`, census-clean) and
zero new tokens; **`in` is deliberately not reserved**, the constraint-14 census
having found it as a genuine reference in the pinned contract corpus
(`ensures normal resource_borrowed(in);`) — the `len` shape, taking that card's
narrow contextual answer. `Q097`–`Q099` under a new `Loop` pass, the band
`HTTP-QUIRIN-V1` returned to the pool, and `STD-BUFFER-COPY` is **real**: a
bounded `dst[i] = src[i]` copy with no new byte statement at all, byte-compared
against a sanitized run as a positive leak control. `STRING-CONCAT` landed
computed text — `concat(…)` over check-time-known pieces, `dec(NAME)`/`hex(NAME)`
as a piece, and `borrow_utf8(NAME)` bridging an owned `std.buffer` — and its
headline is that the dependency three earlier documents recorded ("concat needs
owned storage, which is the borrow's question first") is **false and measured
so**: every v1 piece is check-time-known, so the compiler holds the composed
bytes and the result is a new `static const` payload with no allocation, no
copy, no custody and a length that is itself a constant. `Q077`–`Q079`, the
`StandardString` pass's third band and all three used; **zero** reserved words;
one published example, `examples/strings/computed_length.qcore`, rendering a
computed `Content-Length`. The arbiter `AOM-SIBLINGS-BINDING-POISON` repaired the
blanket binding poison with a **bounded allow-list**: a refused statement now
spares only bindings it provably cannot have written — a plain name, an automatic
object of this body whose address is never taken, no write position in the refused
text — while every refused **condition** and control-flow refusal keeps the full
poison (`AOM-POISON-CONDITION-FRAME`). **Nothing is promoted and no pin moves**:
unit verdicts stay 2 `Safe` / 42 provisional `Unsafe` / 137 `Unknown` / 4
data-only with zero status and zero `reason_code` moves, the projected-function
histogram stays 150 / 84 / 4,600 with zero moves across 4,834 functions, and
**leak stays 266** with not one leak leaf added or removed, so constraint 7 gains
no fifth move. What did move is attributed leaf by leaf: `unknown_alias`
4,421 → 4,344 and `unsupported` 49,781 → 49,773, 87 `Unknown`-class leaves across
26 units, 1 added and 86 removed. The L19399 leaf — `CONSTRUCTOR-RESIDUALS-LEAK-271`,
which wave 23 decided by **withdrawal** — is now decided by the **discharging**
arm of `flb_input_chunk_destroy_direct_routes`'s contract, that arm's first
firing anywhere in the pinned corpus, which is a better answer to the same
question and changes no count. Three workspace assertions were re-pinned, each
argued and one strictly more accusing (a `UnknownAlias` becoming a
`DoubleRelease`), and the card's pre-registered prediction record is published
as it stood: six structural claims held, three numeric ranges were wrong with
the reason named. Two integration reconciliations were made in the integrator's
own surface rather than inside any card: the Windows-wiring supersession above,
and `STRING-CONCAT` reconciled with `LOOP-V1` on its own branch after the loop
card merged first — `ValueRun` keeps the walk, `BufferEvaluation::apply` keeps
meaning, a flat prelude-scan fail-**open** trap was caught before it could
silently regress, the loop-window refusal is fail-**closed** at `Q068`, and
`LOOP-V1`'s own 39 and 52 test suites are unchanged on the merged tree, so
`STRING-CONCAT-LOOP-WINDOW` was predicted and then **met** rather than left
open. The grammar's loop section was renumbered §G07.11 → **§G07.12** at
integration, `SOCKET-EVENT-COMPOSE` having merged first, with its
cross-references repaired in the same commit. Gate at the integration tip:
4,089 workspace tests (8 ignored), 122 provider carried, 193 Python, 107 Lean
audits (103 → 107: `AuditSocketEventCompose`, `AuditSourceLoop`,
`AuditStringConcat`, `AuditRefusedWrites`), both probes exit 0, fmt and clippy
clean; `qcore-schemas` stays 14.0.0, so constraint 18's rebuild-and-verify
obligation is not engaged. The corpus baseline stays
`docs/reports/wave23-c-operations/artifacts/` with the attributed AFTER runs
chained beneath it — `aom-destroy-siblings`, now `aom-binding-poison` — and no
frozen archive is edited. Nothing is promoted: every raw Cycle 19 unit stays
unpromoted, a provisional `Unsafe` is never an upstream defect claim, and full
language conformance stays **false**. Evidence:
`docs/reports/windows-native-sockets-harness/README.md`,
`docs/reports/socket-v1-windows-run/README.md`,
`docs/reports/socket-event-compose/README.md`, `docs/reports/loop-v1/README.md`,
`docs/reports/string-concat/README.md`,
`docs/reports/aom-binding-poison/README.md`,
`docs/reports/wave25-close/gate.json`.

SOCKET-V1-WINDOWS-RUN (2026-09-12): the two published `source.socket.v1` examples now compile and execute through the shared Visual Studio infrastructure on Windows, with a harness entry point that refuses unless the `cc` toolchain resolved Microsoft's actual `cl.exe` rather than an MSVC-like substitute. The TCP and UDP programs link Winsock solely through the emitted source's `#pragma comment(lib, "ws2_32.lib")`, run their real loopback implementations, keep stderr empty and reproduce the reference observation byte for byte; a repeated TCP run pins platform-independent determinism. Visual Studio 18 Community MSVC 19.51.36248 passes all three native rows and the complete 49-test `std_socket` target, and the MSVC-core CI lane now names that target explicitly while Clang is unavailable. The standing workspace gate also exposed and repaired one unrelated CRLF-only test-reader defect: `string_slice_io` now normalizes the included README before locating its LF-spelled transcript fence, without changing source or string semantics. This is native execution evidence, not a sanitizer claim; socket semantics, certificates, verdicts, providers, Lean and frozen corpus artifacts are unchanged. Evidence: `docs/reports/socket-v1-windows-run/README.md`.

<!-- migrated-section:end -->
