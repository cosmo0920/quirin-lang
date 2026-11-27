# Wave-47 round-2a integration

**Branch** `integrate/w47-r2a-land`: `main` `a6b0e215` (W47-TYPED-CONST-POINTER-VALUE, fast-forwarded and pushed) + merge
`78529381` (W47-LEXICAL-CLOSURE-NESTED-ASSIGNMENT) + a pinned-test update. Both cards were measured from the round-1
baseline (`../../w47-callee-closure-r2/integration/artifacts/`).

**Merge.** No code conflicts. Records resolved: `proof/QCore.lean` (both imports), gateway rules 10m and 10o with their
router rows, `wave-47.md` (newest first), the plan's P3 note, STATUS (figures below), the matrix (main + the card's
`update_ledger.py`, 536 → 538) and the index.

## Gate

| Step | Exit | Count |
|---|---:|---|
| submodule, fmt, clippy, reevaluation probe / followup, docs index | 0 | |
| `run_test_binaries.py` at `78529381` | 101 | 6,162 passed / 1 failed / 11 ignored |
| `run_test_binaries.py` after the pin update | 0 | 343 units, 356 summaries; 6,163 passed / 0 failed / 11 ignored |
| `check_lean_audits.py` | 0 | 219 |
| `scripts/tests` | 0 | 365 |
| provider cargo test / fmt / clippy / `contracts.py check` / 5 package build+verify | 0 | 129 passed, 1 ignored |
| `providers/tests` | 0 | 24 OK |
| release build, corpus arbiter, system slice | 0 | |

**The one failure was a composition effect, not a defect.** W47-TYPED-CONST-POINTER-VALUE pinned
`const struct c_box *b; if (!(b = c_make(n))) return 0; return b->item;` as `Unknown [branch]`. It was written on a base
where the lexical closure missed the nested assignment, so the typed return was admitted there. Rule 10o now binds `b`, so
the return is also refused: `Unknown [branch, return]`. The pin was updated to the stricter vector, and its comment
now names rule 10o.

## Composition

`compose3.py` over the base, the two cards' AFTER runs and the merged run ([corpus](compose-corpus.log),
[system](compose-system.log)):

- **System:** COMPOSES, 0 mismatches. Unique functions are 31 / 2 / 201, the nested card's two withdrawals.
- **Corpus verdicts** compose to 516 Safe / 10 provisional Unsafe / 4,308 Unknown (510 + 10 − 4), with 0 status mismatches.
  Four Unknown functions interact at the leaf level:

| Function | Const card alone | Merged |
|---|---|---|
| `rd_kafka_metadata_topic_match` | an indirect-access refusal removed | kept: the join binds the root |
| `rd_unittest` | a for-condition refusal removed | kept: the join binds the root |
| `rd_kafka_mock_cgrp_consumer_member_find_by_instance_id` | a branch refusal removed | removed, with `unknown_alias:member` at the admitted row |
| `rd_kafka_ConfigResource_get_single_broker_id` | a branch refusal removed | removed, with `unknown_alias:config` at 21949 beside the nested card's at 21942 |

Each merged leaf set is the const card's admission constrained by the nested card's binding. Each is the stricter
direction. No Safe, Unsafe or leak comes from the interaction.

## Combined figures (new baseline: [artifacts/](artifacts/))

| Measure | Round 1 | Round 2a |
|---|---:|---:|
| Functions Safe / provisional Unsafe / Unknown | 510 / 10 / 4,314 | 516 / 10 / 4,308 |
| Units | 6 / 8 / 167 / 4 | unchanged |
| leak | 36 | 36 |
| unknown_alias | 7,443 | 7,746 |
| unsupported | 36,209 | 36,154 |
| System unique functions | 33 / 2 / 199 | 31 / 2 / 201 |
| System unsupported / unknown_alias | 2,908 / 985 | 2,890 / 1,020 |
