# Wave-15 corpus consolidation — one fresh measurement, deliberately re-pinned

> **Superseded as the current baseline, 2026-09-10 (wave-16 consolidation):**
> `docs/reports/wave16-corpus/artifacts/` is the current baseline — four
> diagnostics units (NEIGHBOURS-CHAINED-TAIL's one leaf and ALIAS-AGGREGATE's
> four units, triangulated in both directions), one contract module's text
> (LOADER-ABI) and the identity/package residue. Headline and every unit and
> function status unchanged. This directory remains the wave-15 record.

**Date:** 2026-09-10. **Tree:** main `056efb7` (all six wave-15 cards
merged). Integrator-run, closing the wave under hard invariants 3 and 5.

## Provenance

Same prepared pinned clone (Fluent Bit `66910c10…`, explicit Clang), provider
packages rebuilt at the merged tree (schemas 12.0.0, ChunkIO 0.11.0),
canonical pinned route, exit 0, zero gateway failures. Log:
`corpus-run.log.gz`.

## Headline

**185 units: 2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only —
unchanged. Zero function status moves.** Nothing is promoted.

## Attribution — every changed leaf against the wave-14 baseline

Three diagnostics units changed — exactly NEIGHBOURS' attributed set
(`cio_chunk.c`, `cio_file.c`, `flb_input_chunk.c`: the six callee contracts
landing, `flb_input_chunk_create` 36 → 21 diagnostics with every
callee-naming refusal gone, both added leaves fail-closed) — plus 3
`source-effects` evidence rows (the schemas-12.0.0 package residue) and the
contract-inventory growth (corpus 64 → 67) in `contracts.json`, verified
directly past the recorded DIFF-TOOL-CONTRACTS blind spot. E01-TAIL,
MEMCLOSE, HASH-REFINE, CT-LABELS and BORROW-RESIDUALS each claimed
byte-identity or doc-only movement and each claim holds; BORROW-RESIDUALS'
declaration census (1 of 25 Released+Borrowed pairs) was re-checked on the
31-declaration merged tree — NEIGHBOURS' six are all borrowed/none, so the
census conclusion stands. Nothing unattributed remains.

## Re-pin

`artifacts/` here is the current 185-unit baseline (`manifest.json` carries
each SHA-256). The wave-14 archive is untouched as that record and carries a
supersession note; the sidecar reason-inventory test retargets here.
