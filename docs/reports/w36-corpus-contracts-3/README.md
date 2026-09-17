# W36-CORPUS-CONTRACTS-3 — six declarations, chosen by function-cover yield

Wave 36's **second arbiter**, chained behind `JOINED-PROVENANCE-CALL-FRAMES`.
**Declarative**: six `.qcore` contract modules and six manifest rows. No
gateway checker, loader, backend or any other crate source changed, so **one
audit binary served both stages** and the only difference between BEFORE and
AFTER is which modules the manifest selects.

Base `60958bf0`. Baseline `docs/reports/joined-provenance-cycles/artifacts/`.

---

## Headline

| Measure | BEFORE | AFTER | Note |
|---|---|---|---|
| Units | 2 Safe / 42 Unsafe / 137 Unknown / 4 data-only | **identical** | **zero unit moves** |
| Projected functions (4,834) | 186 Safe / 84 Unsafe / 4,564 Unknown | **205 Safe / 84 Unsafe / 4,545 Unknown** | +20 Safe, −1 Safe, net **+19** |
| `leak` | 272 | **272** | **pin holds** (constraint 7), no re-pin needed |
| `unsupported` | 50,106 | 49,922 | **−184** |
| `unknown_alias` | 4,214 | 4,308 | **+94**, all accusing |
| `summary_conflict` | 0 | **1** | new in this corpus; §5.2 |
| Contract corpus | 76 modules | **82** | constraint 5 pins updated deliberately |
| Manifest-selected | 21 | **27** | |
| resource-v1 declarations | 43 | **49** | |

Leaves: **217 removed, 128 added, net −89.** `flb_input_chunk_create` remains
function-level **Safe**. No unit was promoted, and **no `Unknown` became `Safe`
by a refactor** — every status move is a consequence of a declared premise,
which is what a declaration is for (invariant 2: these verdicts are `Safe`
RELATIVE TO the six declarations, never a proof of the C).

---

## 1. The chained re-pin, and which baseline

The card brief named `docs/reports/c-gateway-constant-do-while/artifacts/` as
the baseline. That was **stale by one**: `AGENTS.md` names
`docs/reports/joined-provenance-cycles/artifacts/` (wave 35's second arbiter).
This was checked rather than assumed, and the check is worth recording — the two
archives' `diagnostics.json` and `chunk-contracts.json` **decompress
byte-identically** (sha256 `9af9ee2e4e0837b6…` and `c4f438e4ed3dab3c…`
respectively), i.e. `JOINED-PROVENANCE-CYCLES` moved neither verdict-bearing
artifact. Only `contracts` and `source-effects` differ between them, which is
the checkout-root class. So the cover derived from either is the same cover;
the BEFORE gate below nonetheless targets the **ledger-named** baseline.

`origin/main` was re-fetched before the final runs. At that point
`JOINED-PROVENANCE-CALL-FRAMES` had **not** merged (`origin/main` was still at
`60958bf0`), so no rebase was required and the baseline did not move under this
card. **If it merges before this card does, the integrator must have the BEFORE
re-run against the new baseline** — see §10.

### 1.1 BEFORE reproduces the baseline byte for byte

```
*diagnostics      raw IDENTICAL  canonical IDENTICAL   9af9ee2e4e0837b6 / 9af9ee2e4e0837b6
 source           raw IDENTICAL  canonical IDENTICAL   ae38f6c93dfbe341 / ae38f6c93dfbe341
*chunk-contracts  raw IDENTICAL  canonical IDENTICAL   c4f438e4ed3dab3c / c4f438e4ed3dab3c
 source-effects   raw differs    canonical differs
 contracts        raw differs    canonical differs
RESULT: the verdict-bearing artifacts reproduce the baseline BYTE FOR BYTE.
```

(`logs/byte-identity-before.txt`.) The `source-effects`/`contracts` residue is
the checkout-root and identity-digest class every prior re-pin has recorded.
**This is the load-bearing fact of the whole measurement**: because BEFORE
reproduces the baseline exactly, everything in §4 and §5 is these six
declarations and nothing else.

`chunk-contracts.json` is furthermore **byte-identical between BEFORE and
AFTER** (both 281,879 bytes archived, same content) — the declarations move no
chunk contract at all.

### 1.2 One binary, both stages

Because the card is declarative, the BEFORE and AFTER binaries are **the same
file**, sha256
`3ea853168788d2f8392fc19e368667ad84faf92e896bcd6740d0f00fe51d2178`. There is
therefore no "was it the same build?" question to answer: the manifest was at
its 21-module form for BEFORE (the six `.qcore` files did not yet exist on
disk), and at 27 for AFTER. BEFORE 254.3s, AFTER 285.8s, both exit 0.

---

## 2. Why contracts, re-derived

The brief's measurement was re-derived from the baseline rather than carried.
Re-derived numbers (`logs/prediction.txt`, `predict.py`):

* **193 projected functions are blocked *only* by `call to uncontracted
  function` leaves.** All 193 are `Unknown`.
* Their distinct-callee histogram is `{1: 122, 2: 35, 3: 22, 4: 8, 5: 1, 6: 5}`
  — **122 of the 193 have exactly one blocker.** That is the real opportunity
  space, and it is what makes a single declaration able to move a function at
  all.
* The brief's naming counts reproduce exactly: `cfl_sds_destroy` 13, `memcpy`
  11, `__builtin_va_start`/`__builtin_va_end` 11 each, `snprintf` 8,
  `cfl_sds_cat_safe` 7, `fprintf` 5, `rd_kafka_mem_free` 5.
* The brief's greedy cover ("20 declarations free 46") re-derives at **49**
  with a sole-blocker-first greedy. The difference is tie-breaking in the
  heuristic, not a disagreement about the corpus; both are recorded.

**Selection was by sole-blocker count, not leaf count**, exactly as the brief
directs. The leaf mass is elsewhere — `fprintf` 1,138 leaves and `snprintf`
1,044 — but those decide almost no *function*, because the functions holding
them hold other blockers too.

---

## 3. The six declarations, each read before it was declared

Every body below was read from the pinned checkout
`build/cycle19/fluent-bit-clang-source` at Fluent Bit
`66910c10a4d7eafa810b84229cf2dcf7b0f26f97` **on this card's own run**, not
carried from any predecessor. Each module's own comment block carries the full
line-cited reading, the ABI paragraph, and its own "NOT CLAIMED" list; the
substance is below.

| Module (new) | Callee | Mode | Sole blocker of |
|---|---|---|---|
| `contract/qcore/cfl/sds_lifetime.qcore` | `cfl_sds_destroy` | `released(nullable)` | 8 |
| `contract/qcore/librdkafka/kafka_mem.qcore` | `rd_kafka_mem_free` | `borrowed` + `released(nullable)` | 4 |
| `contract/qcore/cmetrics/context_lifetime.qcore` | `cmt_destroy` | `released(**nonnull**)` | 3 |
| `contract/qcore/librdkafka/cjson.qcore` | `cJSON_Delete` | `released(nullable)` | 3 |
| `contract/qcore/monkey/logging.qcore` | `mk_print` | borrowed ×2 + **variadic tail** | 3 |
| `contract/qcore/ctraces/context_lifetime.qcore` | `ctr_destroy` | `released(**nonnull**)` | 2 |

`ctraces/` is a **new contract family** — the corpus's first.

### 3.1 The nullability markers are not uniform, and that is the point

Each was read off its own body, and the corpus now carries both answers for the
same shape:

* **`cfl_sds_destroy`** (`lib/cfl/src/cfl_sds.c:249-275`) — `nullable`, because
  the body's **first** statement is `if (!s) { return; }`. This is
  `flb_input_chunk_destroy_direct_routes`' position, for its reason.
* **`cJSON_Delete`** (`lib/librdkafka-2.15.0/src/cJSON.c:253-276`) —
  `nullable`, on the loop condition rather than a guard: `while (item != NULL)`
  is evaluated before any dereference. This matters concretely: all three call
  sites pass a `cJSON_DetachItem…` result, which is NULL when the item is not
  found, so **the null path is reached** and `nonnull` would have been wrong.
* **`cmt_destroy`** (`lib/cmetrics/src/cmetrics.c:92-146`) and **`ctr_destroy`**
  (`lib/ctraces/src/ctraces.c:58-71`) — **`nonnull`**, the opposite reading on
  opposite evidence: neither has a leading guard, and each dereferences its
  parameter in its first executable statement (`cfl_list_foreach_safe` expands
  to a read of `cmt->counters` / `ctx->resource_spans`). `cmt_destroy(NULL)` is
  upstream UB, not a tolerated no-op. This is `flb_input_chunk_destroy`'s
  position, for its reason.
* **`rd_kafka_mem_free`** (`…/src/rdkafka.c:7131-7133`) — `rk` is read
  **nowhere** in the two-line body, so it is `Borrowed` and is deliberately
  **not** asserted non-null (LZ4's `FREEMEM` passes a null handle); `ptr` is
  forwarded to `rd_free`, whose own declaration already carries `nullable`, so
  the marker is **inherited from the delegate** rather than guessed. §5.2 is
  what that inheritance cost, and why it was still right.

### 3.2 Two ABI facts worth recording (constraint 8)

* **`rd_kafka_mem_free` is declared twice in C, differently.** The public
  header (`…/src/rdkafka.h:3643-3644`) says `rd_kafka_t *rk`; the vendored
  `lz4.c` carries its own local extern at `:227` saying `struct rdkafka_s *rk`.
  Neither difference reaches the v1 projection, which is stated in **pointer
  levels** and not struct tags — both are one object-pointer level,
  `ptr<mut, u8>` — so the declaration agrees with the C of *both* declaring
  units. A reader who checked only the public header would not have seen the
  second spelling.
* **`mk_print` carries a trailing attribute.** Its header
  (`…/mk_core/mk_utils.h:49`) is
  `void mk_print(int type, const char *format, ...) PRINTF_WARNINGS(2,3);`,
  where `PRINTF_WARNINGS` expands to
  `__attribute__ ((format (printf, a, b)))`. It changes no type, arity or
  calling convention. The **declared** arity is two; the `...` is carried by
  `resource_variadic_data_tail()` and not by a third parameter, and LOADER-ABI's
  arity gate is what keeps that honest — a tail-free contract for a variadic C
  declaration would be withdrawn. The parameter C spells `type` is spelled
  `level` here because `type` is a reserved word, which is the same rename
  `fluent_bit/logging.qcore` already makes for `flb_log_print`.

### 3.3 Lean: none added, with the argument

No Lean module is added. Invariant 6 binds **semantic changes** — definitions
and theorems must match a changed semantic surface. This card changes no
semantic surface: it adds premises to a corpus the loader already consumes,
through a vocabulary (`resource_contract_v1`,
`resource_variadic_data_tail`) whose meaning, loader and checker are all
unedited. A **grown premise set is not a semantic change**. This is exactly
`W31-CORPUS-CONTRACTS-2`'s recorded position and is taken on the same ground,
stated rather than assumed.

---

## 4. Predicted vs measured

The prediction was computed **from BEFORE only** and committed (`a3d734a3`)
**before the AFTER run finished**, so this is a real comparison and not a
retrofit. `predict.py` and `logs/prediction.txt` are archived.

| | Predicted | Measured |
|---|---|---|
| Leaves named by the six, corpus-wide | **204** | **204** — exact |
| Functions losing their last blocker | **23** | **20** |
| Function status moves | (not predicted) | **21** (20 gained, 1 lost) |

### 4.1 The three-way split the brief asks for

Over the 23 predicted functions:

* **LEAF-FREE — 20.** Every leaf gone, every one `Unknown → Safe`:
  8 via `cfl_sds_destroy` (the cmetrics/ctraces encoder destructors),
  4 via `rd_kafka_mem_free` (`LZ4_freeStream`, `LZ4_freeStreamDecode`,
  `LZ4_freeHC`, `LZ4_freeStreamHC`),
  3 via `cmt_destroy`, 3 via `mk_print`, 2 via `ctr_destroy`.
* **EXCHANGED for another refusal — 3.** All three `cJSON_Delete` functions.
  §5.1.
* **UNMOVED (the leaf is still there) — 0.** No declaration failed to decide
  its own leaf; in particular **none of the six was withdrawn** by constraint
  8's ABI or duty gate.

### 4.2 Where the prediction was wrong, and why

The 204-leaf prediction was **exact**. The 23-function prediction overshot by 3,
and the reason is a **methodological limit worth naming, not a mistake in the
readings**: the cover is computed from the leaves a function *visibly* holds,
and a blocker that only becomes reachable once the first is removed is
invisible to it. The three cJSON functions each held exactly one leaf; behind
it stood a second refusal that the projection could not reach while the outer
call was uncontracted. Residual **`W36-COVER-HIDDEN-BLOCKER`**: a sole-blocker
cover is an **upper bound** on function moves, never a forecast. Discharge
condition: a cover that models the projection's own reachability, not just its
emitted leaves.

---

## 5. Every count that moved, attributed

`logs/census.txt` prints every moved leaf individually at the **internal
preprocessed coordinate** that is its identity, with the presented `origin`
beside it (constraint 12a). `logs/attribution.txt` groups them. Nothing is
summarised away.

**Removed: 217** (all `unsupported`). 204 name one of the six declared callees
— the prediction, exactly. The other 13 are **cascade**: 5 naming
`mk_config_error` and 1 naming `mk_rconf_warning` (both became `Safe`, so W4
now derives summaries for them and their own callers' leaves are decided), and
7 unrelated `unsupported` leaves in `ctr_id.c`, `mk_server.c`, `mk_socket.c`
and `mk_user.c` that the changed abstract states no longer produce.

**Added: 128** — 94 `unknown_alias`, 33 `unsupported`, 1 `summary_conflict`.
Every one is in the **accusing** direction. The 94 `unknown_alias` leaves (40
distinct identities, concentrated in `cmt_decode_statsd.c` 16,
`librdkafka/cJSON.c` 20, `cmt_encode_splunk_hec.c` 10,
`rdkafka_sasl_oauthbearer_oidc.c` 12) are the direct consequence of modelling a
release: once `cfl_sds_destroy(x)` really releases `x`, a later mention of `x`
or of something derived from it is an unresolved identity rather than an
unexamined one. **This is the mechanism working**, and it is why `unknown_alias`
rising while `unsupported` falls is the expected shape of a declarative card,
not a regression.

### 5.1 `CJSON-DELETE-NESTED-DETACH` — the three exchanged functions

The declaration **did** decide its leaf: 40 `cJSON_Delete` leaves were removed
corpus-wide (23 in `cJSON.c`, 15 in `rdkafka_sasl_oauthbearer_oidc.c`, 2 in
`rdhttp.c`). But all three target functions are of the shape

```c
CJSON_PUBLIC(void) cJSON_DeleteItemFromArray(cJSON *array, int which)
{
    cJSON_Delete(cJSON_DetachItemFromArray(array, which));
}
```

and each now refuses on a **new** ground:

```
librdkafka/cJSON.c :: cJSON_DeleteItemFromArray  L3059 origin=cJSON.c:2291
  [unsupported] nested call `cJSON_DetachItemFromArray` inside a modeled
                call is outside the resource projection
```

Before the declaration the outer call was uncontracted and the whole statement
refused there; now the outer call is modeled and the projection reaches the
**inner** call, which it does not model in argument position. The three
functions stay `Unknown`. This is fail-closed behaviour and the correct
outcome — but it means these three are **not** discharged, and the residual
names them. Discharge condition: the nested-call-in-argument-position
admission, or declarations for the three `cJSON_DetachItemFrom*` callees (all
three are defined in this same unit, so W4 is the more natural lever).

### 5.2 `KAFKA-MEM-FREE-SUMMARY-CONFLICT` — the one lost `Safe`

`librdkafka/rdkafka.c :: rd_kafka_mem_free` moved **`Safe → Unknown`**, the
card's only regression, carrying the corpus's first `summary_conflict`:

```
inferred custody summary  ResourceFunctionContract { parameters: [Borrowed, Released { nullable: false }], result: None }
contradicts the declared  ResourceFunctionContract { parameters: [Borrowed, Released { nullable: true  }], result: None };
the declared contract governs every call site
```

W4 derived `nonnull` from the body; the declaration says `nullable`. **The
declaration is the faithful one and stands.** `rd_free(NULL)` reaches libc
`free(NULL)`, which is defined and is a no-op, and `rd_free`'s own
already-declared contract carries `nullable` for exactly that reason.
Declaring `nonnull` to silence the conflict would **assert a precondition the
body does not have** — precisely what constraint 4 forbids ("do not seed a
formal parameter with entry custody the body did not demand … that asserts
non-nullness and deletes failure paths"). LZ4's own call sites confirm the null
path is contemplated (`lz4.c:1536` `if (!LZ4_stream) return 0;  /* support free
on NULL */`).

So this is constraint 8's machinery working exactly as specified: a
disagreement is **reported as a `SummaryConflict`, never silently resolved**,
the declared contract governs every call site, and the defining unit's own
function is marked. The direction is conservative — a safety claim withdrawn,
not manufactured — which invariant 1 permits; the forbidden direction is
`Unknown → Safe` by refactor, and nothing here does that. Net across the
declaration: **+4 functions, −1**. Residual discharge condition: a W4
derivation that reads a delegate's declared nullability instead of defaulting a
released formal to `nonnull`.

### 5.3 `MK-PRINT-TAIL-ACTUALS` — 26 tail arguments still refused

`mk_print` removed **87** leaves and added **26** of

```
arguments beyond the declared parameters of `mk_print` are outside the
resource projection
```

This is the sentence `monkey/logging.qcore` predicts in its own comment:
`resource_variadic_data_tail()` **admits nothing on its own** — the gateway
independently requires, per call site, that each tail actual really is data,
and refuses one that may carry a tracked resource identity exactly as it would
without the declaration. The three target functions' tails were admitted; 26
other sites' were not. Measured confirmation of the clause's stated semantics,
and a named place for a later card to look.

### 5.4 The leak pin

`leak` is **272 → 272**, unmoved. Constraint 7's pin therefore needs **no
re-pin and no attribution**, and this card adds no leak leaf and withdraws
none. `double_release` remains 0 in both stages (it does not appear in either
histogram).

---

## 6. Read and DELIBERATELY NOT declared

Each was read as carefully as the six that landed; declining is a result, not
an omission.

### 6.1 `add` — 3 functions of yield, declined on the P07 ground

`add` is the sole blocker of `cmt_metric_add`, `cmt_metric_inc` and
`cmt_metric_sub` (all `cmetrics/cmt_metric.c`). A whole-tree search found
**three distinct definitions with three different signatures**:

| File | Line | Signature | Linkage |
|---|---|---|---|
| `lib/cmetrics/src/cmt_metric.c` | 46 | `static inline void add(struct cmt_metric *, uint64_t, double)` | internal |
| `lib/cfl/tests/list.c` | 45 | `static void add()` | internal, test-only |
| `lib/wasm-micro-runtime-WAMR-2.4.1/…/native-symbol/func.c` | 12 | `int add(int, int)` | **external** |

A contract is keyed by **name**, so declaring `add` would be exactly the
"declare on a neighbour's shape" error the brief warns against. Two further
grounds, each sufficient on its own: the cmetrics `add` is **`static inline`
with no header declaration at all**, so there is no extern C declaration for
constraint 8's ABI gate to agree with; and it is **defined in the corpus unit
that calls it**, which makes W4 — not declaration — the correct lever.
Residual **`ADD-NAME-COLLISION`**; discharge condition is a unit-scoped
declaration vocabulary, which v1 does not have.

### 6.2 `memcpy` and `memset` — 9 functions of yield, declined on the result mode

Their 9 sole-blocked functions are all type-punning helpers (`XXH_read32/64`,
`get_unaligned32/64`, `put_unaligned16/32/64` in `rdxxhash.c`/`snappy.c`;
`ctr_opts_init`, `LZ4_resetStream`). Custody-neutral, and tempting.

They were declined because **both return `void *` aliasing their first
parameter**, and the v1 result vocabulary cannot say that. Its choices are
`return_owned`, `return_reallocated(p)`, and `return_none`. `return_none` is
*available* — `flb_input_chunk_get_name` and `__errno_location` are both
pointer-returning and declared `return_none`, so there is precedent — and it is
*conservative* (dropping the alias makes a later `free` of the result release
nothing and the original leak, which is the accusing direction, never an
admission). But it is a premise that is **imprecise in a way the reading can
see**, and this card's other six are each exactly faithful. Declaring it would
also have to survive glibc's decorated prototypes
(`extern void *memcpy (void *__restrict __dest, const void *__restrict __src, size_t __n) __THROW __nonnull ((1, 2));`,
`/usr/include/string.h:46-48`), which is an untested ABI path.

Residual **`LIBC-MEM-RESULT-ALIAS`**, discharge condition: a v1 result mode
meaning "aliases parameter *n* and creates no custody" — at which point these 9
functions plus the wider `memcpy`/`memset` leaf mass (69 and 61 leaves) become
declarable in one step. Recorded as a **vocabulary gap**, not as a judgement
that the functions are undeclarable.

### 6.3 Not attempted, and why

`fprintf` (1,138 leaves) and `snprintf` (1,044) carry the corpus's largest leaf
mass and were **not** selected, because they are the sole blocker of **1 and 0**
functions respectively. Declaring them would decide thousands of leaves and
move almost no function — the brief's point about choosing by cover yield,
confirmed by measurement rather than taken on faith. They remain good
candidates for a *leaf*-directed card, and both would need the variadic-tail
vocabulary this card has now exercised once.

---

## 7. What was touched

* **New** (6): `contract/qcore/cfl/sds_lifetime.qcore`,
  `contract/qcore/cmetrics/context_lifetime.qcore`,
  `contract/qcore/ctraces/context_lifetime.qcore`,
  `contract/qcore/librdkafka/cjson.qcore`,
  `contract/qcore/librdkafka/kafka_mem.qcore`,
  `contract/qcore/monkey/logging.qcore`.
* `contract/cycle19-resource-contracts.json` — six manifest rows, 21 → 27.
* **Constraint 5 pins, updated deliberately and said so:**
  * `compiler/tests/qcore_contract_corpus.rs` — corpus 76 → **82**, six
    `EXPECTED_CONTRACTS` rows with their clause counts (3, 3, 3, 3, 4, 5).
  * `compiler/tests/contract_loader.rs` — `source_assumptions().len()` 21 → 27
    (twice), `modules.len()` 76 → 82,
    `(selected, v1_declarations, rejected_declarations)` **(21, 43, 145) →
    (27, 49, 145)** (the rejected count is unchanged because every added module
    is entirely v1), and the six declarations added to the explicit
    old-table parity list — `mk_print` inside the all-borrowed loop at arity 2,
    the five releasing ones listed individually so each nullability marker is
    visible as its own decision.
  * `crates/qcore-c-gateway/tests/prod_contracts.rs`,
    `crates/qcore-c-gateway/tests/neighbours.rs` — manifest count 21 → 27.
* `docs/development/increments/wave-36.md` — one paragraph prepended.
* `AGENTS.md` — headline numbers.
* This report plus `artifacts/`, `logs/`, `scripts/`.

**No crate source file changed.** No gateway checker, loader, W4 or backend
code was touched, which is what makes every movement in §5 attributable to the
six premises alone.

---

## 8. Gate

See `logs/gate-exits.txt` for every captured exit code (captured to a file, never
through a pipe) and §11 for the run's own statement. Disk was checked before
the gate: **255 GB free**, well above the 60 GB floor, so the full gate ran.

---

## 9. Named follow-ons

| Name | What it records | Discharge condition |
|---|---|---|
| `W36-COVER-HIDDEN-BLOCKER` | A sole-blocker cover is an upper bound on function moves: a refusal reachable only after the first is removed is invisible to it (cost this card 3 of 23). | A cover modelling the projection's reachability, not only its emitted leaves. |
| `CJSON-DELETE-NESTED-DETACH` | The 3 cJSON functions exchanged their decided leaf for `nested call … inside a modeled call`. | Nested-call-in-argument-position admission, or W4 summaries for the three `cJSON_DetachItemFrom*` callees (same unit). |
| `KAFKA-MEM-FREE-SUMMARY-CONFLICT` | W4 derives `Released{nonnull}` where the faithful declaration says `nullable`; costs `rd_kafka_mem_free` its own `Safe`. | A W4 derivation that reads a delegate's declared nullability instead of defaulting a released formal to `nonnull`. |
| `MK-PRINT-TAIL-ACTUALS` | 26 sites refuse `arguments beyond the declared parameters of mk_print` — the tail clause admitting nothing on its own, as designed. | A per-site tail-actual admission, or acceptance as a measured boundary. |
| `CJSON-DELETE-SIBLING-CHAIN` | `cJSON_Delete` releases the whole `next` chain and subtree; the declaration states the fact about `item` only. | A v1 mode for "releases a set reachable from this parameter", or a call site binding two items of one chain. |
| `LIBC-MEM-RESULT-ALIAS` | `memcpy`/`memset` declined: v1 cannot say "the result aliases parameter 1". | A v1 result mode meaning exactly that; unlocks 9 sole-blocked functions plus 130 leaves. |
| `ADD-NAME-COLLISION` | `add` declined: three distinct definitions, three signatures, the cmetrics one `static inline` with no header declaration. | A unit-scoped declaration vocabulary (v1 has none); W4 is the right lever meanwhile. |
| `W36-CTRACES-FAMILY-FIRST` | `contract/qcore/ctraces/` is a new family with one module; its manifest ordering and naming convention are this card's choice alone. | Integrator confirmation, or a second `ctraces` module settling the convention. |

**RETIREMENTS: none.** This card retires no residual and removes no
`named_followups` entry. Stated explicitly because the matrix's union merge
loses unnamed removals.

---

## 10. For the integrator

Please work these as items, not as headline numbers — the wave-32
`W32-RECONCILE-SWEEP` finding is that the numbers get applied and the prose does
not.

1. **Ledger headline numbers** (`AGENTS.md`), applied in this card's own edit —
   please verify: projected functions **186 Safe / 84 Unsafe / 4,564 Unknown →
   205 / 84 / 4,545**; unit statuses **unchanged** at 2/42/137/4; `leak` **272,
   unmoved** (no re-pin); `unsupported` **50,106 → 49,922**; `unknown_alias`
   **4,214 → 4,308**; `summary_conflict` **0 → 1**; contract corpus **76 → 82**;
   resource-v1 declarations **43 → 49**; manifest-selected **21 → 27**.
2. **The baseline moves to `docs/reports/w36-corpus-contracts-3/artifacts/`**,
   and the brief's named baseline was **stale by one** — please correct any
   chain text that still says `c-gateway-constant-do-while`; the ledger's
   `joined-provenance-cycles` was right (§1).
3. **If `JOINED-PROVENANCE-CALL-FRAMES` merges before this card**, this card's
   BEFORE must be re-run against the new baseline before merging, exactly as
   §1 says. `origin/main` was still `60958bf0` when these runs were made. The
   AFTER is unaffected in kind but the *attribution* is only sound against a
   BEFORE that reproduces whatever baseline is then current.
4. **`summary_conflict` is a new diagnostic category in this corpus** (0 → 1).
   If any tooling enumerates the corpus's categories — `diff_artifacts.py`
   sections, the reason-code sidecar tripwire (constraint 24), the matrix's
   `corpus` block — it now sees a fourth. Please check rather than assume; I
   did not find a pinned category list, but I could not rule one out.
5. **Proposed amendment text for standing constraint 7.** After the existing
   text, please add, in these words:

   > Card `W36-CORPUS-CONTRACTS-3` (wave 36) moved the number **not at all**:
   > 272 → 272 across a re-pin that added six declared contracts and moved 21
   > function statuses. A declarative card that decides 204 uncontracted-call
   > leaves is therefore not automatically a leak-pin event, and the pin was
   > re-verified rather than re-derived.

6. **Proposed amendment text for standing constraint 8.** After the existing
   text, please add, in these words:

   > The mode gate's `SummaryConflict`-only arm for a **nullability
   > refinement** was exercised for the first time by a *declaration* rather
   > than by the corpus: card `W36-CORPUS-CONTRACTS-3`'s `rd_kafka_mem_free`
   > declares `Released{nullable}` where W4 derives `Released{nonnull}`, and
   > the declaration is the faithful one — `rd_free(NULL)` reaches
   > `free(NULL)`. The declaration governs every call site, the defining unit's
   > own function moves `Safe → Unknown`, and **that is the gate working**: a
   > disagreement is reported, never silently resolved, and declaring `nonnull`
   > to silence it would assert a precondition the body does not have
   > (constraint 4).

7. **Constraint 14 (keyword census): this card does not engage it.** No word is
   reserved and the lexer is untouched; a `.qcore` contract module adds no
   source-language surface. Please list it among the wave's non-engaging cards.
8. **`contract/qcore/ctraces/` is a new family.** Its placement in the manifest
   (after `cmetrics/`, before `monkey/`) and the module name
   `context_lifetime.qcore` (paired with cmetrics' identically-named sibling)
   are my choices; overrule either if the corpus has a convention I missed
   (`W36-CTRACES-FAMILY-FIRST`).
9. **Eight residuals in §9 reach no tracker yet.** `--residuals` will list them.
   Please add all eight to `named_followups` (**121 → 129** if none collides);
   `W36-COVER-HIDDEN-BLOCKER`, `KAFKA-MEM-FREE-SUMMARY-CONFLICT` and
   `LIBC-MEM-RESULT-ALIAS` are the three I would keep visible — the first
   because it bounds what any future cover card may promise, the second because
   it is a live W4 defect with a concrete repair, the third because it is worth
   ~9 functions and ~130 leaves for one vocabulary addition.
10. **The acceptance matrix was deliberately NOT regenerated**, on
    `W31-CORPUS-CONTRACTS-2`'s precedent — that card, the directly analogous
    declarative arbiter, touched neither `acceptance_matrix.json` nor
    `build_matrix.py`. `named_followups` is therefore still at **121** in the
    tree and the eight names in §9 are carried by this report and the increment
    record only, which is the half of the `DOCS-COMPACT-4`/`-5` defect the
    `--residuals` sweep exists to catch. Two matrix fields worth a glance at the
    close: `corpus.contract_modules_per_directory` and
    `corpus.contract_clause_pin` are derived from card B00's **frozen** archive
    and describe a 59-module checkpoint beside a corpus this card takes to 82 —
    that divergence is the already-recorded `D02-B00-LEDGER-SNAPSHOT`
    (constraint 25), it widens here, and it must be repaired by publishing
    provenance beside them and **never** by editing the archive (constraint 21).
11. **Nothing here is a claim about upstream.** Every `Safe` is `Safe` relative
    to six declared premises; the added `unknown_alias` leaves are
    `Unknown`-class evidence and not defect claims (invariant 2).

---

## 11. Reproduction

```sh
git submodule update --init --recursive
cargo build --release -p qcore-compiler --example cycle19_fluent_bit_libraries_audit
cp target/release/examples/cycle19_fluent_bit_libraries_audit measure/w36/bin/audit
# BEFORE: with the manifest at its 21-module form (the six .qcore files absent)
python3 -B measure/w36/run_audit.py before measure/w36/bin/audit
python3 -B measure/w36/byte_identity.py measure/w36/runs/before \
    docs/reports/joined-provenance-cycles/artifacts
python3 -B measure/w36/predict.py measure/w36/runs/before
# ... land the six modules and the six manifest rows ...
python3 -B measure/w36/run_audit.py after measure/w36/bin/audit
python3 -B measure/w36/census.py --before measure/w36/runs/before \
    --after measure/w36/runs/after
python3 -B measure/w36/evidence.py
sh measure/w36/gate.sh
```

The pin is the shared long-lived one,
`/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin`, with
`QCORE_CLANG=/usr/bin/clang` and the wave-20 ChunkIO package;
`QCORE_SOURCE_CONTRACTS` is deliberately unset (the two selectors are
ambiguous together). Scripts are archived under `scripts/`, logs under `logs/`,
and both stages' five artifacts under `artifacts/`. A run takes four to five
minutes.
