# CG10-SMALL-BUNDLE: prediction

Committed before the implementation commit. Base `8e7e8d509` (integrate/r48:
main `0057bac00` plus CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 4). USER
DECISION 2026-11-21: launch this bundle. Specification: the three
`named_followups` entries CG10-UNCAST-RETURN-UNCONTRACTED-ATTRIBUTION,
CG10-RESULT-HOLDER-RETAINED-BY and CG10-W4-FUNCTION-POINTER-DECLARATOR-FORMALS.
Gateway rule ids 74, 74a, 74b.

**Disclosure.** Before this file was committed: the base binary was built
(`91f0c2d6…`) and its opt-in `current` run started (its report not yet read);
the census below was read from round 4's tip-2 report
(`agent-aab78…/private/r4/run-tip2`, main `9b107caf1` plus round 4, without
CG10-READER-GAPS), the closest report on disk; the rule-74 hook (below) was
written but not built, tested or measured. No tip binary exists.

## 1. Readings and the census

### (a) Rule 74: CG10-UNCAST-RETURN-UNCONTRACTED-ATTRIBUTION (3 rows)

Rule 12k reads a whole-call return through its synthetic `PLACE = f(args)`
with every memory-reading actual read at a `Borrowed` position; a cast actual
is one neither reader recognises, so `return print_string_ptr((unsigned char
*)item->valuestring, p)` keeps the gate's text. **Reading:** where rule 12k's
spelled reading declines and the return hides a place behind views rule 71a
removes (`returned_call_hides_a_place`, `uncast_returned_views`), rule 12k is
asked of rule 71a's uncast call, with W44's typed actual reader matching each
uncast actual to the view's operand (rule 71a's `Transparent` guard). A
re-attribution only: the node, its writes and every verdict are unchanged.

### (b) Rule 74a: CG10-RESULT-HOLDER-RETAINED-BY (2 sole rows, 4 mentioning)

**Refusal planned, with its reason.** Deriving `resource_retained_by(q,
result)` needs (1) a per-identity retention record on the body's tracked
result, ended by a release of the result and stated at a returning exit, and
(2) at every caller, the end of the record at the result's release (rule 13e,
§C04.14.2: "a record ends when the holder is released (a heap handle's
destructor)"). For a declared contract (2) rests on a reviewed, pinned
assumption that the destructor does not release `q` (constraint 4). A derived
summary has no review: the result's destructor is any body, and the member
release model (rule 38, O04-B08's field link) reads a member it releases as
an owned field, so a destructor releasing the member that holds `q` would end
the record and release the caller's object unseen (a double release when the
caller then releases `q`). The same holds inside the body on an error path
that releases the result through a destructor. No reading of the census rows
is sound without a destructor member-effect premise no derived summary
supplies. A Lean counterexample, a refusal fixture and the follow-up are
planned; no row moves.

### (c) Rule 74b: CG10-W4-FUNCTION-POINTER-DECLARATOR-FORMALS

**Reading:** `parse_parameter_declarators` splits the list at top-level
commas and reads a group spelled exactly `SPEC+ ( * NAME ) ( BALANCED )` as
the formal `NAME`, a function-pointer formal (no lifetime, rule 10v's class),
**unless the body spells a call through it** (`NAME (` or `( * NAME ) (`):
an invocation of a caller-chosen function can release what it is handed
(C04.19's callback shape), and a formal named like a contracted function
would otherwise be read as that function, so the whole list stays refused.
Every other parenthesised group (`int (cmp)(…)`, `void (*log_cb)`, a macro
`STACK_OF(X509) *`), a nested or qualified declarator and a variadic list stay
refused.

Census (scripts/fnptr_formals.py over the units of round 4's tip-2 report,
source text not preprocessed, a lower bound): **102 definitions** hold a
parenthesised parameter group: **64 read** (61 Unknown, 3 Safe:
`rd_kafka_set_logger`, `rd_kafka_buf_set_maker`,
`rd_kafka_conf_set_oauthbearer_token_refresh_cb`), **30 with a called
formal** (stay refused), **8 other declarators** (stay refused). Callers
(scripts/callers.py): 12 Unknown rows are blocked only by an uncontracted call
to a function of the 64 (an upper bound: each callee must also validate a
summary, and most are Unknown for other reasons: a timer or registry
registration, a store of the function pointer into a member, which rule 10v's
summary gate keeps unsummarised).

Consumer census (the ten families sharing the reader): planned in the
report, each consumer's behaviour for a function-pointer formal checked.

## 2. Predicted moves (opt-in `current --monkey-linked`, Fluent Bit 046accf6)

* Rule 74: `print_string`, `pb_dec_bool`, `pb_enc_fixed_length_bytes`
  **re-attributed** (the access leaf replaced by ``call to uncontracted
  function `print_string_ptr` `` / `pb_decode_bool` / `pb_encode_string`),
  each still `Unknown`; access-alone rows **-3** (if no other leaf), access
  leaves -3 to -10 (other uncast returns to an uncontracted callee); no
  status move.
* Rule 74a: no move.
* Rule 74b: **0-4 Unknown -> Safe** (callers above or census rows whose only
  blocker was the missing formal list); leaf changes in 0-60 rows (census
  rows and their callers now reading a summary or a formal set).
* No Safe -> non-Safe move (the three Safe census rows stay Safe), no new
  Unsafe, Pending unchanged, no leak added.

## 3. Pinned routes

`quirin-audit measure --declared` cannot run on this host (no `build/cycle19`
inputs or baselines). The corpus route (`66910c10`) carries the same three
rule-74 rows where it analyses cJSON and nanopb; rule 74b may move rows where
the route analyses librdkafka, Monkey or chunkio.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

No Safe -> non-Safe move and no new Unsafe on any route.
