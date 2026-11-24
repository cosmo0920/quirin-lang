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

## 5. Fixtures and Lean

[`tests/k5_local_origin_aliases.rs`](../../../crates/qcore-c-gateway/tests/k5_local_origin_aliases.rs)
(9 tests; every `Safe` rechecks its certificate). Admitted (`Unknown` on the
local at base, `Safe` at tip): a test of a local assigned in the condition, an
initializer through `.` members and a `void *` member, a self-loading cursor,
two nested loaded locals under a stepped index, a literal rebinding after the
test, a test then a `Borrowed` actual, three returns (after a use, joined with
null, a cursor returned from a loop), and a null stored into the loaded member
after the load; each beside the direct-path reading that is `Safe` at base
and tip. Near misses, unchanged from base (each answer was taken at base too):
a release of the local, of a copy of it, and in a freeing cursor loop; a root
released, rebound, handed to an uncontracted callee, or a call result; an
acquisition on one path (the `Leak` stays), an address handed out, a step, an
interior address returned, a shadowed spelling and a file-scope array's decay;
and a leak on the tested path, already reported at base, still reported
(`Unsafe`), with its released twin `Safe`. Unit tests
(`tests/unit/borrowed_path_local.rs`, 4): the decision, value and binding
tables against their vectors, each bound to the Lean lists. Rule 62's and 54's
fixture files (`ag2_unresolved_alias` 14, `thread_local_custody_free_read` 9)
pass unchanged.

[`QCore.BorrowedPathLocal`](../../../proof/QCore/BorrowedPathLocal.lean),
audited by `proof/AuditBorrowedPathLocal.lean` (19 theorems; 4 use no axiom,
14 `propext`, 1 `propext, Classical.choice, Quot.sound`; no placeholder):
`decision_vectors_hold`, `a_root_is_admitted_exactly_on_a_read`,
`a_step_or_address_withholds_the_local`, `value_vectors_hold`,
`binding_vectors_hold`, `a_widened_state_is_never_untracked`,
`an_other_value_withholds`, `eval_untracked`,
`an_admitted_write_keeps_every_local_untracked`,
`every_admitted_local_holds_an_untracked_value`,
`the_admitted_guard_leaves_the_old_state`, `the_guard_touches_no_obligation`,
`the_guard_narrows_only_its_binding`, `the_admitted_guard_reports_nothing`,
`a_withheld_guard_is_the_old_guard`, `a_tracked_binding_keeps_the_old_guard`,
`returning_an_untracked_local_escapes_nothing`,
`the_old_return_escaped_nothing_either`,
`a_release_of_an_untracked_local_still_refuses`.

## 6. Unit-scoped reproduction of `current`

[scripts/run_repro.sh](scripts/run_repro.sh) runs the user's `current
--monkey-linked` argv on the compile database filtered
([scripts/filter_db.py](scripts/filter_db.py)) to AG2's 48 units
([artifacts/repro-units.txt](artifacts/repro-units.txt): every unit of the
29 rows, plus the units the route requires) and the full Monkey database: 49
configurations, 1,894 rows, through `gate_slot.py audit` (`--jobs 3`, peak
2.1 GB, about 12 minutes a side;
[receipts](artifacts/repro-base.slot.json), [tip](artifacts/repro-tip.slot.json)).
Both sides ran one binary (sha256 `11d957a2…`), built from the working tree
committed as `43d93dcfc` while it still held a test-only switch,
`K5_PROTOTYPE_OFF`, that returned the empty reading before the AST was asked
(so the switched side takes exactly the base code paths); it was set for the
base side, unset for the tip side, and removed before the commit, which differs
from the measured tree otherwise only in doc comments and one test helper.
**Fidelity:** the base side equals the user's 11-24 report in status on all
1,894 rows; 8 rows differ only by run-wide diagnostics a unit subset cannot
see, as in AG2's reproduction ([artifacts/repro-fidelity.json](artifacts/repro-fidelity.json)).

| Measure | Base | Tip |
|---|---:|---:|
| rows Safe / Unknown / Unsafe | 386 / 1,506 / 2 | **394 / 1,498 / 2** |
| `Unknown -> Safe` | | 8 |
| other moves | | 0 |
| `unknown_alias` leaves removed | | 19 |
| diagnostics added | | 0 |
| rows with changed content, status unchanged | | 7 (all Unknown, leaves removed only) |

The 8 moves are exactly the 8 declared in [PREDICTION.md](PREDICTION.md);
nothing unpredicted moved, no row left `Safe`, no `Unsafe` changed
([artifacts/repro-compare.json](artifacts/repro-compare.json)). The 7
content-changed rows (`cio_file_open`, `cmt_encode_opentelemetry_split_payload`,
`add_item_to_array`, `rd_kafka_share_toppar_enq_error`,
`rd_kafka_topic_partition_get_topic_id`, `rd_kafka_sasl_oauthbearer_close`,
`rd_kafka_sasl_oauthbearer_ready`) lose loaded-local test or return leaves and
keep other blockers. On the full run, the same rows of these units move; the
other units' rows with such leaves are not measured here.

## 7. Shared-file hook points

New logic is in `borrowed_path_local.rs` and `legacy_borrowed_path_local.rs`.
Shared files carry only blocks between `K5-LOCAL-ORIGIN-ALIASES hook (rule
77): begin` and `end` comments:

| File | Hook |
|---|---|
| `lib.rs` | `pub mod borrowed_path_local;` after `custody_free_copy` |
| `legacy_c.rs` | the `#[path = "legacy_borrowed_path_local.rs"] mod borrowed_path_local_route;` declaration after rule 62's; the `borrowed_path` field of `TraversalNames` (serde default, skipped when empty); the call `parser.borrowed_path_locals(&cfg, &scalars)` after rule 62's and the field in the struct literal there |
| `legacy_transfer.rs` | in the work list, `guard_refinement` applied to `refine_edge`'s answer before its `match`; one `EscapeIfOwned` arm before the generic one in `transfer_base` |
| `legacy_leak_witness.rs` | the same `guard_refinement` on the census's `refine_edge` answer |
| `custody_free_copy.rs` (rule 62's module) | `type_of`, `Visit`, `visit` and `classify` widened to `pub(crate)`; no behaviour change |

No change to `legacy_cfg_builder.rs`, `legacy_classify.rs`,
`typed_access_path.rs` or `typed_field_access.rs`.

## 8. Gate

Not run, on the user's instruction of 2026-11-24 relayed by the coordinator
(verbatim): "We don't need to run the each of K-related gates. We must run the
merged K-related tip's gate." A scoped gate started on `43d93dcfc`
(`python3 scripts/gate_scope.py --base 3ca85c7a6 --out private/gate-1`) was
stopped through its own task handle after 3 h 50 min while it still waited for
the Rust-suite slot; nothing of it is evidence (`private/gate-1/STOPPED.txt`).
Targeted checks run on this tree: `cargo test -p qcore-c-gateway --test
k5_local_origin_aliases --test ag2_unresolved_alias --test
thread_local_custody_free_read` (exit 0; 9, 14, 9 passed),
`cargo test -p qcore-c-gateway --lib borrowed_path_local` (exit 0, 4 passed),
`cargo clippy -p qcore-c-gateway --all-targets -- -D warnings` (exit 0, after
one fix), `cargo fmt --all` (clean), `lake -d proof build
QCore.BorrowedPathLocal` and the audit file under `lake env lean
-DwarningAsError=true` (exit 0), `gen_rule_docs.py --check` and
`gen_lean_roots.py --check` (exit 0). The full gate on the merged K tip is the
arbiter, including for pinned expectations elsewhere in the suite that this
rule could flip (none was found by the targeted runs).

## 9. Records

- [PREDICTION.md](PREDICTION.md), committed before the semantic commit, with
  its `measure-prediction` block (8 declared moves on corpus and twelve-home).
- [integration-amendments.json](integration-amendments.json): amends
  `W47-ALIAS-FLOW-RESIDUALS`, `W47-ALIAS-TYPED-SCALAR-NAMES`,
  `W47-ALIAS-NONNULL-ALLOCATOR-RESULTS`, `AG2-STATIC-OBJECT-COPY`,
  `CG17-CONTAINER-OF-LIST-LOOKUPS`, `AG3-RETURN-MEMBER-ADDRESS-DERIVED` and
  `W47-ALIAS-CALLEE-BORROWED-CLOSURE`; registers `K5-CALL-RESULT-BASE-LOADS`,
  `K5-BUILTIN-EXPECT-ROOT-CLOSURE` and `K5-LOOP-CARRIED-LOADED-DEREFERENCE`.
- Measurement command (the integrator's): `quirin-audit measure --side
  base=rev:3ca85c7a6 --side tip=rev:<tip> --declared
  docs/reports/k5-local-origin-aliases/PREDICTION.md` on corpus, twelve-home,
  system and slice, then the user's `current --monkey-linked` rerun.
