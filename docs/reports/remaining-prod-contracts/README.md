# PROD-CONTRACTS — production declared contract text for the logging family, the name accessor and the routes-mask helpers — 2026-09-09

The named continuation of cards **O04-TRACE**
([`docs/reports/remaining-o04-trace/README.md`](../remaining-o04-trace/README.md)
§2.4 and `O04-TRACE-REACH`) and **HASH-CONTRACTS**
([`docs/reports/remaining-hash-contracts/README.md`](../remaining-hash-contracts/README.md)
§5.3 and §10). O04-TRACE built the variadic-data-tail vocabulary and the
one-level nested-call admission and deliberately left its two callee
declarations in the gateway fixtures, recording the production reach of both as
**zero admitted statements**. HASH-CONTRACTS landed the first production
declaration and recorded that its four publications stay refused in production
because a *neighbouring* callee is uncontracted. This card brings that text into
`contract/qcore`.

**Base tree:** main `009bde0` ("Consolidate wave 13: C6 discharge, reach
frontier, corpus re-pin"), submodules initialised, `third_party/libevent` at
`d82464a2`. Every measurement below was taken on that tree with only this card's
own diff applied.

---

## Headline

**The 42-provisional-`Unsafe` row does not move, and NO newly surfaced
provisional `Unsafe` unit appeared — measured, not argued.** The 185-unit
headline is **2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only**,
identical before and after, with **zero unit status moves in either direction**,
**zero unit `reason_code` moves**, **zero function status moves** across all
4,834 projected functions, and **leak diagnostics unchanged at 271**. Nothing is
promoted, at any level.

**Exactly one unit's diagnostics move**, `fluent-bit-chunk/flb_input_chunk.c`:
**58 refusal leaves removed, 17 added**, net −41 (−36 `unsupported`,
−5 `unknown_alias`), across **15 functions**. Every one of the 75 leaves is
attributed to a rule in §3.3. `monkey-source.json` and
`resource-diagnostics.chunk-contracts.json` are **byte-identical**; the
three checked function contracts / 93 Unknown source-effect split is unchanged.

**Three measured corrections to the card brief**, each of which changed what
this card built:

1. **`flb_trace` and `flb_error` are macros, and the corpus never names them.**
   The call sites reach `flb_log_check` and `flb_log_print`. A production
   `flb_trace` declaration would be inert text. §1.
2. **"Hundreds of `flb_trace` sites" is 21 `flb_log_print` refusals.** The
   pinned corpus holds exactly one Fluent Bit unit. §1.
3. **C6's discharge does NOT reach the production diagnostics, and the reason is
   named.** `flb_input_chunk_create` keeps function-level `Unknown` with 36
   diagnostics (was 38). Its `flb_trace` statement moves from *two* refusals to
   *one*, and that one is the fail-closed refusal a declared tail issues when a
   tail actual is not data — caused by a **neighbouring** uncontracted callee
   (`input_chunk_write_header`), which §5.3 isolates by withdrawing that one
   declaration and reproducing the production refusal exactly. §3.4.

The three new modules are **trusted declared assumptions**, each read against
the pinned upstream body first and cited by line. A function-level `Safe` that
ever rests on them is `Safe` **relative to** them, never a proof of the C.

**Pins moved deliberately and loudly: the contract corpus is 61 → 64 modules,
the resource-v1 declaration count 20 → 25, and the manifest's selected-module
count 6 → 9.** §4.

---

## 1. What the call sites actually reach — the card's premise, corrected by measurement

The card brief asked for "the `flb_trace`/`flb_error` logging family with
`resource_variadic_data_tail`", and instructed: *verify the macro expansions —
what function do the call sites actually reach?* They reach neither name.

`flb_trace`, `flb_error`, `flb_warn`, `flb_info`, `flb_debug`, `flb_helper`,
`flb_idebug` and `flb_errno` are **function-like macros**
(`include/fluent-bit/flb_log.h:247-325` at Fluent Bit
`66910c10a4d7eafa810b84229cf2dcf7b0f26f97`). The pinned production build defines
`FLB_HAVE_TRACE` (measured in the prepared CMake database's own command for
`src/flb_input_chunk.c`: `-DFLB_HAVE_TRACE`), so

| Written | Preprocessed |
|---|---|
| `flb_trace(fmt, …)` | `if (flb_log_check(5)) flb_log_print(5, __FILE__, __LINE__, fmt, …)` |
| `flb_error(fmt, …)` | `if (flb_log_check(1)) flb_log_print(1, NULL, 0, fmt, …)` |
| `flb_errno()` | `flb_errno_print((*__errno_location ()), __FILE__, __LINE__)` |

The 185-unit baseline agrees, exactly: across all 185 units the diagnostic
inventory contains **zero** `call to uncontracted function \`flb_trace\`` and
**zero** for `flb_error`, and **21** for `` `flb_log_print` `` — all 21 in
`fluent-bit-chunk/flb_input_chunk.c`, the corpus's only Fluent Bit unit.

Two consequences, both load-bearing for this card:

* **a production `flb_trace` declaration would be inert text.** It would move
  the corpus module count and buy nothing, because no analyzed statement can
  name it. This card declares `flb_log_check` and `flb_log_print` instead.
* **card O04-TRACE's fixture models a function that does not exist.** Its
  prelude declares `void flb_trace(const char *format, ...)`
  (`crates/qcore-c-gateway/tests/o04/prelude.c:188`), so its C6 discharge is a
  claim about a *hypothetical non-macro* `flb_trace`. That fixture is
  byte-unchanged here — its measurement keeps its own premise set — and this
  card's real-source leg drives the same byte-verbatim caller through the
  **upstream macro**, so the preprocessor produces the production shape.

The card brief's expectation that "hundreds of `flb_trace` sites gain a
contracted callee" is refuted by the same measurement, in a second way: the
pinned 185-unit corpus is CMetrics, CTraces, librdkafka, ChunkIO, Monkey and
**one** Fluent Bit wrapper. Hundreds of `flb_trace` call sites exist in Fluent
Bit; **21 reachable `flb_log_print` refusals** exist in the corpus.

---

## 2. Each function, read from the pinned body, with its mode decision

Every declaration below is a **trusted assumption**. Nothing in this card
analyzes a C body; a declaration is a premise, and a function-level `Safe` that
rests on one is `Safe` **relative to** it and is never a proof of the C
implementation. Each module's own header carries the same reading, cited by
file and line, beside the declaration it justifies.

### 2.1 `flb_log_print` — `contract/qcore/fluent_bit/logging.qcore`

```c
void flb_log_print(int type, const char *file, int line, const char *fmt, ...)
```
(`src/flb_log.c:1242`.)

| Parameter | What the body does | Mode |
|---|---|---|
| `type` | `int` by value; compared, and passed to `flb_log_construct` and `flb_log_message_type_str` | `resource_borrowed` |
| `file`, `line` | forwarded to `flb_log_construct` (`src/flb_log.c:1179`), whose **first two statements are `(void) file;` and `(void) line;`** — discarded | `resource_borrowed` |
| `fmt` | reaches `vsnprintf(body, body_size, fmt, *args)`, where `body` is a function-local `char` array; read, never freed, never stored | `resource_borrowed` |
| the tail | consumed by `va_start`/`vsnprintf` into that same local `body`. What leaves afterwards (`log_queue_enqueue`, `log_render_message`, `fprintf`) carries the **rendered text** of `body`, never a tail argument's pointer | `resource_variadic_data_tail()` |
| result | `void` | `resource_return_none()` |

That last row is exactly what `resource_variadic_data_tail()` declares — no tail
argument's custody is taken and no pointer to one is retained past the call —
and this is the first time in the repository that the clause is written against
a body that was read for it rather than assumed.

**Not claimed, recorded rather than hidden:** `vsnprintf` *reads through* every
`%s` tail pointer. That is a validity requirement on the caller, not a custody
fact, and this projection models custody only. The declaration neither creates
nor detects a caller that passes a released pointer in a tail position; the
gateway's own liveness precondition (card BORROW-LIVENESS) is what refuses that,
on settled denotations, and it is unchanged here.

### 2.2 `flb_log_check` — same module

```c
static inline int flb_log_check(int l)
```
(`include/fluent-bit/flb_log.h:162`.) One `int` **by value**; reads the
thread-local worker, compares a level, returns a flag. It has no pointer
parameter at all, so under no reading can it affect any caller's custody
obligations. `resource_borrowed` on its one parameter and `resource_return_none`
is the faithful — and only faithful — declaration.

Its triviality is not why it matters. `flb_log_check(N)` is the **branch
condition** every logging expansion wraps its print in, and an uncontracted call
in a condition is refused. Declaring it lets the existing F1 neutral-call
admission (card W4-EXPR-POS, refined by REACH-GATE) decide those conditions.
Nothing about F1 changed: it still demands every parameter borrowed, the result
`None`, an **exact** arity, and every argument admitted by the same predicate.

### 2.3 `flb_input_chunk_get_name` — `contract/qcore/fluent_bit/input_chunk_name.qcore`

```c
flb_sds_t flb_input_chunk_get_name(struct flb_input_chunk *ic)
{
    struct cio_chunk *ch;
    ch = (struct cio_chunk *) ic->chunk;
    return ch->name;
}
```
(`src/flb_input_chunk.c:3849`; `flb_sds_t` is `typedef char *`,
`include/fluent-bit/flb_sds.h:36`.)

**The mode decision the brief asked for — "scalar-or-pointer result per its
body".** The result is a *pointer*, and its faithful mode is nevertheless
`resource_return_none()`: `ch->name` is the chunk's own sds, allocated and freed
by ChunkIO, and the caller must not free it. Declaring
`resource_return_owned(...)` would install a cleanup obligation on the caller for
storage another owner releases — a double-free the projection would then
*demand*. The mode is a decision about custody, not about the C type.

Two things this does **not** say, named rather than hidden:

* the returned pointer **aliases into `ic`**, and the projection does not track
  the handback at all, so it does not model that alias. What keeps this from
  being a fail-open on the admitted route is the gateway's rule, not this
  module: card O04-TRACE's premises N6/N8 admit a `None`-result all-`Borrowed`
  call only where the result occupies a position needing no identity, and never
  bind it to a name, so it cannot outlive the statement. A body that *stored*
  the handback is a different statement shape and is refused. The alias itself
  stays open (O00 blockers B03/R13; `O04-TRACE-HANDBACK`).
* **measured** on the pinned corpus: all 20 call sites of this function in
  `src/flb_input_chunk.c` — the only unit in the corpus that calls it — occur
  inside a logging call's argument list, and **none binds the result**. The
  other two occurrences of the name in the preprocessed unit are its prototype
  and its definition. That is a fact about today's corpus, not a rule; the rule
  is the previous bullet, and it is the gateway's.

`resource_borrowed(ic)` imposes **no** non-null precondition, although the body
dereferences `ic` unconditionally. That is deliberate: a mode demanding entry
custody would assert non-nullness and delete the caller's failure paths.

**Precedence.** This function is *defined* in a corpus unit. A declared contract
always wins over an inferred W4 summary and a disagreement is reported as a
`SummaryConflict` diagnostic, never silently resolved; the unit's own definition
stays analyzable. Measured on the AFTER corpus run: **zero** custody summary
conflicts, in every one of the 136 units that report the count. The unit's own
`flb_input_chunk_get_name` body binds `ch` through a cast, which W4 refuses, so
no summary is derived for it and the declared contract is the only one in force.

### 2.4 `flb_routes_mask_set_by_tag` — `contract/qcore/fluent_bit/routes_mask.qcore`

```c
int flb_routes_mask_set_by_tag(flb_route_mask_element *routes_mask,
                               const char *tag, int tag_len,
                               struct flb_input_instance *in)
```
(`src/flb_routes_mask.c:49`.) **The brief's instruction here was to declare
faithfully or refuse and record why. It is faithful, and it is declared.**
Argument by argument:

| Argument | What the body does with it | Mode |
|---|---|---|
| `in` | read only: `if (!in) return 0;`, then `in->config`/`in->config->router` null-tested, then `in->config->router` and `&in->config->outputs` traversed. Never freed, never stored | `resource_borrowed` |
| `tag`, `tag_len` | forwarded to `flb_router_match` (`src/flb_router.c:140`) and its `router_match` above it, which read the bytes and may run a regex over them. Never freed, never stored | `resource_borrowed` |
| `routes_mask` | **written through**: `memset(routes_mask, 0, sizeof(flb_route_mask_element) * size)` and then `flb_routes_mask_set_bit(routes_mask, …)` per match. Nothing is allocated for it, nothing is freed through it, and the pointer is stored nowhere | `resource_borrowed` |
| result | `int has_routes`, a flag | `resource_return_none()` |

The write is the reason this one needed reading rather than pattern-matching: a
callee that writes through a caller's pointer is not thereby taking custody of
it, and `resource_borrowed` says only the latter. What the declaration does
**not** say is that the write is in bounds — the callee memsets `size` elements
where `size` comes from the router, and nothing here relates that to the
caller's allocation. Named **`PROD-CONTRACTS-MASK-EXTENT`**.

`flb_routes_mask_get_size(struct flb_router *router)`
(`src/flb_routes_mask.c:27`) is declared in the same module: one null test, one
member read, one scalar out. It is included because it is the *sibling* of the
declaration above at the same two call sites in `flb_input_chunk_create`
(preprocessed lines 18762 and 19614), and its blocker there was measured to be
the same `in` the routes-mask call made risky.

### 2.5 Read at the same time and **deliberately left undeclared**

Following card O04-ARMS's precedent for the measured-inert `flb_time_now`: a
declaration with no measured payoff is a premise bought for nothing.

| Function | Read | Why undeclared |
|---|---|---|
| `flb_log_is_truncated(int, const char *, int, const char *, ...)` (`src/flb_log.c:1222`) | same body shape as `flb_log_print`; would take the same faithful declaration, tail included | **zero** call sites in the whole preprocessed corpus — only its prototype occurs |
| `flb_errno_print(int, const char *, int)` (`src/flb_log.c:1321`) | `strerror_r` into a local buffer, then logs; no custody | the `flb_errno()` macro expands its first argument to `errno`, i.e. `(*__errno_location ())`. The **dereference** refuses the statement before any contract is consulted, so a declaration cannot decide those sites |
| `flb_log_check_level(int, int)` (`flb_log.h:153`) and `flb_log_suppress_check(int, const char *, ...)` (`flb_log.h:200`) — the `flb_plg_*` family's two gates | both custody-free | **`PROD-CONTRACTS-PLG`**: F1's admission requires an **exact** declared arity (`QCore.ReachGatePosition.a_mismatched_arity_is_never_admitted`), so a variadic tail can never be admitted in an expression position. `flb_log_suppress_check` occurs *only* in branch conditions, so its declaration is unreachable by construction, and admitting only the outer `flb_log_check_level` gate leaves the inner one refusing |

### 2.6 `resource_stored` is not on the table here

For completeness against HASH-CONTRACTS §2.1: none of these five callees stores
anything, so the question that card had to settle does not arise. Every one of
the five is `borrowed`/`none`, which is the only v1 mode combination with the
caller-side effect each body was read to have
(`QCore.LiteralArgumentPremise.borrowed_is_the_only_mode_with_the_measured_effect`,
over the whole v1 parameter vocabulary).

---

## 3. Corpus impact — every leaf attributed

### 3.1 Provenance

Canonical pinned route, on the shared prepared pinned clone (Fluent Bit
`66910c10a4d7eafa810b84229cf2dcf7b0f26f97`, explicit `/usr/bin/clang` 21.1.8),
following [`docs/reports/corpus-repin/provenance.json`](../corpus-repin/provenance.json):

```sh
python3 -B providers/package.py build --output build/cycle19/provider-packages-prod
QCORE_CLANG=/usr/bin/clang \
QCORE_CHUNKIO_PACKAGE=<worktree>/build/cycle19/provider-packages-prod/qcore.chunkio \
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  --pinned-evidence \
  <pin>/fluent-bit-clang-source \
  <pin>/monkey/monkey-build/compile_commands.json \
  <out>/monkey-source.json <out>/resource-diagnostics.json
```

Run **twice from this same worktree** with the same provider packages: the
BEFORE run was started before any file of this card's diff existed and its
`resource-diagnostics.contracts.json` records the six-module manifest verbatim,
so the isolation carries no path provenance at all. Both exit 0 with **zero
gateway failures**. Transcripts:
[`logs/corpus-before.log.gz`](logs/corpus-before.log.gz),
[`logs/corpus-after.log.gz`](logs/corpus-after.log.gz).

### 3.2 The BEFORE run reproduces the current pinned baseline

[`docs/reports/wave13-corpus/artifacts/`](../wave13-corpus/artifacts/), the
current baseline.

| Artifact | wave-13 archived | BEFORE | AFTER |
|---|---|---|---|
| `diagnostics.json` | `70994e38bac7e45a…` | **identical** | `40691eab1fdd7b34…` |
| `source.json` | `53d55b7e1fb174de…` | **identical** | **identical** |
| `chunk-contracts.json` | `c4f438e4ed3dab3c…` | **identical** | **identical** |
| `source-effects.json` | `64b9216864c3aad7…` | `0437d4722f6774ab…` | `0437d4722f6774ab…` — **identical to BEFORE** |
| `contracts.json` | `358ba47cdc96eee0…` | `d16ef4ab1934a6ba…` | `75f756eae40e9d28…` |

The two artifacts that differ from the archived baseline in the BEFORE run are
the two residue classes CORPUS-REPIN and every consolidation since have
recorded: `contracts.json` path-literal provenance, and `source-effects.json`
package identity (the packages were rebuilt for this run). Both are **identical
between BEFORE and AFTER**, so neither touches the isolation:
`source-effects.json` is byte-identical across this card's whole diff, and the
`contracts.json` change is the three new module texts and the manifest, nothing
else.

Full exhaustive diff against the archived baseline:
[`diff-wave13-to-after.json.gz`](diff-wave13-to-after.json.gz) —
`chunk-contracts` 0 changed rows, `source` 0 changed rows, `diagnostics`
1 changed unit, `source-effects` 3 changed rows (the package-identity residue,
`resource_contracts:<digest>` on the three checked function contracts).

### 3.3 The census — every movement, at leaf level

[`census.py`](census.py) (card O04-TRACE's, extended with a leaf-level diff so
no movement can be summarised into a count), transcript at
[`logs/census.txt`](logs/census.txt):

```
units: 185 -> 185;  added: [];  removed: []
unit status before/after: {'Safe': 2, 'Unknown': 137, 'Unsafe': 42, 'data_only': 4}
unit status moves: 0
NEWLY SURFACED PROVISIONAL UNSAFE UNITS: 0
unit reason_code moves: 0
projected functions: 4834 -> 4834;  function status moves: 0
leak: 271 -> 271 (+0);  unknown_alias: 4410 -> 4405 (-5);  unsupported: 49956 -> 49920 (-36)
units with diagnostic-count changes: 1
  fluent-bit-chunk/flb_input_chunk.c: {'unknown_alias': -5, 'unsupported': -36}
leaves added: 17; leaves removed: 58
```

**Every leaf, by rule.**

| Δ | Category / description | Rule it is attributed to |
|---|---|---|
| **−26** | `unmodeled call, aggregate or indirect access in branch condition` | **`flb_log_check` declared.** Each is an `if (flb_log_check(N))` gate; F1's neutral-call admission now decides the condition, because the contract in force declares its one parameter `Borrowed`, its result `None`, the arity is exact, and the actual is a numeric literal |
| **−21** | `` call to uncontracted function `flb_log_print` `` | **`flb_log_print` declared.** Every occurrence in the corpus; the count is exactly the 21 §1 measured |
| **+3** | `` nested call `flb_input_name` inside a modeled call is outside the resource projection `` | **fail-closed direction.** The statement is now *modeled*, so its tail is examined and the nested call refuses on O04-TRACE's premises — `flb_input_name` is uncontracted (premise N3) |
| **+2** | `` nested call `flb_input_chunk_get_name` inside a modeled call is outside the resource projection `` | **fail-closed direction**, same rule, at two `flb_input_chunk_destroy` sites where a *second* nested call is present, so premise N2 (exactly one nested call) fails |
| **+1** | `` arguments beyond the declared parameters of `flb_log_print` are outside the resource projection `` | **fail-closed direction**, at `flb_input_chunk_create` L19626 — the C6 statement. The declared tail is in force and this call site's tail actuals are refused; §3.4 isolates the cause |
| **−8/+3** | `` unresolved resource identity `ic` `` | attribution movement, §3.5 |
| **−3/+8** | `indirect memory access is outside the resource projection` | attribution movement, §3.5 |

Fifteen functions are touched, all in the one unit: `flb_input_chunk_create`,
`flb_input_chunk_destroy`, `flb_input_chunk_flush`,
`flb_input_chunk_get_real_size`, `flb_input_chunk_map`,
`flb_input_chunk_output_size_subtract`, `flb_input_chunk_protect`,
`flb_input_chunk_release_route`, `flb_input_chunk_release_space`,
`flb_input_chunk_set_limits`, `flb_input_chunk_write_header_v2`,
`input_chunk_append_raw`, `input_chunk_get`, `input_chunk_output_size_add`,
`input_chunk_write_header`. **None of the fifteen changes status.**

**Five of the six added leaves are new refusals.** That is the direction hard
invariant 1 demands and the direction this card wanted: contracting a callee
does not erase what its call site contains, it exposes it to the premises that
must hold. The sixth is the attribution movement of §3.5.

### 3.4 The C6 statement in production — the headline's third correction

Before and after at `flb_input_chunk_create` L19626, the preprocessed
`flb_trace` expansion:

| | Diagnostics at L19626 |
|---|---|
| before | `unmodeled call, aggregate or indirect access in branch condition` **and** `` call to uncontracted function `flb_log_print` `` |
| after | `` arguments beyond the declared parameters of `flb_log_print` are outside the resource projection `` |

The gate is decided; the statement is not. `flb_input_chunk_create` goes from 38
diagnostics to 36 and stays **`Unknown`**, and its remaining refusals name six
callees production leaves undeclared — `generate_chunk_name`,
`cio_chunk_is_up`, `cio_chunk_up_force`, `input_chunk_write_header`,
`flb_time_now`, `cio_chunk_down` — exactly the fixture premises card O04
declared and HASH-CONTRACTS §5.3 predicted would keep blocking.

**The cause is isolated, not inferred.** Test
`the_remaining_production_blocker_is_one_uncontracted_neighbour` withdraws
`input_chunk_write_header`'s declaration from the fixture premise set and
nothing else, and the same statement that the test above shows *decided* becomes
the *same* refusal text the production corpus reports. `tag` is an argument of
that uncontracted call, so C02-P's `seed_call_arguments` seeds it risky and the
tail's premise T4 fails on it.

So: **C6 discharging on the fixture route and C6 refusing in production are the
same rule reading two different premise sets.** Landing those six declarations
is contract-text work of exactly this card's shape, and it is deliberately not
taken here — each needs its own body read, and two of them (`cio_chunk_*`) are
ChunkIO, a different source set. Named **`PROD-CONTRACTS-NEIGHBOURS`**.

### 3.5 The attribution movement, recorded rather than smoothed over

Eleven leaves swap category between `unknown_alias: unresolved resource
identity \`ic\`` and `unsupported: indirect memory access …` — eight one way,
three the other, net five — in `flb_input_chunk_map` and one site of
`flb_input_chunk_create`. **Both are refusals**, so no statement changed from
refused to admitted in either direction, and no exit, function or unit verdict
moves; only which of two refusals is reported does. They cluster at statements
whose surrounding branch condition became a `NoOp`, so the reaching abstract
state at them changed.

The exact internal ordering that decides which of the two fires is **not
established here** — measuring it would need instrumentation this card does not
add. What is established is that the movement is confined to two refusal
categories, that the count of leaves in the union is unchanged in the admitting
direction, and that the unit's verdict, its leak count and every function's
status are identical. Named **`PROD-CONTRACTS-ATTRIB`**.

### 3.6 The configured audits

Run in this same worktree, with only `contract/cycle19-resource-contracts.json`
switched between the six-module and nine-module selections.

| Audit | Result | before vs after |
|---|---|---|
| `contract/profiles/cmetrics-counter` | 1 unit: 0 Safe / 1 provisional Unsafe / 0 Unknown; **1 checked function contract / 6 Unknown** | **byte-identical transcript AND byte-identical report JSON** |
| `contract/profiles/chunkio-core` | 13 units: 0 Safe / **2 provisional Unsafe** / 11 Unknown / 0 gateway failures | **byte-identical transcript AND byte-identical report JSON** |

Transcripts in [`logs/`](logs/). Neither source set calls a Fluent Bit function,
so the three added modules are in force and reach nothing there — which is what
these two runs measure rather than assume.

### 3.7 A cost this card looked for and could **not** measure

`legacy_c::declared_variadic_tails` is called **once per analyzed function**, and
it short-circuits on a substring scan: before this card no `contract/qcore`
module contained the string `resource_variadic_data_tail`, so the scan returned
an empty set at zero parse cost. Landing the first production tail stops that
short-circuit, and every declared module text is then parsed by
`parse_source_program` once per function. That is a real mechanism, read from
the code.

It was **not** measurable here. The 185-unit BEFORE run took ~9 minutes and the
AFTER run ~19, but they ran under very different concurrent load (the host was
running several other agents' audits and this card's own test suites; load
average reached 30), so that comparison isolates nothing. A targeted repeated
measurement on the chunkio-core configured audit, alternating configurations
within one script, could not separate them either:

```
six modules:                 9.35s  9.15s 10.29s
nine modules, with tail:    18.29s 13.29s 12.24s   /  8.87s  8.56s 18.41s
nine modules, without tail:                           6.24s  9.44s 14.23s
```

The minima do not order the configurations at all (nine-with-tail 8.56s beats
six-module 9.15s). **No cost is claimed and none is denied**; the mechanism is
named and a measurement on a quiet host, with a per-environment cache as the
obvious repair, is recorded as **`PROD-CONTRACTS-TAIL-COST`**. This card changes
no gateway code, so it does not take that repair.

---

## 4. Pin updates — stated loudly

| Pin | before | after | Why |
|---|---|---|---|
| `compiler/tests/qcore_contract_corpus.rs` corpus size | **61** | **64** | the three new modules |
| `compiler/tests/qcore_contract_corpus.rs` `EXPECTED_CONTRACTS` | 146 rows | **151** rows | five `extern fn` rows at their path-sorted positions, with clause counts 3 / 3 / 7 / 6 / 3 |
| `compiler/tests/contract_loader.rs` corpus module count | **61** | **64** | same three files |
| `compiler/tests/contract_loader.rs` `(selected, v1, rejected)` | `(6, 20, 145)` | **`(9, 25, 145)`** | all three modules are entirely v1 and all three are manifest-selected, so the rejected count does not move |
| `compiler/tests/contract_loader.rs` `source_assumptions().len()` ×2 | **6** | **9** | the manifest selects nine modules |
| `contract/cycle19-resource-contracts.json` | 6 modules | **9 modules** | a declaration must be in force to matter |
| `contract/qcore/README.md` | "six modules … 20 declarations" | "nine modules … 25 declarations" | plus a new subsection on declaring the callee the call site reaches |

`production_contracts_preserve_the_previous_resource_modes` keeps its
load-bearing half — every *previous* declaration is still compared against the
historical table — and the five new declarations are listed **explicitly** in
that table, as additions, rather than exempted from the comparison. That is
card HASH-CONTRACTS' convention, followed unchanged.

**No other pin moved, in either direction.** The `third_party/libevent`
submodule revision, the loader's rejected-module count (145), every earlier
card's fixture premise set — `o04_path.qcore`, `o04_arms_callees.qcore`,
`o04_b08_callees.qcore`, `o04_b09b10_callees.qcore`, `o04_read_bound_generic.qcore`,
`o04_trace_callees.qcore`, `hash_contracts_callees.qcore` — and
`contract/qcore/fluent_bit/hash_table_publication.qcore` are **byte-unchanged**,
so cards O04, O04-ARMS, O04-B08, O04-READ-BOUND, O04-B09B10, O04-TRACE and
HASH-CONTRACTS keep their exact premises and their exact measurements. No
diagnostic code was added, removed or downgraded, no gate was weakened, and this
card changes **no production Rust at all**: the whole `crates/` and `compiler/`
diff is contract text, the manifest, two pin tables, one new test file and one
new test fixture.

**The corpus baseline is NOT re-pinned here.** `artifacts/` is this card's
after-measurement, archived as its diff record with a manifest carrying each
SHA-256; the wave-13 baseline is untouched and stays current until an integrator
consolidation moves it. That is the integrator's call, and §3 is the input to it.

---

## 5. Lean — judged not applicable, with the judgment recorded

The card brief reads: *"Lean: judged per content — the vocabulary theorems
exist; add only where new relations appear; record the judgment otherwise."*

**No new relation appears.** This card adds no decision procedure, no admission
rule, no diagnostic and no production Rust line. It adds declared contract text
and the pins that account for it. Every load-bearing sentence it relies on is
already carried by an audited theorem, and each is named here rather than
gestured at:

| Sentence this card relies on | Audited theorem |
|---|---|
| the tail declaration admits nothing by itself | `QCore.VariadicNested.the_declaration_alone_admits_nothing` |
| every admitted tail actual is data | `QCore.VariadicNested.every_admitted_tail_actual_is_data` |
| a tail actual that may hold an identity is never admitted | `QCore.VariadicNested.a_tail_actual_that_may_hold_an_identity_is_never_admitted` |
| without a declared tail an excess actual refuses | `QCore.VariadicNested.an_excess_actual_without_a_declared_tail_is_refused` |
| the accessor's result may occupy only a position needing no identity | `QCore.VariadicNested.a_custody_slot_never_admits_a_nested_call` |
| an owning nested result is refused there | `QCore.VariadicNested.an_owned_nested_result_is_never_admitted` |
| an admitted nested call is the identity on the abstract state, in either evaluation order | `QCore.VariadicNested.an_admitted_nested_contract_is_neutral`, `…evaluation_order_is_unobservable` |
| the gate is admitted in a condition only under an exact arity, a `None` result and all-`Borrowed` parameters | `QCore.ReachGatePosition.a_neutral_call_of_admitted_arguments_is_admitted`, `…a_mismatched_arity_is_never_admitted`, `…a_non_none_result_is_never_admitted`, `…a_non_borrowed_parameter_is_never_admitted` |
| an uncontracted call in a condition refuses | `QCore.ReachGatePosition.an_uncontracted_call_is_never_admitted` |
| `borrowed` is the only v1 parameter mode with the caller-side effect each body was read to have | `QCore.LiteralArgumentPremise.borrowed_is_the_only_mode_with_the_measured_effect` |

`PROD-CONTRACTS-PLG` (§2.5) is a *corollary* of
`a_mismatched_arity_is_never_admitted`, not a new fact: a variadic callee at a
site supplying more actuals than its declared vector is a mismatched arity, so
F1 refuses it in every expression position.

`python3 -B scripts/check_lean_audits.py` therefore stays at **62 audits**,
unchanged, and `docs/reviews/r0_3/acceptance_matrix.json`'s
`validation.lean_audits` tripwire does **not** fire — the Python suite passes
125/125 with no integrator-owned pin left to move on that axis. This is the same
judgment card T00-CT-SCHEMA recorded, reached the same way.

---

## 6. Evidence — 14 assertions

[`crates/qcore-c-gateway/tests/prod_contracts.rs`](../../../crates/qcore-c-gateway/tests/prod_contracts.rs),
through the public gateway entry points only.

### 6.1 Held-out generic family

A `w_*` API deliberately unrelated to Fluent Bit, to ChunkIO **and** to the
`g_*` (O04), `h_*` (O04-ARMS), `k_*` (O04-B08), `r_*` (O04-READ-BOUND), `m_*`
(O04-B09B10), `v_*` (O04-TRACE) and `p_*` (HASH-CONTRACTS) families. Its shapes
mirror the production ones: a scalar-argument level gate in a branch condition,
a variadic printer with a declared data tail, a borrowed-handback accessor
nested in one tail position, and a mask writer.

| Test | Result |
|---|---|
| `the_expanded_logging_shape_is_admitted_end_to_end` | the whole macro-expanded shape — gate condition, printer, nested accessor, data tail — is admitted and the caller's own obligation still settles: **Safe**, certificate rechecks |
| `an_admitted_logging_statement_does_not_hide_a_leak` | the same body without the release is **Unsafe** with exactly one leak: precision in the defect direction too |
| `a_tail_actual_that_may_carry_an_identity_is_never_admitted` | **adversarial row 1**, at every new site class: the tracked handle in a tail position, its address, a dereference, the handle inside the *nested call's* arguments, and the handle inside the *gate's* argument — each refuses **under** the declarations |
| `the_declaration_covers_only_what_it_covers` | the same call with a data actual is Safe and with the tracked handle is not; the two differ only in the actual |
| `withdrawing_the_tail_restores_the_refusal` | **adversarial row 2**, in both shapes the tail carries: the nested accessor beyond the vector restores O04-TRACE's nested refusal, and a plain data actual beyond it restores the arity refusal — distinct texts, both asserted |
| `withdrawing_the_gate_restores_the_branch_condition_refusal` | **adversarial row 3**: the exact `unmodeled call, aggregate or indirect access in branch condition` returns |
| `the_accessor_result_mode_is_load_bearing` | **the accessor's mode, both directions**: `resource_return_none()` admits the nested call; substituting `resource_return_owned(nullable)` on the same declaration refuses the same statement |
| `the_routes_mask_mode_is_load_bearing_in_both_directions` | **the routes-mask mode, both directions**: declared `borrowed` the call moves nothing and a caller that fails to release still leaks; substituting `resource_released` discharges the obligation instead — which is precisely why the mode had to be read off the body |
| `a_certificate_resting_on_the_tail_is_rederived_from_its_own_sources` | **the sharp substitution**: withdrawing the clause from the certificate's retained sources leaves the loaded contract table byte-identical (asserted, per callee) and the **independent recheck refuses** |

### 6.2 Real source

The byte-verbatim upstream `flb_input_chunk_create` and `cio_chunk_open` regions
the six earlier cards pinned, driven through the **upstream logging macro** so
the preprocessor produces the production shape. `fixtures/o04/prelude.c` is
byte-unchanged; the test derives a macro-carrying prelude from it by an explicit
substitution and asserts the substitution happened. The declared input identity
is `CSourceInput::HistoricalStdin`, the pinned compatibility route's own (card
G02) — the expanded `flb_trace` carries `__FILE__`, which an anonymous text
must refuse, and that refusal was observed before the identity was declared.

| Test | Result |
|---|---|
| `the_expanded_logging_statement_is_decided_on_the_real_caller` | neither logging callee is refused as uncontracted and the gate's branch condition is decided — the production declarations reaching the statement O04-TRACE's fixture reached through a `flb_trace` function that does not exist |
| `the_remaining_production_blocker_is_one_uncontracted_neighbour` | **§3.4's isolation**: withdrawing `input_chunk_write_header` and nothing else reproduces the production corpus's exact refusal text at the same statement, while the gate stays decided |
| `withdrawing_each_production_declaration_restores_its_refusal` | three substitutions on the same real source — no `flb_log_check`, `flb_log_print` renamed away, `flb_input_chunk_get_name` renamed away — each restoring the refusal it removed |
| `the_real_caller_is_not_promoted_by_this_card` | the fixture program is **not** Safe and the constructor keeps its own refusals: a decided statement is not a `Safe` function, and a `Safe` function is not a `Safe` unit |
| `the_modules_under_test_are_the_production_ones` | the three modules are the files under `contract/qcore`, they carry the tail clause, they declare **neither** `flb_trace` nor `flb_error`, and all five declarations are in force through the real production manifest with nine selected modules |

One new fixture,
`crates/qcore-c-gateway/tests/fixtures/contracts/prod_contracts_generic.qcore`,
carries the `w_*` declarations. Nothing else under `tests/fixtures/` changed.

---

## 7. Gate results

Run in this worktree at branch `claude/prod-contracts`, base main `009bde0`,
with the Libevent submodule initialised.

| Gate | Command | Observed | Exit |
|---|---|---|---|
| Workspace tests | `cargo test --workspace --no-fail-fast` | **2,321 passed / 0 failed / 7 ignored** | 0 |
| Format | `cargo fmt --all -- --check` | clean | 0 |
| Clippy | `cargo clippy --all-targets -- -D warnings` | clean | 0 |
| Probe | `cargo run -p qcore-compiler --example reevaluation_probe` | ran | 0 |
| Follow-up | `cargo run -p qcore-compiler --example reevaluation_followup` | ran | 0 |
| Lean audits | `PATH=$HOME/.elan/bin:$PATH python3 -B scripts/check_lean_audits.py` | **62 audits passed**, unchanged (§5) | 0 |
| Provider tests | `cargo test --locked --offline --manifest-path providers/Cargo.toml --workspace` | **94 passed / 0 failed** | 0 |
| Provider packages | `python3 -B providers/package.py build` then `verify` on each | five packages built; all five verify | 0 |
| Python | `python3 -B -m unittest discover -s scripts/tests` | **125 ran, 0 failures** | 0 |
| 185-unit corpus | pinned route, before and after | §3, zero gateway failures both runs | 0 / 0 |
| Configured audits | `qcore-audit` over cmetrics-counter and chunkio-core, before and after | §3.6, byte-identical | 0 / 0 |

### 7.1 What this card contributes

**14 new assertions** in one new integration test file, **three new production
contract modules** (five declarations, 22 clauses) and **one new test fixture**.
No existing test was added, removed or renamed. **Zero new Lean theorems**, by
the recorded judgment of §5, so the Lean audit count and the acceptance
matrix's `validation.lean_audits` tripwire are both unmoved — this card leaves
the integrator no pin to regenerate on that axis.

### 7.2 No integrator-owned pin needs to move for this card

`scripts/tests/test_build_matrix_sections.py` passes: 125 Python tests, zero
failures. The contract-corpus and contract-loader pins §4 lists are **not**
integrator-owned and were updated here, deliberately, in the same change as the
text they account for.

### 7.3 Load-flaky

None of the three recorded load-flaky cases (`vp04`, P02 `RLIMIT_NPROC`, P03
confined-write) failed in the full-workspace run recorded above, which was
executed while several other agents' audits were running on the same host. No
assertion was touched.

---

## 8. Open boundaries

Carried forward unchanged: O04's blockers **B02-B07** and **B11**;
**O04-ARMS-SCALAR-CHAIN**; **C02-P-ALIAS**; **P-B08-SHALLOW**,
**O04-B08-BORROWED-AGG**, **O04-B08-DOT**, **O04-B08-CAST**;
**O04-READ-BOUND-SCALAR**, **O04-READ-BOUND-REACH**; **O04-B09B10-FRAME**,
**O04-B09B10-REACH**; **O04-TRACE-DEPTH**, **O04-TRACE-EFFECTFUL**,
**O04-TRACE-HANDBACK**, **O04-TRACE-UNDERSUPPLY**, **O04-TRACE-TAILTRUST**;
**HASH-CONTRACTS-CLAUSE**, **HASH-CONTRACTS-PAIRING**,
**HASH-CONTRACTS-RETENTION**, **HASH-CONTRACTS-CALL-POSITION**.

**`O04-TRACE-REACH` is closed by measurement**: the production reach of the
variadic-tail vocabulary is no longer zero. It is 21 decided `flb_log_print`
callee refusals and 26 decided gate conditions, with 6 refusals in their place
and one unit's diagnostics moving — §3.3. It is *not* a verdict movement, and
saying so is the point.

| # | Boundary | Status |
|---|---|---|
| **PROD-CONTRACTS-NEIGHBOURS** | C6 does not discharge in production because six callees on its path stay undeclared (`generate_chunk_name`, `cio_chunk_is_up`, `cio_chunk_up_force`, `input_chunk_write_header`, `flb_time_now`, `cio_chunk_down`). The blocker is isolated to one of them by measurement (§3.4). Each needs its own body read, and two are ChunkIO, a different source set | **open, named, sized** |
| **PROD-CONTRACTS-PLG** | the `flb_plg_*` family's inner gate `flb_log_suppress_check` is variadic and occurs only in branch conditions, where F1's exact-arity rule can never admit it. Admitting it needs an expression-position rule for declared tails, which is checker work, not contract text | **open, refused by construction** |
| **PROD-CONTRACTS-MASK-EXTENT** | `resource_borrowed(routes_mask)` is a custody statement; it does not say the callee's `memset` of `size` elements is inside the caller's allocation | **open, a non-claim** |
| **PROD-CONTRACTS-ATTRIB** | eleven leaves swap between two refusal categories (§3.5); the internal ordering that decides which fires is not established here. No verdict moves in either direction | **recorded, not repaired** |
| **PROD-CONTRACTS-TAIL-COST** | the first production tail stops `declared_variadic_tails`' substring short-circuit, so every declared module text is parsed once per analyzed function. The mechanism is read from the code; the cost could **not** be measured on this host (§3.7). Needs a quiet-host measurement, and a per-environment cache if it is real | **open, unmeasured** |
| **PROD-CONTRACTS-UNDECLARED** | `flb_log_is_truncated`, `flb_errno_print`, `flb_log_check_level` and `flb_log_suppress_check` were read and deliberately left undeclared, each on a measured ground (§2.5). If a later corpus reaches them, the reading is already recorded | **recorded** |
| — | a tail actual carrying `&`, `*`, a subscript or a member access; a nested call that is part of an expression; two nested calls in one argument list; a tail actual naming a risky name | **refused by construction, and asserted** |

---

## 9. Scope and non-claims

- **No raw unit is promoted, and none moved.** The 185-unit aggregate is
  unchanged in every direction; no unit verdict, no unit `reason_code` and no
  function status moved; no newly surfaced provisional `Unsafe` unit exists.
- **Every declaration here is a trusted assumption.** Each was read against the
  pinned upstream body and cited by file and line, and nothing in this card
  analyzes a C body. A function-level `Safe` resting on one is `Safe`
  **relative to** that declaration; it is never a proof of the C implementation
  and must never be presented as one.
- **A decided statement is not a `Safe` function, and a `Safe` function is not a
  `Safe` unit.** `flb_input_chunk_create` keeps `Unknown` in production, the
  fixture program keeps `Unknown`, and the unit keeps its provisional `Unsafe`.
- **A provisional `Unsafe` is never an upstream defect claim**, here or
  anywhere; the `w_*` family is synthetic and its `Safe` is `Safe` for a
  synthetic declared custody projection.
- **`resource_borrowed` says nothing about validity, bounds or retention.** The
  printer reads through its `%s` tail pointers; the mask writer writes through
  its first argument; the accessor hands back a pointer that aliases into its
  argument. None of those is a custody fact and none is modeled.
- **The containment publications remain unmodeled** (O00 blockers B03/R13).
- **Unknown means Unknown.** A statement whose path carried a refusal alleges
  nothing in either direction.
- Projection coverage is not safety coverage; unknowns stay in the denominator.
- CT coverage is unchanged at 32/50; `qcore-schemas` is unchanged at 10.0.0; no
  plugin, protocol or evidence version moved; no diagnostic code was added,
  removed or downgraded; no production Rust file changed.

---

## 10. Files changed

| Path | Change |
|---|---|
| `contract/qcore/fluent_bit/logging.qcore` | **new** — `flb_log_check` and `flb_log_print`, the latter with `resource_variadic_data_tail()`; the header carries the body reading, the macro-expansion measurement and the four deliberate non-declarations |
| `contract/qcore/fluent_bit/input_chunk_name.qcore` | **new** — `flb_input_chunk_get_name`, borrowed, `resource_return_none()` on a pointer result, with the mode decision and the alias non-claim |
| `contract/qcore/fluent_bit/routes_mask.qcore` | **new** — `flb_routes_mask_set_by_tag` and `flb_routes_mask_get_size`, argument by argument |
| `contract/cycle19-resource-contracts.json` | 6 → 9 selected modules |
| `contract/qcore/README.md` | the selection paragraph, plus "Declare the callee the call site reaches" |
| `compiler/tests/qcore_contract_corpus.rs` | corpus size 61 → 64; five `EXPECTED_CONTRACTS` rows |
| `compiler/tests/contract_loader.rs` | module count 61 → 64; `(6,20,145)` → `(9,25,145)`; two `source_assumptions().len()` pins 6 → 9; five explicit rows in the historical-mode table |
| `crates/qcore-c-gateway/tests/prod_contracts.rs` | **new** — 14 assertions |
| `crates/qcore-c-gateway/tests/fixtures/contracts/prod_contracts_generic.qcore` | **new** — the held-out `w_*` API |
| `docs/reports/remaining-prod-contracts/` | this record, the census and archive tools, the artifacts, the full diff and the transcripts |

No production Rust changed. No file owned by the integrator was touched:
`AGENTS.md`, `CLAUDE.md`, `README.md`, `DEVELOPMENT.md`,
`docs/reevaluation/ct_manifest.json`, `docs/reviews/r0_3/acceptance.json` and
`docs/reviews/r0_3/acceptance_matrix.json` are unchanged, as is every archived
predecessor report, every existing contract module and every existing test
fixture.
