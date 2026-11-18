# CG25-VOID-CAST-GUARD-ESCAPE: prediction

Frozen before any code. The integrator measures with base `rev:36b2cf931`.
No measurement was run for this file.

## Mechanism

The repair is one body-level check that no route can bypass. It is added to
`legacy_in_place_custody.rs` and amends rule 11v (6).

* **Before any route runs**, every mention of a tracked in-place object in
  the body's token stream is read once. The stream is the one the parser
  reads, after its rewrites.
* A mention is **admitted** in exactly three cases:
  * it is the object's declarator, under the guard's own `declares_only`;
  * it is the operand of `sizeof`, which is not evaluated;
  * it is a whole `&NAME` actual of a direct call to a declared, contracted
    callee, at a position of the object's kind.
* Any other mention refuses the function at its entry, whichever route would
  have lowered it.
* After the CFG is built, every admitted custody actual must be carried. Some
  call node of that callee must hold the object's place at that position. A
  route that dropped the call refuses the function the same way.

This also closes D3's by-value case (`(void) pthread_getspecific(key)`). The
check reads names off the tokens, not off the nodes, so a by-value scalar
actual is seen too.

## Rows it can touch

Only a body that tracks an in-place object is read by the new check. Every
other body runs exactly as before: in a body with no tracked name, the
reading is the identity.

The census ([README](README.md) §2) found one such body on any route:
`rd_regex_match`, in corpus and `current`. All five of its mentions are
admitted (the declarator and four `regex`-position `&re` actuals). Each
actual's call node carries the place, so the row is unchanged.

| Function | Route(s) | Today | Predicted |
|---|---|---|---|
| `rd_regex_match` | corpus, `current` | Safe | Safe (unchanged, no new diagnostic) |

The system and the slice have no row on an in-place callee. No published
Safe rests on an escape (README §2), so no `Safe -> Unknown` move is
expected. No refusal text changes either: no row today names a tracked
object outside a custody position.

## Predicted moves

* corpus, system and slice: none, and no content change.
* the user's `current --monkey-linked`: none. `rd_regex_match` stays Safe,
  and no Monkey row tracks a key: automatic keys are refused at creation (D3)
  and static keys bind no obligation.

```measure-prediction
route corpus
none
route system
none
route slice
none
```
