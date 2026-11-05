# MEMBER-EFFECT-SUMMARY prediction

> **USER DECISION 2026-11-05** (verbatim), answering MEMBER-ACCESS-PROJECTION's
> stop (member custody through a formal parent): "Option 1 (rec.)": a
> member-effect summary, with option 3's re-attribution done first as the
> verdict-neutral step.

Revision 1, committed before step 1's implementation on base `855b34ade`
(origin/main). Rows are relisted against `baseline:c8c7edf21`
(`private/baselines/c8c7edf2165a2065c234f3a63b4eed23e937ced6/` in the main
checkout), the newest published baseline. The tip also carries the round-3
cards merged at `25508ccaf` (integrate/r4: 38 corpus moves against that
baseline, recorded in docs/reports/member-access-projection/README.md §3);
they are theirs, not this card's.

## Step 1: rule 38i, re-attribution only

Rule 5k is unchanged. The lexical gate's refusal `indirect memory access is
outside the resource projection` of a statement whose only deciding access
is member custody through a formal parent `P` (a formal the body never
rebinds) takes a name per premise, as rule 12k names an uncontracted callee:

| premise | statement | name |
|---|---|---|
| member release with the parent kept | `R(.., P->F, ..)`, `Released` at that position | `member-release-through-formal` |
| custody-carrying store into a member | `P->F = V`, `V` a pointer name or a custody-returning call | `member-store-through-formal` |
| an embedded member's address released | `R(.., &P->F[.G…], ..)`, `Released` at that position | `member-address-release-through-formal` |

The node keeps its writes and stays a non-held `Unknown` refusal, so **no
verdict moves on any route**:

```measure-prediction
route corpus
none
route system
none
route slice
none
```

Rows: every corpus, system and slice row whose gate leaf has one of these
shapes changes that leaf's text only; its status and its leaf count are
unchanged (a line holding two refused statements that carried one gate leaf
can show two named leaves, as with rule 12k).

## The user's `current` run (Fluent Bit 046accf6), step 1

Estimated from MEMBER-ACCESS-PROJECTION's census of the run
([`scripts/estimate_step1.py`](scripts/estimate_step1.py) over
`census/premises.jsonl.gz`, callee modes read from every provider's `.qv`
text; the census has no route answers, so this is an estimate with the
direction of its error stated):

| name | leaves | functions | functions whose every leaf it names |
|---|---:|---:|---:|
| `member-release-through-formal` | 72 | 53 | 4 |
| `member-store-through-formal` | 160 | 119 | 11 |
| `member-address-release-through-formal` | 1 | 1 | 0 |
| total | 233 | 164 | 15 |

- The indirect-access family loses about 233 of its 9,996 leaves to the three
  names. Over-estimate: the rule also asks that the formal is never rebound,
  that a stored name is one the lexical reader declares a pointer (a
  typedef-spelled pointer such as `sds` keeps the gate's text) and that a
  release reading admits with every actual borrowed; under-estimate: a leaf
  whose statement the census bound to two typed paths is not counted.
- No function, unit or library changes status.
