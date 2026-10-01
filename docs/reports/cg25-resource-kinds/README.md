# CG25-RESOURCE-KINDS: a library-handle kind and an in-place kind

Card CG25-RESOURCE-KINDS (wave 48, round 10; High; gateway rule
[11v](../../agents/gateway.md#constraint-11v)), 2026-10-01/02, on base
`b6a3dc42` (local main = origin/main). Clang gap entry K07, route G, absorbing
`W44-DLOPEN-DECLARATION-REMEASURE` and `W43-LIBC-IN-PLACE-LIFECYCLE`. The
property published is **custody safety** (no leak, double release, release of
another kind or use after release of a tracked identity) for the two new kinds,
scoped to route G over the libc declarations of `providers/quirin-provider-libc`
and over automatic objects; it adds no other property.

## Headline

* **Kinds** (contract spec [§C04.6, §C04.6.1](../../QCORE_CONTRACT_SPEC_r0_3.md)):
  `dynamic_library` (pointer-carried, failure value NULL; `dlopen`/`dlclose`)
  and the **in-place** `regex` (`regcomp`/`regfree`; the obligation lives in the
  caller's `regex_t`, the failure value is a nonzero status). Each kind has one
  spelling and one carrier (`ResourceCarrier::{Pointer, Integer, Storage}`); no
  kind is equated with another.
* **Gateway** (`legacy_in_place_custody.rs`, thin hooks in `legacy_c.rs`): an
  in-place obligation is bound to one automatic object passed as `&NAME`; the
  initialiser's status, held by a plain name, witnesses it and a null test of
  the witness refines it; tracked storage is custody, never data.
* **Declarations** (`libc/dlfcn.qv`, `libc/regex.qv`): `dlopen`, `dlclose`,
  `regcomp`, `regfree`; `regexec` and `regerror` gain the `regex` kind on
  `__preg`. Pins 122 modules unchanged; six pin rows changed or added; the libc
  `contracts.lock` refreshed for the six declarations (`quirin-audit pin`).
* **Lean**: [`QCore.ResourceKinds`](../../../proof/QCore/ResourceKinds.lean),
  28 theorems audited by [`AuditResourceKinds.lean`](../../../proof/AuditResourceKinds.lean);
  `QCore.KindedCustody.Kind` gains the two constructors, every one of its 30
  theorems unchanged. Shared vectors `kindVectors` and `protocolVectors`, bound
  by `crates/qcore-c-gateway/tests/resource_kinds_lean.rs`.
* **Arbiter**: AFTER_PLACEHOLDER
