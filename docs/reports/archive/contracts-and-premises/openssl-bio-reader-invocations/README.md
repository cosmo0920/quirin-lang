# OPENSSL-BIO-READER-INVOCATIONS: the BIO slots beside a literal-argument premise

Base `dd0225012` (integrate/r32); main `54d067729` (r32 landed, adding
only its gate record) was merged in `a3896c10c`, and the gate and the
measurement compare against it. Named follow-up
`OPENSSL-BIO-READER-INVOCATIONS` (recorded by OPENSSL-BIO-METHODS-AND-CALLBACKS);
user decision 2026-11-19. [PREDICTION](PREDICTION.md) was committed before
any implementation (`855bddfeb`). The implementation, lock and Lean are
`871181001`.

## 1. The readers were silent

`PEM_read_bio_X509`, `PEM_read_bio_PrivateKey` and `d2i_PKCS12_bio` read
through the caller's BIO. A read through a BIO runs its callback, which is
"called during most high-level BIO operations" (`BIO_set_callback` 3.5),
and the functions of its method table, each called "in response to" the
operation it implements (`BIO_meth_new` 3.5). The declarations stated
neither and named no premise for them. The slots have been stated since
r32 (`bio_slots.qv`), but rule 57 admitted only a synchronous invocation
beside the readers' literal-argument premise (`x = 0`, `p12 = 0`).

## 2. Rule 57 amended in place

**Loader** (`contract_loader.rs`, the retaining branch): beside a premise,
every callback clause must now be `CallbackClause::Invoked` (as before) or
`CallbackClause::MemberInvoked { slot: true }`, the opaque slot invocation
`resource_callback_slot_invoked` (C04.19.3). A plain member invocation
(no `slot`), a registration of either kind, a table, a registry and every
other retaining clause beside a premise still reject the module. The
rejection text is unchanged.

**Why this is sound, with no route change:**

* A slot invocation keeps nothing after the call, which was the ground for
  admitting the synchronous invocation in r29.
* Route G (`legacy_literal_premise::restrict_to_declared_literals`) has
  renamed every unmet call of a *retaining* declaration since r29. It reads
  `contracts.retaining()` directly, so it does not depend on which callback
  clause made the declaration retaining. An unmet premise, a missing actual
  or a poisoned premise set therefore renames the call before the callback
  route, the retaining environment or the member route sees it. So a slot
  is never decided at such a call, and the callee is undeclared.
* At a met call the callback route decides each slot exactly as rule 12n
  decides an opaque slot. It reads only the run's facts (no pre-pass:
  refused). It reads over the slot's declared registrations (a key no loaded
  declaration registers: refused). Each candidate must be the identity, and
  an empty closed set invokes nothing. The premise adds no candidate.
* The declaration-level readers ignore the premise and stay conservative.
  The member pre-pass (`callback_member_fold`) collects invoked keys. Rule
  15w names a position only through a parameter actual, and every actual
  here is `_`. Rule 13i classes the clause `Callback`.

The changes are in contract specification C04.5 ("Beside a slot
invocation"), the G05 prose in the implemented grammars, and
[rule 57](../../../../agents/rules/57.md), amended in place. No new rule was
needed.

## 3. The readers state the BIO slots

* **`pem.qv`.** Both PEM readers state all 14 BIO slot invocations on
  `out`. The password callback's `resource_callback_invoked_or_default`
  stays.
* **`pkcs12.qv`.** `d2i_PKCS12_bio` states the same 14 on `bp`.
* **Which slots.** The pages do not say which BIO operations a read
  performs, and a filter BIO forwards operations down its chain. So every
  key is stated, exactly the set the SSL I/O calls state.
* **Actuals.** Every actual is `_`, because the BIO handed to a slot
  function may be one chained to the caller's. `_` refuses any candidate
  that releases, consumes or keeps what it is handed
  (`callback_contracts::classify`).
* **The built-in premise.** Each reader names `openssl_builtin_bio_methods`.
  Its statement used to read "run on a BIO the SSL object holds". It now
  reads "run on a BIO the call reads or writes through (its BIO argument or
  a BIO the SSL object holds) or one chained to it". The five SSL I/O and
  shutdown declarations carry the same statement, so their digests change.
* **Checker** (`documented.rs`). `PEM_READ` gains `BIO_SLOTS` (the premise
  and the 14 invocations on position 0), and a new `PKCS12_READ` carries
  the `p12` premise and `BIO_SLOTS`. `a_drifted_bio_slot_reading_is_refused`
  now also refuses each of these on each reader:
  * any of the 14 invocations dropped;
  * the BIO given as the callback's actual;
  * the built-in premise dropped, or carrying its old SSL-only statement;
  * the literal-argument premise dropped.

  It also refuses the old statement on `SSL_write`.

**Other declarations that take a BIO.** No other declaration in the crate
reads or writes through a caller's BIO:

* `BIO_free` rests on `openssl_bio_free_callbacks` (r32);
* the 14 setters register and run nothing;
* `BIO_new_file` creates a fresh BIO with the built-in file method and no
  callback;
* the SSL calls state the slots already.

`BIO_read`, `BIO_write`, `BIO_puts`, `BIO_gets`, `BIO_ctrl`, other
`PEM_*_bio`, `d2i_*_bio` and `i2d_*_bio` are not declared.

**Lock.** The regeneration followed r32's order (logs `private/logs/regen-*.log`):

1. `quirin-plugin load-path`, from `providers/`, over libcurl then openssl.
   The first attempt named the wrong package and did not run. The second,
   `regen-1b-*.log`, exited 0.
2. `package.py checkers`. As expected in a fresh worktree, it installed the
   packages and then refused the stale lock (exit 1).
3. `plugin views --write`, exit 0. After the load-path rerun it was run
   again, `regen-3b-views.log`.
4. `pin --source-root …fixture --crate quirin-provider-openssl`, exit 0.
5. `contracts.py sync`, exit 0.
6. `package.py checkers`, exit 0.

**Lock diff, reviewed.** `contracts.lock` changes 8 `contract_sha256` and
`pinned_at` values: `PEM_read_bio_PrivateKey`, `PEM_read_bio_X509`,
`d2i_PKCS12_bio` (the slots), and `SSL_connect`, `SSL_do_handshake`,
`SSL_read`, `SSL_shutdown` and `SSL_write` (the premise statement). The
other 99 entries and every top-level key are unchanged. After the views
rerun, `providers/user-crates-load-path.json` and `quirin-plugin.json` are
byte-identical to base. No function was added, so there are no new
bindings.

## 4. Tests, each fail-closed arm with its own

In existing targets, so the read census needs no new unit:

* **`crates/qcore-contract/tests/callback_literal_premise.rs`**:
  * `a_premise_stands_beside_a_slot_invocation` covers both shapes: slot
    invocations beside the synchronous invocation (the PEM readers), and
    alone (`d2i_PKCS12_bio`). Each loads withheld, with its premise
    rederived.
  * `a_plain_member_invocation_or_a_slot_registration_beside_a_premise_still_rejects`
    covers a plain member invocation, a slot registration and a member
    registration. Each rejects with the rule 57 text, and the rederivation
    refuses too.
* **`crates/qcore-c-gateway/tests/openssl_undeclared_handshake_callbacks.rs`**
  (Clang). It loads the crate's `pem.qv`, `pkcs12.qv`, `x509.qv`, `evp.qv`
  and `bio_slots.qv`:

| Arm | Test | Reading |
|---|---|---|
| the readers' declarations | `the_readers_state_every_bio_slot` | 14 slot invocations on position 0, every actual `_`; the PEM readers keep the null default |
| within the premise, nothing or identity candidates registered (the shape of `read_cert` and `tls_load_builtin_credentials`) | `identity_bio_candidates_decide_the_readers` | `Safe`, no `callback-` refusal |
| an uncontracted method, an uncontracted callback, a destroy function releasing its BIO | `a_bio_candidate_that_is_not_the_identity_refuses_the_readers` | refused, naming the candidate |
| outside the premise (`&slot` at `x` or `p12`) with an uncontracted candidate registered | `a_reader_outside_its_premise_decides_no_bio_slot` | "outside its declared literal-argument premise", no `callback-` reading, the candidate never named |
| `bio_slots.qv` absent; no run facts | `a_reader_bio_slot_without_registrations_or_run_facts_refuses` | `no declaration registers`; not `Safe` |

**Lean.**

* **`QCore.CallbackLiteralPremise`** (amended). `Retaining` gains
  `slotInvoked`, and `admits` takes `besidePremise` (invoked or
  slotInvoked).
  * `a_premise_stands_only_beside_invocations` and
    `every_other_clause_beside_a_premise_rejects` are restated for the two
    kinds.
  * New: `a_plain_member_invocation_beside_a_premise_rejects`,
    `the_admission_only_grows` (against `admitsOld`) and
    `the_admission_grows_only_by_slot_invocations`.
* **`QCore.OpensslBioReaders`** (new, audited by
  `proof/AuditOpensslBioReaders.lean`, 21 theorems plus the 3 above).
  * an unmet premise or a poisoned set decides no slot;
  * a met premise defers to rule 12n's decision of each slot;
  * applied only over closed slot sets of identity candidates, with the own
    invocation applied;
  * no run facts, or no registration, refuses;
  * the premise adds no candidate;
  * stating more slots never improves the outcome;
  * the `reader_*` vectors, among them the reader shape: admitted now and
    refused by the old admission.

  Axioms are within `propext` and `Quot.sound`
  (`private/logs/lean-2.log`).

**The predecessor fixture's user path.** This is the `contracts` step over
`openssl-provider-remaining-callees/evidence/fixture`, with the tip binary
and load path (`private/logs/userpath-tip.log`, exit 0). `read_cert` and
`pkcs12_cycle` stay `Pending`, `read_cert_reuse` stays `Unknown`, and there
are 113 bindings, all `Pending`. The same step at base, run in the
base side tree with its binary (`private/logs/userpath-base.log`, exit 0),
gives the same status for every fixture function
([comparison](evidence/user-path/fixture-base-tip.txt)).

## 5. Measurement

**Pinned routes: no move predicted, arbiter deferred.** Corpus, system and
slice take no user load path. The loader change only admits modules it
rejected before, and no pinned module carries a slot invocation beside a
premise. The prediction's `measure-prediction` block (none on all three)
is for the integrator's `quirin-audit measure --declared`.

**The opt-in `current` run (this card's arbiter).** It ran `quirin-audit
current --monkey-linked` over Fluent Bit 046accf6 with
OPENSSL-SSL-CTX-CALLBACK-SLOTS'
[script](../openssl-ssl-ctx-callback-slots/evidence/opt-in/opt-in-current.sh)
(`SOURCE_DATE_EPOCH=1767225600`, `--jobs 12`). The driver is
`private/measure.sh` and its log `private/logs/measure.log`.

* **Side trees.** Each side is a `git archive` side tree with a release
  `quirin-audit` built in it, and each tree's own `package.py checkers`
  installed its checker (`checkers.exit` 0 on both). Base is main
  `54d067729`, which equals `dd0225012` plus r32's gate-record commit. Tip
  is `a3896c10c`, the implementation merged with main. The later commits
  are docs only.
* **Inputs** were prepared by the base binary's `prepare` into
  `private/fb-046accf6` (exit 0).

| Run | Tree | Files / configurations | Functions Safe / Pending / Unknown / Unsafe | Bindings |
|---|---|---|---|---|
| base (`private/opt-in/base-1`) | `54d067729` | 180 / 181: 9 Safe, 0 Pending, 5 Unsafe, 163 Unknown, 4 data-only | 1098 / 2 / 3699 / 5 | 113, all `Pending` |
| tip (`private/opt-in/tip-1`) | `a3896c10c` | same | 1098 / 2 / 3699 / 5 | 113, all `Pending` |

Both runs exit 0 with no unmatched row
([runs, invocations](evidence/opt-in/),
[comparison](evidence/opt-in/comparison-base-tip.json) by
OPENSSL-PROVIDER-REMAINING-CALLEES' `evidence/compare.py`).

**As predicted, no row moved and no row changed content.** No row went to
`Safe` and there is no new `Unsafe`. The rows applying a reader are all
`Unknown` on both sides with identical content:

* Monkey's `tls_load_builtin_credentials`;
* librdkafka's `rd_kafka_ssl_X509_from_string` and
  `rd_kafka_ssl_PKEY_from_string`, which refuse first on their password
  callback `rd_kafka_transport_ssl_passwd_cb`, as at base;
* `rd_kafka_ssl_read_cert_chain_from_BIO`, `rd_kafka_ssl_set_certs` and
  `rd_kafka_cert_new`.

So neither the stated slots nor the widened premise statement changed any
row's reading. The prediction named the first of these rows as a possible
`Pending` or `Safe` to `Unknown` move. It is `Unknown` at base already, so
the run cannot show whether its reader calls would now decide. The gateway
test `identity_bio_candidates_decide_the_readers` pins that reading for the
same shape. The report this run writes carries no `callback_members`
sidecar, so the BIO sets are not shown separately. Their effect is visible
only as the unchanged rows.

## 6. Gate

Command: `sh private/toolchain/env.sh python3 scripts/gate_scope.py --base
54d067729 --out private/gate-1` (Clang 21, Lean 4.35.0), the scoped card
gate with the provider gate included (14 steps). Record:
[`gate/gate-1.json`](gate/gate-1.json), logs `private/gate-1/` and
`private/logs/gate-1.log`. Tree `a3896c10c`; the report, evidence and
amendments were uncommitted (docs only). Overall exit 1, from one known
flake.

| Step | Exit | Counts |
|---|---:|---|
| submodules, `fmt`, `clippy` | 0 | 1 submodule (`third_party/libevent` at `d82464a27`) |
| Rust suite (scoped) | 0 | 662 units, 707 summaries; 9702 passed, 0 failed, 21 ignored, 148 filtered |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| Lean (`check_lean_audits.py`) | 0 | 452 audits (new `AuditOpensslBioReaders`) |
| Python suite | 1 | 779 tests; 1 failure, `test_gate_slot_queue.Status.test_each_waiter_shows_its_queue_position` (`'holder' != 'waiting for a slot'`), a known flake |
| provider package build, verify-all, verify `qcore.libevent` | 0, 0, 0 | |
| provider tests | 0 | 49 summaries; 158 passed, 0 failed, 1 ignored |
| provider fmt, provider clippy | 0, 0 | |

The flaky module was then rerun alone, `python3 -m unittest
scripts.tests.test_gate_slot_queue`: 39 tests OK, exit 0
(`private/logs/pyflake-rerun.log`). No new test target was added, so the
read census was not re-recorded.

After the report was written, the plain Python suite (`python3 -m unittest
discover -s scripts/tests`, no card base) was run again, exit 1
(`private/logs/python-2.log`): 779 tests, 1 failure. This time the queue
flake passed. The one failure is the stale-matrix check
`test_build_matrix_sections…test_the_derived_facts_are_recomputed_rather_than_carried`:
the committed matrix's `lean_audits` is 451, and this card produces 452. The
card gate passes the card base, which excuses that check. It catches up
when the integrator applies the amendments.

Outside the gate, during the work (`private/logs/`):

* the crate's checker tests: 15 passed (`ptest-2.log`);
* `qcore-contract`'s `callback_literal_premise`: 7 passed (`ctest-2.log`);
* the gateway's `openssl_undeclared_handshake_callbacks`: 17 passed
  (`gtest-3.log`);
* `lake build` of the two Lean modules and their audits, exit 0
  (`lean-2.log`).

## 7. Follow-ups

[`integration-amendments.json`](integration-amendments.json):

* retires `OPENSSL-BIO-READER-INVOCATIONS`;
* registers **CALLBACK-CANDIDATE-LITERAL-PREMISE**. This was found here and
  predates this card. A premise-carrying declaration met as a callback
  candidate is classified by its contract, and its premise is not checked
  against the invocation's actuals. That holds for function-map
  declarations through `classify`, and for withheld member-invocation
  declarations through `candidate_composition`, which `d2i_PKCS12_bio`
  now qualifies for. The three readers return a result, which `classify`
  reads as not the identity, so they are never applied as candidates.
  Premise-carrying declarations in other crates were not surveyed.

`scripts/apply_amendments.py … --check --profiles-base 54d067729` exits 0.

**Files likely to conflict** with other OpenSSL cards:

* `providers/quirin-provider-openssl/src/documented.rs` (`PEM_READ`,
  `PKCS12_READ`, `BUILTIN_BIO_METHODS_STATEMENT`) and `src/check.rs`;
* `contracts/openssl/pem.qv`, `pkcs12.qv`, `ssl_io.qv` and `ssl.qv` (the
  built-in premise's statement);
* `contracts.lock`;
* `contract_loader.rs`'s rule 57 branch;
* `docs/QCORE_CONTRACT_SPEC_r0_3.md` C04.5;
* `docs/agents/rules/57.md` and the generated `docs/agents/gateway.md`;
* `proof/QCore/CallbackLiteralPremise.lean` and the generated
  `proof/QCore.lean`;
* `crates/qcore-c-gateway/tests/openssl_undeclared_handshake_callbacks.rs`.
