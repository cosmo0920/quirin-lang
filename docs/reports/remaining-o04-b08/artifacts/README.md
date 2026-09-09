# O04-B08 archived 185-unit artifacts

Read-only evidence for [`../README.md`](../README.md) §5.1. **Nothing here is a
pin**: the archived baseline stays at
[`docs/reports/remaining-e01-f1/artifacts/after/`](../../remaining-e01-f1/artifacts/after)
and the re-pin is the integrator's decision.

| Path | Contents |
|---|---|
| `before/` | the five-artifact set measured with `crates/qcore-c-gateway/src/` reverted to the branch base `05c51a5`, gzipped, with `manifest.json` (uncompressed size and SHA-256 per artifact) |
| `after/` | the same set measured from this branch |
| `diff-baseline-to-before.json.gz` | exhaustive `docs/reports/corpus-repin/diff_artifacts.py --full` document, archived baseline → before. **Harness validation**: 0 added / 0 removed / 0 changed rows in `diagnostics.units`, `chunk-contracts.results` and `chunk-contracts.backend_teardown` |
| `diff-before-to-after.json.gz` | the same tool, before → after. This card's whole effect on the corpus |

Both runs were made from the *same* worktree with only
`crates/qcore-c-gateway/src/` reverted between them, so the before/after
comparison carries no path provenance at all.

`before/` and `after/` share four of five digests exactly — `chunk-contracts`,
`contracts`, `source` and `source-effects` are byte-identical — so the whole
effect of this card is inside `diagnostics.json`.

Reproduce the runs with the command in [`../README.md`](../README.md) §5.1;
reproduce these documents with:

```sh
python3 -B docs/reports/corpus-repin/diff_artifacts.py --full \
    --archived docs/reports/remaining-e01-f1/artifacts/after --measured <before reports>
python3 -B docs/reports/corpus-repin/diff_artifacts.py --full \
    --archived docs/reports/remaining-o04-b08/artifacts/before --measured <after reports>
python3 -B docs/reports/remaining-o04-b08/census.py \
    <before>/resource-diagnostics.json <after>/resource-diagnostics.json
```
