# CELL-GUARD-RESULT-RANGE-CHECKED

Card brief: `private/integrator-briefs/r8-common.md` (round 8, 2026-11-03).
Base `1d94f0995`, baseline `e3050fe46`. The DISCHARGE of `named_followups`
entry CELL-GUARD-RESULT-RANGE-CHECKED, registered by the census card
[CELL-GUARD-RESULT-RANGE](../cell-guard-result-range/README.md).

## The decisions (verbatim)

> **USER DECISION 2026-11-03**: "Yes, as recommended". This chose Q1 (a), the
> libc result sets as flexible libc contracts under `libc-c-standard-abi`, and
> Q2 (ii), the range derived at `quirin-audit pin --bounds` time from the
> configured backend's body chain.
>
> **USER DECISION 2026-11-03**, on (a): "This should be needed to use here."
>
> **USER DIRECTION 2026-11-03**: "If there is no contract candidates for
> applying this combination we must write down them first."
>
> **USER DECISION 2026-11-03** (the form): "C: a new result-set form that
> allows unions, such as result_set(-1 | [0, MAX]). It is the only option that
> can also state open's and timerfd_create's results (needed by
> RULE-38-VALIDITY-READING-RESULT-SET). It is also the largest grammar change.
> I choose this because we must not loosen the custody of the contracts if we
> cannot select other options."
>
> **USER DIRECTION 2026-11-03**: "Also, we need to use mathematical symbols on
> them and we can use XOR or AND or OR with the normal symbols of programming
> languages."
>
> **USER DIRECTION 2026-11-03**: "The Fluent Bit's caller tests sometimes check
> the incorrect assertions so we need to pay attention for the assertions and
> actual returning values of Fluent Bit functions."

The card first stopped once: no existing form could state a flexible libc
result set. It reported options A (a flexible mode in bounds v1), B (a new
resource v1 clause) and C (a dedicated union form). The user chose C.

## 1. The C library contracts: what existed, what was written

The derivation reads only the result sets of C library calls whose results
reach a `return`. For each one, `quirin-provider-libc` was searched; it has no
include or inherit parents (no dependencies).

| Call | Read by | Existing declaration | States the set? | This card |
|---|---|---|---|---|
| `pipe` | the channel chain, every backend but libevent | none (declined for custody, `unistd.qv`: no out slot) | no | `result_set(-1 \| 0)`, POSIX |
| `epoll_ctl` | epoll `_mk_event_add` | none | no | `result_set(-1 \| 0)`, Linux epoll_ctl(2) |
| `open` | the timer chain | `unistd.qv`, resource v1 (`resource_return_owned(sentinel)`), unchanged | only as custody | `result_set(-1 \| [0, MAX])`, POSIX |
| `timerfd_create` | epoll's timer arm | none | no | `result_set(-1 \| [0, MAX])`, Linux timerfd_create(2) |
| `kevent` | kqueue only | none (no audited unit's header declares it) | no | not declared: no audited build configures kqueue |
| `evutil_socketpair` | libevent; its result is tested and a constant returned | libevent provider (legacy form) | no | not needed: its result never reaches a `return` |
| `clock_gettime`, `timerfd_settime` | tested, then a constant returned | `clock_gettime` custody only | no | not needed |

The four declarations are in
[`libc/result_sets.qv`](../../../providers/quirin-provider-libc/contracts/libc/result_sets.qv),
the libc crate's first bounds view. Each one states a value set and nothing
else. They were committed in their own step, `86d93cf98`, with
`providers/contracts.py sync` (load path, inventory lock) and the libc
`bounds-contracts.lock` (four flexible entries). This was before the
prediction (`74db04a0c`) and before the derivation.

## 2. The form (C09.4, grammar G05j)

`ensures normal result_set(SET);` is a bounds contract v1 clause:

* Literals stand for themselves.
* `[a, b]` is a closed interval.
* `MIN` and `MAX` are the bounds of the declared result type.
* `|` is union (∪), `&` is intersection (∩) and `^` is symmetric difference
  (△).
* Precedence is C's: `&` binds tightest, then `^`, then `|`. All three are
  left associative, and parentheses group.

Choices made inside C, and why:

* **ASCII only in source.** The mathematical symbols are documentation
  notation. A non-ASCII operator is a lexical error. This follows the
  user's "no aliases" decisions of 2026-10-11 and 2026-10-16, and the source
  lexer is ASCII. Every spelling of one set loads to the same set. Its
  canonical serialization is the clause's canonical text and digest:
  maximal intervals in increasing order, `|`-joined, singletons as
  literals, `MIN`/`MAX` at the type bounds. So `-1 | [0, MAX]` serializes
  as `[-1, MAX]`. The pin digest is over the text as written, as for every
  bounds clause.
* **No collision.** The lexer already has `|`, `&` and `^`. The operator
  gate (`Q038`) now admits them inside a `result_set(…)` clause and nowhere
  else. This follows the precedent of rule 12r's `quant checks` connectives.
* **Refusals at load.** Empty or contradictory operands and sets are
  refused (`result-set-empty`), as are a literal outside the type
  (`result-set-outside-type`) and any other shape (`result-set-shape`). An
  operand that denotes the empty set is refused too, not only an empty
  whole.
* **Libc only, flexible.** A `result_set` stands only on a declaration that
  carries `requires bounds_revision(flexible);` and states nothing else. Such
  a declaration is admitted only from `quirin-provider-libc`; a route holds
  one from any other crate. It is rule 12r's one flexible bounds
  declaration (12r amended), and its lock lists it as a flexible entry. No
  custody clause hosts it, and none changed: `pipe` stays undeclared for
  custody, and `open`'s resource v1 declaration is byte-identical. The
  `open` result set stands beside it as a registered `null_deref_beside`
  relation.
* **`nonnull_result_range` stays separate.** There is no alias and no
  migration. A pinned `nonnull_result_range(LO, HI)` is read as the set
  `[LO, HI]` (`BoundsFunctionContract::result_values`). One declaration
  states at most one of the two. The derived Monkey range below uses the
  existing pinned form, as the DISCHARGE names it.
* **Binding.** A C library set binds a call only where the unit defines no
  function of that name, and only where the call's C type is the declared
  result type.

Code:

* `qcore_core::result_set`: the set algebra, the parser and the class
  decision.
* `qcore-contract`'s `bounds_contract.rs`: the loader.
* `qcore-frontend`'s `pipe.rs`: the operator gate.

## 3. The pin-derived range (rule 38e)

`mk_event_channel_create` declares `nonnull_result_range(-1, 0)` in the
libevent provider's new bounds view,
[`monkey/result_range.qv`](../../../providers/qcore-provider-libevent/contracts/monkey/result_range.qv),
beside its unchanged resource declaration.

`quirin-audit pin --bounds --derive-results --compile-database FILE` does the
derivation:

* It finds the declaration's one defining file in the reviewed tree and
  reads its configured compile command.
* It derives the body chain's result set through Clang's AST of that unit,
  using only the C library's result sets:
  * a constant is itself;
  * an in-unit helper is its own derived set;
  * a C library call is its declared set;
  * a local whose address is never taken and whose every write is a plain
    assignment is the union of what it is assigned (C11 6.3.2.1p2);
  * anything else is the type's range.
* The derived set must lie inside the declared range; otherwise nothing is
  pinned.

It ran at all three reviewed trees with Fluent Bit's own builds (epoll):

| Revision | Compile database | Derived | Helpers | Rests on |
|---|---|---|---|---|
| 046accf6 | system `fluent-bit-clang-production` | `[-1, 0]` | `_mk_event_add`, `_mk_event_channel_create` | `epoll_ctl`, `pipe` |
| 34155955 | slice `fluent-bit-clang-production` | `[-1, 0]` | the same | the same |
| 66910c10 | corpus `fluent-bit-clang-source/build` | `[-1, 0]` | the same | the same |

From the bodies alone the result set is every `int`
(`result_set_derivation.rs`, `from_the_bodies_alone_the_chain_returns_every_int`).

The lock entry records the derivation: the set, the helpers and each C library
declaration with its text digest. Every definition site of each helper joins
every identity's sites. 046accf6 and 34155955 share one identity; 66910c10
has its own, because the poll backend differs there.

At audit time a derived range is in force only when all of these hold: its
pin is in force, which covers the helper sites; and every C library set it
rested on is loaded, listed and unchanged. Otherwise the range is held
(`contract-changed`). A pinned range the lock never derived is `unchecked`.

Re-pinning a derived entry without `--derive-results` refuses. `pin --check`
and `--verify-contracts` read the derivation.

The route's table (`bounds_selection::result_range_table`) reaches route G
through the resource environment (`ResourceContracts::result_ranges`). That
field is omitted when empty, and empty unless the bounds or null property is
selected.

## 4. Rule 38's reading (rule 38e)

The reading applies to a creation whose `#fresh` class is a status guard
(`#result == 0` / `!= 0`) and whose callee's derived set `S` is in force. A
branch that is exactly `ret OP K` (a null test keeps its own reading) lowers
to `BranchOnResult`. On each edge it decides the class exactly when, on `S`,
the test holds where the class holds, or where it fails
(`qcore_core::result_set::test_class`, from `compared_class` in
`legacy_descriptor_cells.rs`). Everything else reads nothing.

Without a set in force, no name is a result witness and every lowering is
byte-identical. The decision lives in `qcore-core`, not the rule-38 package,
because a rule package depends on the rule API only (`workspace_layering`).
The registry lists `QCore.ResultSet` under rule 38.

Fixtures (`crates/qcore-c-gateway/tests/cell_guard_result_range.rs`) assert
the full diagnostic vector of a channel creator caller:

| Test | Set | Outcome |
|---|---|---|
| `ret != 0` | none, or in force | `Safe` |
| `ret == -1`, `-1 == ret`, `ret < 0`, `ret <= -1` | in force | `Safe` |
| `ret == -1`, `ret < 0` | none, `unchecked`, held | `Unknown`, the same 7 leaves (`descriptor-cell-escapes` ×6, `descriptor-cell-guard-undecided`) |
| `ret == -1` | held after a C library set changed (a refuted derivation is never pinned) | the same 7 leaves |
| `ret == -2`, `ret != -2` (a constant the set excludes), `ret > 0`, `ret >= -1` (misclassify 0 or -1) | in force | the same 7 leaves, never smoothed over |

The derivation fixtures (`result_set_derivation.rs`, 4 tests) cover:

* the epoll chain;
* the bodies alone;
* an unbound set (the unit defines `pipe`);
* a mistyped call;
* locals with other writes.

## 5. The null property's bound (part of (a))

The range check of rule 15q now reads a call result through a declared set in
force: a C library `result_set`, or a pinned range not held and not refuted
here. It also reads through an in-unit helper's derived set (`range.rs`,
`Deriver`; `summary` and `evidence` are now set-based).

A body check validates a declared range when the derived set lies inside it,
and refutes it when a `return` gives a constant outside it, as before.

## 6. Lean

`QCore.ResultSet` (`proof/QCore/ResultSet.lean`, audited in
`proof/AuditResultSet.lean`, standard axioms only) proves:

* **The operators' meaning:** `denote_or`, `denote_and`, `denote_xor`,
  `denote_in_universe`.
* **The class equivalence under a set:** `holds_true_edge`,
  `holds_false_edge`, `fails_true_edge`, `fails_false_edge`.
* **Narrowing keeps a decision:** `narrowing_keeps_holds`,
  `narrowing_keeps_fails`.
* **An out-of-force set never decides beyond an exact-on-every-value test:**
  `out_of_force_reads_the_universe`,
  `a_decision_out_of_force_is_exact_on_every_value`.

The vectors `setVectors` (every operator, precedence, every refusal),
`memberVectors` (the interval implementation against the meaning) and
`classVectors` are checked by `decide +kernel`. They are bound entry for entry
to the Rust `SET_VECTORS`, `MEMBER_VECTORS` and `CLASS_VECTORS` by
`crates/qcore-core/tests/result_set.rs`, which reads the Lean text.

## 7. The caller-test census, rerun

`census.py --check` at 66910c10, 046accf6, 34155955 and the corpus's Monkey
exited 0 with 0 problems. `census.json` is unchanged, sha256 `4c22ec16…`
([census-rerun.json](census-rerun.json)). Its C library result sets are now
each a declared contract.

Immediate tests with the C library's sets:

| Tree | Exact | Redundant | Wrong | Untested |
|---|---|---|---|---|
| 046accf6 | 23 | 13 | 1 | 1 |
| 34155955 | 23 | 13 | 1 | 1 |
| 66910c10 | 19 | 11 | 1 | 1 |
| Monkey | 2 | 2 | 1 | 1 |

Stored-result tests: 20 wrong per Fluent Bit tree, 5 exact, 4 outside; 3 wrong
in Monkey.

**Every wrong site** (046accf6; the earlier trees are the same sites):

* **Immediate result:** Monkey `mk_server_worker_loop`, the timer created at
  mk_server.c:502 and tested `timeout_fd > 0` at :585.
* **Stored results:**
  * mk_server.c:585, :630 (`signal_channel_w <= 0`) and :648
    (`lib_ch_start[1] <= 0`). These are Monkey's three, the same in the
    corpus's Monkey.
  * flb_config.c:617, :625, :627 (`ch_manager[0]`, `ch_notif[0]`,
    `ch_notif[1] > 0`).
  * flb_engine.c:1034, :1048 (`ch_notif[1] <= 0`), :1497
    (`shutdown_fd <= 0`) and :1563 (`shutdown_fd > 0`).
  * flb_input.c:1311 and :1315 (`ch_events[0|1] > 0`), and :3145
    (`coll->fd_timer > 0`).
  * flb_input_thread.c:210, :213, :218 and :221.
  * flb_output.c:615 and :619.
  * flb_sp.c:670 (`task->window.fd > 0`).

These remain candidate defects tracked as CELL-GUARD-DESCRIPTOR-ZERO-READ-AS-ABSENT
and CELL-PROTOCOL-PER-END-GUARDS. Nothing is filed upstream.

Rule 38e reads none of them, and no result set comes from a caller's test.
The nine Fluent Bit `ret == -1` channel tests and Monkey's `ret < 0` are
redundant: they now decide under the derived set. Every one of those
creations is still refused at the access gate first.

## 8. Prediction and measurement

[PREDICTION.md](PREDICTION.md) (`74db04a0c`, before the derivation) predicts
no move on any route. Every named creation keeps `indirect memory access is
outside the resource projection` at the baseline, so rule 38 reads none of
their results. Environment-only changes are expected:

* the null and bounds environments now hold the libc bounds view and the
  derived range;
* the resource environment carries the range table.

Where the prediction could be wrong: the null property's bound (§5) now reads
a C library or helper set. A null row moves only where an in-unit caller
compares such a result with a value the set excludes. The census of the
analysed `pipe`/`epoll_ctl` calls found none. If the measurement shows a null
move, it comes from this reading.

The measurement is the integrator's. The command is in the hand-back.

## 9. The card gate

See [gate.json](gate.json) and the hand-back.

## 10. Follow-ups and observations

* **CELL-GUARD-RESULT-RANGE-CORPUS-MONKEY-IDENTITY** (registered). The corpus
  route evaluates pins over the Fluent Bit tree (`lib/monkey/…`), but its
  Monkey units come from Monkey's standalone libevent build. A range in force
  there therefore rests on the Fluent Bit epoll derivation, not on the
  analysed unit. This is the same property the resource pins already have.
  It moves nothing today: the corpus creators are refused at the access gate.
* **RULE-38-VALIDITY-READING-RESULT-SET** (amended). `open` and
  `timerfd_create` are now declared result sets. What remains is reading the
  timer's validity witness through a derived timer set.
* **Flexible binding reads the result type only.** A C library set binds a
  call by name and the call's C result type, not its parameter types. This
  is a stated premise.
