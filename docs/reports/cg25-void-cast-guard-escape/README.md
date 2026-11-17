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
