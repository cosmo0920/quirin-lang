# RULE32A-PILOT prediction

Committed before any code edit, against base
`db2795100a760efa0a2cbcbe13217bffc3cf4733`.

The card moves gateway rule 32a's decision (the caller-frame hold: inside a
frame member's own body, a call to a member of the same reviewed caller-frame
projection whose declaration is in force is held `Unknown` under
`caller-frame-self-reliance`) into a rule package under
`crates/gateway-rules/`, asked through the rule interface's `TokenHold`
position by the gateway's `legacy_caller_frame_hold` host in all three token
pipelines, and registers it as registry row `32a`. The hold is registered
`not-switchable`: no switch list can remove it, so the card adds no switched
side. With no rule switched off it changes no decision, so BASE -> TIP is an
identity on all three live routes: zero verdict movement and byte-identical
reports, up to the known working-directory digest substitution of the
measure tool's corpus side (MEASURE-CORPUS-SIDE-CWD-IN-BUILD-IDENTITY), which
must be checked one to one.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

No `--rules-off` side is measured: `QCORE_GATEWAY_RULES_OFF=32a` is refused
by the analyzer (a hold is not switchable), so there is no switch-measured
contribution to predict.
