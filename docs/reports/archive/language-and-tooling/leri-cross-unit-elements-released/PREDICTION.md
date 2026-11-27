# LERI-CROSS-UNIT-ELEMENTS-RELEASED: prediction

Frozen before any tip measurement. Base `8fd7c8d0d`. Inputs: this card's own
preparations ([inputs.json](artifacts/inputs.json)), each also linked with
`prepare clang-build --link` so that rule 40's link-resolution artifact
exists for it.

**Disclosure.** The fixtures (`crates/qcore-c-gateway/tests/cross_unit_elements_released.rs`,
`crates/qcore-audit/tests/link_standing_elements_released.rs`), the Lean
module `QCore.CrossUnitElementRelease` and a one-line change in
`crates/qcore-audit/src/current.rs` were drafted in the worktree before this
file was committed. The census below comes from base-binary runs only
(`9574cd8f…`, `private/runs/{slice-base-map-1,slice-base-resolution-1,system-base-resolution-1}`).

## Census at base

1. **The gateway already carries the clause.** A derived summary is exported
   by rule 4c with its whole contract (`container_effects` included), and the
   caller's certificate compares the import with the replayed home whole, so
   `ElementsReleased` crosses and is applied by the caller's call reading
   exactly as in the home's unit. Fixture, base code: `drain`'s export states
   `{0: ElementsReleased}`; a caller is Safe; an import with the clause
   dropped is refused (`cross-unit imported contracts differ from replayed
   homes`). No gateway change is needed.
2. **No Fluent Bit caller is reachable through the link-standing route on
   these inputs**, for reasons that are not rule 43's:
   * system (artifact `adb20950…`): 2 homes derived; every home named by the
     follow-up (`flb_engine_destroy_tasks`, `flb_storage_input_destroy`,
     `flb_output_exit`, `flb_sched_timer_cleanup`, `flb_task_retry_clean`,
     `flb_sched_destroy`) is refused by condition 13: the selection keeps
     both configurations of every core file, so `fluent_bit/src/<f>.c` names
     two route configurations. Their callers (`flb_input_instance_destroy`
     in `flb_input.c`) are not system units at all.
   * slice (artifact `21c7c6d8…`): `flb_tail_file_remove_all`'s home
     `plugins/in_tail/tail_file.c` has one configuration, but its definition
     census is incomplete (a conditional group whose arms leave different
     file-scope brace depths). Under the stated map that already refuses the
     slice's whole import phase on every run (`cross-unit import phase
     refused: incomplete definition census`). Under rule 40 the home is
     requested, its export analysis fails on the same incompleteness, and
     the failure **rejects the whole route** (`current audit rejected
     (Unknown): enumeration completeness refused: …`): no row is published.
   * Even if carried, neither slice caller would move status:
     `in_tail_exit` also reads an indirect access, unresolved `data`/`ctx`
     and uncontracted `flb_tail_fs_exit`; `flb_tail_config_destroy` reads
     five other uncontracted calls.

## Mechanism

The only code change: a unit whose definition census is incomplete is not
analysed for export (its census already refuses the import phase), so that
refusal holds instead of the route rejection. Rule 4c's text already says an
incomplete census keeps the calls uncontracted; this makes the derived route
do so.

## Predicted moves

* **Arbiter, stated map (the live routes as the measure plugin runs them),
  system and slice: none.** The change is reached only when a *requested*
  home unit is census-incomplete; the eight-home map requests no system or
  slice unit.
* **Corpus: not measured, none.** `quirin-audit pinned` never calls the
  changed function, and no gateway, contract or schema code changes.
* **Link-standing route (quick runs, not the arbiter):**
  * slice: base rejected (no rows); tip completes, prints the census
    refusal, and its rows equal the tip's stated-map rows (no import is held
    in either).
  * system: base and tip identical (the change is not reached: no system
    unit is census-incomplete and requested). Rows the two derived homes
    move against the stated map are rule 40's, read and listed, and none is
    a caller of an `ElementsReleased` summary.
* No Safe row moves; no leak, double release or Unsafe changes.

```measure-prediction
route system
none
route slice
none
```
