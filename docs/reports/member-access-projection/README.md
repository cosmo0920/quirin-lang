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
an audit slot behind the integrator's measurements and the rust-suite queue
for an hour and a half and was withdrawn so the card gate could take the slot; it was
never started. Where a premise below needs the callee's mode or a route's
answer, the probe fixtures of §2 stand in, and "uncontracted" is the run's own
list of uncontracted callees. The script is ready for the next card (§4).

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
  first fix (§2).** The other 86: 40 call statements whose only lexical
  access is a cast address actual (`(char **) &labels`), and 46 initialized
  multi-declarators (`char *p = NULL, *q = NULL;`, 2 sole) and
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

## 2. Step 2: what the card reads, and what it does not

### 2.1 Probe fixtures (base `96a3efd3b`)

Before changing anything, the census shapes were run through the production
analysis (`analyze_unrestricted_c11`, libc contracts, one statement per line):

| shape | base answer |
|---|---|
| `strlen(st->name);`, `r = q_serve(rk->rk_rep, t);` (formal root, `Borrowed`) | Safe |
| `len = strlen(ctx->options.root_path);` | Safe |
| `len = strlen(a->p) + strlen(b->q) + 2;` | indirect-access refusal |
| `p->a = a; p->b = 2;` on a fresh `calloc` | Safe |
| `list_init(&c->list);` (formal root, and on a fresh `calloc`) | Safe |
| `free(opts->name); free(opts);` (`struct o5 *opts`) | Safe (rule 5k) |
| the same with `typedef char *sds` member or `o2_t *opts` base | indirect-access refusal |
| `free(opts->a); free(opts->b);` (parent kept) | indirect-access refusal |
| `struct mc *a, *b;`, `mb_t *a;`, `struct m6 members[19];`, `char *p = NULL;` | Safe |
| `m_t *a, *b;`, `m5_t members[19];`, `char *p = NULL, *q = NULL;` | indirect-access refusal |

So readings (a), (b) and (d) already hold where a route can type the root and
the position is custody-free; what the census's custody-free and
non-releasing rows still hit is another premise of the same statement, not a
missing member reading. Two refusals are reading defects in the brief's sense
(the AST has the facts and an existing route's premises hold): the
uninitialized typedef-specified declaration (fixed here) and the
typedef-spelled base or member at a `Released` position under rule 5k
(W45-MEMBER-RELEASE-UNTYPED-DECLARATIONS, left as the next fix: it frees no
sole-blocker function of this run, §1.2, and needs 5k's own Lean premise
model reopened).

### 2.2 Rule 38h: an uninitialized declaration from the Clang AST

[Rule 38h](../../agents/rules/38h.md). Where the lexical gate refuses a
statement with no `=` that its own declaration-prefix test accepts, the
statement's unique Clang node (line and exact tokens, occurrence as rule 10t)
must be one `DeclStmt` of `VarDecl`s only, none initialized, none with a
storage class or thread-local marker, every array extent as written and
desugared a decimal constant. Then the statement takes the keyword branch's
admission and the classifier's unchanged `NoOp`; otherwise the refusal stays
byte for byte, and no admitted statement is consulted.

- Admits: `rd_segment_t *seg, *tmp;`, `uint32_t odd[32];`,
  `rd_kafka_group_member_t members[2];`, `char buf[1024 * 3];`,
  `char varint_NameLen[RD_UVARINT_ENC_SIZEOF(int32_t)];`.
- Refuses (unchanged): an initialized declarator (`char *p = NULL, *q = NULL;`),
  a variably modified array, `static`/`extern`/`register`/thread-local
  objects, a declaration of a function, typedef or record, and every
  statement Clang does not show as one declaration.
- Code: `crates/qcore-c-gateway/src/typed_declaration.rs` (facts and
  decision), `legacy_typed_declaration.rs` (the hook, with a
  `typed-declaration` parity row), two calls in `legacy_classify.rs`.
- Lean: `QCore.TypedDeclarationReading` (`proof/AuditTypedDeclarationReading.lean`):
  `vectors_match` (132 decision vectors, the Rust table's, checked by
  `typed_declaration::tests::refusal_vectors_are_the_lean_vectors`),
  `an_old_admission_stays`, `no_ast_answer_keeps_the_refusal`,
  `a_non_declaration_keeps_its_answer`, `a_new_admission_needs_every_fact`,
  the four keeping theorems (initializer, variably modified, non-automatic,
  non-object) and the three `NoOp` lowering theorems. No axioms, no
  placeholders.
- Fixtures: `crates/qcore-c-gateway/tests/member_access_projection.rs`
  (admitted shapes; the initialized and VLA refusals; a double release and a
  loop re-acquisition after an admitted declaration stay non-Safe; the
  librdkafka `crc32c.c` bodies, where `crc32c_zeros_op` and, through its
  in-unit summary, `crc32c_zeros` are Safe), and the decision's unit tests
  over Clang's JSON shapes.

## 3. Stop: member custody through a formal parent (user choice)

The rest of the sole-blocker mass is custody, not typing (§1.2): 25 functions
release a member or element of a **formal** parent and keep the parent
(`cmt_opts_exit`, `metric_release_storage`, `free_chunk_header`,
`rd_kafka_metadata_partition_clear`, the twelve `*_destroy_array` helpers),
50 store a pointer that may carry an identity into a member
(`map->unit = cfl_sds_create(unit)`, `confval->name = name`), and 5 hand an
embedded list's address to a releasing helper (`rd_list_destroy(&conf->…)`).
Fail-closed does not settle whether such a body may be admitted: the caller
holds the parent, so whatever the body does to `P->F` is an effect on the
caller's object that no summary mode states (rule 5k admits a member release
only when the body also releases the parent, P4; W45-MEMBER-RELEASE-PREMISES).
This is the brief's own example ("whether a pointer member read through a
borrowed parent may be passed to a releasing callee").

Options:

1. **A member-effect summary (recommended).** A body's summary states, per
   formal, the members it releases (`P->F` released, nullable or not) and the
   members it stores a custody-carrying value into (`P->F` takes the value),
   checked against the body (each place used only at those positions, as rule
   5k's P1–P3 and rule 13a check slot moves), and applied at the call site to
   the caller's own place map: a released member's later use or release is
   refused or accused as rule 5k's caller guard does today, a stored member
   becomes a field link (CG11's owned-field model). Contract specification
   and grammar for the declared form, loader, gateway, Lean parity. Frees the
   clear-style helpers and the setters' callers in a cascade; it discharges
   W45-MEMBER-RELEASE-PREMISES and the formal-parent half of
   W44-CALL-RESULT-INDIRECT-DESTINATION.
2. **A borrowed parent's member is never released** (decision class 8 read
   strictly): a pointer member read through a borrowed parent is an interior
   borrow, so its release is refused by name (`interior-borrow-released`), and
   a store into a borrowed parent's member is refused by name. No verdict
   moves; the 9,996-leaf family shrinks into named custody refusals that the
   census can rank.
3. **Admit only where the body also releases the parent** (rule 5k as it
   stands), and re-attribute the rest to a named refusal (as rule 12k does for
   uncontracted callees). No verdict moves.

Recommendation: 1, with 3's re-attribution first as a cheap, verdict-neutral
step so the next census names the blocker. Option 2 closes the door option 1
needs.

**Measurement (integrator, 2026-11-05, combined).** This card was merged by
the user together with CELL-PROTOCOL-RELEASE-WINDOW-FRAMES,
CFL-MPACK-EXTERNAL-DECLARATIONS, LIBC-GAP-DECLARATIONS,
LOG-PRINT-BODY-CONTRACTS, LIBRDKAFKA-HEADER-INLINES,
MEMBER-ACCESS-PROJECTION and CURRENT-RUN-DEFECTS (main `25508ccaf`, records
`8531b005b`, full gate passed), so one undeclared measurement covers them:
tip `rev:8531b005b` against `baseline:c8c7edf21`, output
`private/measure/integrate-r4-20261105T111129Z`, exit 0, 145 min. System
and slice: 0 moves. Corpus: 38 moves (Safe +37, Unsafe +1), 2 unit moves
(`ctr_attributes.c` -> Safe, `cmt_decode_statsd.c` -> provisional Unsafe), 0
unmatched. Every move declared by a merged card's PREDICTION.md happened. 29
moves are declared there; `_rd_strcasestr` is LIBC-GAP's recorded extra; 8
are cascades no single-card prediction counted, each with no leaf left at
the tip:
- `crc32c_init_hw` (crc32c.c): its only base leaves were the two calls to
  `crc32c_zeros`, which MEMBER-ACCESS-PROJECTION makes Safe (in-unit summary).
- `rd_kafka_mock_Produce_reply_tags_partition_write`,
  `rd_kafka_msgset_writer_write_MessageSet_v2_header`,
  `rd_kafka_msgset_writer_write_Produce_header`: base leaves were calls to
  the `rd_kafka_buf_write_*` writers this card declares, plus `unresolved
  resource identity` and member-store `indirect memory access` leaves on
  their results (`msetw->msetw_of_start = rd_kafka_buf_write_i64(rkbuf, 0)`,
  a scalar offset) that were artefacts of the undeclared call.
- `rd_kafka_msg_partitioner_consistent` (`rd_crc32_init`/`rd_crc32_update` in
  its return expression), `rd_kafka_outq_len` and `rd_kafka_queue_length`
  (`rd_kafka_q_len(rkqu->rkqu_q)` in a return): the return-expression and
  argument-member leaves were on calls this card declares borrowing.
- `rd_kafka_partition_leader_destroy_free(void *ptr)`: passes its formal (as
  `leader = ptr`) to `rd_kafka_partition_leader_destroy`, declared here
  `resource_released(leader, nonnull)`: a release of its own formal, as the
  base's `unresolved resource identity ptr` leaf could not read.
[integrate-r4-native-comparison.json](../librdkafka-header-inlines/integrate-r4-native-comparison.json)
(sha256 `01ab43b871fb84afdc0710b3326a00c5dc67ea785daad61d2a22f0d7519eb9f5`).

## 4. Follow-ups

- **MEMBER-CUSTODY-THROUGH-FORMAL-PARENT** (registered): §3's choice, with
  the census numbers; its card should first run
  [`scripts/run_current_census.py`](scripts/run_current_census.py) (the
  `indirect-census` build's call-argument and parity streams) to name each
  refused actual's position and route answer, which this card could not.
- W45-MEMBER-RELEASE-UNTYPED-DECLARATIONS (amended): the typedef-spelled base
  or member at a `Released` position is still refused at `96a3efd3b`
  (probe, §2.1); the next reading-defect fix.
- W46-LEXICAL-GATE-NON-ACCESS (amended): rule 38h reads the uninitialized
  declarations; on the 046accf6 run 86 non-access leaves remain (40
  cast-address call statements; 46 initialized multi-declarators and
  `cfl_list_entry`/curl macro statements), overlapping CG14-DECLARATION-FORMS
  and CG08-CAST-AND-QUERY-RESIDUALS.
- W44-CALL-RESULT-INDIRECT-DESTINATION (amended): 424 call-result stores into
  members on the 046accf6 run, 19 sole-blocker functions.
- Two calls in one statement (`strlen(a->p) + strlen(b->q)`): each call alone
  is admitted; the statement is refused. Recorded under
  MEMBER-CUSTODY-THROUGH-FORMAL-PARENT's census note for the next census to
  size, not registered on its own.
