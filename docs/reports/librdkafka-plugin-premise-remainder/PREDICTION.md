# LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER: prediction (frozen before implementation)

Base: local `main` `711287771`. Written after the census
([`scripts/member_reach.py`](scripts/member_reach.py),
[artifact](artifacts/member-census.json)) and before any analyser, loader,
contract or Lean change.

## 1. User decisions (2026-10-15, verbatim)

1. **Null views: "Only nonnull_member* need it".** A `quant null-deref` fact
   does not depend on which callback runs, so it needs no premise. Only
   `nonnull_member_stable` / `nonnull_member_scope` declarations must carry
   it, through a premise clause added to just those forms. A stray callback
   store could falsify them.
2. **Crash path: "Yes, list it".** `rd_kafka_crash` (`memory.qv`) and
   `rd_kafka_dump0` (`kafka_mem.qv`) name the premise.
   `rd_kafka_toppar_get_avail` keeps it. No verdict should change; the
   dependent Safe rows gain the premise.
3. **Model contract: "Add premise support".** Extend the Quirin model-contract
   format so that `queue_destructor.qv`'s `rd_kafka_q_destroy_final` can state
   the premise. State it there.

## 2. The member census and its rule

The census preprocesses librdkafka's 94 C units at `66910c10` and `046accf6`
with LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED's `preprocess.py` (each with its own
recorded command; 94 units, 0 nonzero at both) and builds that card's
over-approximating call graph (`reach.py`, imported unchanged). All 72
member clauses have identical results at both revisions.

**Rule.** A member declaration names `rk_no_plugin_library` when a callback
outside the run could falsify its fact:

* **(a)** a function that may store into `TAG.FIELD` (an assignment, compound
  update or initializer naming the member, or a fill or copy in a body that
  names `struct TAG`, read textually from the preprocessed bodies) can run
  inside a plugin-registered callback: it is reachable on the graph from a
  function with external linkage, which a callback a plugin library
  registers may call (the `RD_EXPORT` functions of `rdkafka.h` are listed
  first as public entries); or
* **(b)** a plugin-registered callback can run while the fact is relied on:
  the declared function's own body reaches an rk_conf callback or an
  interceptor call (`rk-conf`), or an unresolved indirect call
  (`indirect`).

A store an analysed unit makes is checked by the run's fold whatever context
runs it (rules 14t, 14x). What no check covers is a store made by code
outside the run, which only the premise excludes. librdkafka runs its
callbacks on its own threads, and a stable fact persists across the whole run
(rule 14y), so (a) alone decides. (b) is recorded for each declaration.

**Result.** `rd_kafka_broker_s.rkb_rk` has 14 functions that may store into it
(2 by assignment or initializer, 12 by a possible whole-object write). Both
assignment sites have external linkage. `rd_kafka_broker_add` is reached
from the public `rd_kafka_new`, so (a) holds for the member.

| Form | Declarations | (a) | (b) `rk-conf` | (b) `indirect` | (b) `none` | Premise |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| `nonnull_member_stable` | 1 (`rd_kafka_broker_add`) | 1 | 1 | 0 | 0 | **all** |
| `nonnull_member_scope` | 66 (`rkb_rk` of `rkb`) | 66 | 62 | 2 | 2 | **all** |
| `nonnull_member` (rule 14t, for the record) | 4 declarations, 5 clauses | 5 | 5 | 0 | 0 | not admitted (decision 1) |

So **67 declarations** in `null_deref.qv` (1 stable, 66 scope) will carry the
premise. The two scope bodies with (b) `none` are `rd_kafka_broker_needs_connection`
and `rd_kafka_buf_new_request0`. The two with (b) `indirect` are
`rd_kafka_sasl_broker_init` and `rd_kafka_sasl_broker_term`. All four are
premised by (a).

No other crate declares `nonnull_member_stable` or `nonnull_member_scope`
(fluent-bit, cfl and cmetrics have none on this base). The parallel card
FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE adds some in the fluent-bit crate.
Whichever card lands second applies this rule to them.

The rule says rule 14t's five plain `nonnull_member` clauses would need the
premise too (each member has an externally reachable store). Decision 1
admits the clause only beside the two scoped forms, so they stay unpremised.
They are registered as a follow-up for the user, with this census.

## 3. Crash path and model contract

* `memory.qv` `rd_kafka_crash` and `kafka_mem.qv` `rd_kafka_dump0`: the clause
  `requires resource_premise(rk_no_plugin_library, "…")`, byte-identical to
  the 89 in `client.qv`. Both are re-pinned at both revisions.
  `rd_kafka_toppar_get_avail` keeps its premise.
* `queue_destructor.qv` `rd_kafka_q_destroy_final`, a model contract (`resource
  type KafkaQueueOwner`) outside every resource view, pinned by no lock: the
  same clause. The format change is in the contract grammar every module
  parses through, so it is total-or-reject. A `resource_premise` clause
  must be a `requires` of exactly `resource_premise(NAME, "STATEMENT")`,
  with a lowercase name, one plain string and no repeated name in the
  declaration. Anything else rejects the module at parse, in every vocabulary.
  The clause-count pin moves 6 → 7, and `contracts.json`'s digest is
  re-synced.

## 4. Mechanism

* Bounds contract v1 admits `requires resource_premise(NAME, "STATEMENT");`
  only in a declaration that also states `nonnull_member_stable` or
  `nonnull_member_scope`. Elsewhere in a bounds or null view it rejects the
  module. The bounds consumer never reads it (`for_bounds` removes it with
  the null clauses).
* A declaration's premises cover its stable members (row dependency
  `stable:TAG.FIELD`) and its scope entries (row dependency: the function's
  name). Every null row whose dependency set names a premised fact lists the
  premise in a new row field. That includes rows whose dependencies grow
  through the run fold's travel. The fold's `stable_declarations` /
  `scope_declarations` records list their premises.
* No decision reads the premise: verdicts, checks, reasons and every
  pre-existing field are unchanged.

## 5. Predicted outcome

* **Verdicts:** byte-identical on the corpus, system and slice routes, custody
  and null, apart from:
  * the new null-row premise field and premise dependency entries;
  * the fold sections' premise lists;
  * custody contract-dependency records of `rd_kafka_crash` and
    `rd_kafka_dump0`;
  * contract-source and pin digests.
  Any verdict change is a named correction. None is expected.
* **Null rows:** every null row whose `contract_dependencies` names
  `stable:rd_kafka_broker_s.rkb_rk` or one of the 66 scope functions lists
  `rk_no_plugin_library`. They are counted from the BEFORE run as the rows
  with such a dependency. No such row lacks it in AFTER. No null row
  without such a dependency gains it.
* **Custody rows:** every row resting on `rd_kafka_crash` (67 on the
  previous card's BEFORE corpus, 1 Safe) or `rd_kafka_dump0` (3, 2 Safe) lists
  the premise through the sidecar record. Rows already listing it through
  another declaration are unchanged. No roll-up bucket moves.
* **System and slice:** they analyse no librdkafka unit, so no row there rests
  on the premised declarations. Only contract-source identities change.
* **Lean:** a new module stating that a member fact carrying a premise is in
  force only under it, that a row resting on it lists the premise, and that
  listing changes no verdict.
