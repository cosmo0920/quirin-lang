# K2-NO-TYPED-ACCESS: leaves with no typed access, re-censused

Card K2 of round 37 (brief `private/integrator-briefs/r37-k246-common.md`), base
`5087f9295` (origin/main, round 34 landed). Bundle: an IND/COND/RET leaf whose
construct carries no typed access path at all (census tag `none`); owners
W46-LEXICAL-GATE-NON-ACCESS (primary), W44-CONDITION-CALL-REFUSED-SHAPES,
W44-CONDITION-CALL-ASSIGNMENT; rules 10k, 10n. The census draft is
[current-access-alias-census/predictions/K2.md](../current-access-alias-census/predictions/K2.md).

## 1. Re-census (11-07 vs 11-24)

Source: the user's `current` report written 2026-11-24 09:58 (Fluent Bit
`046accf6`, 1,173 Safe / 3,750 Unknown / 11 Unsafe rows; read-only, streamed
once into a private pickle by [scripts/slim.py](scripts/slim.py)), joined to
CURRENT-ACCESS-ALIAS-CENSUS's 11-07 leaf tags by (file, function, family, sub,
preprocessed line) ([scripts/recensus.py](scripts/recensus.py), output
[census/recensus.json](census/recensus.json)). Monkey's analysed text moved by
a constant per file between the runs (`--monkey-linked`, offsets −1,081 to −459,
[scripts/offsets.py](scripts/offsets.py), [census/monkey-line-offsets.json](census/monkey-line-offsets.json)),
so Monkey lines are joined after that per-file shift; every other file joins at
offset 0. 12,083 of 12,259 IND/COND/RET leaf groups join; an unjoined group
(176, mostly `rdhttp.c` and `cmt_map.c`, unjoined at 11-07 too) carries `new`
and is never counted removable.

| Measure | 11-07 (base `7b726587b`) | 11-24 (user's run) |
|---|---:|---:|
| function rows / Unknown | 4,912 / 3,832 | 4,934 / 3,750 |
| K2 leaves | 1,058 (COND 952, IND 77, RET 29) | **240** (COND 133, IND 74, RET 29; 236 on Unknown rows) |
| Unknown rows touched | 601 | 181 |
| rows freed alone / distinct functions | 40 / 39 | **19 / 19** |

**Where the 1,058 leaves went** ([scripts/fate.py](scripts/fate.py),
[census/fate.json](census/fate.json), keyed by line): **801 COND leaves were
exchanged by AG3's rule 63 (2)** for the named text
``call to uncontracted function `F` in branch condition`` (a re-attribution,
never an admission: their blocker is now the callee, owned by
W44-CONDITION-CALL-UNCONTRACTED); 247 keep the same refusal text (240 join at
the same family and sub); 9 are gone with no diagnostic at the line; 1 is on a
row now Safe.

**The 11-07 frozen list (39 functions) now:** **1 Safe** (`LZ4_initStream`),
**17 still freed alone**, **19 Unknown with a changed blocker**, and the 2
Monkey rows join only after the line shift (`mk_handler_param_get` freed alone,
`mk_socket_set_nonblocking` freed alone). The 19 with a changed blocker are all
AG3 exchanges: 16 now carry an uncontracted run-defined callee
(`pb_decode_varint`, `pb_read`, `pb_field_iter_*`, `pb_encode`,
`rd_kafka_set_fatal_error0`, `rd_kafka_global_cnt_get`,
`rd_kafka_conf_is_modified`, `rd_kafka_get_legacy_ApiVersions`,
`rd_kafka_topic_partition_list_find`, `rd_kafka_pattern_list_init`,
`rd_kafka_broker_ApiVersion_at_least_no_lock`, `rd_list_elem`,
`rd_rand_bytes_by_ossl`, `varint_parse32_with_limit`, `mk_file_get_info`,
`flb_input_chunk_has_other_routes`), 3 an external one only (`RAND_priv_bytes`,
`setegid`/`seteuid`, `strptime`).

### The 240 leaves by concrete source shape

Each leaf's source line (read at its `origin` in the prepared tree, read-only)
classified lexically ([scripts/classify.py](scripts/classify.py), all sites in
[census/shapes.json.gz](census/shapes.json.gz)); "alone" counts the rows whose
every blocker is a K2 leaf of that shape.

| Shape | Leaves | Rows | Alone | Owner, and why it stays refused (or is a K2 candidate) |
|---|---:|---:|---:|---|
| negated direct call in a condition (`if (!pb_encode_varint(…))`) | 33 | 21 | 1 | the callee is a run-defined function with no summary in force, or the call is nested (`!pthread_setname_np(pthread_self(), …)`): AG3 rule 63 reads direct calls only, none nested (W44-CONDITION-CALL-REFUSED-SHAPES) |
| assignment in a condition (`if ((err = f(…)))`, `(err = ErrorCode)`) | 33 | 31 | 0 | rule 10x's split owns assigned calls (W44-CONDITION-CALL-ASSIGNMENT) |
| other direct call in a condition (`fcntl(s, F_SETFL, fcntl(s, F_GETFL, 0) \| O_NONBLOCK)`, `EVP_*`, `strncmp(buf + n, …)`) | 25 | 22 | 1 | nested calls, pointer arithmetic actuals, uncontracted externals (AG3's refused shapes) |
| initialized multi-pointer declarator (`char *s = str, *d = str;`) | 18 | 16 | 2 | W46-CONTRACT-ACCESS-EXPANSION admits it only when every source and destination is statically untracked; a formal source is a custody-bearing alias (rules 5i/5l, K4's territory) |
| `mk_list_entry` / `cfl_list_entry` / `container_of` | 16 | 10 | 1 | `(T *)((char *)p - offsetof(T, m))` is pointer arithmetic deriving an enclosing record (CG16): an access derivation, not a non-access |
| cast of an address actual (`(char **)&end`) | 14 | 13 | 0 | CG08, rule 11b |
| call nested in a returned scalar (`return f(k, n) % c;`, `return !strchr(v, ',') && …`) | 13 | 13 | 4 | rule 10k (2): `&&`, `\|\|`, `?:` and calls in a return refuse; the elaborator refuses a call unordered with another event (CG05) |
| `rd_kafka_buf_{skip,read}_tags` loop macro | 13 | 4 | 0 | a goto-bearing `do`/`while` expansion |
| unit-test macro (`RD_UT_ASSERT`, `verifyNonRackAwareAssignment`) | 12 | 10 | 0 | test bodies |
| call through a function pointer (`cmp(a, b)`, `cb(e, o)`) | 12 | 12 | 0 | an open set, rule 11p (by design) |
| whole-call or pointer-valued return (`return cJSON_DetachItemViaPointer(o, d);`, `return rd_flags2str(ret, sizeof(ret), …);`) | 8 | 8 | 6 | the callee's own summary is Unknown in every one of the 6 alone rows (below) |
| call actual that is a comparison or `?:` (`f(rkcg, err == X)`) | 6 | 5 | 0 | CG05 / rule 10k (3) |
| `__builtin_clz` / `__builtin_ctz` bit count in a return | 5 | 2 | 2 | **K2 candidate** |
| pointer comparison with arithmetic (`pIn < pInLimit - (STEPSIZE - 1)`) | 4 | 3 | 0 | gap entries M15/M16 (CG16) |
| OpenSSL `sk_*_num` macro | 4 | 3 | 0 | uncontracted externals |
| `sizeof(*p)` in a call-free scalar statement | 2 | 2 | 1 | **K2 candidate**: rule W44-COLLECTION-ALLOCATION reads it only in a statement whose first call is an ordinary callee |
| `sizeof(T) == K` beside a scalar test in a condition | 1 | 1 | 1 | **K2 candidate** (reason to be read from the unit, §2) |
| other (scalar arithmetic and `sizeof` inside call actuals, `return RD_KAFKA_UUID_ZERO;`) | 21 | 21 | 0 | CG05, CG14 |

### The 19 rows freed alone, by what removing the leaf would still leave

The freed-alone figure is an upper bound: it takes a row Safe once its K2 leaf
is gone, but the leaf's own construct still has to discharge. Reading each
row's construct and callee in the same report:

| Row | Construct | After the leaf | Expected |
|---|---|---|---|
| `cJSON_DetachItemFromObject`, `…CaseSensitive` | `return cJSON_DetachItemViaPointer(object, to_detach);` | callee Unknown (same unit) | Unknown |
| `rd_kafka_broker_ApiVersion_at_least`, `…_no_lock` | `return rd_kafka_broker_ApiVersion_supported0(…) != -1;` | callee Unknown (same unit) | Unknown |
| `rd_kafka_purge_flags2str`, `rd_kafka_actions2str` | `return rd_flags2str(buf, sizeof(buf), names, flags);` | `rd_flags2str` is declared in the librdkafka provider and its own row is Unknown; a pointer result derived from a static buffer | Unknown (W45-RETURN-POINTER-RESULT-READS) |
| `rd_kafka_msg_partitioner_murmur2` | `return (rd_murmur2(key, keylen) & 0x7fffffff) % partition_cnt;` | `rd_murmur2` cross-unit, Unknown, uncontracted | Unknown |
| `rd_kafka_msg_partitioner_fnv1a` | `return rd_fnv1a(key, keylen) % partition_cnt;` | `rd_fnv1a` cross-unit Safe but uncontracted (no cross-unit summary route) | Unknown |
| `rd_kafka_path_is_dir` | `return rd_file_stat(path, &is_dir) && is_dir;` | `rd_file_stat` cross-unit, uncontracted | Unknown |
| `thrd_setname` | `if (!pthread_setname_np(pthread_self(), name))` | `pthread_setname_np` uncontracted; a nested call | Unknown |
| `rd_kafka_sw_str_sanitize_inplace`, `rd_kafka_topic_cmp_rkt` | `char *s = str, *d = str;`, `T *a = (void *)_a, *b = (void *)_b;` | a formal aliased by two declarators: refused on purpose | Unknown |
| `mk_handler_param_get` | `return mk_list_entry(head, …);` | an interior/enclosing pointer returned: refused on purpose | Unknown |
| `mk_socket_set_nonblocking` | `if (fcntl(fd, F_SETFL, fcntl(fd, F_GETFL, 0) \| O_NONBLOCK) == -1)` | nested call on a descriptor formal: refused on purpose (AG3) | Unknown |
| `rd_kafka_conf_validate_single` | `return !strchr(val, ',') && !strchr(val, ' ');` | `strchr` contracted (libc, interior result); a connective of negated calls in a return; but see §2: glibc spells it as a `_Generic` | not K2's (§2) |
| `LZ4_NbCommonBytes` | `return (unsigned)__builtin_ctzll((U64)val) >> 3;` (×4) | a pure bit count | **candidate** |
| `log2_floor` | `return n == 0 ? -1 : 31 ^ rd_clz(n);` (`rd_clz` = `__builtin_clz`) | a pure bit count under `?:` | **candidate** |
| `LZ4_hashPosition` | `if ((sizeof(reg_t)==8) && (tableType != byU16)) return …;` | every callee Safe in the unit | **candidate** |
| `rd_kafka_share_partition_offsets_list_new` | `size = sizeof(*list) + (capacity * sizeof(T *));` | `sizeof(*p)` unevaluated | **candidate** |

**Remaining yield by this reading: at most 5 rows** of the 19 (against the
draft's 15 Safe, band 10-20). The other 14 keep a refusal that is either the
callee's (an Unknown or uncontracted callee: W44-CONDITION-CALL-UNCONTRACTED,
cross-unit summaries) or one an earlier card made on purpose. §2 reads the
reasons from the bodies and lowers the 5 to **4, of which 2 need a user
decision**.

## 2. The candidates' reasons, read from the bodies (method step 2)

Two probe instruments, both on the base gateway (`5087f9295`), read-only
inputs, outputs under [probes/](probes/):

- **generic probes**: the K1 scratch harness
  ([probes/k2_probe_scratch.rs.txt](probes/k2_probe_scratch.rs.txt), kept as
  text, not a cargo target) runs `analyze_unrestricted_c11` on one function
  `w` per line over the generic `d_*`/`k_*` fixture contracts
  ([probes1.txt](probes/probes1.txt) … [probes3.txt](probes/probes3.txt), outputs
  `*.base.out`, [scripts/probe-run.sh](scripts/probe-run.sh));
- **unit probes**: the same harness's `unit_probe` analyses named functions of
  the real preprocessed production unit (its own compile command,
  [scripts/preprocess.py](scripts/preprocess.py)) with libc's `string.qv`, with
  the typed-path parity stream on (`QCORE_TYPED_PATH_PARITY`), which records
  every reader's refusal by name ([scripts/unit-probe.sh](scripts/unit-probe.sh),
  [scripts/parity_reasons.py](scripts/parity_reasons.py)). The `lz4.c` probe
  reproduces the user's leaves exactly (`LZ4_hashPosition` 1 branch-condition
  leaf, `LZ4_NbCommonBytes` 4 return leaves, [unit-lz4.base.out](probes/unit-lz4.base.out)).

A unit-scoped `current --selection` reproduction ([census/selection.json](census/selection.json),
[scripts/run_current_census.py](scripts/run_current_census.py)) was queued in an
`audit` slot and stopped before it was admitted (fifth in the queue behind four
other jobs); the unit probes answered the question it was meant to answer.

| Row | Reader and refusal (parity stream) | Reading |
|---|---|---|
| `LZ4_hashPosition` | `elaboration-condition`: ``unlisted form: event outside this card's lowering`` on `( sizeof ( reg_t ) == 8 ) && ( tableType != byU16 )` ([unit-lz4.reasons.json](probes/unit-lz4.reasons.json)) | The elaborator reads the connective as a CG06 value region (`Branch`, the arm, `Merge`) but the **condition** lowering (`lower_events`) lowers no `Branch`/`Merge` event; rule 5h's leaf reader refuses the `sizeof` leaf (``C operator `sizeof` requires type/operand analysis``). Generic probes: `if (sizeof(long) == 8)` Safe, `if ((t == 8) && (t != 2))` Safe, `if ((sizeof(long) == 8) && (t != 2))` Unknown, `return (sizeof(long) == 8) && (t != 2);` Safe (rule 10z's return route). A K2 shape: a connective condition whose leaf is a type query. |
| `LZ4_NbCommonBytes` | `call-instantiation`: ``CallRefused("__builtin_ctzll", "call to uncontracted function")``; `elaboration-return` and `nested-call`: ``indirect call (CG22)`` (×4: `ctzll`, `ctz`, `clzll`, `clz`) | No contract or table names the bit-count builtins, and the elaborator reads a builtin callee as an indirect call. |
| `log2_floor` | (same builtin, `rd_clz` = `__builtin_clz`, under `?:`) generic probe: `return __builtin_clz(v);` gives ``call to uncontracted function `__builtin_clz` `` | As above. |
| `rd_kafka_share_partition_offsets_list_new` | lexical gate: ``indirect memory access is outside the resource projection``; `elaboration-statement`: ``unlisted form: write to an object the gateway does not read as a scalar local`` on `size = sizeof ( * list ) + ( capacity * sizeof ( rd_kafka_share_partition_offsets_t * ) )` ([unit-share.reasons.json](probes/unit-share.reasons.json); the probe, stopped after this function's rows, had no `rd_calloc` contract loaded, so its `rd_calloc` refusal is the probe's, not the run's) | W44-COLLECTION-ALLOCATION's unevaluated-operand reading covers `sizeof(*v)` only in a statement whose first call is an ordinary callee; this statement has no call. Generic probes reproduce it narrowly: `s = sizeof(*b) + n * sizeof(char *);` Safe, `s = (n * sizeof(char *));` Safe, `s = sizeof(*b) + (n * sizeof(char *));` Unknown (same two refusals). A K2 shape: `sizeof(*p)` beside a parenthesised product holding a type query, in a call-free scalar statement. |
| `rd_kafka_conf_validate_single` | `value-connective-return` and `nested-call`: ``no unique Clang node``; `call-instantiation`: ``a connective (CG06)`` ([unit-rdkafka_conf.reasons.json](probes/unit-rdkafka_conf.reasons.json)) | glibc's const-preserving `strchr` macro expands each call to `_Generic (0 ? (val) : (void *) 1, const void *: (const char *) (strchr (val, ',')), default: strchr (val, ','))`, so the return's tokens no longer match one Clang node. Generic probe `return !d_chr(v, ',') && !d_chr(v, ' ');` is Safe. **Owned by CG07-GENERIC-TOKEN-AST-MATCH**, not a no-access shape. |

**The bit-count builtins are a semantic choice fail-closed does not settle.**
GCC and Clang document `__builtin_clz(x)` and `__builtin_ctz(x)` as having an
undefined result when `x = 0`. Admitting the call needs one of two readings,
and the project has no recorded decision for either:

- (a) *obligation-bearing*: admit `__builtin_c{l,t}z{,l,ll}(x)` only where the
  flow proves `x ≠ 0`. `log2_floor`'s arm is guarded (`n = 0 ? -1 : 31 ^ clz(n)`,
  the call only on the edge `n ≠ 0`); `LZ4_NbCommonBytes` is not (its
  `assert(val != 0)` is `((void)0)` unless `LZ4_DEBUG ≥ 1`, so `val ≠ 0` is a
  caller precondition), so under (a) only `log2_floor` can move;
- (b) *value-only*: read the result as an unspecified integer and the call as
  custody-neutral (no read, no write, no store), so both move. This departs
  from rule 10s's table, where a node with a C fault case is never called
  custody-neutral.

Where the builtin's declaration lives is a second open choice: builtins are
not libc, so `quirin-provider-libc` is not obviously their home, and no
compiler-builtins provider exists.

**Vocabulary.** The premise-formula vocabulary (`qcore_frontend::premise_formula`,
C04.18.9) has no term for the conditions these shapes need (a type query's
operand type, a builtin's argument value, an evaluated arm), so the admission
conditions below are written as formulas over C-level predicates and cannot be
checked by its validator; any option that implements them needs those terms'
specification rows and Lean models first, or the user's ruling that gateway
admission conditions are outside that vocabulary.

## 3. The stop (first hand-back): the remaining yield is near zero

The brief's stop condition holds: of the draft's 39 frozen functions (forecast
15 Safe, band 10-20), **2 rows can move without a user decision**
(`LZ4_hashPosition`, `rd_kafka_share_partition_offsets_list_new`), **2 more
only under a semantic choice** (the builtins), against 3,750 Unknown rows. 801
of the 1,058 leaves were AG3 re-attributions to uncontracted callees, 6 freed
rows' constructs wait on their callees' own Unknown summaries, and the rest are
refusals earlier cards made on purpose. At that hand-back no semantic change,
Lean module, fixture or rule text had been written.

### Options (recommendation first)

1. **Land the records only (recommended).** Land this report and its
   [integration-amendments.json](integration-amendments.json): amend the three
   owner ids and CG07-GENERIC-TOKEN-AST-MATCH with the 11-24 re-census, and
   register the three residual sub-shapes as follow-ups
   (`K2-CONNECTIVE-CONDITION-TYPE-QUERY`, `K2-SIZEOF-OPERAND-PRODUCT`,
   `K2-BUILTIN-BIT-COUNTS`), each with its row, reason and discharge. No row
   moves; nothing to measure.
2. **Implement the two decided shapes (about 2 rows).** (i) A connective
   condition rule 5h's leaf reader and the elaborator's condition lowering
   both refused is lowered as rule 10z's value region followed by a branch
   that refines no edge, admitted only when
   `∀ e ∈ events(region). e ∈ {Read(x) | x automatic ∧ ¬addr_taken(x)} ∪ {Branch, Merge}`
   (no call, store, dereference or volatile access in any arm), so no arm's
   obligation is moved off the edge C evaluates it on; (ii) the
   unevaluated `sizeof(*p)` operand in a call-free scalar statement, admitted only when
   `¬variably_modified(type(*p))` (C11 6.5.3.4p2). Each needs a Lean module
   (region-to-condition lowering preserves the state on both edges; the
   unevaluated operand yields no event), fixtures with near-misses (`p && p->x`,
   a call in an arm, a VLA operand), rule 79 text, PREDICTION.md
   (`LZ4_hashPosition`, `rd_kafka_share_partition_offsets_list_new` Unknown →
   Safe on `current`; corpus and twelve-home only if they hold `lz4.c` /
   `rdkafka_share_acknowledgement.c`), and the integrator's measurement. Cost:
   a full card for 2 rows.
3. **Option 2 plus the builtins, after the user decides (a) or (b) and the
   declaration's home.** Up to 4 rows: (a) adds `log2_floor`, (b) adds
   `log2_floor` and `LZ4_NbCommonBytes`. The admission condition under (a) is
   `call = __builtin_c{l,t}z{,l,ll}(x) ∧ (path ⊢ x ≠ 0) ⇒ custody_neutral(call)`;
   under (b) `call = __builtin_c{l,t}z{,l,ll}(x) ⇒ custody_neutral(call)`.

**Decision.** USER DECISION 2026-11-25 (verbatim): "Build the 2 rows":
option 2 only; the builtins stay out with their open choices
(`K2-BUILTIN-BIT-COUNTS`). The formula rule stands: the vocabulary gap is
closed as rule 38k closed it, with a prefixed K2 block of the typed rendering
(§4). The branch was then rebased onto origin/main `110a69c8e`
(MEMBER-EFFECT-STORES landed), and the K2 block appended after its MES block.

## 4. Rule 79: the two shapes and why each is sound

[Rule 79](../../agents/rules/79.md), `crates/qcore-c-gateway/src/type_query_positions.rs`;
Lean `QCore.TypeQueryPositions` (`proof/QCore/TypeQueryPositions.lean`,
audit `proof/AuditTypeQueryPositions.lean`). Every condition and refusal is a
formula of the C04.18.9 typed rendering over the K2 terms, validated by
`premise_formula::guarded_proposition` before any admission (an unvalidated
table admits nothing); the gloss after each formula only reads it.

**The K2 block** (`qcore-frontend/src/premise_formula.rs`, appended after
MES; [spec C04.18.9](../../QCORE_CONTRACT_SPEC_r0_3.md), "K2 block"): sorts
`k2_site`, `k2_event`, `k2_query`, `k2_run`; terms `#k2_condition`,
`#k2_older_refused`, `#k2_region`, `#k2_scalar_result`, `#k2_event_of`,
`#k2_branch_event`, `#k2_object_read`, `#k2_unaliased`, `#k2_query_of`,
`#k2_type_operand`, `#k2_variably_modified`, `#k2_run_at`,
`#k2_name_star_sizeof`, `#k2_declaration`. Each is modelled in Lean
(`termModel`: `Facts`, `EventClass`, `Query`, `Run`, `nameStarSizeof`,
`isDeclaration`).

### (1) A connective condition with a type query (`LZ4_hashPosition`)

Asked in `elaborate_condition` after its result is a non-pointer value, only
when the elaboration holds a CG06 region; before the rule every such condition
was refused at the lowering (`Branch` is no lowered event), so it only adds
admissions.

| id | formula |
|---|---|
| `k2_region_site` | `((#k2_older_refused(#k2_condition) && #k2_region(#k2_condition)) && #k2_scalar_result(#k2_condition))` |
| `k2_type_query_present` | `exists (#q: k2_query) { #k2_query_of(#q,#k2_condition) }` |
| `k2_queries_unevaluated` | `forall (#q: k2_query) { (#k2_query_of(#q,#k2_condition) => (#k2_type_operand(#q) && !#k2_variably_modified(#q))) }` |
| `k2_events_inert` | `forall (#e: k2_event) { (#k2_event_of(#e,#k2_condition) => (#k2_branch_event(#e) \|\| (#k2_object_read(#e) && #k2_unaliased(#e)))) }` |

*Sound because* a type query of a type that is not variably modified is not
evaluated (C11 6.5.3.4p2), and every other event, on every arm, is a
`Branch`/`Merge` (no storage) or a read of an automatic object through its own
name whose address is never taken (no lvalue but its name designates it: no
dereference, no non-null or live obligation, no state change). Whichever arm
C evaluates, the condition is the identity and owes nothing
(`an_admitted_condition_is_the_identity_on_every_path`,
`an_admitted_condition_owes_nothing`), so lowering it as no instruction and a
branch refining neither edge is the condition on both edges
(`the_lowering_is_the_condition`); not refining keeps both successors.

### (2) `NAME * sizeof` is no declaration (`rd_kafka_share_partition_offsets_list_new`)

| id | formula |
|---|---|
| `k2_product_run_is_no_declaration` | `(#k2_name_star_sizeof(#k2_run_at) => !#k2_declaration(#k2_run_at))` |

The total-or-reject declaration reader tried `capacity * sizeof ( T * )`
(after the `(` of `(capacity * sizeof(T *))`) as a declaration, failed, and
left the whole function's declared types unread; the elaborator then refused
the store into `size`. `is_declaration_candidate` now drops a run whose
tokens are an identifier, `*`, `sizeof`. *Sound because* a declaration naming
a variable puts an identifier after the `*` and the keyword `sizeof` is none
(C11 6.4.1p2, 6.7.6): no declaration is lost, and every reader of the
candidate keeps its total-or-reject premise
(`a_product_run_is_no_declaration`,
`dropping_product_runs_loses_no_declaration`). What the statement means is
then decided by the routes that already read it (the elaborator's
`sizeof(*p)` is card CG08's type query: unevaluated unless variably
modified).

## 5. Refused shapes and their named reasons

| Reason | Formula (its truth refuses) | Near-miss fixtures |
|---|---|---|
| `k2_refuse_no_type_query` | `forall (#q: k2_query) { !#k2_query_of(#q,#k2_condition) }` | unit vector `no_query` |
| `k2_refuse_evaluated_query` | `exists (#q: k2_query) { (#k2_query_of(#q,#k2_condition) && (!#k2_type_operand(#q) \|\| #k2_variably_modified(#q))) }` | `(sizeof(t) == 4) && (t != 2)` (expression operand); unit: `int[n]`, a typedef desugaring to `int[n]` |
| `k2_refuse_event` | `exists (#e: k2_event) { (#k2_event_of(#e,#k2_condition) && (!#k2_branch_event(#e) && (!#k2_object_read(#e) \|\| !#k2_unaliased(#e)))) }` | an uncontracted call in an arm, a store `(g = t)`, an address-taken local, a global read, a `volatile` formal, a path read `((b + 1)->n != 2)` |

Each keeps the position's older refusal byte for byte. Shapes an older route
already decides at base keep that answer and rule 79 is not asked (an
assignment to a local, an increment, a member read, a contracted call, a
variably modified `sizeof`, `||`): `what_an_older_route_decides_is_unchanged`.
Statement side: a variably modified operand (`int (*v)[n]`) stays Unknown;
`( T * p )` and `( T * sizeofx )` stay declaration candidates. Still out of
scope and registered: the bit-count builtins (`K2-BUILTIN-BIT-COUNTS`) and the
other 15 freed-alone rows of §1 (their owners).

## 6. Tests and Lean

- `crates/qcore-c-gateway/tests/k2_no_typed_access.rs` (13 tests): admitted
  shapes (LZ4's own spelling with a typedef'd operand and an enum formal, both
  operand orders, `while`, `for`, `do`; the parenthesised product with a
  typedef'd and a plain pointer, the flexible allocation) and every near-miss
  above. Each body's base answer is in [probes/](probes/) (`probes1`-`probes7`,
  `*.base.out`, the base binary built from `5087f9295`).
- `crates/qcore-c-gateway/tests/unit/type_query_positions.rs` (9 tests): every
  formula validated and K2-only; the K2 block new, prefixed, contiguous and the
  table's tail, its term and sort lists equal to Lean's; the formula table and
  decision vectors equal to Lean's; each refusal negates its condition; the
  Clang-node query reading (type operand, VLA, typedef'd VLA, expression); the
  product run against `is_declaration_candidate`.
- `crates/qcore-contract/tests/plugin_guarded_symbols.rs`: the SP9 ordering
  test now allows `SP10`, `MES` and `K2` after SP9 (the one edit outside the
  card's files besides the hooks).
- Lean `QCore.TypeQueryPositions`, 17 audited theorems, placeholder-free,
  axioms `propext` and `Quot.sound` only.

## 7. Shared-file hook points

| File | Hook |
|---|---|
| `crates/qcore-c-gateway/src/legacy_elaboration.rs` | `elaborate_condition`: one block before `self.lower_events(&elaborated)?`, guarded by `!elaborated.branches.is_empty()` (validated, facts, decide, one `NoOp` branch plan) |
| `crates/qcore-c-gateway/src/typed_field_access.rs` | `is_declaration_candidate`: three lines at the top (`product_run` → `false`) |
| `crates/qcore-c-gateway/src/lib.rs` | `mod type_query_positions;` and its `#[cfg(test)]` unit-test module, after `mod pointer_offset_place;` |
| `crates/qcore-c-gateway/Cargo.toml` | the comment line beside MES's `qcore-frontend` dependency (the dependency itself is MES's) |
| `crates/qcore-frontend/src/premise_formula.rs` | the K2 sorts after MES's in `RENDERED_VALUE_SORTS`, the K2 terms after MES's in `RENDERED_TERMS` (K6's k6_ block merges beside it) |
| `crates/qcore-contract/tests/plugin_guarded_symbols.rs` | the SP9 tail check: `"SP10" \|\| "MES" \|\| "K2"` |
| `docs/QCORE_CONTRACT_SPEC_r0_3.md` | the K2 block paragraph and table, before the SP8 block |
| generated: `docs/agents/gateway.md`, `docs/agents/README.md`, `proof/QCore.lean` | `gen_rule_docs.py`, `gen_lean_roots.py` (rerun on merge) |

## 8. Prediction and the tip probes

[PREDICTION.md](PREDICTION.md), committed before the semantic change:
`LZ4_hashPosition` and `rd_kafka_share_partition_offsets_list_new` Unknown →
Safe on `current`, corpus (eight-home) and twelve-home; system and slice
unchanged; 1,173 → 1,175 Safe on `current`. Tip unit probes (release build of
the scratch harness over the real preprocessed units with libc `string.qv`,
[scripts/tip-unit-probes.sh](scripts/tip-unit-probes.sh)):
[probes/unit-lz4.tip.out](probes/unit-lz4.tip.out): `LZ4_hashPosition` own
status `Safe` with no leaf (base: its one branch-condition leaf);
`LZ4_getPosition`/`LZ4_putPosition` keep only their other callee's leaf, as
predicted.
[probes/unit-share.tip.out](probes/unit-share.tip.out),
[probes/unit-share.tip.reasons.json](probes/unit-share.tip.reasons.json): the
`size = sizeof(*list) + (capacity * sizeof(T *));` statement (`:1120`) and its
sibling `:1082` are now admitted by the elaborator (base: the gate leaf and
"write to an object the gateway does not read as a scalar local"); with only
`string.qv` loaded the probe still refuses `rd_calloc` (uncontracted there,
contracted in production), and with a probe copy of `rd_calloc`'s effect
clauses ([probes/rd_calloc_probe.qv](probes/rd_calloc_probe.qv), not a
provider contract) `rd_kafka_share_partition_offsets_list_new`'s own status is
`Safe` with no leaf ([probes/unit-share-rd_calloc.tip.out](probes/unit-share-rd_calloc.tip.out)).
`rd_kafka_share_partition_offsets_new` keeps its `:1085` member-store leaf
(an `unknown_alias:elem` also appears beside it in the probe), as predicted
Unknown. A unit's own result in these probes is the whole unit's (Unknown).

## 8a. Targeted checks (no card gate, user 2026-11-24)

On the tip `3e1b54f5a` (analyzer bytes; later commits are records), rebased on `110a69c8e`:

| Command | Exit | Counts | Log (private) |
|---|---:|---|---|
| `scripts/targeted-tests.sh t3`: `cargo test -p qcore-c-gateway --lib` | 0 | 1,443 passed, 2 ignored | `private/k2/t3/gateway-lib.log` |
| same: `--test k2_no_typed_access` | 0 | 13 passed | `private/k2/t3/gateway-k2_no_typed_access.log` |
| same: every gateway test file holding a product run or a `sizeof(T)` comparison: `cg08_casts_and_type_queries`, `neutral_expression_positions`, `scalar_statement_forms`, `sizeof_operand` | 0 each | 14, 34, 15, 17 passed | `private/k2/t3/gateway-*.log` |
| same: `qcore-contract` `plugin_guarded_symbols`, `formal_binding_descriptions`, `cjson_guarded_symbols` | 0 each | 4, 9, 8 passed | `private/k2/t3/contract-*.log` |
| same: `cargo test -p qcore-frontend` | 0 | 312 passed | `private/k2/t3/frontend.log` |
| `cargo clippy -p qcore-c-gateway -p qcore-frontend --all-targets -- -D warnings` | 0 | | `private/k2/clippy2.log` |
| `cargo fmt --check` | 0 | | |
| `gate_lean.py --module QCore.TypeQueryPositions --audit AuditTypeQueryPositions.lean` (lean slot) | 0 | 17 theorems audited, axioms `propext`, `Quot.sound` only | `private/k2/lean3.log` |
| `gen_rule_docs.py --check`, `gen_lean_roots.py --check` | 0 | 274 rule files, 566 modules | |
| `apply_amendments.py integration-amendments.json --check` | 0 | 4 amended, 1 registered | |

## 9. Records

- [census/](census/): re-census join, leaf fates, Monkey line offsets, shapes,
  product-run sites, the unrun selection.
- [scripts/](scripts/): `slim.py`, `recensus.py`, `offsets.py`, `fate.py`,
  `allsites.py`, `classify.py` (re-census); `probe-build.sh`, `probe-run.sh`,
  `preprocess.py`, `unit-probe.sh`, `parity_reasons.py`, `tip-unit-probes.sh`
  (probes); `product_sites.py`; `targeted-tests.sh` (the card's checks);
  `build.sh`, `run_current_census.py` (the stopped unit reproduction);
  `make_amendments.py`.
- [probes/](probes/): probe inputs, outputs and parity extracts.
- [integration-amendments.json](integration-amendments.json).
