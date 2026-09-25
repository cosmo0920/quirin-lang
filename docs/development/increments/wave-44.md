# Wave 44 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Counts and observations describe their recorded dates. Current rules and
> headline status live in [AGENTS.md](../../../AGENTS.md) and
> [STATUS.md](../../STATUS.md); each report owns its evidence. The preceding
> [record](wave-43.md) holds the wave-43 paragraphs and stays unchanged.

**W44-INDIRECT-CALL-ARGUMENT-ACCESS** (2026-09-25, base `c1e69bb5`). A census
first: the call-argument family is 11,069 leaves on 2,675 Unknown functions
(288 sole-blocked; W43's classifier window 9,522 / 2,407 / 250), and 7,953 of
them hand the read to a callee with no contract. A simple statement the lexical
gate refused for an actual of a contracted call is classified again with every
typed read at a declared `Borrowed` position or in a declared variadic data
tail admitted (scalar values, pointer reads through `T **` bases and pointer
members, non-pointer sub-object addresses, `sizeof` of a type name as a
constant); when that is a contracted call whose named destination is its whole
left-hand side, it lowers to W43's typed read of every base, a new
`ArgumentReads` obligation (integer-carried identities, pointer reads through
owners of sub-custody, bound pointer places refuse) and the call. Custody-taking
positions and uncontracted callees keep the refusal. Lean
`QCore.CallArgumentAccess` (26 theorems). Arbiter: 186 leaves removed and 47
added, all attributed; 1 function Unknown → Safe, 7 functions and 3 units
Unknown → provisional Unsafe; leak 271 → 282, eleven exposures of pre-existing
leak semantics, false by source reading; 0 use_after_release. It found and
pinned a base soundness hole (`W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE`: a
suffix-named destination certifies a wrong release `Safe`). Ledger 345 → 354.
[Report](../../reports/w44-indirect-call-argument-access/README.md).
