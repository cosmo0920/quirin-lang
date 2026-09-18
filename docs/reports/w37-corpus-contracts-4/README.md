# W37-CORPUS-CONTRACTS-4 — ten custody-neutral declarations, and a measured ABI boundary

Wave 37's **second declarative arbiter**, chained behind
`JOINED-PROVENANCE-ROUND-BOUND` (and, beneath it, `SCALAR-ROUTE-EXIT-EDGE` and
`JOINED-PROVENANCE-CALL-FRAMES`) after the rebase of §1.0.
**Declarative**: six `.qcore` contract modules and six manifest rows. No
gateway checker, loader, backend or any other crate source changed, so **one
audit binary served both stages** and the only difference between BEFORE and
AFTER is which modules the manifest selects.

Base **`797661f3`** (rebased from `477b733d`; §1.0). Baseline
`docs/reports/joined-provenance-call-frames/artifacts/`.

---

## Headline

| Measure | BEFORE | AFTER | Note |
|---|---|---|---|
| Units | 2 Safe / 42 Unsafe / 137 Unknown / 4 data-only | **identical** | **zero unit moves** |
| Projected functions (4,834) | 205 Safe / 84 Unsafe / 4,545 Unknown | **213 Safe / 84 Unsafe / 4,537 Unknown** | **+8 Safe, −0**; no `Safe` lost |
| `leak` | 272 | **272** | **pin holds** (constraint 7), no re-pin needed |
| `unsupported` | 49,922 | **49,753** | **−169** |
| `unknown_alias` | 4,308 | **4,309** | **+1**, and it is a same-position substitution |
| `summary_conflict` | 1 | **1** | unmoved; no new conflict |
| Contract corpus | 82 modules | **88** | constraint 5 pins updated deliberately |
| Manifest-selected | 27 | **33** | |
| resource-v1 declarations | 49 | **59** | rejected unchanged at 145 |

Leaves: **183 removed, 15 added, net −168.** `flb_input_chunk_create` remains
function-level **Safe**.

**The strongest single fact in this measurement:** of the 15 added leaves,
**all 15 are same-position text substitutions and none is a genuine addition**
(`logs/attribution.txt`, which separates the two mechanically rather than by
eye). No position in the 185-unit corpus refuses that did not refuse before.
This card is purely subtractive on refusals: 168 positions stopped refusing,
15 changed *which* refusal they report, zero started.

Every `Safe` here is `Safe` **relative to ten declared premises**, never a
proof of the C (invariant 2).

---

## 1. The chained re-pin, and which baseline

The brief named `docs/reports/joined-provenance-call-frames/artifacts/`, and
that was **checked rather than assumed**: its `diagnostics.json`,
`chunk-contracts.json` and `source.json` decompress byte-identically to
`w36-corpus-contracts-3`'s AFTER (`a7aa3b5f9ab193a0`, `c4f438e4ed3dab3c`,
`ae38f6c93dfbe341`), i.e. `JOINED-PROVENANCE-CALL-FRAMES` moved neither
verdict-bearing artifact. Only `contracts` and `source-effects` differ between
the two archives, which is the checkout-root class every re-pin in this chain
has recorded.

### 1.0 The card was REBASED, and both stages were re-run on the new base

The card was first measured at base `477b733d`. `origin/main` then advanced
three times, and on the integrator's instruction the card was **rebased onto
`797661f3`** and **both stages re-run there**. Three cards now sit between this
card's original base and its new one:

| Merged | Card | Kind | Effect on the corpus |
|---|---|---|---|
| `c1814e3c` | `BASE-TYPE-READER-CENSUS` | measurement only | none |
| `0b0b4ac1` | `SCALAR-ROUTE-EXIT-EDGE` | **gateway code** — a loop header's exit edge is retained | zero movement claimed |
| `797661f3` | `JOINED-PROVENANCE-ROUND-BOUND` | **gateway code** — the 64-round cap becomes a proved per-graph bound | zero movement claimed, no archive published |

Two of the three change gateway code *beneath* this card, so their
zero-movement claims are premises this card's own re-run can check rather than
inherit. **The audit binary was rebuilt on the rebased tree** (sha256
`05011d2f…`) before either stage, so both stages exercise the new gateway code.

**Result: nothing moved, anywhere.**

* **BEFORE, re-run on the rebased tree with this card's six modules staged off
  disk** (82 modules present, manifest at 27), reproduces the wave-36 baseline
  byte for byte — `diagnostics` **`a7aa3b5f9ab193a0`**, `chunk-contracts`
  **`c4f438e4ed3dab3c`** (`logs/byte-identity-before2.txt`). Since both code
  cards' AFTER is byte-identical to that same baseline, this one run reproduces
  **all three** cards' AFTER at once, and it is an **independent confirmation of
  both zero-movement claims** measured from outside those cards.
* All **five** BEFORE artifacts are byte-identical to the pre-rebase BEFORE,
  including the checkout-root-bearing ones (the worktree path is unchanged).
* **AFTER, re-run on the rebased tree, is byte-identical to the archived AFTER
  in all five artifacts** — `diagnostics` `c54ad4219fa85793`, `chunk-contracts`
  `c4f438e4ed3dab3c`, `source` `ae38f6c93dfbe341`, `source-effects`
  `c199353eddb00e20`, `contracts` `fe945ffb298efc2b`
  (`logs/rebase-after-identity.txt`).

So **the entire attribution in §4 and §5 stands unchanged**, every count in the
Headline is re-measured rather than carried, and there is **no interaction** to
name between this card's ten declarations and either the retained exit edge or
the raised round bound. That was the one finding the re-run existed to look
for, and it is a negative result — reported as one.

The staging-out used a temporary file move into the gitignored `measure/`
directory and `git checkout` of the manifest, never `git stash`: the stash stack
is shared across worktrees (`CLAUDE.md`). The tree was verified restored to 88
modules / 33 selected afterwards.

### 1.1 BEFORE reproduces the baseline byte for byte

```
*diagnostics      raw IDENTICAL  canonical IDENTICAL   a7aa3b5f9ab193a0 / a7aa3b5f9ab193a0
 source           raw IDENTICAL  canonical IDENTICAL   ae38f6c93dfbe341 / ae38f6c93dfbe341
*chunk-contracts  raw IDENTICAL  canonical IDENTICAL   c4f438e4ed3dab3c / c4f438e4ed3dab3c
 source-effects   raw differs    canonical differs
 contracts        raw differs    canonical differs
RESULT: the verdict-bearing artifacts reproduce the baseline BYTE FOR BYTE.
```

(`logs/byte-identity-before.txt`.) **This is the load-bearing fact of the whole
measurement**: because BEFORE reproduces the baseline exactly, everything in §5
and §6 is these ten declarations and nothing else.

`logs/byte-identity-after.txt` is the same tool pointed at AFTER, where
`diagnostics` *is expected* to differ — that is this card's movement — so its
closing `RESULT:` line reads as a failure and should not be misread: it is the
BEFORE gate's sentence printed against the AFTER run. The three artifacts that
should not move did not: `chunk-contracts`, `source` and `source-effects` are
**byte-identical between BEFORE and AFTER**. The ten declarations move no chunk
contract at all.

### 1.2 One binary, both stages

The card is declarative, so within each measurement BEFORE and AFTER are the
**same executable** (`target/release/examples/cycle19_fluent_bit_libraries_audit`).
There is no "was it the same build?" question to answer: the manifest was at its
27-module form for BEFORE and at 33 for AFTER, and nothing else differed.

The card was measured **twice**, once at each base, and each measurement used one
binary built at *that* base — the rebase's whole point being that the second
binary contains the two gateway code changes the first did not:

| Measurement | Base | Binary sha256 | BEFORE | AFTER |
|---|---|---|---|---|
| first | `477b733d` | (built at that base) | 213.9s, exit 0 | 465.5s, exit 0 |
| **rebased** | **`797661f3`** | `05011d2f…` | 228.5s, exit 0 | 263.6s, exit 0 |

Both measurements agree artifact for artifact (§1.0).

---

## 2. The cover, re-derived — and the finding that shaped the card

Re-derived from the baseline rather than carried (`scripts/cover.py`,
`scripts/greedy.py`, `logs/cover-baseline.txt`, `logs/greedy-baseline.txt`):

* **163 projected functions are blocked *only* by `call to uncontracted
  function` leaves** (W36 measured 193 before its own harvest). All 163 are
  `Unknown`.
* Distinct-callee histogram `{1: 99, 2: 34, 3: 21, 4: 7, 6: 2}` — **99 have
  exactly one blocker.**
* The sole-blocker ranking is now **very flat**: `memcpy` 7, `add` 3, then
  `memset`, `rd_kafka_destroy_app`, `rd_kafka_topic_partition_destroy0` and
  `rd_kafka_topic_partition_list_sort` at 2, and a long tail of 1s. W36's top
  was 8.

### 2.1 The frontier is now mostly `static` helpers, and that is the card's finding

Twenty-two candidate callees were located in the pinned tree with their
definitions, linkage, headers and **definition counts**. The result is
structural, not incidental:

| Class | Count | Examples |
|---|---|---|
| `static` / `static inline`, **defined in the very unit that calls it** | 10 | `rd_kafka_topic_partition_destroy0`, `rd_kafka_destroy_app`, `sds_cat_safe`, `set_result`, `otlp_kvpair_list_destroy`, `destroy_context_list`, `crc32c_zeros`, `pack_kvlist`, `pack_context_metrics`, `rd_kafka_dump0` |
| Several definitions with **disagreeing** signatures | 3 | `set_result` (7 definitions, 2 signatures), `pack_context_metrics` (2, different arity), `cmt_atomic_initialize` (5, `inline` without `static`/`extern` against a plain extern prototype) |
| External, single or **identical** definitions, header-declared | 9 | the ten this card declares, less one |

A resource contract is **keyed by name**. For the first two classes a name-keyed
declaration is either meaningless (there is no extern C declaration for
constraint 8's ABI gate to agree with) or actively wrong (it would speak for a
neighbour's shape). For a body **defined in the unit that calls it**, W4 — not
declaration — is the correct lever, which is constraint 4's own division of
labour.

**So the inverse of W36 is not a coincidence.** W36 landed five releases and one
neutral declaration; this card lands **ten neutral declarations and not one
release**. W36 harvested the release-shaped callees; what the sole-blocker
frontier still offered afterwards to a *name-keyed* declaration was
custody-neutral, because the release-shaped remainder had gone `static`. That
asymmetry is recorded in the pinned table's own comment
(`compiler/tests/contract_loader.rs`, the all-borrowed loop) so it survives as
a fact rather than as this report's rhetoric. Residual
**`W37-COVER-FRONTIER-IS-STATIC`**.

### 2.2 Selection was by set yield, not by sole-blocker count

Choosing by sole-blocker count alone under-counts a *set*: a function with two
blockers moves when **both** are declared. The greedy cover
(`scripts/greedy.py`) makes that visible — `sds_cat_safe` contributes 1 alone
but 4 once `snprintf` is present. The chosen ten were scored as a set: **11
functions**, against a sole-blocker sum of 11 (they happen to coincide here
because no chosen pair shares a target, which was checked rather than assumed).

Leaf mass was deliberately **not** the key: `fprintf` carries 1,138 leaves and
is the sole blocker of **zero** functions. W36 made this point and this card
re-measured it rather than inheriting it.

---

## 3. The ten declarations, each read before it was declared

Every body was read from the pinned checkout
(`build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source`) at Fluent Bit
`66910c10a4d7eafa810b84229cf2dcf7b0f26f97` **on this card's own run**, not
carried from any predecessor. Each module's own comment block carries the full
line-cited reading, the ABI paragraph and its own "NOT CLAIMED" list.

| Module (new) | Callee(s) | Arity | Sole blocker of |
|---|---|---|---|
| `contract/qcore/librdkafka/topic_partition.qcore` | `rd_kafka_topic_partition_list_sort` | 3 | 2 |
| `contract/qcore/mpack/writer.qcore` | `mpack_write_cstr`, `mpack_write_str`, `mpack_write_i64`, `mpack_write_double` | 2/3/2/2 | 1 each |
| `contract/qcore/cmetrics/metric_values.qcore` | `cmt_metric_get_timestamp`, `cmt_metric_set_double` | 1/3 | 1 each |
| `contract/qcore/cmetrics/mpack_utils.qcore` | `cmt_mpack_consume_uint_tag` | 2 | 1 |
| `contract/qcore/cfl/time.qcore` | `cfl_time_now` | 0 | 1 |
| `contract/qcore/librdkafka/lz4_stream.qcore` | `LZ4_resetStream` | 1 | 1 |

**Every one is the neutral element** — every formal `resource_borrowed`, result
`resource_return_none()`. `mpack/` is a **new contract family**, the corpus's
second after W36's `ctraces/`.

### 3.1 Four readings worth stating here

* **`mpack/` is not `msgpack/`.** `contract/qcore/msgpack/packer.qcore`
  declares `msgpack_packer_init` of **msgpack-c**; `mpack_write_*` belongs to
  **mpack**, a different library with a different API, vendored beside it. The
  two families are adjacent in the pinned table on purpose, so a reader meets
  the distinction where confusing them would be easiest.
* **Four vendored copies are not `ADD-NAME-COLLISION`.** Each mpack name has
  **four** definitions (the amalgamation copy plus cmetrics', ctraces' and
  cprofiles'). W36 declined `add` for having three definitions — but `add`'s
  three **disagree** (three signatures, one `static inline` with no header).
  The four mpack copies were **compared** and spell each signature identically
  with external linkage and a matching prototype in each copy's own header.
  The distinction that matters is not *how many* definitions but *whether they
  agree*; measured, not assumed.
* **An `Out` parameter carrying a scalar is `borrowed`, not a custody `Out`.**
  `cmt_mpack_consume_uint_tag(reader, uint64_t *output_buffer)` writes a
  **value** through its out slot, not a resource identity. Declaring a custody
  `Out` would assert the caller acquires an obligation it does not acquire —
  and constraint 10 records that a declared `Out` naming an outliving
  destination is refused as the `Acquire` arm seen from the callee's side.
* **Zeroing is not releasing.** `LZ4_resetStream` is one `MEM_INIT` (memset)
  through its parameter; the freeing neighbour `LZ4_freeStream` is a *different
  name*, already decided through W36's `rd_kafka_mem_free`. The two neighbours
  also differ on null: `LZ4_freeStream`'s first statement is
  `if (!LZ4_stream) return 0;  /* support free on NULL */` while
  `LZ4_resetStream` dereferences immediately — recorded even though v1 carries
  a nullability marker only on a *released* parameter, so neither could express
  it here.

### 3.2 `cfl_time_now`'s unprototyped header, settled by precedent not hope

`lib/cfl/include/cfl/cfl_time.h:25` spells `uint64_t cfl_time_now();` — an
**empty parameter list**, which in C is *unspecified parameters*, not arity
zero. Whether constraint 8's arity gate accepts a zero-arity contract against
that is a real question. It was answered by measurement before the declaration
was written: the corpus **already** carries `flb_time_now` against the
identically-shaped `FLB_EXPORT double flb_time_now();`
(`include/fluent-bit/flb_lib.h:94`), manifest-selected and not withdrawn. The
AFTER run confirms it — `cfl_time_now`'s nine leaves were all decided and
`ctr_span_end` reached `Safe`.

### 3.3 The duty gate, and why no `SummaryConflict` arose

Six of the ten are defined in **analysed** units (`cmt_metric.c`,
`cmt_mpack_utils.c`, `lz4.c`, `rdkafka_partition.c`); `mpack.c` and
`cfl_time.c` are not among the 185, checked against the baseline's own unit
list. So constraint 8's duty gate applied to six, and `summary_conflict`
nevertheless stayed at **1**.

That was forecast on a structural ground rather than optimism, and the ground
is worth keeping: all ten declarations are the **neutral element** `N(m)`,
which is the *weakest* statement the vocabulary can make, so a derived summary
can contradict one only by claiming a formal is released, transferred,
delivered or restored — and none of these bodies performs any such operation on
any parameter. W36's `rd_kafka_mem_free` conflict arose precisely because that
declaration was **not** neutral (it declared a release, and W4 derived the same
release with the opposite nullability). **Declaring the neutral element is the
one declaration that cannot conflict with a faithful summary.**

### 3.4 Lean: none added, with the argument

No Lean module is added. Invariant 6 binds **semantic changes** — definitions
and theorems must match a changed semantic surface. This card changes no
semantic surface: it adds premises to a corpus the loader already consumes,
through a vocabulary (`resource_contract_v1`, `resource_borrowed`,
`resource_return_none`) whose meaning, loader and checker are all unedited. A
**grown premise set is not a semantic change**. This is `W31-CORPUS-CONTRACTS-2`'s
and `W36-CORPUS-CONTRACTS-3`'s recorded position, taken on the same ground and
stated rather than assumed.

---

## 4. Predicted vs measured, both ways

The prediction was computed **from BEFORE only** and committed at `219d3f01`
**before the AFTER run was started** (`PREDICTION.md` in this directory), so
this is a real comparison and not a retrofit.

| | Predicted | Measured |
|---|---|---|
| Leaves named by the ten, corpus-wide | **179** | **179 — exact** |
| Functions losing their last *visible* blocker | **11** | **11 — exact** |
| Of those, reaching `Safe` | **10** | **8** |
| Of those, EXCHANGED for another refusal | **1** (`pack_string`) | **3** |
| Of those, UNMOVED | **0** | **0** |
| Unit status moves | **0** | **0** ✓ |
| Function status moves | — | **8**, all `Unknown → Safe`, none lost |
| `leak` | **272 → 272** | **272 → 272** ✓ |
| `summary_conflict` | **1 → 1** | **1 → 1** ✓ |
| `unknown_alias` | "much smaller than W36's +94" (an expectation, recorded as one) | **+1** |

### 4.1 The exchange forecast was right where it mattered, and exactly right

The brief asked for the exchange to be **predicted** rather than discovered,
because W36's `W36-COVER-HIDDEN-BLOCKER` recorded that a sole-blocker cover is
an upper bound and never a forecast. `pack_string` was forecast to exchange, on
the ground that `ctr_encode_msgpack.c:37` is

```c
mpack_write_str(writer, str, cfl_sds_len(str));
```

with `cfl_sds_len` a **real external function nested in the modeled call's
argument list**, emitting no leaf today only because the outer call refuses
first. The measurement:

```
ctraces/ctr_encode_msgpack.c :: pack_string L6772 origin=ctr_encode_msgpack.c:37
  was: [unsupported] call to uncontracted function `mpack_write_str`
  now: [unsupported] nested call `cfl_sds_len` inside a modeled call is
                     outside the resource projection
```

**The forecast named the mechanism, the function and the inner callee, and all
three were right.** The method is therefore demonstrated, not merely proposed:
reading the target body ex ante converts `W36-COVER-HIDDEN-BLOCKER` from a
surprise into a prediction.

It also found two exchanges *nobody was looking for*: `cmt_encode_msgpack.c ::
pack_metric` gained the same nested-call refusal twice
(`cmt_metric_hist_get_sum_value`, `cmt_metric_get_int64_value`) at positions
that previously named `mpack_write_double`/`mpack_write_i64`. That function was
not among the 11 (it holds other blockers) and stays `Unknown`. Residual
**`NESTED-CALL-IN-ARGUMENT-POSITION`**, which is `CJSON-DELETE-NESTED-DETACH`
recurring in a third library and is now the single most common way a decided
leaf is exchanged rather than discharged.

### 4.2 Where the prediction was wrong, and it was not the exchange mechanism

The two misses are both `rd_kafka_topic_partition_list_sort`, and they are
**not** hidden blockers: the declaration was **withdrawn**. §5.1.

---

## 5. Every count that moved, attributed

`logs/census.txt` prints every moved leaf individually at the **internal
preprocessed coordinate** that is its identity, with the presented `origin`
beside it (constraint 12a). `logs/attribution.txt` groups them and, uniquely to
this card, **separates a genuine addition from a same-position substitution** —
a distinction a raw added/removed count cannot make and which turns out to
carry the whole result.

Everything in this section was **re-measured on the rebased tree** and is
byte-identical to the pre-rebase measurement (§1.0), so no figure here is
carried across the rebase.

**Removed: 183.** 179 name one of the ten declared callees — the prediction,
exactly. The other 4 are cascade: 1 naming `pack_cmetrics_type` (which became
`Safe`, so W4 now derives it and its own caller's leaf is decided) and 3
`indirect memory access` rows, two of which are the substitution sites in §5.1
and one in `cmt_decode_msgpack.c :: unpack_metric_value` at L7420.

**Added: 15 — and every one is a same-position text substitution.** Zero
genuine additions. Broken down: 11 are the withdrawal rows of §5.1, 3 are the
nested-call exchanges of §4.1, and 1 is the lone `unknown_alias`:

```
ctraces/ctr_span.c :: ctr_span_event_add_ts L7059 origin=ctr_span.c:436
  was: [unsupported] call to uncontracted function `cfl_time_now`
  now: [unknown_alias] unresolved resource identity `ev`
```

That is W36's mechanism in miniature — once a call is modeled, a later mention
of an operand is an unresolved identity rather than an unexamined one — and it
is in the **accusing** direction. It is also the entire `unknown_alias` move:
**+1**, against W36's +94, which is what a card of ten custody-neutral
declarations should produce and is here measured rather than asserted.

### 5.1 `LOADER-ABI-INLINE-FNPTR-DECLARATOR` — the declaration the ABI gate withdrew

`rd_kafka_topic_partition_list_sort` was this card's **highest-yield** callee at
two functions. Its declaration is **withdrawn in every translation unit that
calls it**:

```
call to `rd_kafka_topic_partition_list_sort` whose declared contract was
withdrawn against its C declaration in this translation unit
(abi-unreadable-declaration)
```

so both target functions stay `Unknown`, and the two predicted `Safe` moves did
not happen.

**The cause was diagnosed, not guessed.** `crates/qcore-c-gateway/src/legacy_declared_abi.rs`
records that card `CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF` taught the reader two
shapes it previously withdrew on, one of them a **function-pointer typedef**
(`typedef int (*msgpack_packer_write)(void *, const char *, size_t);`). That is
exactly why `msgpack_packer_init`'s `callback: fnptr` works: its C parameter is
spelled with a **named typedef** (`lib/msgpack-c/include/msgpack/pack.h:36`
and `:43`). This card's callee is spelled with an **inline function-pointer
parameter declarator** and no typedef at all:

```c
RD_EXPORT void rd_kafka_topic_partition_list_sort(
    rd_kafka_topic_partition_list_t *rktparlist,
    int (*cmp)(const void *a, const void *b, void *cmp_opaque),
    void *cmp_opaque);                         /* rdkafka.h:1163-1166 */
```

The reader resolves the typedef form and not the inline declarator form, and
**fails closed** — which is the gate working exactly as constraint 8 specifies.
The projection it would need already exists (`FUNCTION_POINTER_ABI_TYPE`), so
the discharge condition is narrow: extend the parameter declarator reader to
the inline form, on `CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF`'s own precedent. This
is the **first measurement of that boundary from production text**.

**The withdrawal costs zero leaves**, which is why the module was kept rather
than reverted. All 11 withdrawal rows are same-position substitutions:

* **9** replace the `call to uncontracted function
  rd_kafka_topic_partition_list_sort` row at the identical
  `(library, file, function, line, origin)` — verified as a set equality, not
  by counting;
* **2** (`rdkafka_assignment.c:991` and `:876`) replace an
  `indirect memory access is outside the resource projection` row at that same
  position, because the withdrawal check now fires where the indirect-access
  refusal previously did. Both functions' leaf counts are **unchanged** (30 →
  30 and 15 → 15).

Keeping the module therefore adds no refusal anywhere, preserves the reading
for a later card, and is what makes the boundary measurable at all. Reverting
it would have deleted the only evidence for the residual. Residual
**`RDKAFKA-SORT-WITHDRAWN`** tracks the two functions it did not free.

### 5.2 The leak pin

`leak` is **272 → 272**, unmoved. Constraint 7's pin needs **no re-pin and no
attribution**: this card adds no leak leaf and withdraws none. That is the
expected shape — ten declarations that release nothing cannot create or
discharge a leak — and it is the second consecutive declarative arbiter to
leave the pin still (W36 also measured 272 → 272).

### 5.3 One prediction premise was wrong, and the ledger was right

`PREDICTION.md` forecasts `double_release` **1 → 1**. That category is **absent
from both stages' histograms**, i.e. its count is **0** and was already 0 in the
BEFORE run that reproduces the baseline byte for byte. The `1` was carried from
the *wave-32* ledger text rather than read from the current baseline; the
**current `AGENTS.md` already says `double_release` 0**, correctly, because
`C-GATEWAY-CONSTANT-DO-WHILE` (wave 35) withdrew the corpus's only
repeated-release row.

So there is **no ledger drift to report** — the error was this card's
prediction, not the ledger. It is recorded here rather than quietly deleted,
because a prediction's errors are part of its evidence, and because it is a
small instance of the failure this project's documentation rules exist to
prevent: a number restated from memory instead of re-read from its source.

---

## 6. Read and DELIBERATELY NOT declared

Each was read as carefully as the ten that landed; declining is a result.

### 6.1 `memcpy` / `memset` — 9 functions of yield, and one half of W36's ground is now discharged

Still the largest single opportunity (`memcpy` sole-blocker of **7**, `memset`
of **2**; 69 and 61 leaves). W36 declined them on **two** grounds and this card
re-read both:

1. **The result mode.** Both return `void *` **aliasing parameter 1**, which v1
   cannot say (`return_owned`, `return_reallocated(p)`, `return_none`).
   `return_none` is available and conservative, but imprecise in a way the
   reading can see. **This ground stands, unchanged.**
2. **"Untested ABI path" — the glibc decorated prototype.** This ground is
   **narrower than W36 recorded.** The decoration class is already exercised in
   production: `libc/memory.qcore` declares `malloc`, whose glibc prototype is
   `extern void *malloc (size_t __size) __THROW __attribute_malloc__
   __attribute_alloc_size__ ((1)) __wur;` (`/usr/include/stdlib.h:676`), and it
   is manifest-selected and not withdrawn. What `memcpy`/`memset` add over
   `malloc`/`free` is specifically **`__restrict`** on the pointer parameters
   (`/usr/include/string.h:46-48`, `:65`), which no corpus declaration has met.

So `LIBC-MEM-RESULT-ALIAS`'s ABI half reduces to one named qualifier rather
than to "decorated prototypes generally". **Is the vocabulary card worth it?**
Named, not built, as the brief directs: **yes, and it is the best-value
vocabulary addition currently visible** — a v1 result mode meaning "aliases
parameter *n* and creates no custody" unlocks 9 sole-blocked functions plus 130
leaves for one addition, and the same mode would serve `memmove`, `strcpy`,
`strcat` and `stpcpy`, which have the identical shape. Its risk is bounded:
being a *result* mode it cannot delete a failure path, and being an alias rather
than an acquisition it opens no obligation. Recorded as
**`LIBC-MEM-RESULT-ALIAS`** (carried, W36's name kept) with the ABI half
narrowed to **`LOADER-ABI-RESTRICT-QUALIFIER`**, whose discharge is one
measurement: declare any `__restrict`-qualified libc callee and see whether the
reader withdraws it.

### 6.2 `snprintf` and `sprintf` — read, faithful in v1, and still declined

Both are **more declarable than `memcpy`**, and the reason is worth recording
because it is the exact inverse of §6.1: their result is `int`, a **scalar**, so
`resource_return_none()` is not a conservative approximation but the *literally
faithful* mode. Their buffer parameter is written through, which
`msgpack_packer_init`'s `pk` already establishes as `borrowed`, and their
variadic tail is `mk_print`'s `resource_variadic_data_tail()`. There is no
vocabulary gap at all.

They were declined on **blast radius and attributability**, not on faithfulness:
`snprintf` alone carries **1,044** leaves against a function-cover yield of
**1** (`rd_kafka_mk_nodename`, itself forecast to exchange — its tail carries a
`const char *` formal, the shape `MK-PRINT-TAIL-ACTUALS` measured refusing at 26
sites). Deciding 1,044 leaves inside an arbiter whose other ten declarations
must each be attributable leaf by leaf would have made this card's movement
hard to read, for one function. `sprintf`'s single target `cJSON_Version`
returns a **`static` local array**, an independent exchange risk.

Named **`LIBC-PRINTF-FAMILY`**: `snprintf`, `sprintf`, `fprintf` (1,138 leaves)
and `vsnprintf` are a coherent **leaf-directed** card with no vocabulary gap and
a ready precedent, and they should be landed together as one, not smuggled into
a cover card. `snprintf` additionally pairs with `sds_cat_safe` to unlock four
ctraces formatters — see §6.3.

### 6.3 The `static` helpers — `sds_cat_safe`, `rd_kafka_topic_partition_destroy0`, `rd_kafka_destroy_app`

All three are `static`/`static inline` **in the very unit that calls them**, with
**no header declaration**, so there is no extern C declaration for constraint
8's ABI gate to agree with and W4 is the correct lever. That is `add`'s ground
minus the collision half, and it now covers the top of the frontier (§2.1).

`rd_kafka_topic_partition_destroy0` carries a **second, independent** ground and
it is the more interesting one: it is a **conditional** release.

```c
static void rd_kafka_topic_partition_destroy0(rd_kafka_topic_partition_t *rktpar,
                                              int do_free) {
        if (rktpar->topic)    rd_free(rktpar->topic);
        if (rktpar->metadata) rd_free(rktpar->metadata);
        if (rktpar->_private) rd_kafka_topic_partition_private_destroy(...);
        if (do_free)          rd_free(rktpar);       /* rdkafka_partition.c:2970-2982 */
}
```

Its two *target* sites (`:3105`, `:3109`) pass `1`/`rd_true`, but five further
sites in the same unit (`:3098`, `:3122`, `:3600`) pass **`0`**. A contract is
per-name, not per-site, so declaring `released` would be **false at those five
sites** and would move in the *admitting* direction — invariant 1's forbidden
direction. The corpus does carry a `resource_literal_argument(PARAMETER,
LITERAL)` clause that could in principle condition a contract on `do_free == 1`;
that is a real avenue and is named **`CONDITIONAL-RELEASE-BY-LITERAL`**, not
taken here because the `static`-in-calling-unit ground is independently
sufficient and W4 is the better lever.

`sds_cat_safe` (`ctr_encode_text.c:22`, one definition in the whole tree) blocks
four ctraces formatters. It is reachable by W4 rather than by declaration once
its own two callees — `strlen` and `cfl_sds_cat_safe`, both external and
header-declared — are decided; with `snprintf` that is the five-function chain
§2.2 describes. Named **`CTRACES-FORMATTER-CHAIN`**.

### 6.4 `add`, and the multi-definition family

`add` is carried forward declined on W36's `ADD-NAME-COLLISION`, re-verified
here: three definitions, three signatures, the cmetrics one `static inline` with
no header. Joining it from this card's own survey, on the same ground: `XXH32`
(3 definitions, and `XXH_PUBLIC_API` expands to `static` under
`XXH_PRIVATE_API`, so the **linkage itself is configuration-dependent** — a
shape one reading cannot settle), `cmt_atomic_initialize` (**5** definitions,
three `inline` without `static`/`extern` against a plain extern prototype, a C99
inline-definition mismatch), `set_result` (**7** definitions, **two**
signatures), `pack_context_metrics` (2, different arity),
`otlp_kvpair_list_destroy` (4) and `destroy_context_list` (2).

Two by-products worth recording. `ADD-NAME-COLLISION` should be read as
**"definitions that disagree"**, not "several definitions" — the four mpack
copies are the counter-example that fixes the rule (§3.1), and without that
refinement this card would have declined its own largest module. And
`lib/cprofiles/` carries a **stray duplicate tree**: `lib/cprofiles/*.c`
duplicates `lib/cprofiles/src/*.c` and the two are **not identical**
(`cprof_encode_opentelemetry.c` is 92,051 vs 82,980 bytes). Nothing in this card
depends on it and no claim is made about which is built; recorded as
**`CPROFILES-STRAY-DUPLICATE-TREE`** because a future name-keyed declaration in
that library would meet it.

---

## 7. What was touched

* **New** (6): `contract/qcore/mpack/writer.qcore`,
  `contract/qcore/cmetrics/metric_values.qcore`,
  `contract/qcore/cmetrics/mpack_utils.qcore`,
  `contract/qcore/cfl/time.qcore`,
  `contract/qcore/librdkafka/lz4_stream.qcore`,
  `contract/qcore/librdkafka/topic_partition.qcore`.
* `contract/cycle19-resource-contracts.json` — six manifest rows, 27 → 33.
* **Constraint 5 pins, updated deliberately and said so:**
  * `compiler/tests/qcore_contract_corpus.rs` — corpus 82 → **88**, and ten
    `EXPECTED_CONTRACTS` rows with their clause counts (2, 3, 3, 4, 4, 4, 5, 5,
    5, 5). The table is **ordered by module path** and the test compares it
    against the corpus in that order; the first placement was chosen for
    narrative adjacency, the test caught it, and the rows were moved rather
    than the test loosened.
  * `compiler/tests/contract_loader.rs` — `source_assumptions().len()` 27 → 33
    (twice), `modules.len()` 82 → 88,
    `(selected, v1_declarations, rejected_declarations)` **(27, 49, 145) →
    (33, 59, 145)** (rejected unchanged: every added module is entirely v1), and
    all ten declarations added to the explicit old-table parity list — **all ten
    inside the all-borrowed loop and none below it**, the exact inverse of W36.
  * `crates/qcore-c-gateway/tests/prod_contracts.rs`,
    `crates/qcore-c-gateway/tests/neighbours.rs` — manifest count 27 → 33.
* `docs/reviews/r0_3/acceptance_matrix.json` — this card's residuals
  **registered** (§9), which W36 did not do and which cost that close a
  registration pass.
* `docs/development/increments/wave-37.md` — one paragraph prepended.
* `AGENTS.md` — headline numbers.
* This report plus `PREDICTION.md`, `artifacts/`, `logs/`, `scripts/`.

**No crate source file changed.** No gateway checker, loader, W4 or backend
code was touched, which is what makes every movement in §5 attributable to the
ten premises alone.

---

## 8. Gate

Disk was checked first: **168 GB free**, above the 60 GB floor, so the full gate
ran. Every exit code was captured to a file, never read through a pipe. The gate
was run **three times** and all three are published, because the first found two
things and the third is the one that counts — the rebased tree.

```
run 1, base 477b733d   (logs/gate-exits-run1.txt)  test=101  fmt=1  clippy=0  probe=0  followup=0  lean=0  pytests=0
run 2, base 477b733d   (logs/gate-exits-run2.txt)  test=0    fmt=0  clippy=0  probe=0  followup=0  lean=0  pytests=0
run 3, base 797661f3   (logs/gate-exits.txt)       test=0    fmt=0  clippy=0  probe=0  followup=0  lean=0  pytests=0
```

**Run 3 is the gate of record: green in every step, first try, on the rebased
tree.** Measured at it:

* **4,628 workspace tests passed, 0 failed, 8 ignored**, over 232 test-result
  summaries (`cargo test --workspace --no-fail-fast`). The **+17** over run 2's
  4,611 is the two wave-37 code arbiters' own tests, which run 2 did not have.
* `cargo fmt --all -- --check` clean; `cargo clippy --all-targets -- -D warnings`
  clean.
* Both reevaluation examples **exit 0**, reported as the brief requires whether
  or not that is the convenient answer.
* **137 Lean audits** passed, `AuditAll` axiom closure clean — the figure the
  integrator predicted, the two new audits having landed above this card.
* **293 Python tests** pass (`python3 -m unittest discover -s scripts/tests`),
  which includes `CommittedMatrixIsSelfConsistent` — the row that catches a
  hand-set `named_followups` count, and the reason §10 item 4 regenerates
  through the generator rather than editing the number.
* Constraint 16's rows, stated because that constraint requires a gate report to
  **say which it saw**: every one of them was **green** in run 3, including
  `a_declared_process_limit_stops_the_fixture_from_forking`, which had failed in
  run 1 (§8.2). This is a full-suite run on a machine carrying its own build at
  a falling load, **not** the quiet-host isolated number
  `HOST-ROW-STARVATION-DESCRIPTION` asks for, and it is not offered as one.

### 8.1 `fmt=1` in run 1 was mine, and it was a real finding

`cargo fmt` wanted the multi-line tuple form for the three
`EXPECTED_CONTRACTS` rows whose module path plus symbol name exceed the line
width (`cmt_metric_get_timestamp`, `cmt_mpack_consume_uint_tag`,
`rd_kafka_topic_partition_list_sort`). Fixed at `cf5c41d9`; the pinned **values**
are unchanged and both pin tests stay green. Recorded rather than quietly
amended, because it is the second time this card's pinned-table edits were
caught by a check rather than by review — the first being the table's
module-path ordering (§7).

### 8.2 `test=101` in run 1 was constraint 16's FIRST named row, in its exact documented shape

One test failed in run 1, and only one:

```
---- a_declared_process_limit_stops_the_fixture_from_forking stdout ----
thread '…' panicked at crates/qcore-verifier-host/tests/enforced_process_limits.rs:389:5:
assertion `left == right` failed: Broken pipe (os error 32)
  left: Io
 right: Terminated
```

Constraint 16 describes this row as installing `RLIMIT_NPROC=1`, "which counts
every process of the real UID, so under load the host observes `Io(Broken pipe)`
instead of `Terminated`". The observed failure is that sentence verbatim — same
row, same mechanism, same two values. It is **P02-FLAKE's first listed row**, not
a new one, so no amendment to constraint 16 is proposed.

**The repetitions.** Re-run **alone**, 12 times (`scripts/reverify.sh`,
`logs/reverify.txt`), recording the load average before each repetition:

```
ISOLATED RESULT: 12 passed, 0 failed, of 12
one-minute loads at the repetitions: 12.99 12.99 12.67 12.67 12.67 13.50
                                     13.50 13.50 13.50 13.06 13.06 13.06
fifteen-minute load across the window: 21.2 - 21.4   (16 cores)
```

**12 of 12 pass alone**, and the row then passed **again inside the full run-2
gate** and **again inside the full run-3 gate on the rebased tree**. Both facts
are recorded, which is what constraint 16 asks of a load-sensitive row: it
failed once at the busy tail of a 16-core machine whose fifteen-minute load was
above 21, and passed **14 subsequent times**, twice of them inside a full
suite.

**It cannot be this card.** This card changes **no crate source at all** — six
`.qcore` files, one manifest, four test-pin files and documentation.
`qcore-verifier-host` depends only on `qcore-frontend` and `qcore-schemas`,
never on the contract crate, and does not read
`contract/cycle19-resource-contracts.json`. No claim is made here that the row
is fixed or unfixable; the measurement says it passed 14 of 15 and failed once
under load, and nothing more.

---

## 9. Named follow-ons — **registered in the matrix**, not only named here

W36 named nine residuals in its report alone and none reached
`named_followups`, which cost the wave-36 close a registration pass. This card
**registers every one of the following in
`docs/reviews/r0_3/acceptance_matrix.json`**, taking `named_followups`
**171 → 181**.

| Name | What it records | Discharge condition |
|---|---|---|
| `LOADER-ABI-INLINE-FNPTR-DECLARATOR` | The ABI reader resolves a function-pointer **typedef** but not an **inline** function-pointer parameter declarator, and withdraws the declaration at every call site. First measured from production text here; cost this card 2 of its 11. | Extend the parameter declarator reader to the inline form, on `CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF`'s precedent; `FUNCTION_POINTER_ABI_TYPE` already exists. |
| `RDKAFKA-SORT-WITHDRAWN` | `rd_kafka_topic_partition_list_sort`'s declaration is withdrawn in all four calling units; its two target functions stay `Unknown` at zero leaf cost (11 same-position substitutions). | The above; then both functions should reach `Safe` with no other change. |
| `W37-COVER-FRONTIER-IS-STATIC` | After W36's harvest the sole-blocker cover's top is dominated by `static` helpers defined in their own calling units, where declaration is structurally the wrong lever. | A W4-directed card, or acceptance as a measured boundary on the declaration lever's reach. |
| `NESTED-CALL-IN-ARGUMENT-POSITION` | A call nested in a modeled call's argument list is refused; now seen in a third library (`cfl_sds_len`, `cmt_metric_hist_get_sum_value`, `cmt_metric_get_int64_value`), and the commonest way a decided leaf is exchanged. Generalises `CJSON-DELETE-NESTED-DETACH`. | Admit the nested-call-in-argument-position shape, or declare/derive the inner callees. |
| `CTRACES-FORMATTER-CHAIN` | `sds_cat_safe` blocks four ctraces formatters and is `static` in its own unit; it becomes W4-derivable once `strlen` and `cfl_sds_cat_safe` are decided, and with `snprintf` that is a five-function chain. | Declare `strlen` + `cfl_sds_cat_safe` (+ `snprintf`) and re-measure. |
| `LIBC-PRINTF-FAMILY` | `snprintf`/`sprintf`/`fprintf`/`vsnprintf` are faithful in v1 today (scalar result, borrowed buffer, variadic tail) and carry ~3,200 leaves at near-zero function-cover yield — a **leaf**-directed card, not a cover card. | A dedicated leaf card landing them together with its own attribution. |
| `LOADER-ABI-RESTRICT-QUALIFIER` | Narrows `LIBC-MEM-RESULT-ALIAS`'s ABI half: glibc *decoration* is already exercised by `malloc`; what is untested is specifically `__restrict` on a pointer parameter. | Declare any `__restrict`-qualified libc callee and observe whether the reader withdraws it. |
| `CONDITIONAL-RELEASE-BY-LITERAL` | `rd_kafka_topic_partition_destroy0` releases only when `do_free != 0`; five of its seven sites pass 0. v1 has `resource_literal_argument`, which could condition a contract on a literal argument. | A literal-conditioned release contract, or acceptance that W4 is the lever for unit-internal statics. |
| `CPROFILES-STRAY-DUPLICATE-TREE` | `lib/cprofiles/*.c` duplicates `lib/cprofiles/src/*.c` and the copies are **not** identical. Nothing here depends on it; a future name-keyed declaration in that library would. | Determine which tree the pinned build compiles, and record it. |
| `MPACK-WRITER-INTERIOR-REALLOC` | A growable mpack writer's flush callback reallocs `writer->buffer`, so interior addresses are unstable across the declared calls. The parameter's custody and the caller's destroy obligation are unchanged, which is all the declaration states. | Acceptance as a stated boundary, or an interior-address model (none exists). |

**Carried, not retired:** `LIBC-MEM-RESULT-ALIAS` and `ADD-NAME-COLLISION` are
W36's and both **stand**, re-verified here with their grounds sharpened (§6.1,
§6.4) rather than discharged.

**RETIREMENTS: none.** This card retires no residual and removes no
`named_followups` entry — stated explicitly because the matrix's union merge
loses unnamed removals, and the brief asks for retirements to be named and each
verified as a registered entry. There are none to verify.

---

## 10. For the integrator

Please work these as **items**, not as headline numbers — the `W32-RECONCILE-SWEEP`
finding is that the numbers get applied and the prose does not.

1. **Ledger headline numbers** (`AGENTS.md`), applied in this card's own edit —
   please verify: projected functions **205 / 84 / 4,545 → 213 / 84 / 4,537**;
   unit statuses **unchanged** at 2/42/137/4; `leak` **272, unmoved** (no
   re-pin); `unsupported` **49,922 → 49,753**; `unknown_alias` **4,308 →
   4,309**; `summary_conflict` **1, unmoved**; contract corpus **82 → 88**;
   resource-v1 declarations **49 → 59**; manifest-selected **27 → 33**.
2. **The baseline moves to `docs/reports/w37-corpus-contracts-4/artifacts/`.**
3. **This item is DISCHARGED, not outstanding.** `SCALAR-ROUTE-EXIT-EDGE` and
   `JOINED-PROVENANCE-ROUND-BOUND` did merge ahead of this card, and on the
   integrator's instruction the card was rebased onto `797661f3` and **both
   stages re-run there against a rebuilt audit binary**. BEFORE still
   reproduces the baseline byte for byte and AFTER is byte-identical to the
   archived AFTER in all five artifacts, so the attribution stands unchanged
   and no count moved (§1.0). The re-run also **independently confirms both
   code cards' zero-movement claims** from outside those cards.
4. **Residuals ARE registered** (§9) — the change from W36's practice the brief
   asked for — and the registration was **applied through the generator, not by
   hand**. `named_followups` is **197 → 207**: main reached 197 across the
   wave-36 close and wave 37's two code arbiters, and this card's ten are
   appended, with **zero id collisions** verified against all 197. The matrix was
   then regenerated with
   `build_matrix.py . --gate docs/reports/wave36-close/gate.json`, which is what
   keeps the derived `dimensions.named_followups` in step — the wave-36 close
   recorded that hand-setting it leaves the count stale and trips
   `CommittedMatrixIsSelfConsistent` (`D02-DIMENSIONS-FOLLOWUP-COUNT`,
   constraint 25), and this card followed that record rather than repeating the
   mistake. `measurement_provenance` is therefore
   `integrator_supplied (docs/reports/wave36-close/gate.json)`, not the
   generator's carried-and-unmeasured warning, and `contract_modules` derives to
   **88**.
5. **No action — recorded so it is not re-investigated.** This card's
   `PREDICTION.md` forecast `double_release` `1 → 1`; the true value is **0** in
   both stages. The **ledger is already correct** at 0 (wave 35 withdrew the
   only row); the error was the prediction's, carried from stale ledger text
   rather than read from the baseline (§5.3). Nothing to change.
6. **Proposed amendment text for standing constraint 7.** After the existing
   text, please add, in these words:

   > Card `W37-CORPUS-CONTRACTS-4` (wave 37) left the number unmoved a second
   > consecutive time: 272 → 272 across a re-pin adding ten declared contracts
   > and moving eight function statuses. Two declarative arbiters in a row have
   > now decided several hundred uncontracted-call leaves without touching the
   > leak pin, which is the expected shape when the declarations release
   > nothing; the pin was re-verified rather than re-derived.

7. **Proposed amendment text for standing constraint 8.** After the existing
   text, please add, in these words:

   > The ABI reader's **withdrawal** arm was exercised for the first time by a
   > *production declaration* in card `W37-CORPUS-CONTRACTS-4` (wave 37):
   > `rd_kafka_topic_partition_list_sort`'s C declaration spells its comparator
   > as an **inline function-pointer parameter declarator**, which the reader
   > does not parse — it resolves the function-pointer **typedef** form that
   > card `CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF` taught it, and nothing else — so
   > the declaration is withdrawn at every call site with
   > `abi-unreadable-declaration`. The withdrawal is **leaf-neutral**: all
   > eleven rows are same-position substitutions and no function's leaf count
   > changed. This is the gate failing closed on a *reader* boundary rather than
   > on a disagreement, which is `LOADER-ABI-UNDECLARED`'s neighbour and is
   > named `LOADER-ABI-INLINE-FNPTR-DECLARATOR`.

8. **Constraint 14 (keyword census): this card does not engage it.** No word is
   reserved and the lexer is untouched; a `.qcore` contract module adds no
   source-language surface. Please list it among the wave's non-engaging cards.
9. **`contract/qcore/mpack/` is a new family**, the corpus's second after W36's
   `ctraces/`. Its manifest position (after `msgpack/packer.qcore`) and the
   module name `writer.qcore` are my choices; overrule either if the corpus has
   a convention I missed. **Please keep it separate from `msgpack/`** — they are
   different libraries, and the pinned table's comment says so at the point of
   confusion (`W37-MPACK-FAMILY-FIRST`, registered).
10. **`ADD-NAME-COLLISION` should be restated as "definitions that disagree"**,
    not "several definitions" (§3.1, §6.4). Four identical vendored copies are
    safe to declare; three copies with three signatures are not. Without that
    refinement the rule would have declined this card's largest module. I have
    **not** edited W36's residual text; this is a proposed restatement for the
    integrator to apply where that residual is recorded.
11. **The acceptance matrix was regenerated in place, with the close's gate.**
    `build_matrix.py . --gate docs/reports/wave36-close/gate.json` was run on the
    rebased tree, so the derived sections follow the corpus (`contract_modules`
    **88**, `named_followups` **207**, `lean audits` **137**) and
    `measurement_provenance` reads
    `integrator_supplied (docs/reports/wave36-close/gate.json)` rather than the
    generator's carried-and-unmeasured warning. This supersedes what an earlier
    draft of this item said — before the rebase the card had *declined* to
    regenerate, to avoid downgrading that provenance label; passing `--gate`,
    as the integrator directed, resolves that properly instead of working around
    it. `named_followups` is `hand_maintained`, so the ten new entries are
    carried by the generator rather than authored by it. Two derived fields remain worth a glance: `corpus.contract_modules_per_directory` and
    `corpus.contract_clause_pin` come from card B00's **frozen** archive and
    describe a 59-module checkpoint beside a corpus this card takes to **88**.
    That is the already-recorded `D02-B00-LEDGER-SNAPSHOT` (constraint 25), it
    widens here, and it must be repaired by publishing provenance beside them
    and **never** by editing the archive (constraint 21).
12. **The "Merged gate" ledger row is nearly current; one number moved.**
    `AGENTS.md` now reads **4,611 passed / 0 / 8**, **293 Python** and **137
    Lean audits**, measured by the wave-36 close. This card measured **293
    Python** and **137 Lean audits** — exact agreement — and **4,628** workspace
    tests, the **+17** being the two wave-37 code arbiters' own tests, which
    merged after that close. Deliberately **not** edited here: the row is the
    close's to move, and the next close will re-measure it anyway (§8).
13. **Nothing here is a claim about upstream.** Every `Safe` is `Safe` relative
    to ten declared premises; the substituted leaves are `Unknown`-class
    evidence and not defect claims (invariant 2).

---

## 11. Reproduction

```sh
git submodule update --init --recursive
cargo build --release -p qcore-compiler --example cycle19_fluent_bit_libraries_audit
# BEFORE: with the manifest at its 27-module form (the six .qcore files absent)
sh measure/w37/stage.sh before
python3 -B measure/w37/byte_identity.py measure/w37/runs/before \
    docs/reports/joined-provenance-call-frames/artifacts
python3 -B measure/w37/cover.py measure/w37/runs/before
python3 -B measure/w37/greedy.py measure/w37/runs/before --depth 25
# ... land the six modules and the six manifest rows ...
sh measure/w37/stage.sh after
python3 -B measure/w37/census.py --before measure/w37/runs/before \
    --after measure/w37/runs/after
python3 -B measure/w37/attribution.py measure/w37/runs/before measure/w37/runs/after
python3 -B measure/w37/evidence.py
sh measure/w37/gate.sh
```

The pin is the shared long-lived
`/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin`, with
`QCORE_CLANG=/usr/bin/clang` and the wave-20 ChunkIO package;
`QCORE_SOURCE_CONTRACTS` is deliberately unset (the two selectors are ambiguous
together). Scripts are archived under `scripts/`, logs under `logs/`, and both
stages' five artifacts under `artifacts/`. A run takes four to eight minutes.
