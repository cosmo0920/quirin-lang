# Io rules

Current reference, routed by [AGENTS.md](../../AGENTS.md). Rule numbers are stable.
Paths in backticks are repository-relative. Read every applicable rule,
including cross-topic rules, before editing; the index is not a substitute.

<a id="constraint-15c"></a>

<!-- preserved:constraint-15c:start -->
15c. **An event program's streams are compared byte for byte, and that is a
    measurement about the reactor profile rather than a convention.**
    `source.event.v1`'s one accepted profile is single-threaded and the reactor
    refuses a cross-thread activation under it (`L04`), so the thread running a
    callback is the entry's own, every record of the derived
    `qcore.obs-reordering.v1` relation (constraint 15a) is a **barrier**, and
    the relation is strict. `observe_event_program`'s live-set walk over this
    model's regions (a dispatch turn) derives the window structure, and the
    emitters a region makes live are read from
    `ReactorProfile::threading_enabled`. A route that runs a callback
    on another thread, or gives a callback a source-level body, **supplies its
    emitters there** rather than declaring strictness or a reordering by hand.
    Do not weaken the two premises the composition rests on: a source line is
    program output and writes no trace record, and a `wake`'s payload is a value
    and never a reactor fact — which together are why every transcript this
    route published before the card is byte-identical after it.
    `wave22-std-event`, `wave22-obs-reordering`, `compose-event-lines`.
<!-- preserved:constraint-15c:end -->

<a id="constraint-15d"></a>

<!-- preserved:constraint-15d:start -->
15d. **Windows `std.event` selects AF_UNIX by capability, not by compilation.**
    A usable `<afunix.h>` only exposes the candidate; the emitted shim probes
    `socket(AF_UNIX, SOCK_STREAM, 0)` on the running system. Only an explicit
    unsupported-family/type/protocol result from that probe selects the
    retained AF_INET loopback pair; other probe failures refuse. Once the
    probe succeeds, pathname, bind/listen/connect/accept or peer-validation
    failure refuses pair creation and must not be hidden by changing
    transports. Both routes stay native-tested and preserve `SOCKET`,
    `WSAPoll`, the abstract endpoint identities and the byte-identical trace.
    `windows-std-event-af-unix`.
<!-- preserved:constraint-15d:end -->

<a id="constraint-15e"></a>

<!-- preserved:constraint-15e:start -->
15e. **A rendered string's observation is the view's exact bytes, and the
    emitted write is length-bounded.** `source.string.v1`'s text lines write no
    escaping, no terminator and no newline beyond the framing every line already
    carries, because a payload's `0` is an ordinary byte inside `len` and a
    scan-to-terminator write and the model **disagree** on a value the route
    admits. Do not render a string through any `printf` conversion: `%s` and
    `%.*s` both stop at the first `0`. The write lives in `io_emit`'s second
    prelude fragment, inserted only into a program that renders text — which is
    what keeps every other emitted program byte-identical — and exempted from the
    single-emission-route scan by whole-line identity exactly as the first is;
    its `fwrite` result is **checked**, because nothing in the checker rules a
    short write out. The stream, the `[LEVEL] ` prefix and the single newline are
    `std_io::line_framing` for **both** line kinds, so a rendered scalar line is
    the text line of its own digits (`rfl` in Lean); do not give either kind a
    second framing. A string takes **no** rendering selector and one written in
    front of a string refuses at `Q072`. `string-slice`, `string-slice-io`.
<!-- preserved:constraint-15e:end -->

<a id="constraint-15f"></a>

<!-- preserved:constraint-15f:start -->
15f. **An HTTP message's framing is constructed, and its `Content-Length` is
    decided.** `source.http.v1` writes every CRLF itself — one after the start
    line, one after each field, one to end the head — and **no admitted component
    may carry a `CR` or an `LF`**. Do not relax that exclusion: it is what makes
    "every CRLF in an admitted head is one the compiler wrote" true, and header
    injection unexpressible rather than merely unintended. The exclusion applies to
    **composed** bytes exactly as to written ones — a composition is framing-free
    *if and only if* every piece is — so injection stays unexpressible however a
    value was built. A declared `Content-Length` must **equal** the content's byte
    count, decided at check time **because every component is check-time-known**: a
    quoted token, or (card `HTTP-V1-COMPUTED-LENGTH`) a value the entry composed
    from check-time-known pieces. Do not turn that equality into a premise, and do
    not admit a content whose length the route cannot compute. A component is
    **resolved to bytes before `admit_field` and `admit_framing` run**, which is
    what keeps both gates unedited and keeps a computed length decided rather than
    assumed; the field **name** stays a written token. The transport half is
    `source.socket.v1`'s **unedited**
    checker over a program this route lowers to, and its refusals are
    re-attributed at `Q095` carrying that checker's own error value — do not
    re-implement a custody, transport-state or determinism rule here. The
    composition's emission goes through `c_program::merge` plus one
    `c_program::assemble`; do not fork a whole-program backend. `std.http` grants
    **no** transport and **no** text: a quoted token at a message component is
    this module's syntax and constructs no `string` value
    (`STRING_SLICE_V1.md` §2), while a `let`-bound one does and takes
    `std.string`'s gate. `http-quirin-v1`.
<!-- preserved:constraint-15f:end -->

<a id="constraint-15g"></a>

<!-- preserved:constraint-15g:start -->
15g. **A reactor registration is a loan, and the reactor owns no descriptor of
    `source.socket.v1`.** A composed entry's projection carries **no**
    `CreateSocketPair` and **no** `CloseSocket` — those are the operations by
    which `event_reactor` owns a number — and `watch E S …;` is an
    `OwnershipOp::BorrowShared` of `S` to `E` running to the matching
    `unwatch E;`. Do not give the reactor a descriptor of this route: the three
    rules a reader wants (a close while registered refuses, a
    `bind`/`listen`/`connect` on a watched socket refuses, a
    `send`/`recv`/`accept` on one is admitted) are the **unedited** ownership
    checker's drop, write and read under a shared loan, and "deregister before
    close" is their consequence rather than a rule. The loan's span is
    **lexical**, because the ownership IR's borrow nodes are; a crossed pair is
    refused by name and must not be approximated
    (`SOCKET-EVENT-CROSSED-SPANS`). A dispatch turn **drains nothing** — it
    observes level-triggered readiness and the entry's own transport statements
    do the work — and the four bookkeeping statements emit no call and derive no
    effect, because the reactor an emitted program runs *is* the declared
    registration set materialised as the `poll` set at each turn. `Q083`'s
    condition is "an inherited, unedited custody model refused, and the variant
    carries that model's own error value": do not split it per model.
    `socket-event-compose`.
<!-- preserved:constraint-15g:end -->

<a id="constraint-15i"></a>

<!-- preserved:constraint-15i:start -->
15i. **A declared address is a check-time constant parsed once, and the two
    address families are disjoint.** `source.socket.v1`'s `bind NAME ADDRESS;`,
    `connect NAME ADDRESS;` and `sendto NAME ADDRESS …;` carry a literal IPv4
    dotted quad or a bracketed IPv6 literal and a port that is an integer literal
    or a check-time-constant binding. The address is parsed **once**, by
    `qcore_core::socket_address`, and the emitted program carries its **octets** —
    do not add an `inet_pton` (or any other text conversion) to the emitted C, or
    two parsers can read one spelling differently. **The address form admits no
    external peer**: a target is resolved against the addresses this program's own
    binds declared, and one no bind answers to refuses at `Q085`
    (`SOCKET-EXTERNAL-PEER`); an **ephemeral** port never names a socket, which is
    also what keeps every pre-existing implicit-bind program unreachable by
    address. Custody and the transport check read **one** resolution
    (`std_socket::resolve_addresses`) so a `connect` through an address and a
    `connect` through a name write the *same* listener — do not add a second
    resolution, or constraint 15g's loan could be evaded by spelling a socket
    differently. Two per-arm decisions are **explicit and must stay explicit**,
    because the platforms' defaults differ and an inherited default is a silent
    divergence: every IPv6 socket sets `IPV6_V6ONLY` to **1** on every arm (proved
    constant in `QCore.SocketInetAddresses`), and a declared bind must *succeed*
    when no live socket holds the address and *fail* when one does — which needs
    `SO_REUSEADDR` on the POSIX arms (a `TIME_WAIT` remnant is not a live holder)
    and needs it **absent** on Winsock, where it would admit a second live socket.
    `SO_REUSEPORT` is set nowhere. A host without IPv6 is **probed on the running
    system** (constraint 15d's shape): only an explicit
    unsupported-family/type/protocol result selects the distinct recorded exit
    status, every other failure refuses, and **nothing falls back to IPv4** — a
    fallback would bind a different address from the declared one. The implicit
    `bind NAME;` and `connect NAME NAME;` forms emit the statements they always
    did. `socket-inet-addresses`.
<!-- preserved:constraint-15i:end -->

<a id="constraint-15p"></a>

<!-- preserved:constraint-15p:start -->
15p. **A socket program's protocol cells are derived, never declared, and the
    derivation refuses where it cannot pair.** Card `CT42-CELL-ROUTE` (wave
    39) routes `qcore_core::socket_footprint`'s cells from source through
    `disjoint NAME NAME;`, the one statement of `source.socket.v1` that asks a
    question rather than performing an operation: the transport checker
    decides only that both names are live sockets of the entry (`Q081`,
    `Q082`, and the inherited ownership checker's `Q083` through a `Read` of
    each), the question derives no effect and **erases from emission**
    (byte-identical C and observation with and without it, measured), and it
    is answered by the **unedited** `DisjointFootprintsV1` checker over a
    projection of `DeriveDescriptorAlias` events plus one `AssertDisjoint`,
    reported under that checker's own code and pass (`Q664`, `Concurrency`).
    Four rules must survive refactors. **The deriver's unpaired arms are
    refusals** (`SocketCellError::UnpairedAccept`/`UnpairedConnect`, reported
    at `Q085`): card `STABILITY-REEVAL-3` measured that a singleton cell
    assigned there is judged *disjoint* from the binding's real connector —
    the admitting direction — so do not restore the wave-31 fallback
    (`QCore.SocketFootprint.an_admitted_accept_takes_an_existing_cell` states
    the direction; the arms are driven by fixtures through
    `derive_cells_of_admitted`). **A derived alias stamps no
    `DeclaredProtocolCellIdentity`** (`FootprintEvent::DeriveDescriptorAlias`,
    the T09 edit `CT42-CELL-DERIVED-PREMISE` asked for): its ground is the
    transport checker's admission — no external peer (`Q085`), every socket
    enumerated — and the projection is produced by one function over one
    admitted program; do not add a route that hands the checker a derived
    alias from any other source, and do not make the variant stamp the premise
    "to be safe" (that is the relabelling the edit replaced). **Every
    composition that lowers to a socket program composes the cell check**:
    the reactor composition shares the reader, and the HTTP composition —
    which hands unknown statements to the socket reader — was measured to
    admit the question with no judgement until its driver composed
    `check_socket_cells` over its re-lowered transport; a new composition
    that lowers to `SocketProgram` inherits the question and must answer it.
    **A datagram pair is not a cell** (`CT42-CELL-UDP-QUEUE`, re-affirmed
    with its ground and retired by re-affirmation): a `sendto` appends to a
    queue but the two sockets share no connection phase, so the question is
    admitted for them; landing a datagram cell is a deliberate reopening. What
    stays measured rather than proved is that the transport model's cross-name
    coupling *is* the shared logical state (`SOCKET-CELL-COUPLING-UNPROVED`);
    what stays out of scope is the dup-like alias (`CT42-CELL-DUP-SOURCE`)
    and the C-twin arm (`CT-CTWIN-DESCRIPTOR`). `ct42-cell-route`,
    `ct42-protocol-cell`, `stability-reeval-3`.

<!-- preserved:constraint-15p:end -->
