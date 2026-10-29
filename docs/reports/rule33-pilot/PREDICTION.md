# RULE33-PILOT prediction

Committed before any code edit, against base
`80c8aea989029c10b3dfbd0179715b7ea050262c`.

The card moves rule 33 (the checked removed-selection frame: its pure
decisions, its caller expansion and its selection entry) into
`crates/gateway-rules/rule-33-removed-selection` behind the GATEWAY-RULE-API
interface and registers its switch. With no rule switched off it changes no
decision, so BASE -> TIP is an identity on all three live routes: zero
verdict movement and byte-identical reports, up to the checked one-to-one
substitution of revision-derived digests (the class RULE37-PILOT recorded,
and MEASURE-CORPUS-SIDE-CWD-IN-BUILD-IDENTITY).

```measure-prediction
route corpus
none
route system
none
route slice
none
```

The contribution (TIP -> TIP with `QCORE_GATEWAY_RULES_OFF=33`) is not part of
the declared comparison. Expected direction only: off expands no frame body or
caller statement and installs no selection frame, so every moved row moves
away from `Safe`. Forecast from rule 33's own report: exactly
`cJSON_DetachItemFromArray` and `cJSON_DeleteItemFromArray` (cJSON, corpus)
move `Safe -> Unknown`; no system or slice row moves (no checked frame
exists there). No row may move toward `Safe`; any such row is a soundness
finding, not a contribution.
