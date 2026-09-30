# B08-FIELD-LINK-JOIN-SILENT-UNRESOLVED: frozen prediction

Frozen before the repair was run on any corpus, system or slice input. Base:
local `main` = `a235ead6` (the brief's base).

**Deviation, stated:** the brief asks for the prediction before implementing.
The withheld-obligation census (CG04's, unchanged) and a new escape census
were measured first, the second with a behaviour-neutral census build
(`dc296d00`: parity rows only). The repair was then written, and its
synthetic fixtures and unit tests run (`tests/field_link_join.rs`,
`tests/loop_site_generations.rs`, the two unit modules), before this file was
committed; its release binary was built but analysed nothing. CG00 froze no
prediction for this card (a repair the queue did not list), so there is no
`predictions/CGnn.md` to re-freeze; this file is the first one.

## 1. BEFORE on the base

Release binary at `a235ead6`, `--jobs 8`
([measure.sh](scripts/measure.sh) `before`; totals by
[totals.py](scripts/totals.py), keyed by unit file and function):

| Route | Figures | Identity |
|---|---|---|
| corpus (`66910c10`) | 602 / 10 / 4,222 | `resource-diagnostics.json` `2fc3af8c…`, the brief's digest |
| system (`046accf6`) | 37 / 2 / 195 | as the brief states |
| tail-pause slice (`90978ac6`) | 26 / 1 / 242 / 1 Pending | as the brief states |

The census build (`dc296d00`) reproduces all three byte for byte
(`2fc3af8c…`, `60f636af…`, `a4cfa035…`).

## 2. Exposure, measured on the base

- **Withheld obligations** (CG04's census, parity `position:
  "withheld-obligation"`: a `Safe` flow run, W4 derivations included, whose
  settled exit holds a possibly-live identity with an unresolved effect):
  **0** rows in corpus, system and slice. The ledger's figure reproduces.
- **Escapes** (new census, parity `position: "field-link-escape"`, every
  iterate: an escape of an owner that reaches through a field link a child
  that may be live and already carries an unresolved effect, which on the
  base every one-sided child does, or whose status is mixed): corpus **0**,
  system **0**, slice **8 rows in one function**, `flb_create`
  (`src/flb_lib.c`, Unknown on the base), a live child carrying an unresolved
  effect at an escape. **0 Safe** functions anywhere.

The withheld census cannot see the one-sided escape: the base cascade turned
the joined child `ESCAPED`, so nothing withheld remains at the exit. The
escape census closes that blind spot for this card's measurement.

## 3. Prediction

| Route | Prediction |
|---|---|
| corpus | **no Safe → non-Safe move** (both censuses 0). Unknown functions may change leaves: rule-11a refusals that rested on the join's unresolved mark may disappear or change reason, and a leak the join hid in an already-Unknown function may now be reported (Unknown → Unsafe is possible; each such leak is classified by reading). **No Unknown → Safe move** is expected; any is named and argued in the report. The settled-exit floor adds **0** leaves (the insertion audit found no other silent insertion). |
| system | no status move; no leaf change |
| slice | no status move; `flb_create` (Unknown) may gain the escape refusal leaf |

Upper bounds, not targets: every function that moves is attributed in the
report.
