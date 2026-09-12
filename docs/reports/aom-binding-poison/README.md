# AOM-BINDING-POISON — a refused statement stops destroying what it cannot write — 2026-09-12

## Headline

**Card AOM-DESTROY-SIBLINGS' residual `AOM-SIBLINGS-BINDING-POISON` is
repaired, and the declaration it left unexercised now fires.** Every refused
statement used to set `unknown` and `may_be_null` on **every** binding in the
state, including bindings its own text cannot have written. It no longer does,
where three premises hold — a plain name, an automatic object of this body whose
address is never taken, and no write position for it in the refused text.

**The measured payoff, on the leaf that names the residual.** At the three exits
`L19384`, `L19392` and **`L19399`** of `flb_input_chunk_get_direct_routes`, the
acquisition `site0@L19357` moves from `Mixed` — the withdrawal-by-refusal state —
to **`Released`**. `L19399` is `CONSTRUCTOR-RESIDUALS-LEAK-271`, the leaf wave 17
added and card AOM-DESTROY-SIBLINGS **decided by withdrawal**; it is now decided
by a **modelled release that discharges**. That is the discharging arm of
`flb_input_chunk_destroy_direct_routes`'s contract, which the preceding card
measured as "still unexercised anywhere in the pinned corpus", firing for the
first time. Both probe transcripts are archived (§5).

**Nothing is promoted, at any level, and no pin moves.** Unit verdicts are
unchanged at **2 `Safe` / 42 provisional `Unsafe` / 137 `Unknown` / 4 data-only**
with **zero** status moves and zero `reason_code` moves; the projected-function
histogram is unchanged at **150 / 84 / 4,600** with **zero** function status
moves across all 4,834 projected functions; **`leak` stays at 266** with not one
leak leaf added or removed anywhere in the 185-unit corpus, so constraint 7's
pin does **not** move and its four-move history is unchanged. The contract
corpus is untouched (73 modules, 18 manifest-selected, 18 raw keys before and
after).

**87 leaves moved — 1 added, 86 removed — every one of them `Unknown`-class,
across 26 units, each named in §4.** Corpus totals: `unknown_alias`
4,421 → **4,344** (−77), `unsupported` 49,781 → **49,773** (−8), `leak`
266 → **266**. Both audit runs exit 0 with zero gateway failures in all six
scopes. The BEFORE run reproduces the authoritative baseline — card
AOM-DESTROY-SIBLINGS' AFTER archive, per that report's §12 — **byte for byte**
in `diagnostics.json` and `chunk-contracts.json`.

**The prediction was right about the structure and wrong about the size**, in
the direction that made the card easier rather than harder: the predicted
`unknown_alias` fall of 200–1,500 measured **77**, and the predicted net leak
*increase* of up to 450 measured **zero**. §3 says exactly why, and the reason is
the repair's own boundary rather than an accident.

---

## 1. The mechanism, and the repair

### 1.1 What the poison was

`crates/qcore-c-gateway/src/legacy_c.rs`, `transfer`'s arm for
`LegacyInstruction::Unknown`, before this card:

```rust
for pointer in state.pointers.values_mut() {
    pointer.unknown = true;
    pointer.may_be_null = true;
}
```

`state.resources` is not touched by this arm and never was, so the poison is
purely about **names**: which acquisition identity a place denotes, and whether
it may be null. Card AOM-DESTROY-SIBLINGS located it at `legacy_c.rs:6483-6486`
and measured its cost: between the acquisition at preprocessed line 19357 and
the first release of `result` at L19383 there are three refused statements, so
`result`'s *binding* did not survive, twelve declared releases could only refuse,
and six leak leaves retired by **withdrawal** rather than by discharge.

### 1.2 The write-set classification

The honest question the card asks is which refusal classes have a write set that
is sound to bound. The answer this card implements turns on **what the refusal
node stands for**, not on the refusal's wording:

| class | stands for | write set | answer |
|---|---|---|---|
| every refusal `Parser::classify_simple` raises | exactly one statement, or one `for` init/increment clause, whose **complete** token run it was handed | that text's own write positions | **bounded** |
| a refused `return`, `if`, `while`, `for`, `do`, `switch` or `case` **condition** | an expression the CFG builder classified elsewhere (`condition_supported`, `null_condition`, `distinct_condition`); the instruction never held its tokens | — | **unbounded** |
| `goto` to an unresolved label; malformed for-loop clauses; `break`/`continue` outside their construct; a stray or duplicate label | a control-flow edge, not a text | — | **unbounded** |

The bounded half covers, by wording, `indirect memory access is outside the
resource projection`, `braced initializer, expression, or declaration is not
modeled` (a GNU statement expression's inner statements are inside the same
token run, because `consume_to_semicolon` tracks delimiters), `unrecognized
assignment destination`, `unsupported assignment expression`, `call to
uncontracted function …`, every withdrawn-declaration wording, `C operator
\`sizeof\` requires type/operand analysis`, and the rest of `classify_simple`'s
list. That is a decision about the *reader*, not about the words: any future
refusal raised from `classify_simple` is bounded by construction, and any raised
elsewhere is not.

### 1.3 The three premises

`RefusedWrites::Sparing(set)` carries a **closed, positively built allow-list**
of the places the refusal spares. Everything outside it — including every place
the projection has not bound yet — is poisoned exactly as before. A place is on
the list only when all three hold:

1. **it is a plain name.** `canonical_place` and `direct_assignment_destination`
   build `NAME`, `NAME.f`, `NAME->f`, `NAME[0]` and `*NAME`; only the first is a
   slot, and every other shape denotes storage another lvalue can reach.
2. **`frame_protected_locals` recognised it.** It must be a name
   `publication::body_declarations` read out of this body's own declaration runs,
   or a formal; not a `static` local (whose object outlives the call exactly as a
   file-scope object does); not also a file-scope object of this translation unit
   (which is what keeps a misread declaration run — `a * b;` contributing `b` —
   from sparing a global); and not a name **any** `&` in the body is followed by,
   after a run of `(`. To take a declared object's address you must name it, and
   after any parentheses that name comes first, so `&p->f`, `&a[i]` and a bitwise
   `x & y` all drop a name they did not need to — the harmless direction.
3. **`may_write_names` finds no write position for it** in the refused text.

Under those three the statement cannot change the value of that slot: a store
through a pointer, a subscript or a member selection writes the pointed-to
object, and a call can reach only what its arguments and file-scope objects give
it. The premise is the ordinary C object model under the same
**no-undefined-behaviour** reading the projection already makes when it models
`flb_free(p)` as releasing exactly `p`'s site — a write past the end of one
object onto another is undefined, not a modelled effect. Stated, not hidden:
`AOM-POISON-NO-UB` (§8).

### 1.4 `may_write_names`, the load-bearing reader

It is written as a **positive recognition of C's write positions**, with every
operator run it does not recognise defaulting to *written*. The importer's lexer
emits one token per punctuation character, so `==` is `=` `=`, `+=` is `+` `=`,
`++` is `+` `+` and `->` is `-` `>`.

A name at index `i` is written when, skipping a run of `)`:

* the operator run that follows contains `=` and does not begin with one of the
  four comparisons `==`, `!=`, `<=`, `>=` — which covers `=` and every compound
  assignment including `<<=` and `>>=`, and defaults **toward** written for any
  run this reader does not know;
* or that run begins `++`/`--`, or the two tokens before the name are;
* or the next token is `.`, so a member store into a struct **object** is
  attributed to the object.

Every other occurrence is a read: a subscript base (`a[i] = v` writes `*(a+i)`),
a `->` base, a by-value argument, an operand. Reserved words are deliberately
**not** excluded — over-recognising a write costs precision, missing one would be
unsound. `crates/qcore-c-gateway/tests/unit/legacy_c.rs`'s
`a_refused_statement_reads_its_write_positions_and_defaults_to_written` pins
fourteen write shapes and thirteen read shapes.

Taking an address is not read here at all: premise 2 has already removed every
name a `&` reaches anywhere in the body, which is what makes "a call can only
write what it can reach" true of the names this set does not mention.

### 1.5 What the repair cannot do

Two properties, each resting on a code path rather than on the measurement:

* **No function carrying a refusal can reach `Safe`.** The repair only changes
  the transfer of `LegacyInstruction::Unknown` nodes; a function containing one
  carries an `Unsupported` diagnostic, and `classify_diagnostics` maps
  `Unsupported` to at least `Unknown`. A function with no refusal has no such
  node and its analysis is bit-for-bit unchanged.
* **No W4 summary can get stronger.** Every path through a refusal sets
  `state.unmodeled` (`is_path_poisoning` on the `Unsupported`), and
  `observe_residual` maps `unmodeled` to `ExitCustody::Unknown`. Both probe
  transcripts show it directly: **every** exit of
  `flb_input_chunk_get_direct_routes` reads `custody Unknown`, before and after.

Both are proved as theorems in §7 and both held in the measurement: zero
function status moves, zero unit status moves, nothing `Safe` anywhere.

---

## 2. The prediction, and where it landed

[`prediction.md`](prediction.md), commit `18bc16ca`, committed **before** the
implementation commit and before any AFTER audit existed, and not edited
afterwards.

| prediction | measured |
|---|---|
| **P1** no function moves to `Safe` | **held** — zero function status moves of any kind |
| **P2** no W4 summary gets stronger | **held** — every exit of the flagship function reads `custody Unknown` in both probes |
| **P3** no unit is promoted | **held** — 2/42/137/4 before and after, zero moves |
| **P4** `summary_conflict` 0 in all 136 reporting units | **held** — 0 in both runs |
| **P5** contract corpus unmoved (73/18/39/145) | **held** — 18 raw keys before and after, 0 added, 0 removed, no collision |
| **P6** both runs exit 0, 185 units, BEFORE reproduces the baseline byte for byte | **held** — §5 |
| the 12 `unknown_alias: … \`result\`` leaves are removed | **3 of 12** — L19383, L19391, L19398. §3 |
| the six retired leak leaves convert from withdrawal to discharge | **3 of 6** — L19384, L19392, **L19399**, each `Mixed` → `Released`. §3 |
| L19544 stays; no leaf added in that function; status stays `Unsafe` | **held** — 92 → 89 diagnostics |
| `flb_input_chunk_map` L18929 stays (`&direct_routes` is in that body) | **held** — unmoved |
| `unknown_alias` falls by 200–1,500 | **wrong** — falls by **77** |
| `unsupported` unchanged to within ±5 | **wrong** — falls by **8**, all attributed in §4.2 |
| `leak` rises, 266 → between 266 and 450 | **wrong** — **266**, not one leak leaf moved |
| unit/function status moves in the `Unknown` ↔ `Unsafe` direction, first dominating | **wrong** — **zero** moves in either direction |
| **recorded as NOT expected:** anything reaching `Safe` | **did not happen** |

**Three of the four numeric predictions were wrong, all in the same direction and
for the same reason**, which §3 sets out: the repair's own boundary. The
prediction reasoned from the poison's *ubiquity* — every refused statement
poisoned everything — and did not weigh how often a refused **condition**, which
this card deliberately leaves unbounded, stands between an acquisition and its
release. It does so constantly. The mechanisms the prediction named are all real;
their reach is a tenth of what it guessed.

The accusing arm — leaks reappearing where a blanket `mark_unresolved_effect` had
suppressed them — did not fire **at all**, which is worth stating plainly rather
than glossing as a lucky result. It needs a modelled release on an unresolvable
actual *and* an unrelated live site on the same path, and no such pair exists in
the 185-unit corpus once the bounded classes are the only ones repaired.

---

## 3. The boundary, measured exactly where it bites

The flagship function is the cleanest possible demonstration, because the
boundary runs straight through it. From the AFTER probe
([`logs/probe-after.txt.gz`](logs/probe-after.txt.gz)):

```
L19357  Call flb_calloc(_, _) -> result             result = flb_calloc(…);
L19365  Unknown: indirect memory access …           read_id = … view.routing_data[id_offset] …   <- BOUNDED
L19371  Unknown: indirect memory access …           read_id = … view.routing_data[id_offset] …   <- BOUNDED
L19379  Unknown: unrecognized assignment destination id_offset += id_bytes;                      <- BOUNDED
L19381  Unknown: C operator `sizeof` …              lengths_offset = sizeof(uint16_t) + …;       <- BOUNDED
L19383  Call flb_input_chunk_destroy_direct_routes(result, stored_count)                         <- RESOLVES
L19391  Call flb_input_chunk_destroy_direct_routes(result, stored_count)                         <- RESOLVES
L19398  Call flb_input_chunk_destroy_direct_routes(result, stored_count)                         <- RESOLVES
L19402  Unknown: unmodeled call, aggregate or indirect access in branch condition                <- UNBOUNDED
L19405  !! unknown_alias: unresolved resource identity `result`                                  <- still refuses
```

`result` is declared `struct flb_chunk_direct_route *result;` in that body; the
only `&` in the body is `&view`, every other `&` being bitwise; and the refused
statements above name `result` only as a **subscript base**. So `result` survives
all four bounded refusals and the first three declared releases discharge.

`L19402` is `if ((size_t) (label_data_offset + (int) sizeof(uint16_t)) >
routing_length)`. Its node stands for a *condition*, which this card leaves
`Unbounded`, so it poisons everything — and from `L19405` on all nine remaining
call sites refuse exactly as before. **That single line is the whole difference
between 3 of 12 and 12 of 12.** Named `AOM-POISON-CONDITION-FRAME` (§8); it is
the obvious next card and it is deliberately not this one, because extending the
bound to conditions means changing what `Statement::If`/`While`/`For`/`DoWhile`/
`Switch`/`Case`/`Return` carry from the parser to the builder, which is a
different change with a different blast radius, to be predicted and measured on
its own.

---

## 4. Corpus — every leaf attributed

[`census.py`](census.py), transcript [`logs/census.txt`](logs/census.txt):

```
units: 185 -> 185;  added: [];  removed: []
unit status before/after: {'Safe': 2, 'Unknown': 137, 'Unsafe': 42, 'data_only': 4}
unit status moves: 0
NEWLY SURFACED PROVISIONAL UNSAFE UNITS: 0
unit reason_code moves: 0
projected functions: 4834 -> 4834;  function status moves: 0
leak: 266 -> 266 (+0);  unknown_alias: 4421 -> 4344 (-77);  unsupported: 49781 -> 49773 (-8)
units with diagnostic-count changes: 26
leaves added: 1; leaves removed: 86
```

### 4.1 The 26 units, and the one added leaf

| unit | delta |
|---|---|
| `chunkio/cio_file_unix.c` | `unknown_alias` −4 |
| `chunkio/cio_memfs.c` | `unknown_alias` −4, `unsupported` −1 |
| `chunkio/cio_scan.c` | `unknown_alias` −2 |
| `chunkio/cio_stream.c` | `unknown_alias` −6 |
| `cmetrics/cmt_encode_influx.c` | `unknown_alias` −1 |
| `cmetrics/cmt_encode_opentelemetry.c` | `unknown_alias` −2, `unsupported` −1 |
| `cmetrics/cmt_encode_opentelemetry_batch.c` | `unknown_alias` −2 |
| `cmetrics/cmt_encode_prometheus_remote_write.c` | `unknown_alias` −3 |
| `cmetrics/cmt_encode_splunk_hec.c` | `unknown_alias` −6, `unsupported` −1 |
| `cmetrics/cmt_exp_histogram.c` | `unknown_alias` −8 |
| `cmetrics/cmt_histogram.c` | `unknown_alias` −4, `unsupported` −3 |
| `ctraces/ctr_attributes.c` | `unknown_alias` −2 |
| `ctraces/ctr_encode_opentelemetry.c` | `unknown_alias` −7 |
| `ctraces/ctr_id.c` | `unknown_alias` −4 |
| `ctraces/ctr_resource.c` | `unknown_alias` −1 (**−2 removed, +1 added**) |
| `ctraces/ctr_scope.c` | `unknown_alias` −3, `unsupported` −1 |
| `fluent-bit-chunk/flb_input_chunk.c` | `unknown_alias` −6 |
| `librdkafka/rdbuf.c` | `unknown_alias` −1, `unsupported` −1 |
| `librdkafka/rdkafka_sasl_oauthbearer.c` | `unknown_alias` −1 |
| `librdkafka/rdunittest_msgset_errors.c` | `unknown_alias` −1 |
| `monkey-libevent/mk_core/mk_event.c` | `unknown_alias` −3 |
| `monkey-libevent/mk_core/mk_iov.c` | `unknown_alias` −1 |
| `monkey-libevent/mk_core/mk_memory.c` | `unknown_alias` −1 |
| `monkey-libevent/mk_core/mk_string.c` | `unknown_alias` −1 |
| `monkey-libevent/mk_server/mk_http_parser.c` | `unknown_alias` −2 |
| `monkey-libevent/mk_server/mk_lib.c` | `unknown_alias` −1 |

The **one added leaf** is `ctraces/ctr_resource.c`, `ctr_resource_create` L6774:
`unknown_alias: unresolved resource identity \`attr\`` replaces
`unknown_alias: unresolved resource identity \`res\`` at the same line, with a
second `res` leaf at L6775 removed. `res` now resolves, so the statement reports
the *other* operand it cannot resolve instead. **One `Unknown`-class refusal
replaced by another at the same statement, reporting the reaching state rather
than the first name it failed on** — the pattern card CONSTRUCTOR-RESIDUALS
recorded for `mk_list_add` and card AOM-DESTROY-SIBLINGS recorded at its own four
sites. Net −2 in that unit, no verdict move.

The other 85 removals are the repair working as intended: a binding an unrelated
refusal used to destroy now survives, and the release or guard that reads it
resolves. Sixteen of the removed leaves are in the two functions §3 discusses.
The full leaf-level list is in [`logs/census.txt`](logs/census.txt); nothing is
summarised away into a count there.

### 4.2 The `unsupported` −8, itemised

`unsupported` was predicted unchanged to within ±5. Eight leaves left, and none
of them is a refusal that stopped being raised where it is reached:

**Seven are `nullable resource X used by a non-null contract`** —
`buckets` ×3 (`cmt_histogram.c`), `context` ×1 (`cmt_encode_splunk_hec.c`),
`ins_scope` ×1 (`ctr_scope.c`), `mf` ×1 (`cio_memfs.c`),
`resource_metrics` ×1 (`cmt_encode_opentelemetry.c`). Each is a
`p = alloc(); if (!p) { return …; } … refused statement … ; f(p);` shape where
the callee's declaration is `nonnull`. The blanket poison set `may_be_null = true`
on a pointer a guard had **already proved non-null**, so the non-null position
refused on a modelling artefact. With the binding spared, the guard's refinement
survives and the refusal retires. This is a repair of a false refusal, in the
same class as the 85 `unknown_alias` removals; it is counted separately here
only because it lands in a different diagnostic category.

**One is `call to uncontracted function \`rd_slice_read\``** in
`librdkafka/rdbuf.c`, `do_unittest_write_read_payload_correctness` L4156, and it
is a **finding, not a repair** (§8, `AOM-POISON-NOOP-INCREMENT`). The shape is

```c
for (pass = 0; pass < 2; pass++) {
        …
        if (pass == 0) r = rd_slice_peek(…);
        else           r = rd_slice_read(…);
```

`pass = 0` lowers to `Clear pass` (a definitely-null place); `pass++` lowers to
`NoOp`, so the binding is never updated and `pass` stays definitely-null for
ever; with `pass` spared from the refusal at L4151, `refine_pointer_null` now
returns `Infeasible` on the `pass != 0` edge and that arm is never visited, so
the refusal inside it is never emitted. The stale fact is **not** created by this
card — it is exactly the hazard constraint 5a already records ("expression
mutations need modeled writes rather than a `NoOp`") — but the blanket poison was
masking it, and this card unmasks it. The measured consequence is **one**
`unsupported` leaf in **one** function that stays `Unknown` with 32 refusals,
and zero verdict moves anywhere; the `unknown_alias: \`pass\`` leaf at L4152 that
went with it is a genuine repair. Recorded rather than absorbed.

---

## 5. The route, and both runs' oracles

```sh
git submodule update --init --recursive          # FIRST: the libevent pin gate fires at the END of a run
python3 -B scripts/prepare_cycle19_clang_audit.py \
  --fluent-bit /home/cosmo/Documents/GitHub/fluent-bit \
  --revision 66910c10a4d7eafa810b84229cf2dcf7b0f26f97 \
  --output <worktree>/build/cycle19/clang-pin \
  --clang /usr/bin/clang --clangxx /usr/bin/clang++
cp <pin>/fluent-bit-clang-production/compile_commands.json \
   <pin>/fluent-bit-clang-source/build/compile_commands.json
python3 providers/package.py build --output build/cycle19/provider-packages-poison
sh measure/run-audit.sh <before|after> measure/bin/<before|after>   # archived as run-audit.sh
sh measure/run-probe.sh measure/bin/probe-<before|after> \
   flb_input_chunk_get_direct_routes measure/logs/probe-<before|after>.txt
```

A **fresh** Clang pin, per the wave-20 corpus record: neither the long-lived
developer database nor any earlier card's pin is reused. The pin's own production
database is copied into `<source>/build/compile_commands.json`, which the
preparation script does not create, and **both runs read the same bytes**
(SHA-256 `39053627b60e6929…`, **1,415 entries**, `/usr/bin/clang` spelled, not
`cc`). The analysed source is byte-identical to the two preceding cards':
`src/flb_input_chunk.c` SHA-256 `a537fe347ec1be6c2af7971b55579284e0402f351334f3cfcf0c2c800fe0977e`,
the digest wave 23 recorded. Both runs: **exit 0**, zero gateway failures, all
six scopes summing to **185 units**, 136 reporting units with **0** custody
summary conflicts.

**One route detail worth recording, because a first attempt was discarded for
it.** The BEFORE binary was first built from a `git archive` of the base commit
into a worktree-private directory. That run **failed** at the libevent pin gate
(`SourceRevisionMismatch … actual: a53fe2a4…`): the extracted tree has no
submodule, so the pin reader resolved `third_party/libevent` against that tree's
own root and read the *enclosing* repository's HEAD. The run was discarded whole.
Both measured binaries are built **in the worktree itself**, from the same
manifest directory, with the base gateway sources checked out over the tip's and
restored afterwards, so the only difference between them is the gateway source.

| binary | SHA-256 |
|---|---|
| `measure/bin/before` (base `a53fe2a4`) | `90508855f2d74299b372fc07b01d0b598a0b70a30ae8be693a19ad67bc3f2eff` |
| `measure/bin/after` (card tip) | `d2c08265e8cac59502401edaf264932c0b0528c2dac6587a92851c954e20e435` |

### 5.1 The BEFORE run reproduces the baseline byte for byte

The authoritative baseline is card AOM-DESTROY-SIBLINGS' **AFTER** archive, per
that report's §12: it is the run the tip's leak pin of 266 and its 73-module
contract corpus describe. `docs/reports/wave23-c-operations/` remains the frozen
corpus baseline underneath it and is **not touched** (constraint 21).

[`byte_identity.py`](byte_identity.py),
[`logs/byte-identity-before.txt`](logs/byte-identity-before.txt):

| artifact | archived baseline | BEFORE run |
|---|---|---|
| `diagnostics.json` | `fdbce36b4a815c67…` | **identical** |
| `chunk-contracts.json` | `c4f438e4ed3dab3c…` | **identical** |

and `diff_artifacts.py --archived docs/reports/aom-destroy-siblings/artifacts`
([`logs/diff-summary-before.txt`](logs/diff-summary-before.txt),
[`artifacts/diff-baseline-to-before.json.gz`](artifacts/diff-baseline-to-before.json.gz))
itemises **zero** differences in every section that carries a verdict:

```
--- chunk-contracts: results 96/96 +0 -0 ~0;  backend_teardown 21/21 +0 -0 ~0
--- contracts:       sources 18/18, manifest 18/18, audit_selection 5/5
--- diagnostics:     units 185/185, +0 -0 ~0
--- source:          envelope key `compile_entry`
--- source-effects:  3 result rows changed
```

The residues are the checkout-root and identity-digest classes every prior re-pin
has recorded, and this card can say so exactly: `source.json` and
`source-effects.json` are byte-identical **between this card's own two runs**.

### 5.2 The probes, which is where the discharge is visible

`compiler/examples/alias_aggregate_probe` prints each exit's residual sites and
their `SiteDisposition`, which is the one place the difference between "withdrawn
by refusal" and "discharged" is directly observable. Both transcripts are
archived; the exits table alone
([`logs/probe-exits-before.txt`](logs/probe-exits-before.txt),
[`logs/probe-exits-after.txt`](logs/probe-exits-after.txt)):

| exit | BEFORE | AFTER |
|---|---|---|
| `L19384` | `custody Unknown  ["site0@L19357 Mixed"]` | `custody Unknown  ["site0@L19357 `**`Released`**`"]` |
| `L19392` | `custody Unknown  ["site0@L19357 Mixed"]` | `custody Unknown  ["site0@L19357 `**`Released`**`"]` |
| **`L19399`** | `custody Unknown  ["site0@L19357 Mixed", "site1@L19388 Mixed"]` | `custody Unknown  ["site0@L19357 `**`Released`**`", "site1@L19388 `**`Released`**`"]` |
| `L19434` | `["site0@L19357 Mixed", …]` | unchanged — `Mixed` |
| `L19489` | `["site0@L19357 Mixed", …]` | unchanged — `Mixed` |
| `L19505` | `["site0@L19357 Mixed", …]` | unchanged — `Mixed` |
| `L19544` | `["site0@L19357 Mixed", …]` | unchanged — `Mixed`; the leak leaf stands |

**Every exit reads `custody Unknown` in both columns** — the direct measurement
of §1.5's second property, and the reason no summary and no caller can move.

The probe is read-only, publishes no verdict and writes no audit artifact. It
took a one-token pattern change (`Unknown { description, .. }`) because the
variant gained a field; that is the only file outside `crates/qcore-c-gateway`
and this report that this card edits.

### 5.3 Hygiene

Memory was checked before each audit and each probe (18–20 GB available
throughout; the audit's own RSS stays under 100 MB) and no run was killed. The
machine carried **concurrent workspace test runs from parallel cards** for most
of the session, which matters only for §6's constraint-16 row. Measurement output
lives in the worktree-private, gitignored `measure/`, never under `target/` and
never in the shared scratchpad. No `git stash` was used at any point; the base
gateway sources were staged with `git checkout <base> -- <paths>` and restored
with `git checkout HEAD -- <paths>`, both verified with `git status`.

---

## 6. Gate

| step | result |
|---|---|
| `git submodule update --init --recursive` | run **first**, before either audit — the `libevent` pin gate fires at the *end* of a run, and a first BEFORE attempt was discarded for exactly that reason (§5) |
| `cargo test --workspace` | **GATE_TESTS** |
| `cargo fmt --all -- --check` | **clean** |
| `cargo clippy --all-targets -- -D warnings` | **clean**, zero warnings |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **GATE_PROBE1** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **GATE_PROBE2** |
| `python3 scripts/check_lean_audits.py` | **GATE_LEAN** |
| `python3 -m unittest discover -s scripts/tests` | **GATE_PYTHON** |
| corpus audits | **both exit 0**, zero gateway failures in all six scopes, 185 units, 136 reporting units with **0** custody summary conflicts |

GATE_FLAKE

Provider packages were **built** for the audit route (`providers/package.py
build`, `qcore-schemas` 14.0.0, exit 0) and are otherwise untouched: constraint
18's rebuild-and-verify obligation is triggered by a manifest-template change or
a `qcore-schemas` major bump, and this card makes neither. No provider source,
manifest template or schema version is edited.

### 6.1 Pinned expectations moved, deliberately

Three workspace tests were re-pinned, each argued in place and none weakening a
gate. All three are the same shape: an `unknown_alias` that was an artefact of
the blanket poison leaves, and what replaces it is stronger or nothing.

1. **`legacy_c::tests::braced_declarations_retain_following_cleanup_without_certifying_their_effects`**
   — was: a following `q_free(p); q_free(p);` reports one `UnknownAlias` and the
   function is `Unknown`. Now: it reports **`DoubleRelease`** and the function is
   **`Unsafe`**. Strictly more accusing. None of the four braced declarations
   mentions `p` and `p`'s address is never taken, so none can write that slot;
   the first release discharges and the second is reported as what it is. The
   `braced` refusal and a `!= Safe` assertion are re-asserted beside it, so the
   test's stated intent — "unsupported effects must never certify subsequent
   cleanup" — is checked more tightly than before.
2. **`c03_neutral_frame_recursion_probe::undeclared_neutral_recursion_leaves_the_component_and_its_callers_unknown`**
   — the caller no longer reports `unknown_alias` for a pointer an uncontracted
   call received **by value**. **Every verdict in that probe is unchanged**: the
   entry, `depth` and `subject` are all still `Unknown`, and `subject` still
   carries `call to uncontracted function \`depth\``.
3. **`publication_custody::a_publishing_helper_no_longer_launders_its_callers_release`**
   — the collateral `unknown_alias:h` leaves the diagnostic vector. The
   uncontracted-call refusal and **both** `Unknown` verdicts stay, so the helper
   still does not launder its caller's release and **constraint 10 is
   undisturbed**. `publish(h);` passes `h` by value and `w` never takes that
   slot's address.

No other pinned expectation anywhere in the workspace moved.

---

## 7. Lean

`proof/QCore/RefusedWrites.lean`, registered in `proof/AuditRefusedWrites.lean`
and imported by `proof/QCore.lean`. Eighteen theorems, in five families:

* **Nothing was replaced.** `unbounded_is_the_blanket_poison` and
  `an_empty_sparing_list_is_also_the_blanket_poison`: the pre-repair rule is the
  degenerate case of the new one, on **every** state. Every refusal this card
  does not positively admit behaves exactly as it did.
* **The three premises.** `spared_places_are_unwritable` is the conjunction;
  `a_selection_place_is_never_spared`, `an_indirect_place_is_never_spared`,
  `a_written_name_is_never_spared`, `a_static_local_is_never_spared`,
  `a_file_scope_object_is_never_spared`, `an_address_taken_name_is_never_spared`
  and `an_unrecognised_name_is_never_spared` each state one way a place stays
  poisoned whatever the other premises say. The last is absence of evidence: a
  name the reader never recognised is never spared.
* **Monotonicity.** `the_repair_only_removes_poison` — every binding the
  repaired rule poisons, the blanket rule poisoned too. `a_weaker_frame_spares_less`
  — recognising fewer names can only poison more, which is what makes every
  fail-closed default (an undelimitable body yields the empty frame) safe by
  construction rather than by inspection.
* **The refusal itself is untouched.** `a_refusal_never_certifies` and
  `the_poison_moves_no_resource_state` are §1.5's two properties.
* **Non-vacuity**, on card AOM-DESTROY-SIBLINGS' own fixture:
  `the_acquisition_survives_the_identifier_loop` (`result` is spared by
  `result[index].id = read_id;`), `the_address_taken_aggregate_does_not` (`view`
  is not, because of `&view`), `the_written_member_is_still_poisoned`, and
  `the_fixture_state_spares_exactly_one_binding` at the state level.

Every theorem's axiom surface is `propext` (two also `Quot.sound`) or empty —
inside `AuditAll`'s allowed closure, placeholder-free, with no project axioms.

**Premises, not theorems**, stated in the module header: that the Rust readers
extract the declaration runs, the static-storage names, the file-scope objects,
the address-of operands and the write positions faithfully from preprocessed C
(the `QCore.PublicationCustody` / `QCore.BorrowedActual` / `QCore.BorrowLiveness`
precedent), and the no-undefined-behaviour reading of the C object model the
frame premise rests on.

---

## 8. Named follow-ons

* **`AOM-POISON-CONDITION-FRAME`** — the measured boundary, and the reason the
  flagship function repairs 3 of 12 sites rather than 12 of 12. A refused
  **condition** (`return`, `if`, `while`, `for`, `do`, `switch`, `case`) keeps
  the full poison, because its CFG node is built from `condition_supported` /
  `null_condition` / `distinct_condition` rather than from the tokens, which the
  instruction never holds. `flb_input_chunk_get_direct_routes` L19402 is the
  exact witness: it re-poisons `result` and every one of the nine later call
  sites refuses as before. Extending the bound means changing what
  `Statement::If`/`While`/`For`/`DoWhile`/`Switch`/`Case`/`Return` carry from the
  parser to the builder; the write-set rule itself would be unchanged
  (`may_write_names` on the condition's own tokens). Its payoff must be
  **predicted and measured**, not assumed from this card's 77.
* **`AOM-POISON-NOOP-INCREMENT`** — §4.2's one unmasked defect. `pass++` lowers
  to `LegacyInstruction::NoOp`, so a `Clear`-bound scalar keeps a stale
  definitely-null fact for ever; the blanket poison used to overwrite it, and
  with the binding spared the fact survives and makes a branch `Infeasible`. This
  is constraint 5a's already-recorded hazard ("expression mutations need modeled
  writes rather than a `NoOp`"), unmasked rather than introduced. The measured
  consequence is exactly **one** `unsupported` leaf in `librdkafka/rdbuf.c`,
  `do_unittest_write_read_payload_correctness` L4156, in a function that stays
  `Unknown` with 32 refusals, and **zero** verdict moves. The repair is to give
  an increment a modelled write rather than a `NoOp` — which `may_write_names`
  already identifies — and it is in the *accusing* direction, so it is its own
  card with its own measurement.
* **`AOM-POISON-NO-UB`** — the frame premise is the ordinary C object model
  under the no-undefined-behaviour reading: a write past the end of one object
  onto an adjacent local is undefined, not a modelled effect, so it cannot reach
  a slot whose address is never taken. This is the same premise the projection
  already makes when it models `flb_free(p)` as releasing exactly `p`'s site, and
  it is recorded here because this card is the first to make a *sparing*
  decision depend on it.
* **`AOM-POISON-DECLARATION-READER`** — premise 2 rests on
  `publication::body_declarations`, whose own documentation says it
  over-approximates the declared set in the direction that *reduces* refusal
  (`a * b;` contributes `b`). That direction is wrong for this card, so the
  file-scope exclusion is applied **unconditionally** here rather than with the
  shadowing exemption `publication::destinations` uses. A name that is both
  misread as a declaration *and* missed by `file_scope_objects` (residual
  `CTWIN-PUBLISH-SCAN`) would be spared wrongly. Not observed; stated because it
  is the one double-miss the construction does not close.
* **`AOM-DESTROY-JOINED-PROVENANCE`** carried forward, **untouched**.
  `input_chunk_get` is byte-identical before and after, `Unknown` with 23
  diagnostics and both `L19847` leaves intact. That residual is about narrowing a
  *join* of two provenances; this card's subject was a binding lost to an
  unrelated refused statement on a straight-line path.
* **`AOM-DESTROY-CORRUPTED-UNPAID-PREMISE`** carried forward, untouched: its
  blocker is an indirect actual at L19823, which this card does not repair.
* **`AOM-SIBLINGS-BINDING-POISON` closes**, partly by repair and partly by
  succession: the statement half is repaired and measured; the condition half
  becomes `AOM-POISON-CONDITION-FRAME`.

---

## 9. What was touched

| file | what |
|---|---|
| `crates/qcore-c-gateway/src/legacy_c.rs` | `RefusedWrites`, `may_write_names`, `frame_protected_locals`, `is_plain_name_place`, `Parser::refused_writes`, the `writes` field on `LegacyInstruction::Unknown` at all 35 construction sites, the sixth element of `typed_access_facts`, and the bounded poison in `transfer` |
| `crates/qcore-c-gateway/src/legacy_neutral.rs` | one pattern gains `..` |
| `crates/qcore-c-gateway/tests/unit/legacy_c.rs` | three new tests; one assertion re-pinned (§6.1) |
| `crates/qcore-c-gateway/tests/c03_neutral_frame_recursion_probe.rs`, `.../publication_custody.rs` | one assertion each, re-pinned (§6.1) |
| `compiler/examples/alias_aggregate_probe.rs` | one pattern gains `..` |
| `proof/QCore/RefusedWrites.lean`, `proof/AuditRefusedWrites.lean`, `proof/QCore.lean` | the Lean parity (§7) |
| `docs/reports/aom-binding-poison/` | this report, the pre-registered prediction, the tooling and the evidence |

No `.qcore` module, manifest, contract pin, frozen archive, provider, source
language pass or frontend file is touched.

---

## 10. For the integrator

Nothing in `AGENTS.md`, `DEVELOPMENT.md`, `README.md` or
`docs/reviews/r0_3/acceptance_matrix.json` is edited by this card, and no frozen
archive is touched. The facts to reconcile, all measured and all in this report:

1. **The leak pin does NOT move.** 266 before, 266 after; not one leak leaf added
   or removed anywhere in the 185-unit corpus. Constraint 7's number and its
   four-move history are unchanged, and its text needs no edit — except that
   `CONSTRUCTOR-RESIDUALS-LEAK-271`'s **decision** is now a discharge rather than
   a withdrawal (§ Headline, §5.2), which is a strictly better answer to the same
   question and does not change any count.
2. **Zero promotions of any kind.** Units 2 `Safe` / 42 provisional `Unsafe` /
   137 `Unknown` / 4 data-only, unchanged, zero status moves, zero `reason_code`
   moves, zero newly surfaced provisional `Unsafe` units. Functions 150 / 84 /
   4,600, unchanged, **zero** status moves across 4,834 projected functions.
   Nothing became `Safe` anywhere, and §1.5 says why that is structural rather
   than lucky.
3. **`unknown_alias` 4,421 → 4,344; `unsupported` 49,781 → 49,773.** Both are
   headline numbers in the ledger. All 87 moved leaves are itemised in §4; the
   eight `unsupported` are itemised individually in §4.2, seven of them repairs
   of a false `nullable resource … used by a non-null contract` refusal and one a
   recorded finding.
4. **Contract corpus unchanged**: 73 modules, 18 manifest-selected, 18 raw keys
   before and after, no normalisation collision on either side (constraint 22).
5. Baseline **not** re-pinned. `docs/reports/aom-destroy-siblings/artifacts/` is
   the authoritative baseline this card measured against and it stands unedited;
   `docs/reports/wave23-c-operations/` stands beneath it (constraint 21). This
   card's own AFTER artifacts are archived under
   `docs/reports/aom-binding-poison/artifacts/` for the next card to read.
6. **One new standing constraint is proposed** for the subsystem list, since it
   must survive refactors: *a refused statement invalidates only the bindings it
   could have written, and only where the reader positively recognised all three
   premises; every refusal that does not stand for one complete statement — every
   condition, every control-flow refusal — keeps the blanket poison, and the
   allow-list is built positively so an unrecognised place is always poisoned.
   The refusal itself is never weakened: the `Unsupported` diagnostic, the
   `unmodeled` path flag and the `Unknown` function status are untouched, which
   is what makes it impossible for this rule to promote anything.*
7. **Three workspace assertions re-pinned**, §6.1, each argued and none weakening
   a gate.
8. **Six follow-ons open** (§8), of which `AOM-POISON-CONDITION-FRAME` is the
   direct successor and carries the measured witness line.
