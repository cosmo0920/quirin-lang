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

## 3. Stop: the remaining yield is near zero

The brief's stop condition holds: of the draft's 39 frozen functions (forecast
15 Safe, band 10-20), **2 rows can move without a user decision**
(`LZ4_hashPosition`, `rd_kafka_share_partition_offsets_list_new`), **2 more
only under a semantic choice** (the builtins), against 3,750 Unknown rows. 801
of the 1,058 leaves were AG3 re-attributions to uncontracted callees, 6 freed
rows' constructs wait on their callees' own Unknown summaries, and the rest are
refusals earlier cards made on purpose. No semantic change, Lean module,
fixture or rule text was written; no PREDICTION.md was frozen; rule id 79 was
not taken.

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

## 4. Records

- [census/](census/): re-census join, leaf fates, Monkey line offsets, shapes,
  the unrun selection.
- [scripts/](scripts/): `slim.py`, `recensus.py`, `offsets.py`, `fate.py`,
  `allsites.py`, `classify.py` (re-census); `probe-build.sh`, `probe-run.sh`,
  `preprocess.py`, `unit-probe.sh`, `parity_reasons.py` (probes);
  `build.sh`, `run_current_census.py` (the stopped unit reproduction).
- [probes/](probes/): probe inputs, outputs and parity extracts.
- [integration-amendments.json](integration-amendments.json).
