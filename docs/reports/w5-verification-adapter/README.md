# W5 trusted verification adapter

`qcore-verification` now owns obligation derivation, witness replay and scope
dispatch. All three implementation files and both unit-test files are byte-identical
to the compiler versions at `d355af3`; `parity.json` records exact file hashes.
Compiler module paths remain compatibility reexports. An architecture test checks
the identity of derived tasks, dispatch results and lifecycle replay types.

This adapter depends on core/frontend/contract/schemas/host. It owns trusted
checking logic above the dependency-free semantic kernel, not a provider or a
schema-only transport layer. Providers continue to depend only on schemas;
no compiler, audit or provider dependency enters this crate. The layering guard
records the allowed downward edges. This clarifies the architecture without
forcing host/frontend types into the zero-dependency kernel.

All existing source-derivation incomplete markers, anti-vacuousness conditions,
source/identity inventories, observation trust premises and certificate replay
rules are preserved. Lifecycle source effects still lack production ABI/effect
import, so this move cannot turn them into Safe. No diagnostic or wire version
changes, no accepted semantic witness expansion and no new Lean definition or
refinement claim accompany relocation.

Standalone tests and compiler W3 process conformance exercise the new placement.
The complete standing gate and Lean axiom audit are archived after integration.
Full W5 still requires specialized source/model provider extraction, reactor and
descriptor boundary separation, registry-based Fluent Bit parity and old-route
removal. The compiler is not yet empty.

The combined workspace passes 571 tests and the complete standing gate plus
19 Lean audits. An added Unix CLI process test verifies that enabled source
scopes launch exactly the selected executable, disabled scopes do not launch it,
and a failing process cannot certify source. Existing W3 conformance still
replays and rejects forged observation evidence through the real host.
