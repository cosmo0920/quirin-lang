# Wave-22 corpus re-pin

The 185-unit pinned-route baseline, re-measured on the composed wave-22
tree and re-pinned here. Supersedes `docs/reports/wave20-corpus/` as the
current baseline; the wave-17 and wave-20 archives stay frozen beneath it
(constraint 21).

## Headline

| Measure | wave-20 baseline | this re-pin |
|---|---|---|
| Units | 2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only | **unchanged** |
| Projected functions (4,834) | 150 / 85 / 4,599 | **150 / 84 / 4,600** |
| Leak diagnostics | 273 | **272** (a withdrawal; constraint 7's third move) |
| unknown_alias | 4,403 | 4,408 |
| unsupported | 49,825 | 49,821 |
| Contract corpus | 71 modules | **72 modules** |

Nothing is promoted. The one function move is `input_chunk_get`
provisional `Unsafe` → `Unknown`: card `AOM-DESTROY`'s faithful
`flb_input_chunk_destroy` contract retires the wave-20 leak leaf as a
**withdrawal** — the release refuses at the call on joined provenance
(`AOM-DESTROY-JOINED-PROVENANCE`) — with every moved leaf attributed in
`docs/reports/wave22-aom-destroy/README.md`. Every other wave-22 card is
corpus-neutral by construction, verified structurally at each merge.

## Measurement and triangulation

One fresh run through the canonical pinned route on the final composed
tree (`0a7e3660`: all six wave-22 cards, five Windows batches), using the
wave-20 close's prepared Clang pin (`build/cycle19/wave20-pin`, revision
`66910c10…`, the pin's own production database in `<source>/build/` — the
identity lesson of the wave-20 close applied), `EXIT=0`, six scopes
summing to 185 units.

Triangulation is exact: the fresh `resource-diagnostics.json` is
**byte-identical** to card AOM-DESTROY's archived AFTER artifact
(SHA-256 `d60b95b1…`), so the card's per-leaf attribution against the
wave-20 baseline transfers to this re-pin verbatim; the itemized diff is
archived here as `artifacts/diff-wave20-to-final.json.gz`.

## Artifacts

`artifacts/manifest.json` lists the five gzipped artifacts plus the
archived diff (uncompressed sizes and SHA-256). No `reason_code` moved
(constraint 24 unaffected).
