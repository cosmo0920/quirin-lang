# Source Routes rules

Current rule text, routed by the [rule router](README.md), which says what to
read. Rule numbers and anchors are stable; backticked paths are repository-relative.

<a id="constraint-13"></a>

<!-- preserved:constraint-13:start -->
13. Concurrency is opt-in: a file declares `#import std.concurrency;`, the
    context is unavailable by default, and imports do not leak across
    compilation inputs (`Q014`/`Q015` refuse before checking each file). The
    task profile keeps its separate `concurrency checked;` marker. Runtime
    `#import` edges preserve this file-local boundary for every capability;
    imported extern declarations do not grant callers `std.extern`.
    `core-concurrency-prelude`.
<!-- preserved:constraint-13:end -->

<a id="constraint-14"></a>

<!-- preserved:constraint-14:start -->
14. New keywords are a recorded compatibility hazard: every candidate is
    **censused against every fixture** before it is reserved, and the census is
    run rather than assumed. The word set itself is
    `qcore_frontend::frontend::is_reserved_source_word` — **113** words at this
    writing, which is the one place to count them, and the count is
    **case-sensitive**: `FULL` is a reserved source word beside `zero`, `one`,
    `affine`, `relevant`, `many` and `inf`, and a lowercase-only extraction of
    the `matches!` arms misses it and reads 112
    (`CONSTRAINT-14-WORD-COUNT-CASE-FILTERED`, measured at the wave-40 close) —
    and each arm carries its own
    card's census in a source comment, so the *per-card* ledger this constraint
    used to keep is redundant with a more authoritative copy. That ledger, card
    by card, is a checkpoint in the wave-40 record; what stands here is the rule,
    the decisions a refactor could undo, and the method.

    **Decisions that must survive refactors.**
    * **Rejected spellings stay rejected, and each was rejected on evidence.**
      `stream` (a contract `extern fn` parameter name, shielded only by the
      following `:`) and `socket` (it would name a transport the route
      deliberately makes explicit at creation), from `QUIRIN-SOCKET-V1`;
      `record` from `AGGREGATES-V1`, because
      `docs/reports/remaining-v07/probes/d02_boundary_record.qcore` is the
      single line `record R { }` and two archived reports pin it to `Q000`, so
      claiming the word would falsify a frozen archive (constraint 21);
      `struct`, census-clean and still not taken, the language already having a
      declaration head for a named type; and `Buffer` as an owned field type
      (`AGGREGATE-OWNED-FIELD`), the task route's word for a different resource,
      so `type R { f: Buffer }` refuses at `Q051` naming `AGGREGATE-FIELD-TYPES`.
      All five are absent from `is_reserved_source_word` and a census can check
      that mechanically.
    * **`in` is deliberately not reserved.** The census found it as a genuine
      reference inside the pinned contract corpus
      (`ensures normal resource_borrowed(in);`) — the shape that kept `len` out
      of the global list — so `LOOP-V1` took a narrow contextual rule in
      `referenced_names` instead. **Do not promote it to the global set.**
    * **Rename the syntax rather than edit the pin.** `work` → `effort`
      (T08-EPOCH-SRC, rather than editing the pinned golden surface),
      `buffer` → `bytes`, `record` → `type`: where a candidate spelling would
      move a pinned expectation, the card changes its own syntax. This is
      invariant 5 applied at the lexer.
    * **Contextual words stay contextual.** `error` is accepted as a logger
      level **only after `log`** and was not removed from the global set;
      `unroll` and `where` are contextual because a `[| … |]` clause list
      contributes no referenced name, the statement head takes **two** tokens
      (`[|` *followed by* `#`), and `#` is still not an identifier character, so
      the scoped-name premise behind `c#0` survives and
      `quant[| n where n := 5 |]` still reads; `lend` is contextual in three
      positions (the `lend(` call shape, before `<Buffer>` in a task parameter
      type, and as a capture's first word), so `let lend = 5;` is still a
      program; and `borrow_utf8`/`qcore_buffer_write`/`qcore_buffer_read` select
      their form by **arity** rather than by a new word.
    * **Five name-resolution exclusions, and one deliberate non-exclusion.**
      `NATIVE-LIBRARY-SOURCE-BINDING` excluded binding spans from both
      whole-token-stream route selectors (`module_imports::concurrency_span`,
      `source_tasks::has_source_tasks`), so a library named `spawn`, `join` or
      one of the four buffer/payload intrinsics cannot select a route from its
      own spelling — **do not remove that exclusion**. `SOCKET-INET-ADDRESSES`
      excluded `source_socket::address_host_span`, deliberately leaving the
      **port** outside it so a port name stays a genuine reference. And
      `AGGREGATES-V1` **measured that exclusion and deliberately did not copy
      it** (`AGGREGATE-SELECTOR-WORDS`): a record naming a selector word refuses
      `Q015` declared and `Q000` constructed, both fail-closed, and copying the
      exclusion would retire the `Q015` half while leaving the `Q000` half —
      buying a type that declares and cannot be constructed.
    * `MUTEX-SHARE-CROSS` claims a **position** and no word: both clause words
      of `thread scope NAME share PLACE under MUTEX { … }` were already
      reserved, so no exclusion was carved out of `referenced_names` and both
      clause **names** stay genuine references (an unbound one refuses at
      `Q013`), measured rather than asserted.
    * `W48-QR-SCALAR-TYPES` claims a **position** and no word, and adds a
      fourth name-resolution exclusion: in `lexical_references`, a word that is
      a scalar type name (`bool`, `i8` … `usize`) immediately after the
      already-reserved `as` is not collected, so the conversion `EXPR as TYPE`
      resolves. Both sides are contextual — neither `as` nor any type name
      became a global word — and a `with lock m as g` binder is a different
      word in the same position. The census
      (`docs/reports/w48-qr-scalar-types/census_spelling.py`) found **0** `as`
      followed by a scalar type name across the 625 tracked
      `.qr`/`.qv`/`.qrlib` files. A conversion written on a route that does not
      admit it now refuses at that route's own reader rather than at `Q013`.
    * `W48-QR-SCALAR-TYPES-R2` claims a **position** and no word, and adds a
      fifth exclusion: `ArithmeticError` immediately followed by `::` is not
      collected, so `raise ArithmeticError::VARIANT;` names the builtin typed
      error that no declaration introduces. `match`, `raise`, `Result`, `value`
      and `error` are the task profile's and were already handled. The census
      (`docs/reports/w48-qr-scalar-types-r2/census_error_name.py`) found
      `ArithmeticError` in no tracked `.qr`/`.qv`/`.qrlib`/`.qcore`/`.qc` file
      but the card's own example, and the qualified shape in none.

    **Method, twice corrected by its own findings.** A wave's entry is settled
    by **measuring the seam**, not by reading the cards' reports: the wave-37
    and wave-39 closes each ran `git diff --name-only <base> <tip> --
    crates/qcore-frontend/src/` and attributed every moved file with
    `git log --name-only` (`docs/reports/{wave37-close,wave39-close}/README.md`).
    The reason is that a wave-by-wave roll call of cards that *do not* engage
    invites a false universal: one such list once contained a card that had run
    a census (`CONSTRAINT-14-WAVE-32-CENSUS-INCOMPLETE`), which is the same
    shape as the Q-space note's own miscount, in the same wave, in the
    neighbouring paragraph. Waves 28 through 37 each closed at zero reserved
    words — the longest such run — and wave 39's `CT42-CELL-ROUTE` ended it
    deliberately with `disjoint`, stated as such.

    `remaining-t03-f`, `remaining-t08-epoch-src`, `extern-call-v1`,
    `wave17-std-modules`, `remaining-extern-modes`,
    `native-library-source-binding`, `quirin-socket-v1`, `loop-v1`,
    `socket-event-compose`, `string-concat`, `socket-inet-addresses`,
    `loop-unroll-budget`, `http-v1-computed-length`, `aggregates-v1`,
    `quant-region-source`, `ct07-slice-permissions`, `ct23-credit-error-path`,
    `region-credit-joins`, `region-joins-duties`,
    `slice-permissions-write-lend`, `write-lend-tasks`,
    `mutex-share-cross`, `write-lend-tasks-exercise`, `aggregate-owned-field`,
    `ct42-cell-route`, `loops-on-composed-routes`, `w48-qr-scalar-types`,
    `w48-qr-scalar-types-r2`.
<!-- preserved:constraint-14:end -->

<a id="constraint-15"></a>

<!-- preserved:constraint-15:start -->
15. Lock ranks and protection growth are **opt-in** disciplines: unranked
    modules stay byte-identical, and a rank is a declared position, not an
    assumption. `remaining-ct33-ranks`, `remaining-ct34-growth`.
<!-- preserved:constraint-15:end -->

<a id="constraint-15a"></a>

<!-- preserved:constraint-15a:start -->
15a. **A source line in a composed task module stands in the parent's body
    or in a spawned worker's, and the two are compared differently.** A
    parent line's position is determined because every *trace* write an
    emitted task program performs is performed by the parent's thread
    (workers buffer events privately; the parent replays them at collect),
    so a parent-only program's streams are compared **byte for byte** and
    every published `examples/threads` program is of that kind. A
    **worker**-body line is written by that worker's own thread, and placing
    one **declares** `qcore.obs-reordering.v1`: that program's streams are
    then compared window by window and emitter by emitter, where a window
    boundary is a point at which no worker is live and an emitter is the
    thread that wrote the bytes — the opposite of F02's causal attribution,
    and right for the opposite reason. The strict comparison is that
    relation's own degenerate case (`strict_iff_byte_equality`), so nothing
    was replaced. Do **not** weaken the two premises the declaration rests
    on: two emitters of one window may not share a line's bytes
    (`AmbiguousAlphabet` refuses the declaration, never a multiset
    comparison), and a worker that writes declares `io` in its **own**
    effect row. `wave21-compose-concurrency`, `wave22-obs-reordering`.
<!-- preserved:constraint-15a:end -->

<a id="constraint-15b"></a>

<!-- preserved:constraint-15b:start -->
15b. **A route reads its statement tokens from the parsed program's
    post-desugaring stream, never by re-lexing source bytes,** and no reader
    may attribute a diagnostic to a synthesized token's span (synthesized
    tokens are zero-width). The last raw re-lexing site (the sequential
    driver's `statement_tokens`) made `qcore_acquire` through a pipe refuse
    while naming the hand-written form; it now uses the same span filter the
    extern, std-io and task readers have used since their own cards.
    `wave22-pipe-intrinsic-stage`.
<!-- preserved:constraint-15b:end -->

<a id="amendments-after-the-migration-checkpoint"></a>

## Amendments after the 2026-09-19 migration checkpoint

The blocks above are the migration's **preserved checkpoint text**, which
`docs/reports/docs-structure-compression/verify.py` reconstructs byte for byte;
amendments therefore land here, beside the frozen text rather than inside it
(standing constraint 21's own discipline). They are part of the rule.

<a id="constraint-14-amendments"></a>

### 14 — census entry, card `OWNED-FIELD-ROUTE-REACH` (wave 41)

Reserves **+0 words and +0 tokens** — the copy bind `let NAME = NAME;` is four
things the lexer already produces — and its census is the one that **found a
collision and kept it**. The claimed token shape occurs **29 times** across the
535 `.qcore`/`.qc` files of the tree, every occurrence `source.socket.v1`'s
creation form (`let listener = tcp;`, `let client = udp;`) in the socket, HTTP,
event and CT42 families. The two constructs are separated by **route selection
and by nothing else**, so the card deliberately did *not* add the shape to
`source_std_buffer::buffer_use` — the function route selection and the composed
string route both read this route's statements through — and pinned a socket
program's admission and the composed route's `Q068` rather than arguing them
(`COPY-BIND-SOCKET-SHAPE-COLLISION`). `: string` at a field type and a
record-typed field are censused at **zero** each, which is why giving those two
spellings their own by-name refusals changes no program's verdict.

Add `owned-field-route-reach` to the rule's closing evidence list.

### 14 — census entry, card `W48-QR-SCALAR-TYPES-R3` (wave 48)

Reserves **+0 words and +0 tokens**, and claims four **positions** of words
already there. On `source.buffer.v1`, `let NAME = OPERAND as TYPE;` is
card 0's `as` rule (a scalar type name after the already-reserved `as` is not
a reference) reached by a second route, and — like the copy bind — it is **not**
a `buffer_use`, so it selects no route and the composed string route never
lowers one; `true`/`false` are read as a field value only at a field declared
`bool`. On the extern route, a value statement is four token shapes the lexer
already produced (`let NAME = A OP B;`, `A CMP B`, `- NAME`, `A as TYPE`),
tried after both call shapes and before the literal binding so no call or
literal loses its reading; `throws<ArithmeticError>` in an extern or composed
entry's row is the scalar route's contextual spelling. The census is the one
card 0 shipped (`docs/reports/w48-qr-scalar-types/census_spelling.py`): `as`
before a scalar type name occurs in the tracked `.qr`/`.qv`/`.qrlib` files only
in the three card examples, and the whole-tree fixture freeze
(`docs/reports/w48-qr-scalar-types-r3/artifacts/fixtures-compare.txt`) is the
census of the value-statement and typed-field shapes: no tracked file changes
its reading, and the only admissions that move are `remaining-v04`'s P08 and
P10, refused by the extern-declaration read — a declaration check, not a word.

Add `w48-qr-scalar-types-r3` to the rule's closing evidence list.

### 14 — CT implementation source forms

CT24 reuses the parsed error-region and spawn-success-arm forms. CT10 uses
contextual `bits<…>`/`field<…>` in record declarations and existing task
captures. Neither adds a reserved word. CT10 selects only explicit layouts
with direct field-write captures; other routes retain their existing rules.
See the [CT10 census](../reports/ct10-source-layout/README.md) and
[CT24 scope](../reports/ct24-cleanup-task-composition/README.md).

### SOURCE-SCALAR-CALLS-V1 — scalar source functions and explicit output

Adds zero reserved words/tokens. The selector follows existing specialized
profiles; function-style `print(` selects it before the legacy standard-IO
route. The selector must preserve sequential resource intrinsics, literal-only
legacy programs and unsupported control-flow diagnostic pins. Read retained
tokens, check every definition, and never treat a closed entry observation as
proof for all possible function arguments. `print(EXPR);` is this route's one
output statement: it requires the same file's `#import std.io;`, prints one
evaluated i32 plus a newline to stdout, and does not require `std.fmt`. An absent
effect row infers direct and transitive `io`; `effects {io}` permits it and
explicit `effects {}` refuses it. The emitter contributes `c_program` fragments
and an entry body, and one shared assembly emits exactly one `main`; common IO
fragments retain their keys and deduplicate on merge. A print-free program and
an arithmetic-fault exit are silent. This emission interface is not general
cross-profile call composition; resource composition and non-IO effects require
another checked extension.

### RESOURCE-HELPER-FUNCTIONS — bounded resource calls

`source.resource-calls.v1` carries extent-aware `bytes` through direct acyclic
calls; task-route `Buffer` remains a distinct resource. Read retained source
tokens, consume the complete supported signature, and preserve identifier
boundaries when recognizing types. An explicit annotation is checked or
refused, never erased by lowering. Check every helper and require its
reachability from the closed entry. Propagate minimum extents and effects
through calls, discharge each instantiated call's obligations, reject
incompatible aliases and stale moves, and bound generated operations as well
as call depth. An allocating call cannot strand caller-owned resources on
failure. The emitted C retains real calls; the existing ownership and buffer
checkers validate the expanded model. These checks and native observations do
not establish universal helper arguments or end-to-end C refinement. Exact
grammar, bounds, host conditions and residuals belong to the
[resource-helper report](../reports/resource-helper-functions/README.md).
