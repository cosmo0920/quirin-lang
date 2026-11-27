# LINK-STANDING-SELECTION-TWO-CONFIGURATION-KEYS: prediction

Frozen before any code change and before any tip measurement. Base
`2b5c8907a` (merge of LERI-CROSS-UNIT-ELEMENTS-RELEASED `7e2e255e1` onto
origin/main `591e5523f`). No fixture, Lean or route code had been drafted
when this file was committed.

## Census at base (no tip binary exists)

Read from the cross-unit card's own link-resolution artifacts of the same two
revisions (system `adb20950…`, Fluent Bit `046accf6`; slice `21c7c6d8…`,
upstream `90978ac6`), read-only (`private/scripts/samefile.py`,
`private/scripts/keycheck.py`), and from that card's base quick-run lines:

* system: rule 40 derives 2 homes and refuses 103 by condition 13. The 103
  are exactly the default-visibility, strong, linked, reproduced
  definitions of the seven two-configuration core files of the system
  selection: `flb_input_chunk.c` 26, `flb_output.c` 28, `flb_scheduler.c`
  16, `flb_task.c` 13, `flb_engine.c` 10, `flb_storage.c` 8,
  `flb_engine_dispatch.c` 2.
* slice: 62 derived, 62 refused by condition 13: `flb_input.c` 45,
  `flb_lib.c` 11, `flb_input_thread.c` 6.
* In every one of those files, the artifact records two configurations: the
  home's (linked, reproduced, attributed to exactly one link input) and one
  other (`linked: false`, its object read, **attributed to no link input**).
  So each other configuration is shown outside the link.

## Mechanism (to be implemented)

Condition 13 is decided over the configurations the link can use: under the
home's `library/file` key, every route configuration other than the home's
must be one the artifact records and attributes to no link input. A
configuration the artifact does not record, or that some link input is
attributed to (even an ambiguous input that left it `linked: false`), keeps
the refusal. Under a derived resolution, a route configuration shown
outside the link exports and defines nothing in rule 4c's census, as a
not-linked standalone unit already does (CURRENT-MONKEY-LINKED-CONFIGURATION),
so the linked configuration is the key's one candidate home.

## Predicted measurements

* **Arbiter (`measure --declared`, stated map, system and slice): none.**
  Under the stated map no derived resolution exists, so neither the new
  condition nor the census exclusion is reached.
* **Rule 40 quick runs, home counts:**
  * system: 2 → **105** derived homes; refusals by condition 13: 103 → **0**;
    every other refusal count unchanged.
  * slice: 62 → **124** derived; condition 13: 62 → **0**; others unchanged.
* **Rule 40 quick runs, rows:**
  * slice: **no row moves.** `plugins/in_tail/tail_file.c` is linked and has
    one configuration, so it is not excluded; its incomplete census still
    refuses the slice's whole import phase
    (TAIL-FILE-DEFINITION-CENSUS-INCOMPLETE, out of scope).
  * system: the import phase now has homes in the linked configurations of
    the seven core files. Moves are possible only from `Unknown`, in a
    linked (static) configuration unit, at a call to a core home in another
    file. Expected: **few, 0 to 6, `Unknown -> Safe`**, because most callers
    keep other leaves, a home must itself be a Safe row with a validated W4
    summary to be exported, and a unit reaching more than 24 homes holds
    none (rule 4c's budget; the large core units such as `flb_engine.c`
    likely exceed it). No `Safe` row moves; no row moves to `Unsafe`; rows
    of the unlinked (shared) configurations do not move (condition 3).
  * The released-element callers named by LERI-CROSS-UNIT-ELEMENTS-RELEASED
    (`flb_input_instance_destroy`) stay out of reach: they are slice units
    only (SELECTION-ROUTES-SPLIT-CALLER-AND-HOME, out of scope).
* **Corpus: not measured, none.** `quirin-audit pinned` refuses a
  link-resolution artifact, and the change is reached only under one.

```measure-prediction
route system
none
route slice
none
```
