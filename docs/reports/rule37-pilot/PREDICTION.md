# RULE37-PILOT prediction

Committed before any code edit, against base
`80c8aea989029c10b3dfbd0179715b7ea050262c`.

The card moves rule 37 (the typed `sizeof` exemption of the aggregate store
scan and the named `asserted_allocation` premise of checked allocator
specifications) into `crates/gateway-rules/rule-37-result-constructors` behind
the GATEWAY-RULE-API interface and registers its switch. With no rule switched
off it changes no decision, so BASE -> TIP is an identity on all three live
routes: zero verdict movement and byte-identical reports, up to the known
working-directory digest substitution of the measure tool's corpus side
(MEASURE-CORPUS-SIDE-CWD-IN-BUILD-IDENTITY), which must be checked one to one.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

The contribution (TIP -> TIP with `QCORE_GATEWAY_RULES_OFF=37`) is not part of
the declared comparison. Expected direction only: every moved row moves away
from `Safe` (off removes the exemption and withdraws every checked
specification resting on `asserted_allocation`, and adds no admission).
Candidates are the librdkafka constructors whose stores rest on the exemption
and the callers and definitions of `rd_calloc`/`rd_strdup` on the corpus route;
no system or slice row is forecast to move. No row may move toward `Safe`; any
such row is a soundness finding, not a contribution.
