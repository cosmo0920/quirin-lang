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

**W45-SUB-OBJECT-STORES (2026-09-26).** Censused the sub-object stores gateway
constraint 5i refuses (10,490 distinct statements, 8,391 gate-refused, 6,912
in Unknown functions; at most 64 of the 388 sole-blocked functions freeable)
and lowered them as `LegacyInstruction::SubObjectStore` through a resolved
typed base (`p->f`, `(p)->f`, `a[i].f`, `s.f`, `(&s)->f`, typedef-defined
structs through `TypedAccessModel::record_tag`), admitting only stores that
move no custody; every failing premise is the gate's refusal byte for byte, so
a refused store still hands on. The escape of an owned identity into a
caller-reachable record was implemented and withdrawn after three arbiter runs:
it produced artifact leaks through derived summaries of stored formals
(`cfl_list_add`, `mk_list_init`) and `release_after_escape` in `cio_file_open`,
and it broke the O04-B08 and W44-LEAK-PIN-AUDIT pins that such a transfer is
invisible at the call site. Result: `destroy_id` Unknown → Safe, `unsupported`
43,590 → 43,360 (223 store leaves, 7 callee-summary leaves, nothing added),
indirect family 17,710 / 3,324 / 388 → 17,487 / 3,307 / 387, leak 37 and every
violation leaf unchanged. Lean `QCore.SubObjectStore` (18 theorems). Retired
`W44-SUB-OBJECT-STORE-REFUSED`; registered seven measured residuals, the
reader's (incomplete records, chains, roots, prefixes) dominating.
[Report](../../reports/w45-sub-object-stores/README.md).

