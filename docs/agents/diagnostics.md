# Diagnostics rules

Current rule text, routed by the [rule router](README.md), which says what to
read. Rule numbers and anchors are stable; backticked paths are repository-relative.

<!-- generated:rules:start — edit docs/agents/rules/<id>.md, then run python3 scripts/gen_rule_docs.py -->

<a id="constraint-1"></a>

<!-- preserved:constraint-1:start -->
1. `docs/QCORE_SPEC_r0_3.md` §L21 is the canonical per-code ownership table. A
   **semantic pass** owns a range, `qcore_frontend::frontend::source_diagnostic`
   is the authoritative pass → code map, any crate running that pass emits from
   its range, and a code may not be reused by a second pass. Codes are stable:
   a code change is semantic, and new codes go through the §L21 process (as
   `Q624`/`Q625` and `Q626`/`Q627` did). Fixing where a diagnostic is
   *attributed* (file, line, span) is not a code change. `remaining-d01`,
   `remaining-ct33-ranks`, `remaining-ct34-growth`.
<!-- preserved:constraint-1:end -->

<a id="constraint-2"></a>

<!-- preserved:constraint-2:start -->
2. `Q020` (task route) and `Q021` (sequential route) both refuse an unenforced
   source `contract { … }` and **must stay separate** until a source-contract
   consumer lands: the task gate also refuses an empty `contract { }` and the
   sequential gate accepts it, so unifying them would change rejection
   coverage. `source.extern-call.v1` is not that consumer — it consumes
   explicitly selected *declared* contracts. §L21, `remaining-d01`,
   `extern-call-v1`.
<!-- preserved:constraint-2:end -->

<a id="constraint-3"></a>

<!-- preserved:constraint-3:start -->
3. §L21.1's reason vocabulary is implemented as `qcore_schemas::reason`, but
   **do not add a category field to `DerivationIncomplete`** without
   regenerating every task digest, provider package and witness in the same
   change. An `environment-unavailable` row must never be relabeled a source
   defect, and must never contribute to a provisional `Unsafe`.
   `remaining-reason-codes`, `remaining-d01`.

<!-- preserved:constraint-3:end -->

<!-- generated:rules:end -->

### Diagnostics namespaces

Authority is `docs/QCORE_SPEC_r0_3.md` §L21 (ownership by semantic pass) —
this table is a summary of it, not a second allocation.

**Q-space note — `Q-NAMESPACE-DECISION`, an open decision in the ice box.** The
`Q000`–`Q099` route-code space is **exhausted**: `Q077`–`Q079` (`STRING-CONCAT`)
and `Q097`–`Q099` (`LOOP-V1`) were the last two pools and are spent, so no free
range is left below `Q100`. The next allocation needs a §L21 namespace decision
rather than a range — candidates: open `Q101`/`Q102`/`Q104`+ around the legacy
three-digit bands, or take a new prefix. That decision is **iceboxed by an
integrator-recorded decision of 2026-09-12**, on the stated ground that naming
collisions happen easily, and was **re-affirmed (not resolved) on 2026-09-18 for
wave 39** in the wave-38 close's deferred decisions. It is an open decision,
**not** a rule.

**Operative consequence until then: no card allocates new `Q` codes.** A new
refusal lands as a **condition extension of an existing band**, on the
`SOCKET-EVENT-COMPOSE` / `NATIVE-LIBRARY-HEADER-DECORATION` precedent — the same
condition reached by a new route — with the widening stated in §L21 and in the
card's report rather than taken silently. A deliberate **narrowing** is stated
the same way (`REGION-JOINS-LOAN`'s `Q600`; `AGGREGATE-OWNED-FIELD`'s `Q055`,
the first deliberate lift under the icebox; `LOOPS-ON-COMPOSED-ROUTES`, whose
movements are all narrowings). Constraint 1 is unchanged: a code is still
allocated to a condition, still stable, still not reusable by a second pass.

**The tally, and why it is the fragile part.** Across waves 26–39 the
consequence ran **eighteen** times on **ten** bands, and **wave 40 added
nothing to it** — a two-card cadence wave (`STABILITY-REEVAL-4`,
`DOCS-COMPACT-8`) that moves no crate source, so neither card engages the rule
and the icebox held again without being exercised (re-affirmed, not resolved,
at the wave-40 close; the decision itself stays open). The per-instance
enumeration is a checkpoint in the wave-40 record rather than a paragraph here.
The ten bands are `Q020`–`Q021`, `Q051`–`Q057`, `Q068`–`Q079`, `Q080`–`Q089`,
`Q090`–`Q096`, `Q097`–`Q099`, `Q300`/`Q302`–`Q309`, `Q600`, `Q610`–`Q627` and
`Q630`–`Q641`; `AGGREGATE-OWNED-FIELD` also reaches the `Q100`/`Q103` row, but
by **vocabulary alone** with that condition not widened ("covers by
definition"), so it is not an eleventh band — re-derived this pass from the
enumeration and the table rows below, which is the only way to derive it.
**Nothing derives this tally mechanically** (`Q-SPACE-EXTENSION-TALLY-HAND-KEPT`),
and it has been wrong twice, both times in the direction of undercounting:
it put the total at thirteen behind a false universal about waves 32–34 that its
own `Q600` and `Q610`–`Q627` rows falsified, and it **silently missed
`CT42-CELL-ROUTE` for one merge** — a card that stated its widenings in §L21, in
its report §4.5 and in its merge message (`45a33285`) and was still uncounted
here until the wave-39 close counted it from the table rows. That is the same
shape as `CONSTRAINT-14-WAVE-32-CENSUS-INCOMPLETE`, in the same wave, in the
neighbouring constraint, and it is the argument for deriving both.

**Cards that allocate nothing and are *not* instances of the rule**, so that the
tally is not padded: `GATEWAY-C-LOCATION` and `CT24-CLEANUP-PREEMPTED`, because
fixing where a diagnostic is *attributed* is not a code change (constraint 1);
`CT42-PROTOCOL-CELL`, which reaches the footprint checker's existing `Q664`
condition from a new deriver without widening it; and **waves 33, 34 and 37**
entire, whose gateway arbiters (`C-GATEWAY-RETURN-POSITION`,
`C-GATEWAY-RETURN-CALL-W4`, `SCALAR-ROUTE-EXIT-EDGE`) move **gateway
diagnostics**, which are categories rather than `Q` codes, so the icebox does not
engage.

| Codes | Owning pass | Meaning |
|---|---|---|
| `Q000` | `NameResolution` (per file) | input file does not parse as a QCore declaration, or a standalone frontend entry point receives an unresolved runtime `#import` |
| `Q001`, `Q016` | `Index` | index/quant presentation; dimension/axis validity and quant-index type resolution |
| `Q010`–`Q015` | `NameResolution` | duplicates, unknown name, `Q014` unknown imported module and runtime lookup/path/alias/cycle/dependency-mode/bound refusals, `Q015` concurrency without its file-local import |
| `Q020`, `Q021` | `OrdinaryTypes` | unsupported ordinary construct / task-route source-contract gate; sequential whole-program contract gate. **Route-specific, not to be unified** (constraint 2). Card `WRITE-LEND-TASKS` (wave 31) widens `Q020`'s task-parameter condition and **allocates no code**: the admitted worker parameter types grow by the ungraded `lend<Buffer>` range write lease, receivable only from a `lend PLACE LOW..HIGH` capture, with the place/parameter agreement check covering the new pair exactly as it covers `borrow<Buffer>`; `lend` stays contextual (constraints 14 and 15h). Card `WRITE-LEND-TASKS-EXERCISE` (wave 36) widens the same condition again and **allocates no code**: a worker takes **at most one** `lend<Buffer>` parameter, and the byte spellings `qcore_buffer_write(LEASE, INDEX, VALUE)` / `qcore_buffer_read(NAME, INDEX)` refuse here when the named binding is the wrong kind for the position. The **arity** selects the form (the `borrow_utf8` precedent), so +0 words and +0 tokens Card `W48-QR-SCALAR-TYPES-R4` (wave 48) widens it again, **allocating no code** (spec §L21): the task parameter and worker result conditions admit the §G02t table, and the task route's value statement refuses at `Q020` for its typing conditions (literal range, mixed types, `bool`, a check-time fault, a target width with no record, a fixed-width entry result). Card `W48-QR-SCALAR-TYPES-R6` (wave 48) widens it again, **allocating no code**: a Buffer, loan or lease type may name its payload type (`own<Buffer<T>>`), refused here when the payload is malformed, a target width with no record, or of another type than a local annotation or a capture delivers. |
| `Q022`–`Q027` | `ExternCall` | admission and bridging of `source.extern-call.v1` declared extern calls; `Q022` checks each caller’s own `std.extern` import against the linked extern declaration inventory, including imported and direct multi-file inputs |
| `Q028`–`Q031` | `StandardIo` | `source.std-io.v1` print/log/flush admission behind `std.fmt`/`std.io`/`std.logger` |
| `Q032`–`Q035` | `UncheckedLinkage` | `source.extern-call.unsafe.v1` explicit unchecked linkage; scalars only, every pointer refused |
| `Q036`–`Q038` | `Pipe` | `\|>` pipeline desugaring shapes; `Q038` an operator token used where this revision gives it no meaning (`\|`, `&`, `^` reserved for bitwise) |
| `Q039`–`Q042`, `Q047`–`Q049`, `Q063`–`Q067` | `NativeLibrary` | `source.native-library.v1` in four bands: manifest load, binding and agreement plus manifest-resident waivers (`Q039`–`Q042`); manifest-bound contract slices, identity stacking, withdrawal (`Q047`–`Q049`); exported-header reading and manifest/header disagreement (`Q063`–`Q064`); the source file's own `extern library NAME;` binding — resolution refusals, a foreign manifest's symbol, the publication tripwire (`Q065`–`Q067`). Card `W48-QRLIB-LINKAGE` (wave 48) **allocates no code** and widens seven conditions for the FFI route and the generated (v2) manifest: `Q039` a v2 grammar violation (a v2 statement in v1, `include` in v2, an `unchecked fn` in v2, a missing or malformed `export`/`input`/`runtime`, a runtime on a static or none on a dynamic manifest); `Q065` an FFI file calling C that binds no library, or whose binding resolves to a v1 manifest; `Q066` an FFI callee no manifest the file bound binds; `Q041` a generated entry naming another contract module or C signature than the callee (refused, not withdrawn); `Q063` a struct position under the bounded header reader of an extern-route manifest; `Q064` a manifest header whose digest is not the `struct!` header of a struct the callee carries; `Q067` an FFI certificate missing the linkage premise of a called function. `Q047` is reached unwidened (slice totality) |
| `Q043` | `ComposedStdIo` | composed extern/std-io route's own refusals (`source.std-io.v1` × `source.extern-call.v1`) |
| `Q044`–`Q046` | `Branch` | `source.branch.v1` `if`/`else` admission: shape and arm subset, decidable condition tier (connectives, `!`, bitwise, constant bindings), custody join over survivors |
| `Q050` | `ComposedTasks` | `source.std-io-tasks.v1`: the composed task/std-io route's own refusals — a worker-body line now admissible where `qcore.obs-reordering.v1` covers it (constraint 15a) Card `W48-QR-SCALAR-TYPES-R4` widens it to `hex` of a `bool` operand, allocating no code. |
| `Q051`–`Q057` | `StandardBuffer` | `source.buffer.v1`: the owned extent-carrying byte buffer (`bytes`, `[i]`, `extent`, `drop`). Three cards land on this band and **allocate no code**, each widening stated in §L21: `AGGREGATES-V1` (`source.record.v1`'s scalar records: `Q051` record and field declarations, constructions outside the form and non-operand field values or numeric field paths; `Q052` a name at a record or field position that is not one, while a construction of an *undeclared type* refuses earlier, at `Q013`; `Q053` a field count outside `1..=32`); `SLICE-PERMISSIONS-WRITE-LEND` (wave 29, the write lend, constraint 15h: `Q051` the lend's window shapes, `Q052` a lend of anything but an owned buffer this entry acquired and a `drop` of a loan, `Q053` the lend's range, `Q054` **any refusal of the lend-extended cycle-3 checker** — the checker itself was extended and the route still inherits rather than re-implements —, `Q055` a loan at a scalar, index or rendering position); `AGGREGATE-OWNED-FIELD` (wave 39, the owned field `FIELD: bytes`, constraint 15k: **`Q055` narrowed**, the one lift, so a buffer at a field position refuses only at an `i32` field; `Q051` a malformed field-path statement and the by-name `OWNED-FIELD-LEND-WINDOW` and `OWNED-FIELD-LOOP-BODY`; `Q052` an integer, scalar or record at a `bytes` field and a custody position naming an `i32` field; `Q053` an index outside a field's extent; `Q054` the path-extended checker's vocabulary). Otherwise `Q054` and `Q055` are the route's existing rules reached unchanged, and **`Q056` is not widened** by any of them: the recheck comparing more published fields is that condition working. The per-card wording is in the wave-45 compaction checkpoint. Card `W48-QR-SCALAR-TYPES-R5` (the value statement) widens `Q051` (more than one operation, a malformed statement, a target width with no record), `Q052` (mixed operand types, `bool` arithmetic, negation or ordering) and `Q053` (a fault on known operands, naming the variant; a literal outside its operand's type), allocating no code; on the composed string route it widens `Q077` (a conversion, a value statement or a non-`i32` loop index at any depth) |
| `Q058`–`Q062`, `Q074` | `Event` | `source.event.v1` in two bands: the checked event vocabulary over the reactor/descriptor custody models, `Q061` the determinism boundary (`Q058`–`Q062`); its composition with the source line vocabulary (`print`, `log`, `flush`) and that composition's one value form, `let NAME = dispatch ready E;` (`Q074`, relocated from `Q065` at integration) |
| `Q068`–`Q073`, `Q075`–`Q079` | `StandardString` | `source.string.v1` behind `std.string`, three bands. `Q068`–`Q073` (relocated from `Q065`–`Q070` at integration) the immutable UTF-8 `{ptr, len}` view: entry shape, an undecidable slice bound, a quoted token that does not decode, a bound out of range or off a code-point boundary, a string at a position it may not cross, the declared `effects` row. `Q075`–`Q076` (`STRING-SLICE-IO`) text reaching a stream: `Q075` the line's shape, `Q076` a text operand naming no string this entry bound. `Q077`–`Q079` (`STRING-CONCAT`) **computed text** — `concat(…)`, `dec`/`hex`, `borrow_utf8`: `Q077` shape and bound refusals **including an embedded `source.buffer.v1` statement carrying that route's own message under this pass's code**, `Q078` the borrow-exclusion window with the inherited ownership checker's own refusal, `Q079` the **static UTF-8 decision** on a buffer's write history. Condition extensions, **no code allocated**: `CT07-SLICE-PERMISSIONS` makes the borrow a **range** (`borrow_utf8(NAME, LOW, HIGH)`, arity one or three): `Q077` also covers a range bound violating `0 <= low <= high <= extent` and the two-argument spelling (deliberately not `Q071`, whose condition is a bound on a *view*); **`Q078` is widened** to "a store **into the bytes** of a live loan, or a release of a buffer any live loan holds", a whole-extent loan reporting its old sentence byte for byte; `Q079` is reached **without widening**; a non-constant bound stays `Q069`/`Q013` and a buffer or string at a bound position `Q072`. `SLICE-PERMISSIONS-WRITE-LEND` (wave 29) makes a `lend(…)` in a composed string/buffer entry refuse **by name** at `Q077` (`SLICE-PERMISSIONS-COMPOSED-LEND`, constraint 15h) Card `W48-QR-SCALAR-TYPES-R6` (wave 48) **narrows** `Q077`'s R5 condition and allocates nothing: a conversion, a value statement and a loop of any integer index type are admitted on the composed route; `Q077` keeps a typed name at an `i32` position of the route and carries the buffer route's own typed refusals. |
| `Q080`–`Q089` | `StandardSocket` | `source.socket.v1`: the `std.socket` bundled module's checked TCP and UDP surface over loopback — `Q080` entry shape, `Q081` the operand rule, `Q082` the transport state machine, `Q083` the inherited custody refusal, `Q084` byte/payload/count ranges, `Q085` the **determinism boundary** (a receive the program's own committed history does not satisfy), `Q086` the crossing boundary, `Q087` the `effects` row, `Q088`/`Q089` the certificate recheck and the backend refusal. Three later cards extend the row and **allocate no code**, each stated in §L21 (constraint 1): `SOCKET-EVENT-COMPOSE` (the **reactor composition**) widens `Q083` from "the inherited ownership checker refused" to "an inherited, unedited **custody model** refused", so a reactor `L00`–`L12` diagnostic lands there carrying its own value; `SOCKET-INET-ADDRESSES` (**declared IPv4/IPv6 addresses with ports**, whose external-peer refusal is `Q085`) widens `Q084` to a **port** range; both land on `Q080`–`Q082` and `Q085`–`Q087`. `CT42-CELL-ROUTE` (wave 39) adds the **disjointness question** `disjoint NAME NAME;` (one reserved word, census-clean): `Q080` its shape, `Q081` a question naming no socket, **`Q085`** the cell derivation's two refusals (an accept with nothing queued, a connect with no listener — unreachable after the transport checker admitted the program); a question of a closed socket is the existing `Q083` then `Q082`, and the answer is `Q664` (the `Concurrency` row) |
| `Q090`–`Q096` | `StandardHttp` | `source.http.v1`: the `std.http` bundled module's checked HTTP/1.1 message subset — `Q090` entry and statement shape, `Q091` the start line, `Q092` header fields (including the required single `Host` and the `CR`/`LF` exclusion that makes the constructed framing unforgeable, so header injection is not expressible), `Q093` framing (signature condition: a declared `Content-Length` that is not the content's byte count), `Q094` a name at a message position that is not one, `Q095` the **inherited, unedited** `source.socket.v1` refusal carrying that checker's own error value, `Q096` an undeclared derived effect (`Q097`–`Q099` of the pre-assigned band returned to the pool, `HTTP-QUIRIN-V1`). Condition extensions, **no code allocated**: `HTTP-V1-COMPUTED-LENGTH` admits a **computed** header value or content (a quoted token, a text binding, a `concat(…)` of check-time-known pieces, `dec(NAME)`/`hex(NAME)`), widening `Q090` (malformed `concat`/`len`, fewer than two pieces, a payload above 4096 bytes, a selector without its operand, a rebound value name) and `Q094` (a component naming a binding of the other kind) and no gate: every component is resolved to bytes *before* `admit_field` and `admit_framing`, so `Q093`'s equality stays decided rather than trusted (constraint 15f), and the field **name** stays a written token. `CT42-CELL-ROUTE` (wave 39) **widens `Q095`**: `disjoint NAME NAME;` lowers into the transport program and the protocol-cell check is composed over it, so `Q095` also covers a refusal of the cell **derivation**, while the answer stays `Q664`; before that composition `quirin check` admitted an HTTP exchange asking about its own two ends with no judgement |
| `Q097`–`Q099` | `Loop` | `source.loop.v1`: the counted `for NAME in 0..BOUND { … }` loop, admitted on `source.buffer.v1` only — `Q097` the loop's **shape** (a header outside that form, a low end not the written-out `0`, a `break`/`continue`/`return` in the body, a body binding shadowing one the entry made), `Q098` the **bound** (not a check-time constant the lowering can compute, zero, negative, above `MAX_ITERATIONS`, or an entry above `MAX_UNROLLED_EVENTS` unrolled), `Q099` one iteration of the body is not the **identity** on custody. A separate pass on the `IF-STATEMENT` precedent: a control shape, not a capability of a bundled module. The band `HTTP-QUIRIN-V1` returned to the pool, now spent (`LOOP-V1`). Card `LOOP-UNROLL-BUDGET` widens two conditions and allocates **no code**: `Q097` also covers the statement-adjacent `[| #unroll, where #bound < N |]` clause list's **shape** and a list that licenses nothing (before a non-`for`, doubled, or dangling; an *unterminated* list stays a `Q000` parse failure), and `Q098` becomes the **three-layer budget** — an absolute ceiling of 4096 no declaration may raise, a default budget of 256 an unattributed `for` is held to (refusing *with the licensing clause list written into the message*), and a licence that is **checked, never trusted**, a declared `N` above the ceiling refusing before this loop's bound is read. The licence **erases from emission**: attributed and unattributed programs of the same admitted bound emit byte-identical C. Card `W48-QR-SCALAR-TYPES-R5` widens `Q098` and allocates no code: a `bool` bound, an `isize`/`usize` bound with no recorded target, and (the checker's re-derivation) a count that is not a value of the index type or disagrees with its bound's value; card `W48-QR-FLOAT-BUFFER-PAYLOADS` widens it again, allocating nothing, to an `f32`/`f64` bound, refused by name before its bit pattern could be read as a count |
| `Q100`, `Q103` | `Usage`, `Ownership` | usage grade; ownership. Card `AGGREGATE-OWNED-FIELD` (wave 39) grows `qcore_core::ownership`'s refusal vocabulary (`AggregateAtOwnerPosition`, `NotAnAggregate`, every variant able to name a `BASE.FIELD` path), which `Q103`'s condition covers by definition — but **no sequential program reaches it**, that route building no aggregate operation (`OWNED-FIELD-SEQUENTIAL-UNREACHED`); `Q100` is not widened, the copy refusal being the checker's `CopyOfOwned` |
| `Q200`, `Q202` | `ResourceFlow`, `CleanupLowering` | resource flow; cleanup lowering and reference interpretation |
| `Q300`, `Q302`–`Q309` | `TypedErrors`, `ErrorRegion` | `Q300` typed errors; `Q302` error region, with `Q303`–`Q309` the finer conditions the pass has allocated since Cycle 6 and `RegionTaskDiagnostic::code` already reads back — `Q303` a `quant open` naming another region, `Q304` a pack consumed twice, `Q305` a handler returning with its pack open, `Q306` a **nonempty** pack caught without being opened, `Q307` an error leaving a handler undischarged, `Q308` a cleanup action that is not total and noexcept, `Q309` one resource in both the payload and a pack. Card `QUANT-REGION-SOURCE` allocates **no code** and widens two conditions: `Q302`'s from "the error-region model refused" to that **and** `source.region.v1`'s own shape refusals, each v1 path boundary refused **by name** there (`REGION-NORMAL-EXIT`, `-MULTIPLE-CATCH`, `-MULTIPLE`, `-PROPAGATE`, `-FAILING-ACQUIRE`, `-IN-ARM`, `-HANDLER-BODY`, `-CATCH-PAYLOAD-BINDING`); and these codes are now emitted from the **driver** as well as the frontend. `REGION-CREDIT-JOINS` (wave 28), `REGION-JOINS-DUTIES` (wave 29) and `REGION-JOINS-LOAN` (wave 30) widen `Q302`'s composed-route condition a second, third and fourth time — the region on the epoch task route after every join, then its sealed pack carrying real duties, then the **adoption** `move NAME;` beside a worker's counted read share (with an adopted name the entry does not bind, and one owner taken twice through either shape, as that card's own `Q302` conditions) — constraint 15m's admitted set, each stated in §L21, each allocating nothing Card `W48-QR-SCALAR-TYPES-R4` widens `Q300` to a task value statement over a run-time value that can fault in a function whose row lacks `throws<ArithmeticError>`, allocating no code. Card `W48-QR-SCALAR-TYPES-R6` (wave 48) widens `Q300` and allocates nothing: an error variant whose payload names a typed Buffer (`V(own<Buffer<T>>)`) is refused at the declaration (`W48-QR-TYPES-ERROR-PAYLOAD`). |
| `Q400`, `Q401`, `Q403` | driver invocation | no inputs; missing/ambiguous entry; `--entry` against a library module |
| `Q500` | `CEmission` | construct the reduced C11 CFG cannot encode |
| `Q600` | `Concurrency` | scoped-task checking and the carried-over structural refusal shared by every checked concurrency profile. Condition extensions, **no code allocated**, each stated in §L21: `REGION-CREDIT-JOINS` (wave 28: a spawn or join whose span follows the entry's `error region`, the ordering derived from the checked task IR, never declared); `CT47-COPY-CAPTURE` (wave 29: the transferability judgment's refusal of a task-creation capture, attributed at the crossing construct and naming the missing transfer/share contract, where it used to fall through to the `Q500` catch-all, kept unedited as the fail-closed backstop); `REGION-JOINS-LOAN` (wave 30: a deliberate **narrowing** and no widening, discharging the composed epoch route's blanket refusal of read-loan captures, `T08-COMPOSE-LOAN`, constraint 15m); `WRITE-LEND-TASKS` (wave 31: the judgment's `TransferMode::LendWrite`, the delivered `lend<Buffer>` and `ShareProtocol::DisjointWriteLeases`, the lease's check-time bound decisions, and the ledger's `OverlappingLeases` / `LeaseOutstanding` / `InvalidLease` / `TooManyWriters` through the unedited `check_task_module`, constraint 15h); `MUTEX-SHARE-CROSS` (wave 32: `TransferKind::ProtectedResource`, the delivered `MutexShare` and `TransferMode::ShareUnderMutex`, each sharing mode requesting **one** named protocol, and the crossing's **absent** and **mismatched** protocol gates at the clause, while `unverified` is the unedited checker's own `Q611`, constraint 15o; the row landed three waves late, `MUTEX-SHARE-CROSS-INTEGRATOR-LIST-UNLANDED`); `WRITE-LEND-TASKS-EXERCISE` (wave 36: the worker-side byte access, the creator's **footprint fit**, the constant-index tier and the three authority rules, decided in the IR, constraint 15h). `CT24-CLEANUP-PREEMPTED` (wave 30) moves no code and widens no condition: `ScopeBusy` is rendered from the exit rule's own loan backstop, same code, span and refused set `W48-QR-SCALAR-TYPES-R4` (wave 48: the checker's independent re-decision of every task value statement, a raising statement's error edge checked as an exit, and `TransferKind::FixedScalar`, duplicated). `W48-QR-SCALAR-TYPES-R6` (wave 48: a lease and a byte read past the payload type's size, a byte write into a `bool` payload, a `protected isize`/`usize` with no recorded target; and a **narrowing** — the composed epoch route's blanket refusal of a fixed-width task half is discharged). |
| `Q610`–`Q627` | `Concurrency` | `CheckedMutexInvariantV1`; `Q624`/`Q625` opt-in lock ranks, `Q626`/`Q627` opt-in protection growth. Card `MUTEX-SHARE-CROSS` (wave 32) reaches `Q611` `UnguardedProtectedAccess` from a **new route** and allocates nothing: the third refusal of the declared task crossing (`thread scope NAME share PLACE under MUTEX { … }`, constraint 15o) — a child reaching the shared payload outside a critical section — is reported here under this **unedited** checker's own code rather than at `Q600`, because a protected footprint has no unguarded access at all and no rule of the mutex checker is re-implemented beside the judgment. Row landed at the wave-35 close (`MUTEX-SHARE-CROSS-INTEGRATOR-LIST-UNLANDED`) |
| `Q630`–`Q641` | `Concurrency` | `FlatEpochBoundV1` flat-epoch accounting (`source.epoch.bound.v1`), and its sibling `EpochCreditRecoveryV1` — the error path over the same epoch's credits, reached from source by `source.epoch.tasks.v1`'s `error(move\|drop\|close);` row clause (card `CT23-CREDIT-ERROR-PATH`). The same pass and the same quantities, so it **extends four of this band's conditions and takes no code of its own**: `Q630` a plan that does not describe its epoch (including a clause on the declarative route, which has no error edge), `Q635` credit **abandoned** at the creator's error edge, `Q636` a restoration larger than its grant, `Q641` the error path's arithmetic. `Q637` is deliberately **not** extended (constraint 15j) |
| `Q650`–`Q667` | `Concurrency` | `DisjointFootprintsV1` disjoint footprints. Card `CT42-CELL-ROUTE` (wave 39) reaches **`Q664`** (`SharedProtocolCell`) from source for the first time and **allocates nothing**: `source.socket.v1`'s `disjoint NAME NAME;` question (and the same question inside the reactor and HTTP compositions, whose readers share the socket reader) projects the derived protocol cells as `DeriveDescriptorAlias` events plus one `AssertDisjoint` onto the **unedited** checker, whose refusal of two descriptors over one cell is reported under its own code and this pass, attributed to the question — the two ends of one accepted connection are one cell (CT42). The checker's one edit is the derived-alias event, which stamps no `DeclaredProtocolCellIdentity`; no decision rule moved |
| `Q670`–`Q677` | `Concurrency` | `CheckedRegionTaskCreditV1` error-region task credit |
| `Q700`–`Q703` | `PluginVerification` | scope resolution, undischarged obligations, conflicting evidence, unvalidatable provider allegation |
| `QP001`–`QP016` | provider host | protocol and evidence-firewall diagnostics |
| `<plugin-id>.<local-code>` | providers | provider-owned; exact plugin ID plus one 1–128-byte ASCII local component. Libevent uses `qcore.libevent.L00`–`L12` |


<a id="amendments-after-the-migration-checkpoint"></a>

## Amendments after the 2026-09-19 migration checkpoint

The blocks still marked preserved above are the migration's reconstruction
text. `docs/reports/docs-structure-compression/verify.py` checked them at that
card's own checkpoint; it no longer passes against the tree, failing at its first
row (`AGENTS.md` → `docs/development/architecture.md`), so the markers pin
nothing mechanically (`W45-DOCS-PRESERVATION-VERIFIER-DECAYED`). Edit such a
block only to correct or compact the current rule, and move its migration text,
markers included, verbatim into a checkpoint. The namespaces section was edited
this way at wave 45 (`DOCS-COMPACT-9`): its table rows are compacted to their
current conditions, and the per-card wording is in the
[wave-45 compaction checkpoint](../development/increments/wave-45-compaction-checkpoints.md#diagnostics).
Amendments land here, carry the same authority as the rule they amend, and stay
short: `docs/QCORE_SPEC_r0_3.md` §L21 carries each condition's full text, and
this page is capped at 32 KiB.

### Wave 41 — two condition extensions on the `Q051`–`Q057` band, no code allocated

* **Card `OWNED-FIELD-ROUTE-REACH`** — the copy bind `let NAME = NAME;` lands on
  this band: a copy of a buffer, of an owning record, of a hollow record or of a
  loan refuses at `Q054` through the value half, and the shape refusals it
  supersedes were `Q051`. The wave-39 `Q054` sentence is amended accordingly (it
  said the condition carries the four listed refusals of the unedited checker;
  it now carries the copy rule's as well). `Q100` and `Q103` were re-measured and
  **neither moved**; `Q500` moves no code and widens no condition — a record type
  in a task capture refuses there by the message it already had, and a message is
  not a code (constraint 1).
* **Card `COMPOSED-WINDOW-REACH`** — `Q051` widens once more, in the **refusing**
  direction: a field path that the window's key space cannot name is refused
  rather than silently unwindowed, a second independent line behind
  `OWNED-FIELD-LEND-WINDOW` and `OWNED-FIELD-LOOP-BODY`.

**Tally, reconciled by the integrator at the wave-41 merge: twenty instances on
the same ten bands.** The preserved paragraph records eighteen through wave 40.
Each of the two cards above measured its own widening as "the nineteenth" without
seeing the other's, which is the hand-kept tally undercounting for the third
recorded time (`Q-SPACE-EXTENSION-TALLY-HAND-KEPT`; the wave-39 close corrected it
once and the wave-40 close once). The wave-41 close should **re-derive** the total
from §L21's rows rather than carry this sentence forward.

### CT implementation amendments

`CT24-CLEANUP-TASK-COMPOSITION` narrows `Q302`/`Q600` placement refusals for
the bounded spawn-success-arm cleanup; the direct epilogue ordering remains.
`CT10-SOURCE-LAYOUT` extends `Q600`'s structural condition to the bounded
field-write profile and reaches unchanged `Q661`. No code is allocated.
These are two further instances of the namespace rule, on existing bands;
the twenty-instance checkpoint above becomes **twenty-two on ten bands**.
Canonical conditions: specification §L21; scopes: [effects](effects.md) and
[concurrency](concurrency.md).

### SOURCE-SCALAR-CALLS-V1 amendment

OrdinaryTypes Q020 now also covers the scalar-call profile's admission,
type/arity, required I/O effects, recursion, bounds and closed arithmetic failures. Q021 retains the
nonempty contract refusal; no new code or pass is allocated. This is one further
instance of the namespace rule on the existing Q020/Q021 band (twenty-three
instances on the carried ten bands, not a fresh re-derivation of earlier entries).

### SOURCE-ANNOTATION-ENFORCEMENT amendment

Q020 also covers executable local annotation compatibility, unsupported
explicit local types, malformed local heads before `=`, and an owned annotation
whose operation publishes no nominal payload. Q100 still owns local usage
annotations. The sequential `qcore_acquire` operation publishes custody but
no nominal payload type; its explicit annotations refuse, including `own<T>`.
This is one further namespace instance on the existing Q020/Q021 band
(twenty-four on the carried ten bands), with no new diagnostic code.

### Resource helper diagnostic amendment

`RESOURCE-HELPER-FUNCTIONS` applies the existing conditions to the new bounded
helper route: Q020 admission/expansion/empty-contract refusal, Q021 nonempty
unenforced contracts, Q053 extents, Q054 custody/aliasing, Q056 certificate
construction/recheck and Q057 declared effects; Q100 retains usage grades.
Refusal categories are typed, not inferred from English message substrings.
Q058 remains Event-owned. This adds one condition-extension instance on the
existing ordinary/buffer bands, no band or code; the preceding historical
tallies retain their checkpoint scope. Canonical meanings are in §L21.

### CG18-FOOTPRINT-CONNECTION amendment

The footprint band `Q650`–`Q667` gains new routes to eight conditions it
already owns, and no code: `Q650` (a zero-byte transfer), `Q652`/`Q655` (a
view or byte transfer naming a retired or unknown handle), `Q653` (a byte
count past its handle), `Q659` (a view or byte transfer over a bit-field unit
or a descriptor), `Q662` (a `memcpy` or a mutex-invariant composition across
two allocations not both declared fresh), `Q665` (a composed mutex interval no
live handle derives) and `Q667` (a punned view, an overlapping `memcpy`). One
condition-extension instance on one band not counted in the carried ten; the
historical tallies above keep their checkpoint scope. Canonical text: §L21.

### Contract inclusion namespace (`QI`)

Card `CONTRACT-INHERIT` opens `QI001`–`QI020`, owned by the contract loader
(`qcore_contract::contract_inclusion`), on the `QP***` precedent. It allocates
**no `Q` route code** and does not settle `Q-NAMESPACE-DECISION`; unresolved
inclusion at the structural parser stays `Q000` (a new message, not a code).
Conditions (full text in contract specification C15): `QI001` malformed form
or inheriting declaration; `QI002` clauses after `inherit`; `QI003` target not
`crate:module`; `QI004` crate absent or not own/declared dependency; `QI005`
module not published; `QI006` include cycle; `QI007` repeated include;
`QI008` no parent in scope; `QI009` two parents in scope; `QI010` arity, type
or result disagreement; `QI011` inheritance cycle; `QI012` capture or
consumer reading differs; `QI013` inclusion without a load path; `QI014`
invalid load path, or a load-path manifest that is malformed, lists other modules than its
crate's contract directory, or pins a changed published module (card
`CONTRACT-INHERIT-ADOPTION`); `QI015` parent not inheritable; `QI016` evidence replay
differs; `QI017` crate neither `quirin-provider-*` nor (card
`CONTRACT-INHERIT-QCORE-PROVIDER-NAMING`) `qcore-provider-*`, namespace not
derived, or shared (one remainder under both prefixes included); `QI018` unqualified parent only an included module declares;
`QI019` namespace names no directly included crate; `QI020` child named like
its qualified parent.
