# Wave-9 corpus consolidation — one fresh measurement, deliberately re-pinned

**Date:** 2026-09-09. **Tree:** main `13a67f6` (all six wave-9 cards merged).
**Integrator-run**, closing the wave under hard invariants 3 and 5: three
wave-9 cards changed different artifact leaves against the same E01-F1
baseline, so no single card's after-set could be the pin; this measurement is
the union, validated leaf-by-leaf against the archived per-card diffs.

## Provenance

- Clone: Fluent Bit `66910c10a4d7eafa810b84229cf2dcf7b0f26f97`, prepared by
  `python3 -B scripts/prepare_cycle19_clang_audit.py --fluent-bit
  /home/cosmo/Documents/GitHub/fluent-bit --revision 66910c10… --clang clang
  --clangxx clang++` into `build/cycle19/clang/66910c10…/`.
- Provider packages: `python3 -B providers/package.py build --output
  build/cycle19/provider-packages-wave9` (ChunkIO 0.7.0, `qcore-schemas`
  8.0.0).
- Audit: the canonical pinned route,
  `QCORE_CLANG=/usr/bin/clang QCORE_CHUNKIO_PACKAGE=…/qcore.chunkio
  cargo run -p qcore-compiler --example cycle19_fluent_bit_libraries_audit --
  --pinned-evidence <clone> <monkey compile_commands.json> …`, exit 0,
  zero gateway failures. Full log: `corpus-run.log.gz`.

## Headline

**185 units: 2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only —
unchanged.** No unit or function verdict moved in either direction; no
newly surfaced provisional Unsafe unit. Nothing is promoted.

## Attribution — every changed leaf against the E01-F1 baseline

Exhaustive diff: `diff-e01f1-to-wave9.json.gz`
(`docs/reports/corpus-repin/diff_artifacts.py`).

| Artifact | Changed | Cause (archived prediction) |
|---|---|---|
| `diagnostics.json` | 39 units | 38 units' per-function diagnostics and reason strings — O04-B08's fail-closed `unsupported`→`unknown_alias` movement (its report §5); 1 unit's `enumeration.absent_from_raw_scan` 146→15 — E01-F1-RAW |
| `chunk-contracts.json` | 93 rows | `reason_code` `null`→classified on every `unknown` row — GEN-REASON B3's blocking measurement, now carried (91 `unsupported-syntax` + 2 `incomplete-obligation-coverage`) |
| `source-effects.json` | 96 rows | the same 93 `reason_code` rows, plus 3 checked-function rows whose task/witness/import identities move with the rebuilt ChunkIO 0.7.0 package and the measurement directory |
| `source.json` | 4 wrappers | `__FILE__`-style absolute-path string literals tracking the measurement directory (`char[189]` worktree → `char[147]` build tree) — the residue class O04-B08 §5.1 and CORPUS-REPIN itemised |
| `contracts.json` | — | byte-relevant sections unchanged beyond package identity |

The union of the three archived per-card diffs plus the two known residue
classes (path-literal provenance, package identity) explains every leaf;
nothing unattributed remains.

## Re-pin

`artifacts/` here is the current 185-unit baseline
(`manifest.json` carries the SHA-256 of each artifact). The E01-F1 archive is
untouched as that card's record and carries a supersession note. The
REASON-CODES-B3 tripwire test
(`scripts/tests/test_sidecar_reason_inventory.py`) is flipped per its own
design: it now asserts the classified field against this baseline.

## Boundaries

This is a measurement and re-pin, not new evidence: declared trust stays
declared, raw units stay unpromoted, and the path-literal provenance means a
future measurement from a different directory will show the same residue
class again (as every prior re-pin has recorded).
