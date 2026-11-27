# NULL-SAFETY-PROPERTY: prediction (frozen before implementation code)

Base: local `main` at `0085fa346` (= the brief's base). BEFORE, measured with
`scripts/measure-binary.sh before` on the base's release binary
(`private/nsp/bin/before`, sha256 `e33abba0…`): corpus **903 / 9 / 3,922**,
`resource-diagnostics.json` sha256 `24b59d52…` (the brief's figures and CG22's
`a5` digest: reproduced). System and slice are recorded in the report once
their BEFORE runs finish; the prediction below does not depend on them.

## What will be built (in this order)

1. **The property** `c.null.deref.v1`, selected by `QCORE_NULL_PROPERTY` (rows
   in a JSON-lines file beside the run, like `QCORE_BOUNDS_PROPERTY`), a
   route-G consumer of Clang's typed AST in a new gateway module: every `*p`,
   `p->f` and `p[i]` read or write is a check; sources of possibly-NULL are
   allocation results (resource contracts with an owned nullable result),
   nullable and lookup results, uncontracted call results, formals, pointers
   read from memory, globals, NULL literals; guards are NULL tests (`if (p)`,
   `!p`, `p != NULL`, `&&`/`||`, `__builtin_expect`), early exits and
   no-return calls, and `assert(p)` (a guard only when the analysed text still
   holds it, so `NDEBUG` was absent, and then a listed premise). Decision 1's
   dominating dereference: after a real read or write through `B`, `B` (and
   `&B->F`) is non-NULL with the question moved to that dereference, until `B`
   is reassigned; `&B->G`, `sizeof` and other unevaluated operands are not
   dereferences. Verdict: `Safe` only when every check is proved or moved to a
   check that is; `Unsafe` only for a definitely-NULL pointer dereferenced on
   the straight-line prefix from entry; `Unknown` otherwise, with the
   candidates listed. Custody is never read or changed by it.
2. **Custody integration** (decision 1, `CG10-INTERIOR-ALIAS-NULLABILITY`): a
   scalar read or write through a **target-free, possibly-null borrowed alias**
   (what a pointer-valued path read and rule 14b bind) is read non-NULL under a
   named premise `null-deref:<function>:<line>:<variable>` tied to that
   dereference; the premise is recorded on the function's summary, carried in
   the row's `contract_dependencies` (transitively through in-unit summaries)
   and counted in the Safe roll-up. A tracked or literal-NULL binding keeps its
   refusal.
3. **The clause** `quant null-deref #B;` in the bounds contract v1 vocabulary
   only (reason: a check form like `quant checks`, read by a separately
   selected property; admitting it in resource v1 would change the custody
   environment's digests and certificates for a clause custody never reads).
4. **Inference**: an in-unit body that dereferences a formal on every path
   before any NULL test infers the clause; in-unit callers carry the
   obligation; a validated non-NULL result is a source of non-NULL.

## Predicted outcome

**Custody (corpus).** Only stage 2 can move custody; stages 1, 3 and 4 change
no custody byte (the null property is a separate selection and the bounds
consumer ignores null-deref clauses).

* On the base, **6** of the 7 `CG10-INTERIOR-ALIAS-NULLABILITY` functions are
  still blocked only by unresolved identities (`append`, `peek`, `sink_peek`,
  `rd_kafka_share_ack_type_from_msg_op`,
  `rd_kafka_topic_partition_list_regex_cnt`, `mk_vhost_fdt_chain_lookup`);
  `rd_kafka_topic_partition_list_has_duplicates` now also carries an
  uncontracted `rd_kafka_topic_partition_list_sort_by_topic` and indirect
  accesses (a correction to the follow-up's population, re-measured here).
  Predicted: those **6 move Unknown → Safe**, each with at least one listed
  null premise.
* The upper bound from the report: **67** Unknown functions (66 names) whose
  every leaf is an unresolved identity (`scripts/alias_only.py`). Many are
  copies of an unbound `void *` formal (`context`, `_a`/`_b`), which this
  reading does not touch. Predicted total corpus movement: **+6 to +30
  Unknown → Safe**, plus at most a handful through newly validated W4
  summaries; **no Safe → non-Safe move, no new leak or violation, Unsafe 9
  unchanged**.
* Every new Safe row lists its premises; Safe rows resting on a null premise:
  equal to the moved rows plus any row whose in-unit summaries rest on one
  (predicted under 40).
* System: +0 to +3. Slice: no status move.

**Null property (corpus, all projected functions).** About 4,834 rows (one
per custody row). Predicted tally: `Safe` 25–55 % (functions with no
dereference, or every dereference proved, moved or under an inferred
precondition), `Unknown` the rest, `Unsafe` 0–3. Candidates: thousands of
checks; by source kind, predicted ranking: pointers read from memory first
(list iteration, `ctx->member->field`), then formals not under an inferred
clause, then uncontracted call results, allocation results last (Fluent Bit
tests its allocations). Of the custody null premises, predicted discharged by
the null property: under half (the 6 named functions dereference pointers read
from memory, which the property cannot prove).

**Wall time.** Custody arbiter unchanged within noise (±10 %). The null
selection adds one Clang AST dump and walk per unit: predicted +30 % to +100 %
on a run that selects it.

**Lean.** A new `QCore.NullDeref` registered in an `Audit*.lean`: the
dominating-dereference transfer, guard soundness, the clause's caller and
callee obligations, inference soundness, the custody composition (a relied-on
non-NULL reading is always a listed premise) and no `Safe` on an undecided
pointer, with vectors bound to the Rust.
