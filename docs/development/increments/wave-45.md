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


**W45-CALLEE-SUMMARY-CASCADE (2026-09-26).** A cascade census of the 9,200
uncontracted-call leaves on Unknown functions ranks each callee, and each
blocker family of the callee bodies, by the Unknown functions its validation
would unblock through summary chains: one callee unblocks at most 12, a
cross-unit route over existing `Safe` bodies 2, and the indirect-access family
186, most of it sub-object stores and return expressions owned by sibling
cards. The card models the destructor body the census surfaced (gateway
constraint 5k, `legacy_member_release.rs`, `QCore.MemberRelease`). Its first
version read a released member as untracked and certified a double release
`Safe`; the integration review returned it. A member release is now admitted
only as the last use of its place: the base is a formal never copied, the
place is released once and only null-tested before, no loop or `goto` can
repeat it, and the base is released afterwards on every path, so a summary
releases its formal. A pointer member null test is the typed read of its base,
every dereferencing null test now obliges its bases (closing the pre-existing
O04-B08 hole `AGGREGATE-NULL-TEST-AFTER-RELEASE`), and calls to undeclared
unit callees that may release through a formal refuse an actual owning
sub-custody. The arbiter reproduces the baseline byte for byte, then moves 7
functions and `cio_error.c` Unknown → Safe with all 350 changed leaves
attributed; no function leaves `Safe` and no leak or violation leaf moves; the
sole-blocker population goes 412 → 411. The 110 releases the premises refuse
and the typed reader's typedef limits (254 actuals) are registered.
[Report](../../reports/w45-callee-summary-cascade/README.md).
