# Wave 24 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

<!-- migrated-section:start -->
Wave 24 close (2026-09-12): five cards merged one at a time under the
failure-gated chain, each under its own green gate. `DOCS-COMPACT-2` ran the
recurring compaction of `AGENTS.md`, `CLAUDE.md` and `README.md` before the
wave opened — 33 standing constraints in and 33 out with zero dropped and zero
renumbered (renumbering was ruled out by the card's own census, which found
five Rust files citing constraints by number), 67 evidence pointers in and 67
out, 63 diagnostic codes in and out, and three drifts corrected by re-reading
their sources: constraint 16 said "four rows" and listed five, constraint 25
put the frozen B00 ledger's 59-module checkpoint "beside a live 69" where
`contract/qcore` measures 73, and `docs/GENERALIZATION_PLAN.md` §6.1 still
carried the pre-sync-rule instruction to update `README.md` in the same change.
`README.md` was deliberately *grown* by 21 lines, because six admitted source
profiles had no mention in the human summary at all, and the every-2–3-waves
cadence is now a standing working rule in `CLAUDE.md`; the card changed no fact
about the system and carries the follow-on `DOCS-COMPACT-2-W4-DESIGN-POINTER`.
`QUIRIN-SOCKET-V1` then provided `std.socket` and `source.socket.v1`, a checked
TCP **and** UDP surface whose defining decision is that every peer of every
socket in an admitted program is another socket of the same program, on
loopback, in one thread — not a testing convenience but what makes the route's
two determinism obligations decidable: the model carries each stream's
committed bytes and each datagram queue, a receive the program's own history
does not satisfy is refused at `Q085` rather than deadlocked on at run time,
and what a receive yields is published in the certificate and **compared by the
emitted program**, which exits non-zero on disagreement. Custody is the
unedited `qcore_core::ownership` checker's, the close discipline is read out of
`os_endpoint::close_poststate` — which answered the poisoned `Attempted` where
the card expected `Indeterminate`, recorded as `SOCKET-V1-CLOSE-ERRNO` rather
than argued away — the derived row carries `Alloc` and `Io` and never `Mut`,
and ten reserved words landed after a 412-fixture / 41,529-literal census that
**rejected** `stream` and `socket` by measurement. `STRING-SLICE-IO`
implemented the string amendment's own §7 follow-on: a `source.string.v1` entry
may write `print TEXT;` and `log LEVEL TEXT;` over a quoted literal, a bound
string or one inline slice, with the `|>` spellings the same statements — and
the capability rule did not move, which is the point: `std.io` alone still
grants no string and `std.string` alone still opens no stream, and what changed
is that a file naming **both** can render text. The rendering rule is the
identity on bytes, framed for both line kinds by one function so a rendered
scalar line **is** the text line of its own digits (`rfl` in Lean); a string
takes no rendering selector and one written in front of one keeps refusing at
`Q072`; and an embedded NUL renders as an ordinary byte, which was not free —
every `printf` conversion for a byte range stops at the first `0`, so the
emitted line writes the payload with a length-bounded `fwrite` from a **second
prelude fragment** inserted only into a program that renders text, with the
write's result checked rather than discarded, which is what keeps every other
emitted program byte-identical. Zero reserved words, `driver.rs` untouched, and
one shared-ground edit turning the pipe's closed sink table from selector
counts into ranges, which moves one refusal from the pipe to the route (`x |>
print;` now refuses at `Q030` naming the missing `dec`/`hex`) and creates no
acceptance, on a 126-occurrence census of every piped sink statement on the
base. One real cross-card composition surfaced at integration and was repaired
there rather than inside either card: the socket card's ninth `Imports` field
met the string-io card's eight-field constructors, caught by the Lean `TEXT`
check rather than by a test. `C-EMIT-MAIN-UNIFY` answered the user's
architecture direction with the census first — ten emitters wrote `main` in
four byte spellings, and the collision was measured **not reachable today**,
because the driver's route dispatch is a chain of mutually exclusive arms and
33 emitted units carry exactly one `int main(` each — so the card manufactures
no defect and repairs the architecture the concern correctly names: composition
no longer means writing a new whole-program backend with a new entry point, but
contributing keyed `Fragment`s in declared sections plus one `Entry`, where the
single-entry guarantee is a type rather than a check. **33 emitted translation
units, 33 identical digests, zero differing bytes** against a pristine checkout
of the base, every pin unmoved, with the one section inversion the card found —
the task route's `#include <stdio.h>` sitting among its preludes — reported as
`C-EMIT-TASK-LATE-STDIO` rather than repaired under two F02 emission digests.
`HTTP-QUIRIN-V1`, the wave's milestone, is `merge`'s first consumer and uses
that mechanism as designed: **an executed HTTP/1.1 subset with the boundary
named**, written in pure Quirin over the socket route's unedited checker, with
zero external C libraries. Its move is that the design record's prerequisite
list was the list for *parsing* HTTP and not for constructing and checking it —
every component of a declared message is a quoted literal, so its bytes and
therefore its length are known at check time — and 22 static conformance checks
follow, two of them the card's signature: **no admitted component may carry a
`CR` or an `LF`**, so every CRLF in an admitted head is one the compiler wrote
and header injection is not expressible on this route rather than merely
unintended; and a declared `Content-Length` must **equal** the literal
content's byte count, a compile-time refusal a contracted binding to a C
library could not have offered at all, since the library computes the length at
run time. Two published programs run under ASan/UBSan/LSan with their output
byte-compared, and the second is published for the contrast:
`raw_request.qcore` puts the same kind of bytes on the wire by hand and the
compiler checks only that the receive is satisfiable — a payload a program
*writes* is bytes, a message it *declares* is checked. Four reserved words on a
416-fixture / 43,239-literal census that **rejected `body`**, a binding name in
two shipped fixtures, where RFC 9110 §6.4 had already renamed the concept
`content`. Band discipline held all wave: `Q080`–`Q089` and `Q090`–`Q099` were
pre-assigned by the integrator before their cards started rather than relocated
after a collision, two ranges came back — `Q077`–`Q079` from `STRING-SLICE-IO`
and `Q097`–`Q099` from `HTTP-QUIRIN-V1` — and the returned lower range was
deliberately not backfilled. Beside the merges, `EVENT-V1-APPLE-RUN` is
**discharged** on the user's own macOS environment: the `std_event` suite
including its `cfg(not(windows))` native comparison ran green, scope recorded
exactly as Apple ASan/UBSan and byte comparison with no LSan claim;
`SOCKET-V1-APPLE-RUN` stays open until a macOS run at or after the socket card
covers `std_socket`'s native tests. Measured close gate: **3,950 workspace
tests, 0 failed, 8 ignored**, `fmt` and `clippy` clean, both reevaluation
probes exit 0, **193 Python tests**, **103 Lean audits** (99 → 103:
`AuditStdSocket`, `AuditStringSliceIo`, `AuditCProgram`, `AuditStdHttp`), and
122 provider tests carried unchanged with `qcore-schemas` still 14.0.0 and no
package rebuilt. **The corpus is untouched and unread all wave**: no gateway,
contract, audit or provider file appears in any of the five diffs, so the leak
pin stays **266**, the contract corpus stays 73 modules, unit verdicts stay 2
`Safe` / 42 provisional `Unsafe` / 137 `Unknown` / 4 data-only, the function
histogram stays 150 / 84 / 4,600, and the baseline
`docs/reports/wave23-c-operations/artifacts/` is neither re-measured nor
re-pinned. Nothing is promoted: no raw Cycle 19 unit is promoted, no function
moved status in either direction, every provisional `Unsafe` stays a
provisional claim and never an upstream defect claim, and full language
conformance stays false. Evidence: `docs/reports/docs-compact-2/README.md`,
`docs/reports/quirin-socket-v1/README.md`,
`docs/reports/string-slice-io/README.md`,
`docs/reports/c-emit-main-unify/README.md`,
`docs/reports/http-quirin-v1/README.md`.

Native-library decoration Windows fixture repair (2026-09-12): the manifest and visibility-header mutation helpers now normalize checkout line endings before applying LF-spelled edits, so the four negative rows exercise their intended malformed include and non-derivable decoration cases under CRLF rather than either failing at the test guard or silently compiling the unedited manifest. The header-removal row also asserts that both binding statements were actually removed. The focused `native_library_header_decoration` suite passes all 22 tests; the full workspace test gate, formatting, Clippy, both reevaluation probes and all 99 Lean audits pass. Production reader semantics, verdicts, provider packages and frozen corpus artifacts are unchanged.

<!-- migrated-section:end -->
