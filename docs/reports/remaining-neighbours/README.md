# NEIGHBOURS — production declared contract text for the six uncontracted callees on the constructor's path — 2026-09-09

The named continuation of card **PROD-CONTRACTS**
([`docs/reports/remaining-prod-contracts/README.md`](../remaining-prod-contracts/README.md)
§3.4 and its open boundary `PROD-CONTRACTS-NEIGHBOURS`). That card landed the
logging family, the input-chunk name accessor and the routes-mask helpers,
measured that `flb_input_chunk_create`'s success exit **does not** discharge in
production, and isolated the cause by withdrawing **one** declaration and
reproducing the production refusal exactly. It named six uncontracted
neighbours. This card reads all six pinned bodies and lands all six as
production contract text.

**Base tree:** main `19fbaa5` ("Consolidate wave 14: corpus re-pin, §C05
correction, follow-ons, ledger"), submodules initialised, `third_party/libevent`
at `d82464a2`. Every measurement below was taken on that tree with only this
card's own diff applied.

---

## Headline

**No. `flb_input_chunk_create` does not reach function-level `Safe` in
production.** It stays `Unknown`. But **every uncontracted-callee refusal on its
path is gone, and so is the exact refusal card PROD-CONTRACTS isolated** —
measured on the pinned 185-unit corpus, not on a fixture:

| `flb_input_chunk_create`, `fluent-bit-chunk/flb_input_chunk.c` | before | after |
|---|---|---|
| function status | `Unknown` | **`Unknown`** — unchanged |
| diagnostics | **36** | **21** |
| refusals naming a callee | **6** | **0** |
| the C6 tail refusal at L19626 (`arguments beyond the declared parameters of \`flb_log_print\``) | present | **gone** |
| `unresolved resource identity` | 13 | 12 |
| `indirect memory access` | 15 | 8 |

**What moved is exactly the contract family, and nothing else moved with it.**
The 12 residual refusals that name an unresolved `ic` or `chunk` are the
aggregate-aliasing family, and the 8 residual indirect accesses are the
chained-read family; neither is a contract question, and no declaration of any
function can decide either. Card PROD-CONTRACTS' `PROD-CONTRACTS-NEIGHBOURS` is
**closed by declaration, not by discharge**, and saying so is the point.

**The single blocker on the failure arm, isolated by substitution.** The
surviving refusal at production line 19581 is the `flb_error` expansion on the
`!chunk` arm, whose tail actual is `storage->stream->name` — a two-level chained
pointer-field read whose **final field is itself pointer-typed**. On the
byte-verbatim fixture leg, replacing that one actual with the plain local array
the same statement already passes takes the constructor to **zero** diagnostics
(`withdrawing_the_chained_tail_actual_decides_the_constructor`). Recorded as
**`NEIGHBOURS-CHAINED-TAIL`** (§8) and deliberately not taken here: extending
the chained-access admission to a pointer-typed final field is checker work with
a real soundness obligation, and this is a contract-text card.

**Three measured corrections to the card brief:**

1. **The brief's six are the six *callee-naming* refusals, not the whole refusal
   set.** PROD-CONTRACTS §3.4 wrote that the constructor's "remaining refusals
   *name* six callees"; read exactly that is true and complete, and it is not
   the same statement as "six refusals remain". Thirty-six diagnostics remained,
   of which six name a callee and one is the C6 tail. §3.
2. **One of the six could not be declared faithfully in the v1 vocabulary at
   all, and that had to be repaired before it could be written.** `flb_time_now`
   returns `double`; the scalar ABI table held integers only, so the only
   writable projections were an integer spelling the function does not have or
   `unit`, which would say it returns nothing. Card O04-B08's gateway *fixture*
   took the first of those and wrote `-> u64` beside a C prelude declaring the
   real `double`. That is acceptable in a fixture, where its header says so; it
   is not what production text may say. §2.6.
3. **The fixture leg and the production corpus are not the same measurement,
   and the fixture's residual understates production.** The caller excerpt is
   byte-verbatim, but its prelude is reduced, so the real headers' richer types
   are absent: the fixture reaches **1** residual refusal where production
   reaches **21**. Both routes agree on the movement this card causes — all six
   callee refusals and the C6 tail refusal gone — and on the one blocker they
   share. The other twenty are production-only and named in §4.3. Reporting the
   fixture's 1 as if it were production's would have been the error this
   correction exists to avoid.

**Corpus impact.** [§4](#4-corpus-impact--every-leaf-attributed). Nothing is
promoted, at any level.

**Pins moved deliberately and loudly:** the contract corpus is 64 → 67 modules,
the resource-v1 declaration count 25 → 31, the manifest's selected-module count
9 → 12, and `SCALAR_ABI_TYPES` gains `f32`/`f64`. §5.

Every declaration this card lands is a **trusted declared assumption**, read
against the pinned upstream body first and cited by line. A function-level
`Safe` that ever rests on one is `Safe` **relative to** it, never a proof of the
C.

---

## 1. Provenance of every body read

All six bodies, and every body reached from them, were read in the prepared
pinned clone at Fluent Bit `66910c10a4d7eafa810b84229cf2dcf7b0f26f97`
(the same clone [`docs/reports/corpus-repin/provenance.json`](../corpus-repin/provenance.json)
records), in `src/` for the Fluent Bit halves and in the vendored `lib/chunkio/`
copy for the ChunkIO halves. Each module header cites file and line beside the
declaration it justifies; the readings are reproduced in §2 with the same
citations.

---

## 2. Each callee, read from the pinned body, with its mode decision

The brief's instruction: *contract faithfully (naming the mode
argument-by-argument), or refuse to contract with the measured reason — an
unfaithful declaration is worse than Unknown (the P07 lesson).* **All six are
faithful, and all six are declared.** Each subsection below names what was read
and what the declaration does **not** say.

### 2.0 The question a declaration answers

Over the whole v1 parameter vocabulary, `resource_borrowed` is the only mode
that says *the caller's obligations are unchanged across this call*.
`resource_released` and `resource_consumed` discharge an obligation;
`resource_owned` asserts entry custody, which asserts non-nullness and deletes
the caller's failure paths; `resource_stored` moves custody permanently to a
declared owner; `resource_out` writes a fresh acquisition into a slot. Each
subsection below states which of those the body was read to do, and every one of
the six was read to do none of them.

The brief also asks a sharper question: *where a callee's body itself calls
uncontracted functions, measure whether the contract can still be faithful.*
It can, and the reason is structural: **a declared summary describes the
callee's own interface obligations — what happens to the CALLER's resources
across the call — not its body's internal accounting.** Four of the six call
functions this corpus does not contract (`snprintf`, `getpid`, `flb_time_get`,
`memcpy`, `flb_calloc`, `cio_meta_write`, `cio_file_native_*`, `mmap`/`munmap`
wrappers). In every case the reading below follows those calls far enough to
establish what they do to the caller's arguments, and no further. Where a
callee's internal accounting is *not* established, that is stated as a
non-claim rather than folded into the declaration.

### 2.1 `cio_chunk_is_up` — `contract/qcore/chunkio/chunk_residency.qcore`

```c
int cio_chunk_is_up(struct cio_chunk *ch)
```
(`lib/chunkio/src/cio_chunk.c:509-524`.)

| Argument | What the body does with it | Mode |
|---|---|---|
| `ch` | `type = ch->st->type;`, then either a constant `CIO_TRUE` (memory backend) or `cf = ch->backend;` and `return cio_file_is_up(ch, cf);`. `cio_file_is_up` (`cio_file.c:1378-1388`) discards its chunk with an explicit `(void) ch;` and answers from two predicates on `cf` | `resource_borrowed` |
| result | `CIO_TRUE`/`CIO_FALSE`, a flag | `resource_return_none()` |

Nothing is allocated, freed, stored or published on any path. This one is a pure
predicate and the declaration is exact.

### 2.2 `cio_chunk_up_force` and `cio_chunk_down` — same module

```c
int cio_chunk_up_force(struct cio_chunk *ch)   /* cio_chunk.c:590-605 */
int cio_chunk_down(struct cio_chunk *ch)       /* cio_chunk.c:556-571 */
```

Both have the same three-step shape: `cio_error_reset(ch)` (whose entire body is
`ch->error_n = 0;`, `cio_error.c:56-59`), then for the filesystem backend only
`cio_file_up_force(ch)` or `cio_file_down(ch)`, then `chunk_state_sync(ch)`
(`cio_chunk.c:538-554`).

**These two needed reading rather than pattern-matching**, because both do real
work and a careless reading would reach for the wrong mode.

* `cio_file_up_force` is `_cio_file_up(ch, CIO_FALSE)` (`cio_file.c:901-904`,
  body at `:816-883`): it refuses when already mapped or already open, opens the
  native file, updates `cf->fs_size`, maps the content, and on
  `CIO_CORRUPTED`/`CIO_RETRY` closes the descriptor it had just opened.
* `cio_file_down` (`cio_file.c:920-958`) unmaps through `munmap_file`, zeroes
  `cf->alloc_size`, updates the size and closes the descriptor.

**Neither frees `ch`, and neither frees `ch->backend`.** That was checked, not
assumed: the only `free(cf)` sites in `cio_file.c` are `cio_file_open`'s four
rollback arms (`:722`, `:736`, `:761`, `:772`) and `cio_file_close`'s teardown
(`:990`), none of which either function reaches. So the caller's obligation to
`cio_chunk_close` is unchanged across both calls, which is exactly what
`resource_borrowed` says.

| Argument | Mode |
|---|---|
| `ch` | `resource_borrowed` |
| result | `int` status — `resource_return_none()` |

**Not claimed, recorded rather than hidden.**

* **These calls move OS resources *inside* the chunk.** `cio_chunk_up_force`
  opens a descriptor and creates a mapping; `cio_chunk_down` unmaps and closes.
  This projection models the caller's custody of the chunk handle and nothing
  else, so it neither tracks those endpoints nor asserts that `cio_chunk_close`
  releases them. That is separate observed evidence
  ([`cycle19-file-descriptor-failures`](../cycle19-file-descriptor-failures/README.md),
  [`cycle19-reopen-failures`](../cycle19-reopen-failures/README.md)), not a
  consequence of any declaration here. Named **`NEIGHBOURS-RESIDENCY-ENDPOINTS`**.
* **`chunk_state_sync` relinks the chunk between two of its stream's lists**
  (`mk_list_del(&ch->_state_head)` then `mk_list_add` into `st->chunks_up` or
  `st->chunks_down`). That is not a new publication — the chunk was already
  published into its stream by `cio_chunk_open`'s own body, which this
  projection does not follow either (O00 blockers B03/R13) — and it does not
  move the caller's obligation. `resource_stored` would say it did, and would be
  unfaithful: the caller must still close.
* **Nothing about success.** Both return failure codes on several paths. A
  declaration is a custody statement, not a postcondition on the status.

### 2.3 `generate_chunk_name` — `contract/qcore/fluent_bit/input_chunk_helpers.qcore`

```c
static void generate_chunk_name(struct flb_input_instance *in,
                                char *out_buf, int buf_size)
{
    struct flb_time tm;
    (void) in;
    flb_time_get(&tm);
    snprintf(out_buf, buf_size - 1, "%i-%lu.%4lu.flb",
             getpid(), tm.tm.tv_sec, tm.tm.tv_nsec);
}
```
(`src/flb_input_chunk.c:743-754` — that is the whole body.)

| Argument | What the body does with it | Mode |
|---|---|---|
| `in` | **discarded** by an explicit `(void) in;` on every path | `resource_borrowed` |
| `out_buf` | **written through** by `snprintf`. Nothing is allocated for it, nothing is freed through it, the pointer is stored nowhere | `resource_borrowed` |
| `buf_size` | `int` by value | `resource_borrowed` |
| result | `void` | `resource_return_none()` |

`flb_time_get(&tm)` takes the address of a function-**local** `struct flb_time`,
not of anything the caller owns. A callee that writes through a caller's buffer
is not thereby taking custody of it, and `resource_borrowed` says only the
latter — the same reading PROD-CONTRACTS recorded for
`flb_routes_mask_set_by_tag`'s `routes_mask`, reached the same way.

**Not claimed:** that the write is in bounds. The body passes `buf_size - 1` as
`snprintf`'s size while the caller passes `sizeof(name) - 1` as `buf_size`, and
nothing here relates either number to the caller's array. Extent is not custody.
Named **`NEIGHBOURS-NAME-EXTENT`**, the same class of non-claim as
`PROD-CONTRACTS-MASK-EXTENT`; it is neither a defect claim about the upstream
code nor a licence to ignore one.

### 2.4 `input_chunk_write_header` — same module

```c
static int input_chunk_write_header(struct cio_chunk *chunk, int event_type,
                                    char *tag, int tag_len)
```
(`src/flb_input_chunk.c:1782-1852`.) **This is the one PROD-CONTRACTS §5.3
isolated**, and it is the one whose faithfulness had to be established most
carefully, because its body allocates.

| Argument | What the body does with it | Mode |
|---|---|---|
| `chunk` | written into via `cio_meta_write`; never freed, never stored | `resource_borrowed` |
| `event_type` | `int` by value, compared | `resource_borrowed` |
| `tag` | a `memcpy` **source** (`:1840`): read, never freed, never stored | `resource_borrowed` |
| `tag_len` | `int` by value; truncated at `:1800-1803` | `resource_borrowed` |
| result | `0`/`-1` status | `resource_return_none()` |

The body allocates exactly one buffer and accounts for it on **both** exits:
`meta = flb_calloc(1, meta_size);` (`:1807`), with the allocation-failure arm
returning `-1` with nothing allocated (`:1808-1811`), `flb_free(meta)` on the
write-failure arm (`:1846`) and `flb_free(meta)` on the success arm (`:1849`).

That the double free is not a defect — that `cio_meta_write` does not retain
`meta` — was **checked, not assumed**. `cio_meta_write`
(`lib/chunkio/src/cio_meta.c:46-73`) copies the buffer on both backends: the
memory backend `malloc`s `mf->meta_data` and `memcpy`s into it (`:60-66`), and
the filesystem backend forwards to `cio_file_write_metadata`
(`lib/chunkio/src/cio_file.c:1075-1145`), whose two arms both end in
`memcpy(meta, buf, size)` into the chunk's own map (`:1100`, `:1141`).

**Not claimed:** that `memcpy` reading `tag_len` bytes through `tag` is in
bounds or that `tag` is live. That is a validity and extent requirement on the
caller, not a custody fact. The declaration neither creates nor detects a caller
that passes a released pointer here; the gateway's own liveness precondition
(card BORROW-LIVENESS) is what refuses that, on settled denotations, and it is
unchanged by this module.

### 2.5 Both helpers are `static`, and that is deliberate

`generate_chunk_name` and `input_chunk_write_header` are file-`static`
definitions inside `src/flb_input_chunk.c`, itself a unit of the pinned corpus.
Declaring a *defined* function is safe by construction: a declared contract
always wins over a W4-inferred summary, a disagreement is reported as a
`SummaryConflict` diagnostic rather than silently resolved
(`crates/qcore-c-gateway/src/legacy_summary.rs`), and the unit's own body still
analyzes. What a declaration fixes is only what the callee's *callers* may
assume. PROD-CONTRACTS declared `flb_input_chunk_get_name` under exactly this
arrangement and measured zero custody summary conflicts corpus-wide; §4
re-measures that count with these two added.

### 2.6 `flb_time_now` — `contract/qcore/fluent_bit/time.qcore`, and the vocabulary repair it forced

```c
double flb_time_now()
{
    struct flb_time t;
    flb_time_get(&t);
    return flb_time_to_double(&t);
}
```
(`src/flb_lib.c:936-942` — the whole body.)

It takes **no parameter**, so under no reading can it touch any caller's
custody. `flb_time_get(&t)` takes the address of a function-local
`struct flb_time` and forwards it to `clock_gettime` via `_flb_time_get`
(`src/flb_time.c:76-79`); `flb_time_to_double` (`src/flb_time.c:95-98`) reads
two integer fields and returns arithmetic on them. Nothing is allocated, freed,
stored or published; the result is a real-valued reading, not a handle.
`resource_return_none()` on a parameterless declaration is exact.

**And it could not be written.** Before this card, `SCALAR_ABI_TYPES` held
integers only, so the only writable projections of a `double` result were an
integer spelling the function does not have, or `unit`, which would say it
returns nothing at all. Card O04-B08's gateway fixture
(`tests/fixtures/contracts/o04_b08_callees.qcore:25`) took the first of those
and declares `extern fn flb_time_now() -> u64`, while the same fixture's C
prelude declares the real `double flb_time_now(void);` — acceptable as a fixture
premise, where the file header says it is one, and **not** what production text
may say.

So `f32`/`f64` were added to `SCALAR_ABI_TYPES`, under exactly the condition
that table's own note stated for them: *"no resource API in scope passes one,
and this loader adds no vocabulary without a use."* There is now a use.

**The widening is strictly additive to what can be *written*, and nothing else**
— checked at the source, not asserted:

* a float is refused in every custody mode exactly as every other scalar is
  (`"custody mode requires a pointer parameter"`,
  `crates/qcore-contract/src/contract_loader.rs`);
* a non-`None` result still requires a pointer return type
  (`"owned result requires a pointer return type"`), so no float can occupy a
  custody position in any declaration;
* the loop in `every_abi_scalar_is_a_borrowed_only_projection` runs over the
  **whole** table, so the two new entries are checked to reject every custody
  mode exactly as the integer entries do;
* `unit` as a parameter type and `ptr<const, ptr<const, u8>>` **stay refused**,
  and their refusals stay pinned — that half of the pinned rejection list is
  the load-bearing half and it is unchanged.

The declared type is a projection label the loader admits from a table; it is
not compared against the C signature. That is precisely why writing `u64` for a
`double` would have been *silent* — and why it is a defect rather than a
conservatism.

### 2.7 The four functions PROD-CONTRACTS read and left undeclared

The brief instructs: *respect those decisions; re-read only if your body reading
contradicts theirs.* **No body reading here contradicts any of the four**, and
none is re-opened:

| Function | PROD-CONTRACTS' recorded ground | Status here |
|---|---|---|
| `flb_log_is_truncated` | zero call sites in the whole preprocessed corpus | unchanged |
| `flb_errno_print` | the `flb_errno()` macro's dereference refuses the statement before any contract is consulted | unchanged |
| `flb_log_check_level`, `flb_log_suppress_check` | F1's exact-arity rule can never admit a variadic tail in an expression position (`PROD-CONTRACTS-PLG`) | unchanged |

### 2.8 Read at the same time and deliberately left undeclared by *this* card

Following the same standard (card O04-ARMS': a declaration with no measured
payoff is a premise bought for nothing):

| Function | Read | Why undeclared |
|---|---|---|
| `cio_chunk_up`, `cio_chunk_is_file` (`cio_chunk.c:573-588`, `:526-536`) | same shapes as their declared siblings; would take the same faithful declarations | zero call sites in the pinned corpus's projections |
| `flb_input_chunk_write_header_v2` (`src/flb_input_chunk.c:1854`) | the exported v2 entry point beside the declared one | its body is substantially larger, allocates through a different route, and has arms this card did **not** read. Declaring it on the strength of its neighbour's shape is exactly the unfaithful-declaration defect card P07 recorded |
| `flb_time_get`, `flb_time_to_double`, `flb_time_to_nanosec` | each takes a caller-local `struct flb_time *` | none occurs in a statement the pinned corpus's projections currently reach |

All recorded under **`NEIGHBOURS-UNDECLARED`**.

---

## 3. The fixture leg — measured on the byte-verbatim caller

This section measures the **gateway fixture** route: the byte-verbatim caller
excerpt under a reduced prelude. §4 measures the **pinned 185-unit corpus**,
which is the authority for anything called a production result. The two agree on
the movement this card causes and disagree on the residual, and §4.3 says why.

### 3.1 The route

`crates/qcore-c-gateway/tests/neighbours.rs`, real-source leg: the byte-verbatim
upstream `flb_input_chunk_create` and `cio_chunk_open` regions the seven earlier
cards pinned, driven through the **upstream logging macro** so the preprocessor
produces the production shape, with the declared input identity
`CSourceInput::HistoricalStdin` — the pinned compatibility route's own (card
G02).

All six neighbours' declarations come from the three **production** modules;
each is cut out of the fixture that used to carry it (the loader rejects a
duplicate function contract), so the fixture **files** stay byte-unchanged on
disk and cards O04, O04-ARMS, O04-B08, O04-READ-BOUND, O04-B09B10, O04-TRACE,
HASH-CONTRACTS and PROD-CONTRACTS keep their exact premise sets and their exact
measurements.

### 3.2 The measurement

Both states measured in one run
(`the_constructor_keeps_exactly_one_refusal_under_production_declarations`),
so the movement is not asserted from memory. **These are the FIXTURE leg's
numbers**; the production corpus's are 36 → 21 (§4.3):

| | diagnostics |
|---|---|
| six withdrawn | **31** |
| six declared | **1** |

The 31, in full — six naming a callee, one the C6 tail, twenty-four the cascade:

| Preprocessed line | Diagnostic |
|---|---|
| 204 | indirect memory access |
| 205 | call to uncontracted function `generate_chunk_name` |
| 206 | indirect memory access |
| 209 | indirect memory access |
| 212 | call to uncontracted function `cio_chunk_is_up` |
| 214 | call to uncontracted function `cio_chunk_up_force` |
| 216, 223, 229, 235, 248 | unresolved resource identity `chunk` |
| 221 | call to uncontracted function `input_chunk_write_header` |
| 237 | unresolved resource identity `in` |
| 240 | call to uncontracted function `flb_time_now` |
| 241, 243, 249, 252, 256, 275 | unresolved resource identity `ic` |
| 242, 257, 262, 265, 266, 267, 270, 273 | indirect memory access |
| 254 | **arguments beyond the declared parameters of `flb_log_print`** — the C6 refusal PROD-CONTRACTS §5.3 isolated |
| 259 | call to uncontracted function `cio_chunk_down` |
| 275 | nullable resource `ic` used by a non-null contract |

After declaring the six, **only line 209 survives**. That is the `flb_error`
expansion on the `!chunk` arm:

```c
flb_log_print(1, ((void *) 0), 0,
              "[input chunk] could not create chunk file: %s:%s",
              storage->stream->name, name);
```

### 3.3 Every one of the six is load-bearing

`withdrawing_any_single_neighbour_leaves_the_constructor_refusing` withdraws
each of the six in turn, one at a time, from the assembled production text:
every single withdrawal leaves the constructor with at least one refusal. None
of the six is decoration.

`withdrawing_one_neighbour_still_reproduces_the_refusal` re-runs
PROD-CONTRACTS' own experiment against the new state: withdrawing
`input_chunk_write_header` **alone** reproduces the exact C6 tail refusal,
while the gate's branch condition stays decided. The experiment is unchanged;
what changed is that the declaration it withdraws now lives in
`contract/qcore`.

### 3.4 The residual, isolated

`withdrawing_the_chained_tail_actual_decides_the_constructor` replaces
`storage->stream->name` with the plain local array `name` — an explicitly
derived probe whose substitution is asserted to have applied — and the
constructor reports **zero** diagnostics. So the entire residual is one actual,
in one argument list, on one failure arm.

`storage` is a local bound at line 204 from `in->storage`; `storage->stream` is
`struct cio_stream *`; `->name` is `char *`. Card O04-ARMS' chained-access
admission requires every link to be declared exactly `struct P *`, and this
chain's **final field is pointer-typed**, so the chain rule does not reach it.
The read yields a pointer the projection does not track, and admitting it as a
borrowed data actual needs the same argument the tail premise makes — an
`untracked_bases`-style closure over a chain, which is C02-P-ALIAS territory,
not contract text. Named **`NEIGHBOURS-CHAINED-TAIL`** and sized in §8.

**This is the one refusal the two routes share.** It is present in the
production corpus too, at line 19581 (§4.3, row 1), so it is not a fixture
artifact — unlike the twenty other production residuals, which are.

**This is a fail-closed refusal doing its job**, and it is left in place.

### 3.5 What a function-level result would and would not be

Stated in advance, so nothing here can be quoted as more than it is. Even in the
probe's residual-free state, the result would be:

* **function-level.** `flb_input_chunk_create` only. The enclosing translation
  unit keeps its own verdict, and `the_real_program_is_not_promoted_by_this_card`
  pins that the fixture *program* is not `Safe` — `cio_chunk_open`, in the same
  text, keeps its own refusals, exactly as card O04 recorded.
* **relative to twelve named declared contracts**, each a trusted assumption
  about a body read by hand: this card's six, PROD-CONTRACTS' five, and
  HASH-CONTRACTS' one, plus the ChunkIO constructor/destructor pair and the
  memory family. None is a proof of the C.
* **not a promotion.** No raw unit moves; see §4.

---

## 4. Corpus impact — every leaf attributed

### 4.1 Provenance

Canonical pinned route, on the shared prepared pinned clone (Fluent Bit
`66910c10a4d7eafa810b84229cf2dcf7b0f26f97`, explicit `/usr/bin/clang`),
following [`docs/reports/corpus-repin/provenance.json`](../corpus-repin/provenance.json).
Provider packages were rebuilt for this run. Both runs use the same script,
[`build/neighbours/run-audit.sh`](#) (reproduced in
[`provenance.json`](provenance.json)):

```sh
python3 -B providers/package.py build --output build/cycle19/provider-packages-neighbours
QCORE_CLANG=/usr/bin/clang \
QCORE_CHUNKIO_PACKAGE=<worktree>/build/cycle19/provider-packages-neighbours/qcore.chunkio \
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  --pinned-evidence \
  <pin>/fluent-bit-clang-source \
  <pin>/monkey/monkey-build/compile_commands.json \
  <out>/monkey-source.json <out>/resource-diagnostics.json
```

The BEFORE run was started **before any file of this card's diff existed**, and
its `resource-diagnostics.contracts.json` records the nine-module manifest
verbatim, so the isolation carries no path provenance at all. Transcripts in
[`logs/`](logs/).

Both runs exit 0 with **zero gateway failures**.

### 4.1.1 The BEFORE run reproduces the current pinned baseline

[`docs/reports/wave14-corpus/artifacts/`](../wave14-corpus/artifacts/), the
current baseline:

| Artifact | wave-14 archived | BEFORE | AFTER |
|---|---|---|---|
| `diagnostics.json` | `40691eab1fdd7b34…` | **identical** | `1da0555bd5e3c5a6…` |
| `source.json` | `53d55b7e1fb174de…` | **identical** | **identical** |
| `chunk-contracts.json` | `c4f438e4ed3dab3c…` | **identical** | **identical** |
| `source-effects.json` | `5d857ba336be0a69…` | `c1fbb48a6374b40c…` | **identical to BEFORE** |
| `contracts.json` | `72e665be6308223f…` | `055fe4beb1f1492e…` | `4fa4fd1b441e4f9b…` |

The two artifacts that differ from the archived baseline in the BEFORE run are
the two residue classes CORPUS-REPIN and every consolidation since have
recorded: `contracts.json` path-literal provenance, and `source-effects.json`
package identity (the packages were rebuilt for this run). **`source.json`,
`chunk-contracts.json` and `source-effects.json` are byte-identical between
BEFORE and AFTER**, so neither residue class touches the isolation: the only
artifacts this card's diff moves are `diagnostics.json` and the three new module
texts plus the manifest inside `contracts.json`.

### 4.2 The census — every leaf attributed

[`census.py`](census.py) (card PROD-CONTRACTS', reused byte-for-byte apart from
its header and example paths), transcript at [`logs/census.txt`](logs/census.txt):

```
units: 185 -> 185;  added: [];  removed: []
unit status before/after: {'Safe': 2, 'Unknown': 137, 'Unsafe': 42, 'data_only': 4}
unit status moves: 0
NEWLY SURFACED PROVISIONAL UNSAFE UNITS: 0
unit reason_code moves: 0
projected functions: 4834 -> 4834;  function status moves: 0
leak: 271 -> 271 (+0);  unknown_alias: 4405 -> 4403 (-2);  unsupported: 49920 -> 49896 (-24)
units with diagnostic-count changes: 3
  chunkio/cio_chunk.c:              {'unknown_alias': -1}
  chunkio/cio_file.c:               {'unsupported': -6}
  fluent-bit-chunk/flb_input_chunk.c: {'unknown_alias': -1, 'unsupported': -18}
leaves added: 2; leaves removed: 28
```

**The 185-unit headline is `2 Safe / 42 provisional Unsafe / 137 Unknown /
4 data-only`, identical before and after, with zero unit status moves in either
direction, zero unit `reason_code` moves, zero function status moves across all
4,834 projected functions, NO newly surfaced provisional `Unsafe` unit, and leak
diagnostics unchanged at 271.** `monkey-source.json` and
`resource-diagnostics.chunk-contracts.json` are byte-identical; the three
checked function contracts / 93 Unknown source-effect split is unchanged.

**Every leaf, by rule.** Thirty leaves moved across three units. All thirty are
attributed:

| Δ | Category / description | Rule |
|---|---|---|
| **−4** | `` call to uncontracted function `cio_chunk_down` `` | `cio_chunk_down` declared — three in `chunkio/cio_file.c`'s `cio_file_content_copy` (L3177, L3185, L3194), one in the constructor (L19631) |
| **−2** | `` call to uncontracted function `cio_chunk_up_force` `` | `cio_chunk_up_force` declared — `cio_file_content_copy` L3167 and the constructor L19586 |
| **−1** each | `cio_chunk_is_up` (L19584), `generate_chunk_name` (L19577), `input_chunk_write_header` (L19593), `flb_time_now` (L19612) | each declared |
| **−1** | `` arguments beyond the declared parameters of `flb_log_print` `` | **the C6 refusal PROD-CONTRACTS isolated**, at the constructor's L19626. Its cause was the risky seeding of `tag` by the uncontracted `input_chunk_write_header`; that call is now modelled |
| **−14 / +1** | `indirect memory access is outside the resource projection` | the cascade: statements whose reaching abstract state was poisoned by an adjacent refusal now settle. Seven in the constructor (L19576, L19578, L19614, L19634, L19638, L19642, L19645), one in `flb_input_chunk_set_up_down` (L19961), three in `get_input_chunk_record_count` (L17985, L17988, L18009), two in `cio_file_content_copy` (L3165, L3173), one in `chunk_state_sync` (L2383); the `+1` at `cio_chunk_open` L2053 is attribution movement, §4.4 |
| **−3 / +1** | `unresolved resource identity` — `ch` 12→11, `chunk` 5→3, `ic` 33→34 | two removed at the constructor (L19588, L19595) as its `chunk` settles; one swapped at `cio_chunk_open` L2053 (§4.4); one **added** at the constructor's L19612, §4.2 below |

**Both added leaves are in the fail-closed direction**, which is what hard
invariant 1 demands and what this card wanted: contracting a callee does not
erase what its call site contains, it exposes it to the premises that must hold.
At the constructor's L19612 the `flb_time_now` call stops being an uncontracted
refusal and the store `ic->create_time = flb_time_now()` is examined instead,
where `ic`'s identity is unresolved — a refusal *replacing* a refusal, at the
same statement.

**Zero custody summary conflicts**, in every one of the units that report the
count, with the two file-`static` declarations of §2.5 in force.

### 4.3 The constructor in production, statement by statement

| | before | after |
|---|---|---|
| status | `Unknown` | `Unknown` |
| diagnostics | **36** | **21** |

Removed (15): L19576, L19578 (indirect access), L19577 `generate_chunk_name`,
L19584 `cio_chunk_is_up`, L19586 `cio_chunk_up_force`, L19588 unresolved
`chunk`, L19593 `input_chunk_write_header`, L19595 unresolved `chunk`, L19612
`flb_time_now`, L19614 (indirect), **L19626 the C6 tail refusal**, L19631
`cio_chunk_down`, L19634, L19638, L19642, L19645 (indirect). Added (1): L19612
unresolved `ic`.

By family, the whole vector:

| Family | before | after |
|---|---|---|
| refusals naming a callee (L19577, L19584, L19586, L19593, L19612, L19631) | **6** | **0** |
| the C6 tail refusal (L19626) | **1** | **0** |
| `indirect memory access` | 15 | **8** |
| `unresolved resource identity` (`ic`, `chunk`) | 13 | **12** |
| `nullable resource \`ic\` used by a non-null contract` (L19647) | 1 | 1 |
| **total** | **36** | **21** |

The **21 that remain**, none of which names a callee:

| Count | Description | Family |
|---|---|---|
| 1 | `indirect memory access` at **L19581** | the `flb_error` arm's chained tail actual — `NEIGHBOURS-CHAINED-TAIL`, the one blocker the fixture leg shares (§3.4) |
| 7 | `indirect memory access` at L19600, L19607, L19619, L19628, L19629, L19637, L19639 | the chained/aggregate read family (O04-ARMS, O04-B08, O04-B09B10 blockers) |
| 10 | `unresolved resource identity \`ic\`` at L19608-L19613, L19615, L19621, L19624, L19647 | the aggregate-aliasing family (C02-P-ALIAS, ALIAS-LOCALS) |
| 2 | `unresolved resource identity \`chunk\`` at L19601, L19620 | same family |
| 1 | `nullable resource \`ic\` used by a non-null contract` at L19647 | the nullable-handback family |

**Why production keeps twenty more residuals than the fixture leg.** The fixture
caller excerpt is byte-verbatim upstream, but its *prelude* is reduced: it
declares simplified `struct flb_input_chunk`, `struct cio_chunk` and
`struct flb_input_instance` shapes so the excerpt compiles standalone. The real
headers carry richer members — unions, bitfields, function-pointer fields and
nested aggregates — which C02's typed scalar-field whitelist and O04-ARMS' chain
rule refuse. Those refusals exist only where the real declarations are read, so
they appear in production and not in the fixture. This is a limitation of the
fixture as a proxy, recorded rather than smoothed over, and it is why §4 rather
than §3 is this card's production authority.

### 4.4 The attribution movement, recorded rather than smoothed over

Three leaves swap category and one appears, all in ChunkIO: in
`chunkio/cio_chunk.c`, `chunk_state_sync` L2383 loses an `indirect memory
access` while `cio_chunk_open` L2053 loses an `unresolved resource identity
\`ch\`` and gains an `indirect memory access`. **Both are refusals**, so no
statement changed from refused to admitted in either direction, and no exit,
function or unit verdict moves; only which of two refusals is reported does.
They cluster where a neighbouring statement became modelled, so the reaching
abstract state at them changed. This is the same phenomenon card PROD-CONTRACTS
recorded as `PROD-CONTRACTS-ATTRIB`, at a different site and in a smaller
quantity; the exact internal ordering that decides which of the two fires is
still not established here, and measuring it would need instrumentation this
card does not add.

### 4.5 The configured audits

Run in this same worktree with only `contract/cycle19-resource-contracts.json`
switched between the nine-module and twelve-module selections, through
`qcore-audit` — a completely different driver from the compatibility adapter §4.2
measures. Results in [`logs/`](logs/).

| Audit | Result | before vs after |
|---|---|---|
| `contract/profiles/cmetrics-counter` | 1 unit: 0 Safe / 1 provisional Unsafe / 0 Unknown; **1 checked function contract / 6 Unknown** | **byte-identical transcript AND byte-identical report JSON** (`142c0539…` both) |
| `contract/profiles/chunkio-core` | 13 units: 0 Safe / **2 provisional Unsafe** / 11 Unknown / 0 gateway failures — **unchanged** | transcript and report JSON **differ**, in exactly two units |

The CMetrics identity is the expected result: that source set calls no Fluent
Bit function and reaches no ChunkIO residency helper, so the three added modules
are in force and reach nothing there.

**The ChunkIO difference is the useful one, and it independently confirms §4.2
through a second driver.** `cio_chunk.c` moves `unknown_alias` 20 → 19 and
`cio_file.c` moves `unsupported` 293 → 287 — the same −1 and −6 the 185-unit
census attributes to `chunk_state_sync`'s category swap and
`cio_file_content_copy`'s five newly modelled residency calls plus two settled
indirect accesses. **No verdict moves in either unit or at unit level**, and the
profile's 0/2/11 split is unchanged. Two independent routes, the same movement,
the same non-movement.

---

## 5. Pin updates — stated loudly

| Pin | before | after | Why |
|---|---|---|---|
| `compiler/tests/qcore_contract_corpus.rs` corpus size | **64** | **67** | the three new modules |
| `compiler/tests/qcore_contract_corpus.rs` `EXPECTED_CONTRACTS` | 151 rows | **157** rows | six `extern fn` rows at their path-sorted positions, clause counts 3 / 3 / 3 / 5 / 6 / 2 |
| `compiler/tests/contract_loader.rs` corpus module count | **64** | **67** | same three files |
| `compiler/tests/contract_loader.rs` `(selected, v1, rejected)` | `(9, 25, 145)` | **`(12, 31, 145)`** | all three modules are entirely v1 and all three are manifest-selected, so the rejected count does not move |
| `compiler/tests/contract_loader.rs` `source_assumptions().len()` ×2 | **9** | **12** | the manifest selects twelve modules |
| `crates/qcore-c-gateway/tests/prod_contracts.rs` `source_assumptions().len()` | **9** | **12** | same; PROD-CONTRACTS' own five declarations, three modules and every other assertion are unchanged |
| `crates/qcore-contract/src/contract_loader.rs` `SCALAR_ABI_TYPES` | 11 spellings | **13** — `f32`, `f64` added | §2.6 |
| `compiler/tests/contract_loader.rs` refused-parameter list | `["unit", "f32", "f64", "ptr<const, ptr<const, u8>>"]` | **`["unit", "ptr<const, ptr<const, u8>>"]`** | the two floats move **into** the borrowed-only loop above, which checks them against every custody mode; the two remaining entries are the load-bearing half and are unchanged |
| `contract/cycle19-resource-contracts.json` | 9 modules | **12 modules** | a declaration must be in force to matter |
| `contract/qcore/README.md` | "nine modules … 25 declarations" | "twelve modules … 31 declarations" | plus two new subsections: declaring a file-`static` callee, and float scalars in the ABI table |

`production_contracts_preserve_the_previous_resource_modes` keeps its
load-bearing half — every *previous* declaration is still compared against the
historical table — and the six new declarations are listed **explicitly** in
that table, as additions, rather than exempted from the comparison. That is
cards HASH-CONTRACTS' and PROD-CONTRACTS' convention, followed unchanged.

**No other pin moved, in either direction.** The `third_party/libevent`
submodule revision, the loader's rejected-module count (145), every earlier
card's fixture premise set — `o04_path.qcore`, `o04_arms_callees.qcore`,
`o04_b08_callees.qcore`, `o04_b09b10_callees.qcore`,
`o04_read_bound_generic.qcore`, `o04_trace_callees.qcore`,
`hash_contracts_callees.qcore`, `prod_contracts_generic.qcore` — and the six
existing `contract/qcore` modules this card's tests load are **byte-unchanged**.

**The corpus baseline is NOT re-pinned here.** `artifacts/` is this card's
after-measurement, archived as its diff record with a manifest carrying each
SHA-256; the wave-14 baseline is untouched and stays current until an integrator
consolidation moves it. That is the integrator's call, and §4 is the input to it.

### 5.1 One pre-existing gate failure repaired in passing

`cargo clippy --all-targets -- -D warnings` **fails at base main `19fbaa5`**:
`crates/qcore-c-gateway/tests/prod_contracts.rs:699` trips
`clippy::manual_contains`. It is repaired here as a lint-only rewrite — the
asserted text is byte-identical and the assertion is unchanged — with the reason
recorded in a comment at the site. The wave-14 record states clippy clean; this
is a measured disagreement with that record, reported rather than smoothed over.
Named **`NEIGHBOURS-CLIPPY-BASE`**.

---

## 6. Lean — judged not applicable, with the judgment recorded

The card brief reads: *Lean: judged per content; record the judgment.*

**No new relation appears.** This card adds no decision procedure, no admission
rule and no diagnostic. It adds declared contract text, one entry pair in a
*writable-signature* table, and the pins that account for both. Every
load-bearing sentence it relies on is already carried by an audited theorem, and
each is named here rather than gestured at:

| Sentence this card relies on | Audited theorem |
|---|---|
| `borrowed` is the only v1 parameter mode with the caller-side effect each body was read to have | `QCore.LiteralArgumentPremise.borrowed_is_the_only_mode_with_the_measured_effect` |
| the tail declaration admits nothing by itself | `QCore.VariadicNested.the_declaration_alone_admits_nothing` |
| every admitted tail actual is data | `QCore.VariadicNested.every_admitted_tail_actual_is_data` |
| a tail actual that may hold an identity is never admitted | `QCore.VariadicNested.a_tail_actual_that_may_hold_an_identity_is_never_admitted` |
| the gate is admitted in a condition only under an exact arity, a `None` result and all-`Borrowed` parameters | `QCore.ReachGatePosition.a_neutral_call_of_admitted_arguments_is_admitted` and its three companions |
| an uncontracted call in a condition refuses | `QCore.ReachGatePosition.an_uncontracted_call_is_never_admitted` |
| a member read through a chain is admitted only under the declared-link premises | `QCore.ChainedAccess` (card O04-ARMS) |

The `SCALAR_ABI_TYPES` widening in particular introduces **no** new proof
obligation, and the reason is checkable rather than argued: the loader's own
mode/type consistency rules mean a float can occupy only the two positions every
existing scalar can occupy — a `resource_borrowed` parameter and a
`resource_return_none` result — and those two positions carry no custody by the
theorem in row 1. A widening that adds no reachable state adds no relation to
prove.

`python3 -B scripts/check_lean_audits.py` therefore stays at **64 audits**,
unchanged, and `docs/reviews/r0_3/acceptance_matrix.json`'s
`validation.lean_audits` tripwire does **not** fire. This is the same judgment
cards T00-CT-SCHEMA and PROD-CONTRACTS recorded, reached the same way.

---

## 7. Evidence — 12 assertions

[`crates/qcore-c-gateway/tests/neighbours.rs`](../../../crates/qcore-c-gateway/tests/neighbours.rs),
through the public gateway entry points only.

### 7.1 Held-out generic family

An `n_*` API deliberately unrelated to Fluent Bit, to ChunkIO **and** to the
`g_*` (O04), `h_*` (O04-ARMS), `k_*` (O04-B08), `r_*` (O04-READ-BOUND), `m_*`
(O04-B09B10), `v_*` (O04-TRACE), `p_*` (HASH-CONTRACTS) and `w_*`
(PROD-CONTRACTS) families. Its shapes mirror the production ones: a residency
predicate and two residency transitions, a name generator writing through a
caller buffer, a header writer, and a parameterless clock with a floating-point
result.

| Test | Result |
|---|---|
| `the_residency_shape_is_admitted_end_to_end` | the whole acquire/interrogate/transition/header/clock/transition-back/escape shape is admitted, the caller's obligation settles on every exit, and the certificate rechecks from its own retained sources |
| `an_admitted_residency_shape_does_not_hide_a_leak` | the same body with one failure arm's release deleted is `Unsafe` with **exactly one** leak: precision in the defect direction too |
| `the_residency_modes_are_load_bearing_in_both_directions` | **the sharp one.** Declared `borrowed`, a body that drops the handle after `n_down` still leaks; substituting `resource_released` — the mode a careless reading of "puts the chunk down" would reach for — **erases that leak**. This is exactly why the residency modes had to be read off the bodies |
| `withdrawing_each_generic_declaration_restores_its_refusal` | six substitutions, one per declaration, each restoring a refusal |
| `a_tracked_handle_is_never_admitted_where_the_projection_tracks_it` | the handle's **address** in a residency argument is not silently admitted under any declaration |
| `a_certificate_resting_on_the_new_modules_is_rederived_from_its_own_sources` | **contract substitution, one per new pointer-taking declaration**: each faithful `resource_borrowed` in turn replaced by `resource_released` in the certificate's own retained text, and the independent recheck refuses all five |

### 7.2 Real source

| Test | Result |
|---|---|
| `the_constructor_keeps_exactly_one_refusal_under_production_declarations` | **the headline.** 31 diagnostics before, 1 after, the whole vector pinned in both directions, all six callee names asserted in the before-state, and the C6 tail refusal asserted present-before/absent-after |
| `withdrawing_the_chained_tail_actual_decides_the_constructor` | the residual isolated by substitution: zero diagnostics without that one actual |
| `withdrawing_one_neighbour_still_reproduces_the_refusal` | PROD-CONTRACTS' isolation experiment re-run against the new state, now withdrawing production text |
| `withdrawing_any_single_neighbour_leaves_the_constructor_refusing` | all six load-bearing, one withdrawal at a time |
| `the_real_program_is_not_promoted_by_this_card` | the fixture program is **not** `Safe`; `cio_chunk_open` keeps its own refusals |
| `the_modules_under_test_are_the_production_ones` | the three modules are the files under `contract/qcore`, all six declarations are in force through the real production manifest at twelve selected modules, and the clock declaration carries `-> f64` and neither integer projection |

One new fixture,
`crates/qcore-c-gateway/tests/fixtures/contracts/neighbours_generic.qcore`,
carries the `n_*` declarations. Nothing else under `tests/fixtures/` changed.

### 7.3 Gate results

Run in this worktree at branch `worktree-agent-ad1192c365f7f1a48`, base main
`19fbaa5`, with the Libevent submodule initialised.

| Gate | Command | Observed | Exit |
|---|---|---|---|
| Workspace tests | `cargo test --workspace --no-fail-fast` | **2,548 passed / 0 failed / 8 ignored** | 0 |
| Format | `cargo fmt --all -- --check` | clean | 0 |
| Clippy | `cargo clippy --all-targets -- -D warnings` | clean **after** the §5.1 repair; **fails at base** without it | 0 |
| Probe | `cargo run -p qcore-compiler --example reevaluation_probe` | ran | 0 |
| Follow-up | `cargo run -p qcore-compiler --example reevaluation_followup` | ran | 0 |
| Lean audits | `PATH=$HOME/.elan/bin:$PATH python3 -B scripts/check_lean_audits.py` | **64 audits passed**, unchanged (§6) | 0 |
| Provider tests | `cargo test --locked --offline --manifest-path providers/Cargo.toml --workspace` | all targets pass | 0 |
| Provider packages | `python3 -B providers/package.py build` | five packages built | 0 |
| Python | `python3 -B -m unittest discover -s scripts/tests` | **135 ran, 0 failures** | 0 |
| 185-unit corpus | pinned route, before and after | §4, zero gateway failures both runs | 0 / 0 |
| Configured audits | `qcore-audit` over cmetrics-counter and chunkio-core, before and after | §4.5 | 0 / 0 |

**Load-flaky.** None of the three recorded load-flaky cases (`vp04`, P02
`RLIMIT_NPROC`, P03 confined-write) failed. No assertion was touched.

### 7.4 What this card contributes

**12 new assertions** in one new integration test file, **three new production
contract modules** (six declarations, 22 clauses), **one new test fixture**, and
one entry pair in the loader's writable-signature table. No existing test was
removed or renamed; two existing assertions moved their pinned counts and one
was rewritten lint-only (§5.1). **Zero new Lean theorems**, by the recorded
judgment of §6, so the Lean audit count and the acceptance matrix's
`validation.lean_audits` tripwire are both unmoved.

---

## 8. Open boundaries

Carried forward unchanged: O04's blockers **B02-B07** and **B11**;
**O04-ARMS-SCALAR-CHAIN**; **C02-P-ALIAS**; **P-B08-SHALLOW**,
**O04-B08-BORROWED-AGG**, **O04-B08-DOT**, **O04-B08-CAST**;
**O04-READ-BOUND-SCALAR**, **O04-READ-BOUND-REACH**; **O04-B09B10-FRAME**,
**O04-B09B10-REACH**; **O04-TRACE-DEPTH**, **O04-TRACE-EFFECTFUL**,
**O04-TRACE-HANDBACK**, **O04-TRACE-UNDERSUPPLY**, **O04-TRACE-TAILTRUST**;
**HASH-CONTRACTS-CLAUSE**, **HASH-CONTRACTS-PAIRING**,
**HASH-CONTRACTS-RETENTION**, **HASH-CONTRACTS-CALL-POSITION**;
**PROD-CONTRACTS-PLG**, **PROD-CONTRACTS-MASK-EXTENT**,
**PROD-CONTRACTS-ATTRIB**, **PROD-CONTRACTS-TAIL-COST**,
**PROD-CONTRACTS-UNDECLARED**.

**`PROD-CONTRACTS-NEIGHBOURS` is closed by this card**: all six named callees
are declared from `contract/qcore`, each against its read body, and the
constructor's refusal set is measured at 31 → 1. It is **not** closed by a
discharge, and saying so is the point.

| # | Boundary | Status |
|---|---|---|
| **NEIGHBOURS-CHAINED-TAIL** | the one surviving refusal: a tail actual `storage->stream->name`, a two-level chained pointer-field read whose final field is pointer-typed. Isolated by substitution (§3.4); zero diagnostics without it. Admitting it is checker work with a real soundness obligation — the read yields a pointer the projection does not track — and needs an `untracked_bases`-style closure over a chain, not a declaration | **open, isolated, sized at one actual** |
| **NEIGHBOURS-RESIDENCY-ENDPOINTS** | `cio_chunk_up_force`/`cio_chunk_down` move a descriptor and a mapping *inside* the chunk. The projection models the caller's custody of the handle only; it neither tracks those endpoints nor asserts `cio_chunk_close` releases them | **open, a non-claim** |
| **NEIGHBOURS-NAME-EXTENT** | `resource_borrowed(out_buf)` is a custody statement; it does not say `snprintf`'s write is inside the caller's array, and the body's `buf_size - 1` is not related to it here | **open, a non-claim** |
| **NEIGHBOURS-UNDECLARED** | `cio_chunk_up`, `cio_chunk_is_file`, `flb_input_chunk_write_header_v2`, `flb_time_get`, `flb_time_to_double`, `flb_time_to_nanosec` were read or sized and deliberately left undeclared, each on a measured ground (§2.8). `flb_input_chunk_write_header_v2` in particular was **not** read to the standard this card requires, and declaring it on its neighbour's shape would be the P07 defect | **recorded** |
| **NEIGHBOURS-FIXTURE-PROXY** | the `fixtures/o04` caller excerpt is byte-verbatim but its **prelude is reduced**, so the real headers' richer member types are absent and the fixture leg reaches 1 residual refusal where the production corpus reaches 21 (§4.3). Seven cards have now measured that fixture; this is the first to measure both routes on the same function in the same change and record the gap. Anything called a production result must come from §4, not §3 | **recorded, newly measured** |
| **NEIGHBOURS-CLIPPY-BASE** | `cargo clippy --all-targets -- -D warnings` fails at base main `19fbaa5` on a pre-existing `clippy::manual_contains` in `prod_contracts.rs`; repaired here lint-only, the disagreement with the wave-14 record reported (§5.1) | **repaired, recorded** |
| **NEIGHBOURS-FLOAT-REACH** | `f32`/`f64` are now writable. Exactly ONE declaration uses one, and no other corpus function was surveyed for a floating-point signature. The widening is not a claim that the float vocabulary is complete or that any other float-taking API should be declared | **recorded** |
| — | a residency argument carrying `&`; a substituted `resource_released` mode; a withdrawn declaration at any of the six sites; a tail actual naming a risky name | **refused by construction, and asserted** |

---

## 9. Scope and non-claims

- **No raw unit is promoted.** See §4 for the measured movement.
- **Every declaration here is a trusted assumption.** Each was read against the
  pinned upstream body and cited by file and line, and nothing in this card
  analyzes a C body. A function-level result resting on one is relative to that
  declaration; it is never a proof of the C implementation and must never be
  presented as one.
- **The constructor is not `Safe` in production**, at any level, and this card
  does not claim it is. It carries one refusal, and that refusal is left in
  place because removing it soundly is not contract-text work.
- **A decided statement is not a `Safe` function, and a `Safe` function is not a
  `Safe` unit.** The fixture program keeps `Unknown` and the unit keeps its
  provisional `Unsafe`.
- **A provisional `Unsafe` is never an upstream defect claim**, here or
  anywhere; the `n_*` family is synthetic and its `Safe` is `Safe` for a
  synthetic declared custody projection.
- **`resource_borrowed` says nothing about validity, bounds or retention.** The
  header writer reads `tag_len` bytes through `tag`; the name generator writes
  through `out_buf`; the residency transitions open and close OS endpoints
  inside the chunk. None of those is a custody fact and none is modeled.
- **Nothing about the upstream code is alleged.** The `buf_size - 1` observation
  in §2.3 and the v2 header writer's unread arms in §2.8 are statements about
  what this card did and did not establish, not about defects.
- **The containment publications remain unmodeled** (O00 blockers B03/R13).
- **Unknown means Unknown.** A statement whose path carried a refusal alleges
  nothing in either direction.
- Projection coverage is not safety coverage; unknowns stay in the denominator.
- CT coverage is unchanged at 32/50; `qcore-schemas` is unchanged at 11.0.0; no
  plugin, protocol or evidence version moved; no diagnostic code was added,
  removed or downgraded.

---

## 10. Files changed

| Path | Change |
|---|---|
| `contract/qcore/chunkio/chunk_residency.qcore` | **new** — `cio_chunk_is_up`, `cio_chunk_up_force`, `cio_chunk_down`; the header carries the body reading, the `free(cf)`-site check and the endpoint/relinking non-claims |
| `contract/qcore/fluent_bit/input_chunk_helpers.qcore` | **new** — `generate_chunk_name` and `input_chunk_write_header`, argument by argument, with the copy chain that justifies the header writer's double `flb_free` |
| `contract/qcore/fluent_bit/time.qcore` | **new** — `flb_time_now`, parameterless, `-> f64`, with the vocabulary-repair reasoning |
| `contract/cycle19-resource-contracts.json` | 9 → 12 selected modules |
| `contract/qcore/README.md` | the selection paragraph, plus "A file-`static` callee may be declared" and "Float scalars in the ABI table" |
| `crates/qcore-contract/src/contract_loader.rs` | `SCALAR_ABI_TYPES` gains `f32`/`f64`; both doc notes rewritten to record the condition that was met |
| `compiler/tests/qcore_contract_corpus.rs` | corpus size 64 → 67; six `EXPECTED_CONTRACTS` rows |
| `compiler/tests/contract_loader.rs` | module count 64 → 67; `(9,25,145)` → `(12,31,145)`; two `source_assumptions().len()` pins 9 → 12; the scalar table pin; the refused-parameter list; six explicit rows in the historical-mode table |
| `crates/qcore-c-gateway/tests/prod_contracts.rs` | one pin 9 → 12; one lint-only rewrite (§5.1) |
| `crates/qcore-c-gateway/tests/neighbours.rs` | **new** — 12 assertions |
| `crates/qcore-c-gateway/tests/fixtures/contracts/neighbours_generic.qcore` | **new** — the held-out `n_*` API |
| `docs/reports/remaining-neighbours/` | this record, the census and archive tools, the artifacts, the full diff and the transcripts |

No production Rust changed except the one scalar-table entry pair and its
doc notes. No file owned by the integrator was touched: `AGENTS.md`,
`CLAUDE.md`, `README.md`, `DEVELOPMENT.md`, `docs/reevaluation/ct_manifest.json`,
`docs/reviews/r0_3/acceptance.json` and
`docs/reviews/r0_3/acceptance_matrix.json` are unchanged, as is every archived
predecessor report and every existing contract module.
