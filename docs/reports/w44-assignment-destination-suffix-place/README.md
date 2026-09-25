# W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE: a destination is the whole left-hand side

Card W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE, 2026-09-25, a **soundness repair**.
Its base is `b4430a14` (local `main`, "Record the chain-access integration and
archive the combined baseline").

`direct_assignment_destination` (`crates/qcore-c-gateway/src/legacy_c.rs`) named
a left-hand side that is not headed by `*` by the **first suffix that spells a
place**. So `recs[i].name = x_dup(s); x_free(name);` bound the acquisition to
the local `name` and certified **Safe**. In the C, one object leaks and a
different one is freed. Card W44-INDIRECT-CALL-ARGUMENT-ACCESS found the hole,
pinned it on its branch (`worktree-agent-a833cb615fbcb8088`), and registered
it. This card measures the hole's reach, repairs it with Lean parity, and
checks the other readers for the same flaw.

The forecast was frozen before the repair was committed
([PREDICTION.md](PREDICTION.md), commit `9a098610`). The commits are:

* census instrumentation: `f326255b` and `f4fc5ef0`;
* the repair: `289fdfb2`;
* tests: `f19f1c89` and `92b3d9e2`.

## Headline

* **Reach** ([§1](#1-reach)).
  * Every statement or `for` clause whose left-hand side has a place-spelling
    suffix that starts after index 0: **10,979 distinct** records.
  * **7,853** are declarations (`struct T * p = …`), which are named exactly as
    before.
  * **3,126** name a sub-object. **1,450** of those were **admitted** under a
    misnamed destination: 1,148 `Alias`, 295 `Clear`, 6 `ForgetScalar` and 1
    `ContractCall`.
  * The misnamed root was undeclared in 1,416 cases, a local in 24 and a formal
    in 10.
* **Safe functions whose certification relied on it: two.** Both are in `lz4.c`:
  **`LZ4_write16`** and **`LZ4_write32`**. Each is the single statement
  `((LZ4_unalign *) memPtr)->u16 = value` (or `u32`), which was lowered as a
  binding of the undeclared name `u16` instead of a store through `memPtr`.
  Both move **Safe → Unknown**.
* **The rule** ([§2](#2-the-rule)). A destination is named only when the
  suffix **is** the whole left-hand side, or when a *declarator prefix*
  (identifiers, `*` and `,` only) precedes it. Every other left-hand side with
  such a suffix is a store into a sub-object, and the lexical indirect-access
  gate refuses it. The typed routes are then consulted as for any `a.b = …`;
  they admit none of the 1,450. No new `LegacyInstruction` variant is added.
* **Lean.** `QCore.AssignmentDestinationSuffix` has 17 theorems, audited by
  `AuditAssignmentDestinationSuffix.lean`, with no placeholders and no project
  axioms. They cover:
  * refinement: the repaired reader names nothing new;
  * preservation: an exact place or a declaration keeps its name;
  * that a named destination follows no selection;
  * **the repair never promotes**;
  * non-vacuity on the card's fixture.
* **Arbiter** ([§4](#4-arbiter)).
  * BEFORE reproduces the combined baseline's diagnostics byte for byte, and
    both census runs reproduce their trees' five artifacts.
  * AFTER: **every delta is attributed** (`attribution_complete: true`).
  * Functions: **292 / 84 / 4,458** (Safe / provisional Unsafe / Unknown).
    Units are unchanged.
  * **Leak 271 → 269.** `unpack_basic_type`'s two false leaks are withdrawn
    with the misnamed acquisition.
  * `unknown_alias` 4,407 → 3,291: 1,116 reports of misnamed aliases are
    withdrawn.
  * `unsupported` 45,774 → 45,703.
* **Other readers** ([§3](#3-other-readers)).
  * The descriptor guard inherited the flaw: an index read under a member
    store went unseen. It is fixed by the same repair and pinned.
  * The aggregate store's premise-3 reader has a sibling flaw. It is pinned and
    registered.
  * The protected-formal alias closure keeps the old reader deliberately, since
    it only over-approximates protection. It is pinned.
  * The traversal, collection, typed-access, out-alias, publication and
    refused-write readers read the whole left-hand side and are not affected.

| Measure | BEFORE (combined baseline, `b4430a14`) | AFTER | Delta |
|---|---:|---:|---:|
| Functions Safe / provisional Unsafe / Unknown | 294 / 85 / 4,455 | **292 / 84 / 4,458** | −2 / −1 / +3 |
| Units Safe / provisional Unsafe / Unknown / data-only | 3 / 43 / 135 / 4 | 3 / 43 / 135 / 4 | 0 |
| `leak` | 271 | **269** | −2 |
| `unknown_alias` | 4,407 | **3,291** | −1,116 |
| `unsupported` | 45,774 | **45,703** | −71 |
| `summary_conflict` / `double_release` / `invalid_release` / `use_after_release` | 1 / 0 / 0 / 0 | 1 / 0 / 0 / 0 | 0 |
| indirect memory access: leaves / Unknown functions / sole | 17,236 / 3,278 / 408 | **17,423 / 3,285 / 413** | +187 / +7 / +5 |
| unresolved resource identity: leaves / functions / sole | 4,180 / 1,256 / 3 | 3,084 / 1,220 / 3 | −1,096 / −36 / 0 |
| call to uncontracted function: leaves / functions / sole | 9,547 / 2,591 / 407 | 9,355 / 2,587 / 407 | −192 / −4 / 0 |

The family rows are [scripts/blocker_census.py](scripts/blocker_census.py) over
the two runs ([blocker-census.txt](blocker-census.txt) is AFTER's). Over the
baseline the script reproduces the published
`w44-indirect-chain-access/blocker-census.txt` row.

## 1. Reach

### 1.1 Instrumentation (`f326255b`, `f4fc5ef0`)

The `indirect-census` feature gains a stream, `QCORE_DESTINATION_SUFFIX_CENSUS`
(`crates/qcore-c-gateway/src/destination_suffix_census.rs`). It records every
statement or `for` clause whose left-hand side has these properties:

* the left-hand side is the tokens before the first `=`, exactly the slice the
  classifier hands the reader;
* it is not headed by `*`;
* its first place-spelling suffix (`first_suffix_place`) starts after index 0.

Each record carries:

* the prefix, and the answer of `is_declarator_prefix`, the predicate the rule
  uses;
* the classifier's final outcome;
* for a non-declarator prefix, the named root's kind (formal, local, file-scope
  or undeclared), its whole-name mentions on later lines, and whether a later
  line passes it as a whole call argument;
* the answers of every route that can still admit the statement once it is
  unnamed: the scalar-field no-op and W42's member store before the gate, and
  the typed scalar, chain and stepped routes after it.

`f326255b` also committed the two readers the rule uses, which no rule called.
`direct_assignment_destination` read its suffix through `first_suffix_place`,
with unchanged behaviour. The second census commit added the pre-gate route
answers after the first run showed they were needed.

Both census runs reproduce all five artifacts of the same-checkout BEFORE run
byte for byte ([census-identity.json](census-identity.json); the BEFORE
record is [before-identity.json](before-identity.json)). The streams are not
committed. Their SHA-256 prefixes are `6e9c7356…` (base) and `732315c2…` (AFTER
tree).

### 1.2 Classes

The full output is [census/reach-census.txt](census/reach-census.txt), with
details in [census/reach-census.json.gz](census/reach-census.json.gz).
Records repeat because W4 rounds analyse bodies more than once. Of **38,183**
raw records, **10,979** are distinct. Six raw records (two distinct) do not
join, because one unit sequence matches four `cmt_encode_*.c` units; all six
are declarator records.

| Class | Distinct | What it is |
|---|---:|---|
| declarator | 7,853 | `size_t n = …`, `struct T * p = …`, `rd_segment_t * seg , * next = …`. No prefix carries a keyword (`return`, `else`, `case`). |
| sub-object | 3,126 | the last prefix token is `->` (2,389), `(` (427), `.` (204), `,` (58) or other (48) |

In 683 sub-object records the first `=` belongs to `==`. That slice is not a
left-hand side (`f(a, i == 0)`); see `W44-FIRST-EQUAL-IS-A-COMPARISON`.

### 1.3 The misnamed stores

The BEFORE classifier admitted **1,450** sub-object records:

| Lowering | Records |
|---|---:|
| `Alias` | 1,148 |
| `Clear` | 295 |
| `ForgetScalar` | 6 |
| `ContractCall` | 1 |

By base shape:

* **1,353 have a parenthesised base.** They come from macro-expanded C:
  `(rkbuf)->rkbuf_err = E`, `((LZ4_unalign *) p)->u16 = v`,
  `(&s)->str = NULL` and `(*map)->opts = calloc(…)`.
* **97 have a subscripted aggregate**, for example `result[index].label = NULL`
  and `opt[pos].mlen = 1`.

By the named root, the name the old lowering bound:

| Named root | Records | Later use |
|---|---:|---|
| undeclared | 1,416 | none later |
| local of this body | 24 | 10 mentioned later, 2 passed later as a whole call argument |
| formal | 10 | 1 passed later as an argument |

All 34 local and formal cases are listed in
[census/misnamed-real-names.txt](census/misnamed-real-names.txt). They include:

* `md->topics[i].topic = NULL`, which cleared the local `topic` that
  `rd_kafka_parse_Metadata0` later passes to a call;
* `opt[pos].mlen = 1`, which wrote the local `mlen` in
  `LZ4HC_compress_optimal`;
* `mcluster->fds[n].fd = fd`, which aliased the formal `fd` to itself.

The 1,450 records lie in **173 functions**:

* 168 Unknown;
* 3 provisional Unsafe: `unpack_basic_type`, `rd_kafka_handle_OffsetFetch`
  and `flb_input_chunk_get_direct_routes`;
* **2 Safe**: `LZ4_write16` and `LZ4_write32`.

**Every Safe function whose certification relied on the misnaming is one of
those two.** For each of the others, the verdict at base was already not Safe.

## 2. The rule

### 2.1 Reader (`legacy_c.rs`)

* `first_suffix_place(tokens)` is the old reader. It returns the first index
  whose suffix `canonical_place` accepts, with the place.
* `is_declarator_prefix(prefix)` is a nonempty run of identifiers, `*` and `,`.
  It is positive and fail-closed: `.`, `-` `>`, `[` `]`, `(` `)` and every
  other token are outside the set. A comma operand such as `x , y = 1` is named
  `y`, which is the object the comma expression assigns.
* `direct_assignment_destination` keeps the `* NAME` out-slot case unchanged.
  Otherwise it names the suffix only when it starts at 0 or
  `is_declarator_prefix` holds for what precedes it.
* `names_a_sub_object(tokens)` holds when the left-hand side is not headed by
  `*`, some suffix spells a place, and the reader names nothing.

### 2.2 Classifier

The lexical indirect-access gate in `Parser::classify_simple_base` refuses an
assignment whose named destination contains `.`, `[` or `->` (unless O04-B08's
aggregate store admitted it). It now also refuses one that
`names_a_sub_object`. The indirect census mirrors the arm.

Nothing else in the classifier changes, and three consequences follow:

* **Unchanged lowering.** A statement whose left-hand side is a place or a
  declaration keeps its exact lowering. This is
  `QCore.AssignmentDestinationSuffix.an_exact_place_keeps_its_name` and
  `a_declaration_keeps_its_name`, and the AFTER census confirms it: no leaf
  changed at a declarator-only line.
* **Unnamed stores reach no route.** The routes that read the destination come
  before the gate: `OpaqueValue` for a fixed query, `OpaqueValue` for an opaque
  field read, the aggregate-store equality and `ContractCall`'s destination. A
  sub-object store now reaches none of them unnamed.
* **Typed routes after the refusal.** The gate's refusal is in
  `TYPED_SCALAR_CONSULTED`, so the typed scalar, chain and stepped routes are
  consulted after it. The census measured that they admit none of the 1,450,
  and the AFTER census shows **0** admitted sub-object records.

### 2.3 Composition with wave-43/44 machinery

* **Descriptor custody.** The repair adds no variant, so
  `admits_descriptor_read` needs no decision. The refusal is `Unknown`, which
  the guard admits as it admits any refusal. `statement_reads_descriptor` now
  sees an index read under a member store; see §3.
* **`Derivation::Derived`, the `B[X++]` forget rule and the typed routes** are
  untouched. No lowering they produce changes.
* **Refused-write frame.** A refused sub-object store poisons what
  `may_write_names` reads as written: the misnamed last member name (`name =`)
  and a `.`-selected object. That is the over-approximating direction. It is
  why the card's fixture reports `unknown_alias:name` at `x_free(name)`.

### 2.4 Lean (`proof/QCore/AssignmentDestinationSuffix.lean`)

The module models tokens as an abstract alphabet: identifiers, digits, `*`,
`,`, `.`, `-`, `>`, brackets and parentheses. `isPlace` is `canonical_place`'s
shape, and `firstSuffix` is `first_suffix_place`. The theorems:

* **Refinement:** `the_repair_names_nothing_new`.
* **Preservation:**
  * `an_exact_place_keeps_its_name`;
  * `a_declaration_keeps_its_name`;
  * `the_readers_differ_only_on_sub_objects`.
* **Meaning:** `a_named_destination_follows_no_selection`. A named destination
  is a place, and the prefix is empty or a declarator prefix with no `.`, `>`,
  `]` or `)`.
* **Verdict direction:** `the_repair_never_promotes`. The status is modelled
  abstractly, with only the premise that a body carrying a refusal is not
  `safe`; that is the fact `QCore.RefusedWrites.a_refusal_never_certifies`
  records for `transfer`. Under it, a body the repaired classifier certifies
  was lowered exactly as before and was certified before.
* **Non-vacuity:**
  * the fixture `recs [ i ] . name` is named `name` by the old reader and
    nothing by the new one, and is refused;
  * `( * p ) - > f` is refused the same way;
  * a declaration and a whole member place keep their names;
  * `the_old_name_releases_the_wrong_object` is a two-location store model in
    which the old binding "releases" the acquired object while `recs[i].name`
    still holds it.

These are premises, not theorems:

* that the Rust readers transcribe the definitions;
* that `canonical_place`'s whole-slice parenthesis stripping only changes the
  shape test at an index it already tests.

## 3. Other readers

| Reader | Finding | Disposition |
|---|---|---|
| `legacy_descriptor_custody::statement_reads_descriptor` | It read a named destination as a plain-name write when the name is the last token. So `conns[fd].flags = 0` read `flags` as written and missed the **index read of the descriptor `fd`**. At base the fixture certified **Safe** (measured). | Fixed by the repair, because the reader now names nothing there. Pinned by the unit test `an_index_read_under_a_member_store_is_a_read` and the integration test `a_descriptor_read_as_an_index_of_a_member_store_is_seen` (now Unknown). |
| `protected_resource_formals` (alias closure) | A flow-insensitive candidate inventory in which a larger set only protects more. The suffix reader relates `name` and `src` in `recs[i].name = src`, which over-approximates. | **Kept on the old reader, deliberately**: `* NAME` goes through `direct_assignment_destination`, and every other left-hand side through `first_suffix_place`. No name loses protection. Pinned by `the_protected_alias_closure_keeps_the_suffix_reader`. |
| `aggregate_store::store_bases`, premise 3 (`acquired_bindings` and the assignment count) | It reads the name before `=` even after `.`, `->` or `]`. So `hs[i].rec = x_record()` counts as the acquisition of a local `rec`. With the misnamed `ContractCall` this certified `hs[i].rec = x_record(); rec->count = 1; x_record_free(rec);` as **Safe** (measured). | The classifier now refuses the store, so `rec` is never bound and the flow-sensitive owner check fails closed. Pinned: `a_misnamed_binding_no_longer_feeds_the_aggregate_store` (Unknown). The syntactic over-approximation remains and is registered as `W44-AGGREGATE-PREMISE3-MEMBER-NAME`. |
| `DescriptorNames::read` (`plain_assignments`) | Skips a name after `.` or `->`, and a `]`-closed destination is not an identifier. The set only ever refuses, so over-approximation is safe. | Not affected. |
| `index_traversal::pointer_step`, `typed_field_access::typed_scalar_statement`, `indexed_collection::store_statement` | Each reads the whole left-hand side: a name with typed specifiers, a typed access, or an exact `B[I]`. | Not affected. |
| `legacy_out_alias`, `publication::read_indirect_store` / `simple_assignment` | Out-alias requires a declaration `T * a = b` whose type parses. Publication reads the *root* of the left-hand side. | Not affected. |
| `may_write_names` / `refused_writes` | Reads a name before `=` as written, including a misnamed member name. That over-approximates the poison, which is sound. | Not affected. |
| `legacy_call_argument_access::destination_is_exact` (on the call-argument branch, not on this base) | Its own guard against the same naming. | Redundant after this repair, and harmless. See §6. |

New tests:

* `crates/qcore-c-gateway/tests/assignment_destination_suffix.rs` has 11
  integration tests. Six were `Safe` or `Unsafe` at base
  ([evidence/new-tests-at-base.txt](evidence/new-tests-at-base.txt)): the
  card's fixture, the same-named formal, the parenthesised dereference, the
  aggregate premise, the descriptor index, and the no-name store whose leak
  accusation named nothing.
* Two unit tests in `tests/unit/legacy_c.rs` pin the reader table and the
  protected closure.
* One unit test in `legacy_descriptor_custody.rs`.
* Two census unit tests (feature `indirect-census`).

No existing test re-pinned.

## 4. Arbiter

The baseline is `docs/reports/w44-indirect-chain-access/integration/artifacts/`.
The runner is [scripts/run_audit.py](scripts/run_audit.py), with PIN
`/home/cosmo/Documents/GitHub/qcore-c/build/cycle19`,
`QCORE_CLANG=/usr/bin/clang` and the wave-20 ChunkIO package.

* **BEFORE** ([before-run.json](before-run.json)) reproduces the baseline's
  `diagnostics`, `chunk-contracts` and `source` byte for byte, and `contracts`
  after the checkout-root alias. `source-effects` differs only in the
  checkout-path digests the brief lists; the previous card's record shows the
  same split. The record is [before-identity.json](before-identity.json).
* **Census runs:** base [census-run.json](census-run.json) and AFTER tree
  [census-after-run.json](census-after-run.json). Each reproduces its tree's
  five artifacts byte for byte
  ([census-identity.json](census-identity.json),
  [census-after-identity.json](census-after-identity.json)).
* **AFTER** ([after-run.json](after-run.json)) is binary source `e6fc628d`.
  Later commits change tests and records only. It is archived as the new
  baseline under [artifacts/](artifacts/).

### 4.1 Attribution

The script is [scripts/attribute.py](scripts/attribute.py), with output in
[attribution-summary.json](attribution-summary.json) and
[attribution.json.gz](attribution.json.gz). The result is
`attribution_complete: true`, with 0 unattributed.

| Delta | Leaves | What it is |
|---|---:|---|
| `gate-added` unsupported | +186 | the gate's refusal at a refused or exchanged line that had none |
| `exchanged` unsupported | −258 | a later refusal (`unsupported assignment expression`, `call to uncontracted function `__bswap_16``, …) replaced by the gate's at the same line |
| `alias-withdrawn` unknown_alias | −1,115 | the old `Alias` lowering's own report, for example `unresolved resource identity `RD_KAFKA_RESP_ERR__UNDERFLOW`` at `(rkbuf)->rkbuf_err = …` |
| `leak-withdrawn` leak | −2 | `unpack_basic_type` L8378 and L8418, both "acquired at cmt_decode_msgpack.c:1754", the `(*map)->opts = calloc(…)` bound to the name `opts` |
| `callee-summary` unsupported / unknown_alias | +5 / −4, −1 | same-unit callers of `LZ4_write16`/`LZ4_write32`, which lost their validated W4 summaries: `LZ4_compress_generic_validated` (2 lines), `LZ4_decompress_generic`, `LZ4_memcpy_using_offset_base` and `LZ4_writeLE16`, each now `call to uncontracted function` |
| downstream | 0 | |

The arithmetic balances: `unsupported` changes by +186 − 258 + 5 − 4 = −71,
`unknown_alias` by −1,115 − 1 = −1,116, and `leak` by −2.

### 4.2 Verdict changes

| Function | Unit | Status | Why |
|---|---|---|---|
| `LZ4_write16` | `lz4.c` | **Safe → Unknown** | `((LZ4_unalign *) memPtr)->u16 = value` was lowered as a binding of the undeclared name `u16`. The certification never checked the store through `memPtr`; the old Safe was unsound. |
| `LZ4_write32` | `lz4.c` | **Safe → Unknown** | the same, with `u32` |
| `unpack_basic_type` | `cmt_decode_msgpack.c` | **provisional Unsafe → Unknown** | its only acquisition, `(*map)->opts = calloc(…)`, bound the name `opts`, and its two leaks accused that name. The provisional Unsafe was never an upstream defect claim, and it is withdrawn. |

No unit moves:

* `lz4.c` was already Unknown;
* `cmt_decode_msgpack.c` keeps nine other provisional-Unsafe functions.

`rd_kafka_handle_OffsetFetch` and `flb_input_chunk_get_direct_routes` gain
refusals and keep their provisional Unsafe. Their leaks (L21228, L19544) do not
depend on a misnamed statement. Nothing moves toward Safe.

### 4.3 Prediction against outcome

| Frozen forecast | Measured | |
|---|---|---|
| functions 292 / 84 / 4,458, with those three movers | the same three | hit |
| units unchanged | unchanged | hit |
| leak 271 → 269 | 269 | hit |
| `unknown_alias` down about 1,101 | −1,116 (1,115 withdrawn aliases, plus 1 callee-summary line) | hit within the stated approximation. 14 more alias reports sat at refused lines than the single-token-source forecast counted. |
| `unsupported` ≤ +188 / ≤ −428 | +186 / −258 gate and exchange, and +5 / −4 callee-summary | hit on the census deltas |
| 0 violation candidates | 0 | hit |
| every changed leaf in one of the 634 functions with a sub-object record | 9 leaves in 4 same-unit **callers** of the two moved functions are outside that set | **miss**: the forecast did not follow the loss of a W4 summary to its callers. The status of every caller was already Unknown. |
| declarator lines unchanged | unchanged | hit |

The frozen file is not edited; these corrections live here.

## 5. Validation

The runner is [scripts/run_gate.py](scripts/run_gate.py) (the chain card's
runner, copied), in this worktree (`agent-a663627263bbedb07`), with
`CARGO_INCREMENTAL=0` and `QCORE_CLANG=/usr/bin/clang`. Logs are in the private
`measure/gate/` directory. Every exit was captured directly from the process.

GATE_TABLE

## 6. Ledger and follow-ups

* **`W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE` is not registered on this base.**
  It is on the call-argument branch only, so this card retires nothing.
  Removing a never-registered name would be a no-op.
  * **Discharge, for the integrator to apply** when the call-argument card
    lands: the discharge condition is met by this card, since the reader names
    only a whole place or a declared object, sub-object stores refuse, and
    §4.2 lists the verdict changes.
  * **Re-pin, for the integrator to apply:** the call-argument branch's pin
    `the_pre_existing_suffix_destination_hole_is_pinned` (which expects `Safe`)
    becomes `Unknown` with `[GATE, "unknown_alias:name"]`, exactly this card's
    `a_member_store_no_longer_binds_the_same_named_local`.
  * **No action needed:** that branch's `destination_is_exact` is then
    redundant, and harmless.
* **Registered** ([followups.json](followups.json)): follow-ups go from
  **352 to 355**.
  * `W44-SUB-OBJECT-STORE-REFUSED`: the 1,450 now-refused sub-object stores in
    173 functions, by shape.
  * `W44-AGGREGATE-PREMISE3-MEMBER-NAME`: §3.
  * `W44-FIRST-EQUAL-IS-A-COMPARISON`: the 683 `==`-first statements.
* `corpus.cycle19_raw_units` now points at this report's artifacts. Its
  previous contents are in
  [previous-corpus-row.json](previous-corpus-row.json)
  ([scripts/update_ledger.py](scripts/update_ledger.py)).

## 7. For the integrator

* **Likely conflicts:**
  * `crates/qcore-c-gateway/src/legacy_c.rs`. The gate arm in
    `classify_simple_base` and its census mirror are shared with the
    call-argument and branch-condition cards. The reader functions are near
    `direct_assignment_destination`, the alias closure is in
    `protected_resource_formals`, and there is a one-line census hook in
    `census_traversal_statement`.
  * `crates/qcore-c-gateway/src/lib.rs` (one feature-gated module).
  * `crates/qcore-c-gateway/src/legacy_descriptor_custody.rs` (a test appended
    at the end).
  * `crates/qcore-c-gateway/tests/unit/legacy_c.rs` (tests appended).
  * `proof/QCore.lean` (one import).
  * `docs/reviews/r0_3/acceptance_matrix.json`, `docs/STATUS.md`,
    `docs/development/increments/wave-44.md` and `docs/reports/INDEX.md`.
* **Composition with W44-INDIRECT-CALL-ARGUMENT-ACCESS.** That card's 22
  first-run leaks came from `result[index].label = flb_malloc(…)` being named
  `label`. After this repair the classifier refuses such a statement at the
  gate before any probe can admit it, so the combined tree needs a composed
  arbiter run.
* **Proposed gateway amendment**, to sit beside 5g in `docs/agents/gateway.md`:

  > **An assignment's destination is its whole left-hand side.**
  > `direct_assignment_destination` names a place only when it is the whole
  > left-hand side or follows a declarator prefix (identifiers, `*`, `,`);
  > a left-hand side ending in a place-spelling suffix it does not name is a
  > store into a sub-object and is refused by the lexical gate
  > (`names_a_sub_object`). Do not reintroduce suffix naming: it bound
  > `recs[i].name`'s value to a local `name` and certified a body that leaks
  > one object and frees another. A reader that only over-approximates a
  > refusing inventory (the protected-formal closure) may keep the suffix
  > reader. `w44-assignment-destination-suffix-place`.
