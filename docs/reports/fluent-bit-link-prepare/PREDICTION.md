# FLUENT-BIT-LINK-PREPARE prediction

Committed before implementation, against base
`1e522ca5d` (origin/main). The measurement's base side is `rev:1e522ca5d`.

The card changes preparation only: `quirin-audit prepare` gains an explicit
option that builds and links the `fluent-bit` executable, extracts a
link-resolution artifact from the linker's output and emits a candidate
rule 4c link map. No route, profile or measurement input reads the artifact
or the candidate map (adopting it is BODY-SUMMARY-REUSE-LINK-STANDING's
work), and no analyzer, gateway, contract, provider or Lean file changes. No
row moves on any live route.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

The user's `current` run (Fluent Bit 046accf6, no `QCORE_CROSS_UNIT_LINK_MAP`)
is likewise unchanged: no callee leaves the uncontracted list, no function
leaves the "only uncontracted" set and no unit status changes. `prepare`
without the new option writes the same files as at base.
