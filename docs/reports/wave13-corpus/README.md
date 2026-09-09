# Wave-13 corpus consolidation — one fresh measurement, deliberately re-pinned

> **Superseded as the current baseline, 2026-09-09 (wave-14 consolidation):**
> `docs/reports/wave14-corpus/artifacts/` is the current baseline — one
> diagnostics unit (PROD-CONTRACTS, attributed) plus the deliberately grown
> contract inventory. This directory remains the wave-13 record.

**Date:** 2026-09-09. **Tree:** main `57d6372` (all six wave-13 cards
merged). Integrator-run, closing the wave under hard invariants 3 and 5.

## Provenance

Same prepared pinned clone (Fluent Bit `66910c10…`, explicit Clang), provider
packages rebuilt at the merged tree (schemas 10.0.0, ChunkIO 0.9.0),
canonical pinned route, exit 0, zero gateway failures. Log:
`corpus-run.log.gz`.

## Headline

**185 units: 2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only —
unchanged.** Exactly **2 function-level Unknown → Safe** moves
(`logs_tag_records_metrics_enabled`, `mk_plugin_time_now_unix`), both on
REACH-GATE's attributed list; no unit movement. **Nothing is promoted** —
including the wave's landmark: O04-TRACE's C6 discharge
(`flb_input_chunk_create` function-level Safe with every premise named)
lives on the gateway fixture route by that card's deliberate §2.4 placement
decision, so the production corpus is untouched by it, exactly as measured.

## Attribution — every changed leaf against the wave-12 baseline

Exhaustive diff (`diff-wave12-to-wave13.json.gz`): 4 diagnostics units — the
exact REACH-GATE set, with HASH-CONTRACTS' single premise leaf
(`flb_input_chunk.c:216`, INDIRECT → premise refusal) riding inside that
unit's counts — plus 3 `source-effects` evidence rows from the schemas-10.0.0
package rebuild. BORROW-LIVENESS, O04-TRACE and P04-COMPOSE each claimed
byte-identity and each claim holds on the composed tree. Nothing
unattributed remains.

## Re-pin

`artifacts/` here is the current 185-unit baseline (`manifest.json` carries
each SHA-256). The wave-12 archive is untouched as that record and carries a
supersession note; the sidecar reason-inventory test retargets here.
