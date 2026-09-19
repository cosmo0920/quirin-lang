# Diagnostics rules

Current reference, routed by [AGENTS.md](../../AGENTS.md). Rule numbers are stable.
Paths in backticks are repository-relative. Read every applicable rule,
including cross-topic rules, before editing; the index is not a substitute.

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

<!-- preserved:diagnostic-namespaces:start -->
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
| `Q000` | `NameResolution` (per file) | input file does not parse as a QCore declaration |
| `Q001`, `Q016` | `Index` | index/quant presentation; dimension/axis validity and quant-index type resolution |
| `Q010`–`Q015` | `NameResolution` | duplicates, unknown name, `Q014` unknown imported module, `Q015` concurrency without its file-local import |
| `Q020`, `Q021` | `OrdinaryTypes` | unsupported ordinary construct / task-route source-contract gate; sequential whole-program contract gate. **Route-specific, not to be unified** (constraint 2). Card `WRITE-LEND-TASKS` (wave 31) widens `Q020`'s task-parameter condition and **allocates no code**: the admitted worker parameter types grow by the ungraded `lend<Buffer>` range write lease, receivable only from a `lend PLACE LOW..HIGH` capture, with the place/parameter agreement check covering the new pair exactly as it covers `borrow<Buffer>`; `lend` stays contextual (constraints 14 and 15h). Card `WRITE-LEND-TASKS-EXERCISE` (wave 36) widens the same condition again and **allocates no code**: a worker takes **at most one** `lend<Buffer>` parameter, and the byte spellings `qcore_buffer_write(LEASE, INDEX, VALUE)` / `qcore_buffer_read(NAME, INDEX)` refuse here when the named binding is the wrong kind for the position. The **arity** selects the form (the `borrow_utf8` precedent), so +0 words and +0 tokens |
| `Q022`–`Q027` | `ExternCall` | admission and bridging of `source.extern-call.v1` declared extern calls |
| `Q028`–`Q031` | `StandardIo` | `source.std-io.v1` print/log/flush admission behind `std.fmt`/`std.io`/`std.logger` |
| `Q032`–`Q035` | `UncheckedLinkage` | `source.extern-call.unsafe.v1` explicit unchecked linkage; scalars only, every pointer refused |
| `Q036`–`Q038` | `Pipe` | `\|>` pipeline desugaring shapes; `Q038` an operator token used where this revision gives it no meaning (`\|`, `&`, `^` reserved for bitwise) |
| `Q039`–`Q042`, `Q047`–`Q049`, `Q063`–`Q067` | `NativeLibrary` | `source.native-library.v1` in four bands: manifest load, binding and agreement plus manifest-resident waivers (`Q039`–`Q042`); manifest-bound contract slices, identity stacking, withdrawal (`Q047`–`Q049`); exported-header reading and manifest/header disagreement (`Q063`–`Q064`); the source file's own `extern library NAME;` binding — resolution refusals, a foreign manifest's symbol, the publication tripwire (`Q065`–`Q067`) |
| `Q043` | `ComposedStdIo` | composed extern/std-io route's own refusals (`source.std-io.v1` × `source.extern-call.v1`) |
| `Q044`–`Q046` | `Branch` | `source.branch.v1` `if`/`else` admission: shape and arm subset, decidable condition tier (connectives, `!`, bitwise, constant bindings), custody join over survivors |
| `Q050` | `ComposedTasks` | `source.std-io-tasks.v1`: the composed task/std-io route's own refusals — a worker-body line now admissible where `qcore.obs-reordering.v1` covers it (constraint 15a) |
| `Q051`–`Q057` | `StandardBuffer` | `source.buffer.v1`: the owned extent-carrying byte buffer (`bytes`, `[i]`, `extent`, `drop`). Card `AGGREGATES-V1` admits the first source-level aggregate on this route — `source.record.v1`'s `type NAME { FIELD: i32, … }`, its construction and its field read — and **allocates no code**, widening three conditions: `Q051`'s to a record declaration, a field declaration, a construction outside the form (including one supplying a field twice, an undeclared field, or not every declared field), a field value that is not one operand and a numeric field path; `Q052`'s to a name at a record or field position that is not one (a construction of an *undeclared type* refuses earlier still, at `Q013`, a type name being an ordinary reference); and `Q053`'s from an index/byte/extent range to those and a **field count** outside `1..=32`. `Q054` (leak beside a record) and `Q055` (a buffer at a field position) are the route's existing rules reached unchanged, and `Q056`'s condition is **not** widened — the recheck comparing two more published halves is that condition working. Card `SLICE-PERMISSIONS-WRITE-LEND` (wave 29) lands the **write lend** `let NAME = lend(BUFFER, LOW, HIGH);` on this route (constraint 15h carries the rules) and again allocates no code, widening five conditions: `Q051` to the lend's window shapes (a `for` beside a lend, crossed windows, an acquisition inside a window, a malformed or shadowing `lend(…)`); `Q052` to a lend of anything but an owned buffer this entry acquired, and a `drop` of a loan; `Q053` to the lend's range beside the existing ranges; `Q054` from the four listed refusals of the unedited checker to **any refusal of the lend-extended cycle-3 checker** — the checker itself was extended, in the semantic model where every other custody rule lives, and the route still inherits rather than re-implements; and `Q055` to a loan at a scalar, index or rendering position. `Q056` is once more not widened — the recheck comparing the published `lends` field for field is that condition working. Card `AGGREGATE-OWNED-FIELD` (wave 39) lands the **owned field** `FIELD: bytes` (constraint 15k carries the rules) and again allocates no code: **`Q055` is narrowed**, the one lift — a buffer at a field position refuses only at an `i32` field; `Q051` widens to a malformed field-path statement and the two by-name refusals `OWNED-FIELD-LEND-WINDOW` and `OWNED-FIELD-LOOP-BODY`; `Q052` to an integer, scalar or record at a `bytes` field and to a custody position naming an `i32` field; `Q053` to an index outside the extent a field holds; `Q054` carries the path-extended checker's vocabulary (a use after move of a path, a double drop of a record or path, a leak naming `RECORD.FIELD`, `AggregateAtOwnerPosition`); `Q056` is not widened — the recheck comparing the published moves and field releases is that condition working |
| `Q058`–`Q062`, `Q074` | `Event` | `source.event.v1` in two bands: the checked event vocabulary over the reactor/descriptor custody models, `Q061` the determinism boundary (`Q058`–`Q062`); its composition with the source line vocabulary (`print`, `log`, `flush`) and that composition's one value form, `let NAME = dispatch ready E;` (`Q074`, relocated from `Q065` at integration) |
| `Q068`–`Q073`, `Q075`–`Q079` | `StandardString` | `source.string.v1` behind `std.string`, three bands. `Q068`–`Q073` (relocated from `Q065`–`Q070` at integration) the immutable UTF-8 `{ptr, len}` view: entry shape, an undecidable slice bound, a quoted token that does not decode, a bound out of range or off a code-point boundary, a string at a position it may not cross, the declared `effects` row. `Q075`–`Q076` (`STRING-SLICE-IO`, which returned `Q077`–`Q079` to the pool) text reaching a stream: `Q075` the line's shape, `Q076` a text operand naming no string this entry bound. `Q077`–`Q079` (`STRING-CONCAT`, reassigned to this pass) **computed text** — `concat(…)`, `dec`/`hex`, `borrow_utf8`: `Q077` shape and bound refusals **including an embedded `source.buffer.v1` statement carrying that route's own message under this pass's code**, `Q078` the borrow-exclusion window with the inherited ownership checker's own refusal, `Q079` the **static UTF-8 decision** on a buffer's write history. Card `CT07-SLICE-PERMISSIONS` makes the borrow lend a **range** (`borrow_utf8(NAME, LOW, HIGH)`, arity one or three) and allocates **no code**: `Q077` also covers a range bound violating `0 <= low <= high <= extent` and the two-argument spelling (deliberately not `Q071`, whose condition is a bound on a *view* decided against `len(s)`); **`Q078`'s condition is widened** from "a store into or a release of a buffer inside a live borrow's window" to "a store **into the bytes** of a live loan, or a release of a buffer any live loan holds", a whole-extent loan reporting the sentence it always reported byte for byte; and `Q079` is reached **without widening at all**, because that condition was always *the bytes this borrow would view* — so a range whose end splits a code point refuses there by the rule already written. A bound that is not a check-time constant stays `Q069`/`Q013` and a buffer or string at a bound position stays `Q072`. `SLICE-PERMISSIONS-WRITE-LEND` (wave 29) widens `Q077` once more: a `lend(…)` in a composed string/buffer entry refuses **by name** until a cross-model overlap rule with `borrow_utf8`'s read loans exists (`SLICE-PERMISSIONS-COMPOSED-LEND`, constraint 15h) |
| `Q080`–`Q089` | `StandardSocket` | `source.socket.v1`: the `std.socket` bundled module's checked TCP and UDP surface over loopback — `Q080` entry shape, `Q081` the operand rule, `Q082` the transport state machine, `Q083` the inherited custody refusal, `Q084` byte/payload/count ranges, `Q085` the **determinism boundary** (a receive the program's own committed history does not satisfy), `Q086` the crossing boundary, `Q087` the `effects` row, `Q088`/`Q089` the certificate recheck and the backend refusal. Band assigned by the integrator before the card, with `Q075`–`Q079` held for a sibling card. Two later cards extend the row and **allocate no code**, landing on `Q080`–`Q082` and `Q085`–`Q087`: `SOCKET-EVENT-COMPOSE`, the **reactor composition** (a checked socket registered with `source.event.v1`'s reactor), and `SOCKET-INET-ADDRESSES`, **declared IPv4/IPv6 addresses with ports** (`bind S 127.0.0.1:8080;`, `connect C [::1]:8080;`, `sendto U ADDRESS […];`), whose external-peer refusal is `Q085`. Each **widens one condition**, stated here and in §L21 (constraint 1): `Q083`'s from "the inherited ownership checker refused" to "an inherited, unedited **custody model** refused", so a reactor `L00`–`L12` diagnostic lands there carrying its own value; and `Q084`'s from a byte/payload-length/receive-count range to those and a **port** range. Card `CT42-CELL-ROUTE` (wave 39) adds the **disjointness question** `disjoint NAME NAME;` — one reserved word, census-clean — and again allocates nothing, widening three conditions: `Q080`'s to the question's shape (not exactly two names), `Q081`'s to a question naming no socket, and **`Q085`'s** to the cell derivation's two refusals (an accept with nothing queued, a connect the resolution gives no listener — the determinism boundary's own conditions, reached from the deriver and unreachable after the transport checker admitted the program); a question of a closed socket is the existing `Q083` then `Q082`, and the question itself is answered at `Q664` (the `Concurrency` row). Per-condition mapping in §L21 |
| `Q090`–`Q096` | `StandardHttp` | `source.http.v1`: the `std.http` bundled module's checked HTTP/1.1 message subset — `Q090` entry and statement shape, `Q091` the start line, `Q092` header fields (including the required single `Host` and the `CR`/`LF` exclusion that makes the constructed framing unforgeable, so header injection is not expressible), `Q093` framing (signature condition: a declared `Content-Length` that is not the content's byte count), `Q094` a name at a message position that is not one, `Q095` the **inherited, unedited** `source.socket.v1` refusal carrying that checker's own error value, `Q096` an undeclared derived effect. Band pre-assigned as `Q090`–`Q099`; `Q097`–`Q099` unused and returned to the pool (`HTTP-QUIRIN-V1`). Card `HTTP-V1-COMPUTED-LENGTH` admits a **computed** header value or content — a quoted token, a name this entry bound to text, a `concat(…)` of check-time-known pieces, or `dec(NAME)`/`hex(NAME)` — allocating **no code** and widening no gate, since every component is resolved to bytes *before* `admit_field` and `admit_framing` run and `Q093`'s equality stays decided rather than trusted (constraint 15f). It widens `Q090`'s condition (a malformed `concat`/`len`, a composition of fewer than two pieces, a composed payload above the 4096-byte bound, a selector without its operand, a value name rebound) and `Q094`'s (a component, piece or `len` operand naming a binding of the other kind). The field **name** stays a written token. Card `CT42-CELL-ROUTE` (wave 39) **widens `Q095`'s condition** and allocates nothing: the HTTP reader hands every statement it does not spell to the socket reader, so `disjoint NAME NAME;` lowers into the transport program, and the driver now composes the protocol-cell check over that program too — `Q095` covers a refusal of the cell **derivation** (unreachable after the transport checker admitted the program) beside the transport checker's own, while the question's answer stays `Q664`. Before that composition `quirin check` **admitted** an HTTP exchange asking about its own two ends with no judgement, measured and closed in the same card |
| `Q097`–`Q099` | `Loop` | `source.loop.v1`: the counted `for NAME in 0..BOUND { … }` loop, admitted on `source.buffer.v1` only — `Q097` the loop's **shape** (a header outside that form, a low end not the written-out `0`, a `break`/`continue`/`return` in the body, a body binding shadowing one the entry made), `Q098` the **bound** (not a check-time constant the lowering can compute, zero, negative, above `MAX_ITERATIONS`, or an entry above `MAX_UNROLLED_EVENTS` unrolled), `Q099` one iteration of the body is not the **identity** on custody. A separate pass on the `IF-STATEMENT` precedent: a control shape, not a capability of a bundled module. The band `HTTP-QUIRIN-V1` returned to the pool, now spent (`LOOP-V1`). Card `LOOP-UNROLL-BUDGET` widens two conditions and allocates **no code**: `Q097` also covers the statement-adjacent `[| #unroll, where #bound < N |]` clause list's **shape** and a list that licenses nothing (before a non-`for`, doubled, or dangling; an *unterminated* list stays a `Q000` parse failure), and `Q098` becomes the **three-layer budget** — an absolute ceiling of 4096 no declaration may raise, a default budget of 256 an unattributed `for` is held to (refusing *with the licensing clause list written into the message*), and a licence that is **checked, never trusted**, a declared `N` above the ceiling refusing before this loop's bound is read. The licence **erases from emission**: attributed and unattributed programs of the same admitted bound emit byte-identical C |
| `Q100`, `Q103` | `Usage`, `Ownership` | usage grade; ownership. Card `AGGREGATE-OWNED-FIELD` (wave 39) grows `qcore_core::ownership`'s refusal vocabulary (`AggregateAtOwnerPosition`, `NotAnAggregate`, every variant able to name a `BASE.FIELD` path), which `Q103`'s condition covers by definition — but **no sequential program reaches it**, that route building no aggregate operation (`OWNED-FIELD-SEQUENTIAL-UNREACHED`); `Q100` is not widened, the copy refusal being the checker's `CopyOfOwned` |
| `Q200`, `Q202` | `ResourceFlow`, `CleanupLowering` | resource flow; cleanup lowering and reference interpretation |
| `Q300`, `Q302`–`Q309` | `TypedErrors`, `ErrorRegion` | `Q300` typed errors; `Q302` error region, with `Q303`–`Q309` the finer conditions the pass has allocated since Cycle 6 and `RegionTaskDiagnostic::code` already reads back — `Q303` a `quant open` naming another region, `Q304` a pack consumed twice, `Q305` a handler returning with its pack open, `Q306` a **nonempty** pack caught without being opened, `Q307` an error leaving a handler undischarged, `Q308` a cleanup action that is not total and noexcept, `Q309` one resource in both the payload and a pack. Card `QUANT-REGION-SOURCE` allocates **no code** and widens two conditions: `Q302`'s from "the error-region model refused" to that **and** `source.region.v1`'s own shape refusals, each v1 path boundary refused **by name** there (`REGION-NORMAL-EXIT`, `-MULTIPLE-CATCH`, `-MULTIPLE`, `-PROPAGATE`, `-FAILING-ACQUIRE`, `-IN-ARM`, `-HANDLER-BODY`, `-CATCH-PAYLOAD-BINDING`); and these codes are now emitted from the **driver** as well as the frontend. `REGION-CREDIT-JOINS` (wave 28), `REGION-JOINS-DUTIES` (wave 29) and `REGION-JOINS-LOAN` (wave 30) widen `Q302`'s composed-route condition a second, third and fourth time — the region on the epoch task route after every join, then its sealed pack carrying real duties, then the **adoption** `move NAME;` beside a worker's counted read share (with an adopted name the entry does not bind, and one owner taken twice through either shape, as that card's own `Q302` conditions) — constraint 15m's admitted set, each stated in §L21, each allocating nothing |
| `Q400`, `Q401`, `Q403` | driver invocation | no inputs; missing/ambiguous entry; `--entry` against a library module |
| `Q500` | `CEmission` | construct the reduced C11 CFG cannot encode |
| `Q600` | `Concurrency` | scoped-task checking and the carried-over structural refusal shared by every checked concurrency profile. Five later cards move this row and **allocate nothing**, each stated in §L21: `REGION-CREDIT-JOINS` (wave 28) widens the composed-route reconciliation condition — a spawn or join whose span follows the entry's `error region` is refused, the ordering derived from the checked task IR and never declared; `CT47-COPY-CAPTURE` (wave 29) widens the boundary condition — the transferability judgment's refusal of a task-creation capture, attributed at the crossing construct and naming the missing transfer/share contract, previously falling through to the `Q500` executable-subset catch-all (kept unedited as the fail-closed backstop); `REGION-JOINS-LOAN` (wave 30) **narrows** one condition deliberately and widens none, discharging the composed epoch route's blanket refusal of read-loan captures (`T08-COMPOSE-LOAN`, constraint 15m); and `WRITE-LEND-TASKS` (wave 31) widens the boundary and scoped-task conditions three ways on the `CT47-COPY-CAPTURE` precedent — the judgment's grown vocabulary (`TransferMode::LendWrite`, the delivered `lend<Buffer>`, `ShareProtocol::DisjointWriteLeases`), the lease's check-time bound decisions (a varying bound, an empty range, a range past `BUFFER_PAYLOAD_BYTES`), and the ledger's own `OverlappingLeases` / `LeaseOutstanding` / `InvalidLease` / `TooManyWriters` arriving through the unedited `check_task_module` (constraint 15h). `WRITE-LEND-TASKS-EXERCISE` (wave 36) widens the scoped-task condition a fourth time and allocates nothing: the worker-side byte access — the creator's **footprint fit** (the bound a worker's body states against the lease that creation supplies), the constant-index tier (an unresolvable index is not provably inside, a negative one and one past `BUFFER_PAYLOAD_BYTES` refuse beside it), and the three authority rules (no owner byte write, no read-share byte view, the owner's byte read on the whole-payload read's own gate) — all decided in the IR, constraint 15h. `MUTEX-SHARE-CROSS` (wave 32) is the fifth, widening `Q600`'s boundary condition and the `Q610`–`Q627` sub-band's route and allocating nothing, on the same precedent: the judgment's vocabulary grows again (`TransferKind::ProtectedResource` and the delivered `MutexShare`, `TransferMode::ShareUnderMutex`, each sharing mode now requesting **one** named protocol), and the crossing's two protocol gates — **absent** (the `under` name is not a mutex generation of this module) and **mismatched** (it guards some other resource) — land at `Q600` at the clause, while the third refusal, `unverified`, is the unedited checker's own `Q611` (constraint 15o). This row's entry is the wave-35 close's, three waves late; the authority it summarises, spec §L21, has carried the widening since the card (`MUTEX-SHARE-CROSS-INTEGRATOR-LIST-UNLANDED`). `CT24-CLEANUP-PREEMPTED` (wave 30) moves no code and widens no condition: the `ScopeBusy` refusal is rendered from the exit rule's own loan backstop, same code, same span, same refused set |
| `Q610`–`Q627` | `Concurrency` | `CheckedMutexInvariantV1`; `Q624`/`Q625` opt-in lock ranks, `Q626`/`Q627` opt-in protection growth. Card `MUTEX-SHARE-CROSS` (wave 32) reaches `Q611` `UnguardedProtectedAccess` from a **new route** and allocates nothing: the third refusal of the declared task crossing (`thread scope NAME share PLACE under MUTEX { … }`, constraint 15o) — a child reaching the shared payload outside a critical section — is reported here under this **unedited** checker's own code rather than at `Q600`, because a protected footprint has no unguarded access at all and no rule of the mutex checker is re-implemented beside the judgment. Row landed at the wave-35 close (`MUTEX-SHARE-CROSS-INTEGRATOR-LIST-UNLANDED`) |
| `Q630`–`Q641` | `Concurrency` | `FlatEpochBoundV1` flat-epoch accounting (`source.epoch.bound.v1`), and its sibling `EpochCreditRecoveryV1` — the error path over the same epoch's credits, reached from source by `source.epoch.tasks.v1`'s `error(move\|drop\|close);` row clause (card `CT23-CREDIT-ERROR-PATH`). The same pass and the same quantities, so it **extends four of this band's conditions and takes no code of its own**: `Q630` a plan that does not describe its epoch (including a clause on the declarative route, which has no error edge), `Q635` credit **abandoned** at the creator's error edge, `Q636` a restoration larger than its grant, `Q641` the error path's arithmetic. `Q637` is deliberately **not** extended (constraint 15j) |
| `Q650`–`Q667` | `Concurrency` | `DisjointFootprintsV1` disjoint footprints. Card `CT42-CELL-ROUTE` (wave 39) reaches **`Q664`** (`SharedProtocolCell`) from source for the first time and **allocates nothing**: `source.socket.v1`'s `disjoint NAME NAME;` question (and the same question inside the reactor and HTTP compositions, whose readers share the socket reader) projects the derived protocol cells as `DeriveDescriptorAlias` events plus one `AssertDisjoint` onto the **unedited** checker, whose refusal of two descriptors over one cell is reported under its own code and this pass, attributed to the question — the two ends of one accepted connection are one cell (CT42). The checker's one edit is the derived-alias event, which stamps no `DeclaredProtocolCellIdentity`; no decision rule moved |
| `Q670`–`Q677` | `Concurrency` | `CheckedRegionTaskCreditV1` error-region task credit |
| `Q700`–`Q703` | `PluginVerification` | scope resolution, undischarged obligations, conflicting evidence, unvalidatable provider allegation |
| `QP001`–`QP016` | provider host | protocol and evidence-firewall diagnostics |
| `<plugin-id>.<local-code>` | providers | provider-owned; exact plugin ID plus one 1–128-byte ASCII local component. Libevent uses `qcore.libevent.L00`–`L12` |

<!-- preserved:diagnostic-namespaces:end -->

<a id="amendments-after-the-migration-checkpoint"></a>

## Amendments after the 2026-09-19 migration checkpoint

The preserved blocks above are the migration's frozen reconstruction, pinned by
`docs/reports/docs-structure-compression/verify.py` and never edited in place
(constraint 21). Amendments land here, carry the same authority as the rule they
amend, and stay short: `docs/QCORE_SPEC_r0_3.md` §L21 carries each condition's
full text, and this page is capped at 32 KiB.

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
