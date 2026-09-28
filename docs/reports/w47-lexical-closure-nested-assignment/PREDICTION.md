# W47-LEXICAL-CLOSURE-NESTED-ASSIGNMENT — frozen prediction

Frozen before any analyzer change is wired. Base `66be515f` (local `main` =
`origin/main`, "Merge PROVIDER-PY-DUAL-STEM-FIXTURE-STALE onto the round-1
integration"). Gateway rule **10o**.

## The gap

`borrowed_actual::untracked_bases` reads one assignment per `;{}`-delimited
token run: the first top-level `=`. A `=` nested in a condition, a `for`
clause (the run starts at `for`), a comma or `?:` operand, a later declarator,
a parenthesised destination (`(p) = …`, the macro-expanded spelling:
`TAILQ_FOREACH`'s `(var) = …`), and a brace initialiser's value are never
read. Rule 10l's records (`w47-typed-root-declaration-reader/artifacts/after/
root-declarations/`) name, in readable functions, AST-bound names the lexical
closure calls never bound: 119 nested call assignments, 295 nested address
assignments, 120 nested copies, 177 later-declarator copies, among others.

## Mechanism to be implemented

1. **(A) Lexical superset reader** (`lexical_nested_assignment.rs`, hook in
   `borrowed_actual::close_over_assignments` and its seeds). Every simple `=`
   anywhere in the body is read with the value the rest of its group (or run)
   holds, a parenthesised sole-name destination is a destination, a brace
   initialiser's value is read, and `&(p)`, `(p)++`, `++(p)`, `(p) op=` seed
   `p`. The old reading is kept unchanged beside it, so the risky set only
   grows. This serves **every** consumer of `never_bound` (C02-P borrowed
   actuals and opaque field reads, O04-B08 store values, rule 5j returns, rule
   5l head reads and guard, W44 call actuals, P2/P3), which must be
   reproducible from the certificate's own tokens, so the AST cannot serve
   them.
2. **(B) The join at the typed positions** (P2 statements and P3 positions):
   in every function where the lexical declaration reader answers, a root is
   never bound only when the extended lexical closure calls it never bound
   **and** the rule-10l AST closure does not bind its spelling (wherever the
   AST reader answers the function; where it refuses the function, today's
   lexical answer stands). The pointer-root premise (rule 5b) also joins
   `may_bind`: the lexical answer or the AST's spelling. Where the lexical
   reader does not answer, rule 10l's served answer is unchanged (it already
   joins the lexical closure, now extended).
3. **Other readers of the same shape** (step 3): `may_bind` (whole-body token
   adjacency: nested `=` already read; parenthesised writes are not — hook),
   O04-B08's store-base premises 3 and 5 (a nested or later-declarator alias
   `x = base` and a parenthesised second binding are not read — hook), the
   protected-formal alias closure (first `=` of a run — hook). The
   declared-local check, the W43 traversal recognisers and the publication
   store reader are audited and either shown not to share the fail-open
   direction or registered with a measured size.

## Prediction

1. **BEFORE reproduces the round-1 baseline**
   (`w47-callee-closure-r2/integration/artifacts/{corpus,system}`):
   `resource-diagnostics.json` byte-identical, sorted parity identical.
2. **Named corrections (Safe → non-Safe), from (B)**, from the 10l exposure
   records (admitted typed rows rooted at a name the AST binds and the
   lexical closure calls never bound):
   - corpus: `mk_fifo_queue_get` (`q`, a rule 5l `container_of` statement
     expression), `mk_plugin_cap` (`plugin`, the same), `rd_kafka_Uuid_str`
     (`bytes`, a sub-object store into a local array) → **Unknown**;
   - system slice: `flb_engine_get_chunk_backpressure_percent` (`ins`) and
     `sb_find_segregated_backlog_by_output_plugin_instance` (`backlog`), both
     `container_of` statement expressions → **Unknown**.
   These five are withdrawals by the join's fail-closed reading, not unsound
   `Safe`s: each root is bound only by a class the AST closure reads more
   conservatively than an established rule (5l's element reading, 5i's
   admitted store). No function with a disputed root of the nested or
   later-declarator class is Safe (81 roots, 79 functions, all non-Safe).
3. **Corpus totals:** functions 510 / 10 / 4,314 → **507 / 10 / 4,317**,
   within a tolerance of up to three further Safe → non-Safe moves from (A) in
   the non-typed consumers (each would be named as a correction; expected
   none). Units unchanged (6 / 8 / 167 / 4): the three corpus functions sit in
   Unknown units. No provisional Unsafe added; leak count ≤ 36 (a refusal
   only withdraws accusations).
4. **Leaves:** changes confined to functions with a disputed root (join) or a
   base bound by a newly read assignment (A): at the 249 admitted typed rows of
   the 130 answered disputed roots (124 functions), `unknown_alias` added where
   a chain's root now needs an identity (condition rows), and COND/IND/RET
   refusals added where a mixed path or a pointer value needs a never-bound
   root; `unknown_alias` also possibly *removed* where a refusal replaces a
   read. Outside the typed rows, (A) refuses C02-P/W44 reads through
   `TAILQ_FOREACH` elements and nested-bound bases (librdkafka mostly), adding
   IND/unsupported or `unknown_alias` leaves in already-Unknown functions.
5. **System slice:** unique functions 33 / 2 / 199 → **31 / 2 / 201** (the two
   above), leaves at the 42 admitted rows of the 11 disputed roots and at
   (A)'s bases.
6. **Disputed set re-emitted:** at every admitted P2/P3 row of a function the
   AST reader answers, zero roots are served never bound while the AST closure
   binds them (by construction). The lexical closure alone (serving the
   non-typed consumers) keeps calling never bound only names the AST binds by
   a class the lexical closure does not model — statement-expression values
   (rule 5l), sub-object stores (rule 5i), undeclared names, by-value record
   actuals, array-object increments/addresses — and **zero** of the nested,
   `for`-clause, later-declarator or parenthesised class.
7. **Negative controls** (each nested form, comma, macro-expanded, `?:` arm,
   compound assignment in a condition) bind the root at the premise level
   after the change, and at the base the nested ones do not; positive
   controls (untouched roots, a nested assignment from an admitted field
   read) certify before and after.
