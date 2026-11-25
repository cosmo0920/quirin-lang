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
| `rd_kafka_purge_flags2str`, `rd_kafka_actions2str` | `return rd_flags2str(buf, sizeof(buf), names, flags);` | `rd_flags2str` contracted (librdkafka provider) but its body Unknown; a pointer result derived from a static buffer | Unknown (W45-RETURN-POINTER-RESULT-READS) |
| `rd_kafka_msg_partitioner_murmur2` | `return (rd_murmur2(key, keylen) & 0x7fffffff) % partition_cnt;` | `rd_murmur2` cross-unit, Unknown, uncontracted | Unknown |
| `rd_kafka_msg_partitioner_fnv1a` | `return rd_fnv1a(key, keylen) % partition_cnt;` | `rd_fnv1a` cross-unit Safe but uncontracted (no cross-unit summary route) | Unknown |
| `rd_kafka_path_is_dir` | `return rd_file_stat(path, &is_dir) && is_dir;` | `rd_file_stat` cross-unit, uncontracted | Unknown |
| `thrd_setname` | `if (!pthread_setname_np(pthread_self(), name))` | `pthread_setname_np` uncontracted; a nested call | Unknown |
| `rd_kafka_sw_str_sanitize_inplace`, `rd_kafka_topic_cmp_rkt` | `char *s = str, *d = str;`, `T *a = (void *)_a, *b = (void *)_b;` | a formal aliased by two declarators: refused on purpose | Unknown |
| `mk_handler_param_get` | `return mk_list_entry(head, …);` | an interior/enclosing pointer returned: refused on purpose | Unknown |
| `mk_socket_set_nonblocking` | `if (fcntl(fd, F_SETFL, fcntl(fd, F_GETFL, 0) \| O_NONBLOCK) == -1)` | nested call on a descriptor formal: refused on purpose (AG3) | Unknown |
| `rd_kafka_conf_validate_single` | `return !strchr(val, ',') && !strchr(val, ' ');` | `strchr` contracted (libc, derived result); a connective of negated calls in a return | **candidate** |
| `LZ4_NbCommonBytes` | `return (unsigned)__builtin_ctzll((U64)val) >> 3;` (×4) | a pure bit count | **candidate** |
| `log2_floor` | `return n == 0 ? -1 : 31 ^ rd_clz(n);` (`rd_clz` = `__builtin_clz`) | a pure bit count under `?:` | **candidate** |
| `LZ4_hashPosition` | `if ((sizeof(reg_t)==8) && (tableType != byU16)) return …;` | every callee Safe in the unit | **candidate** |
| `rd_kafka_share_partition_offsets_list_new` | `size = sizeof(*list) + (capacity * sizeof(T *));` | `sizeof(*p)` unevaluated | **candidate** |

**Remaining yield: at most 5 rows** of the 19 (against the draft's 15 Safe,
band 10-20). The other 14 keep a refusal that is either the callee's (an
Unknown or uncontracted callee: W44-CONDITION-CALL-UNCONTRACTED,
cross-unit summaries) or one an earlier card made on purpose.
