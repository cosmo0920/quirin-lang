# JUMPED-SHADOW-FORMAL-READINGS: prediction

Committed before the implementation commit. Base `4fef267f4` (the
NAME-KEYED-HOLDER-SHADOWING tip, rule 86). USER DECISION 2026-11-25,
"launch the JUMPED-SHADOW-FORMAL-READINGS card". Discharges
`JUMPED-SHADOW-FORMAL-READINGS`, registered by NAME-KEYED-HOLDER-SHADOWING.

## What was known when this was written

No implementation was measured. Known from reading the base and from the
probes `crates/qcore-c-gateway/tests/jumped_shadow_formal_readings.rs`
(committed `66339083c`), run at base (`private/card-logs/probes-at-base.log`):

* **Admitting at base, stopped only by the internal certificate check.**
  Container publication into a container rooted at a jumped-shadowed formal
  (CJ1, CJ2, CJ4); rule 38's descriptor-cell `formal` closure (DCJ1-DCJ3);
  W4 entry custody, where a release through a jumped shadow is read as the
  formal's release (WDJ1) and a caller consumes that summary (WJ1, WJ2).
* **Admitting at base past the certificate check.** A derived local of
  container publication (`L = P->F`, `CallerFormals::derived`) shadowed by
  a jumped-over local array (CJ5): a publication into the local array was
  read as into the caller's container, and the row was `Safe`.
* **Not admitting.** Counted users refuse every `switch` and jump; slot
  moves refuse by their slot-mention census; member custody attribution
  only re-attributes a refusal (it names the shadow as "the formal").
  The unrecognized declarations (U-shapes) refuse incidentally everywhere,
  but the counted-user clause record reads a shadow as the formal (KU1,
  KU2, KDU1: `Held`, row `Unknown` only by an indirect access).

## Planned change

1. Rule 86 is extended to `CallerFormals` itself, at its one construction
   point: a formal is a stable formal only where it is one declaration
   (`every_reference_is_the_formal`), a derived local only where its name
   is one `VarDecl` every `DeclRefExpr` of the name designates, and its root
   formal is still stable. The counted-user pass's stable formals read the
   same. Every consumer (container publication and its foreign unlinks,
   member custody attribution, slot moves, rule 38's `formal` closure,
   rule 12t) reads the narrowed set.
2. The shadow guard reads Clang's declarations: a function with a formal
   that is not one declaration (any second declaration of its name in the
   body, whatever its spelling, or a reference to another declaration)
   refuses at its entry, on every path, so a jumped declaration and an
   unrecognized one are refused directly. Where Clang's reading of the
   function is unavailable, the function refuses at entry.

Only `Safe -> Unknown` is possible.

## Predicted movements

Opt-in `current --monkey-linked` (this card's arbiter) over Fluent Bit
046accf6: few rows, all `Safe -> Unknown` or leaf-only. At base a
shadowing declaration the token guard recognizes already refuses every
path through it, so a move needs a body whose shadowing declaration lies
off some `Safe` path, a declaration the guard does not recognize, a
derived local shadowed elsewhere, or a function whose Clang reading is
unavailable. I expect at most a handful (0-10) of `Safe -> Unknown`
moves, and leaf-only changes (the entry refusal added) on rows that are
already `Unknown` because of a shadowing declaration. No `Unknown`,
`Pending` or `Unsafe` row becomes `Safe`, and no `Unsafe` is added.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**Pinned routes** (`quirin-audit measure --declared`) cannot run on this host
(no `build/cycle19` inputs or baselines). Expected there: no move. A move,
if any, is `Safe -> Unknown`.
