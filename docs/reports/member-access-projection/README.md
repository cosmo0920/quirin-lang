# MEMBER-ACCESS-PROJECTION

Round 18, 2026-11-05. **USER REQUEST** (verbatim): "We're still needed to
handle tons of unsupported instructions on our corpus..."; offered three
candidate cards, the user chose: "Member access first". Base `96a3efd3b`
(integrate/r3).

## 1. Step 1: census of the indirect-access refusal

**Input.** The user's `quirin-audit current` run at Fluent Bit 046accf6 (main
945c0db1b binary): `build/cycle19/clang/046accf6…/fluent-bit-clang-source/reports/resource-diagnostics.json`
in the main checkout, read only. 9,996 leaves `indirect memory access is
outside the resource projection`, 2,346 functions, 162 units; 267 Unknown
functions have it as their only unsupported family.

**Method: the run's own analysed text and Clang's AST, not the source line.**
[`scripts/prep_units.py`](scripts/prep_units.py) rebuilds every unit's
analysed text with the gateway's own preprocessing command
(`legacy_preparation.rs` `preprocess_plain`: `clang -x c -std=gnu17 -E -P` plus
the unit's recorded preprocessor options) and accepts it only when its SHA-256
equals the report's `preprocessed_source_sha256`: **186 of 186 match**, so the
leaves' `line` values index the text the analyzer read.
[`scripts/classify.py`](scripts/classify.py) dumps each text's AST with the
unit's frontend flags (the gateway's `-ast-dump=json` transport) and walks it
with W46-ACCESS-PATH-CENSUS's `access_census.py`, imported, extended with the
store's right-hand side, the actual's callee and index, and a correction:
W46's `kind_of` strips qualifiers by whitespace token, so a pointer member of a
const-qualified record (`char *const`) read as a scalar; the census separates
every `*` first. Each leaf binds to its function's Clang constructs at its line
(9,989 of 9,996; 7 have no construct, counted under `no-evaluated-path`).
[`scripts/premise.py`](scripts/premise.py) gives each leaf **one premise**,
the first that holds in this order: no evaluated access path; a path form
outside every typed place reader (union, bit-field, volatile/atomic, statement
expression, cast in the path, pointer arithmetic, aggregate value, a call
through a member); a root that is not a parameter or automatic local
(file-scope, block `static`, call result); an indirect callee; a callee the
run reports uncontracted anywhere (the access is not what decides); then the
most custody-bearing path: a pointer stored into a member (by value class), a
pointer member read, a sub-object address, a custody-free store, a
custody-free read. "Macro-expanded" means the origin line spells no `->`,
`[`, `.` member selection or `*` dereference.

**What it does not have.** The analyzer's own route answers (which route
refused, the callee's mode at the actual's position) are in the
`indirect-census` build's streams (`QCORE_CALL_ARGUMENT_CENSUS`,
`QCORE_TYPED_PATH_PARITY`); the census run of that build
([`scripts/run_current_census.py`](scripts/run_current_census.py)) waited for
an audit slot behind the integrator's measurements for the whole card (§5).
Where a premise below needs the callee's mode, the probe fixtures of §2 stand
in, and "uncontracted" is the run's own list of uncontracted callees.

### 1.1 Leaves by premise

| premise | sites | macro-expanded | functions | sole-blocker functions | units |
|---|---:|---:|---:|---:|---:|
| `read:pointer-member` | 2395 | 1017 | 1010 | 52 | 135 |
| `call:uncontracted-callee` | 1888 | 944 | 770 | 31 | 97 |
| `address:sub-object` | 1778 | 726 | 685 | 35 | 93 |
| `form:union` | 749 | 105 | 275 | 10 | 40 |
| `store:pointer-name` | 518 | 12 | 325 | 21 | 98 |
| `store:pointer-call-result` | 424 | 21 | 259 | 19 | 84 |
| `store:pointer-other` | 364 | 6 | 238 | 10 | 71 |
| `store:pointer-identity-free` | 275 | 173 | 83 | 0 | 21 |
| `store:pointer-null` | 233 | 0 | 153 | 0 | 54 |
| `no-evaluated-path` | 227 | 55 | 197 | 5 | 58 |
| `read:custody-free` | 186 | 64 | 114 | 7 | 51 |
| `form:aggregate-value` | 178 | 53 | 115 | 5 | 36 |
| `form:indirect-call` | 158 | 17 | 119 | 5 | 48 |
| `store:custody-free` | 156 | 8 | 74 | 1 | 41 |
| `root:file-scope` | 152 | 55 | 84 | 2 | 19 |
| `root:call-result` | 109 | 94 | 75 | 1 | 38 |
| `form:statement-expression` | 99 | 47 | 69 | 0 | 24 |
| `form:cast` | 52 | 13 | 26 | 0 | 16 |
| `root:static-local` | 31 | 10 | 15 | 2 | 10 |
| `call:indirect-callee` | 18 | 1 | 14 | 4 | 12 |
| `form:other-root` | 5 | 2 | 4 | 0 | 3 |
| `form:pointer-field-step` | 1 | 0 | 1 | 0 | 1 |
| total | 9996 | 3423 | 2346 | 267 | 162 |

A sole-blocker function counts under a premise when every one of its
indirect-access leaves has that premise; 57 of the 267 have several.
Path-level counts by position are in [census/path-table.md](census/path-table.md);
every leaf with its typed paths in `census/premises.jsonl.gz`.

### 1.2 The single-premise sole-blocker functions by custody shape

[`scripts/sole_split.py`](scripts/sole_split.py), callee classes by name
(`*_free`, `*_destroy`, `free`, `rd_free`, `fclose`, …):

| premise | shape | functions |
|---|---|---:|
| (several premises) | | 57 |
| `call:uncontracted-callee` | | 31 |
| `address:sub-object` | at a non-releasing callee | 30 |
| `read:pointer-member` | at a non-releasing callee | 27 |
| `store:pointer-name` | a parameter's or local's pointer stored into a member | 21 |
| `store:pointer-call-result` | a call result stored into a member | 19 |
| `read:pointer-member` | an element `A[i]` of a formal array released | 12 |
| `read:pointer-member` | a member of a formal released, the formal kept | 11 |
| `form:union` | | 10 |
| `store:pointer-other` | | 10 |
| `read:custody-free` | | 7 |
| `form:indirect-call`, `no-evaluated-path`, `form:aggregate-value` | | 5 each |
| `address:sub-object` | handed to a releasing/clearing callee | 5 |
| `call:indirect-callee` | | 4 |
| `read:pointer-member` | elements and members released | 2 |
| `root:static-local`, `root:file-scope` | | 2 each |
| `root:call-result`, `store:custody-free` | | 1 each |

### 1.3 What each premise is, and who owns it

- **`no-evaluated-path` is a reading defect** (227 sites): the AST has no
  evaluated access at all. 141 are declarations without an initializer that
  the lexical gate reads as an access because their specifier is a typedef
  name (`rd_segment_t *seg, *tmp;` reads the second `*` as a dereference,
  `uint32_t op[32];` reaches the bracket check only for keyword specifiers) or
  their constant bound is not a literal (`char buf[1024 * 3];`): 126
  functions, 12 whose every indirect-access leaf is one, 2 sole-blocker
  (`crc32c_zeros_op`, `ut_testConflictingPreviousAssignments`). The keyword
  path admits exactly these declarations; the type is Clang's. **This is the
  first fix (§2).** The rest: 78 initialized multi-declarators
  (`char *p = NULL, *q = NULL;`, 2 sole), 40 call statements whose only
  lexical access is a cast address actual (`(char **) &labels`) and 25
  `cfl_list_entry` / curl macro statements: follow-ups (§4).
- **`read:pointer-member`, reading (b)** (2,395). In the sole-blocker set the
  shape decides: 25 functions release a member or an element of a
  **formal** parent and keep the parent (`cmt_opts_exit`,
  `metric_release_storage`, `rd_kafka_metadata_partition_clear`,
  `free_chunk_header`, the `*_destroy_array` helpers). Rule 5k admits a member
  release only when the body also releases the base (P4); a body that keeps
  the base has no summary mode that states what it released
  (W45-MEMBER-RELEASE-PREMISES, open). That is the semantic choice §3 stops
  on. The other 27 read a pointer member at a non-releasing callee; the probe
  fixtures show a formal root at a `Borrowed` position is already admitted
  (rule 10k(3) with a never-bound root), so their refusal is another premise
  of the statement (a second call, a cast or arithmetic around the read, a
  tracked root under rule 11d, a non-`Borrowed` mode), which the analyzer
  stream would name.
- **`address:sub-object`, reading (d)** (1,778). `&P->F` at a `Borrowed`
  position is admitted on a never-bound root (rule 10k(3), O04-B09B10) and on
  a body's own allocation (probe); the refused ones are addresses handed to
  list, lock and atomic helpers whose own mode or root premise refuses
  (`rd_list_destroy(&conf->interceptors.on_new)` releases the embedded list's
  elements; `rd_kafka_interceptor_method_add(&rk->…, …)` inserts into it).
- **`store:pointer-*`, reading (c)** (1,814). A pointer stored into a member
  is custody only where a form states it. 10v admits identity-free values;
  a parameter's pointer stored into its parent (`confval->name = name`) and a
  call result stored into a member (`map->unit = cfl_sds_create(unit)`) stay
  refused: W44-CALL-RESULT-INDIRECT-DESTINATION, W43-INDIRECT-POINTER-VALUE-STORE,
  W45-SUB-OBJECT-STORE-OWNED-VALUE. They are the second half of §3's choice:
  member custody through a formal parent.
- **`read:custody-free` / `store:custody-free`, reading (a)** (342). Each
  probe shape of a custody-free member access the routes read (a scalar
  member store on a body's own allocation, a scalar actual) is admitted; the
  residual leaves carry something else in the same statement: two calls
  (`strlen(a->p) + strlen(b->q)`, refused while each alone is admitted), a
  call-valued scalar store (`x->f = g()`, W44-CALL-RESULT-INDIRECT-DESTINATION),
  `*formal` scalars under the formal-mutation gate.
- **Not member access**: uncontracted callees (out of scope:
  BODY-SUMMARY-REUSE and the contract cards), unions (rule 10w residuals),
  aggregate values (CG14-POINTER-BEARING-COPIES-AND-INITIAL-ALIASES), calls
  through a member (rule 11p), file-scope/static/call-result roots (rules
  10t, 12j), statement expressions (rule 11g).
- **Macro expansions** (3,423 leaves) are not a premise of their own: the
  census reads the expansion's AST, and each lands in the rows above
  (`rd_kafka_dbg` and `rd_rkb_dbg` mostly as `address:sub-object`
  `&rk->rk_conf` and `read:pointer-member` log names; `cmt_errno()` as
  `root:call-result` `*__errno_location()`).
