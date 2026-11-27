# Wave-12 corpus consolidation — one fresh measurement, deliberately re-pinned

> **Superseded as the current baseline, 2026-09-09 (wave-13 consolidation):**
> `docs/reports/wave13-corpus/artifacts/` is the current baseline — 4 units
> changed (REACH-GATE's set, with HASH-CONTRACTS' premise leaf), 2 attributed
> function discharges, headline unchanged. This directory remains the
> wave-12 record.

**Date:** 2026-09-09. **Tree:** main `205e8bf` (all seven wave-12 cards
merged). Integrator-run, closing the wave under hard invariants 3 and 5.

## Provenance

Same prepared pinned clone (Fluent Bit `66910c10…`, explicit Clang), provider
packages rebuilt at the merged tree (schemas 9.0.0, ChunkIO 0.8.0), canonical
pinned route, exit 0, zero gateway failures. Log: `corpus-run.log.gz`.

## Headline

**185 units: 2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only —
unchanged.** Exactly **28 function-level Unknown → Safe** moves, every one on
W4-EXPR-POS's attributed list, none in the other direction, no unit movement.
Nothing is promoted.

## Attribution — exact decomposition, triangulated

Against wave-11: 59 changed diagnostics units plus 3 `source-effects`
evidence rows (the package-identity residue class). Triangulation
(`triangulation-vs-*.json.gz`): the measured tree differs from
W4-EXPR-POS's archived AFTER in exactly the 9 units O04-B09B10 changed, and
from O04-B09B10's AFTER in exactly the 55 units W4-EXPR-POS changed;
55 + 9 − 5 = 59, the five shared units being where both cards touched, with
no interaction leaf beyond each card's own effects. Function status moves
measured directly: 28, all Unknown→Safe, equal to EXPR-POS §5.3's list.
Nothing unattributed remains.

## Re-pin

`artifacts/` here is the current 185-unit baseline (`manifest.json` carries
each SHA-256). The wave-11 archive is untouched as that record and carries a
supersession note; the sidecar reason-inventory test retargets here.
