# VERDICT-BODY-STANDING-CLAUSE-SCOPE: prediction

Frozen before any implementation code. Base: local `main` at `9564eeb6d`.

## What the card changes

Body standing (gateway rule 13g, contract specification C06.5) becomes per
clause. Each piece of body evidence covers exactly the clauses it compares or
reads:

* an equal W4 summary and the `entry` check compare the custody projection:
  every parameter mode (with its kind) and the result mode (with its kind);
* rule 11r compares each exit class's custody projection;
* rule 12w reads the guards of `resource_retained_by` clauses;
* rule 13a reads slot moves, derived out slots and a borrowed result;
* rule 13c reads `resource_reference_kept` / `_dropped`;
* rule 13d reads a returned reference.

No evidence covers a container or counted-user clause, a retention, a holder
reset, a named premise, a variadic tail or a no-return clause. A declaration is
`validated` only when every clause is covered in every unit that examined its
body; `partly-validated` when no unit failed or left it unvalidated and some
clause is covered everywhere; otherwise `declared-assumption`; `held` as
before. The Safe roll-up's "only validated" bucket counts rows whose every
declaration is fully validated; rows resting on a partly-validated declaration
(and no declared assumption) are counted separately.

## Predictions

1. **No verdict moves.** Verdict-row digests equal BEFORE on the corpus, the
   system route and the tail-pause slice; parity multisets identical; totals
   881 / 9 / 3,944, 87 / 4 / 357, 45 / 1 / 354 / 4 Pending. The corpus report
   projection without the card's keys is byte-identical to BEFORE.
2. **The 28 structural-alone declarations** (corpus) all become
   `partly-validated`: each has at least one `resource_borrowed` formal or a
   result clause its structural check does not read.
3. **Closing the gap (task 3)** closes none of the 28: a retaining declaration
   is never given a W4 summary (rule 12t), and the five borrowed-result
   declarations have no validated W4 summary in any unit, so there is no
   existing exact reading of their borrowed formals.
4. **Corpus validated declarations** fall from 73 to between 40 and 45 (the
   W4/entry-backed ones stay validated unless a text-level clause — a named
   premise, a variadic tail, no-return — is uncovered; predicted 0–3 such).
5. **Partly-validated** corpus declarations: 28 to 35 (the 28, plus any
   declaration with container or counted-user clauses whose W4 summary now
   covers its custody clauses, plus any W4/entry declaration with an uncovered
   text-level clause).
6. **Safe roll-up (corpus):** "only validated" falls from 85 to between 55 and
   80; the difference moves to the new partly-validated bucket; "no
   declaration" stays 418, "declared assumption" stays 378.
7. **System and slice:** validated 8 and 4 unchanged (no structural-alone
   declaration there); their "only validated" Safe rows 3 and 0 unchanged.
8. **Size:** the per-clause coverage in the unit field adds under 5% to each
   report.
