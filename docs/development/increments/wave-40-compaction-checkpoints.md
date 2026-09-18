# Wave 40 compaction checkpoints (`DOCS-COMPACT-8`)

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md) · [Wave 40 records](wave-40.md)

> **Preserved checkpoint text. These blocks are not a current rule authority.**
> They are the verbatim text `DOCS-COMPACT-8` compacted out of `AGENTS.md` and
> `CLAUDE.md` at base `d6a57f66`, kept so that nothing the compaction removed
> is lost. Where a block and the current file disagree, **the current file
> wins**; where a block states something the card measured to be false, the
> correction is in `docs/reports/docs-compact-8/README.md` §4 and is
> deliberately *not* applied here, since that would defeat a verbatim copy.

This appendix is separate from [`wave-40.md`](wave-40.md) so that the wave's
record page stays a record page: `wave-37.md` reached 64,613 bytes after
`DOCS-COMPACT-7` put its checkpoints inline, against the retrieval guidance
that a new history page stay below 32 KiB where a complete record boundary
permits it, and that card flagged the split as the close's call to make.

## `AGENTS.md` standing constraint 14 — the per-card keyword-census ledger

The whole entry as it stood at `d6a57f66`, 19,638 bytes. The compacted entry keeps the rule, every standing decision and the method; what is here is the card-by-card census narrative, which each card's own report and each reserved word's source comment in `qcore_frontend::frontend::is_reserved_source_word` also carry.

```text
14. New keywords are a recorded compatibility hazard: every candidate is
    **censused against every fixture** before it is reserved, and the census is
    run rather than assumed. The ledger, in card order:
    * **Reserving cards.** T03-F **+5**; T08-EPOCH-SRC **+8** (with `work`
      renamed `effort` rather than editing the pinned golden surface);
      `source.extern-call.v1` **+1** (`succeeds`); STD-MODULES **+8** (`print`,
      `log`, `flush`, `dec`, `hex`, `info`, `warn`, `fatal`); the
      quiet-emission amendment **+2** (`trace`, `debug`) plus the explicit
      system-trace `log_level` declaration, after a full fixture census. The
      already-reserved `error` keyword is accepted contextually as a logger
      level **only after `log`**; it was not removed from the global keyword set.
    * `QUIRIN-SOCKET-V1` **+10** — `tcp`, `udp`, `bind`, `listen`, `connect`,
      `accept`, `send`, `recv`, `sendto`, `recvfrom` — each a clause word in a
      statement and never a reference, each measured load-bearing, all ten
      census-clean across every `.qcore`/`.qc` fixture (412 files) and every
      Rust string literal (41,529). `let`, `close` and `return` are reused
      rather than respelled, which is why the release is `close NAME;`. Two
      candidate spellings were **rejected by the census and must stay
      rejected**: `stream` (a contract `extern fn` parameter name, shielded
      only by the following `:`) and `socket` (it would name a transport the
      route deliberately makes explicit at creation).
    * `LOOP-V1` **+1** (`for`, census-clean at zero code occurrences across all
      418 `.qcore`/`.qc` files) and **no new token**, `..` being two existing
      `Dot` tokens. **`in` is deliberately not reserved**: the census found it
      as a genuine reference inside the pinned contract corpus
      (`ensures normal resource_borrowed(in);`), exactly the shape that kept
      `len` out of the global list, so it takes that card's narrow contextual
      rule instead. Do not promote it to the global set.
    * `LOOP-UNROLL-BUDGET` **+0 words — a first for this family — and +1
      token**, `#`, promoted from a lexer error on `OR-AND-LOWERING`'s
      precedent for `^` and census-clean at **zero** code occurrences across
      422 `.qcore`/`.qc` files. It reserves nothing because a `[| … |]` clause
      list contributes **no referenced name** —
      `frontend::lexical_references` returns the empty set for the attribute
      statement — so `unroll` and `where` stay contextual and
      `let unroll = 1;` is still a program. That matters most for `where`,
      which the census found **eight** times in code, every one inside a
      quantitative index type whose readable alias is spelled
      `quant[| n where n := 5 |]` — the very brackets this attribute borrows.
      The disambiguation therefore takes **two** tokens (a statement head is
      `[|` *followed by* `#`), the alias's continued behaviour is
      **re-measured** rather than asserted, and `#` is still not an identifier
      character, so the scoped-name premise behind `c#0` survives.
    * `NATIVE-LIBRARY-SOURCE-BINDING` **+0**: `extern library NAME;` reuses
      `extern` and `library`, claiming only the two-word sequence at
      declaration position, which was a `Q000` parse failure before. Its census
      found the two whole-token-stream route selectors —
      `module_imports::concurrency_span` and `source_tasks::has_source_tasks` —
      and **excluded binding spans from both**, so a library named `spawn`,
      `join`, `qcore_buffer_new`, `qcore_buffer_read`, `qcore_buffer_write` or
      `qcore_error_payload` cannot select a route from its own spelling. **Do
      not remove that exclusion.**
    * `SOCKET-INET-ADDRESSES` **+0, and no lexer rule changed**: a declared
      address (`bind S 127.0.0.1:8080;`, `connect C [::1]:8080;`) is read back
      out of the tokens the existing lexer already produces — a dotted quad is
      `Float Dot Float` because `127.0` is a float spelling, `::` is the
      existing atomic `ColonColon`, and `[|` still wins over `[` by longest
      match, so the `[| |]` and `..` golden surfaces are untouched. An address
      position is told from a name position by the one fact that **every
      address of this route spells a port**, so its token run carries a `:`;
      the run's tokens must be lexically adjacent. Its one name-resolution
      exclusion is **the address reader itself** —
      `source_socket::address_host_span` returns the token range of a
      statement's address *host*, deliberately leaving the **port** outside it
      so a port name stays a genuine reference — and the census measures it
      claiming a span in **no** `.qcore`/`.qc` file of this repository outside
      the card's own two examples.
    * `AGGREGATES-V1` **+0 words and +0 tokens**, and its census is the one that
      **rejected a spelling on archived evidence**. The head is the
      already-reserved `type`, claimed contextually at `type NAME {` — censused
      at **zero** bare occurrences across 435 `.qcore`/`.qc` files (all eleven
      `type NAME {` in the tree are `resource type`, which an earlier branch
      claims) and zero in Rust string literals, so the shape was a parse failure
      before the card, the `extern library NAME;` precedent. **`record` is
      rejected and must stay rejected**:
      `docs/reports/remaining-v07/probes/d02_boundary_record.qcore` is the single
      line `record R { }` and **two archived reports pin it to `Q000`**, so
      claiming the word at declaration position would make that file parse and
      falsify a frozen archive (constraint 21) — the `buffer` → `bytes` and
      `work` → `effort` shape, the syntax renamed rather than the pin edited.
      `struct` is census-clean and was still not taken, on the ground that the
      language already has a declaration head for a named type. The card also ran
      the census in the **other** direction — which *existing* reserved words does
      the construct put in a new position — and found the six whole-token-stream
      route-selector words: a record naming `spawn`, `join` or one of the four
      buffer/payload intrinsics refuses `Q015` declared and `Q000` constructed.
      Both are fail-closed, so `NATIVE-LIBRARY-SOURCE-BINDING`'s span exclusion
      was measured and **deliberately not copied** — it would retire the `Q015`
      half and leave the `Q000` half, buying a type that declares and cannot be
      constructed (`AGGREGATE-SELECTOR-WORDS`).
    * **+0, exact rather than statistical:** `QUANT-REGION-SOURCE`. Every word of
      `error region … catch (…) quant open … as …` and `discharge` is reserved
      already (Cycle 6; `credit`/`effort` from `T08-EPOCH-SRC`) and the shape
      already parsed, so the reservation question does not arise: before the card
      the statement collected `R` as a referenced name and refused at `Q013`,
      measured, so **no program in this repository could contain it at all**. The
      `credit`/`effort` seam with the epoch route is one-way and was probed in
      both directions rather than argued.
    * **+0, censused not assumed:** EXTERN-MODES (`move NAME` spells all four
      departing modes and the declaration decides which ran),
      `SOCKET-EVENT-COMPOSE`, `STRING-CONCAT`, `HTTP-V1-COMPUTED-LENGTH`,
      `CT-WIRE-LANE`; and in wave 27 `CT07-SLICE-PERMISSIONS` (`borrow_utf8` has
      been contextual since `STRING-CONCAT`, `,` and `)` are existing tokens, and
      **no `.qcore`/`.qc` file on disk spells the new three-argument arity**, so
      no shipped program's meaning changes — with **`part` recorded as colliding**
      in binding position in a shipped fixture, and `lend`, `range`, `low`,
      `high`, `sub`, `window` recorded free for a later card) and
      `CT23-CREDIT-ERROR-PATH` (whose census is **vacuous by construction**:
      `error`, `move`, `drop`, `close` and `effort` are all already reserved, so
      the `error(…)` row clause can stop no existing source parsing); and in
      waves 28–29 `REGION-CREDIT-JOINS` and `REGION-JOINS-DUTIES`, vacuous by
      construction on the same ground (every word of the composed form already
      reserved and the shape previously refused, measured not assumed).
    * `SLICE-PERMISSIONS-WRITE-LEND` **+0 words, one contextual clause word**:
      `lend` — `let lend = 5; print dec lend;` is still a program (pinned),
      route selection claims only the `lend(` call shape, and the read card's
      census had already recorded the word free and reserved it for this card
      (census archived with reproduction commands).
    * `WRITE-LEND-TASKS` **+0 words, one contextual clause word reused**: the
      wave-29 `lend` is recognised in two further positions that refused before
      the card — immediately before `<Buffer>` in a task parameter type, and as
      a capture's first word inside a spawn's argument list — so `lend` stays
      contextual and no global reservation is taken.
    * `WRITE-LEND-TASKS-EXERCISE` (wave 36) **+0 words and +0 tokens**, and the
      census is a **pre-card** measurement taken at base `0a5c2ecc` rather than
      a claim checked afterwards. The worker-side byte access takes **no new
      word at all**: `qcore_buffer_write` and `qcore_buffer_read` are the
      existing intrinsics, and the **arity** selects the lease form on the
      `borrow_utf8` precedent (constraint 15h's own mechanism), censused at
      **zero** occurrences of either new arity across **514** `.qcore`/`.qc`
      fixtures, so no shipped program's meaning changes.
    * `MUTEX-SHARE-CROSS` **+0 reserved words, +0 tokens, +0 name-resolution
      exclusions** — the cleanest outcome in this ledger, and the card asked
      for this row three waves before it was written
      (`MUTEX-SHARE-CROSS-INTEGRATOR-LIST-UNLANDED`). Both clause words of
      `thread scope NAME share PLACE under MUTEX { … }` were **already
      globally reserved** — `share` by the scoped-task route's
      `read PLACE share N`, `under` by card `T03-GUARD-JOIN` — so the card
      claims only a **position**, the fourth token of a child-task header,
      which was a parse failure before it. Censused over **490** `.qcore`/`.qc`
      files outside `third_party/`: 33 total hits, **8** in code rather than
      comments, **0** occurrences of `share` at any position other than the
      read capture's clause, **0** occurrences of `under` in code anywhere, and
      **0** occurrences of `thread scope NAME share|under` in any fixture. No
      exclusion was carved out of `referenced_names`, unlike
      `SOCKET-INET-ADDRESSES` and `WRITE-LEND-TASKS`, because the card claims
      no word — and both clause **names** stay genuine references, measured
      rather than asserted (an unbound one refuses at `Q013`).
    * `AGGREGATE-OWNED-FIELD` (wave 39) **+0 words, +0 tokens, and no lexer
      rule changed**: the owned field type is spelled `bytes`, the acquisition's
      own reserved word (`STD-BUFFER`), censused in field-type position at zero
      occurrences across every `.qcore`/`.qc` file; the four field-path
      statements (`RECORD.FIELD[I] = V;`, `let X = RECORD.FIELD[I];`,
      `extent(RECORD.FIELD)`, `drop RECORD.FIELD;`) reuse the `Dot`, `[`, `=`
      and `drop` the language already has and are read by the record reader's
      shape rule. **`Buffer` was measured and rejected** for the field type: it
      is the task route's word for a different resource (`qcore_buffer_new`'s
      four-byte payload) with a different model, and a field spelled one while
      meaning the other would be a collision by design; it stays what it was
      (a task parameter type), and `type R { f: Buffer }` refuses at `Q051`
      naming `AGGREGATE-FIELD-TYPES`.
    * **Cards that do not engage this constraint at all** — none reserves a
      word or moves the lexer, and the documentation cards touch no crate
      source: `EXAMPLES-SUITE-WIRE`, `DOCS-COMPACT-3`, `GATEWAY-C-LOCATION`;
      waves 28–29's `EXAMPLES-AGGREGATES`, `CT47-COPY-CAPTURE`, `CT22-WIRE`,
      `CT01-PROOF-LEG` and `W29-CORPUS-CONTRACTS`; wave 30's `DOCS-COMPACT-4`,
      `CT24-CLEANUP-PREEMPTED`, `CT-PROOF-LEG-2`, `REGION-JOINS-LOAN`,
      `CT07-WRITE-REMAP` and `W4-RECURSIVE-SUMMARIES`; wave 31's
      `C-GATEWAY-WAVE`, `CT42-PROTOCOL-CELL`, `CT-WIRE-LANE-3`,
      `W31-CORPUS-CONTRACTS-2` and `CT03-PROOF-ALPHABET`; waves 32, 33 and 34's
      `CT-WIRE-LANE-4`, `CT-COMPOSED-NATIVE`, `TRACE-ALPHABET-LEASE-BOUNDARY`,
      `W32-RECONCILE-SWEEP`, `PROOF-LEG-BATCH`, `DOCS-COMPACT-5`,
      `C-GATEWAY-RETURN-POSITION`, `VERIFIER-HOST-CLOSED-INPUT-DEADLINE`,
      the Lean proof-integrity pass, `STABILITY-REEVAL` #1,
      `C-GATEWAY-RETURN-CALL-W4` and `DOCS-COMPACT-6`; and **wave 35's seven**
      — `SWEEP-BLIND-SPOTS`, `WAVE-34-CLOSE`, `VERIFIER-HOST-CONTROL-BUDGET`,
      `CT-CTWIN-LANE`, `C-GATEWAY-CONSTANT-DO-WHILE`, `MUTEX-SHARE-CROSS-ARMS`
      and `JOINED-PROVENANCE-CYCLES`; and **wave 36's six of seven** —
      `CARD-SCOPE-GUARD`, `W36-CORPUS-CONTRACTS-3`, `STABILITY-REEVAL` #2,
      `C-GATEWAY-INDIRECT-CENSUS-2`, `JOINED-PROVENANCE-CALL-FRAMES` and
      `MUTEX-SHARE-CROSS-EMISSION`, none of which reserves a word or moves the
      lexer (the last changes only what an already-admitted program *emits*,
      and the corpus card adds a `.qcore` contract module, which is no
      source-language surface). Wave 36's **seventh**,
      `WRITE-LEND-TASKS-EXERCISE`, is **not** here: it ran a census and belongs
      in the `+0` family above, where its row stands — the
      `MUTEX-SHARE-CROSS` distinction, applied rather than restated. And
      **wave 37's seven of seven** — `BASE-TYPE-READER-CENSUS`,
      `SCALAR-ROUTE-EXIT-EDGE`, `JOINED-PROVENANCE-ROUND-BOUND`, `CT-LANE-5`,
      `TASK-ROUTE-TSAN`, `W37-CORPUS-CONTRACTS-4` and `DOCS-COMPACT-7`, the
      first wave of this ledger with no card in the `+0` family at all: none
      reserves a word, moves the lexer or allocates a `Q` code, and each says
      so in its own report (`JOINED-PROVENANCE-ROUND-BOUND` §12 item 6,
      `CT-LANE-5` §11 item 4, `TASK-ROUTE-TSAN` §6.2,
      `W37-CORPUS-CONTRACTS-4` §10 item 8, `DOCS-COMPACT-7` §11;
      `SCALAR-ROUTE-EXIT-EDGE` §10 item 4 states the `Q`-space half, its
      refusal being a gateway-internal `Unsupported` description, and
      `BASE-TYPE-READER-CENSUS` is a gateway measurement feature that states
      neither). The wave-37 close therefore **measured** the seam rather than
      reading the reports: `git diff --name-only 0f6eb0a6 c33c7640 --
      crates/qcore-frontend/src/` is empty, so no wave-37 commit touched the
      lexer, the parser or any front-end source. The wave-36 entry above was
      re-verified the same way at that close: over `60958bf0..fa48211a` the
      only front-end file that moved is `source_tasks.rs`, in
      `WRITE-LEND-TASKS-EXERCISE`'s own two commits — the one card the entry
      places in the `+0` family — and `lexer.rs` and `parser.rs` did not move
      (`docs/reports/wave37-close/README.md` §6). **Wave 39's two of five** —
      `CT07-TWO-THREAD-PROOF-LEG` ("+0 keywords", merge `0d9e922f`; "census:
      +0, nothing to census", its wave-39 record paragraph) and `TSAN-ROWS-2`
      ("+0 words", merge `0150c904`; "constraint 14 not engaged (no word, no
      token)", its record paragraph) — neither reserves a word nor moves the
      lexer; the wave's other three cards stand in the families above
      (`AGGREGATE-OWNED-FIELD` +0, `LOOPS-ON-COMPOSED-ROUTES` +0,
      `CT42-CELL-ROUTE` +1). The wave-39 close **measured** the seam the way
      the wave-37 close did: `git diff --name-only e878b362 b6d1f101 --
      crates/qcore-frontend/src/` names six files, and `git log --name-only`
      over the range attributes every one to those three cards —
      `frontend.rs` (the one reserved word) and `source_socket.rs` to
      `CT42-CELL-ROUTE` (`2d1f8fa9`); `source_record.rs`, `whole_program.rs`
      and `source_std_buffer.rs` to `AGGREGATE-OWNED-FIELD` (`e60ad0c1`,
      `2f27c652`, `c960a263`); `source_std_buffer.rs` and
      `source_std_string.rs` to `LOOPS-ON-COMPOSED-ROUTES` (`39cc9848`,
      `6adaf1ef`) — `lexer.rs` and `parser.rs` did not move, and no commit of
      the proof-leg or TSan card touched the front end
      (`docs/reports/wave39-close/README.md` §6). **Wave 39 closed at one
      reserved word** (`disjoint`), the first since `LOOP-V1`'s `for` at
      wave 26, ending the wave-28-to-37 run as the `CT42-CELL-ROUTE` bullet
      states.
      **This bullet said "every card of waves
      32, 33 and 34" over a list that named neither `PROOF-LEG-BATCH` nor
      `DOCS-COMPACT-5` nor `MUTEX-SHARE-CROSS`, and the universal was
      therefore false**; card `SWEEP-BLIND-SPOTS` measured that
      (`CONSTRAINT-14-WAVE-32-CENSUS-INCOMPLETE`) and offered the integrator
      either arm. The wave-35 close took **both**, because they are not
      interchangeable: the quantifier is narrowed to the cards actually
      enumerated, the two documentation-and-proof cards join this list, and
      `MUTEX-SHARE-CROSS` does **not** — it ran a census (its §9) and belongs
      in the `+0` family above, where its row now stands. Adding it here in the
      card's stead would have replaced a false universal with a false member.
      **Waves 28 through 37 each closed at zero reserved words**, the longest
      such run in this ledger; the source surface did not grow a global
      keyword between `LOOP-V1` and wave 39.
    * `CT42-CELL-ROUTE` (wave 39) **+1**: `disjoint`, the disjointness
      question `disjoint NAME NAME;` of `source.socket.v1` (also read by the
      reactor and HTTP compositions) — a clause word at a statement head,
      never a reference, load-bearing in the measured sense (without
      reservation name resolution collects it and refuses at `Q013`),
      **census-clean at zero QCore source occurrences** across 525
      `.qcore`/`.qc` files (eleven occurrences, all inside `//` comments) and
      thirteen Rust string literals that are test names, `expect` messages or
      prose; the already-reserved `assert` and `footprint` were considered
      and not taken. The first reserved word since `LOOP-V1`, ending the
      wave-28-to-37 run deliberately and stated as such.
    * `LOOPS-ON-COMPOSED-ROUTES` (wave 39) **+0 words, +0 tokens, and the
      census was run rather than assumed**: the card admits an existing
      statement (`for`) in two positions that refused before it — beside a
      lend on the buffer route, and inside a composed string/buffer entry —
      and spells nothing new; `git diff --stat e878b362 -- crates/qcore-frontend/src/lexer.rs crates/qcore-frontend/src/parser.rs`
      is empty, the composed route reads the loop through
      `source_std_buffer::lower_for_statement` (the same reader, the same
      bound tier), and the three residual names it refuses by name are
      diagnostic text, not words. Its report §9 states the census.

    `remaining-t03-f`, `remaining-t08-epoch-src`, `extern-call-v1`,
    `wave17-std-modules`, `remaining-extern-modes`,
    `native-library-source-binding`, `quirin-socket-v1`, `loop-v1`,
    `socket-event-compose`, `string-concat`, `socket-inet-addresses`,
    `loop-unroll-budget`, `http-v1-computed-length`, `aggregates-v1`,
    `quant-region-source`, `ct07-slice-permissions`, `ct23-credit-error-path`,
    `region-credit-joins`, `region-joins-duties`,
    `slice-permissions-write-lend`, `write-lend-tasks`.
```

## `AGENTS.md` Q-space note — the eighteen-instance enumeration

The whole note as it stood at `d6a57f66`, 6,644 bytes. The compacted note keeps the icebox decision, the operative consequence, the tally (re-derived, 18 instances on 10 bands), the two undercounting findings and the three cards that are not instances; what is here is the per-instance enumeration those totals were derived from.

```text
**Q-space note — `Q-NAMESPACE-DECISION`, an open decision in the ice box.** The
`Q000`–`Q099` route-code space is **exhausted**: `Q077`–`Q079` (`STRING-CONCAT`)
and `Q097`–`Q099` (`LOOP-V1`) were the last two pools and are spent, so no free
range is left below `Q100`. The next allocation needs a §L21 namespace decision
rather than a range — candidates: open `Q101`/`Q102`/`Q104`+ around the legacy
three-digit bands, or take a new prefix. **By user direction (2026-09-12) that
decision is iceboxed**, on the stated ground that naming collisions happen
easily; it is recorded as an open decision, **not** a rule, and is
integrator/spec work when taken up. Operative consequence until then: **no card
allocates new `Q` codes.** A new refusal lands as a **condition extension of an
existing band**, on the `SOCKET-EVENT-COMPOSE` /
`NATIVE-LIBRARY-HEADER-DECORATION` precedent — the same condition reached by a
new route — with the widening stated in §L21 and in the card's report rather
than taken silently. Constraint 1 is unchanged: a code is still allocated to a
condition, still stable, still not reusable by a second pass. **Waves 26–39 ran
the consequence eighteen times, on ten bands, without strain** (wave 37 ran it
none; wave 39 three times, the sixteenth to eighteenth below) — wave 26:
`LOOP-UNROLL-BUDGET` (`Q097`/`Q098`), `HTTP-V1-COMPUTED-LENGTH`
(`Q090`/`Q093`/`Q094`), `SOCKET-INET-ADDRESSES` (widening `Q084` inside
`Q080`–`Q089`); wave 27: `CT23-CREDIT-ERROR-PATH` (`Q630`/`Q635`/`Q636`/`Q641`),
`AGGREGATES-V1` (`Q051`/`Q052`/`Q053` — that wave's **language head**, an
aggregate type, taking a condition extension rather than a pass of its own),
`QUANT-REGION-SOURCE` (`Q302`, and `Q303`–`Q309` reached from a second route),
`CT07-SLICE-PERMISSIONS` (`Q077`/`Q078`); wave 28: `REGION-CREDIT-JOINS`
(`Q302`); wave 29: `CT47-COPY-CAPTURE` (`Q600`), `REGION-JOINS-DUTIES`
(`Q302`) and `SLICE-PERMISSIONS-WRITE-LEND` (`Q051`–`Q055`, `Q077`); wave 30:
`REGION-JOINS-LOAN` (`Q302` a fourth time, plus the one deliberate `Q600`
**narrowing**); wave 31: `WRITE-LEND-TASKS` (`Q020` and `Q600`, the ninth
band) — each stating its widening in §L21 and in its report and allocating
nothing. Three cards allocated nothing either and are **not** instances of the
rule: `GATEWAY-C-LOCATION` and `CT24-CLEANUP-PREEMPTED`, because fixing where a
diagnostic is *attributed* is not a code change (constraint 1), and
`CT42-PROTOCOL-CELL`, which reaches the footprint checker's existing `Q664`
condition from a new deriver without widening it. Wave 32's
`MUTEX-SHARE-CROSS` is the **fourteenth** and the one that took the **tenth
band**: the declared crossing's two protocol gates land on `Q600` and its third
refusal reaches `Q611` in the `Q610`–`Q627` sub-band through that checker
unedited (constraint 15o). Its rows in the table below are the wave-35 close's,
three waves after the card (`MUTEX-SHARE-CROSS-INTEGRATOR-LIST-UNLANDED`), which
is why the paragraph above missed it. Wave 36's
`WRITE-LEND-TASKS-EXERCISE` is the **fifteenth**, on the same two
bands wave 31 used (`Q020` and `Q600`, so no eleventh band): the worker-side byte
access lands entirely as condition extensions — the at-most-one-lease parameter
rule and the byte spellings' kind refusals on `Q020`, and the footprint fit,
the constant-index tier and the three authority rules on `Q600` — with the
**arity** selecting the form on the `borrow_utf8` precedent, so it reserves no
word and takes no code. Wave 39's `CT42-CELL-ROUTE` is the **seventeenth**,
on two bands already counted (`Q080`–`Q089` and `Q090`–`Q096`): `Q080`,
`Q081` and `Q085` widened to the disjointness question's shapes and the cell
derivation's two refusals, `Q095` widened to the HTTP composition's cell
check, the answer itself reaching the unedited `Q664` — each stated in §L21
and in the card's own rows below, and its report §4.5 and merge message
(`45a33285`) saying "five conditions widened, zero codes allocated" — and
**missed by this paragraph until the wave-39 close**, which counted it from
those rows (`Q-SPACE-EXTENSION-TALLY-HAND-KEPT`'s live instance: nothing
derives this tally, and a card that states its widenings everywhere but here
still went uncounted for one merge). Wave 39's `LOOPS-ON-COMPOSED-ROUTES` is
therefore the **eighteenth** (merged last; its own report and the `b6d1f101`
merge message say "seventeenth", counted before this correction — read them
as that checkpoint), and the first whose movements are **all narrowings**
stated as such: `Q051`, `Q068` and `Q077` each lose the blanket `for` refusal (§L21
amendments), `Q078` is reached from a body statement with its condition
unwidened, the three shapes it refuses by name land on `Q051`/`Q077` as
condition extensions, and the composed route reaches the loop pass's own
`Q097`–`Q099` unwidened (`StringError::Loop`, the `BufferError::Loop`
precedent) — no code allocated. **Waves 33, 34 and 37 allocated no code and ran no
condition extension either**: the gateway
arbiters `C-GATEWAY-RETURN-POSITION`, `C-GATEWAY-RETURN-CALL-W4` and wave 37's
`SCALAR-ROUTE-EXIT-EDGE` move **gateway diagnostics**, which are categories
rather than `Q` codes, so the icebox does not engage; and the remaining cards
of those waves touch no diagnostic at all. **This paragraph said "waves 32–34"
there and put the total at thirteen until wave 37**, a universal its own
`Q600` and `Q610`–`Q627` rows falsified — the same shape as
`CONSTRAINT-14-WAVE-32-CENSUS-INCOMPLETE`, in the same wave, in the neighbouring
constraint. The totals above are re-derived from this paragraph's own
enumeration plus those two rows, and nothing derives them mechanically
(`Q-SPACE-EXTENSION-TALLY-HAND-KEPT`). The icebox has now held for thirteen
waves without a card needing a range. **Re-affirmed on 2026-09-18 for wave 39
— an integrator-recorded decision, carried as still open (re-affirmed, not
resolved) in the wave-38 close's deferred decisions:** condition extensions
only; no card allocates a new `Q` code, including the aggregate-custody card,
which widens the existing `StandardBuffer` and `Ownership` bands with each
widening stated in §L21. **Wave 39's `AGGREGATE-OWNED-FIELD` is the
sixteenth instance**, on the `StandardBuffer` band a fourth time (`Q051`–`Q054`
widened, **`Q055` narrowed** — the first deliberate *lift* under the icebox) and
on the `Ownership` band by vocabulary alone (`Q103` covers the grown
`OwnershipError`, reached by no sequential program:
`OWNED-FIELD-SEQUENTIAL-UNREACHED`), allocating nothing; its rows in the table
below are the card's own.
```

## `AGENTS.md` ledger — the headline and arbiter-chain paragraph

As it stood at `d6a57f66`, 2,680 bytes.

```text
Headline numbers after `W37-CORPUS-CONTRACTS-4` (2026-09-18). The **corpus
baseline is `docs/reports/w37-corpus-contracts-4/artifacts/`**, wave 37's second
arbiter and the chain's second **declarative** link, which moved 8 functions and
zero units.

**The chain is what makes each delta attributable**, and it is one rule applied
at every link: each arbiter's BEFORE run reproduces its predecessor's
verdict-bearing artifacts (`diagnostics.json`, `chunk-contracts.json`) **byte for
byte**, which isolates that arbiter's own movement from the gateway change
beneath it. Since wave 33 that identity is **asserted inside the archiving
tool**, which refuses to write if it ever stops holding, rather than merely
logged. Two link kinds are worth naming. A **declarative** link is the same rule
at less risk — it changes no crate source at all, so one audit binary serves both
its stages and the only difference between them is which modules the contract
manifest selects; `w36-corpus-contracts-3` was the chain's first. A
**zero-movement** link is one whose AFTER is byte-identical to its own BEFORE, so
several consecutive ones collapse: wave 37's baseline BEFORE reproduces
`joined-provenance-call-frames`' two artifacts and thereby all of
`scalar-route-exit-edge`'s and `joined-provenance-round-bound`'s AFTER at once,
because the three are the same bytes — which is the measurement showing those two
code changes moved nothing this corpus can see. The links, newest first:
`w37-corpus-contracts-4` (declarative) over `joined-provenance-round-bound` over
`scalar-route-exit-edge` (wave 37's three) over `joined-provenance-call-frames`
over `w36-corpus-contracts-3` (wave 36's two, the second declarative) over
`joined-provenance-cycles` over `c-gateway-constant-do-while` (wave 35's two)
over `c-gateway-return-call-w4` (wave 34) over `c-gateway-return-position`
(wave 33) over `w31-corpus-contracts-2` over `c-gateway-wave` (wave 31's two)
over `w4-recursive-summaries` (wave 30). Each arbiter's leaf attribution and gate
live in its own report, which is authoritative for them; this file carries no
per-arbiter attribution, and the increment records narrate each wave's. **Eleven**
of those twelve links published an archive and all eleven are frozen
(constraint 21) — `joined-provenance-round-bound` published none, having moved
nothing — as are `wave23-c-operations`, `aom-destroy-siblings`,
`aom-binding-poison`, `gateway-c-location` and `w29-corpus-contracts`.
Diagnostics retain two coordinates: the presented original `file:line` and the
internal preprocessed line used for pin, leaf and certificate identity
(`GATEWAY-LOCATION-INTERNAL-CHANNELS`, constraint 12a).
```

## `AGENTS.md` ledger — the *Cycle 19 raw corpus* row

As it stood at `d6a57f66`, 3,064 bytes. Every figure in it was re-derived from `docs/reports/w37-corpus-contracts-4/artifacts/` by `DOCS-COMPACT-8` and none moved.

```text
| Cycle 19 raw corpus (185 units) | **2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only** — **no unit status has moved at any arbiter from wave 31 through wave 37**. 4,834 projected functions: **213 Safe / 84 provisional Unsafe / 4,537 Unknown**; `flb_input_chunk_create` remains function-level **Safe**. Diagnostics: leak **272**, `unknown_alias` **4,309**, `double_release` **0**, `unsupported` **49,753**, `summary_conflict` **1** — all five read from the wave-37 baseline named above. **Every delta is attributed leaf by leaf in its own arbiter's report §5 and narrated in that wave's increment record, and those are the authorities for it** — this row carries the current numbers, the headline paragraph above carries the chain, and constraint 7 carries the leak pin's own history in full. What belongs here is the **current shape of the evidence**, which four measurements fix. (1) The two declarative arbiters bought their function moves with *contracts, not code*, and neither moved the leak pin: wave 36's six declarations took 20 functions to `Safe` at the cost of the corpus's first `summary_conflict`, and wave 37's ten took 8 more, **none of them a release** — every one the neutral element `N(m)`, which a faithful summary of a body that releases nothing cannot contradict, so `summary_conflict` stayed at 1. (2) Wave 37's leaf movement is **183 removed and 15 added**, and the added figure is the card's strongest measurement rather than a raw count: **all 15 are same-position text substitutions and none is a genuine addition**, so no position in the corpus refuses that did not refuse before. (3) The cover that chooses declarations is an **upper bound and never a forecast** (`W36-COVER-HIDDEN-BLOCKER`): a refusal reachable only once the first is removed is invisible to it. Wave 37 turned that from a surprise into a prediction for one case and met a second, different limit — an ABI **reader** boundary, an inline function-pointer parameter declarator the reader does not parse, withdrawing a declaration leaf-neutrally at every call site (`NESTED-CALL-IN-ARGUMENT-POSITION`, `LOADER-ABI-INLINE-FNPTR-DECLARATOR`), and it records that the remaining frontier is dominated by unit-internal `static` helpers, where W4 and not declaration is the lever (`W37-COVER-FRONTIER-IS-STATIC`). (4) Two pre-passes' reach is measured rather than assumed: conditional provenance (constraint 10b) runs on **40** corpus functions, `release_site`'s naming premise passing **6 of 14** sites and its nullable-site premise blocking all 14; and the loop-header exit-edge route (constraint 5c) reaches **neither** deciding arm at any of the corpus's **201** loop-header `BranchOnNull` nodes, 89.6% of those edges landing in an `untracked` early return — a coverage fact and not a safety one, and one that must not be carried forward as if it were stable (`SCALAR-ROUTE-UNTRACKED-ARM-DOMINATES`). Provisional findings remain subject to the reports' coverage and reachability boundaries; the production `input_chunk_get` residual stays Unknown |
```

## `AGENTS.md` ledger — the *CT01–CT50 conformance* row

As it stood at `d6a57f66`, 5,277 bytes. Every figure in it was re-derived from `docs/reevaluation/ct_manifest.json` by `DOCS-COMPACT-8` and none moved.

```text
| CT01–CT50 conformance | **47/50 mapped**, 3 unmapped; sanitized-native **10 of 47**; **proof 4 of 47** (wave 39: `CT07-TWO-THREAD-PROOF-LEG` moved the proof numerator, and `CT42-CELL-ROUTE` mapped CT42 by routing the socket protocol cell from source — a grown denominator, which is mapping and not conformance). Which lane mapped which case, and on whose evidence, is the increment records' business — waves 28 through 39 each carry their lane's paragraph, and each lane's own report is authoritative for its measurements. What this row carries is the **current state and the standing method**. *The proof leg*: CT01 (wave 29), CT37 (wave 30, chosen by a measured survey of all 42 then-mapped cases), CT03 (wave 31, the first leg to need the F02 alphabet itself to grow, by the T05 Buffer write, proved **conservative** so that a write-free trace relates exactly as before and no existing digest moved) and CT07 (wave 39, `CT07-TWO-THREAD-PROOF-LEG`, the first leg bound **through the scoped-task ledger's lease set** rather than the F02 trace relation — `reference_trace` refuses this run's lease events by constraint 15n's decided boundary and was not touched; wave 31's `no_byte_has_two_write_authorities` is instantiated on the artifact's own two-thread ledger state, the `Reachable` witness being a proved replay (`QCore.Ct07LeaseLedger.replay_reaches`) of the lease opens and closes the checked module's reference execution performs, the production `ScopedTaskLedger` agreeing step for step, the kernel-decided digest recomputed from the artifact, and no alphabet growth). Each of the first three is a scenario property proved in Lean of the checked artifact's reference run **and its admitted renderings** (CT37's completion strictly before the join in every admitted rendering), bound to the case by a named host-unconditional test through the F02 digest relation; CT07's is a property of every ledger state its run reaches, bound by a named host-unconditional test through the ledger's own admission; in all four native execution is deliberately observation-only; the per-case gate is the pinned fail-closed `PROOF_GATED_CASES` table in `ct_corpus.rs`, and each flip's stage evidence supersedes the earlier record **from its own record** (constraint 21). CT03's sibling arm — the lease events the alphabet refuses — was observed and not proved when that leg landed; wave 32's `TRACE-ALPHABET-LEASE-BOUNDARY` discharged that residual and wave 36's `WRITE-LEND-TASKS-EXERCISE` **decided** the forward question, so constraint 15n and not this row carries the measured statement. *The sanitized-native leg*: wave 32's `CT-COMPOSED-NATIVE` gave the `composed_epoch` route a runner that compiles and runs an admitted case's own emitted C under the platform sanitizers and compares its stdout byte for byte with that module's own checked task half — every arm of that runner falsified before it was believed. CT23 stays outside it, being the route's refusal. Wave 36's `MUTEX-SHARE-CROSS-EMISSION` took the tenth slot: CT46's emission, native, sanitized and mutex-reference rows moved from `unassessed` to **executed**, retiring the wave-35 boundary that the `mutex_guard` route's crossing stopped at the checker. *Evidence can improve without a count moving*, and twice has: wave 30 put CT07 on the write-lend scenario, and wave 37's `CT-LANE-5` re-mapped it again onto the **two-thread** range write lease, so the scenario's word *independently* rests on two threads for the first time, with a sanitized native run and an allocation-identity witness behind it — each superseding the earlier record from its own record, with **no count moved**. *The three unmapped* (four until wave 39), each re-measured rather than assumed: CT41's obstruction is **structural** — clause (b) of the wave-9 attribution rule needs a control in which removing `volatile` moves the verdict, and `CTWIN-PUBLISH` §5's decision is that it moves nothing, with the scenario's own consumer shape measured `safe` with no diagnostic and the typed-source arm unable to tell `volatile` from `_Atomic` (`CT41-TYPED-SOURCE-UNDISCRIMINATING`); CT42 **left the set in wave 39**: `CT42-CELL-ROUTE` routed the socket arm from source (`disjoint NAME NAME;`, the two ends of one accepted connection refused as one cell at `Q664` by the unedited `DisjointFootprintsV1` checker, both fallback arms of the deriver made refusals first and the T09 derived-alias edit taken), mapped on the `sequential` route under the `reject` mode with no native, sanitizer or proof claim, its C-twin arm still blocked on `CT-CTWIN-DESCRIPTOR`; CT10 has half a scenario; CT24 stays **withdrawn** by the lane that measured it, a withdrawal rather than a loosened test, because `newly_decidable_not_mapped_reasons` is derived from that judgment's own key set (constraint 25). Every mapped case publishes the boundary its scenario's untranslatable words rest on — no pointer type and heap-bound storage for the buffer cases, no wrapper type and no `Send`/`Sync` trait for CT46, whose crossing is carried instead by the transferability judgment's `TransferKind::ProtectedResource`. A grown denominator is mapping, not conformance, and a grown proof numerator is evidence about one case's artifact (constraint 23) |
```

## `AGENTS.md` ledger — the *Merged gate* row

As it stood at `d6a57f66`, 4,995 bytes: the wave-39 close's measurement and provenance. The row now carries `DOCS-COMPACT-8`'s own detached run instead. Its passed/failed/ignored triple (4,727 / 0 / 10) reproduced exactly; its summary arithmetic did not, and the correction is in `docs/reports/docs-compact-8/README.md` §4 rather than applied here.

```text
| Merged gate | **4,727 passed / 0 failed / 10 ignored, 240 result summaries over 236 test targets** (227 `Running` plus 9 doc-test sections; four of those sections print two summaries each) · **141** Lean audits · **303** Python · 122 provider (carried) · both probes exit 0 · fmt/clippy clean · Clang **present** (21.1.8, recorded and checked). Measured by the **wave-39 close** at its own tip `11edf7eb` (base `b6d1f101`, the `LOOPS-ON-COMPOSED-ROUTES` merge), in **two** detached runs whose figures are identical — a base run at `0e73d600` (the wave's merged code plus this close's report skeleton) and a final run at the tip carrying the close's own edits — and the base run **reproduces that merge's own integration figure exactly** (4,727 / 0 / 10 at `4786ae4c`), so the wave's last integration measurement, this close's base and this close's tip agree three times over: +88 workspace tests and +4 Lean audits (`AuditCt07LeaseLedger`, `AuditAggregateOwnedField`, `AuditSocketCellRoute`, `AuditLoopWindows`) against the wave-38 close's 4,639 / 0 / 9 and 137 at `19c61952`, and +1 ignored (`TSAN-ROWS-2`'s `#[ignore]`d measurement harness) beside the ninth, the `#[ignore]`d F02 pin-agreement test, which stays **red by design** on `--ignored` (`F02-WINDOWS-PIN-STALE-SINCE-WAVE-36`, a Windows row no Linux gate can see): `git submodule update --init --recursive` inside the gate script and exit 0, one uninterrupted `cargo test --workspace --no-fail-fast` of 12m56s in the final run, each run **polled by a bounded foreground loop on its own `DONE` marker**, and **every step's exit captured to its own file, in that run's own directory, with no pipe in the command** (`GATE-EXIT-SWALLOWED-BY-PIPE`). Per-command exits and the full accounting in `docs/reports/wave39-close/gate.json` (each card's own gate in its own report; the three **integration** gates the integrator measured at the merges where no card's gate could agree with the merged tree are `docs/reports/{aggregate-owned-field,tsan-rows-2,loops-on-composed-routes}/integration-gate.json`, swept by that close's §4 — their `clang` claims are the cards' own recordings, `INTEGRATION-GATE-CLANG-CLAIM-BORROWED`). A merge message's measured line is **not** such an accounting: `0d9e922f`'s reads "0 failed" while naming three failed rows, because the summary parser behind it summed only `ok.` summaries (`GATE-SUMMARY-PARSER-OK-ONLY`). The gate file carries the Lean count under a `lean_audits` key that `build_matrix.py --gate` checks against `proof/Audit*.lean` and refuses on disagreement (wave 37), and — as of the wave-38 close — **a `clang` presence claim checked against the `clang-version.txt` recorded beside it, with both keys now required rather than optional**, so the one figure of this row that had no machine-readable source and drifted in two consecutive waves (`GATE-ROW-FIGURE-ATTRIBUTION-DRIFT`, `DOCS-COMPACT-7` §1.1) is refused where it would be written, and a green workspace figure taken without Clang is no longer indistinguishable in the record from one taken with it. Each card's own gate is in its own report, and each close's in its own. **A gate figure is a measurement on one machine, never a property**, and four boundaries travel with this one. First, constraint 16's rows, because that constraint requires a gate report to **state which it saw**: all five named rows that live in this workspace were **green**, `closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request` included, and so was the unnamed seventh `a_forced_override_never_weakens_an_execution` — but this is a full-suite run on a machine carrying its own build, **not** the quiet-host isolated number `HOST-ROW-STARVATION-DESCRIPTION` still asks for, and it is not offered as one. `real_packages_preserve_unknown_and_reject_mutations` lives in the separate `providers/` workspace and is not among them. Second, provider **122 is carried, not re-measured** (nothing under `providers/` or `crates/qcore-schemas/` moved across the wave and `qcore-schemas` is still 14.0.0, so constraint 18 is not engaged) **and is a Unix figure** (`PROVIDER-CONFORMANCE-UNIX-ONLY`). Third, this row is **one measurement at one tip**: every figure in it was produced by the final run named here, with the base run agreeing figure for figure. That run's Python step exited **0** — the first close in four waves whose detached Python step did not trip `D02-DIMENSIONS-FOLLOWUP-COUNT` (`229 != 228` at wave 38, `228 != 227` at the wave-39 proof leg) — and the reason is ordering, not a weaker check: this close regenerated the matrix with an interim `gate.json` carrying the base run's figures *before* the final run started, and the two runs' figures are identical, so nothing was left to reconcile. The Python figure is therefore the run's own, and it was re-run once more after the final regeneration (`gate/python-post.exit`). Fourth, the per-wave narrative this row used to carry is in the increment records |
```

## `AGENTS.md` ledger — the follow-on count paragraph

As it stood at `d6a57f66`, 2,213 bytes, i.e. after `STABILITY-REEVAL-4` re-wrote it, including the wave-39 registration chain the paragraph's own next sentence assigns to the increment records.

```text
Where the detail lives, and the only places to look for it: per-increment
history in `DEVELOPMENT.md` → "Increment records" → the wave index (every wave
and card paragraph keeping its own checkpoint and boundaries); open
follow-ons in `docs/reviews/r0_3/acceptance_matrix.json` (`named_followups`,
**256** entries after `STABILITY-REEVAL-4` (wave 40; −1 +23 against the
wave-39 close's **234**, the one retirement being
`W36-W37-MUTATION-HARNESSES-UNSHIPPED`, measured discharged in full and named
in `docs/reports/stability-reeval-4/README.md` §6). The wave-39 chain that
produced the 234, each step named in its card's report and merge message: 228
after the wave-38 close → 227 after `CT07-TWO-THREAD-PROOF-LEG` (−2 +1) →
232 after `AGGREGATE-OWNED-FIELD` (−1 +6) → 234 after `TSAN-ROWS-2` (−4 +6,
one of the four the integrator's own retirement recorded at `0150c904`) →
231 after `CT42-CELL-ROUTE` (−5 +2) → 233 after `LOOPS-ON-COMPOSED-ROUTES`
(−1 +3) → 234 at the close, which retired two and registered three, each named in
`docs/reports/wave39-close/README.md`. Which card registered or retired which name,
and each wave's chain, is the increment records' business — read the wave
paragraphs; the retirement discipline itself is a standing rule in `CLAUDE.md`,
and each close's own record carries that close's arithmetic. The operative rule
here: the section is *carried*, `dimensions.named_followups` is *derived*, and
the two are checked against each other on the committed file — a hand edit that
leaves the two apart goes red as `171 != 187` did at the wave-36 close, which is
`D02-DIMENSIONS-FOLLOWUP-COUNT`'s tripwire working — so add entries through
`docs/reports/remaining-d02/build_matrix.py` and never by
hand — constraint 25) plus each report's own follow-on list;
`docs/reports/<card>/README.md`, which is authoritative for that card's counts,
boundaries and premises; direction in `docs/GENERALIZATION_PLAN.md` (W0–W7) and
`docs/ROADMAP_AFTER_CYCLE_9.md`'s workstream table; trust/verdict rules and
audit history in `docs/REEVALUATION_SPEC.md` and `docs/REEVALUATION_FOLLOWUP.md`;
the architecture boundary and open gates in `docs/cycles/CYCLE_20.md`.
```

## `AGENTS.md` standing constraint 15o — the mutex crossing's TSan sentence

As it stood at `d6a57f66`, 517 bytes. Its last clause ("no TSan row exists anywhere in this repository") was false since wave 37; the correction is in `docs/reports/docs-compact-8/README.md` §4.9 and the theorem and residual the sentence carries are untouched.

```text
    the admitted shape without re-deriving it. The **creator performs no
    trace event while its child runs**, proved as `QCore.MutexShareCrossEmis
    sion.the_creator_performs_no_event_while_its_child_runs`, so this
    backend witnesses a real thread boundary and **nothing about contention,
    a data race or the C memory model**; no such claim may be made from it
    (`MUTEX-SHARE-CROSS-EMISSION-NO-CONTENTION`), and no TSan row exists
    anywhere in this repository. And the **operation/log adjacency** is
```

## `CLAUDE.md` platform records — the Windows open-residual list

As it stood at `d6a57f66`, 2,201 bytes. Every residual name, report pointer and tracking status in it is kept in the compacted list.

```text
  - Open, each with an exact test-name discharge condition:
    `HTTP-V1-WINDOWS-RUN` and `STRING-SLICE-IO-WINDOWS-RUN`
    (`docs/reports/windows-native-sockets-harness/README.md`), and
    `SOCKET-INET-WINDOWS-RUN` on the same footing — the two declared-address
    examples' MSVC rows are written and wired through
    `tests/support/windows_c.rs::run_msvc`, no such run has happened, so no
    claim is made (`docs/reports/socket-inet-addresses/README.md`).
    `SLICE-PERMISSIONS-WINDOWS-RUN` joins them (wave 27): the range-lending
    card's one native test ran the **Linux** arm only — the emitted difference
    is one pointer displacement inside an existing initialiser and the
    `windows_c` path is carried unchanged, but no Windows or Apple run was
    performed and none is claimed
    (`docs/reports/ct07-slice-permissions/README.md`).
    `WRITE-LEND-WINDOWS-RUN` is its wave-29 sibling on the same footing: the
    write-lend card's native test ran the Linux arm only, the `windows_c` path
    carried unchanged, no Windows run claimed
    (`docs/reports/slice-permissions-write-lend/README.md` §13), and wave 30's
    `CT07-WRITE-REMAP` left it unchanged. `WRITE-LEND-TASKS-WINDOWS-RUN`
    (wave 31) joins the pair for the task-route range write lease, on the
    `STRING-CONCAT-WINDOWS-RUN` precedent: the native test ran the Linux arm,
    macOS compiles the same test file, no MSVC run has happened and none is
    claimed (`docs/reports/write-lend-tasks/README.md` §9). `WRITE-LEND-WINDOWS-RUN`
    and `WRITE-LEND-TASKS-WINDOWS-RUN` were the two members of this family that
    `STD-MODULES-O`'s roll-up did not mention and that reached no tracker at
    all; both are **registered in `named_followups`** as of `DOCS-COMPACT-6`
    (wave 34). `OWNED-FIELD-WINDOWS-RUN` (wave 39, `AGGREGATE-OWNED-FIELD` §7)
    joins on the same footing by the wave-39 close's decision:
    `examples/records/owned_field.qcore` ran under ASan/UBSan/LSan on the
    Linux arm only, no Windows or Apple run was performed and none is claimed;
    named here and not registered, like `SLICE-PERMISSIONS-WINDOWS-RUN`
    above, until `PLATFORM-RESIDUAL-FAMILY-PARTIALLY-TRACKED` is decided.
```

---

Nine blocks, **47,344 bytes** of verbatim checkpoint text.
