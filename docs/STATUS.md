# Current status and evidence boundaries

Re-derived on 2026-09-19 from the sources below. This is the concise status
authority routed by `AGENTS.md`; reports own the underlying evidence. Update
numbers from those sources, not from an older summary. History belongs in the
[increment records](development/increments/README.md).

## Coverage

| Measure | Recorded value | Source |
|---|---|---|
| Full language conformance | **false** | [Acceptance](reviews/r0_3/acceptance.json) |
| CT cases mapped | **47/50**; CT10, CT24, CT41 unmapped | [Manifest](reevaluation/ct_manifest.json) |
| CT case-bound proof | **4/47**: CT01, CT03, CT07, CT37 | Same manifest, per-case `proof` stage |
| CT gated sanitized-native | **10/47, Linux/macOS only** | Same manifest, `native_sanitized` stage and host gates |
| Cycle 19 units | **2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only**, 185 total | [Corpus baseline](reports/w37-corpus-contracts-4/README.md) |
| Projected functions | **213 Safe / 84 provisional Unsafe / 4,537 Unknown**, 4,834 total | Same baseline |
| Corpus diagnostics | leak **272**; unknown_alias **4,309**; double_release **0**; unsupported **49,753**; summary_conflict **1** | Same baseline, `artifacts/diagnostics.json.gz` |
| Contract corpus | **88** modules; **59** resource-v1 declarations; **33** manifest-selected | `contract/qcore/`, [selection](../contract/cycle19-resource-contracts.json) and pinned tests |
| Registered follow-ups | **257** | [Matrix](reviews/r0_3/acceptance_matrix.json), carried entries and derived dimension agree |

**Mapping is not conformance.** CT01/CT03/CT37 bind their model proofs through
the F02 trace/digest relation; CT07 binds through the scoped-task lease ledger,
not F02, whose lease refusal remains intentional. Native execution is observed,
not proved. The sanitizer count carries its host condition; emission assertions
can run on other hosts without implying sanitizer execution there.

**Raw units remain unpromoted.** `flb_input_chunk_create` is function-level Safe;
the production `input_chunk_get` residual remains Unknown. Provisional Unsafe
findings are not validated upstream defects. Unknowns stay in the denominator.
The baseline is `docs/reports/w37-corpus-contracts-4/artifacts/`; an arbiter must
reproduce its predecessor's verdict-bearing artifacts byte-for-byte and
attribute every delta. The chain and leaf identities are preserved in the
individual reports, [wave history](development/increments/README.md), and
[gateway rules](agents/gateway.md#constraint-7).

## Milestones and trust

Cycles 1–17 are complete only at their bounded milestones. Cycle 18 has bounded
implementation with aggregate acceptance pending. Cycle 19 is in progress;
per-path identity imports and observation evidence gate promotion. Cycle 20 is
active: all v1 Safe proposals are refused, including empty scopes; unchecked
Unsafe proposals downgrade to Unknown (`QP010`).

Lean gateway theorems are abstract rule theorems, not extraction of the Rust
analyzer or whole-C correctness. Faithful C extraction and Rust–Lean
correspondence remain premises maintained by review. See
[trust boundary 28](agents/evidence.md#constraint-28). Report model proof,
artifact binding, source coverage, native observations and host conditions
separately.

## Last integrated gate

The [wave-40 close](reports/wave40-close/gate.json) records **4,727 Rust passed,
0 failed, 10 ignored; 238 result summaries over 236 targets; 141 Lean audits;
303 Python tests**, clean fmt/clippy and both probes exit 0. Clang 21.1.8 was
recorded in that measuring worktree. **122 provider tests are carried Unix
evidence, not re-measured.** These are that close's measurements, not a promise
about another host or a later tree. A later card reports its own gate separately.

The ignored F02 Windows pin-agreement test is red by design on `--ignored`;
Linux green does not repair it. Host/load-sensitive tests and their reporting
requirements remain in [rule 16](agents/providers.md#constraint-16).
The summary-count and Clang recording mechanisms still have their disclosed
limitations; see [validation policy](agents/validation.md). This restructuring
does not discharge them or replace the wave-40 gate with its own run.

## Where to go next

[Planning and registered work](README.md#planning) ·
[Rule router](agents/README.md) · [Platform evidence](development/platform-evidence.md).
The pre-restructure ledger narrative is preserved as a
[historical checkpoint](development/increments/structure-checkpoints/status.md),
not a second current status authority.
