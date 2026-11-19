# OPENSSL-FREE-SESSION-REMOVE-INVOCATION: slot invocations beside a reference drop

Base `062974679` (main, r33). Named follow-up
`OPENSSL-FREE-SESSION-REMOVE-INVOCATION` (recorded by
OPENSSL-UNSTATED-CALLBACK-SLOTS, appended by OPENSSL-BIO-METHODS-AND-CALLBACKS);
user decision 2026-11-19. [PREDICTION](PREDICTION.md) was committed before any
implementation (`de276404d`). The implementation, lock and Lean are
`76c2b4bd0`.

## 1. The drop ordering

**What base did.** At a call of a retaining declaration, the transfer
(`legacy_transfer.rs`) applies four things in this order:

1. the custody contract (`apply_contract_call`);
2. the retentions;
3. the reference keeps and drops (`reference_route::apply`);
4. the callback route (`callback_route::apply`).

A candidate puts requirements on the call through two channels, and they
sit on different sides of the drop:

* **Before the drop.** A `resource_owned(n)` requirement on a parameter
  actual is folded into the custody contract's mode at that position
  (`legacy_retained_by.rs`, rule 13u), so step 1 checks it.
* **After the drop.** Step 4 applies a candidate's member-path states at a
  parameter position (rule 30, `member_paths`) and the deferred reading of
  the object's constructed state (rules 13w and 14e, `member_uses`). Both
  read or change the object the candidate is handed.

R32's report says all of a candidate's requirements are applied after the
drop. That is true of the second channel only. Admitting a slot invocation
beside a drop with no other change would still let the second channel act
on a dropped object, and at a last drop that object is freed.

**The fix, in two parts.**

* **Order.** The callback route now runs before the reference route
  (`legacy_transfer.rs`). Before this card the loader rejected every
  callback, arm, result-state, hand-off and member-path clause beside a
  reference clause (`contract_loader.rs`, C04.19.2, .4, .5, .6). So a
  declaration with reference operands produced no callback operands
  (`callback_operands` returns `None`), and the swap changes no reading of
  any declaration admitted before.
* **A drop that may not be the last** (`legacy_retained_by.rs`). Only the
  last drop runs these callbacks ("When the reference count drops to zero",
  `SSL_CTX_new`/`SSL_new` 3.5; `BIO_free` frees on the last reference). The
  call does not decide whether its drop is the last: another holder may have
  up-ref'd the `SSL_CTX` or the BIO. So a stated slot is read as "may run".
  Its static decision (every candidate the identity over the run's closed
  set, rule 12n) is sound whatever the answer. A requirement that the caller
  hold the object is sound too, because it is read before the drop and the
  caller holds its reference whether or not the drop is the last. A
  candidate's effect on the dropped object is not sound as a must-effect
  when the slot may not run. So at a dropping call, callback operands
  carrying member-path states, deferred member uses, registry uses, arms,
  a result state or direct member paths refuse the call, with the new
  reason `callback-beside-drop` (`callback_contracts::BESIDE_DROP`). The
  loader already excludes the last four beside a reference clause; the
  guard names them so the refusal stays closed if that changes.

**What stands at a dropping call** is each slot's decision and the
pre-drop `resource_owned` check. Nothing is applied after the drop that
reads the dropped object. With the guard in place, step 4 beside a drop has
nothing state-dependent left to apply. The order fix makes that hold by
construction, not only because the guard removed everything.

**Measured precision gap, fail-closed.** A candidate requiring the dropped
object live (`resource_owned`) turns the call's own mode at the dropped
position from borrowed to owned. On a reference family the drop then reads
as undecided and the call refuses (`reference-drop-unheld`), never `Safe`.
No OpenSSL declaration hands a slot the dropped formal: every actual is
`_`. Registered as CALLBACK-REQUIREMENT-BESIDE-DROP.

## 2. C04.19 amended in place

* **Loader** (`contract_loader.rs`, C04.19). Beside reference clauses, a
  callback clause stands only when every reference clause is a drop and
  every callback clause is `CallbackClause::MemberInvoked { slot: true }`.
  Still rejected with the unchanged C04.19 text, beside a reference clause:
  a keep, a plain member invocation, a synchronous invocation, a table, a
  registration and a registry. A literal-argument premise beside a reference
  clause still rejects, by rule 57's unchanged `callbacks_only` check.
* **Readers that might misread a declaration with both kinds.**
  * `candidate_composition::composable` requires no references, so a free
    met as a callback candidate stays uncontracted, as before.
  * Rule 13c's body validation (`settle_clauses`) reads only the reference
    clauses.
  * The callback body check (`callback_route::settle`) reads the callback
    clauses of a declaration whose body the unit defines. The OpenSSL frees
    have no body in user units.
  * The member pre-pass collects the invoked keys of every declaration a row
    lists, as for any slot invocation.
* **No new rule.** The text lives in contract specification C04.19 ("Beside a
  reference drop"), the G05 placement sentence in the implemented grammars,
  and [rule 12n](../../agents/rules/12n.md), amended in place. Rule id 61
  was not needed.

## 3. The declarations

Every claim cites a 3.5 page fetched during the work (2026-11-19, pages kept
privately, not committed, as the crate's snapshot rule says):

* **`SSL_CTX_free`.** "If a session-remove callback is set
  (SSL_CTX_sess_set_remove_cb()), this callback will be called for each
  session being freed from ctx's session cache" (SSL_CTX_free).
* **`SSL_free`.** "If the session is considered bad, because SSL_shutdown(3)
  was not called for the connection ..., the session will also be removed
  from the session cache" (SSL_free NOTES). It also frees "the read and
  write BIOs" (same).
* **`BIO_free`** (r32's citations). The callback "is called before the free
  operation" with `BIO_CB_FREE` (BIO_set_callback). The destroy function
  "will be called in response to the application calling BIO_free()"
  (BIO_meth_new).

What each free now states, every actual `_`:

| Free | Stated slots | Premises |
|---|---|---|
| `SSL_CTX_free` | `ssl_ctx_st.remove_session_cb` (on `ctx`) | `openssl_unstated_callback_slots` |
| `SSL_free` | `ssl_ctx_st.remove_session_cb`, `bio_st.bio_callback_ex`, `bio_st.bio_callback`, `bio_method_st.meth_destroy` (on `ssl`) | `openssl_unstated_callback_slots`, `openssl_bio_free_callbacks` (narrowed), `openssl_builtin_bio_methods` |
| `BIO_free` | `bio_st.bio_callback_ex`, `bio_st.bio_callback`, `bio_method_st.meth_destroy` (on `a`) | `openssl_bio_free_callbacks` (narrowed), `openssl_builtin_bio_methods` |

* **Actuals.** `_` everywhere.
  * The session-remove callback is handed the session's context, which is
    a formal only at `SSL_CTX_free`. `_` is consistent with the I/O calls'
    statement of the same slot, and it refuses any candidate that releases,
    consumes or keeps what it is handed.
  * A BIO's slots at `SSL_free` are handed a BIO the connection holds, not
    the connection.
  * At `BIO_free` the BIO handed is `a` itself. `_` refuses a candidate
    that would free it a second time. Stating `a` would let a
    custody-neutral candidate's requirements fall on the dropped formal,
    which the first finding above makes refuse anyway.
* **`openssl_bio_free_callbacks` narrowed** to "the ex_data free callbacks of
  the BIO class". The callback and destroy slots are stated.
* **`openssl_builtin_bio_methods` widened.** It now reads "run on a BIO the
  call reads or writes through or frees". A built-in method's destroy
  function (a socket BIO with `BIO_CLOSE` closes its descriptor) is no
  setter's value, so the destroy slot alone does not cover it. The eight
  declarations already carrying the premise change digest.
* **`openssl_unstated_callback_slots` stays on both frees.** This departs
  from the registered discharge ("leaving them on `openssl_ex_data_callbacks`
  and the BIO methods SSL_free frees").
  * Freeing a context frees its certificate store: "SSL_CTX_set_cert_store()
    will take ownership of the store" (SSL_CTX_set_cert_store).
  * A store's lookup methods carry user-settable functions:
    `X509_LOOKUP_meth_set_free` and `X509_LOOKUP_meth_set_shutdown`
    (X509_LOOKUP_meth_new 3.5 names both).
  * `SSL_free` may drop the context's last reference.
  * So `openssl_ex_data_callbacks` alone would leave callbacks the frees may
    run uncovered. The unstated premise's statement is shared and unchanged.
    The stated session-remove slot leaves its coverage by its own words ("no
    clause of this declaration states").
  * Registered as OPENSSL-FREE-INDIRECT-CALLBACKS. The source was not read
    for further indirectly freed objects.

**Checker** (`documented.rs`, `check.rs`).

* `BIO_FREED`, `SSL_FREED` and a new `SSL_CTX_FREED` carry the slots.
  `BIO_FREE_STATEMENT` is narrowed and `BUILTIN_BIO_METHODS_STATEMENT`
  widened. The `UNSTATED` constant is removed: `SSL_CTX_free` was its only
  user.
* `a_drifted_bio_slot_reading_is_refused` now refuses each of these on
  `BIO_free` and `SSL_free`:
  * each of the three slots dropped;
  * the freed object handed to the callback;
  * the built-in premise dropped;
  * the free premise dropped, or read in its old statement;
  * the old reading (premise, no invocation);
  * a BIO slot other than the three stated beside the drop.
* On `SSL_CTX_free` and `SSL_free`, it refuses the session-remove invocation
  dropped or handed the freed object, and the unstated premise dropped.

**Lock.** Regenerated in the order r32 and r33 used. Driver
`private/regen.sh`, logs `private/logs/regen-*.log`:

| Step | Exit |
|---|---|
| `quirin-plugin load-path` (from `providers/`, libcurl then openssl) | 0 |
| `package.py checkers` (installs, then refuses the stale views) | 1, as expected |
| `plugin views --write` | 0 |
| `pin --source-root …fixture --crate quirin-provider-openssl` | 0 |
| `contracts.py sync` | 0 |
| `package.py checkers` | 0 |

**Lock diff, reviewed.** `contracts.lock` changes 11 `contract_sha256` and
`pinned_at` values:

* `BIO_free`, `SSL_free`, `SSL_CTX_free` (the invocations and premises);
* `SSL_connect`, `SSL_do_handshake`, `SSL_read`, `SSL_shutdown`, `SSL_write`,
  `PEM_read_bio_X509`, `PEM_read_bio_PrivateKey`, `d2i_PKCS12_bio` (the
  built-in premise's widened statement).

The other 96 entries and every top-level key are unchanged.
`providers/user-crates-load-path.json` and `quirin-plugin.json` are
byte-identical to base (no function added, no binding change).

## 4. Tests, each fail-closed arm with its own

In the existing target
`crates/qcore-c-gateway/tests/openssl_undeclared_handshake_callbacks.rs`, so
the read census needs no new unit. r32's
`bio_free_states_no_invocation_beside_its_drop` is replaced.

| Arm | Test | Reading |
|---|---|---|
| the frees' declarations | `the_frees_state_their_slots_beside_the_drop` | one nullable drop at position 0; exactly the slots above, every actual `_` |
| the loader | `a_slot_invocation_stands_beside_drops_only` | a slot invocation beside a drop loads (withheld). Each of these rejects with the C04.19 text: the same beside a keep, a plain member invocation, a synchronous invocation, a registration, a slot invocation plus a registration |
| identity candidates, or nothing registered | `identity_candidates_decide_the_frees` | `free_ctx`, `free_ssl`, `free_bio` `Safe`, no `callback-` refusal |
| an uncontracted remove callback; a destroy releasing its BIO; an uncontracted BIO callback | `a_candidate_that_is_not_the_identity_refuses_the_frees` | refused, naming the candidate, on each free that runs the slot; the other free stays `Safe` |
| no run facts; `bio_slots.qv` or `ssl_ctx.qv` absent | `a_free_slot_without_registrations_or_run_facts_refuses` | not `Safe`; `no declaration registers` |
| the order and the drop that may not be the last (synthetic `obj_put` dropping and invoking `obj_st.on_free(o)`, once and after a keep) | `a_candidate_effect_on_the_dropped_object_refuses` | identity `Safe` both ways; a member-path candidate refuses `callback-beside-drop`; a releasing candidate refuses; a `resource_owned` candidate refuses `reference-drop-unheld` (§1) |

**Lean.**

* **`QCore.OpensslFreeInvocations`** (new, audited by
  `proof/AuditOpensslFreeInvocations.lean`, 21 theorems):
  * the admission only grows, and only by slot invocations beside drops;
  * a keep, or another callback clause beside a reference clause, rejects;
  * the new order reads the object held and unfreed, while the old order
    read it unheld, and freed at a last drop;
  * the reading does not depend on whether the drop is the last;
  * a member effect beside a drop refuses, and so does a requirement on an
    unheld object;
  * applied implies closed identity slot sets, no member effect and the
    object held;
  * stating more slots never improves the outcome;
  * the frees' vectors.
* **`QCore.OpensslBioSlots`** (amended).
  * `Runnable` loses `freeCallback`: a BIO's free callbacks are slots.
  * The free premise covers only ex_data
    (`the_free_premise_covers_only_ex_data`).
  * `a_bio_drop_states_its_slots` replaces the retired
    `a_bio_drop_needs_the_free_premise`, `an_admitted_drop_states_no_invocation`
    and `a_bio_drop_rests_on_the_premise`.
* Axioms are within `propext` and `Quot.sound` (`private/logs/lean-2.log`).

**The predecessor fixture's user path.** This is the `contracts` step over
`openssl-provider-remaining-callees/evidence/fixture`, run twice:

* at the tip, with the worktree binary and load path
  (`private/logs/userpath-tip.log`, exit 0);
* at base, in the base side tree with its binary
  (`private/logs/userpath-base.log`, exit 0).

Both give 113 bindings, all `Pending`, and every fixture function keeps its
status ([comparison](evidence/user-path/fixture-base-tip.txt)). The rows
differ only in the build, profile, assumption and contract-set digests.

## 5. Measurement

**Pinned routes: no move predicted, not measured here.** This card changes
transfer code, so the pinned-route check is `quirin-audit measure
--declared` against the prediction's `measure-prediction` block (none on
corpus, system and slice). It could not run on this host: the pinned inputs
(`build/cycle19`, the inputs root of `inputs/live.json`) and the published
baselines (`private/baselines`) are absent, as r30 to r33 found. The
argument for none is structural (§1):

* the swap and the refusal read only declarations carrying both reference
  and callback operands, which the loader rejected before this card;
* no pinned module carries a slot invocation beside a drop.

The integrator's `measure --declared` is the arbiter.

**The opt-in `current` run (this card's arbiter).** It ran `quirin-audit
current --monkey-linked` over Fluent Bit 046accf6 with
OPENSSL-SSL-CTX-CALLBACK-SLOTS'
[script](../openssl-ssl-ctx-callback-slots/evidence/opt-in/opt-in-current.sh)
(`SOURCE_DATE_EPOCH=1767225600`, `--jobs 12`).

* **Side trees.** Each side is a `git archive` side tree with its own release
  `quirin-audit` (`private/build-base.sh`, `private/build-tip.sh`), and each
  tree's own `package.py checkers` installed its checker (`checkers.exit` 0
  on both). Base is main `062974679`; tip is `76c2b4bd0`, the
  implementation (later commits are docs only).
* **Inputs** were prepared on this host by the base binary's `prepare` into
  `private/fb-046accf6` (exit 0, `private/logs/prepare.log`).
* **Host load.** Two other cards' opt-in runs were sharing the host, and the
  tip run overlapped the base run's end. Wall times are not comparable; the
  rows are deterministic.

| Run | Tree | Files / configurations | Functions Safe / Pending / Unknown / Unsafe | Bindings |
|---|---|---|---|---|
| base (`private/opt-in/base-1`) | `062974679` | 180 / 181: 9 Safe, 0 Pending, 5 Unsafe, 163 Unknown, 4 data-only | 1098 / 2 / 3699 / 5 | 113, all `Pending` |
| tip (`private/opt-in/tip-1`) | `76c2b4bd0` | same | 1098 / 2 / 3699 / 5 | 113, all `Pending` |

Both runs exit 0 with no unmatched row
([runs, invocations](evidence/opt-in/),
[comparison](evidence/opt-in/comparison-base-tip.json) by
OPENSSL-PROVIDER-REMAINING-CALLEES' `evidence/compare.py`).

**As predicted, no row moved and no row changed content.** No row went to
`Safe`, there is no `Safe` to non-`Safe` move and no new `Unsafe`. The rows
that name a free are `Unknown` on both sides with identical content:

* librdkafka's `rd_kafka_cert_new` and four more naming `BIO_free`;
* `rd_kafka_ssl_ctx_init` and two more naming `SSL_CTX_free`;
* `rd_kafka_transport_ssl_close` and two more naming `SSL_free`;
* Monkey's `context_unset` and `contexts_free` (`SSL_free`), `mk_tls_exit`
  (`SSL_CTX_free`), `tls_configure_dh` and `tls_load_builtin_credentials`
  (`BIO_free`), in `lib/monkey/tls/tls_openssl.c`. Fluent Bit's own
  `src/tls/openssl.c` is not in the run's compile database.

So the stated slots were not what decided any of these rows. Each refuses
first on something else, as at base. The prediction named a free's row
refusing on its new slots as the likeliest way to be wrong. No such row is
`Safe` or `Pending` at base, so the run cannot show whether a free's slots
now decide; the gateway test `identity_candidates_decide_the_frees` pins that
reading for the same shape. As r33 found, the report carries no
`callback_members` sidecar, so the run's sets for the four keys are not
shown separately.

## 6. Gate

**Command:** `sh private/toolchain/env.sh python3 scripts/gate_scope.py --base
062974679 --out private/gate-1` (Clang 21, Lean 4.35.0; driver
`private/gate.sh`).

* **Mode.** The scope is full, because the diff reaches `qcore-c-gateway`
  and `qcore-contract`. The provider gate is included (14 steps).
* **Record.** [`gate/gate-1.json`](gate/gate-1.json), logs `private/gate-1/`
  and `private/logs/gate-1.log`.
* **Tree.** `76c2b4bd0`. The report, evidence and amendments were
  uncommitted (docs only).
* **Result.** `gate_scope: 14 steps passed`, exit 0.

| Step | Exit | Counts |
|---|---:|---|
| submodules, `fmt`, `clippy` | 0 | 1 submodule (`third_party/libevent` at `d82464a27`) |
| Rust suite | 0 | 662 units, 707 summaries; 9707 passed, 0 failed, 21 ignored, 148 filtered |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| Lean (`check_lean_audits.py`) | 0 | 453 audits (new `AuditOpensslFreeInvocations`) |
| Python suite | 0 | 779 tests |
| provider package build, verify-all, verify `qcore.libevent` | 0, 0, 0 | |
| provider tests | 0 | 49 summaries; 158 passed, 0 failed, 1 ignored |
| provider fmt, provider clippy | 0, 0 | |

* **Timing.** The provider tests took 5828 s, most of it waiting for the
  `rust-suite` slot behind other cards' provider runs (receipt
  `private/gate-1/slots/provider-test.json`).
* **No known flake fired.** No new test target was added, so the read census
  was not re-recorded.
* **The committed matrix lags this card.** Its `lean_audits` is 452 and this
  card produces 453. The gate passes the card base, which excuses that
  check, and the integrator's application of the amendments brings the
  matrix up to date.

**Outside the gate**, during the work (`private/logs/`):

* the crate's checker tests: 15 passed (`ptest-1.log`);
* `openssl_undeclared_handshake_callbacks`: 22 passed (`gtest-2.log`);
* six related gateway targets, all passing (`gtest-3.log`):
  * `openssl_callback_slots`;
  * `openssl_unstated_callback_slots`;
  * `refcount_reference_returning`;
  * `refcount_resource_kind`;
  * `reference_drop_null_accepting`;
  * `member_path_release`;
* `lake build` of the two touched modules and both audits, exit 0
  (`lean-2.log`).

## 7. Follow-ups

[`integration-amendments.json`](integration-amendments.json):

* retires `OPENSSL-FREE-SESSION-REMOVE-INVOCATION`;
* registers **OPENSSL-FREE-INDIRECT-CALLBACKS**. The frees keep the unstated
  premise for callbacks of objects they free in turn: the certificate
  store's lookup methods, and anything a source survey finds. Stating those
  would let the premise become `openssl_ex_data_callbacks`, as the
  registered discharge expected;
* registers **CALLBACK-REQUIREMENT-BESIDE-DROP**. A slot candidate's
  `resource_owned` requirement on the dropped formal refuses the drop
  (`reference-drop-unheld`). This is fail-closed but imprecise (§1);
* appends to **OPENSSL-EX-DATA-CLASS-REGISTRY** (r34's follow-up; the first
  version of these amendments appended to OPENSSL-EX-DATA-REGISTRY, which
  r34 retired): a slot invocation now stands beside a drop, a registry
  invocation still does not, and the BIO free premise names only ex_data.

`scripts/apply_amendments.py … --check --profiles-base 41eb7e959` exits 0.

## 8. Merged with main `41eb7e959` (r34, OPENSSL-EX-DATA-REGISTRY)

Coordinator's request, 2026-11-20. The conflicts were resolved so both
cards' readings hold:

* **Declarations.** The `.qv` files were taken from main, and this card's
  edits reapplied on them (`private/reapply_qv.py`): the stated slots, the
  narrowed free premise and the widened built-in premise. `BIO_free`,
  `SSL_free` and `SSL_CTX_free` keep r34's `openssl_ex_data_callbacks`.
* **Checker.** `BIO_FREED`, `SSL_FREED` and `SSL_CTX_FREED` carry
  `Premise::ExDataCallbacks` beside this card's slots. r34's `UNSTATED`
  constant is removed again, because `SSL_CTX_free` was its only user.
  `EX_DATA_AUDIT` and the crate's 17 checker tests pass.
* **Spec C04.18.** Both paragraphs are kept, r34's first.
* **Generated files.** `proof/QCore.lean` and `gateway.md` were regenerated.
* **Lock.** Regenerated in the documented order (logs
  `private/logs/m/regen-*.log`: load-path 0, checkers 1 as expected, views
  0, pin 0, sync 0, checkers 0). Against `41eb7e959`, 11 `flexible`
  entries change digest: the same 11 functions as before the merge. No
  other entry or field changes, and the top-level `pinned` and
  `library_versions` are unchanged. The user load path and
  `quirin-plugin.json` equal main's.

The opt-in measurement above was taken before the merge, against r33. The
merge adds r34's premise lines, which move no verdict, so it was not
repeated. The gate below was rerun on the merge.

**Files likely to conflict:**

* with OPENSSL-EX-DATA-REGISTRY:
  * `providers/quirin-provider-openssl/src/documented.rs` (`BIO_FREED`,
    `SSL_FREED`, `SSL_CTX_FREED`, both premise statements, the removed
    `UNSTATED`) and `src/check.rs`;
  * `contracts/openssl/ssl.qv`, `bio.qv`, and the built-in premise's
    statement in `ssl_io.qv`, `pem.qv` and `pkcs12.qv`;
  * `contracts.lock`, which should be regenerated, not merged by hand;
* with CALLBACK-SUPPLIED-NULL-ACTUAL and CALLBACK-CANDIDATE-LITERAL-PREMISE:
  * `crates/qcore-contract/src/contract_loader.rs` (C04.19 branch);
  * `crates/qcore-c-gateway/src/legacy_retained_by.rs`,
    `legacy_transfer.rs` and `callback_contracts.rs`;
* documentation and proofs:
  * `docs/QCORE_CONTRACT_SPEC_r0_3.md` C04.19 and C04.18;
  * `docs/agents/rules/12n.md`, and the generated `docs/agents/gateway.md`
    (rerun `scripts/gen_rule_docs.py`);
  * `proof/QCore/OpensslBioSlots.lean`, and the generated
    `proof/QCore.lean` (rerun `scripts/gen_lean_roots.py`);
* `crates/qcore-c-gateway/tests/openssl_undeclared_handshake_callbacks.rs`.
