# Wave 47 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Counts and observations describe their recorded dates. Current rules and
> headline status live in [AGENTS.md](../../../AGENTS.md) and
> [STATUS.md](../../STATUS.md); each report owns its evidence. The preceding
> record ([wave-46.md](wave-46.md)) holds the wave-46 paragraphs and stays
> unchanged. Wave 47 follows the [unsupported-coverage queue](../../plans/unsupported-coverage/README.md).

**`W47-ACCESS-PATH-POSITIONS` (2026-09-27, base `2e44e963`).** Plan P3: the W46 Clang-typed access-path reader applied, after every older route, to call-free condition operands wired by the W44 desugaring (so `&&`, `||` and `?:` scope each operand's per-hop obligations to the edges C evaluates), one `NAME = PATH` condition operand, single-read returns (a pointer only from roots the custody closure proves never bound and `may_bind` does not name, with a `NoOp` exit) and call actuals at `Borrowed` or data-tail positions; the owner-aware enum arm was measured as its own step (gateway rule 10k; `QCore.TypedAccessPathPositions`, 33 audited theorems; [report](../../reports/w47-access-path-positions/README.md)). The corpus arbiter at 16 workers moves **447 → 494 → 507 Safe** (positions, then enum arm) of 4,834 functions, 9 provisional Unsafe and 34 leaks unchanged, units unchanged; the system slice gains 4 unique functions (29 → 33, 7 configuration rows). No Safe correction and no new violation; every move is a direct typed admission or an existing-summary cascade, and every non-direct changed leaf is classified. Against the frozen forecast: 12 of 24 position and 10 of 11 enum predictions hit; the 13 misses are typedef-read roots and an anonymous inline enum, registered with 28 unforecast gains named. `W46-ACCESS-PATH-POSITIONS` is retired; `W47-ACCESS-PATH-POSITION-RESIDUALS` and `W47-TYPED-ROOT-DECLARATION-READER` are registered (522 → 523). Gate results and private logs are in the report.
