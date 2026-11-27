# OPENSSL-BIO-METHODS-AND-CALLBACKS: a BIO's callback and method table as opaque slots

Base `f659b24f5` (main, r31). Named follow-up
`OPENSSL-BIO-METHODS-AND-CALLBACKS` (recorded by
OPENSSL-UNDECLARED-HANDSHAKE-CALLBACKS); user decision 2026-11-19.
[PREDICTION](PREDICTION.md) was committed before any implementation
(`00732852e`). The implementation is `103c80c9d`, and the slot keys were
renamed after their setters in `b57e81305` (§5).

## 1. `BIO_free` first

`BIO_free` read silently that it runs callbacks. When the last reference
drops, it runs three things:

* the BIO's callback. `BIO_set_callback` (3.5), CALLBACK OPERATIONS:
  "callback_ex(b, BIO_CB_FREE, NULL, 0, 0, 0L, 1L, NULL) or callback(b,
  BIO_CB_FREE, NULL, 0L, 0L, 1L) is called before the free operation";
* its method's destroy function. `BIO_meth_new` (3.5): "This function will
  be called in response to the application calling BIO_free()";
* the BIO class's ex_data free callbacks (`CRYPTO_get_ex_new_index`, 3.5).

**Stating the invocation was checked and not done.** C04.19 rejects a
callback clause beside a reference drop (`contract_loader.rs`: "a callback
clause stands only beside retentions and plain predicates"). Lifting that
check alone would be unsound:

* a slot invocation's candidates put their requirements on the call's
  actuals (`operands.owned`, `legacy_callback_route.rs`);
* those requirements are applied by `callback_route::apply`, which
  `legacy_transfer.rs` runs after `reference_route::apply`;
* at `BIO_free` the callback is handed the BIO being freed (`b`), so a
  candidate's requirement on it would be checked after the drop. Rebuilding
  that order, and modelling it in Lean, is OPENSSL-FREE-SESSION-REMOVE-INVOCATION's
  work.

Even with that clause, the destroy function and the ex_data callbacks would
still need a premise (ex_data is no slot at all).

So `BIO_free` names a new premise, `openssl_bio_free_callbacks` (C04.18):
"when the call drops the last reference to a BIO (its BIO argument, or a BIO
the SSL object holds and the BIOs chained to it), the callback the BIO holds,
run with BIO_CB_FREE, the destroy function of its method and the ex_data free
callbacks of the BIO class take, release or retain nothing the caller holds".
`SSL_free` frees "the read and write BIOs" (SSL_free 3.5), so it names this
premise beside `openssl_unstated_callback_slots`. A test pins that the loader
still rejects the stated invocation
(`bio_free_states_no_invocation_beside_its_drop`).
OPENSSL-FREE-SESSION-REMOVE-INVOCATION stays open, and its discharge now
also covers stating these invocations (§6).

## 2. The slots

Every claim cites a fetched page. Two pages were added to `doc-snapshots.json`
with `fetch_docs.py --merge` (2026-11-19): `BIO_set_callback` at 3.5, 1.1.1
and 1.0.2, and `BIO_meth_new` at 3.5 and 1.1.1 (1.0.2 is recorded as not
found). The 14 setters' prototypes were pinned with `pin_prototypes.py` over
every recorded function plus the new ones. Every existing entry and every
header digest is byte-identical. The new module is `bio_slots.qv`.

| Slot | Writer (the 3.5 page's) | Range (cited series) |
|---|---|---|
| `bio_st.bio_callback_ex` | `BIO_set_callback_ex` | `documented "1.1.1"` (1.1.1) |
| `bio_st.bio_callback` | `BIO_set_callback` | `documented "1.1.1"` (1.1.1; at 1.0.2 the page documents a macro storing into the then public `struct bio_st`) |
| `bio_method_st.meth_write_ex`, `.meth_write`, `.meth_read_ex`, `.meth_read`, `.meth_puts`, `.meth_gets`, `.meth_ctrl`, `.meth_create`, `.meth_destroy`, `.meth_callback_ctrl` | `BIO_meth_set_write_ex`, `_write`, `_read_ex`, `_read`, `_puts`, `_gets`, `_ctrl`, `_create`, `_destroy`, `_callback_ctrl` | `since "1.1.0"` (1.1.1 HISTORY: "added in OpenSSL 1.1.0") |
| `bio_method_st.meth_sendmmsg`, `.meth_recvmmsg` | `BIO_meth_set_sendmmsg`, `_recvmmsg` | `since "3.2.0"` (3.5 HISTORY: "added in OpenSSL 3.2") |

**How the method table is read.** It is an object of its own, read as a
holder-to-holder slot:

* The holder of a method slot is the `BIO_METHOD` the setter is handed. That
  table reaches a BIO through `BIO_new` and an SSL object through
  `SSL_set_bio`, so the object the I/O calls name is the SSL (`ssl`).
* The key is run-wide, so its set is every function any table's setter is
  handed. That is a superset of what the SSL's BIOs' tables can hold, with
  one exception: the methods the library defines itself (`BIO_s_socket`,
  `BIO_s_mem`, `BIO_f_buffer`, ...), which no setter is handed.
* Those built-ins are the library-value premise `openssl_builtin_bio_methods`
  on every declaration that invokes the slots: "a function of a BIO method the
  library defines itself (...), run on a BIO the SSL object holds or one
  chained to it, takes, releases or retains nothing the caller holds, and so
  does any callback it runs in turn on an object other than the arguments of
  the call (the SSL object of a BIO_f_ssl BIO)".
* `BIO_METHOD` has been opaque since 1.1.0, the setters' range, so no unit
  writes a table except through the setters. A unit that defines
  `struct bio_method_st` (1.0.2's public layout) refuses the slot under rule
  12n's existing check.
* A table whose setter is handed something other than a function name (for
  example `BIO_meth_get_read(BIO_s_socket())`, which the BUGS section warns
  against) opens the set.
* The run never defines the record, so a key is only the slot's name. Each
  key is named after its setter (`meth_write_ex` for
  `BIO_meth_set_write_ex`, `bio_callback_ex` for `BIO_set_callback_ex`),
  not after the library's internal fields (`bwrite`, `ctrl`, `create`,
  `callback`). The first tip run used those field names and found every BIO
  key open (§5).

**The invocations.** `SSL_write`, `SSL_read`, `SSL_connect`,
`SSL_do_handshake` and `SSL_shutdown` invoke all 14 slots on `ssl`, each
with every actual `_`:

* each slot is handed the BIO, never the connection (callback `b`, method
  `BIO *`);
* the SSL pages do not say which BIO operations the record layer performs,
  so every key is stated, `create` and `destroy` included (stating more
  never improves a verdict);
* the counts are now 41 invocations on each I/O call (27 + 14) and 17 on
  `SSL_shutdown` (3 + 14).

**The checker** (`documented.rs`, `check.rs`):

* 14 new `SLOTS`, and `Premise::BuiltinBioMethods` and
  `Premise::BioFreeCallbacks` with their statements;
* `BIO_callback_fn_ex` and `BIO_callback_fn` among the function pointer
  typedefs;
* the NAME exactness check holds for both pages. The getters,
  `BIO_get_new_index`, `BIO_meth_new`, `BIO_meth_free`, the callback
  argument macros, the debugging callbacks and the callback types are
  not-writers.

`a_drifted_bio_slot_reading_is_refused` refuses each of these:

* each of the 14 invocations dropped from each of the five calls;
* the connection given as the callback's BIO;
* the built-in premise dropped;
* the old statement of the unstated-slots premise;
* the free premise dropped from `BIO_free` or `SSL_free`, or replaced by a
  stated invocation;
* a setter's registration moved or dropped;
* a page naming one more method setter (refuses all twelve method slots);
* a key the checker does not document.

## 3. What stays under a premise

* **`openssl_unstated_callback_slots`** drops "a method or callback of a BIO
  the SSL object holds" from its statement. It stays on the same seven
  declarations for:
  * the ctrl arms (OPENSSL-CTRL-COMMAND-ARMS);
  * the CT slot (OPENSSL-CT-VALIDATION-LIBRARY-WRITER);
  * the setters still undeclared (OPENSSL-REMAINING-HANDSHAKE-SETTERS);
  * ex_data (OPENSSL-EX-DATA-REGISTRY).

  `SSL_CTX_free` frees no BIO, and its digest changes only by the narrowed
  statement.
* **`openssl_builtin_bio_methods`** is on the five I/O and shutdown calls.
* **`openssl_bio_free_callbacks`** is on `BIO_free` and `SSL_free`.

**A finding, out of scope.** `PEM_read_bio_X509`, `PEM_read_bio_PrivateKey`
and `d2i_PKCS12_bio` read through the caller's BIO, so they run its callback
and its method's read, gets and ctrl functions, and their declarations do not
say so. A slot invocation cannot stand beside their literal-argument premise
(rule 57: "a literal-argument premise stands beside a synchronous invocation
only"). This is registered as OPENSSL-BIO-READER-INVOCATIONS.

**Nothing becomes `Safe` by this card alone.** Each stated invocation adds
an obligation, and a premise moves no verdict (C04.18,
`QCore.OpensslUnstatedSlots.the_verdict_ignores_the_premise`). A run that
registers an uncontracted or effectful BIO candidate now refuses the I/O
calls. A BIO key nothing registers refuses even when its set is empty.

**Lock.** `quirin-audit pin` (release binary built at the base, which this
card does not change outside tests; toolchain env;
`--source-root docs/reports/openssl-provider-remaining-callees/evidence/fixture
--crate quirin-provider-openssl`; `private/logs/regen-3.log`):

* 14 entries added (`openssl/bio_slots.qv`);
* 8 digests changed: `BIO_free` and `SSL_free` (the free premise),
  `SSL_CTX_free` (the narrowed statement), and `SSL_connect`,
  `SSL_do_handshake`, `SSL_read`, `SSL_shutdown` and `SSL_write`;
* 85 entries unchanged, and `pinned`, `library_versions` and every other
  top-level key unchanged.

The other steps (`private/logs/`):

* `quirin-plugin load-path --extends contract-load-path.json`, from
  `providers/`, over libcurl then openssl (`regen-1.log`): exit 0, 14
  bindings added. `quirin-plugin.json` gained the 14 bindings by hand, as
  r31's did.
* `plugin views --write`. Given without `--load-path` (`regen-1.log`), it
  printed its usage. With `--load-path` (`regen-2.log`), it refused
  (`QI014`) because the checker packages were not installed in this fresh
  worktree. `package.py checkers` (`checkers-pre.log`) installed them and
  then refused the stale views. The views step then succeeded
  (`regen-3.log`). So in a fresh worktree the order is: `load-path`,
  `package.py checkers` (which installs the packages, then fails), `plugin
  views --write`, `pin`, `contracts.py sync`, `package.py checkers`.
* `contracts.py sync`: wrote nothing further.
* `package.py checkers`: exit 0.

## 4. Fail-closed arms, each with a test

The tests are in the existing target
`crates/qcore-c-gateway/tests/openssl_undeclared_handshake_callbacks.rs`, so
the read census needs no new unit. They read the crate's own `ssl.qv`,
`ssl_ctx.qv`, `ssl_io.qv`, `ssl_handshake.qv` and `bio_slots.qv`; with Clang,
6 new tests:

| Arm | Test | Refusal |
|---|---|---|
| libevent's `bufferevent_openssl.c` shape (`BIO_meth_set_write(m, f)`, `f` uncontracted), and an uncontracted `BIO_set_callback_ex` candidate | `an_uncontracted_bio_candidate_refuses` (I/O call and `SSL_shutdown`) | `callback-uncontracted`, naming the candidate |
| a destroy function releasing the BIO it is handed | `an_effectful_bio_method_refuses` | `callback-` refusal naming it |
| a BIO key no loaded declaration registers (`bio_slots.qv` absent) | `a_bio_slot_no_declaration_registers_refuses` | `no declaration registers` (all 14 keys, both rows) |
| an invocation stated beside `BIO_free`'s drop | `bio_free_states_no_invocation_beside_its_drop` | the loader's C04.19 rejection |
| the 14 setters load as their registrations; the five calls invoke every key with `_` actuals | `the_crate_states_every_bio_slot` | n/a |

A row is `Safe` only where every candidate is the identity
(`identity_bio_candidates_decide_the_io_call`). r30's
`openssl_unstated_callback_slots.rs` and r31's `the_crate_states_every_handshake_slot`
now load `bio_slots.qv` too and expect 41 invocations on each I/O call and 17
on `SSL_shutdown`.

**Lean.** `QCore.OpensslBioSlots`
(`proof/AuditOpensslBioSlots.lean`, 11 theorems):

* every function a setter wrote is a candidate of the method slot, and a
  held function outside the set is a built-in;
* no premise covers a BIO slot, so each must be stated;
* a built-in method is covered only by `openssl_builtin_bio_methods`;
* a BIO's free callbacks are covered only by `openssl_bio_free_callbacks`;
* an admitted declaration that drops a reference states no invocation, so a
  BIO drop rests on that premise;
* stating the BIO slots never improves a verdict.

Axioms are within `propext` and `Quot.sound`.

## 5. Measurement

**Pinned routes: no move predicted, arbiter deferred.** Corpus, system and
slice take no user load path, and no gateway code changed. The prediction's
`measure-prediction` block (none on all three) is for the integrator's
`quirin-audit measure --declared`.

**The opt-in `current` run (this card's arbiter).** It ran `quirin-audit
current --monkey-linked` over Fluent Bit 046accf6 with
OPENSSL-SSL-CTX-CALLBACK-SLOTS'
[script](../openssl-ssl-ctx-callback-slots/evidence/opt-in/opt-in-current.sh)
(`SOURCE_DATE_EPOCH=1767225600`, `--jobs 12`).

* **Inputs** were prepared again by the base binary's `prepare` into
  `private/fb-046accf6`, exit 0.
* **Side trees.** Each run used a `git archive` side tree whose own
  `package.py checkers` installed its checker (exit 0 every time).
* **Binary.** Every run used one release `quirin-audit` built at the base.
  The card changes no crate source outside tests, so the tip's binary is the
  same program.
* **A false start.** A first base attempt was stopped while its checkers
  step ran, because the script was given relative paths it resolves after
  `cd`. It produced nothing and was not used.

| Run | Tree | Files / configurations | Functions Safe / Pending / Unknown / Unsafe | Bindings |
|---|---|---|---|---|
| base (`private/opt-in/base-2`) | `f659b24f5` | 180 / 181: 9 Safe, 0 Pending, 5 Unsafe, 163 Unknown, 4 data-only | 1098 / 2 / 3699 / 5 | 99, all `Pending` |
| tip-1, tip-2 (`private/opt-in/tip-1`, `tip-2`) | `103c80c9d` (library field names) | same | 1092 / 2 / 3705 / 5 | 113, all `Pending` |
| tip (`private/opt-in/tip-3`) | `b57e81305` (setter names) | same | 1098 / 2 / 3699 / 5 | 113, all `Pending` |

Every run exits 0 with no unmatched row
([runs, invocations](evidence/opt-in/),
[comparison](evidence/opt-in/comparison-base-tip.json) by
OPENSSL-PROVIDER-REMAINING-CALLEES' `evidence/compare.py`).

**At the final tip, no row moved, as predicted.** No row went to `Safe`,
there is no new `Unsafe`, and **no row changed content**, including the
librdkafka rows that call `BIO_free`, so the premise changed no row's
content. All fourteen BIO slots are closed and empty
([sidecar](evidence/opt-in/slot-sets.txt), `callback_members.members`):
Monkey's libevent `bufferevent_openssl.c` is not in either compile
database. The 28 `ssl_ctx_st` sets are identical to the base's.

**Where the prediction was wrong: the first key names.** The first
implementation named the keys after the library's internal fields
(`bio_method_st.bwrite`, `.ctrl`, `.create`, `.destroy`, `bio_st.callback`,
...). Two runs of it (tip-1 alongside the gate, tip-2 alone) gave the same
result: 6 rows moved Safe to Unknown and 120 rows changed content
([comparison](evidence/opt-in/comparison-base-tip2-field-keys.json),
[sets](evidence/opt-in/slot-sets-tip2-field-keys.txt)). The cause:

* Rule 12n's pre-pass reads every unit naming a token of the member closure.
  Those generic field tokens pulled in C++ units naming `create` and
  `destroy`, and a store into an unread record's `callback`.
* They also pulled in `lib/wasm-micro-runtime-WAMR-2.4.1/core/iwasm/interpreter/wasm_loader.c`,
  whose Clang AST JSON exceeds the reader's recursion limit.
* An unread unit opens every member whose closure tokens it names. So every
  BIO key opened, and so did members the card does not touch: the 28
  `ssl_ctx_st` sets, and `rd_kafka_op_s.rko_op_cb`, which moved
  `rd_kafka_event_destroy`, `ut_op_destroy_free` and four other rows from
  Safe to Unknown.

That was fail-closed, but it was noise the card caused. The run never
defines the record, so a key is only a name, and the keys were renamed after
their setters (`b57e81305`). The final tip shows the predicted result. The
evidence of the first naming is kept beside it. Choosing slot keys whose
field tokens no other unit names is now part of the method (`bio_slots.qv`
says why).

## 6. Gate

Command: `python3 scripts/gate_scope.py --base f659b24f5 --out private/gate-N`,
run through `private/toolchain/env.sh` (Clang 21, Lean 4.35.0). Mode: the
scoped card gate (44 test targets and 7 packages selected), with the provider
gate included.

* **gate-2**, the record of the final tree: [`gate/gate-2.json`](gate/gate-2.json),
  logs `private/gate-2/` and `private/logs/gate-2.log`. Tree `b57e81305`;
  the report, evidence and amendments were uncommitted (docs only).
  `gate_scope: 18 steps passed`, exit 0.
* **gate-1**: [`gate/gate-1.json`](gate/gate-1.json), the same 18 steps
  passed on `103c80c9d`, before the keys were renamed. Its tree is
  superseded.

| Step (gate-2) | Exit | Counts |
|---|---:|---|
| submodules, `fmt`, `clippy` (four scoped runs) | 0 | 1 submodule (`third_party/libevent` at `d82464a27`) |
| Rust suite (scoped) | 0 | 274 units, 281 summaries; 3451 passed, 0 failed, 9 ignored, 10 filtered |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| Lean (`check_lean_audits.py`) | 0 | 451 audits (new `QCore.OpensslBioSlots`) |
| Python suite | 0 | 779 tests |
| provider package build, verify-all, verify `qcore.libevent` | 0, 0, 0 | |
| provider tests | 0 | 49 summaries; 158 passed, 0 failed, 1 ignored |
| provider fmt, provider clippy | 0, 0 | |

No new test target was added: the BIO tests are in
`openssl_undeclared_handshake_callbacks`, so the read census check passes,
and neither `test_gate_slot_lane` nor a stale-matrix check failed. When the
integrator applies the amendments, the committed matrix's `lean_audits`
(450) catches up with the 451 produced.

Outside the gate, during the work (`private/logs/`):

* the crate's checker tests: 15 passed (`ptest-3.log`);
* the two OpenSSL gateway test files touched: 12 and 7 passed
  (`gtest-2.log`);
* `lake build QCore.OpensslBioSlots` exit 0, and its audit printed the
  axioms above (`lean-audit.log`).

## 7. Follow-ups

[`integration-amendments.json`](integration-amendments.json) does four
things:

* retires `OPENSSL-BIO-METHODS-AND-CALLBACKS`;
* registers **OPENSSL-BIO-READER-INVOCATIONS**: the PEM and PKCS#12 readers
  run the BIO's callback and methods unstated, and rule 57 bars a slot
  invocation beside their literal-argument premise;
* appends to **OPENSSL-FREE-SESSION-REMOVE-INVOCATION**: once a slot
  invocation can stand beside a drop, state the BIO callback and destroy
  invocations at `BIO_free` and `SSL_free`, and narrow
  `openssl_bio_free_callbacks` to ex_data;
* appends to **OPENSSL-REMAINING-HANDSHAKE-SETTERS**: the premise no longer
  names the BIOs.

`scripts/apply_amendments.py … --check` exits 0.

Files likely to conflict with other OpenSSL cards (OPENSSL-CTRL-COMMAND-ARMS,
OPENSSL-EX-DATA-REGISTRY, OPENSSL-REMAINING-HANDSHAKE-SETTERS,
OPENSSL-FREE-SESSION-REMOVE-INVOCATION, OPENSSL-CT-VALIDATION-LIBRARY-WRITER):

* `providers/quirin-provider-openssl/src/documented.rs` (`Premise`,
  `UNSTATED_SLOTS_STATEMENT`, `SLOTS`, `HANDSHAKE`, `SHUTDOWN`,
  `DOCUMENTED`) and `src/check.rs`;
* `contracts/openssl/ssl_io.qv`, `ssl.qv` and `bio.qv`. The unstated
  premise's statement text changed on all seven declarations that carry it;
* `contracts.lock`, `quirin-plugin.json` and
  `providers/user-crates-load-path.json`. Regenerate them as §3 says;
* `doc-snapshots.json` and `pinned/openssl-3.5.5.json` (append-only);
* `crates/qcore-c-gateway/tests/openssl_undeclared_handshake_callbacks.rs`
  and `openssl_unstated_callback_slots.rs` (invocation counts 41 and 17);
* `docs/QCORE_CONTRACT_SPEC_r0_3.md` C04.18 and C04.19.3, and rule 12n;
* the generated `docs/agents/gateway.md` and `proof/QCore.lean`. Rerun
  `scripts/gen_rule_docs.py` and `scripts/gen_lean_roots.py`.
