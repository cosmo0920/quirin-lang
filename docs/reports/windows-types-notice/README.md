# WINDOWS-TYPES-NOTICE

USER DECISION 2026-11-25: "we need to size_t related notice on AGENTS.md and
CLAUDE.md indexes and create windows-types.md like notice document and link
there"; "yes, land it as r75", carried into landing batch B by "Three batches". A documentation-only change made at
integration, with no card branch.

## Change

* [`docs/development/windows-types.md`](../../development/windows-types.md)
  (new): the LLP64 notice for test fixtures and synthetic C declarations.
  Spell `size_t` as `__SIZE_TYPE__` (or `host_abi::size_t_declaration!`),
  `ssize_t`/`ptrdiff_t` as `__PTRDIFF_TYPE__` and 64-bit integers as
  `long long`, never plain `long` at a contract position; give an enumeration
  at a `u32` position an `unsigned int` typedef; declare libc callees in the
  fixture. It also gives the landing scan command. It states no rule of its
  own; gateway rules 11q and 12u stay the authorities.
* `AGENTS.md`: a "Windows types notice" section routing to the page.
* `CLAUDE.md`: a pointer section to the page.
* `docs/development/README.md`: a row for the page.

## Grounds

Two rounds of Windows CI failures came from these spellings:
[round 1](../windows-ci-plain-long-fixtures/README.md) (r42, 14 tests) and
[round 2](../windows-ci-llp64-fixtures-2/README.md) (r73, 8 tests). The same
hazard reached a queued card's new test file and was fixed at r74
(`in_place_regex_failure_use.rs`).

No gateway, contract, provider, rule, Lean or test change.
