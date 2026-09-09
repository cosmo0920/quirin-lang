# E01-F1-RAW archived 185-unit artifacts

Read-only evidence for [`../README.md`](../README.md) §3. Nothing here is a pin:
the baseline stays at
[`docs/reports/remaining-e01-f1/artifacts/after/`](../../remaining-e01-f1/artifacts/after)
and the re-pin is the integrator's decision.

| Path | Contents |
|---|---|
| `before/` | the five-artifact set measured with the two changed production files at the branch base `05c51a5`, gzipped, with `manifest.json` (uncompressed size and SHA-256 per artifact) |
| `after/` | the same set measured from this branch |
| `diff-archived-to-before.json.gz` | exhaustive `docs/reports/corpus-repin/diff_artifacts.py` document, pinned baseline → before. **Harness validation**: 0 added / 0 removed / 0 changed rows in `diagnostics.units`, `chunk-contracts.results`, `chunk-contracts.backend_teardown` and `source.callbacks`; the 4 changed `source.wrappers` and 3 changed `source-effects.results` rows are the absolute-path provenance class plus wave-8's ChunkIO `0.5.0 → 0.6.0` bump, both present before any change of this unit |
| `diff-before-to-after.json.gz` | the same tool, before → after. **This unit's whole effect**: 1 of 185 `diagnostics.units` rows changed, 0 added, 0 removed, and 0 changes in every other section |
| `census.json` | per-library denominator delta, refusal grounds and unit verdict moves, from E01-F1's read-only [`census.py`](../../remaining-e01-f1/census.py) — all empty |
| `raw-residual.txt` | the raw-scan-vs-enumeration residual before and after, from [`../raw_residual.py`](../raw_residual.py): 146 → 15, in one unit |

`before/` carries a fifth artifact, `contracts.json`, that E01-F1's archive did
not; it is byte-identical between before and after here.

Reproduce the runs with the commands in `../README.md` §3.1; reproduce these
documents with:

```sh
python3 -B docs/reports/corpus-repin/diff_artifacts.py \
    --archived docs/reports/remaining-e01-f1/artifacts/after --measured <before reports>
python3 -B docs/reports/corpus-repin/diff_artifacts.py \
    --archived docs/reports/remaining-e01-f1-raw/artifacts/before --measured <after reports>
python3 -B docs/reports/remaining-e01-f1/census.py \
    --before <before>/resource-diagnostics.json --after <after>/resource-diagnostics.json
python3 -B docs/reports/remaining-e01-f1-raw/raw_residual.py \
    <before>/resource-diagnostics.json <after>/resource-diagnostics.json
```
