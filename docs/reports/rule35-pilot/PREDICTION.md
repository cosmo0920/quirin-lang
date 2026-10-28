# Prediction

Committed before implementation, against `80c8aea989029c10b3dfbd0179715b7ea050262c`.
Card RULE35-PILOT moves gateway rule 35's two arms (literal read, decayed
array member) into their own rule crate behind the GATEWAY-RULE-API
interface. It is a move: no verdict, leaf, diagnostic or row may change on any
of the three live profiles with every rule on (BASE `rev:80c8aea98` against
TIP). The corpus route's per-unit build identity embeds the side's working
directory (MEASURE-CORPUS-SIDE-CWD-IN-BUILD-IDENTITY), so its digests may move
under a checked one-to-one substitution; no row moves.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

The switch measurement (`--rules-off 35`, TIP against TIP with rule 35 off)
is the rule's contribution, not a prediction of this block: GATEWAY-RULE-API's
incomplete run saw corpus 12 rows Safe→Unknown, system 0, slice 0.
