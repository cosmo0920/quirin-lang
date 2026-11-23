# SP9-PLUGIN-GUARDED-APPLICATION-FORMULAS: prediction

This prediction was committed before any change to a reader, test, fixture,
contract, specification or Lean file. Base origin/main `2c60986b0`.

User decision 2026-11-20 (verbatim): "we don't want to write down the
sentences on premise clauses. We really dislike this kind of results so we
must write down as condition symbols like |-> or other types of logical
symbols to represent the prerequisites which compose for premise
conditions." User 2026-11-23: "Launch both".

The card stopped once, on the draft `01864f410`. The user's decisions of
2026-11-23 on that stop were:

* D1 **A′**: prefixed SP9 terms, with the environment implicit;
* D2 **b**: `default_lifetime_valid` states liveness only;
* D3 **I2**: a warrant term for refuted registration.

## The plan the prediction is about

* The frozen plugin guarded sidecar
  (`docs/reports/premise-guarded-decomposition/applications/plugin.json`)
  has fourteen symbols. Seven are prerequisites and seven are not. Each is
  stated as a C04.18.9 typed-rendering formula over a new, prefixed SP9
  block of rendered terms and value sorts. The texts are recorded for the
  test-side legacy reader, which lifts the frozen sidecar's bindings to
  them, and the plugin replay becomes a positive test again.
* Rust `RENDERED_TERMS` gains a function sort and the SP9 block. The
  specification (C04.18.6, C04.18.9) and Lean gain matching entries. Lean
  proves each symbol equal to its existing definition in
  `QCore.PluginPremiseDecomposition` / `GuardedContractApplications.Plugin`,
  or to the new liveness definition for D2.
* No production module states these symbols. No provider file, contract,
  lock, binding manifest, hypothesis graph or the frozen sidecar's bytes
  change. Guarded symbols are proposed metadata (C04.18.5, C04.18.6), and no
  verdict, standing, certificate or classifier reads one.

## What can move

| Route | Status moves | Content |
|---|---|---|
| corpus | none | byte-identical |
| twelve-home (reads the corpus inputs) | none | byte-identical |
| system | none | byte-identical (no provider or contract digest delta) |
| slice | none | byte-identical |

```measure-prediction
route corpus
none
route system
none
route slice
none
```

There is no Unknown -> Safe, no Safe -> Unknown, and no Unsafe added or
removed. A digest delta on any route would mean an unplanned production
change. It would be reported as a defect of this card.
