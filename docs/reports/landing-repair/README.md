# LANDING-REPAIR

Card LANDING-REPAIR (2026-10-27) repairs the test failures and the one record
drift that stopped integrator gate 7 on the landing base `c745b3ee2`
(`integrate/landing-fix-base`: origin/main `f774f54de`, Codex's repair
`508407913`, GATEWAY-RULE-API and CONTRACT-SUPERSESSION). The card prompt
named three failures. Gate 7's provider run stopped at the first failing
binary, so a `--no-fail-fast` run found three more, which are items 5 and 6.

The card changes no analyser, contract text, lock or Lean file. Its one
non-test source change adds an index entry to the data-only
`quirin-provider-cfl` crate. That crate's `CONTRACTS` has no consumer outside
its own crate and the shared crate's load-path test. No verdict moves, so no
identity run is needed.

## Root causes and fixes

1. **`contract_tip_mode::a_tip_entry_keeps_its_payload_and_version_four_review_stamps`**
   (`crates/qcore-audit/tests/contract_tip_mode.rs`). The panic was not a
   missing stamp. Codex's helper `assert_same_pin_payload_with_monotone_stamps`
   read `before["tip"].as_array().unwrap()`. Its second call compares against the
   reviewed-only lock, and that lock has no `tip` key:
   `ContractLock::tip` is `skip_serializing_if = "Vec::is_empty"`, a format
   rule since CONTRACT-PIN-TIP-MODE. The pin tool is correct. Tip entries are
   stamped when tracked (`contract_pin.rs`, `track`). Every other run keeps
   them unchanged, including `--regenerate`. The test's own `old_tip["tip"] ==
   new_tip["tip"]` assertion passes. The committed libevent lock's tip entry
   `mk_vhost_set` carries `2026-10-25T18:10:00Z`, and
   `pin --verify-contracts` passes on all 17 locks.

   The fix is in the test. The helper now requires both locks to agree on
   whether each section is present. It treats an absent section as empty. It
   still requires a valid stamp on every present entry, and it still requires
   stamps never to move backwards. Spec C06.6 requires a stamp on every entry.
   It does not require an empty `tip` section to be written. A stale comment
   ("version-2 lock, byte for byte") was also corrected.

2. **`cmetrics_unit::cmetrics_process_checks_unit_paths_and_rejects_scalar_or_unbound_claims`**
   (`providers/conformance/tests/cmetrics_unit.rs`). This was not a parser
   regression. The quant migration `6bcda2983` rewrote
   `counter_reset.qv`. Its legacy `requires field_is_boolean` became the typed
   entry condition `#old_field_domain(#old_field) = ((#old_field == 0) ||
   (#old_field == 1))`, and its postcondition became `#exit_1(#field) =
   (#field == 1)`. The test's blanket `contract.replace("field == 1", "field ==
   0")` now also turns the entry domain into `(#old_field == 0) || (#old_field
   == 0)`. `boolean_condition_profile::entry_requirement` correctly refuses
   that domain as not exactly `{0, 1}`. The false-policy form, a changed
   normal exit value, is still supported.

   The fixture now changes only `(#field == 1)` to `(#field == 0)`. The
   verifier must still refuse that change (`Unknown`, no certificate). The
   test adds two assertions. It checks that the contract holds exactly two
   `field == 1` occurrences and one `(#field == 1)`. It also keeps the blanket
   rewrite as a new negative case: derivation must refuse the corrupted entry
   domain. Every earlier rejection is kept, and one rejection is added.
   `composed_five_providers.rs` used the same blanket rewrite with
   `.expect("derives")`. Gate 7 never ran it, because the provider run
   stopped at the first failing binary. It is repaired the same way and
   still requires `Unknown` with `QP009` and no certificate.

3. **`scripts/tests/test_contract_corpus.py`** expected 55 `quirin-provider-*`
   modules and found 56. The added module is `cfl/atomic.qv` in
   `quirin-provider-cfl`, the CFL conditional own-event model added by
   quant-contract-hypotheses `6ecb5498d`. Codex's `508407913` raised the
   total from 143 to 144 but left the crate count at 55. The count is now 56,
   with a provenance comment.

4. **Record drift (not in the card prompt; found in gate 7's `python.log`).**
   `test_build_matrix_sections` failed with `dimensions.named_followups` at 851
   against 862 entries. Integration commit `ae6742618` registered eleven
   follow-ons through `apply_amendments.py` but did not regenerate the matrix.
   The matrix was regenerated with `docs/reports/remaining-d02/build_matrix.py
   <root>`, which changed exactly that one line (851 to 862). The
   regeneration is a separate commit, so the integrator can drop it and
   regenerate during landing instead.

5. **`quirin-provider-cfl` `index_is_exactly_the_contract_tree` and
   `quirin-provider-shared` `load_path_manifest_matches_the_linked_crates`.**
   `6ecb5498d` added `cfl/atomic.qv` to the crate's contract tree and to
   `providers/contract-load-path.json`, but not to the crate's `CONTRACTS`
   index. The fix adds the index entry. Both tests then pass unchanged.

6. **`composed_five_providers` chunkio false policy.** The chunkio contract
   `lock_state.qv` was migrated to `#exit_1 ... = (#result == #old_lock)`, so
   the test's legacy rewrite `result == old_lock` matched nothing. The
   "false" contract was therefore the true one, and it came back `Safe`. The
   rewrite is now `(#result == #old_lock)` to `(#result == 7)`. The test
   also asserts that each rewrite matches exactly once, so a no-op mutation
   fails at its source and can no longer pass for a falsified contract. The
   `configured_source_contract` and `configured_audit` fixtures still use the
   legacy `ensures` spelling, so their rewrites still apply.

## Finding left open

`pin --regenerate` starts from no lock (`contract_pin.rs`, `existing = None`).
Each regenerated entry's stamp is therefore not compared with the stamp of the
entry it replaces. An explicit `--pinned-at` earlier than the replaced stamp is
accepted, and so is a default clock stamp when the clock is behind the lock.
This conflicts with C06.6: "a re-pin ... carries a later stamp than the entry
it replaces". The finding comes from code reading, not from a failing run, and
is registered as `PIN-REGENERATE-STAMP-MONOTONICITY`.

## Verification

All logs are in `/media/Data/GitHub/quirin-lang/private/landing-repair/`. The
commands, exits and counts are in the card's hand-back to the integrator.
