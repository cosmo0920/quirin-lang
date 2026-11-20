# OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS: what the calls that free inside run

* **Base:** main `9b107caf1` (integrate/r45 landed).
* **Card:** the named follow-up recorded by
  OPENSSL-OBJECT-FREES-SILENT-METHOD-CALLBACKS (r45).
* **Commits:** [PREDICTION](PREDICTION.md) `33d892378`, committed before any
  implementation; the implementation, lock and Lean, and this report, follow
  it on the branch.

## 1. Sources

OpenSSL 3.5.5 release tarball `openssl-3.5.5.tar.gz`, sha256
`b28c91532a8b65a1f983b4c28b7488174e4a01008e29ce8e69bd789f28bc2a89`
(unpacked into `private/src/`, not committed). No declaration's range or
shape changed, so no new `fetch_docs.py` snapshot was needed.

## 2. What each call frees, and what it now names

| Call | What it frees inside (3.5.5) | Premise now |
|---|---|---|
| `SSL_CTX_use_PrivateKey` | the key it replaces (`ssl_set_pkey`, `ssl/ssl_rsa.c`: `EVP_PKEY_free(c->pkeys[i].privatekey)`) | free statement (**new**) |
| `SSL_CTX_use_PrivateKey_file` | the key it read, after use (`EVP_PKEY_free(pkey)`), the key `SSL_CTX_use_PrivateKey` replaces, and what the PEM or DER read frees on failure | free statement (**new**) |
| `SSL_CTX_use_certificate` | the certificate it replaces (`X509_free(c->pkeys[i].x509)`) and a private key the new certificate does not match (`ssl_set_cert`) | free statement (**new**) |
| `SSL_CTX_use_certificate_chain_file` | the certificate it read (`X509_free(x)` at `end`), a chain certificate it could not add or the last empty one, the chain `SSL_CTX_clear_chain_certs` clears, and what `SSL_CTX_use_certificate` replaces | free statement (**new**) |
| `PEM_read_bio_PrivateKey` | a decoded key lacking the selected components (`pem_read_bio_key_decoder`), a key a failed legacy decode built, and the `OSSL_DECODER_CTX` (`crypto/pem/pem_pkey.c`) | free statement (**new**) |
| `PEM_read_bio_X509` | on a failed decode, the partial certificate (`asn1_item_ex_d2i_intern`, `tasn_dec.c`: `ASN1_item_ex_free`) and the public key it decoded (`x509_pubkey_ex_free`; the eager decode in `x509_pubkey_ex_d2i_ex` also frees its decoder context) | free statement (**new**) |
| `d2i_PKCS12_bio` | on a failed parse, the partial object: a signed PKCS#7 safe's certificates and their keys, and its CRLs (`crl_cb`'s `crl_free`) | free statement (**new**) |
| `SSL_CTX_load_verify_locations` | the `X509_INFO` stack the file lookup read from CAfile (`X509_load_cert_crl_file_ex`, `by_file.c`; `X509_INFO_free`, `x_info.c`): certificates and CRLs the store did not keep (it takes its own reference on those it adds), and every private key in the file (`X509_PKEY_free`). CApath only records a directory | free statement (**new**) |
| `SSL_CTX_set_default_verify_paths` | the same, for the default certificate file (`X509_STORE_set_default_paths_ex`, `x509_d2.c`); the directory and store lookups load nothing here | free statement (**new**) |
| `SSL_connect`, `SSL_do_handshake`, `SSL_read`, `SSL_write` | peer and ephemeral keys (`s->s3.peer_tmp`, `s->s3.tmp.pkey`, key shares in `extensions_clnt.c` and `extensions_srvr.c`), the peer certificate and chain (`statem_clnt.c`, `statem_srvr.c`), key exchange and signature `EVP_PKEY_CTX`s (`s3_lib.c`, `statem_*.c`), transcript digests, and the record layer's cipher contexts at a key change | the handshake statement, **widened** |
| `SSL_shutdown` | a post-handshake message read before the close_notify (a NewSessionTicket, a KeyUpdate) runs the state machine: the session it replaces, the record layer's cipher contexts | the handshake statement, **widened** |
| `X509_new` | nothing whose functions a user can set: a failed allocation frees only the partial certificate, which holds no decoded key | none (`Frees::Nothing`) |

**The free statement is r45's, unchanged.** `quant all #o in #freed:
#teardown(#o) \ (#stated | #settable) |-> #inert`. SP1 defines `#freed` as
the objects whose last reference the call drops "directly, in turn (an object
a freed object owns), or as a replaced or temporary object it frees inside
the call". So the same formula names exactly what each call above frees, as
it does for a free. No new statement or term was needed.

**The handshake statement is widened.** A declaration names each premise
once (C04.18; the loader refuses a second `openssl_unstated_callback_slots`),
so the handshake and I/O calls cannot add the free statement beside theirs.
`UNSTATED_SLOTS_STATEMENT` gains it as its last conjunct, verbatim (the
ex_data conjunct is now parenthesised, as the canonical printing requires):

```
#slots(#of(SSL, #args) | #ctx(#of(SSL, #args)) | #of(SSL_CTX, #args)) \ #stated |-> #inert
&& (quant all #l in #lookups(#store(#ctx(#of(SSL, #args)) | #of(SSL_CTX, #args))): #fns(#l) \ #stated |-> #inert)
&& (quant all #o in #created | #copied | #freed: #ex_data(#class(#o)) \ #stated |-> #inert)
&& quant all #o in #freed: #teardown(#o) \ (#stated | #settable) |-> #inert
```

It stands on the same five declarations. Nothing was narrowed.

**The ex_data readings.** The `EX_DATA_AUDIT` readings of the thirteen calls
whose free reaches a key now also list `RSA`, `DSA`, `DH`, `EC_KEY` and
`ENGINE`, as r45 did for its frees: a key they free may be a legacy key, and
it may drop an ENGINE's last reference. This is checker metadata, read only
for membership in `EX_DATA_CLASSES`.

**An observation, not acted on.** `SSL_CTX_use_PrivateKey`'s ex_data reading
says it frees "a certificate the new key does not match". In 3.5.5
`ssl_set_pkey` returns 0 on a mismatch without freeing the certificate. The
reading is kept because it is an over-approximation and the range reaches
back to 1.0.2.

## 3. The checker's expectation

* **The extension** (`check::inner_frees_audited`, called by
  `check_function` after `free_audited`): a declaration whose `EX_DATA_AUDIT`
  reading is `Runs` with a class in `INNER_FREE_CLASSES` (`EVP_PKEY`,
  `X509`) must be listed in `FREE_AUDIT`, whatever its shape. Otherwise it is
  refused ("... whose free runs functions of methods, providers and engines,
  and the free audit does not list it"). So a future declaration that frees a
  key or certificate inside cannot omit the statement silently.
* **`free_audited` revised:** an entry names the free formula through the
  free statement or through the handshake's statement. A `Frees::Runs` entry
  must name exactly one of the two. An unlisted entry naming the handshake's
  statement is refused, as one naming the free statement is. `NamedBy` and
  `Nothing` refuse either statement. On an entry that does not name the
  handshake's statement, the reading is r45's.
* **`FREE_AUDIT`** gains 15 entries: 14 `Runs` (the table above) and
  `X509_new` as `Nothing`.

**Checker tests** (25 unit tests in the crate, two new):

* `every_inner_free_names_what_it_runs`:
  * the handshake statement ends with the free statement verbatim;
  * every non-release entry is keyed or names the handshake's statement;
  * a free-statement entry names it in its module, dropping it refuses
    through `check_function`, and the handshake's statement in its place
    refuses;
  * a handshake entry names the widened statement, and the statement before
    this card (no free conjunct) refuses;
  * the readings split 9 free-statement entries, 5 handshake entries and 1
    `Nothing`;
  * every keyed declaration is listed;
  * the modules name the free statement exactly 17 times (r45's 8 and these
    9), as often as the audit reads a run through it.
* `a_drifted_inner_free_reading_is_refused`:
  * each class left out of the audit refuses;
  * an unkeyed class, a `NoClass` reading and no reading require nothing;
  * a dropped premise refuses;
  * the handshake's statement meets a run alone, must be listed, and is
    refused beside the free statement or with `Nothing`;
  * every crate declaration passes, and each keyed one is refused once its
    listing is removed.

r45's tests were changed only where this card changes the behaviour:

* `every_free_names_what_it_runs_on_the_objects_it_frees` reads releases
  only, and its module count moved to the new test;
* `a_drifted_free_reading_is_refused` now accepts the handshake's statement
  alone under `Runs`;
* `a_prose_premise_is_refused` counts 67 clauses (was 58).

`qcore-frontend`'s `the_openssl_formulas_are_the_lean_definitions` counts 17
`unstatedFreeFormula` clauses (was 8) and 67 in all.

## 4. Lock, load path and generated files

The regeneration followed r45's order (`private/regen.sh`, logs
`private/logs/regen-b/regen-*.log`, exits in
`private/logs/regen-b/regen.exits`, captured unpiped). `quirin-audit` was
built in release on this tree.

A first attempt (`private/logs/regen-*.log`) named the wrong cargo package
for `quirin-audit` (step 1, exit 101), so the views and pin steps never ran
(127). It is retained and discarded as evidence. The run below starts over
from step 1.

| Step | Exit |
|---|---|
| build `quirin-audit` (`-p qcore-audit --bin quirin-audit`, release) | 0 |
| build `quirin-plugin` (`quirin-provider-plugin`, release) | 0 |
| `quirin-plugin load-path --extends contract-load-path.json --output user-crates-load-path.json quirin-provider-libcurl quirin-provider-openssl`, from `providers/` | 0 |
| `providers/package.py checkers` | 1, as expected (stale views) |
| `quirin-audit plugin views --load-path providers/user-crates-load-path.json --write` | 0 |
| `quirin-audit pin --load-path providers/user-crates-load-path.json --source-root docs/reports/openssl-provider-remaining-callees/evidence/fixture --crate quirin-provider-openssl` | 0 |
| `providers/contracts.py sync` | 0 |
| `providers/package.py checkers` | 0 |

The reviewed `contracts.lock` diff against `9b107caf1` (`private/lockdiff.py`,
`private/logs/lockdiff.txt`):

* **14 entries changed**, `contract_sha256` and `pinned_at` only, as
  predicted:
  * `PEM_read_bio_PrivateKey` `f234864699bd1f0d122ffa3c9bfae38b6d90fa73bf51c22cd198a258cd523dd2`
  * `PEM_read_bio_X509` `be7debdd0a92e748bb083744ae421e477f043bab12057de0755364986a0606a4`
  * `SSL_CTX_load_verify_locations` `0377bb08b7f17fdecd4554d7ade884917f93555cb054476c3e6d050780508dea`
  * `SSL_CTX_set_default_verify_paths` `5f3abe9c158074acd9dfc735648cbb2e92f68e56fc887903199823eed51332f9`
  * `SSL_CTX_use_PrivateKey` `b253abaeac48b25bd1ce5263f0ea5aa079229cbe986aa78f8d53e8ded9e40891`
  * `SSL_CTX_use_PrivateKey_file` `e81d68b3eaf629fcecebc7222a3182050f7c5273cd58d801ca45fe6d2aad4d2d`
  * `SSL_CTX_use_certificate` `770024f32e6a33962da34e6ffe366d04a5191dcbafa4e9eb3d65a75a2e7f064d`
  * `SSL_CTX_use_certificate_chain_file` `76bd422cc038a46248c819ec20c33cc8feec5897b28b7c009f8c2d9b63a57496`
  * `SSL_connect` `aa6e3f972486e748e229cbb7e5059962dfb30c5d497efbb51a1caf5e844be23c`
  * `SSL_do_handshake` `c4733f21d0c5960b9bc24102a858d685fa544b66e5e0189331e329f9dba17242`
  * `SSL_read` `72cf8ba51fade229ee8d24df63b20327c1cffa4357b5c98fd823f2e011c7973d`
  * `SSL_shutdown` `79d5c08c8e1bba47bcefb8d23a5bf780764ad07ba1553b6f4138f5cacffd9654`
  * `SSL_write` `96ca5185900e98f34704e3d7c12701d8368956f01e1a9a10d8780825d8b31038`
  * `d2i_PKCS12_bio` `51a56460fcc71ab71e90bd8ca72ebeb6cebfc23b0d4b85886aeb275d008cec61`
* **0 added, 0 removed, 130 unchanged.** Every top-level key other than
  `flexible` is unchanged.

`user-crates-load-path.json` and `quirin-plugin.json` are unchanged (no
binding added).

`scripts/gen_rule_docs.py` and `scripts/gen_lean_roots.py` were rerun
(`docs/agents/gateway.md`, `proof/QCore.lean`, `proof/AuditAll.lean`).

**Specification and rule:** C04.18 gains "The inner frees"; the C04.18.9
handshake statement and free-statement bullets are amended; rule 12n is
amended in place. No new rule id (65 stays free).

## 5. Lean

* **`QCore.PremiseFormula`:** `unstatedFormula` is the widened tree. The
  projections `unstated_ex_data_part` and `unstated_slots_part` move one
  conjunct deeper. `openssl_formulas_wf` still holds by `decide`.
* **New module `QCore.OpensslInnerFreesAudit`,** audited by
  `proof/AuditOpensslInnerFreesAudit.lean` (19 theorems; 11 depend on no
  axiom, 8 on `propext` only; `private/logs/lean-1.log`,
  `lean-audit-1.log`):
  * the handshake statement entails the free statement in every model, and
    is well formed;
  * a run names exactly one of the two statements, and an entry naming
    either is listed;
  * without the handshake's statement the revised reading equals r45's
    `decideReadings` (`agrees_with_the_release_audit`), so
    `QCore.OpensslObjectFreesAudit`'s theorems hold for those entries;
  * a keyed entry is listed, and one left out refuses whatever its shape;
  * the premise moves no verdict;
  * vectors from the crate's audit.

## 6. Measurement

**No move predicted, and not re-measured.** The reasons:

* the card changes premise text only, and a premise is metadata that no
  verdict-deciding consumer reads (SP1, "Semantics unchanged");
* no gateway code changed;
* no pinned route loads the OpenSSL crate;
* r45 measured the same kind of change (five frees gaining this premise)
  with the opt-in `current` run and found no row moved and no content
  changed;
* the checker accepts all 14 changed declarations (`every_contract_passes_its_own_checks`,
  the regeneration's `package.py checkers` and the gate's provider tests), so
  the bindings stay `Pending`.

The prediction's `measure-prediction` block (none on corpus, system and
slice) is for the integrator's `quirin-audit measure --declared`.

## 7. Gate

GATE_SECTION

## 8. Follow-ups

[`integration-amendments.json`](integration-amendments.json) retires
OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS and registers:

* **OPENSSL-INNER-FREES-OTHER-CLASSES:** calls that free an engine, context
  or store inside are outside `INNER_FREE_CLASSES`:
  * `ENGINE_by_id`, for a dynamic engine that fails to load;
  * `OPENSSL_init_ssl`, through the configuration's engines;
  * `SSL_CTX_new` and `SSL_new`, which free the partial object on failure;
  * `SSL_set_accept_state`.
* **OPENSSL-INNER-FREES-BIO-EX-DATA:** these calls create and free a
  temporary BIO, yet their ex_data readings omit `BIO`:
  * `SSL_CTX_use_PrivateKey_file` and `SSL_CTX_use_certificate_chain_file`;
  * `PEM_read_bio_PrivateKey`;
  * the two verify-path calls.

  `SSL_shutdown`'s reading lists `SSL_SESSION` only.

It appends to **OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS** (the parallel
card): a setter it declares for a key's or engine's method function makes
that slot `#settable`. The slot then falls out of the free statement on
these 14 declarations too, so each must state the invocation, not only the
`*_free` declarations.

**Files likely to conflict** with OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS:

* `providers/quirin-provider-openssl/src/documented.rs` (`FREE_AUDIT`,
  `EX_DATA_AUDIT`, `UNSTATED_SLOTS_STATEMENT`, the premise lists) and
  `src/check.rs` (`free_audited`, the `pub use`, the r45 tests, the
  prose-premise count 67);
* `crates/qcore-frontend/tests/premise_formula.rs` (the counts 17 and 67);
* `contracts.lock` (regenerate in §4's order);
* `docs/QCORE_CONTRACT_SPEC_r0_3.md` C04.18 and C04.18.9, and
  `docs/agents/rules/12n.md` (appended at the end);
* `proof/QCore/PremiseFormula.lean`;
* the generated `docs/agents/gateway.md` and `proof/QCore.lean` (rerun the
  generators).
