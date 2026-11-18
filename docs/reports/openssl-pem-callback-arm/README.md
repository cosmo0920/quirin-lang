# OPENSSL-PEM-CALLBACK-ARM: a literal-argument premise beside a synchronous invocation

User decision 2026-11-18: remove the PEM readers' `x = cb = NULL`
restriction that OPENSSL-PROVIDER-REMAINING-CALLEES left
([report](../openssl-provider-remaining-callees/README.md) §1). Base
`974434463`; the [prediction](PREDICTION.md) was committed first
(`bcdb9b3b2`), the implementation, Lean and lock in `fbe57c2d8`, and formatting in `3efc68959`.

## 1. What changed

* **Loader** (`contract_loader.rs`, the retaining branch; C04.5 "Beside a
  synchronous invocation", C04.19 placement, G05). A premise stands beside
  retaining clauses only when every one is `resource_callback_invoked`. A
  table dispatch, a registration, a registry, element or member clause, a
  retention and every other retaining clause beside a premise still reject
  the module. The narrowest form the PEM readers need: invocations hold
  nothing after the call, so a withdrawn call leaves no state behind.
* **Route G** (`legacy_literal_premise::restrict_to_declared_literals`).
  The premise rewrite renamed only callees in the function map. A callback
  declaration is withheld from that map, so a premise beside an invocation
  would have been loaded and then **ignored**: measured with the route
  change reverted, the fixture's `reuse_arm` (a non-null `x`) was `Safe`.
  The rewrite now treats a retaining declaration as a contracted callee too
  (reading the map directly, so no consultation is noted). An unmet premise,
  a missing actual or a poisoned premise set renames the call before the
  callback route, the retaining environment or any other reader sees it.
  A met call keeps its name, and rule 13u decides the invocation unchanged.
* **Gateway rule 57** (new, `docs/agents/rules/57.md`).
* **`pem.qv`.** Both readers drop `requires resource_literal_argument(cb,
  0)` and state `ensures normal resource_callback_invoked(cb, _, _, _, u)`:
  `buf`, `size` and `rwflag` are the routine's own values, and `u` is the
  caller's ("The u parameter has the same value as the u parameter passed to
  the PEM routine"). The `x` premise stays, so the reuse arm is still
  refused. The OpenSSL checker's `PEM_READ` premises become
  `[NullArgument(1), CallbackInvoked { callback: 2, arguments: [_, _, _,
  u] }]`, so `expected_clauses` derives the new set and requires exactly
  that set. Its test refuses three drifts: the invocation removed, `u`
  dropped to `_`, and the retired `cb` premise restored.
* **Lock.** `quirin-audit pin --load-path providers/user-crates-load-path.json
  --source-root docs/reports/openssl-provider-remaining-callees/evidence/fixture
  --crate quirin-provider-openssl --function PEM_read_bio_X509 --function
  PEM_read_bio_PrivateKey`, after `providers/package.py checkers` at the tip
  (`private/pin-2.log`). Diff reviewed: only the two pem entries'
  `contract_sha256` and `pinned_at` move (`3e91f807…` → `e70f8404…`,
  `a204c108…` → `76c1dcea…`), and nothing else changes. `pin
  --verify-contracts` exits 0 (`0 pinned, 57 flexible`). `providers/contracts.py
  sync` writes nothing, and `check` exits 0. The user path's `contracts` step
  on the predecessor's fixture shows `63 binding(s): 0 Safe / 63 Pending`
  (`private/user-path/`).

## 2. The null callback: a regression in content, as predicted

A null pointer constant at an invoked callback is an open candidate set
(rule 13u, `callback-undecided`). OpenSSL's documented null-`cb` arm ("the
default callback routine is used", or `u` read as the passphrase) has no
spelling in this vocabulary. So the calls that the old `cb = 0` premise
covered now refuse: Monkey's `tls_load_builtin_credentials`
(`tls_openssl.c:176, 188`) and the predecessor fixture's `read_cert`
(`Pending` → `Unknown`, `callback-undecided` plus `reference-drop-unheld` on
its `X509_free`). The OIDC calls (`rdkafka_sasl_oauthbearer_oidc.c:495, 498`)
behave the same way, but this preparation's database has no record of them.
The refusal is fail-closed: nothing becomes `Safe` and no null callback is
read as no invocation. A one-declaration-per-function vocabulary cannot keep
both arms. Restoring the null arm needs a clause meaning "a null `F` invokes
nothing: the callee uses its own default", which this card leaves out:
follow-up `OPENSSL-PEM-NULL-CALLBACK-DEFAULT`, a user decision.

## 3. Tests

* `crates/qcore-contract/tests/callback_literal_premise.rs`: the PEM shape
  loads, is withheld, and has its premise rederived. A retention, an
  invocation beside a retention, a registration, a table and an invocation
  beside a registry invocation, each beside a premise, are rejected, and the
  rederivation refuses too. The premise's own malformations still reject
  beside an invocation. An invocation without a premise is unchanged.
* `crates/qcore-c-gateway/tests/callback_literal_premise.rs` (route G,
  Clang). Within the premise, an identity callback applies the declaration
  (`in_premise`, `in_premise_zero`, `u_is_not_kept` are `Safe`). The
  declaration really applies: a dropped result and an unreleased `u` are
  both leaks. `u` is read and not kept, as the documentation says.
  **A non-null `x` still refuses**: `&slot` and a formal give "outside its
  declared literal-argument premise", with no `callback-*` reading.
  **A callback with no contract in force refuses** (`callback-uncontracted`),
  and so do an effectful one, a null one and a formal (`callback-undecided`).
  A callback that dereferences `u` puts the requirement on the call, exactly
  as a direct call does.
* The OpenSSL checker test above (`a_drifted_shape_of_the_new_readings_is_refused`).

## 4. Lean

`proof/QCore/CallbackLiteralPremise.lean`, audited by
`proof/AuditCallbackLiteralPremise.lean`: 21 theorems, axioms within
`propext`/`Quot.sound`, with no placeholder and no project axiom. The
admission theorems are `a_premise_stands_only_beside_invocations`,
`every_other_clause_beside_a_premise_rejects` and
`invocations_with_a_premise_are_admitted`. Withdrawal is covered by
`an_unmet_premise_withdraws_a_callback_declaration`,
`a_mismatched_actual_withdraws`, `a_missing_actual_never_satisfies` and
`a_poisoned_set_withdraws_every_declared_callee`. Within the premise:
`a_met_premise_defers_to_the_invocation`, `applied_only_over_identities`
and the three refusals. The rewrite: `the_new_rewrite_refuses_at_least_the_old`,
`the_rewrite_is_unchanged_on_the_function_map` and
`no_premise_is_transparent`. The PEM vectors include
`pem_old_rewrite_missed_the_reuse_arm`.

## 5. Measurement

**Opt-in `current`** ([script](scripts/optin.sh), the seven-file selection
of REFERENCE-DROP-NULL-ACCEPTING, contracts from the generated user load
path). Inputs come from `quirin-audit prepare` at `046accf6`, run by the
base binary from the local Fluent Bit clone (`private/prep/system`). Base
side: the base export with its checkers and binary. Tip side: this tree,
its checkers and binary. Both exit 0 (`private/optin/{base,tip}/run.log`).

| | base | tip |
|---|---|---|
| units (7) | 7 Unknown | 7 Unknown |
| function rows | 422: 131 Safe / 2 Pending / 289 Unknown | 422: 131 / 2 / 289 |
| checker bindings | 63 Pending | 63 Pending |

[compare.json](artifacts/compare.json) ([script](scripts/compare_optin.py)):
**0 moves, 4 rows change content**, as predicted. No row becomes `Safe`, and
there is no new `Unsafe`.

The seven calls and what blocks each at the tip:

| Call | Base | Tip blocker |
|---|---|---|
| `rdkafka_cert.c:272` (`while ((x509 = PEM_read_bio_X509(…)))`) | unmodeled call in loop condition | the same: the route never reads a call in a loop condition |
| `rdkafka_cert.c:349` (`cert->x509 = PEM_read_bio_X509(…)`) | outside the premise | the store into a member: "indirect memory access is outside the resource projection"; no callback refusal is reported (`rd_kafka_conf_ssl_passwd_cb`) |
| `rdkafka_cert.c:405` (`cert->pkey = PEM_read_bio_PrivateKey(…)`) | outside the premise | the same member store |
| `rdkafka_ssl.c:761` (`rd_kafka_ssl_PKEY_from_string`) | outside the premise | `callback-uncontracted`: `rd_kafka_transport_ssl_passwd_cb` has no contract in force (its body calls `rd_kafka_dbg`/`rd_kafka_log`) |
| `rdkafka_ssl.c:790` (`PEM_read_bio_X509(in, &ca, password_cb, …)` in an `if`) | unmodeled call in branch condition | the same; and `&ca` at `x` is the reuse arm, outside the premise by design |
| `rdkafka_ssl.c:833` (`rd_kafka_ssl_X509_from_string`) | outside the premise | `callback-uncontracted`, the same callback |
| `rdkafka_ssl.c:1292` (`while ((x509 = PEM_read_bio_X509(…)))`) | unmodeled call in loop condition | the same |

`tls_load_builtin_credentials` gains `callback-undecided` at both null-`cb`
reads, plus `reference-drop-unheld` on its six `X509_free(cert)` /
`EVP_PKEY_free(key)` drops (§2), and stays `Unknown`.

**Pinned routes.** These were not measured on this host. By construction
they cannot move. No pinned route reads the user load path, so `pem.qv`
reaches none. The loader change admits only declarations that it rejected
before, so no pinned contract set holds one. The route G change acts on a
retaining callee only when the callee carries a premise (none can exist in a
pinned set) or when the premise set is poisoned (no pinned route runs
poisoned, since poisoning refuses every contracted call). The prediction's
`measure-prediction` block (no move on corpus, system or slice) is
therefore left to the integrator's `quirin-audit measure --declared`. This
is disclosed as OPENSSL-PROVIDER-REMAINING-CALLEES disclosed it.

## 6. Gate

`python3 scripts/gate_scope.py --base 974434463 --out private/gate-1/gate`
through `private/toolchain/env.sh` (Clang 21, Lean 4.35.0) on `fbe57c2d8`.
The mode is full, because the diff reaches `qcore-c-gateway` and
`qcore-contract`. Record: [`gate/gate-1.json`](gate/gate-1.json); logs:
`private/gate-1/gate/`.

| Step | Exit | Counts |
|---|---:|---|
| submodules | 0 | 1 |
| `fmt` | **1** | rustfmt layout in the two new tests and one loader line |
| `clippy` | 0 | |
| Rust suite | 0 | 657 units, 702 summaries; 9649 passed, 0 failed, 21 ignored, 148 filtered |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| Lean (`check_lean_audits.py`) | 0 | 446 audits |
| Python suite | 0 | 779 tests |
| provider package build, verify-all, verify `qcore.libevent` | 0, 0, 0 | |
| provider tests | 0 | 49 summaries; 150 passed, 0 failed, 1 ignored |
| provider `fmt` | **1** | the same three files (the check spans the workspace) |
| provider `clippy` | 0 | |

`3efc68959` applies `cargo fmt --all`, a whitespace-only change. The reruns
are `cargo fmt --all -- --check` (exit 0) and `cargo fmt --manifest-path
providers/Cargo.toml --all -- --check` (exit 0), logged in
`private/gate-1-reruns/`. Earlier and outside the gate:
`cargo test -p qcore-c-gateway --test callback_literal_premise` (4 passed);
the same test with the route change reverted (`a_non_null_x_stays_outside_the_premise`
failed with `reuse_arm` `Safe`); and `quirin-provider-openssl`'s tests
(exit 0, `private/openssl-tests.log`).

## 7. Follow-ups

[`integration-amendments.json`](integration-amendments.json) retires
`OPENSSL-PEM-CALLBACK-ARM` and registers:

* `OPENSSL-PEM-NULL-CALLBACK-DEFAULT`: the null-`cb` arm (§2). This needs a
  user decision.
* `RDKAFKA-PASSWD-CB-CONTRACTS`: `rd_kafka_transport_ssl_passwd_cb` has no
  contract in force (two calls). Reading `rd_kafka_conf_ssl_passwd_cb` at
  `rdkafka_cert.c:349, 405` additionally needs the member store of a
  returned reference.

Files likely to conflict: `providers/quirin-provider-openssl/contracts.lock`
(regenerate with `quirin-audit pin`), `src/documented.rs` and `src/check.rs`
(OPENSSL-SSL-CTX-CALLBACK-SLOTS), `contract_loader.rs` (the retaining
branch), C04.19's placement paragraph, and the generated `docs/agents/gateway.md`,
`docs/agents/README.md` and `proof/QCore.lean` (rerun the generators).
