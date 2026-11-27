# CG22-INDIRECT-CALLS-AND-CALLBACKS: prediction

Frozen before the AFTER binary was measured. Inputs: the BEFORE runs on main
`b6a3dc42` (`private/cg22/before*`) and the census runs of the census-only
build (`legacy_indirect_calls.rs` with `ADMIT = false`: every parity row, no
rewrite; `private/cg22/census*`), whose three reports are byte-identical to
BEFORE (corpus `3d53a5d0…`, system `d7aa010b…`, slice `0cdbef8b…`): the
census changes nothing. The implementation existed when this was written;
no AFTER measurement did.

## Census (distinct sites: function, line, callee tokens)

`artifacts/distinct-{corpus,system,slice}.json`, `artifacts/census-corpus.json`.

| | corpus | system | slice |
|---|---:|---:|---:|
| calls the elaborator's reading would call "not direct" | 5,568 | 2,892 | 3,109 |
| of which builtins (`BuiltinFnToFnPtr`, not indirect) | 4,996 | 2,460 | 2,524 |
| genuine indirect call sites | 572 | 432 | 585 |
| in a reported function | 235 (185 functions) | 12 (7) | 20 (12) |
| **closed candidate set** | **0** | **0** | **0** |

Every genuine site is an open set. Corpus by shape: a member read through a
pointer 220 (+66 `p->a.f` chains the census build read as `.` members; the
AFTER build names them through a pointer), a local whose definition is an
explicit cast 172 (xxHash's `*_conv` locals), a formal 102, an element or
member of a mutable static 6, a local without an initializer 2. System and
slice: members through a pointer (362 / 513) and formals (70 / 72) only.
No single-name callee shares a contracted name (no shadowing hazard).

Callback registrations (a call passing a function designator): corpus 676
distinct (584 in reported functions), system 131, slice 157; 5 corpus
registrations go to a declared registrar (`msgpack_packer_init` ×2,
`rd_kafka_topic_partition_list_sort` ×3, the latter withdrawn at BEFORE); the
relation finds 5 corpus, 1 system and 1 slice registrations whose named
context the same body later releases (no unregistration exists to see).

## Prediction

1. **The candidate-set rewrite moves nothing** on any route: no site has a
   closed set, so no token changes. The census rows are the only output.
2. **The inline declarator** (LOADER-ABI-INLINE-FNPTR-DECLARATOR): the 11
   corpus leaves `call to rd_kafka_topic_partition_list_sort whose declared
   contract was withdrawn … (abi-unreadable-declaration)` over 7 functions
   leave. The contract (`cmp: fnptr`, borrowed) then applies at each call.
   - `rd_kafka_topic_partition_list_sort_by_topic` and `…_by_topic_id`
     (sole blocker) move **Unknown → Safe** if the call's designator and
     `NULL` actuals are read as data; otherwise each keeps one replacement
     leaf at that line.
   - The other five (`rd_kafka_assignment_add`/`_subtract`,
     `rd_kafka_share_assignment_add`/`_subtract`, `sortPartitions`) keep
     other leaves and stay Unknown.
   - If the two wrappers publish a summary, same-unit callers lose their
     `call to uncontracted function rd_kafka_topic_partition_list_sort_by_topic`
     leaf only where the wrapper is in the derivation scope; no further
     status move is expected (every such caller in `rdkafka_partition.c`
     has other leaves).
   - System and slice: no withdrawal of this key exists; unchanged.
3. Corpus totals: 629 / 10 / 4,195 → at most **631 / 10 / 4,193**; no Safe →
   non-Safe move, no leak or violation added. System 44 / 2 / 188 and slice
   29 / 1 / 239 / 1 Pending: **byte-identical** reports.
