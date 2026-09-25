# Wave 45 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Counts and observations describe their recorded dates. Current rules and
> headline status live in [AGENTS.md](../../../AGENTS.md) and
> [STATUS.md](../../STATUS.md); each report owns its evidence. The preceding
> [record](wave-44.md) holds the wave-44 paragraphs and stays unchanged,
> apart from the paragraph of W44-CONTRACTS-ONLY-NO-BUILD, still running
> when this wave opened. Wave 44's close (stability review) and the
> documentation compaction are postponed to a later wave by the user's
> decision (2026-09-25).


**`W45-RETURN-EXPRESSION-FAMILIES` (2026-09-25, base `b160d765`).** A re-census first: on main's baseline the return-position refusal is 495 leaves on 417 Unknown functions and the only reason of 147, mostly accessors refused for their operand's member typing ([report](../../reports/w45-return-expression-families/README.md), frozen [prediction](../../reports/w45-return-expression-families/PREDICTION.md)). Two families are admitted in `legacy_return_family.rs`. In a function whose definition declares a scalar value result, a return expression that is a read (no call, write, comma, brace or unary `&`; no tracked descriptor; untracked or scalar names, and a name the body may bind only as the root of one dereference C always evaluates) lowers to a typed read over its dereferenced bases and a `NoOp` exit: the value carries nothing. A cast of a whole call is classified as W4 classifies the call: a refusal is lowered to itself, a `None` result admitted, an `Owned` result only under pointer casts of a declared object pointer, escaping its fresh site; a derived or reallocated result under a cast, an integer cast of an owned one and every pointer-valued read keep the refusal. Lean `QCore.ReturnExpressionFamilies` (23 theorems). Arbiter: BEFORE byte-identical; 76 functions Unknown → Safe (72 sole-blocked, 4 callers of a newly summarized accessor), none to provisional Unsafe, units unchanged, leak 37 preserved; the refusal falls to 305 / 249 / 69; all 405 removed and 40 added leaves attributed. Retired `W43-RETURN-CALL-UNDER-CAST` and `W43-RETURN-TYPE-OPERATOR`; registered `W45-RETURN-POINTER-RESULT-READS` and `W45-RETURN-BOUND-BASE-POSITIONS`. The census-sink guard found W44's `QCORE_CALL_ARGUMENT_CENSUS` unlisted; it is listed now.
