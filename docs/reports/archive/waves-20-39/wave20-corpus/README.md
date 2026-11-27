# Wave-20 corpus re-pin

The 185-unit pinned-route baseline, re-measured on the composed wave-20 tree
and re-pinned here. This directory supersedes `docs/reports/wave17-corpus/`
as the current baseline; the wave-17 archive stays frozen beneath it
(constraint 21 — nothing there is edited).

## Headline

| Measure | wave-17 baseline | this re-pin |
|---|---|---|
| Units | 2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only | **unchanged** |
| Projected functions (4,834) | 149 Safe / 84 provisional Unsafe / 4,601 Unknown | **150 / 85 / 4,599** |
| Leak diagnostics | 272 | **273** (deliberate, accusing; constraint 7) |
| unknown_alias | 4,405 | 4,403 |
| unsupported | 49,835 | 49,825 |
| Contract corpus | 69 modules | **71 modules** |

Nothing is promoted. The two function-status moves are wave 20's card
`ABI-TYPEDEF-ADDRESS-OF-MEMBER`, both fully attributed in that card's report:

- **`flb_input_chunk_create` `Unknown` → function-level `Safe`** — 6
  diagnostics → 0, six exits of six `Discharged`, on the premise list
  published in `docs/reports/wave20-abi-address-of-member/README.md` §5.2.
  The unit stays provisional `Unsafe`.
- **`input_chunk_get` `Unknown` → provisional `Unsafe`** (accusing): with the
  constructor modelled, W4 derives and validates its summary, exposing a
  release through the uncontracted `flb_input_chunk_destroy`. Its leaf at
  L19849 is the leak-pin move 272 → 273
  (`ADDRESS-OF-MEMBER-DESTROY-UNDECLARED` names the declaration that would
  decide it).

Composed against wave 17, the leaf movement is: wave 19's two attributed
removals (`ABI-TYPEDEF`, `mk_sched_worker_free`) plus wave 20's twelve
removed / three added — every leaf attributed in the two cards' reports; no
other wave-18–21-partial card touches the corpus surface (verified
structurally at each merge).

## Measurement and triangulation

One fresh run through the canonical pinned route on the final composed tree
(`3f37cd07`, all six wave-20 cards plus QUIRIN-RENAME merged):
`scripts/prepare_cycle19_clang_audit.py` at revision `66910c10…` with
`--clang /usr/bin/clang` into `build/cycle19/wave20-pin`, the pin's
`fluent-bit-clang-production` database copied into `<source>/build/`
unchanged (the recorded route detail), provider packages rebuilt at
`qcore-schemas` 14.0.0, `EXIT=0`, six scopes summing to 185 units.

Triangulation: `diff_artifacts.py --archived docs/reports/wave17-corpus/artifacts`
over this run (archived here as `artifacts/diff-wave17-to-final.json.gz`)
agrees with card ABI-TYPEDEF-ADDRESS-OF-MEMBER's archived
`diff-wave17-to-after.json.gz` at every leaf except three residue classes,
each recorded by prior re-pins: path literals carrying the respective
checkout root (including the source envelope's byte-budget-truncated command
echo, whose cut point shifts with the root's length), identity digests from
the fresh package/pin rebuild (package, build, configuration, task and
obligation digests), and the `measured` directory label. Diagnostic content
is identical.

**A discarded first attempt is on record:** an initial consolidation run read
the developer's `cc`-spelled `compile_commands.json` (copied from the
long-lived `build/cycle19/fluent-bit-clang-production` rather than a freshly
prepared Clang pin) and diverged from the card's AFTER in 26 units' envelope
identities. It was deleted unadopted and the run repeated through the
prepared pin — the database identity is load-bearing, which is what
constraint 11's transport rules predict.

## Artifacts

`artifacts/manifest.json` lists the five gzipped artifacts (uncompressed
sizes and SHA-256) plus the archived triangulation diff. The
`reason_code` sidecar tripwire (constraint 24) is unaffected: no
`reason_code` moved in this re-pin.
