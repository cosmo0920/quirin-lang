# OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS: the teardown slots of the frees

* **Base:** main `9b107caf1` (integrate/r45 with the SP1 landing record).
* **Card:** the named follow-up recorded by OPENSSL-FREE-INDIRECT-CALLBACKS
  and appended by OPENSSL-OBJECT-FREES-SILENT-METHOD-CALLBACKS.
* **User decisions (2026-11-21)**, which replaced the brief's "do not
  decide" for its decision items:
  1. provider dispatch tables: **"Named premise + built-in source"**;
  2. QUIC: **"Stated slot from source"**;
  3. setters with no 3.5 man page: **"Admit pinned source"**.
* **Commits:** [PREDICTION](PREDICTION.md) `3bb152086`, committed before any
  implementation; the implementation with its lock, Lean and records in the
  commit after it (the tip the reply names).

## 1. Sources

OpenSSL 3.5.5 release tarball `openssl-3.5.5.tar.gz` from openssl.org,
sha256 `b28c91532a8b65a1f983b4c28b7488174e4a01008e29ce8e69bd789f28bc2a89`
(the digest r45 recorded), unpacked in `private/src/` (not committed). New
3.5 page snapshots were fetched with `fetch_docs.py --merge` (2026-11-20 UTC):
`RSA_meth_new`, `DSA_meth_new`, `DH_meth_new`, `ASYNC_WAIT_CTX_new`,
`EVP_MD_meth_new`, `EVP_CIPHER_meth_new` at 3.5; `EVP_PKEY_ASN1_METHOD` at
1.1.1, 3.0 and 3.5 (1.0.2: HTTP 404); `EVP_PKEY_meth_new` at 1.0.2, 1.1.1,
3.0 and 3.5. The recorded `ENGINE_add` 3.5 snapshot lacked its NAME; the new
`fetch_docs.py --name` mode refetched it, refused any digest change (it
matched, `50979256…`) and recorded only `name`. The prototypes were pinned
with `pin_prototypes.py` over every recorded function plus the 13 new ones,
its header list gaining `rsa.h`, `dsa.h`, `dh.h`, `ec.h` and `async.h`: every
existing prototype and header digest is byte-identical, and no header was
added (each was already included transitively).

Two read-only source censuses ran as subagents over the same tarball: the
built-in providers' dispatch functions the frees can reach, and the QUIC
object free path past `quic_impl.c`. Their findings, checked against the
source where the card relies on them, are §3 and §4.

## 2. What was declared and stated

### Setters (`teardown_slots.qv`, 13 setters, 18 slots)

| Setter | Slot(s) | Evidence | Range |
|---|---|---|---|
| `RSA_meth_set_finish` | `rsa_meth_st.rsa_meth_finish` | `RSA_meth_new` 3.5 | `since "1.1.0"` |
| `DSA_meth_set_finish` | `dsa_method.dsa_meth_finish` | `DSA_meth_new` 3.5 | `since "1.1.0"` |
| `DH_meth_set_finish` | `dh_method.dh_meth_finish` | `DH_meth_new` 3.5 | `since "1.1.0"` |
| `ENGINE_set_finish_function` | `engine_st.engine_finish` | `ENGINE_add` 1.1.1 (NAME at 3.5) | `documented "1.1.1"` |
| `ENGINE_set_destroy_function` | `engine_st.engine_destroy` | `ENGINE_add` 1.1.1 | `documented "1.1.1"` |
| `EVP_PKEY_asn1_set_free` | `evp_pkey_asn1_method_st.asn1_meth_pkey_free` | `EVP_PKEY_ASN1_METHOD` 1.1.1 | `documented "1.1.1"` |
| `ASYNC_WAIT_CTX_set_wait_fd` | `async_wait_ctx_st.wait_fd_cleanup` (retains `key`, `custom_data`) | `ASYNC_WAIT_CTX_new` 3.5 | `since "1.1.0"` |
| `EVP_MD_meth_set_cleanup` | `evp_md_st.md_meth_cleanup` | `EVP_MD_meth_new` 3.5 | `since "1.1.0"` |
| `EVP_CIPHER_meth_set_cleanup` (census find) | `evp_cipher_st.cipher_meth_cleanup` | `EVP_CIPHER_meth_new` 3.5 | `since "1.1.0"` |
| `EVP_PKEY_meth_set_cleanup` | `evp_pkey_method_st.pkey_meth_cleanup` | `EVP_PKEY_meth_new` 1.0.2 | `documented "1.0.2"` |
| `EC_KEY_METHOD_set_init` | six `ec_key_method_st.ec_key_meth_*` (init, finish, copy, set_group, set_private, set_public) | **source-derived**: `crypto/ec/ec_kmeth.c:194` | `since "3.5.5"` |
| `ENGINE_set_pkey_meths` | `engine_st.engine_pkey_meths` | **source-derived**: `crypto/engine/tb_pkmeth.c:92` | `since "3.5.5"` |
| `ENGINE_set_pkey_asn1_meths` | `engine_st.engine_pkey_asn1_meths` | **source-derived**: `crypto/engine/tb_asnmth.c:105` | `since "3.5.5"` |

Each documented slot's page NAME list is accounted for (writers and
not-writers), as for every slot since r30. The brief listed
`EC_KEY_METHOD_set_init` as documented; it has no 3.5 page (r45 said so
too), so it falls under decision 3 and is source-derived. A source-derived
entry's series is `source 3.5.5`, its page the defining file, and it is cited
once in `SOURCE_CITATIONS` (file, line, reading); its range admits only the
release read. A source-derived slot (`SOURCE_SLOTS`) has no NAME list; its
writers are every store into the field the source makes (a search of the
tree found only these writers, beside `EC_KEY_METHOD_new`'s and the dynamic
engine's `ENGINE_cpy` copies). The engine enumerators are called first with a
NULL method pointer, then with NULL nids (`tb_pkmeth.c:110,112`,
`tb_asnmth.c:123,125`), so those actuals are `supplied_nullable`, cited in
`SOURCE_NULLABLE_ARGUMENTS`.

### The frees

| Free | Teardown slots stated | Premises now | What is left |
|---|---|---|---|
| `EVP_PKEY_free` | the key's 9 (ASN.1 free handed the key) | builtin + loaded teardown, ex_data | nothing: **drops** the unstated premise |
| `X509_free` | the key's 9 | builtin + loaded, ex_data | nothing: **drops** it |
| `ENGINE_free` | destroy and both enumerators, handed the engine | builtin + loaded, ex_data | nothing: **drops** it |
| `X509_STORE_free` | the key's 9 | free statement, builtin + loaded, ex_data, lookup | a CRL's `X509_CRL_METHOD` free |
| `SSL_CTX_free` | the key's 9 | as above | a CRL's method free |
| `SSL_free` | all 13 teardown slots, and the message and info callbacks | as above, BIO premises | a CRL's method free; QUIC TLS `crypto_release_rcd` |
| `PKCS12_free` | none (releases its argument) | released free statement, builtin + loaded, ex_data | its teardown slots (C04.19.3); a CRL's method free |
| `EVP_MD_CTX_free` | none (releases its argument) | released free statement, builtin + loaded, ex_data | its teardown slots (C04.19.3) |

The key's nine are the ASN.1 method free, the RSA, DSA, DH and EC_KEY method
finish, and the engine's finish, destroy and two enumerators (a key's engine
can drop its last reference). Why each is run, at 3.5.5: `RSA_free`,
`DSA_free`, `DH_free`, `EC_KEY_free` call `meth->finish` then `ENGINE_finish`
(`rsa_lib.c:149-152`, `dsa_lib.c:225-228`, `dh_lib.c:149-153`,
`ec_key.c:84-88`); `engine_unlocked_finish` runs `e->finish`
(`eng_init.c:66-69`) and `engine_free_util` the enumerators and `e->destroy`
(`eng_lib.c:89-96`); `evp_pkey_free_legacy` runs `ameth->pkey_free`;
`EVP_PKEY_CTX_free` runs `pmeth->cleanup` (`pmeth_lib.c:394-395`);
`cleanup_old_md_data` runs `digest->cleanup` (`digest.c:32-34`);
`EVP_CIPHER_CTX_reset` runs `cipher->cleanup` (`evp_enc.c:57`);
`ASYNC_WAIT_CTX_free` runs each fd's cleanup (`async_wait.c:32-33`).

**Why `PKCS12_free` and `EVP_MD_CTX_free` state no slot.** Both are
`resource_released` frees, and the loader reads a slot invocation only through
a `resource_borrowed` parameter (C04.19.3, `contract_callback.rs`'s `object`);
the attempt was refused at load (`a slot invocation's object names a
resource_borrowed parameter`). With the setters declared, their functions are
`#settable`, which the free statement subtracts, so leaving these two on the
free statement would have narrowed it silently. They name the statement of a
free through a released argument, `quant all #o in #freed: #teardown(#o) \
#stated |-> #inert`, which entails the free statement
(`released_statement_entails_free_statement`): nothing is narrowed.

**`SSL_free`'s message and info callbacks.** The QUIC census found that the
record layer's clear can run `ossl_statem_fatal` and so `ssl3_send_alert` on a
fatal record release (`s3_msg.c`: message callback, info callback), an error
path of any SSL object's free. Their setters are declared, so they are
`#settable` and the free statement never covered them: a silent gap before
this card, now stated (`MESSAGE`, `INFO`).

### The checker

* `FREE_AUDIT`'s `Frees::Runs` lists each free's teardown slots and what it
  leaves; the new `Frees::RunsThroughRelease` lists what a released free
  leaves. `free_audited` refuses a free naming the free statement with
  nothing left, one dropping it with something left, either teardown premise
  missing, an audited slot not stated, a stated teardown slot not audited, a
  slot that is no teardown slot, and a released free stating a slot or naming
  the plain free statement.
* `source_cited` refuses a source-derived entry cited zero or several times,
  by another file, or with any other range, and a documented entry cited from
  the source. `source_slot_writers_declared` and
  `source_nullable_argument_cited` are the source-side counterparts of the
  page checks.
* `EX_DATA_AUDIT`: `SSL_free`, `SSL_CTX_free` and `X509_STORE_free` gain
  `RSA`, `DSA`, `DH`, `EC_KEY` and `ENGINE` (and `X509_STORE_free` `EVP_PKEY`).
  Metadata.

**Checker tests** (24 unit tests before the merge of §11, 27 after): `the_teardown_setters_register_their_slots`
(new), `every_free_names_what_it_runs_on_the_objects_it_frees` and
`a_drifted_free_reading_is_refused` (rewritten for the new reading),
`a_prose_premise_is_refused` (71 clauses before the merge, 108 after (§11): SP1's 53, r45's five, less the
three frees that now state everything, plus the 16 teardown premises),
`every_slot_has_every_writer_declared` (87 slots, 8 source slots).

**Gateway tests** (`openssl_undeclared_handshake_callbacks`, 29 tests, no new
target): the free and reader fixtures load `teardown_slots.qv`; the frees'
slot lists and actuals are pinned; an uncontracted RSA finish function and an
uncontracted engine destroy function each refuse `free_store`, `free_ctx` and
`free_ssl`, naming the candidate, and spare `free_bio`. Before the fixtures
loaded the new module, the frees refused with `no declaration registers the
slot` (the fail-closed arm, exercised).

## 3. Decision 1: provider dispatch tables

Census (read-only, file:line in the subagent's report, checked where used):

* **Built-in providers** (default, base, legacy, null): key management frees
  of RSA, EC, DH and DSA keys call `RSA_free` and siblings, which run the
  application's default method's finish and `ENGINE_finish`; MAC key frees
  and MAC and KDF context frees reach `ENGINE_finish`; signature contexts
  free digest contexts, which reach a legacy (engine) digest's cleanup; a MAC
  context can free a cipher context, which reaches a legacy cipher's cleanup.
  Every one of these is a stated slot now. The digest, cipher, ECX, ML-KEM,
  ML-DSA and SLH-DSA functions run nothing application-settable.
  `ossl_provider_free` at the last reference runs only the provider's
  teardown; the default and base teardowns free a BIO method struct (no
  destroy callback) and their context; the legacy teardown frees its child
  library context. The `EC_METHOD` `keyfinish` is 0 in every built-in method.
* So the built-in functions are `#library`, under
  `openssl_builtin_teardown_functions`; what they run in turn is stated (a
  callback run in turn on an object freed in turn is that object's teardown,
  read on its own: reading **R6**).
* **Any other provider** is `#loaded`, under the one named premise the user
  decided, `openssl_loaded_module_functions`. **Reading R7:** the premise
  states that such a function is inert (it takes, releases or retains nothing
  the caller holds), the effect the free statement asks of every teardown
  function, rather than the decision's wording "runs no application
  callback"; an application callback it ran in turn would be a member of a
  freed object's teardown, stated or under these premises. **Reading R8:** an
  engine the dynamic engine loads from a shared object registers its
  functions in code outside the run, so the run's slot set cannot see them;
  `#loaded` includes such engines, the same kind of out-of-run module. Both
  readings are flagged for the integrator to confirm with the user.
* Not covered by either: the allocator's free hook
  (`CRYPTO_set_mem_functions`), which every free runs. It is not a teardown
  function and no premise ever named it: registered as
  OPENSSL-ALLOCATOR-FREE-HOOK.

## 4. Decision 2: QUIC

**The trace** (`SSL_free` on a QUIC connection, stream, listener or domain):
`ossl_quic_free` frees the default stream, the internal TLS SSL object
(`SSL_free(qc->tls)`, a full `ossl_ssl_connection_free`), the channel, port,
engine and reactor, and drops the application's network BIOs with
`BIO_free_all`. The channel's internal callbacks (ACK manager discards,
frame queue frees, token frees, the congestion controller) resolve to
library functions; the qlog sink has no public setter. What is
application-settable and runs: ex_data free callbacks (the QUIC object's and
the internal TLS object's), the BIO callback and destroy method of the
network BIOs (stated BIO slots), the session-remove callback through the
internal TLS object's session (stated), the key and engine teardown slots,
the async wait context's cleanups (stated now), the lookup slots and the
client certificate engine's finish at the context's last drop (stated), and,
on an error path, the message and info callbacks (stated now). Nothing the
trace reached is left unstated.

**`SSL_set_quic_tls_cbs` was not declared.** The decision asked for its
`crypto_release_rcd` to be a stated slot. Two facts from the trace and the
loader:

* it is rejected on QUIC objects (`quic_tls_api.c:132`, `SSL_is_tls`,
  `ssl_lib.c:992-999`); it applies
  to a TLS object of a third-party QUIC stack, whose `SSL_free` runs the
  application's `crypto_release_rcd` from `RECORD_LAYER_clear`;
* it is handed an `OSSL_DISPATCH` table, not a function pointer, and a slot
  registration's callback must be a function pointer parameter (C04.19.3,
  `contract_callback.rs`'s `function_pointer`). Reading the table's entries
  is a vocabulary change, outside this card (gateway semantics).

So `SSL_free` keeps the free statement for it; the amended ledger entry
records the vocabulary decision it needs.

## 5. Decision 3: setters with no 3.5 page

* **Declared, source-derived:** `ENGINE_set_pkey_meths`,
  `ENGINE_set_pkey_asn1_meths` and `EC_KEY_METHOD_set_init` (§2).
* **`X509_CRL_METHOD_new` was not declared.** It stores the four functions
  into the method it returns (`x_crl.c:486-504`), and a slot registration's
  holder is a parameter (C04.19.3); there is no parameter to register into.
  The free statement stays for a CRL's `crl_free` on the four frees that free
  CRLs; the amended entry records the vocabulary decision.
* **`SSL_CTX_set_client_cert_engine` was not declared.** It stores an
  ENGINE, not a function (`tls_depr.c:81-94`), so it writes no slot; the
  engine functions `SSL_CTX_free` runs on it are the engine slots, stated
  whatever stored the engine. Declaring the setter itself needs its
  `ENGINE_init` (the engine's init function, an undeclared slot) and its
  failure path's `ENGINE_finish` stated: registered as
  OPENSSL-CLIENT-CERT-ENGINE-SETTER.

## 6. Lock, load path and generated files

Regeneration in r45's order, logs `private/logs/regen-*.log`:

| Step | Exit |
|---|---|
| build `quirin-audit` (release, this tree: `qcore-frontend` changed) | 0 |
| build `quirin-plugin` | 0 |
| `quirin-plugin load-path --extends contract-load-path.json --output user-crates-load-path.json quirin-provider-libcurl quirin-provider-openssl` | 0 |
| `providers/package.py checkers` | 1, as expected (stale views) |
| `quirin-audit plugin views --load-path providers/user-crates-load-path.json --write` | 0 |
| `quirin-audit pin ... --crate quirin-provider-openssl` (through the toolchain env: the first try without Clang on PATH exited 2) | 0 |
| `providers/contracts.py sync` | 0 |
| `providers/package.py checkers` (after the comment edits too) | 0, 0 |

The reviewed `contracts.lock` diff against `9b107caf1`
(`private/lockdiff.py`, `private/logs/lockdiff.txt`):

* **13 entries added:** `ASYNC_WAIT_CTX_set_wait_fd` `b1b62f2b…`,
  `DH_meth_set_finish` `38e41fcc…`, `DSA_meth_set_finish` `32a2b7ab…`,
  `EC_KEY_METHOD_set_init` `f91f9a4f…`, `ENGINE_set_destroy_function`
  `31ca3cb4…`, `ENGINE_set_finish_function` `77812795…`,
  `ENGINE_set_pkey_asn1_meths` `32e04399…`, `ENGINE_set_pkey_meths`
  `1af9f1d2…`, `EVP_CIPHER_meth_set_cleanup` `d6dcf5ee…`,
  `EVP_MD_meth_set_cleanup` `e8239399…`, `EVP_PKEY_asn1_set_free`
  `43cab4a0…`, `EVP_PKEY_meth_set_cleanup` `ac22ee63…`,
  `RSA_meth_set_finish` `8aac420f…`.
* **8 entries changed** (`contract_sha256`, `pinned_at`): `ENGINE_free`
  `279e80f3…`, `EVP_MD_CTX_free` `91329ad8…`, `EVP_PKEY_free` `0ce1e939…`,
  `PKCS12_free` `9713c91d…`, `SSL_CTX_free` `b7d912e0…`, `SSL_free`
  `56132634…`, `X509_STORE_free` `0322b6cf…`, `X509_free` `2f562571…`.
* **0 removed, 136 unchanged.** Only `flexible` differs among the top-level
  keys.

`user-crates-load-path.json` and `quirin-plugin.json` gain 13 bindings in one
module. `scripts/gen_rule_docs.py` and `scripts/gen_lean_roots.py` were rerun.
**Specification:** C04.18 gains "The teardown slots"; C04.18.9 gains the terms
`#library` and `#loaded` and the three statements. Rule 12n is amended in
place; no new rule id (64 stays free).

## 7. Lean

`QCore.PremiseFormula` gains the two vocabulary terms (and model components),
`builtinTeardownFormula`, `loadedTeardownFormula`,
`unstatedReleasedFreeFormula` and `teardown_formulas_wf`; its three kind
lemmas grow with the vocabulary. The new module
`QCore.OpensslFreeTeardownSlots`, audited by
`proof/AuditOpensslFreeTeardownSlots.lean` (22 theorems after the merge; axioms within
`propext`, `Classical.choice` and `Quot.sound`):

* `teardown_premises_cover_the_free_statement`: when every teardown function
  of every freed object is stated, the library's or a loaded module's, the
  two teardown premises entail the free statement, so dropping the
  unstated premise assumes nothing more;
* `released_statement_entails_free_statement` and
  `free_statement_does_not_entail_released`;
* the audit: `a_left_item_keeps_the_premise`, `nothing_left_drops_the_premise`,
  `a_running_free_names_the_teardown_premises`,
  `the_stated_slots_are_the_audited`, `a_released_free_states_no_slot`;
* `a_source_entry_admits_only_the_release_read`;
* vectors from the crate's audit, as landed and drifted.

The Rust bindings: `qcore-frontend`'s `the_vocabulary_is_the_lean_table`
(29 terms) and `the_openssl_formulas_are_the_lean_definitions` (71 clauses
before the merge: 3 free statements, 2 released, 8 and 8 teardown premises;
108 after, §11).

## 8. Measurement

**Not re-measured.** The opt-in `current` run over Fluent Bit takes hours per
side on this host and would measure only metadata: the premises are read by
no verdict-deciding consumer (SP1), no gateway code changed, and no Fluent
Bit or Monkey source calls any of the 13 setters (searched in the local
Fluent Bit checkout), so each new slot's run set is the closed empty set and
invokes nothing. The bindings grow by 13, all expected `Pending`. Pinned
routes read no user load path; the prediction's `measure-prediction` block
(none on corpus, system and slice) is for the integrator's `quirin-audit
measure --declared`.

## 9. Gate

See [`gate/`](gate/) and the reply; filled in after the run.

## 10. Follow-ups and conflicts

[`integration-amendments.json`](integration-amendments.json) amends
OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS to the three vocabulary gaps (§2,
§4, §5) and registers OPENSSL-CLIENT-CERT-ENGINE-SETTER and
OPENSSL-ALLOCATOR-FREE-HOOK. `ENGINE_by_id`'s failed load (an engine's
destroy function and method enumerators run inside a constructor) is already
tracked by r46's OPENSSL-INNER-FREES-OTHER-CLASSES; when that card states it,
it must state the engine teardown slots (§11).

**Files likely to conflict** (OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS
runs in the same crate):

* `providers/quirin-provider-openssl/src/documented.rs` (`Premise`,
  `Frees`, `FREE_AUDIT`, `SLOTS`, `EX_DATA_AUDIT`, `DOCUMENTED`, the free
  premise arrays) and `src/check.rs` (the `pub use`, `free_audited`,
  `expected_clauses`, tests and their clause counts);
* `contracts.lock`, `quirin-plugin.json`, `src/lib.rs`,
  `providers/user-crates-load-path.json` (regenerate in §6's order);
* `pkcs12.qv` (`PKCS12_free` beside `d2i_PKCS12_bio`), `ssl.qv`, `evp.qv`,
  `x509.qv`, `x509_store.qv`, `engine.qv`, `evp_digest.qv`;
* `doc-snapshots.json`, `pinned/openssl-3.5.5.json` (append-only);
* `crates/qcore-frontend/tests/premise_formula.rs` (the formula counts) and
  `crates/qcore-c-gateway/tests/openssl_undeclared_handshake_callbacks.rs`;
* `proof/QCore/PremiseFormula.lean` (vocabulary), C04.18 and C04.18.9,
  `docs/agents/rules/12n.md`; the generated `docs/agents/gateway.md`,
  `docs/agents/README.md`, `proof/QCore.lean`, `proof/AuditAll.lean`.

## 11. Merge with OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS (r46)

The integrator landed OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS first
(branch tip `dafe6e4d9`, landing as r46) and asked this card to merge it and
keep its fourteen declarations sound. The first gate attempt (`private/gate-1`,
on `983429601`) was stopped for the merge: Lean (470 audits) and the scoped
Rust suite had passed, the Python suite had the known slot flake
(`test_each_waiter_shows_its_queue_position`), and the provider steps had not
run. It is discarded as evidence.

**The soundness point.** That card put r45's free statement (`#teardown(#o)
\ (#stated | #settable)`) on nine calls that free a key inside (the four
`SSL_CTX_use_*` loaders, the two PEM readers, `d2i_PKCS12_bio`, the two
verify-path calls) and conjoined it to the handshake statement of the
handshake and I/O calls and `SSL_shutdown`. The setters this card declares
make the key's method finish, ASN.1 free and engine functions `#settable`, so
without more the free statement would no longer cover them on those fourteen
calls. Each now states them, as the frees do:

* the nine: the key's nine teardown slots through their borrowed formal at
  position 0 (the context or the BIO; the key is run-wide), every actual `_`;
* the five handshake calls: those nine and the legacy digest, cipher and
  public key method cleanups (transcript digests, record-layer cipher
  contexts, key exchange and signature contexts);
* all fourteen name `openssl_builtin_teardown_functions` and
  `openssl_loaded_module_functions`, and keep the free statement (or the
  handshake's free conjunct) for whatever else that card's survey names
  (`INNER_FREES_REST`; `CRL_METHOD_FREE` too where CRLs are freed). This card
  narrows none of it.

**The checker after the merge.** `Frees::Runs` keeps r46's reading that the
handshake statement stands for the free statement, never beside it; with it,
`left` is not checked (the conjunct names it). The new
`every_inner_free_states_its_teardown_slots` test pins all fourteen (each
states the key's nine, checks, and refuses when a slot line or a teardown
premise is dropped); r46's `every_inner_free_names_what_it_runs` and
`a_drifted_inner_free_reading_is_refused` keep passing with the teardown
premises added (the free statement now stands on twelve declarations: the
nine and r39's three frees). Counts: 108 premise clauses refused as prose;
the frontend binds 12 free statements, 2 released, 22 and 22 teardown
premises. Lean: `decideReading` gains the handshake case, with
`the_handshake_statement_never_beside_the_free_statement` and
`vectors_inner_frees`.

**Gateway tests.** The crate and reader fixtures load `teardown_slots.qv`
(without it, `SSL_write` and `SSL_shutdown` refused with `no declaration
registers the slot`, the fail-closed arm); the handshake calls state 79 slot
invocations (67 + 12), `SSL_shutdown` 30 (18 + 12), the readers 24 and 23.

**Lock against `dafe6e4d9`** (`private/logs/lockdiff-merged-vs-inner.txt`):
13 added (the setters, digests as in §6), 22 changed (the eight frees, digests
as in §6, and the fourteen inner frees: `PEM_read_bio_PrivateKey`
`b43fb07d…`, `PEM_read_bio_X509` `5cbefbcd…`, `SSL_CTX_load_verify_locations`
`601d39f9…`, `SSL_CTX_set_default_verify_paths` `3d843f7a…`,
`SSL_CTX_use_PrivateKey` `a7509e54…`, `SSL_CTX_use_PrivateKey_file`
`a7b60f93…`, `SSL_CTX_use_certificate` `19825bae…`,
`SSL_CTX_use_certificate_chain_file` `4609df44…`, `SSL_connect` `fcfb4694…`,
`SSL_do_handshake` `30224787…`, `SSL_read` `b5f0899a…`, `SSL_shutdown`
`6c65576d…`, `SSL_write` `10113fcc…`, `d2i_PKCS12_bio` `5e7d9cd8…`), none
removed, 122 unchanged; only `flexible` differs. The regeneration repeated
§6's order (`private/logs/regen-10` to `regen-14`, `checkers` 1 then views,
pin, sync 0).
