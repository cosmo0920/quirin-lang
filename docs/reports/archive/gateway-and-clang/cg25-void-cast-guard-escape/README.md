# CG25-VOID-CAST-GUARD-ESCAPE

Priority soundness card. Discharges `CG25-VOID-CAST-GUARD-ESCAPE`, which
CURRENT-CONTEXT-NEW-TRIAGE (D3) registered. User decision, 2026-11-17,
verbatim: "Yes, launch it." It answered "Should I launch the priority card
for the `(void)`-cast soundness gap? It would first count the published Safe
rows the gap affects, then fix it fail-closed." Base: `36b2cf931`.

## 1. Shape census (base `36b2cf931`, before any change)

Gateway rule 11v (4)'s statement guard (`Parser::in_place_guard`) wraps
`classify_simple` only. It skips a first classification that is already a
refusal. Every route that runs after it, or outside it, was unguarded:

* the statement fallback table;
* the non-table statement routes;
* the condition readers;
* the return readers.

The probe harness is [census/cg25_probe.rs.txt](census/cg25_probe.rs.txt), an
integration test kept out of the tree. It analyses each body under the
fixture `k_*` regex surface and the libc thread-key declarations. Each shape
was run twice over a tracked `regex_t re`, one compiled with its status
tested (`r = k_regcomp(&re, p, 0); if (r) return 0;`):

* **pre**: the shape, then `k_regfree(&re)`;
* **post**: `k_regfree(&re)`, then the shape.

The inputs are [shapes.tsv](census/shapes.tsv) (127 regex rows, 9 of them returns, and 12
thread-key rows) and [shapes2.tsv](census/shapes2.tsv) (36 regex rows). The
results are [probe-base.tsv](census/probe-base.tsv) and
[probe-base2.tsv](census/probe-base2.tsv). The answering routes were recorded
with the gateway's `route-census` feature, in
[routes-base.summary.txt](census/routes-base.summary.txt). Scratch logs are in
`private/cg25-census/` (`probe-base*.log`, `routes-base.*`).

An **escape** is a shape that names the tracked object outside a whole
`&re` actual at a `regex` position and is still certified `Safe`.

### Per kind

| Kind | Escapes? | Why |
|---|---|---|
| `regex` | **yes** | the 25 shape runs below, over 14 shapes |
| `thread_key`, automatic | no | every creation into an automatic key is refused (`THREAD_KEY_STORAGE_REFUSAL`, D3). The 6 shapes (`(void) pthread_getspecific(key)`, plain, assigned, comma, statement expression, `(void) pthread_setspecific`) are all Unknown, so no key is tracked. |
| `thread_key`, static | no tracked object | a static key binds no frame obligation (P-THREAD-KEY-STATIC). Its getters are unkinded and stay Safe, which is correct, not an escape. |
| any other kind | none exists | `ResourceKind::ALL` has two storage-carried kinds: `regex` and `thread_key` |

### `regex` escapes at base (certified Safe)

| Shape | Runs Safe | Route that admitted it |
|---|---|---|
| `(void) k_take(re.used);` | pre, post | statement fallback 10s (elaborated statement, the cast route) |
| `(void) k_take((unsigned long) re.used);` | pre, post | 10s |
| `(void) re.used;` | pre, post | 10s |
| `(void) (n = re.used);` | pre, post | 10s |
| `(void) (re.used = 0);` (a **write** of tracked storage) | pre, post | 10s |
| `(void) (re.used ? 1 : 0);` | pre, post | 10z (connective) |
| `t = k_regexec(&re, s) + (int) re.used;` | pre | a non-table statement route; post is Unsafe |
| `if (re.used) …` | pre, post | the condition reader |
| `while (re.used) …`, `do … while (re.used);`, `switch (re.used) …` | pre, post | the condition readers |
| `if ((n = re.used)) …` | pre, post | the condition reader |
| `if (k_regexec(&re, s) == 0 && re.used) …` | pre | the condition reader; post is Unsafe |
| `return (int) re.used;` | post | the return reader |

The same escape holds on the real unit. `rdregex.c` was preprocessed with its
recorded compile command (`census/preprocess.py`) and analysed with the libc
`regex.qv`; see [probe-base-units.tsv](census/probe-base-units.tsv). Two
mutants of `rd_regex_match` were certified `own=Safe`:

* `regfree(&re); (void) re.re_nsub;`
* `regfree(&re); if (re.re_nsub) r = 2;`

A third, `regfree(&re); (void) regexec(&re, str, 0, 0, 0);`, refuses.

### Not escaping at base

These forms refuse at base: the guard, the formal-address gate, or "resource
call with a formal embedded in an unmodeled expression":

* the guard-covered statements: `n = re.used;`, the struct copy `other = re;`,
  `re.used = 0;`, `(void) re;`, `(void) &re;`, `(void) sizeof re;`,
  `n = re.used + 1;`, `t = !re.used;`, a subscript store, and the declaration
  forms;
* every `&re` at an unkinded position (`k_peek`, `k_wipe`, `k_free`). This
  holds with or without a cast, a comma operand or a statement expression,
  and in conditions and returns;
* `(void) k_regexec(&re, s);`, `(void) k_regfree(&re);`,
  `(void) k_regcomp(&re, p, 0);`, and the comma forms of a kinded call;
* reads through an indirect member: `(void) &re.used;`, `k_take(re.used);`,
  and `t = k_take(re.used);`.

Kinded uses are lowered with their place on every route that admits them:

* the plain call, the assignment and the declaration;
* `!= 1` and `(int)` (10y), and the statement expression (11g);
* the condition and the return.

Each one's post run is Unsafe (`use_after_close`), so each is a correct
admission.

`n = sizeof re;` and `n = sizeof(re);` are Safe through 10s and **correct**.
`sizeof`'s operand is not evaluated (C11 6.5.3.4p2), and a tracked object is
never an array, so it is never a VLA.

## 2. Published-row census

A row can rest on an escape only if its body tracks an in-place object:

* a `regex` object needs `regcomp(&NAME, …)` with `NAME` one automatic
  object;
* a thread key is never tracked, after D3.

Two read-only censuses looked for such bodies. The routes' own reports were
read with `census/inplace_rows.py`, which lists every row that depends on an
in-place callee ([inplace-rows.tsv](census/inplace-rows.tsv)). The built
sources were read for the initialiser call sites.

| Route, revision | Rows on an in-place callee | Rows tracking an object | Safe rows resting on an escape |
|---|---|---|---|
| corpus, `baseline:497bdeb90` | 4: `rd_regex_comp` U, `rd_regex_destroy` U, `rd_regex_exec` S, `rd_regex_match` S | `rd_regex_match` | **0** |
| system, `baseline:497bdeb90` | 0 | none | 0 |
| slice, `baseline:497bdeb90` | 0 | none | 0 |
| `current`, evidence report `89fa31d21` (Fluent Bit and Monkey) | the same 4 rows, the same statuses | `rd_regex_match` | **0** |
| corpus and `current`, main `36b2cf931` (unit reproduction of `rdregex.c` on both trees) | the same 4 rows: `rd_regex_match` own=Safe, `rd_regex_exec` own=Safe, the other two Unknown | `rd_regex_match` | **0** |

The only tracked object is `rd_regex_match`'s `re`. It has five mentions:

* the declarator `regex_t re;`;
* four whole `&re` actuals at `regex` positions: `regcomp`, `regerror`,
  `regexec` and `regfree`.

None is an escaping shape. `r = regexec(&re, str, 0, NULL, 0) != REG_NOMATCH;`
is lowered by fallback 10y, but that lowering carries the place: the probe's
post run of the same shape is Unsafe.

The other three rows track nothing:

* `rd_regex_exec` uses `&re->re`, reached through a formal: a plain borrowed
  use;
* `rd_regex_comp` and `rd_regex_destroy` name a member and refuse.

The source census of built units found two `regcomp` sites on an automatic
object:

* `rdregex.c:124` (`rd_regex_match`);
* `libmaxminddb-1.12.2/t/libtap/tap.c:339` (`like_at_loc`): it is in the
  compile databases, but it is a row of no route.

The other `regcomp(&reg, …)` sites are Onigmo samples and tests, which are
not built.

**Result: no published Safe rests on the escape, on any route or baseline.**
No published figure is defective. Nothing here is an upstream Fluent Bit
finding.

## 3. The repair (rule 11v (6))

D3 recorded two rejected fixes:

* widening (4)'s guard to every refusal that mentions a tracked object broke
  `rd_regex_match`'s fallback-lowered `r = regexec(&re, …) != 1;`;
* the CFG-level name check missed a by-value scalar actual.

The repair avoids both by reading the **tokens once for the whole body** and
the **finished CFG once**. No individual route is guarded
(`legacy_in_place_custody.rs`: `InPlaceNames::body_mentions`,
`verify_mentions`, and their hook at the end of the CFG build in
`legacy_c.rs`).

1. Every variable mention of a tracked object in the body's token stream
   (the stream the parser reads, after its rewrites) must be one of:
   * its declarator: the guard's own `declares_only` over the enclosing
     statement, not after a statement keyword (`return re;` reads `re`);
   * `sizeof NAME` or `sizeof ( NAME )`;
   * a **custody actual**: a whole `&NAME`, or `NAME` at a by-value
     finaliser or use of a by-value kind, of a direct call to a declared
     callee with a contract in force, at a position of the object's kind.
2. Every custody actual `(callee, position, NAME)` must be carried by at
   least as many `ContractCall` nodes of that callee, holding `NAME` at that
   position, as the body spells it.
3. Otherwise a refusal node is added **before the entry**. It lies on every
   path and replaces no lowering. Its text is either
   ``tracked in-place object `re` named outside its declarator, a `sizeof`
   operand or a declared in-place position (rule 11v (6))`` or
   ``in-place actual `re` of `f` is carried by no call node (rule 11v (6))``.

**Why this closes every census shape.** Every escaping shape in §1 names
`re` outside a custody actual: as a member operand, a cast, comma or
conditional operand, a condition or a return. So (1) refuses it, whichever of
10s, 10z, the non-table statement route or the condition or return reader
admitted it. A by-value scalar is seen, because the name is read off the
tokens and not off a node. The D3 shape `(void) pthread_getspecific(key)`
names `key` at an unkinded position, so it is stray too (the unit test
`a_by_value_getter_of_a_tracked_key_is_stray`). A kinded call that a route
dropped, or lowered without its place, fails (2): the census mutant
`(void) regexec(&re, …)` after `regfree` now carries that refusal beside its
old one. In a body that tracks nothing, the hook is the identity.

**What stays admitted.** Every kinded form keeps its place. Each is `Safe`
before the free and Unsafe (`use_after_close`) after it:

* the plain, assigned and declared call;
* `!= 1` and `(int)` (10y), and the statement expression (11g);
* the condition and the return.

`sizeof` stays admitted, and so does `rd_regex_match`, both the generic
fixture and the real unit at both trees. No correct admitted use was found
unsound.

**Re-probe at the code tip `67fdd5c28`**: all 186 rows of `shapes.tsv` and
`shapes2.tsv` plus the unit rows, in
[probe-tip.tsv](census/probe-tip.tsv), compared with the base results by
[compare.py](census/compare.py). The log is
`private/cg25-census/probe-fix2.log`.

* All 25 escaping runs moved `Safe -> Unknown`.
* No other row moved.
* The only text change is the new refusal, beside existing refusals.
* In the real unit, `rd_regex_match` and `rd_regex_exec` stay own=Safe on
  both trees, and the three mutants refuse.

An earlier probe (`probe-fix1`) ran with the hook just after `verify_cfg`
and gave the same moves. The hook now sits at the end of the build, so it
counts the finished CFG.

The thread-key creation refusal (`THREAD_KEY_STORAGE_REFUSAL`, D3) is kept.
Lifting it is a semantic change, outside this card: see the follow-up
`CG25-THREAD-KEY-AUTOMATIC-REVISIT`.

## 4. Fixtures and Lean

* `crates/qcore-c-gateway/tests/in_place_mention_guard.rs` (7 tests):
  * every census escape refused, pre and post;
  * guarded refusals stay refusals;
  * every kinded form admitted with its place;
  * the `rd_regex_match` shape `Safe` and its mutant refused;
  * `sizeof` admitted;
  * a body with no tracked object untouched;
  * the Lean `shapeAdmissions` rows bound to the gateway.
* `legacy_in_place_custody.rs` `mention_tests` (4 lib unit tests over the
  libc declarations), including the by-value thread-key getter.
* `tests/resource_kinds.rs`: three deliberate vector updates, each the new
  refusal beside an existing one, on bodies that were already Unknown:
  * the unkinded `k_wipe(&re, …)`, the struct copy and `k_free(&re)` gain the
    stray refusal;
  * the discarded and condition-consumed statuses gain the uncarried compile.
* Lean `QCore.InPlaceMentionGuard` (`AuditInPlaceMentionGuard.lean`, 12
  theorems, standard axioms only). Over **every route** (an arbitrary
  function from a body's mentions to a lowering):
  * `every_admitted_lowering_passed_the_guard`, with its two projections
    (`a_safe_body_mentions_only_admitted_forms`,
    `a_safe_body_carries_every_custody_actual`);
  * `a_stray_mention_is_refused_on_every_route`;
  * `an_uncarried_custody_actual_is_refused`;
  * `a_passing_body_is_cg25`;
  * the census vector theorem `shape_admissions_are_the_model`;
  * the witnesses: D3's getter, `rd_regex_match`'s mentions, and the mutant.

## 5. Follow-ups and measurement

* `CG25-THREAD-KEY-AUTOMATIC-REVISIT` (registered): D3 refuses every
  automatic key, because a tracked by-value key reached a data route. Rule
  11v (6) now refuses such a mention on every route, so the stated reason no
  longer holds. Lifting the refusal is a semantic change and needs a user
  decision. No Fluent Bit or Monkey row creates an automatic key.

The integrator's declared measurement (base `rev:36b2cf931`, as the brief
fixes), against [PREDICTION.md](PREDICTION.md):

```sh
target/release/quirin-audit --plugin-path target/release --output <dir> --timeout-ms 36000000 measure --inputs crates/quirin-audit-plugin-measure/inputs/live.json --inputs-base /media/Data/GitHub/quirin-lang --side base=rev:36b2cf931 --side tip=rev:67fdd5c28 --declared docs/reports/cg25-void-cast-guard-escape/PREDICTION.md
```

## 6. Gate

`python3 scripts/gate_scope.py --base 36b2cf931 --out private/gate-1` was
run at `ae28d74cd`, whose code equals `67fdd5c28`. Its record is
[gate.json](gate.json). Clang is recorded in
[gate/](gate/clang-version.txt). The scope was full (the diff reaches
`qcore-c-gateway`; Lean ran in full because `proof/QCore.lean` changed). The
provider gate was not selected.

| Step | Exit | Counts |
|---|---|---|
| submodules, fmt, clippy | 0, 0, 0 | |
| Rust suite | 0 | 651 units, 696 summaries; 9,624 passed, 0 failed, 21 ignored, 148 filtered |
| reevaluation probe, follow-up | 0, 0 | |
| Lean (`check_lean_audits.py`) | 0 | 443 audits |
| Python suite | 0 | 779 tests |

Logs: `private/gate-1/` (Rust per-unit logs in `private/gate-1/rust-suite/`).
The console log is outside the worktree, in the session scratchpad
(`gate-1.console.log`).

## Integrator measurement (2026-11-17)

Declared measurement, base `rev:36b2cf931` (main at the card's base), tip `rev:67fdd5c28` (the measured code tip; the card tip 5c824e443 adds only records under this directory), run from the main checkout (`private/measure/cg25-20261117T105504Z`): exit 0, all three routes complete, no unmatched rows, declared result **match**.

- corpus: 0 rows moved, 0 changed content (4,834 environment-only).
- system: 0 rows moved, 0 changed content (448 environment-only).
- slice: 0 rows moved, 0 changed content (404 environment-only).

No row moves in any direction, as predicted: the guard closes an escape no measured row reached. [Comparison](integration/native-comparison.json.gz) (gzip of `native-comparison.json`, sha256 `2e3febd2557589586219ea81e6c777fba439a44ce46532a256c8ccd94ade0213`).

Landed with CG25-THREAD-KEY-AUTOMATIC-REVISIT through integrate/cg25-thread-key. The landing gate record is [here](integration/gate.json) (full gate on a9c6dcf44; see its note for the two failures and why neither comes from these cards).
