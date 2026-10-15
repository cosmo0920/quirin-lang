# MEMBER-PREPASS-OTHER-ROUTES: prediction (frozen before implementation code)

Base: local `main` `d708641bc`. Authority: the ledger entry
`MEMBER-PREPASS-OTHER-ROUTES` (its DISCHARGE), rule 12n, user decisions R1
and F1, and the brief's second part (the scope file re-checked at run time).

## What changes

1. **Current and no-build routes.** Before any unit is analysed, both routes
   run `qcore_audit::member_prepass` over their own compile database (the
   current route: every entry of `--compile-database`, plus the Monkey units
   it analyses; the no-build route: the selected units under the recorded
   assumed configuration) and install the run's facts on each analysing
   thread. A unit then decides a member invocation over the run's set, as on
   the pinned route. The fold and the sidecar's `callback_members` section
   read the pre-pass (`prepass` key). No pre-pass runs when no declaration
   invokes a member.
2. **Checkpoints.** A checkpoint (schema 5) carries the run's facts: the
   run identity digests them (a changed run set recomputes every unit and
   never composes with another), the global section carries the run's
   closure of each invoked member and the fold's environment, and each
   unit's checkpoint carries the member and state facts it recorded. The
   composer folds them as the whole run does: it composes when the fold
   demotes nothing and refuses otherwise (it does not rewrite rows). A run
   whose contracts invoke a member but whose checkpoints carry no run facts
   is refused, as today.
3. **Scope re-check.** `member-facts-scope.json` (schema 2) names the
   revision its evidence was taken at (`66910c10…`) and, per entry, a typed
   evidence the pre-pass re-verifies against the run's own sources:
   * `tokens-absent`: the unit's raw text, every file Clang resolves for it
     under its own command (`-M -MG`), and the generator sources the entry
     names (for a unit or header generated at build time, absent from the
     inputs) name no token of the pre-pass, and every unresolved include is
     one the entry names as generated;
   * `not-linked`: the CMake link lines the entry names (beside the build's
     `CMakeCache.txt`) link the named archive and not the wrapper.
   An exclusion whose evidence does not hold, or cannot be checked, is
   dropped: the unit is preprocessed and read like any other unit, and a
   unit that then cannot be read opens every member. The check runs inside
   the token fixed point, against the final tokens.

## Predicted measurements

* **Verdicts: none moves on any route.** Corpus 936 / 9 / 3,889, system
  93 / 4 / 351, slice 52 / 1 / 347 / 4 Pending, function by function and
  leaf by leaf. The resource-diagnostics reports of all three routes are
  byte-identical to BEFORE (the pinned route's facts are unchanged when every
  exclusion holds; system and slice hold 0 member-invoking leaves, so
  deciding over the run's set changes no row).
* **Null property:** tallies 1,113 / 1,435 / 2,355 / 18 Unsafe, 84 / 125 /
  245, 71 / 103 / 232; the rows byte-identical to BEFORE on every route.
* **Exclusions:** all 14 entries (100 units) hold on the pinned inputs
  (`66910c10`). On the system tree (`046accf6`) and the slice tree
  (`34155955`) the same entries are expected to hold (the generated parsers,
  LuaJIT, `fluent-bit.c` and `flb_dump.c` name no Kafka token at either
  revision); any entry that does not hold is reported by name and its units
  are read or unread, never silently excluded.
* **Sidecars:** the corpus `callback_members.prepass` section gains the
  re-check record (evidence revision, each entry held or dropped and why);
  the system and slice sidecars gain the `prepass` section (about 1,419 and
  1,421 units, roughly 845 read each, 0 unread if every exclusion holds) and
  the members read from the run's facts.
* **Wall time:** corpus about the same (plus the re-check's Clang `-M` runs
  over about 95 units, a few seconds). System about 77 s → 150–230 s and
  slice about 38 s → 110–200 s: the pre-pass preprocesses the whole
  production compile database (about 105 s alone at `--jobs 4` on the
  corpus).

## What would falsify it

A row moving on any route, a null row differing, an exclusion dropped on the
pinned inputs, or an unread unit on the system or slice route. Each would be
read and reported by name, never adjusted away.
