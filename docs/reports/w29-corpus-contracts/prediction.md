# W29-CORPUS-CONTRACTS — prediction, written before the AFTER run

Committed before `measure/runs/after` exists. The BEFORE run has completed and
its byte-identity against `docs/reports/gateway-c-location/artifacts/` is being
checked; the manifest edit selecting `fluent_bit/input_chunk_sizes.qcore` has
not yet been applied.

## The candidate set, bounded from the baseline

The baseline (`docs/reports/gateway-c-location/artifacts/diagnostics.json`)
carries exactly **13** `unsupported` leaves naming the two callees this card
declares, all inside the one corpus unit that defines them
(`fluent-bit-chunk/flb_input_chunk.c`, status provisional `Unsafe`):

* `call to uncontracted function \`flb_input_chunk_get_real_size\`` — 11
  leaves: L18068, L18229, L18424, L18932, L19661, L19723, L20005, L20116,
  L20214, L20418, L20436.
* `call to uncontracted function \`flb_input_chunk_get_size\`` — 2 leaves:
  L18190, L18375.

Every carrying function is `Unknown` and carries other refusals besides.

## Predicted movement

1. The 13 uncontracted-call leaves go to **0**. Some may be **replaced** at the
   same line by the reaching-state refusal the call rule reports next — the
   `unknown_alias: unresolved resource identity \`ic\`` shape cards
   CONSTRUCTOR-RESIDUALS and AOM-BINDING-POISON recorded — wherever the actual
   does not resolve at that site. Net `unsupported` −13 ± replacements.
2. `unknown_alias` may move **down** by a small number: each admitted call was
   previously a refused statement, and a refused statement poisons bindings
   (bounded by constraint 10a); an admitted one does not, so a release or
   guard downstream may now resolve. Replacement leaves (1.) push the same
   number **up**. No prediction beyond "small, attributed leaf by leaf".
3. **Leak pin 266 unmoved.** The declared bodies are read-only size queries:
   no acquisition, no release, no custody transfer. If the pin moves anyway —
   through an unmasked stale-scalar fact or a spared binding reaching
   `report_leaks` — that is a finding to attribute under constraint 7's full
   narrative, not to absorb.
4. **Zero unit status moves, zero function status moves.** A `Safe` appearing
   anywhere is a red flag to investigate, not a win to report.
5. `contracts.json`: sources 18 → 19, manifest functions 39 → 41,
   audit_selection unchanged at 5. `chunk-contracts.json`,
   `source.json`, `source-effects.json`: byte-identical between the two runs.
