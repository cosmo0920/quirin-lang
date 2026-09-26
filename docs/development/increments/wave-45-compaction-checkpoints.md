# Wave 45 compaction checkpoints: rule pages (`DOCS-COMPACT-9`)

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md) · [Status checkpoint](wave-45-compaction-status.md) · [Wave 45 records](wave-45-continuation.md)

> **Preserved checkpoint text. These blocks are not a current authority.**
> They are the verbatim text `DOCS-COMPACT-9` compacted out of the rule topic pages at
> base `8c1f15eb`, kept so that nothing the compaction removed is lost. Each
> block names its base lines; `git show 8c1f15eb:<file>` reproduces it byte for
> byte. Where a block and the current page disagree, **the current page wins**;
> where a block states something the card measured to be false, the correction
> is in [the card report](../../reports/docs-compact-9/README.md) and is
> deliberately not applied here. Relative links inside a block keep their
> original base, the page they were written in.

<a id="gateway"></a>

## `docs/agents/gateway.md`

Current page: [../../agents/gateway.md](../../agents/gateway.md).

### 1. Constraint 5c

`docs/agents/gateway.md` lines 147–230 at `8c1f15eb`, 6,109 bytes. The whole preserved block, markers included. The rule keeps its operative text; the line-number history, superseded ownership sentence and integrator clauses moved. Corrected in the rule: 28 do-while leaves on the baseline (32 at the re-pin).

~~~~~text
<!-- preserved:constraint-5c:start -->
5c. **A constant `do … while (0)` is sequential composition, and the key is
    the token.** `do S while (0)` executes `S` exactly once (C11 §6.8.5.2),
    so its CFG header has `continuation` as its **only** successor and the
    back edge to `body_entry` is a path the C does not have. Removing that
    edge is the **admitting** direction (invariant 1), which is why three
    things must survive refactors. The rule is keyed on the **single literal
    token `0`** and never on an evaluated expression — constraint 5a's rule,
    applied here — so `while ((0))`, `while (1 - 1)`, `while (0 == 0)` and
    `while (0 && 0)` keep the back edge, and answering "not literal zero" is
    the conservative answer, so any widening of `condition_is_literal_zero`
    (the unclaimed spellings `0u`, `0L`, `0x0`, `'\0'` included) is an
    admitting change needing its own measured re-pin. The **exit** edge is
    never the one removed and the header node is retained, so the rule removes
    a path and never a destination, and no leaf's internal coordinate moves
    for an unrelated reason. And the rule does **not** generalise to
    `while (1)`: modelling that exactly would remove an *exit*
    edge, which is the accusing direction and needs its own Lean statement and
    re-pin (`CONSTANT-DO-WHILE-OTHER-CONSTANTS`). A pre-existing route already
    removes the **exit** edge for `for (;;)`: in `legacy_c.rs`,
    `condition_always_true: condition.is_empty()` sets the flag where the
    `for` header is read, and `self.nodes[header].successors = if
    *condition_always_true` is where it drops `continuation` from the
    header's successors (lines 3325 and 5177 at `c33c7640`; the wave-36
    close's 3312 and 5161 had already been moved by `SCALAR-ROUTE-EXIT-EDGE`,
    so the citation is by identifier and the line numbers name the commit
    they were measured at — `DOCS-COMPACT-7` §7.3 item 5). It is exact
    (C11 §6.8.5.3) and `break` still reaches `continuation`, but it carries
    neither a Lean statement nor a pin, and it is not owned by card
    `C-GATEWAY-CONSTANT-DO-WHILE` (`FOREVER-FOR-EXIT-EDGE-UNOWNED`).
    The 32 `unmodeled call,
    aggregate or indirect access in do-while condition` leaves are a different
    question and stay refused — measured unmoved at 32 across the re-pin.
    Proved as `QCore.ConstantDoWhile`: `constant_do_while_is_its_body` the
    `≡ S` equality, `removing_the_back_edge_only_removes_paths` and
    `the_successor_list_never_grows` the conservativity for **every**
    condition, `the_repeated_release_is_a_path_the_c_does_not_have` the
    non-vacuity. Separately, and **not owned by that card**, a pre-existing
    route already decides a `do`-`while` from a *modelled scalar value*; it
    answers conservatively wherever measured and is pinned by
    `a_modeled_scalar_condition_is_a_separate_pre_existing_route`
    (`CONSTANT-DO-WHILE-SCALAR-ROUTE-UNOWNED`).
    That neighbour is now owned by card `SCALAR-ROUTE-EXIT-EDGE` (wave 37), and
    "conservatively wherever measured" was measured only on positively spelled
    headers. The value route decides a loop header from a **modelled value**,
    and deleting a loop header's **exit** edge orphans the function's exit
    node, so `report_leaks` never runs and `classify_diagnostics` returns
    `Safe` — measured for `while (!flag)`, `while (flag == 0)`, all three loop
    kinds and the known-non-null arm, swallowing a double release after the
    loop as readily as a leak, and **unsound** rather than merely undefended
    for a `volatile` object, which C11 §5.1.2.3 lets change outside the
    program. A loop header's exit edge is therefore **never removed**, whatever
    the value says: `LegacyCfgNode::loop_exit_edge` records the leaving
    successor where each header's successors are assigned, the `Infeasible` arm
    retains that edge carrying the unrefined state, and the refusal is reported
    from the **settled** states by `report_retained_loop_exit_edges` rather
    than inside the work list, because the arm is reached on intermediate
    iterates (constraint 6). The sibling **back** edge is deliberately
    unchanged (`SCALAR-ROUTE-BACK-EDGE-UNDEFENDED`), an `if`'s path sensitivity
    is unchanged (154 corpus prunings), and `for (;;)` carries no exit
    successor for the rule to reach. Proved as
    `QCore.ScalarRouteExitEdge.the_exit_edge_is_never_removed`, with
    `the_rule_never_promotes_to_safe` for the other direction. Finally, removing a back edge
    **makes a function acyclic**, and that changes which route
    `legacy_joined_provenance::derive` takes for it (constraint 10b) — so this
    rule silently changes *that* pre-pass's reach: **654** of 4,771 analysed
    functions at this card's re-pin, with a measured diagnostic effect of zero.
    Any widening of the key therefore moves two subsystems and must re-measure
    both (`docs/reports/c-gateway-constant-do-while/scripts/acyclicity.py`,
    `CONSTANT-DO-WHILE-JOINED-PROVENANCE-REACH`).
    **Two clauses of this paragraph are the integrator's, not the card's**, and
    the wave-35 close's `reconciliation.json` carries both before-and-after
    texts: the card wrote that `derive` "returns no conditional-provenance facts
    at all for a cyclic CFG", which wave 35's **second** arbiter
    (`JOINED-PROVENANCE-CYCLES`) falsified later in the same wave by solving a
    cyclic CFG with a greatest fixed point — landing that sentence verbatim
    would have put a statement contradicting 10b into this file — and the card's
    script path was its own gitignored `measure/`, replaced by the committed
    copy it archived. The rule itself is unchanged, and the residual it names is
    **answered** by that second arbiter's §8, which reproduced the 654
    independently and measured the flip's cost at six functions and 20 leaves of
    servable population — reach that moved to the topological pass, not reach
    lost. `c-gateway-constant-do-while`, `joined-provenance-cycles`.
<!-- preserved:constraint-5c:end -->
~~~~~

### 2. Constraint 7

`docs/agents/gateway.md` lines 473–547 at `8c1f15eb`, 5,491 bytes. The whole preserved block, markers included, with the per-leaf chain. Corrected in the rule: the pin is 34 on the baseline, not 272; its directions, residuals and pointers stay.

~~~~~text
<!-- preserved:constraint-7:start -->
7. Leak diagnostics are **272** across the pinned 185-unit corpus. A change
   that moves that number is a finding to attribute in the re-pin, not to
   absorb. Through wave 30 it had moved four times, each deliberately and each
   either in the *accusing* direction or by **deciding** a leaf. The current
   six-row increase is attributed after that historical chain below.
   271 → 272 (wave 17): `flb_input_chunk_get_direct_routes` L19399,
   accusing, on a function and a unit already `Unsafe`, held provisional as
   `CONSTRUCTOR-RESIDUALS-LEAK-271`. 272 → 273 (wave 20): `input_chunk_get`
   L19849, accusing — modelling the constructor exposed a release through the
   then-uncontracted `flb_input_chunk_destroy`. 273 → 272 (wave 22): that same
   leaf decided by the faithful `flb_input_chunk_destroy` contract — a
   **withdrawal**, not a discharge, since the release refuses at the call on
   joined provenance and `input_chunk_get` goes provisional `Unsafe` →
   `Unknown`; residual `AOM-DESTROY-JOINED-PROVENANCE`. 272 → **266**
   (wave 23): the faithful **`nullable`**
   `flb_input_chunk_destroy_direct_routes` contract (`nullable` where its
   already-declared neighbour carries `nonnull`, because that body's third
   statement is `if (!routes) { return; }`) retires six leaves of
   `flb_input_chunk_get_direct_routes` — L19384, L19392, L19399, L19434,
   L19489, L19505 — again by **withdrawal-by-refusal**: at each preceding call
   site the projection cannot resolve the actual it is handed, so the site is
   marked through the unresolved-effect mechanism and `report_leaks` skips it.
   The cause is that every refused statement poisons every pointer binding in
   the state, so an acquisition's binding does not survive to its release —
   residual `AOM-SIBLINGS-BINDING-POISON`, whose repair moves those sites
   toward the *discharging* arm, a direction invariant 1 constrains. That move
   **DECIDES — withdraws** `CONSTRUCTOR-RESIDUALS-LEAK-271` at a measured cost
   of **five** standing claims, not the six wave 17 recorded: the sixth leaf,
   L19544, is on the success path where no destroy call runs, so it **stands**,
   the function keeps its leak and its provisional `Unsafe`, and the pin reads
   266 rather than 265. Nothing is promoted by any of the four: zero unit status
   moves, zero function status moves. At the wave-25 re-measurement there was
   **no fifth move**: the L19399 leaf wave 23 decided by *withdrawal* is decided instead by the **discharging** arm of that same contract, its first
   firing anywhere in the pinned corpus: a strictly better answer to the same
   question, changing no count, the pin still **266** with not one leak leaf
   added or removed. The wave-27 arbiter also made **no further move**: it
   re-pinned *presentation* only, and the 266 leak rows are exactly the 266 whose
   description text was rewritten — each verified to be the substitution of the
   **site alone**, the text on both sides of the phrase byte-identical. A leak
   leaf's identity is still its preprocessed line.
   `remaining-o04-b08`, `corpus-repin`,
   `remaining-constructor-residuals` §6.4, `wave17-corpus`,
   `wave20-abi-address-of-member`, `wave20-corpus`, `wave22-aom-destroy`,
   `wave22-corpus`, `aom-destroy-siblings` §6 and §6.1,
   `aom-binding-poison` Headline and §5.2, `gateway-c-location` §5.3.
   The current `C-GATEWAY-WAVE` re-pin advances **266 → 272**: six added
   provisional leak leaves, no baseline leak leaf withdrawn. Two arise from
   complete-expression frames and four from nullness refinement; every site is
   listed in `docs/reports/c-gateway-wave/README.md`'s final measured result.
   Four intermediate missing leaves were restored by the scalar-assignment
   repair before publication. The original worklist schedule is retained;
   neither the leak rows nor the new constant-loop repeated-release row is a
   validated upstream defect claim.
   At the wave-34 `C-GATEWAY-RETURN-CALL-W4` arbiter the pin is **unmoved at
   272** — the first gateway *admission* card in this chain to move no leak leaf
   in either direction. That is not luck and is worth the sentence: the exits it
   newly models are allocator wrappers that retain nothing, so exposing them
   accuses nobody. Admitting a previously unmodeled exit remains a move in the
   accusing direction by construction, because `report_leaks` **skips** an
   unmodeled exit; a future admission card reaching bodies that do retain should
   expect the pin to rise, and that rise is a finding to attribute, not a
   regression. `c-gateway-return-call-w4` §5.
   Card `W36-CORPUS-CONTRACTS-3` (wave 36) moved the number **not at all**:
   272 → 272 across a re-pin that added six declared contracts and moved 21
   function statuses. A declarative card that decides 204 uncontracted-call
   leaves is therefore not automatically a leak-pin event, and the pin was
   re-verified rather than re-derived.
   Card `W37-CORPUS-CONTRACTS-4` (wave 37) left the number unmoved a second
   consecutive time: 272 → 272 across a re-pin adding ten declared contracts
   and moving eight function statuses. Two declarative arbiters in a row have
   now decided several hundred uncontracted-call leaves without touching the
   leak pin, which is the expected shape when the declarations release
   nothing; the pin was re-verified rather than re-derived.
<!-- preserved:constraint-7:end -->
~~~~~

### 3. Constraint 8, the preserved start marker

`docs/agents/gateway.md` lines 551–551 at `8c1f15eb`, 37 bytes. Removed with the end marker (next block); lines 552–580 stay byte-identical in place.

~~~~~text
<!-- preserved:constraint-8:start -->
~~~~~

### 4. Constraint 8, the two first-exercise paragraphs

`docs/agents/gateway.md` lines 581–604 at `8c1f15eb`, 1,613 bytes. The rest of the preserved block and its end marker. Kept in the rule as two precedents, with both residual names and the report pointer.

~~~~~text
   The mode gate's `SummaryConflict`-only arm for a **nullability
   refinement** was exercised for the first time by a *declaration* rather
   than by the corpus: card `W36-CORPUS-CONTRACTS-3`'s `rd_kafka_mem_free`
   declares `Released{nullable}` where W4 derives `Released{nonnull}`, and
   the declaration is the faithful one — `rd_free(NULL)` reaches
   `free(NULL)`. The declaration governs every call site, the defining unit's
   own function moves `Safe → Unknown`, and **that is the gate working**: a
   disagreement is reported, never silently resolved, and declaring `nonnull`
   to silence it would assert a precondition the body does not have
   (constraint 4). `w36-corpus-contracts-3` §5.2.
   The ABI reader's **withdrawal** arm was exercised for the first time by a
   *production declaration* in card `W37-CORPUS-CONTRACTS-4` (wave 37):
   `rd_kafka_topic_partition_list_sort`'s C declaration spells its comparator
   as an **inline function-pointer parameter declarator**, which the reader
   does not parse — it resolves the function-pointer **typedef** form that
   card `CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF` taught it, and nothing else — so
   the declaration is withdrawn at every call site with
   `abi-unreadable-declaration`. The withdrawal is **leaf-neutral**: all
   eleven rows are same-position substitutions and no function's leaf count
   changed. This is the
   gate failing closed on a *reader* boundary rather than on a disagreement,
   which is `LOADER-ABI-UNDECLARED`'s neighbour and is named
   `LOADER-ABI-INLINE-FNPTR-DECLARATOR`.
<!-- preserved:constraint-8:end -->
~~~~~

<a id="diagnostics"></a>

## `docs/agents/diagnostics.md`

Current page: [../../agents/diagnostics.md](../../agents/diagnostics.md).

### 5. Diagnostics namespaces, the preserved start marker

`docs/agents/diagnostics.md` lines 43–43 at `8c1f15eb`, 46 bytes. Removed because the block below it is no longer the migration's text; the end marker is in the last block of this section.

~~~~~text
<!-- preserved:diagnostic-namespaces:start -->
~~~~~

### 6. Table row `Q051`–`Q057`

`docs/agents/diagnostics.md` lines 117–117 at `8c1f15eb`, 2,916 bytes. The rule keeps every card and every widened, narrowed or unwidened code; §L21 carries the wording.

~~~~~text
| `Q051`–`Q057` | `StandardBuffer` | `source.buffer.v1`: the owned extent-carrying byte buffer (`bytes`, `[i]`, `extent`, `drop`). Card `AGGREGATES-V1` admits the first source-level aggregate on this route — `source.record.v1`'s `type NAME { FIELD: i32, … }`, its construction and its field read — and **allocates no code**, widening three conditions: `Q051`'s to a record declaration, a field declaration, a construction outside the form (including one supplying a field twice, an undeclared field, or not every declared field), a field value that is not one operand and a numeric field path; `Q052`'s to a name at a record or field position that is not one (a construction of an *undeclared type* refuses earlier still, at `Q013`, a type name being an ordinary reference); and `Q053`'s from an index/byte/extent range to those and a **field count** outside `1..=32`. `Q054` (leak beside a record) and `Q055` (a buffer at a field position) are the route's existing rules reached unchanged, and `Q056`'s condition is **not** widened — the recheck comparing two more published halves is that condition working. Card `SLICE-PERMISSIONS-WRITE-LEND` (wave 29) lands the **write lend** `let NAME = lend(BUFFER, LOW, HIGH);` on this route (constraint 15h carries the rules) and again allocates no code, widening five conditions: `Q051` to the lend's window shapes (a `for` beside a lend, crossed windows, an acquisition inside a window, a malformed or shadowing `lend(…)`); `Q052` to a lend of anything but an owned buffer this entry acquired, and a `drop` of a loan; `Q053` to the lend's range beside the existing ranges; `Q054` from the four listed refusals of the unedited checker to **any refusal of the lend-extended cycle-3 checker** — the checker itself was extended, in the semantic model where every other custody rule lives, and the route still inherits rather than re-implements; and `Q055` to a loan at a scalar, index or rendering position. `Q056` is once more not widened — the recheck comparing the published `lends` field for field is that condition working. Card `AGGREGATE-OWNED-FIELD` (wave 39) lands the **owned field** `FIELD: bytes` (constraint 15k carries the rules) and again allocates no code: **`Q055` is narrowed**, the one lift — a buffer at a field position refuses only at an `i32` field; `Q051` widens to a malformed field-path statement and the two by-name refusals `OWNED-FIELD-LEND-WINDOW` and `OWNED-FIELD-LOOP-BODY`; `Q052` to an integer, scalar or record at a `bytes` field and to a custody position naming an `i32` field; `Q053` to an index outside the extent a field holds; `Q054` carries the path-extended checker's vocabulary (a use after move of a path, a double drop of a record or path, a leak naming `RECORD.FIELD`, `AggregateAtOwnerPosition`); `Q056` is not widened — the recheck comparing the published moves and field releases is that condition working |
~~~~~

### 7. Table row `Q068`–`Q079`

`docs/agents/diagnostics.md` lines 119–119 at `8c1f15eb`, 2,222 bytes. The rule keeps both extending cards and the deliberate choices; the pool history moved.

~~~~~text
| `Q068`–`Q073`, `Q075`–`Q079` | `StandardString` | `source.string.v1` behind `std.string`, three bands. `Q068`–`Q073` (relocated from `Q065`–`Q070` at integration) the immutable UTF-8 `{ptr, len}` view: entry shape, an undecidable slice bound, a quoted token that does not decode, a bound out of range or off a code-point boundary, a string at a position it may not cross, the declared `effects` row. `Q075`–`Q076` (`STRING-SLICE-IO`, which returned `Q077`–`Q079` to the pool) text reaching a stream: `Q075` the line's shape, `Q076` a text operand naming no string this entry bound. `Q077`–`Q079` (`STRING-CONCAT`, reassigned to this pass) **computed text** — `concat(…)`, `dec`/`hex`, `borrow_utf8`: `Q077` shape and bound refusals **including an embedded `source.buffer.v1` statement carrying that route's own message under this pass's code**, `Q078` the borrow-exclusion window with the inherited ownership checker's own refusal, `Q079` the **static UTF-8 decision** on a buffer's write history. Card `CT07-SLICE-PERMISSIONS` makes the borrow lend a **range** (`borrow_utf8(NAME, LOW, HIGH)`, arity one or three) and allocates **no code**: `Q077` also covers a range bound violating `0 <= low <= high <= extent` and the two-argument spelling (deliberately not `Q071`, whose condition is a bound on a *view* decided against `len(s)`); **`Q078`'s condition is widened** from "a store into or a release of a buffer inside a live borrow's window" to "a store **into the bytes** of a live loan, or a release of a buffer any live loan holds", a whole-extent loan reporting the sentence it always reported byte for byte; and `Q079` is reached **without widening at all**, because that condition was always *the bytes this borrow would view* — so a range whose end splits a code point refuses there by the rule already written. A bound that is not a check-time constant stays `Q069`/`Q013` and a buffer or string at a bound position stays `Q072`. `SLICE-PERMISSIONS-WRITE-LEND` (wave 29) widens `Q077` once more: a `lend(…)` in a composed string/buffer entry refuses **by name** until a cross-model overlap rule with `borrow_utf8`'s read loans exists (`SLICE-PERMISSIONS-COMPOSED-LEND`, constraint 15h) |
~~~~~

### 8. Table row `Q080`–`Q089`

`docs/agents/diagnostics.md` lines 120–120 at `8c1f15eb`, 2,092 bytes. The rule keeps every code and the three extending cards; the band history moved.

~~~~~text
| `Q080`–`Q089` | `StandardSocket` | `source.socket.v1`: the `std.socket` bundled module's checked TCP and UDP surface over loopback — `Q080` entry shape, `Q081` the operand rule, `Q082` the transport state machine, `Q083` the inherited custody refusal, `Q084` byte/payload/count ranges, `Q085` the **determinism boundary** (a receive the program's own committed history does not satisfy), `Q086` the crossing boundary, `Q087` the `effects` row, `Q088`/`Q089` the certificate recheck and the backend refusal. Band assigned by the integrator before the card, with `Q075`–`Q079` held for a sibling card. Two later cards extend the row and **allocate no code**, landing on `Q080`–`Q082` and `Q085`–`Q087`: `SOCKET-EVENT-COMPOSE`, the **reactor composition** (a checked socket registered with `source.event.v1`'s reactor), and `SOCKET-INET-ADDRESSES`, **declared IPv4/IPv6 addresses with ports** (`bind S 127.0.0.1:8080;`, `connect C [::1]:8080;`, `sendto U ADDRESS […];`), whose external-peer refusal is `Q085`. Each **widens one condition**, stated here and in §L21 (constraint 1): `Q083`'s from "the inherited ownership checker refused" to "an inherited, unedited **custody model** refused", so a reactor `L00`–`L12` diagnostic lands there carrying its own value; and `Q084`'s from a byte/payload-length/receive-count range to those and a **port** range. Card `CT42-CELL-ROUTE` (wave 39) adds the **disjointness question** `disjoint NAME NAME;` — one reserved word, census-clean — and again allocates nothing, widening three conditions: `Q080`'s to the question's shape (not exactly two names), `Q081`'s to a question naming no socket, and **`Q085`'s** to the cell derivation's two refusals (an accept with nothing queued, a connect the resolution gives no listener — the determinism boundary's own conditions, reached from the deriver and unreachable after the transport checker admitted the program); a question of a closed socket is the existing `Q083` then `Q082`, and the question itself is answered at `Q664` (the `Concurrency` row). Per-condition mapping in §L21 |
~~~~~

### 9. Table row `Q090`–`Q096`

`docs/agents/diagnostics.md` lines 121–121 at `8c1f15eb`, 2,150 bytes. The rule keeps every code, both extending cards and constraint 15f; §L21 carries the long form.

~~~~~text
| `Q090`–`Q096` | `StandardHttp` | `source.http.v1`: the `std.http` bundled module's checked HTTP/1.1 message subset — `Q090` entry and statement shape, `Q091` the start line, `Q092` header fields (including the required single `Host` and the `CR`/`LF` exclusion that makes the constructed framing unforgeable, so header injection is not expressible), `Q093` framing (signature condition: a declared `Content-Length` that is not the content's byte count), `Q094` a name at a message position that is not one, `Q095` the **inherited, unedited** `source.socket.v1` refusal carrying that checker's own error value, `Q096` an undeclared derived effect. Band pre-assigned as `Q090`–`Q099`; `Q097`–`Q099` unused and returned to the pool (`HTTP-QUIRIN-V1`). Card `HTTP-V1-COMPUTED-LENGTH` admits a **computed** header value or content — a quoted token, a name this entry bound to text, a `concat(…)` of check-time-known pieces, or `dec(NAME)`/`hex(NAME)` — allocating **no code** and widening no gate, since every component is resolved to bytes *before* `admit_field` and `admit_framing` run and `Q093`'s equality stays decided rather than trusted (constraint 15f). It widens `Q090`'s condition (a malformed `concat`/`len`, a composition of fewer than two pieces, a composed payload above the 4096-byte bound, a selector without its operand, a value name rebound) and `Q094`'s (a component, piece or `len` operand naming a binding of the other kind). The field **name** stays a written token. Card `CT42-CELL-ROUTE` (wave 39) **widens `Q095`'s condition** and allocates nothing: the HTTP reader hands every statement it does not spell to the socket reader, so `disjoint NAME NAME;` lowers into the transport program, and the driver now composes the protocol-cell check over that program too — `Q095` covers a refusal of the cell **derivation** (unreachable after the transport checker admitted the program) beside the transport checker's own, while the question's answer stays `Q664`. Before that composition `quirin check` **admitted** an HTTP exchange asking about its own two ends with no judgement, measured and closed in the same card |
~~~~~

### 10. Table row `Q600`

`docs/agents/diagnostics.md` lines 128–128 at `8c1f15eb`, 3,113 bytes. The rule keeps all six extending cards, the narrowing, the `Q500` backstop and the non-instance.

~~~~~text
| `Q600` | `Concurrency` | scoped-task checking and the carried-over structural refusal shared by every checked concurrency profile. Five later cards move this row and **allocate nothing**, each stated in §L21: `REGION-CREDIT-JOINS` (wave 28) widens the composed-route reconciliation condition — a spawn or join whose span follows the entry's `error region` is refused, the ordering derived from the checked task IR and never declared; `CT47-COPY-CAPTURE` (wave 29) widens the boundary condition — the transferability judgment's refusal of a task-creation capture, attributed at the crossing construct and naming the missing transfer/share contract, previously falling through to the `Q500` executable-subset catch-all (kept unedited as the fail-closed backstop); `REGION-JOINS-LOAN` (wave 30) **narrows** one condition deliberately and widens none, discharging the composed epoch route's blanket refusal of read-loan captures (`T08-COMPOSE-LOAN`, constraint 15m); and `WRITE-LEND-TASKS` (wave 31) widens the boundary and scoped-task conditions three ways on the `CT47-COPY-CAPTURE` precedent — the judgment's grown vocabulary (`TransferMode::LendWrite`, the delivered `lend<Buffer>`, `ShareProtocol::DisjointWriteLeases`), the lease's check-time bound decisions (a varying bound, an empty range, a range past `BUFFER_PAYLOAD_BYTES`), and the ledger's own `OverlappingLeases` / `LeaseOutstanding` / `InvalidLease` / `TooManyWriters` arriving through the unedited `check_task_module` (constraint 15h). `WRITE-LEND-TASKS-EXERCISE` (wave 36) widens the scoped-task condition a fourth time and allocates nothing: the worker-side byte access — the creator's **footprint fit** (the bound a worker's body states against the lease that creation supplies), the constant-index tier (an unresolvable index is not provably inside, a negative one and one past `BUFFER_PAYLOAD_BYTES` refuse beside it), and the three authority rules (no owner byte write, no read-share byte view, the owner's byte read on the whole-payload read's own gate) — all decided in the IR, constraint 15h. `MUTEX-SHARE-CROSS` (wave 32) is the fifth, widening `Q600`'s boundary condition and the `Q610`–`Q627` sub-band's route and allocating nothing, on the same precedent: the judgment's vocabulary grows again (`TransferKind::ProtectedResource` and the delivered `MutexShare`, `TransferMode::ShareUnderMutex`, each sharing mode now requesting **one** named protocol), and the crossing's two protocol gates — **absent** (the `under` name is not a mutex generation of this module) and **mismatched** (it guards some other resource) — land at `Q600` at the clause, while the third refusal, `unverified`, is the unedited checker's own `Q611` (constraint 15o). This row's entry is the wave-35 close's, three waves late; the authority it summarises, spec §L21, has carried the widening since the card (`MUTEX-SHARE-CROSS-INTEGRATOR-LIST-UNLANDED`). `CT24-CLEANUP-PREEMPTED` (wave 30) moves no code and widens no condition: the `ScopeBusy` refusal is rendered from the exit rule's own loan backstop, same code, same span, same refused set |
~~~~~

### 11. Diagnostics namespaces, the preserved end marker

`docs/agents/diagnostics.md` lines 137–137 at `8c1f15eb`, 44 bytes. With the start marker above, removed because the block is no longer the migration's text; the untouched rows stay byte-identical in place.

~~~~~text
<!-- preserved:diagnostic-namespaces:end -->
~~~~~

### 12. The amendments preamble

`docs/agents/diagnostics.md` lines 143–147 at `8c1f15eb`, 364 bytes. Replaced: `verify.py` exits 1 at base `8c1f15eb`, so it pins nothing (`W45-DOCS-PRESERVATION-VERIFIER-DECAYED`).

~~~~~text
The preserved blocks above are the migration's frozen reconstruction, pinned by
`docs/reports/docs-structure-compression/verify.py` and never edited in place
(constraint 21). Amendments land here, carry the same authority as the rule they
amend, and stay short: `docs/QCORE_SPEC_r0_3.md` §L21 carries each condition's
full text, and this page is capped at 32 KiB.
~~~~~

<a id="buffers-records"></a>

## `docs/agents/buffers-records.md`

Current page: [../../agents/buffers-records.md](../../agents/buffers-records.md).

### 13. 15k amendment (`OWNED-FIELD-ROUTE-REACH`), its opening two paragraphs

`docs/agents/buffers-records.md` lines 319–328 at `8c1f15eb`, 624 bytes. The rule keeps what the amendment makes reachable; the account of the unit tests moved.

~~~~~text
It supersedes nothing above — it makes two of constraint 15k's own sentences
reachable, which `STABILITY-REEVAL-4` measured they were not.

The copy rule of the sentence "`Copy` of a record declaring an owned field
refuses whatever the state of its fields" was decided by **four unit tests and
by nothing else**: `OwnershipOp::Copy` was constructed nowhere outside
`crates/qcore-core/tests/unit/ownership.rs`, so no `.qcore` program could
produce the operation the rule refuses, and the opening sentence "a record of
scalars is a plain **copyable** value" was an assertion this project could not
check in either direction.
~~~~~

### 14. The amendments preamble

`docs/agents/buffers-records.md` lines 307–313 at `8c1f15eb`, 510 bytes. Replaced with the same statement as `diagnostics.md`'s (integrator decision).

~~~~~text
The blocks above are the migration's **preserved checkpoint text**, and
`docs/reports/docs-structure-compression/verify.py` reconstructs the pre-restructure
guides from them byte for byte. Editing inside a `<!-- preserved:… -->` block
therefore breaks that reconstruction, which is standing constraint 21's own
case: provenance is published **beside** a frozen archive, never by editing it.
Amendments to a rule on this page land here, under the rule's number, and are
part of the rule — read them with it.
~~~~~
