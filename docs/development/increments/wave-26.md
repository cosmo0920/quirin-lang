# Wave 26 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

<!-- migrated-section:start -->
Wave 26 close (2026-09-12): five cards merged one at a time under the
failure-gated chain, each under its own green gate, plus the integrator's own
three per-owner conflict compositions — the spec's §L21 rows, the examples index
rows, and `examples/sockets/README.md`'s boundary list, where the merged HEAD's
socket-event bullet was composed with the inet card's post-address boundaries
rather than either side being taken whole. `CT-WIRE-LANE` answered the user's
"anything else we can wire with the current implemented tools?" with a row for
every one of the seventeen unmapped CT cases and moved the three the answer is
**yes** for: CT06 and CT11 on `STRING-CONCAT`'s `borrow_utf8` bridge plus
`STRING-SLICE`'s slices — the first CT cases mapped on the **sequential** route
at all — and CT49 on `OBS-REORDERING`'s window-and-emitter comparison, through a
new `reordering_partial_order` verdict mode that also carries the wave's one new
sanitized-native row. CT mapping moves **33/50 → 36/50**, unmapped 17 → 14, and
harness-gated sanitized-native **4 of 33 → 5 of 36**; the **proof stage stays 0
of 36**, honestly, because no theorem is bound to any CT fixture (`T00.7`). Of
the seventeen reasons, 3 are retired and mapped, 5 partially retired and still
unmapped, and 9 stand. **A grown denominator is mapping, not conformance**:
nothing is promoted, `git diff` over `crates/` and `compiler/src/` is empty for
that card, and eighteen scratch mutations were each refused. The lane also
**withdrew two labelled rows it had measured** (CT24, CT47) rather than loosen a
test around them — `newly_decidable_not_mapped_reasons` is derived from that
judgment's own key set, so publishing them would have put three Python tests red
against a matrix the card may not regenerate; both cases keep card T00's
DECIDABLE-NOW bucket and the measurements are published in the lane's own record
(constraint 23). `LOOP-UNROLL-BUDGET` turned `source.loop.v1`'s single ceiling
into a three-layer budget — an **absolute ceiling of 4096** no declaration may
raise, a **default budget of 256** an unattributed `for` is held to, and a
**licence** the statement-adjacent `[| #unroll, where #bound < N |]` clause list
declares — closing `LOOP-BUDGET`. **The measurement that chose the numbers found
something the brief did not expect**: the cost of exact bounded unrolling is not
a function of the bound at all. Three of four representative bodies are linear
and cheap at every admissible bound, while a body acquiring and releasing once
per iteration was **quadratic** at **56.4 s** of check time at bound 1024,
because `LOOP-V1`'s peak-live narrowing had fixed the *count* of prefixes scanned
and left each one re-executed from the start; replacing that rescan with one pass
takes the same body to **0.39 s**, a factor of **145**, changes no published
value (three checks say so, including a Lean theorem over any stepper and any
measure), and is what made 4096 defensible rather than 1024 — lowering it would
have withdrawn the full-extent traversal `MAX_ITERATIONS == MAX_EXTENT` exists to
keep expressible. The declaration is **checked, never trusted**, the attribute
**erases** (attributed and unattributed programs of the same admitted bound emit
byte-identical C), and the card allocated **zero** diagnostic codes — `Q097` and
`Q098` condition extensions under the iceboxed `Q-NAMESPACE-DECISION` — and
**zero** reserved words, a first for this family, with one new token `#` and the
`quant[| n where n := 5 |]` alias's continued behaviour re-measured rather than
asserted. One `LOOP-V1` fixture was amended deliberately, 38 of 39 untouched.
`HTTP-V1-COMPUTED-LENGTH` closed the seam `HTTP-QUIRIN-V1` and `STRING-CONCAT`
each recorded from their own side: a header value or a content may now be a value
the entry computed — a concatenation, a `dec`/`hex` rendering, a `len` binding —
with `admit_field` and `admit_framing` **unedited**, because a component is
resolved to bytes *before* those gates run and so the framing gate never learns
which spelling produced them. The `Content-Length` equality therefore stays
**decided** rather than becoming a premise, the `CR`/`LF` exclusion strengthens to
an **iff** (a composition is framing-free exactly when every piece is, so header
injection stays unexpressible however a value was built), and
`examples/http/computed_length.qcore` serves a computed length over live loopback
with every pre-existing program byte-identical against reverted sources. Standing
constraint 15f is **amended** accordingly — its wording pinned "the literal
content's byte count", now "the content's byte count", both sides still
check-time-known — deliberately and stated, not quietly. `EXAMPLES-SUITE-WIRE`
put all **eleven** example families under `examples_suite.rs` by **directory
scan** rather than a hand-kept list, so the gap it closes stays closed
structurally; it published `examples/loops/buffer_copy.qcore` with a sanitized
run, added 12 tests and repaired 5 README pages, closing
`STRING-SLICE-IO-EXAMPLE-SUITE`, `HTTP-V1-EXAMPLE-SUITE` and `LOOP-EXAMPLE` while
explicitly **not** closing `STD-BUFFER-EXAMPLE` (a loop example that runs on the
buffer route is not a buffer family page). It carries no Lean on stated ground —
it moves no semantic surface. It also found the gate defect this close acts on:
`cargo test --workspace` is fail-fast, so one constraint-16 flake hid the entire
`qcorec` crate from a run, which is why the standing gate now documents
**`--no-fail-fast`** (`GATE-NO-FAIL-FAST`, a strengthening). `SOCKET-INET-ADDRESSES`
answered the user's "bind IPv4 and IPv6 addresses with ports for outbound and
inbound transportations" with declared address literals carrying **zero** reserved
words and **zero** lexer changes — a dotted quad is already `Float Dot Float`,
`[::1]` rides the existing brackets and `[|` still wins by longest match — an
address position being told from a name position by the one fact that every
address of this route spells a port. RFC 4291 forms 1–2 are admitted and the
embedded-v4 and zone forms refused **by name**; `IPV6_V6ONLY` is 1 on all three
arms and proved constant in Lean; the IPv6 probe follows constraint 15d's shape
with **no v4 fallback**; `SO_REUSEADDR` is set on the POSIX arms and deliberately
**absent** on Winsock — one property, opposite options, argued rather than
inherited. The external-peer refusal is located and proved
(`an_undeclared_address_reaches_nothing`), `Q084`'s condition is widened and
stated, and standing constraint **15i** is the wave's only new rule, numbered in
merge order after 15h. Merged gate at the integration tip `4989622c`: **4,209
workspace tests passed, 0 failed, 8 ignored**; 122 provider tests **carried**
(no provider source, manifest template or `qcore-schemas` version changed, so
constraint 18's rebuild is not engaged and 14.0.0 stands); 193 Python; **110 Lean
audits** (107 → 110: `AuditLoopBudget`, `AuditHttpComputedLength`,
`AuditSocketInetAddresses`), read from the text line rather than an exit code;
both probes exit 0; `fmt` and `clippy` clean. **Nothing is promoted.** No wave-26
card touched a gateway, contract, audit or provider file, so the Cycle 19 corpus
rows are **carried** from the wave-25 measurement rather than re-measured: raw
units stay **2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only**,
functions 150 / 84 / 4,600 of 4,834 projected, leak diagnostics **266** with
constraint 7 gaining no fifth move, and the contract corpus **73 modules**. Every
provisional `Unsafe` stays a provisional claim and never an upstream defect
claim; full language conformance stays **false**. Constraint 16's load-sensitive
rows tripped during two cards' runs (the confined-fixture write test and the
`RLIMIT_NPROC` row) and were verified in isolation exactly as that constraint
prescribes, with no assertion weakened. Evidence:
`docs/reports/ct-wire-lane/README.md`,
`docs/reports/loop-unroll-budget/README.md`,
`docs/reports/http-v1-computed-length/README.md`,
`docs/reports/examples-suite-wire/README.md`,
`docs/reports/socket-inet-addresses/README.md`,
`docs/reports/wave26-close/gate.json`.

<!-- migrated-section:end -->
