# Wave-10 corpus consolidation — one fresh measurement, deliberately re-pinned

> **Superseded as the current baseline, 2026-09-09 (wave-11 consolidation):**
> `docs/reports/wave11-corpus/artifacts/` is the current baseline — one unit
> (rdkafka.c) changed, carrying E01-RAW-COND's enumeration agreement and
> C02-P-ALIAS's one diagnostic, fully attributed there. This directory
> remains the wave-10 measurement record.

**Date:** 2026-09-09. **Tree:** main `f797337` (all seven wave-10 cards
merged). Integrator-run, closing the wave under hard invariants 3 and 5.

## Provenance

Same prepared pinned clone as wave 9 (Fluent Bit `66910c10…`, explicit
Clang, `build/cycle19/clang/66910c10…/`), provider packages rebuilt at the
merged tree (`build/cycle19/provider-packages-wave10`, ChunkIO 0.7.0,
`qcore-schemas` 8.0.0), canonical pinned route, exit 0, zero gateway
failures. Full log: `corpus-run.log.gz`.

## Headline

**185 units: 2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only —
unchanged.** No unit or function verdict moved. Nothing is promoted.

## Attribution — every changed leaf against the wave-9 baseline

Exhaustive diff (`diff-wave9-to-wave10.json.gz`): the ONLY movement is
`chunk-contracts.json`'s backend_teardown section — eight row pairs whose
sole differing leaves are `contract_source`, 18 leaves in total, exactly
card P04-B1's measured contract-vocabulary declaration
(`docs/reports/remaining-p04-b1/README.md`). Diagnostics, source,
source-effects and contracts are byte-identical — as O04-READ-BOUND,
O03-REACTOR, P01-F3, E03-B-O, EXTERN-F1 and CT33-RANKS each measured or
asserted by construction. Nothing unattributed remains.

## Re-pin

`artifacts/` here is the current 185-unit baseline (`manifest.json` carries
each SHA-256). The wave-9 archive is untouched as that record and carries a
supersession note; the sidecar reason-inventory test retargets here.
