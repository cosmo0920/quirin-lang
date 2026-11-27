# Wave-11 corpus consolidation — one fresh measurement, deliberately re-pinned

> **Superseded as the current baseline, 2026-09-09 (wave-12 consolidation):**
> `docs/reports/wave12-corpus/artifacts/` is the current baseline — 59 units
> changed, triangulated exactly to W4-EXPR-POS (28 function discharges) and
> O04-B09B10, headline unchanged. This directory remains the wave-11 record.

**Date:** 2026-09-09. **Tree:** main `0f5b7bd` (all five wave-11 cards
merged). Integrator-run, closing the wave under hard invariants 3 and 5.

## Provenance

Same prepared pinned clone (Fluent Bit `66910c10…`, explicit Clang), provider
packages rebuilt at the merged tree (`build/cycle19/provider-packages-wave11`),
canonical pinned route, exit 0, zero gateway failures. Full log:
`corpus-run.log.gz`.

## Headline

**185 units: 2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only —
unchanged.** No unit or function verdict moved. Nothing is promoted.

## Attribution — every changed leaf against the wave-10 baseline

Exhaustive diff (`diff-wave10-to-wave11.json.gz`): the ONLY changed unit is
`librdkafka/src/rdkafka.c` in `diagnostics.json`, carrying both wave-11
gateway cards at once — E01-RAW-COND's `enumeration.absent_from_raw_scan`
15 → 0 (the conditional-arm max-depth rule; the two enumeration mechanisms
now agree everywhere) and C02-P-ALIAS's single new
`unresolved resource identity app_conf/conf` diagnostic on `rd_kafka_new`
with its count/reason ripple. All four other artifacts are byte-identical;
no residue class even appears (same measurement directory and package
layout). Nothing unattributed remains.

## Re-pin

`artifacts/` here is the current 185-unit baseline (`manifest.json` carries
each SHA-256). The wave-10 archive is untouched as that record and carries a
supersession note; the sidecar reason-inventory test retargets here.
