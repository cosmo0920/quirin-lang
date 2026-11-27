# W7 generic Lean names

Date: 2026-09-07. The three bounded naming targets now expose generic names,
with source compatibility for the historical model APIs. No model behavior,
acceptance condition or proof obligation changed.

| Historical name | Canonical name | Compatibility |
| --- | --- | --- |
| `QCore.MonkeyEvent` | `QCore.DescriptorCustody` | Old module exports all 221 historical public declarations, including generated structure helpers |
| `MonkeyComposition`, `checkMonkeyComposition` | `EventComposition`, `checkEventComposition` | Type, checker, certificate, predicate, field and derived-instance aliases remain |
| `busy_effect_preserves_chunkio_lock` | `busy_effect_preserves_lock` | Old theorem is a direct proof wrapper |

The descriptor implementation moved into `proof/QCore/DescriptorCustody.lean`;
`MonkeyEvent.lean` is now a compatibility export module. The generic teardown
theorem is `accepted_descriptor_teardown_is_complete_and_unique`; its historical
name also remains available. `QCore.lean` retains the old module import so clients
importing the whole project still receive those aliases.

`EventComposition` stays in `QCore.ExternalIntegration`; historical composition
field wrappers use explicit old-type parameters to preserve Lean field notation.
The existing `accepted_monkey_*` theorem names remain for adapter compatibility.
The Cycle 19 audit now uses the canonical descriptor and lock theorem names.

[name-parity.json](name-parity.json) records successful source comparisons:
all definition/proof bodies are byte-identical after the stated name substitutions,
excluding added compatibility wrappers and the descriptor introductory comment.
The new `AuditGenericNames.lean` checks old/new checker, predicate and field
identities with `rfl`, exercises historical field notation, and prints both
canonical and legacy theorem axioms.

## Validation

The full standing gate passed: 473 workspace tests, formatting, clippy, both
reevaluation probes and the 53-job Lean build. All 18 `Audit*.lean` files passed,
including 7,493 declarations in the compiled QCore axiom-closure audit. Only
`propext`, `Classical.choice` and `Quot.sound` appear; no project axioms or
placeholders were introduced. [audits.log](audits.log) preserves the run.

The workflow itself remains unrun until a push. Diagnostic renumbering and
broader provider/model architecture remain separate work; this change only
addresses the named generic Lean wrappers.
