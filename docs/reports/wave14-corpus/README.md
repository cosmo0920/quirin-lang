# Wave-14 corpus consolidation — one fresh measurement, deliberately re-pinned

**Date:** 2026-09-09. **Tree:** main `6f1a77d` (all six wave-14 cards
merged). Integrator-run, closing the wave under hard invariants 3 and 5.

## Provenance

Same prepared pinned clone (Fluent Bit `66910c10…`, explicit Clang), provider
packages rebuilt at the merged tree (schemas 11.0.0, ChunkIO 0.10.0),
canonical pinned route, exit 0, zero gateway failures. Log:
`corpus-run.log.gz`.

## Headline

**185 units: 2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only —
unchanged. Zero function status moves.** Nothing is promoted.

## Attribution — every changed leaf against the wave-13 baseline

Exhaustive diff (`diff-wave13-to-wave14.json.gz`): one diagnostics unit
(`fluent-bit-chunk/flb_input_chunk.c` — PROD-CONTRACTS' attributed
−58/+17 leaves: logging callees contracted, six new fail-closed refusals,
eleven category swaps); 3 `source-effects` evidence rows (the
schemas-11.0.0 package residue); and `contracts.json`'s `sources`/`manifest`
keys growing 13.5 → 30 KB — the loaded-contract inventory HASH-CLAUSE and
PROD-CONTRACTS deliberately expanded (corpus 61 → 64, manifest selection
6 → 9, v1 declarations 20 → 25). Note: `diff_artifacts.py` does not
row-itemize that artifact's top-level keys, so the contracts growth was
verified by direct comparison — a recorded tool blind spot
(`DIFF-TOOL-CONTRACTS`). The byte-identity claims of CT34-SANITIZED,
HYGIENE-14, P04-FRAME and P04-COMPOSE-A all hold. Nothing unattributed
remains.

## Re-pin

`artifacts/` here is the current 185-unit baseline (`manifest.json` carries
each SHA-256). The wave-13 archive is untouched as that record and carries a
supersession note; the sidecar reason-inventory test retargets here.
