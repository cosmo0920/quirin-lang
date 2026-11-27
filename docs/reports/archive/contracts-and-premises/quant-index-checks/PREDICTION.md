# QUANT-INDEX-CHECKS: prediction (frozen before implementation code)

Base: local `main` at `0ebe77f86` (corpus 756 / 9 / 4,069, `resource-diagnostics.json`
`ce57d194…`; system 85 / 4 / 359; follow-ups 705; Lean 287 audits). Written after
reading the consumer (`qcore_contract::bounds_contract`, `qcore_c_gateway::index_extent`,
`qcore_core::index_extent`) and before any implementation code.

## Design chosen

`quant checks P, Idx #i: PRED;` — a new clause introducer inside the contract
block of a bounds contract v1 declaration (`requires bounds_contract_v1;`), one
clause per pointer formal `P`, `PRED` a conjunction/disjunction (`&&`, `||`,
parentheses) of comparisons `==`, `<`, `>`, `<=`, `>=` between the bound variable
`#i` standing alone and a bound term (a canonical literal, a scalar formal, `+`/`-`
of those, a formal at most once with coefficient one). The parameter-collocated
`[| quant Idx #i: … |]` attribute is rejected (reasons recorded in the spec).

## Predicted measurements

| Route | Prediction | Why |
|---|---|---|
| Corpus arbiter (custody) | byte-identical to BEFORE: 756 / 9 / 4,069, `ce57d194…` | no custody code reads bounds contracts; the structural parser accepts one more clause shape that no tracked file contains |
| System route | byte-identical (85 / 4 / 359) | as above |
| Tail-pause slice | byte-identical | as above |
| Corpus with `QCORE_BOUNDS_PROPERTY` | bounds rows byte-identical to BEFORE | the audit routes run the bounds consumer with an empty bounds environment (`CG16-BOUNDS-CONTRACT-TRANSPORT`), so no quant check reaches any function |
| Typed-path parity streams | identical multisets (except the one row the BEFORE binary itself varies, `GAPO-PARITY-DECISIONS-NONDETERMINISTIC`) | as above |

No function moves on any route. A move would be a defect of this card.

## Predicted controls (Rust tests, fixtures modelled on chunkio's `cio_file_st_get_meta_len`)

| Control | Predicted bounds verdict |
|---|---|
| Body reads `map[22]`, `map[23]` under `0 <= #i && #i < 24` (resolved) | Safe (conditional scope) |
| Same, declaration review-held | Pending |
| Body reads `map[24]` under `#i < 24 && 0 <= #i` | Unknown, declaration refuted (withdrawn from callers), never Unsafe |
| Body reads `map[k]` with `k` undecided | Unknown |
| `||` over-wide (`#i < 24 \|\| 0 <= #i`) | rejected at load (unbounded) |
| Empty set (`#i < 0 && #i > 5`) | rejected at load |
| Caller passes a 24-byte array | Safe |
| Caller passes a 16-byte array, definitely reached | Unsafe (call-precondition violation, rule 11s reachability) |
| Same caller, call under a branch | Unknown (reachability unresolved) |
| Formal-relative `0 <= #i && #i < len` | same as `bounds_extent(p, len, elements)` on both sides |
| Formal-relative with offset (`#i < len + 1`) | accepted at load; no fact on either side (Unknown, named) |
| Resource-v1 loader given a quant check | rejected |

Lean: one new module `QCore.IndexQuant` registered in `AuditIndexQuant.lean`
(287 → 288 audit files), theorems as the card brief lists.

Follow-ups: this card's own id registered retired-on-landing; amend
`CG16-BOUNDS-CONTRACT-TRANSPORT` (quant checks travel with bounds modules) and
`CG16-BOUNDS-SYMBOLIC-RANGES` (symbolic quant forms beyond `[0, f)`); register
residuals found during implementation with sizes.
