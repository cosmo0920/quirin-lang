# K5-LOCAL-ORIGIN-ALIASES: locals holding only loads from untracked objects

Card K5 of round 34 (2026-11-24), base `3ca85c7a6` (origin/main). Bundle K5 of
[CURRENT-ACCESS-ALIAS-CENSUS](../current-access-alias-census/README.md): the
ten local-origin alias groups. Owner follow-ups amended:
`W47-ALIAS-FLOW-RESIDUALS` (primary), `W47-ALIAS-TYPED-SCALAR-NAMES`,
`W47-ALIAS-NONNULL-ALLOCATOR-RESULTS`. New gateway rule 77.

## 1. Re-census against the user's 2026-11-24 run

**Method.** The census was built on `7b726587b` (2026-11-07). The user's newest
`current` run (Fluent Bit `046accf6`, written 2026-11-24 09:58: 1,173 Safe /
3,750 Unknown / 11 Unsafe function rows) was read once, read-only, into
function rows ([scripts/extract.py](scripts/extract.py)).
[scripts/recensus.py](scripts/recensus.py) matches every `unknown_alias` leaf
of an Unknown row to the census's classified leaf of the same
`(file, function, variable, origin)` (the census's
`alias-leaves.jsonl.gz`; both runs read the same prepared tree, so origins
agree) and takes its W47 group; a leaf with no match is `new`. A row is freed
by K5 alone when every diagnostic it carries is a leaf of a K5 group.
Output: [artifacts/recensus.json](artifacts/recensus.json) (the 178 `new`
leaves in [artifacts/recensus-new-leaves.json](artifacts/recensus-new-leaves.json)).
[scripts/show.py](scripts/show.py) and [scripts/bodies.py](scripts/bodies.py)
print a row's leaves with their source lines and its body.

| K5 group | Leaves 11-07 | Rows 11-07 | Leaves 11-24 | Rows 11-24 | Freed by this group alone 11-07 / 11-24 |
|---|---:|---:|---:|---:|---:|
| `poisoned-by-refusal` | 1,281 | 462 | 1,114 | 423 | 0 / 1 |
| `non-pointer-name` | 1,072 | 347 | 518 | 200 | 7 / 6 |
| `derived-local-unguarded` | 1,037 | 319 | 1,027 | 312 | 2 / 2 |
| `borrowed-path-unguarded` | 875 | 354 | 860 | 350 | 9 / 10 |
| `uncontracted-result-unguarded` | 859 | 368 | 768 | 328 | 1 / 1 |
| `guarded-other` | 674 | 390 | 624 | 344 | 16 / 1 |
| `contracted-result-unguarded` | 587 | 206 | 482 | 167 | 1 / 1 |
| `address-taken` | 284 | 112 | 275 | 110 | 0 / 0 |
| `statement-expression-unguarded` | 98 | 59 | 38 | 26 | 2 / 3 |
| `field-store-owner` | 56 | 36 | 48 | 33 | 0 / 0 |
| **K5 total** | **6,823** | **1,654** | **5,754** | **1,452** | **43 / 29** (bundle) |

(The bundle column counts rows whose every blocker is in some K5 group; rows
are unit configurations, as the report counts them. 178 leaves of the 11-24
run match no census leaf; one row, `context_get_head`, is alias-only with
them: a loop cursor over a call result's member.)

**The 43 frozen functions.** 17 are now `Safe`, all through
AG2-UNRESOLVED-ALIAS (rule 62): the 15 `cmt_decode_msgpack.c` `unpack_*`
callbacks (S1, a formal copied into a local), `append_summary_metric_value`
(S3, a literal) and `LZ4F_optimalBSID` (S2, an enumerator). The other 26 are
still blocked by K5 leaves alone, with the same blocker as on 11-07; three
rows joined them (`LZ4F_getBlockSize`, `LZ4F_headerSize`,
`rd_kafka_anyconf_warn_deprecated`), for **29 rows**.

**What earlier cards refused on purpose** (nothing below is re-admitted):

- AG2-UNRESOLVED-ALIAS (rule 62) reads copies into automatic locals of a
  literal, an enumerator or a read-only unbound formal. It refused a
  file-scope object (S7: `r = rd_kafka_global_cnt;`, and so the decay of
  `rd_kafka_properties`) on rule 54's boundary (`AG2-STATIC-OBJECT-COPY`), an
  enumerator or literal into a *formal* (`LZ4F_getBlockSize`,
  `AG2-LITERAL-OR-CONSTANT-INTO-FORMAL`), a copy of a formal that is returned,
  stored, stepped or handed on (W4's binding signal), and a release of a
  `return_none` value (`__qcore_nested_value_N`). It filed the S4/S5 locals
  bound from a member read, a call or a loop cursor under
  `W47-ALIAS-FLOW-RESIDUALS` without a soundness objection: rule 62 simply
  does not read a value from a member. That is this card's shape.
- AG3-CONDITION-RETURN-CALLS kept W45's decision that an **interior address**
  (`&p->f`, an array member's decay) returned needs a derived result
  (`W45-RETURN-POINTER-RESULT-READS`, `AG3-RETURN-MEMBER-ADDRESS-DERIVED`);
  W45's own discharge says "a load through an untracked base [lowers] to an
  exit that escapes nothing". This card returns loads only.
- Rule 10b keeps the identity refusal of a null test on an unknown binding;
  rule 10a/11d's path premise reads a refused path as possibly handing a root
  to a custody position; rule 5j refuses an interior address returned; rule
  5l reads the intrusive-list `container_of`; rule 37 already gives
  `rd_calloc` and `rd_strdup` checked non-null results (W47-ALIAS-NONNULL's
  largest callees). This card narrows 10b for one binding shape (§3) and
  touches none of the others.

**Yield.** 29 rows remain, so the bundle is worth a card. By shape (the 29
rows and `context_get_head`, each row's leaves read against its body at
`046accf6`):

| Shape | Rows |
|---|---:|
| A pointer local loaded from a formal's member or element, null-tested or returned | 11 |
| ... of which the root is a call result (`fetch_metadata_kvlist_key`, `context_get_head`) or mentioned inside `likely()` (`rd_kafka_timers_next`) | 3 |
| A loaded dereference in a loop with no test (`pack_header`) | 1 |
| An interior address (`&ht->chain[i]`, `&s->iov[n]` then stepped) | 2 |
| `container_of` statement expressions (two release the element) | 3 |
| Non-pointer names: a static `int`, an enumerator into a formal, an automatic `unsigned char[16]` store, a `static const size_t[]` read, a `struct stat` copy, a descriptor formal closed | 6 |
| A file-scope array's decay walked by a cursor | 2 |
| Call results released, published or dereferenced (`mk_fifo_queue_get`, `mk_sched_get_thread_conf`, `mk_mem_alloc`, `rd_kafka_topic_partition_get_private` stored through), a formal handed on (`LZ4F_headerSize`) | 5 |

## 2. The shape this card reads, and why the old answer refused it

P2 (W46-ACCESS-PATH-STATEMENTS) lowers `L = R->F` on a root no statement
binds to a borrowed, target-free alias: no target, `unknown`, possibly null.
Every read through that alias is already admitted: rule 14n reads it
non-null at a dereference under a named `null-deref:` premise, rule 11d reads
it as an untracked root once it is known non-null. So
`n = r->next; return n->count;` was `Safe` at base, while the guarded
`n = r->next; if (n == 0) return -1; return n->count;` was `Unknown`:

* the **null test** took rule 10b's arm for an unknown binding
  (`refine_pointer_null` → `PointerRefinement::Unknown`): the binding is
  narrowed, `UnknownAlias n` is reported and both edges are marked
  unmodelled, so rule 11d's path premise (4) then refused every later read
  through a loaded root (`request_data_point_count`'s lines 538 and 544);
* a **`return n;`** escaped the unknown binding and reported it, although
  `return r->next;` is rule 10k (2)'s exit that escapes nothing.

Of the 11 loaded-local rows of §1, 8 have only such tests and returns, rooted
at a formal that is only read.

## 3. Gateway rule 77

[Rule text](../../agents/rules/77.md); `crates/qcore-c-gateway/src/borrowed_path_local.rs`
(the reading and the tables) and `legacy_borrowed_path_local.rs` (the parser
and transfer adapter).

**The reading** (Clang's AST of the function, asked only where a null test or
a `return` names a local the declaration reader does not call a scalar):

* *Roots*: a single-level pointer formal, declared once, spelled by no local,
  that rule 5b's full return premise answers for (custody closure never
  bound and not named by `may_bind`), and whose every reference is a null
  test, a dereference base, a call actual or a `return` (never written,
  rebound, copied or address-taken).
* *Locals*, a greatest fixed point over rule 62's automatic locals of
  object-pointer type: every write (`=` or initializer), through parentheses
  and `NoOp`/`BitCast` casts, is a null pointer constant, a string literal, a
  copy of an admitted local or a **load** (the lvalue conversion of an
  object-pointer member or element place whose base reaches an admitted root
  or local through loads, `.` members, subscripts, a member array's decay as
  a subscript base, or `(&PLACE)->`), with no call, assignment, step or
  statement expression inside; every reference is a null test, a dereference
  base, a call actual, a `return`, its own rebinding or a copy into an
  admitted local.

**The transfer.** A `BranchOnNull` of an admitted local whose incoming
binding has no target, the identity derivation and an unwidened state (and
which is no status or cell witness) is `Refined` where rule 10b's arm said
`Unknown`: the same narrowing, no report, no unmodelled edge. An
`EscapeIfOwned` of such a local escapes nothing.

**Why it is sound.** By induction over a run, an admitted local holds null,
a literal's address or a value loaded from an object reachable from an
admitted root through loads, which is the value of the load itself there
(`every_admitted_local_holds_an_untracked_value`; a cursor that loads from
itself is one more admitted write). P2's premise, that a never-bound root
reaches no tracked place, is what makes that load target-free in the first
place; this rule adds none. The decided null test moves nothing but its own
binding, exactly as the old arm did (`the_admitted_guard_leaves_the_old_state`,
`the_guard_touches_no_obligation`, `the_guard_narrows_only_its_binding`);
rule 10b's worry, that non-nullness does not establish a *lookup's* identity,
does not arise: a value from a call withholds the local, and a binding with
no target names no identity. The return escapes nothing, which is all the old
arm escaped too (`the_old_return_escaped_nothing_either`); only the report
goes. A release, a transfer or a custody-taking position still finds no
target and refuses (`a_release_of_an_untracked_local_still_refuses`; fixtures
"released after the test", "copied, then the copy released", "a cursor that
frees").

**Why the closure is asked of roots only.** The never-bound closure binds
every name a written value mentions: `c = a->kids[i]` with `i++` in the loop
header binds `c`. What makes a local's value untracked is read directly
(every write a load, null, literal or admitted copy), and what a custody
position does with it is decided on its binding, which has no target. The
roots keep the full premise, because the load's untrackedness and rule 10k
(2)'s return answer rest on it.

## 4. Refused shapes and their named reasons

| Shape (rows alone of §1) | Why it stays refused | Owner |
|---|---|---|
| the loaded base is a call result (`fetch_metadata_kvlist_key`'s `entry_variant = cfl_kvlist_fetch(...)`, `context_get_head`'s `thctx = local_thread_context()`) | the root is no formal: what the call returned is the callee's account (`return_none` or borrowed), not P2's never-bound premise | `W47-ALIAS-FLOW-RESIDUALS` (amended) |
| the root is mentioned inside `likely()` (`rd_kafka_timers_next`: `__builtin_expect(((rtmr = TAILQ_FIRST(&rkts->rkts_timers)) != NULL), 1)`) | rule 5b's closure binds every variable of an uncontracted builtin's actual, so `rkts` is no root | `W47-ALIAS-FLOW-RESIDUALS` (amended) |
| a dereference with no test or return (`pack_header`: `summary->quantiles[index]` in a loop after a correlated branch) | not a decision on the name; a loop-carried, possibly-null binding the dereference reads | `W47-ALIAS-FLOW-RESIDUALS` (amended) |
| an interior address held or returned (`hc = &ht->chain[i]; return hc;`, `iov = &s->iov[n]; iov++`) | an address is derived from its base (rule 5j, W45, AG3) | `AG3-RETURN-MEMBER-ADDRESS-DERIVED` |
| `container_of` over an iteration cursor (`metrics_check_label_value_existence`; `cio_chunk_close_stream` and `mk_fifo_queue_destroy_all` release the element) | a statement expression; rule 5l (2) reads it as unbound, and releasing a listed element needs rule 10f's unlink | `W47-ALIAS-FLOW-RESIDUALS` (amended) |
| a file-scope array's decay walked by a cursor (`prop = rd_kafka_properties; prop++`) and a static `int` copied (`rd_kafka_global_cnt`) | rule 54's and rule 62's boundary (AG2's S7) | `AG2-STATIC-OBJECT-COPY` (amended) |
| an enumerator into a formal (`LZ4F_getBlockSize`) | AG2's refusal: a formal's binding is entry custody | `AG2-LITERAL-OR-CONSTANT-INTO-FORMAL` |
| non-pointer names: an automatic `unsigned char bytes[16]` store (`rd_kafka_Uuid_str`), a `static const size_t op2size[]` read (`rd_kafka_op_new0`), a `struct stat` copy (`mk_file_get_info`) | not a pointer local; a scalar or record object read as an identity by the lowering, outside this rule | `W47-ALIAS-TYPED-SCALAR-NAMES` (amended) |
| a descriptor formal closed (`net_plain_close`: `return close(socket_fd);`) | a release of the caller's descriptor needs a contract or a W4 summary: a correct refusal | `W47-ALIAS-CALLEE-BORROWED-CLOSURE` |
| call results released, published or stored through (`mk_fifo_queue_id_destroy`, `mk_sched_event_free`, `rd_kafka_topic_partition_new_with_topic_id`), a formal handed on (`LZ4F_headerSize`) | custody moves this rule does not read | `W47-ALIAS-FLOW-RESIDUALS` (amended) |
| `mk_rconf_section_entry_add`: `new = mk_mem_alloc(...)` dereferenced unguarded | Monkey's `mk_mem_alloc` returns `NULL` on failure: a real obligation | `W47-ALIAS-NONNULL-ALLOCATOR-RESULTS` (amended) |

`W47-ALIAS-NONNULL-ALLOCATOR-RESULTS`: rule 37 already gave `rd_calloc` and
`rd_strdup` checked non-null results; the remaining wrappers free no row
alone (the one `contracted-result-unguarded` row alone is `mk_mem_alloc`'s,
a real obligation), so no provider contract is changed by this card.
