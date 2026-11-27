# E01-F1 archived 185-unit artifacts

Read-only evidence for [`../README.md`](../README.md) §2. Nothing here is a pin:
the archived baseline stays at
[`docs/reports/remaining-o04-arms/artifacts/`](../../remaining-o04-arms/artifacts)
and the re-pin is the integrator's decision.

| Path | Contents |
|---|---|
| `before/` | the four-artifact set measured at the branch base `8d0790a`, gzipped, with `manifest.json` (uncompressed size and SHA-256 per artifact) |
| `after/` | the same set measured from this branch |
| `diff-archived-to-before.json.gz` | exhaustive `docs/reports/corpus-repin/diff_artifacts.py` document, archived baseline → before. **Harness validation**: 0 added / 0 removed / 0 changed rows |
| `diff-before-to-after.json.gz` | the same tool, before → after. This unit's whole effect on the corpus |
| `census.json` | per-library denominator delta, every added row, every refusal ground and every unit verdict move, from [`../census.py`](../census.py) |

Reproduce the runs with the commands in `../README.md` §2.1; reproduce these
documents with:

```sh
python3 -B docs/reports/corpus-repin/diff_artifacts.py \
    --archived docs/reports/remaining-o04-arms/artifacts --measured <before reports>
python3 -B docs/reports/remaining-e01-f1/census.py \
    --before <before>/resource-diagnostics.json --after <after>/resource-diagnostics.json
```
