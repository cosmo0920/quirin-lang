# OPENSSL-RELEASED-PARAMETER-SLOTS: slot invocations through a released parameter

* **Base:** main `9193b196a`. **Card:** the released-parameter part of
  OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS, under the **USER DECISION of
  2026-11-23**: "Released-parameter wording (OpenSSL): if yes,
  EVP_MD_CTX_free drops its premise, and PKCS12_free keeps it only for the
  CRL item. -> launch them, too."
* **Commits:** [PREDICTION](PREDICTION.md) `741db5c4f` (before any
  implementation); implementation with lock and Lean `c1c2fbafe` (the
  measured tip); rustfmt `021fdd978`; gate fixes `615d20727`; this report.

## 1. Sources

OpenSSL 3.5.5 `openssl-3.5.5.tar.gz`, sha256
`b28c91532a8b65a1f983b4c28b7488174e4a01008e29ce8e69bd789f28bc2a89` (the
digest r45 and r53 recorded; fetched from the release's GitHub download,
since no copy was on the host), unpacked in `private/src/`, not committed.
Read for this card:

| Free | What it runs (3.5.5) | Stated |
|---|---|---|
| `EVP_MD_CTX_free` | `evp_md_ctx_reset_ex`: `EVP_PKEY_CTX_free(ctx->pctx)` (`pmeth->cleanup(pctx)`, `pmeth_lib.c:394-395`; provider operation `freectx`; `EVP_PKEY_free` of `pkey`/`peerkey`, so a key's nine; `ENGINE_finish`, `:407`); `evp_md_ctx_clear_digest`: provider `freectx`, `cleanup_old_md_data` (`digest->cleanup(ctx)`, `digest.c:32-34`), `ENGINE_finish(ctx->engine)` (`:64`), `EVP_MD_free` (provider teardown at the last reference). A legacy MAC public key method frees a cipher context: `cipher->cleanup` (`evp_enc.c:57`) | key's nine, `evp_md_st.md_meth_cleanup` (handed `ctx`), `evp_cipher_st.cipher_meth_cleanup`, `evp_pkey_method_st.pkey_meth_cleanup`; provider dispatch functions under the two teardown premises. **Nothing left** |
| `PKCS12_free` | the PKCS#7 safes: a signer info's key (`pk7_asn1.c:141`) and a recipient's certificate (`:184`), a signed safe's certificates and CRLs; bags hold certificates and CRLs as octet strings (`p12_asn.c:52-53`). A CRL's `crl_free` (`x_crl.c:259-261`) | key's nine. **Left:** a CRL's `X509_CRL_METHOD` free (OPENSSL-SLOT-REGISTRATION-INTO-RESULT) |

All twelve setters were already declared (r53's `teardown_slots.qv`), so no
setter, prototype or page snapshot was added. No unstated item was found
on `EVP_MD_CTX_free`, so it drops the premise as the decision foresaw.

## 2. The vocabulary (C04.19.3, rule 12n, amended in place)

* **Loader** (`contract_callback.rs`, `contract_loader.rs`). A
  `resource_callback_slot_invoked` object may be a depth-one, kindless
  `resource_released` parameter. Such a declaration states nothing beside
  its custody contract and premises but slot invocations: a registration,
  synchronous, table, registry or plain member invocation, a retention,
  reset, slot move, reference clause, arm, constructed state, hand-off,
  member-path state or helper, cell, container effect, literal-argument
  premise or return-code classes beside it reject ("a slot invocation
  through a released parameter stands only beside the custody contract,
  premises and other slot invocations"). A consumed, owned or other object
  still rejects with the old text; the borrowed form is unchanged.
* **Gateway** (`legacy_retained_by.rs`, new reason `callback-beside-release`
  in `callback_contracts.rs`). The transfer applies the custody contract,
  release included, before the callback route. So at a call stating a slot
  through a released parameter: each slot is decided exactly as before; a
  candidate's `resource_owned(n)` requirement is folded into the call's
  custody mode, read before its effects, and at the released position a
  non-null requirement makes a nullable release non-null; a candidate effect
  the callback route would apply after the release (member-path state,
  deferred member reading, registry use) refuses the call. A use after the
  call is the release's: it refuses as before.
* The soundness points the brief named: nothing reads the object after the
  release (the refusal above; Lean `nothing_reads_the_object_after_the_release`);
  the invocation's effects are the slot's declared ones (identity candidates
  over the closed run set, unchanged); a parameter whose release the
  declaration does not state reads as before (borrowed form, or refused);
  every other form refuses as today (`the_admission_only_grows`,
  `the_admission_grows_only_by_released_slot_invocations_alone`).

## 3. The declarations and checker

* `evp_digest.qv`: `EVP_MD_CTX_free` states 12 slot invocations on `ctx`
  (every actual `_` but the digest cleanup's `ctx`; the engine enumerators'
  `supplied_nullable` as everywhere) and no longer names
  `openssl_unstated_callback_slots`.
* `pkcs12.qv`: `PKCS12_free` states the key's nine on `a` and names the
  free statement `quant all #o in #freed: #teardown(#o) \ (#stated |
  #settable) |-> #inert` (as `X509_STORE_free`) instead of the released
  one. The CRL free is no setter's (`#settable` does not hold it), so the
  free statement covers exactly it.
* Checker (`documented.rs`, `check.rs`): both are `Frees::Runs`
  (`DIGEST_CONTEXT_TEARDOWN` with nothing left; `KEY_TEARDOWN` leaving
  `CRL_METHOD_FREE`). `Frees::RunsThroughRelease`,
  `Premise::UnstatedReleasedFreeSlots`, `UNSTATED_RELEASED_FREE_STATEMENT`
  and `RELEASED_ARGUMENT` are removed: no declaration names the released
  statement, and a module naming it now fails the expected-text check.
  Counts: 107 premise clauses refused as prose (was 108); the frontend binds
  13 free statements, 0 released (was 12 and 2).

## 4. Lock and regeneration

Regeneration in r46/r53's order (`private/logs/regen-1..7.log`): build
`quirin-audit` and `quirin-plugin` (release), `quirin-plugin load-path`
(the views emptied, as always), `package.py checkers` exit 1 as expected,
`quirin-audit plugin views --write`, `quirin-audit pin --load-path
providers/user-crates-load-path.json --source-root
docs/reports/openssl-provider-remaining-callees/evidence/fixture --crate
quirin-provider-openssl` (exit 0), `providers/contracts.py sync` (0),
`package.py checkers` (0). `providers/contracts.py check` exit 0.

`contracts.lock` against `9193b196a`: **0 added, 2 changed, 0 removed**
(157 entries, 155 unchanged), `contract_sha256` and `pinned_at` only:

* `EVP_MD_CTX_free` `1d3f0d39…` → `e0978c39cff1eb4c33954c54792a148730f96d831aba9fb8a4b58b6a23002566`
* `PKCS12_free` `3ec1b53c…` → `b1fe7caa90260df1cc7d19ca05895b248a9c20199adf266bf67d58271ed0e0df`

`user-crates-load-path.json` and `quirin-plugin.json` are unchanged (no
binding added). The OpenSSL crate has no `contract-inventory.lock`, so no
STATUS corpus figure moves.

## 5. Lean

New `QCore.OpensslReleasedSlots`, audited by
`proof/AuditOpensslReleasedSlots.lean` (16 theorems; axioms within
`propext`, `Quot.sound`): admission (`the_admission_only_grows`,
`the_admission_grows_only_by_released_slot_invocations_alone`,
`another_object_mode_still_rejects`,
`a_released_slot_beside_another_clause_rejects`); the reading
(`the_reading_is_the_object_before_the_release`,
`the_route_after_the_release_reads_a_released_object`,
`a_member_effect_beside_a_release_refuses`, `a_use_after_the_release_refuses`,
`a_non_null_requirement_strengthens_the_release`,
`applied_only_over_closed_identity_slots`,
`nothing_reads_the_object_after_the_release`); the frees
(`the_premises_cover_exactly_what_remains`: the free statement leaves
uncovered exactly `PKCS12_free`'s CRL free and nothing of
`EVP_MD_CTX_free`'s; `the_decision_holds`; vectors).
`QCore.OpensslFreeTeardownSlots` loses the `throughRelease` reading:
`a_released_free_states_no_slot` is replaced by
`the_released_statement_is_named_by_no_reading`, and its crate vectors
read the two frees as landed. `scripts/gen_lean_roots.py` and
`scripts/gen_rule_docs.py` rerun.

## 6. Tests

* `crates/qcore-c-gateway/tests/openssl_undeclared_handshake_callbacks.rs`
  (33 tests, 4 new): `a_slot_invocation_through_a_released_parameter_loads_alone`
  (admitted; five neighbouring clauses refused; consumed and owned objects
  refused; borrowed unchanged); `a_released_slot_is_read_before_the_release`
  (identity decides on a checked and a possibly null free; a use after the
  release refuses; a non-null candidate decides the checked free and
  refuses the unchecked one; a member-path candidate refuses
  `callback-beside-release`; a releasing candidate refuses; no run facts
  refuses `callback-member-open`); `the_released_frees_state_their_teardown_slots`;
  `the_released_frees_decide_over_the_run` (both frees `Safe` with nothing
  registered, the row's dependencies list the free; a double free refuses;
  an uncontracted digest cleanup refuses `EVP_MD_CTX_free` and spares
  `PKCS12_free`; without the setters' module `no declaration registers`;
  without run facts refused). The existing reader rows (`read_p12`) and
  the borrowed-parameter frees keep passing unchanged.
* `crates/qcore-audit/tests/contract_dependencies_report.rs`:
  `the_released_frees_name_the_unstated_premise_only_where_stated` (the
  dependency output's premises: `EVP_MD_CTX_free` without
  `openssl_unstated_callback_slots`, `PKCS12_free` with the free statement;
  both with the ex_data and two teardown premises).
* Provider crate: 27 unit tests pass (the free audit tests rewritten for
  the removed reading).

## 7. Measurement

**Pinned routes: no move predicted, not run.** Corpus, system and slice
read no user load path and no pinned module carries a slot invocation, so
the new admission and refusal read nothing there. This host has no
`build/cycle19` inputs, so `quirin-audit measure --declared` cannot run
here; the prediction's `measure-prediction` block (none on all three) is for
the integrator.

**The opt-in `current` run** (this card's arbiter, as r36 and later
OpenSSL cards ran it): `quirin-audit current --monkey-linked` over Fluent
Bit 046accf6 with OPENSSL-SSL-CTX-CALLBACK-SLOTS'
[script](../openssl-ssl-ctx-callback-slots/evidence/opt-in/opt-in-current.sh)
(`SOURCE_DATE_EPOCH=1767225600`, `--jobs 12`), driver
[`measure.sh`](evidence/opt-in/measure.sh). Each side is a `git archive`
tree with its own release `quirin-audit` and its own `package.py checkers`
(`checkers.exit` 0 on both); inputs prepared by the base binary's `prepare`
into `private/fb-046accf6` (exit 0). The runs overlapped each other and the
gate, so wall times are not comparable.

| Run | Tree | Files / configurations | Functions Safe / Pending / Unknown / Unsafe | Bindings |
|---|---|---|---|---|
| base (`private/opt-in/base-1`, 01:12–02:15Z) | `9193b196a` | 180 / 181: 9 Safe, 0 Pending, 7 Unsafe, 161 Unknown, 4 data-only | 1171 / 2 / 3623 / 8 | 163, all `Pending` |
| tip (`private/opt-in/tip-1`, 01:44–02:34Z) | `c1c2fbafe` | same | 1171 / 2 / 3623 / 8 | 163, all `Pending` |

Both exit 0, no unmatched row; [comparison](evidence/opt-in/comparison-base-tip.json)
by OPENSSL-PROVIDER-REMAINING-CALLEES' `evidence/compare.py`: **0 status
moves, 0 content changes, 0 unit moves**, as predicted.

**Dependency lines.** Two rows rest on the frees, both librdkafka and
`Unknown` on both sides (they refuse first elsewhere, as at base):
`rd_kafka_cert_new` (`rdkafka_cert.c`) and `rd_kafka_ssl_set_certs`
(`rdkafka_ssl.c`), each listing `PKCS12_free`. Their fields differ only in
identity digests (the revision in `evidence_version`, the `assumptions`
digest of their premise set, the plugin package digest). The report's
`contract_premises` for the two declarations changed exactly as predicted:
`EVP_MD_CTX_free` no longer names `openssl_unstated_callback_slots`;
`PKCS12_free` names it with `#teardown(#o) \ (#stated | #settable)` instead
of `#teardown(#o) \ #stated`. Both now appear under
`callback_members/declarations` (read by the member pre-pass, an empty
store list each). No row names `EVP_MD_CTX_free` at either side (the
predicted `flb_crypto.c` and OIDC rows do not list it), so the predicted
riskiest move could not occur in this corpus.

## 8. Gate

**Record:** [`gate/gate-3.json`](gate/gate-3.json). **Command:** `sh
private/toolchain/env.sh python3 scripts/gate_scope.py --base 9193b196a
--out private/gate-3` (Clang 21, Lean 4.35), log `private/logs/gate-3.log`.
Tree `615d20727` with this report directory uncommitted (docs only).
Scoped; Lean all; the provider gate included. **Exit 0, 14 steps passed.**

| Step | Exit | Counts |
|---|---:|---|
| submodules, `fmt`, `clippy` | 0, 0, 0 | `third_party/libevent` checked out |
| Rust suite (scoped) | 0 | 678 units, 723 summaries; 9920 passed, 0 failed, 21 ignored, 148 filtered |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| Lean (`check_lean_audits.py`) | 0 | 487 audits (new `AuditOpensslReleasedSlots`) |
| Python suite | 0 | 807 tests |
| provider package build, verify-all, verify `qcore.libevent` | 0, 0, 0 | |
| provider tests | 0 | 49 summaries; 174 passed, 0 failed, 1 ignored |
| provider fmt, provider clippy | 0, 0 | |

**Earlier attempts,** retained and discarded as evidence:
`private/gate-1` (tree `c1c2fbafe`) was stopped after `cargo fmt --check`
showed the new tests unformatted (fixed in `021fdd978`);
`private/gate-2` (tree `021fdd978`) exited 1: Python 1 failure of 807
(`test_gate_scope_census`: the new audit test read the OpenSSL contract
directory by a path literal the census does not declare; now
`include_str!` per module) and provider fmt (an import line in `check.rs`),
every other step 0 with the counts above; both fixed in `615d20727`.

**Outside the gate** (`private/logs/`): the provider crate's unit tests (27,
`provider-unit-1.log`), the gateway target (33, `gtest-4.log`), the
contract and frontend targets (`ctest-1.log`), the audit dependency target
(13, `atest-2.log`), `lake build` of both Lean modules (`lean-1.log`) and the
new audit's axioms (`lean-audit-1.log`).

## 9. Follow-ups and conflicts

[`integration-amendments.json`](integration-amendments.json) retires
OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS (both frees now state their slots;
the CRL item is OPENSSL-SLOT-REGISTRATION-INTO-RESULT's, the QUIC TLS item
OPENSSL-DISPATCH-TABLE-SLOT-REGISTRATION's), appends to
OPENSSL-SLOT-REGISTRATION-INTO-RESULT (PKCS12_free can state the CRL slot
through its released argument once a registration exists), and registers
CALLBACK-MEMBER-RELEASED-REQUIREMENT-FOLD (the plain member form through a
nullable released object would drop a candidate's non-null requirement; not
reachable with shipped contracts). No STATUS edit: no corpus figure moves;
the Lean audit count (one new audit) is the landing gate's figure.

**Files likely to conflict:** `providers/quirin-provider-openssl/src/{documented,check}.rs`,
`contracts.lock`, `evp_digest.qv`, `pkcs12.qv`;
`crates/qcore-c-gateway/src/legacy_retained_by.rs` (the callback/reference
block, not the `&`-token readers), `callback_contracts.rs`;
`crates/qcore-contract/src/contract_loader.rs`, `contract_callback.rs`;
`crates/qcore-c-gateway/tests/openssl_undeclared_handshake_callbacks.rs`;
`crates/qcore-frontend/tests/premise_formula.rs`;
`docs/QCORE_CONTRACT_SPEC_r0_3.md` (C04.18, C04.19.3), `docs/agents/rules/12n.md`;
generated `docs/agents/gateway.md`, `docs/agents/README.md`,
`proof/QCore.lean`.
