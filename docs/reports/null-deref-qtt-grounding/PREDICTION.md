# NULL-DEREF-QTT-GROUNDING: prediction (frozen before the Lean module and spec text)

Base: local `main` at `cc7a20c3e` (NULL-SAFETY-PROPERTY's gate-g2 records).
Written after reading the brief, `QCore.NullDeref`, `QCore.Usage`,
`QCore.UsageDemand`, `docs/QTT_ALIGNMENT.md`, `qcore_core::null_deref`
(`First`, `inferred`, `clause_standing`) and the walk's `firsts` handling in
`crates/qcore-c-gateway/src/null_deref/walk.rs`; before writing any proof or
spec text.

## Behaviour

No analyzer, contract or audit-route source changes. Predicted:

* corpus, system and tail-pause slice: custody `resource-diagnostics.json`,
  the null rows (`QCORE_NULL_PROPERTY` JSON lines and `.summary.json`) and the
  parity stream byte-identical to main's, one AFTER run per route against a
  BEFORE run of main's binary (corpus 915 / 9 / 3,910; system 91 / 4 / 353;
  slice 50 / 1 / 349 / 4 Pending; null corpus 1,219 / 1,442 / 2,260 / 0).
* the release binary built at the tip is byte-identical to main's (only
  documentation, Lean and a Rust test change). If it is not, the outputs are
  still predicted identical.

## The grounding

Predicted provable without changing the walk:

1. A path's entry grade (sequential addition of one per dereference of the
   formal's entry value before any NULL test or assignment of it; offsets add
   nothing) has lower bound >= 1 exactly when the walk's `First` for that path
   is `dereferenced`; the body's grade (the least enclosing interval over its
   terminating paths) is admitted by `relevant` exactly when `inferred` holds,
   i.e. exactly when `clause_standing` is `necessary`.
2. A lower bound of 0 makes `relevant` refuse the grade and imposes no caller
   obligation; `many` admits every grade.
3. After a met demand and no assignment, a later dereference's walk value is
   the first dereference's (`afterDereference`), so its check is discharged and
   is `moved` when the entry value was undecided; the demand of the sequence is
   the sum, whose existence requirement is the first summand's.
4. The grade join's lower bound is the minimum, so `relevant` admits a join
   exactly when it admits both arms; the walk's union of `First` sets infers
   exactly when both arms do; the nullness join is non-null exactly when both
   arms are.
5. `QCore.NullDeref.check`'s discharge of a caller's actual is exactly the
   admission of the actual by `relevant` against an existing identity (no
   concrete value the reading admits is NULL), and the existing
   `caller_obligation_establishes_the_entry` is its consequence.

Predicted divergence to report (not a behaviour change): section L07 says "a
declared grade admits an actual demand only when it contains the actual
interval", so a strict usage checker would *reject* a declared `quant
null-deref #B;` whose body grade has lower bound 0. USER DECISION 2026-10-10
(decision 2) reports such a body `stronger-than-necessary`, never wrong. The
grounded reading has to state that the clause's body check is one-sided (a
caller-side existence precondition, sound for every body), not a usage
rejection. The conservative choice is to keep the walk's reporting.

Predicted possible second divergence: a body with no terminating path has no
least enclosing interval; the walk infers nothing there (`inferred` needs a
non-empty exit set). Conservative (no clause: the formal stays possibly null
in the body).

## Records

No custody or null verdict moves; no ledger entry retired by a measurement.
Lean: one new module and one new `Audit*.lean` (309 audits). One new shared
vector table if the Rust binding needs it, bound by a Rust test.
