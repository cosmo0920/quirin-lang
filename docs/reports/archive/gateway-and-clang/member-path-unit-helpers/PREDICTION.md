# MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS: frozen prediction

Frozen before any implementation code, on local `main` `d708641bc`
(corpus 936 / 9 / 3,889, `71387bd3…`; system 93 / 4 / 351; slice 52 / 1 /
347 / 4 Pending; null 1,113 / 1,435 / 2,355 / 18 Unsafe, 84 / 125 / 245,
71 / 103 / 232). Authority: the ledger entry
`MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS`, rule 30 and C04.19.6, rules
13i–13k.

## 1. The reading planned

Rule 30's body check (`member_path_check.rs`) keeps its contradiction scan
of the declaration's own body unchanged (a contradiction still withdraws).
Validation becomes a bounded path reading of the body:

* a control-flow graph over the Clang statements (`if`, `switch`/`case`,
  the loops, `break`/`continue`, `goto` and labels, `return`), read to a
  fixpoint over a finite value set per stated path: `orig` (the value at
  entry), `null` (a null pointer constant stored or tested), `other`
  (anything);
* a null-pointer test of a stated path (`if (P->x)`, `!P->x`, `== NULL`)
  refines the path on each branch;
* a direct call to a function the unit defines with storage class `static`
  (header `static inline` included) that receives the formal, or the
  address of a member path at, above or below a stated path, is followed
  with its parameters bound to that view of the formal (a call summary that
  is the callee's reading from the caller's state, equal to inlining); a
  wrapper's `return f(…)` keeps the callee's per-return outcomes;
* every other call that receives such a view (an indirect call, an
  external or non-static function, recursion, the depth bound) leaves the
  paths it reaches `other`: a call that is not followed never validates the
  paths it reaches;
* an exit whose returned expression is the enumerator a state's guard names
  is exempt from that state exactly when the guard is false there; every
  other exit must establish it;
* at every exit the guard may hold: `#released` / `#moved` read `null`;
  `{V, #kept}` read `orig` or `null` (and, as before, the reading saw a null
  store into the path); `#stale` / `#kept` read `orig`.
* the reading fails (a declared assumption, never a withdrawal) on a copy
  of the formal or of a member's address into a local, unsequenced effects
  in one expression, a statement it does not model (indirect `goto`, asm,
  statement expressions), a pointer store into an unstated path inside a
  followed helper.

Frame: a call reaches the formal's members only through its arguments; a
pointer to the formal stored in memory is not followed (the reading treats
the formal's object as reached through its bindings). This is stated in the
rule; it is stronger than the base check, which read no call at all.

## 2. Standing per declaration (corpus, `contract_dependencies`)

| declaration | before | predicted after | why |
| --- | --- | --- | --- |
| `rd_kafka_topic_partition_list_query_leaders_async_worker_op_cb` | declared assumption | **partly validated (30)** | the wrapper follows the static worker; `eonce` cleared or tested null on every `HANDLED` exit; `replyq.q` cleared inside the header-inline `rd_kafka_replyq_enq` or tested null; `partitions` cleared or untouched; `opaque` untouched; the `KEEP` exits are exempt by the guard |
| `rd_kafka_DeleteRecords_leaders_queried_cb` | declared assumption | **partly validated (30)** | `goto err` is read; `reply` is only read for its members, so `opaque` reads `orig` at both exits |
| `rd_kafka_admin_worker` | declared assumption | declared assumption | the indirect `cbs->request(rkb, &rko->rko_u.admin_request.args, …)` reaches the stated `args` on the path to `destroy:`; `rd_kafka_admin_response_parse`'s indirect `parse` and the non-static or external calls the result helpers make with the op (residual named in the report) |
| `rd_kafka_admin_fanout_worker` | partly validated (30) | **declared assumption (correction)** | the formal is passed to the indirect `cbs->partial_response(rko_fanout, rko)` after the null store; the base check read no call; a `const` pointee is not trusted |
| `rd_kafka_ListOffsets_leaders_queried_cb` | partly validated (30) | partly validated (30) | `reply` only read for its members |
| the three share fan-outs | partly validated (30) | partly validated (30) | the formal only read for its members after the stores |

Net: partly validated 5 → 6 of the eight member-path declarations (two
gained, one corrected). If the measurement differs, the report names each
difference and its reason.

## 3. Verdicts

No verdict moves on any route; null rows identical. A member-path state's
standing is a label no verdict reads (rule 13k), and no corpus caller links a
tracked object at a stated path (the predecessor's measurement). Report bytes
may differ only where a row's `contract_dependencies` names one of the three
declarations whose standing changes.
