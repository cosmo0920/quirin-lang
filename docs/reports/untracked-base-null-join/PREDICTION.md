# UNTRACKED-BASE-NULL-JOIN-DISCHARGE: prediction

This file is committed before the implementation commit. The base is
`2a598d2af`, the tip of RETURN-STRICT-MEMBER-NULL-UNTRACKED-ROOT (rule 74v,
approved as r80), which sits on rules 74t (r74), 74r (r72) and 74m (r68), on
main `3ca85c7a6`. USER DECISION 2026-11-24: "launch the null-join card".

## What was known when this was written

The base was probed end to end; nothing was implemented.

* **Reachability.** An uncommitted probe
  (`crates/qcore-c-gateway/tests/zz_probe_null_join.rs`, log
  `private/unj/probe/run2-base.log`) reproduces the registered gap. The three
  registered spellings (`return p->n + 1;`, `return p->n;`,
  `long x = p->n; return x;` after `const struct s *p = 0; if (f) p = f;`)
  are `Safe`, and **each lists the rule-14n premise
  `null-deref:w:<line>:p`**. So does the unguarded `if (c) p = f;`, the
  `if (!f) p = 0; else p = f;` form, a member of a formal joined with the null
  (`if (c) p = f->next;`), a statement read, a condition, a store, `p[0]`,
  `*p`, a chain `p->next->n` and a copy `q = p` (premise on `q`).
* **Mechanism (to be confirmed in the report).** `p = 0` lowers to `Clear`,
  a target-free, known, possibly-null binding, which rule 14n never premises
  (`const struct s *p = 0; return p->n + 1;` is `Unknown`, `unknown_alias:p`).
  `p = f` over an unbound formal takes rule 62's custody-free copy arm and
  leaves `p` unbound. `AbstractState::join`'s one-sided arm makes the joined
  binding `unknown` and possibly null: exactly rule 14n's premised shape
  (`legacy_null_premise::premised`), so the base obligation discharges under
  a premise the function's own null constant refutes on one path.
* **Census.** [scripts/census.py](scripts/census.py) reads an opt-in
  `current` report and, for every row listing a null premise, asks whether
  the premised variable is written a null constant in its own function. The
  report read is RETURN-STRICT-MEMBER-NULL-UNTRACKED-ROOT's tip run
  (`8434d92a3`; its later commits change only docs and one test pin, so it
  is this base's behaviour), read in place and not copied. Output:
  `private/unj/census-74v-tip.jsonl`.
  * 502 rows list a null premise; **44 of them are `Safe`** (the console's
    "44 carrying one"). Only these can move status, because the planned
    change touches nothing but the premise reading.
  * 62 premise rows write the premised variable a null constant; **2 are
    `Safe`**: `cmt_encode_cloudwatch_emf.c::pack_summary_metric`
    (`summary = NULL;` then `summary = (struct cmt_summary *) map->parent;`
    on every path) and `cJSON.c::get_array_item`
    (`current_child = NULL;`, then `current_child = array->child;` after the
    early return, then the loop). In both the null constant is overwritten on
    every path before the premised read, so no null binding reaches a join.
  * 5 `Safe` rows carry only a callee's premise; 3 of them are
    `get_array_item`'s (`cJSON_GetArrayItem`, `cJSON_DeleteItemFromArray`,
    `cJSON_DetachItemFromArray`). They move only if `get_array_item` does.
  * The other 60 null-writing premise rows are already `Unknown`.

## Planned change

A new module keeps, per path, the names that may hold a definitely-null
binding (the null constant, a copy of one) on some reaching path. The join
adds a name either side binds definitely null, a rebinding removes the name,
a copy carries it to its destination, and rule 14n's premise refuses a name
in the set. A binding with no null on any reaching path, a guard-narrowed
non-null binding and every non-premised reading are untouched, so the
change only refuses more.

## Predicted movements (opt-in route, base vs tip)

**Status: 0 moves expected (range 0 to 5).** The only candidates are
`pack_summary_metric` and `get_array_item`, and through the latter its three
`Safe` callers; each would move `Safe -> Unknown` with `unknown_alias` on the
premised variable. No `Unknown -> Safe` move is possible.

**Leaves:** some of the 60 `Unknown` null-writing premise rows may gain an
`unknown_alias` leaf where a joined null reaches a premised read; expected
fewer than 10, and none can change status. The console's "carrying a null
premise" count can only fall.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

The block's `none` is for the pinned routes, which this host cannot run; the
opt-in route's expectation is the paragraph above. **Pinned routes**
(`quirin-audit measure --declared`) need inputs this host does not have and
are recorded as refused, not run.
