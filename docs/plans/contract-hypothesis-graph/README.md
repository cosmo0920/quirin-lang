# Contract hypothesis graphs and symbolic premises (`C04.18`)

> [!NOTE]
> **Current status (Graph v1–v4 & Symbolic Formula v1 implemented; Guarded Contract Adoption halted).**
> - **Hypothesis Graph v1–v4** (`C04.18.1`, `C04.18.2`, `C04.18.7`, `C04.18.8`) and **Symbolic Premise Statements v1** (`C04.18.9`, `SP1`–`SP10`, enforcing global prose bans `QF001` and `QF002`) are **implemented and active** across all provider contract bundles.
> - **Guarded contract adoption** (`C04.18.5`–`C04.18.6`) is **explicitly halted at the user's request** ([supplemental card](../../reports/guarded-contract-adoption/SUPPLEMENTAL_CARD.md)) and requires explicit human authorization before restart.
> - Normative contract rules live in [`docs/QCORE_CONTRACT_SPEC_r0_3.md`](../../QCORE_CONTRACT_SPEC_r0_3.md#c04189-symbolic-premise-statements-formula-version-1) (`§C03.1`, `§C04.18`) and [`docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md`](../../QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g01p-typed-contract-propositions-graph-version-3) (`G01p`/`G01c`/`G01t`/`G01e`, `G05-formula`).

## 1. Evolution from opaque prose premises to typed mathematical atoms

| Stage | Spec & Grammar | Standing and evidence |
|---|---|---|
| **Graph v1** (`CONTRACT-HYPOTHESIS-GRAPH-V1`) | `C04.18.1` | **Completed** ([report](../../reports/archive/contracts-and-premises/contract-hypothesis-graph-v1/README.md)): acyclic content-addressed premise/hypothesis DAG, dispositions vs. warrants (`derived`, `observed`, `declared`, `human-reviewed`, `provider-proposed`), context-key equality before conjunction, per-row consultation closure. |
| **Graph v2** (Neutral decomposition & binding refs) | `C04.18.2` | **Completed**: explicit formal binding references and neutral premise decomposition. |
| **Graph v3 & v4** (`quant proposition` & `quant condition`) | `C04.18.7`–`18.8`, `G01p`, `G01c` | **Completed**: first-order mathematical formulas over unbounded `integer` and domain sorts, explicit `requirements` retention, and `entry`/`normal`/`error(E)`/`any` conditions. |
| **Formula v1 & Prose Ban (`SP1`–`SP10`)** | `C04.18.9`, `G05-formula` | **Completed**: all provider premises migrated to symbolic formula v1; prose string premises (`QF001`) and string hypothesis atoms `resource_hypothesis` (`QF002`) are globally refused on every route. |
| **Guarded Decomposition (`v1` / bridge `v2`)** | `C04.18.5`–`18.6`, `G05-guarded` | **Halted at user request** — [GUARDED-CONTRACT-ADOPTION-SUPPLEMENT](../../reports/guarded-contract-adoption/SUPPLEMENTAL_CARD.md). `PREMISE-FLOW-PRODUCTION-DISCHARGE` and production proof discharge of `quant theorem`/`quant effect` remain open/proposed. |

---

## 2. Core soundness invariants (preserved across all graph versions)

1. **Truth disposition and warrant are orthogonal.** Recording `declared`, `human-reviewed`, `observed`, or `provider-proposed` describes provenance only; it never converts an assumption or bounded observation into a derived proof (`Safe` requires discharged obligations under explicit premises).
2. **Context compatibility before conjunction.** Hypotheses bind `(artifact, property, semantic profile, scope, bounds, source identity, evidence version)`; atoms from mismatched source pins, profiles, or bounds never conjoin to discharge an obligation.
3. **Total-or-reject loading & replay.** Duplicate IDs with differing content, missing children, cycles, unknown predicates/sorts, retired prose strings (`QF001`/`QF002`), or replay digest mismatches reject the contract module rather than falling back to weaker premises.
4. **Inheritance and consultation fidelity.** Contract inclusion (`#include`) and `contract inherit` preserve parent hypothesis graphs without shedding obligations, and per-verdict dependency reporting (`C06.5`) records the exact transitive closure consulted.
5. **Static vs. runtime outer-contract separation.** Outer-contract projections (`C04.20`, gateway rule 31) apply only in the separate runtime scope (`Pending` under `outer-contract`) and never alter static verdicts.
