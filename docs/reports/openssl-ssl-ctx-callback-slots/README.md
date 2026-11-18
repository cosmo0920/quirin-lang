# OPENSSL-SSL-CTX-CALLBACK-SLOTS: opaque callback slots replace the OpenSSL registered-callbacks premise

Base `974434463` (main). Named follow-up `OPENSSL-SSL-CTX-CALLBACK-SLOTS`
(OPENSSL-PROVIDER-REMAINING-CALLEES), user decision 2026-11-18: replace the
named premise `openssl_registered_callbacks` (C04.18) with stated callback
registrations and invocations. [PREDICTION](PREDICTION.md) was committed
before any implementation (`1cc63d80b`).

## 1. Why C04.19.3 needed a marked form

C04.19.3 already states a setter's registration into a member and an
invocation of a member, decided over the run's closed set of stores (rule
12n). It was evaluated first, as the brief asked. As written it cannot decide
an OpenSSL slot: `SSL_CTX` and `SSL` are opaque (`typedef struct ssl_ctx_st
SSL_CTX;` in the public headers), no unit of a run defines the record, and
rule 12n decides a member only when it is a function-pointer member of a
record the run defines (`RunMemberFacts::normalise`). Dropping that
requirement for every undefined record would be unsound: a run that does not
compile librdkafka would then decide `rd_kafka_s` members over the declared
setters alone, missing the library's own stores. So the card adds the
smallest marked variant and reuses everything else (the run's closure, the
pre-pass, the fold, C04.19's loop-safe composition):

* `resource_callback_slot_registered(P, TAG.FIELD, F)` and
  `resource_callback_slot_invoked(OBJ, TAG.FIELD, A…)`: the member forms
  with `slot: true` (`qcore_schemas::callback::CallbackClause`, a serde
  field skipped when false, so every existing clause serialises as before).
  Loader (`contract_callback.rs`): a borrowed depth-one object or holder with
  no kind, one field below the tag, no `supplied_owned`, no id, one clause per
  object and key; a malformed clause rejects the module. Grammar G05 updated.
* **Decision** (`RunMemberFacts::decides`, `Parser::member_candidates`; rule
  12n amended in place, C04.19.3 "An opaque slot"): a slot is decided only
  over the run's facts, only while no unit of the run defines `struct TAG`
  (a unit that does can store into it directly), and only when a
  declaration registers it (a store of a declared setter's formal under
  exactly that key); then over the run's closed set with rule 13u's
  composition. Otherwise `callback-member-open`, naming why. The member form
  is unchanged.
* **Premise `P-OPAQUE-SLOT-WRITERS`** (spec level, beside
  `P-CG22-MEMBER-CLOSED-WORLD`): every writer of the slot is declared. No
  unit of a run can write through an incomplete type; the library's own
  writes are its declared setters. The OpenSSL checker enforces the
  documented half: every writer the slot's manual page names (NAME section of
  the 3.5 snapshot) must be declared as that slot's setter
  (`check::slot_writers_declared`).

No gateway rule was added (rule 58 stays unused): 12n is amended in place.

## 2. The declarations

Every claim cites a fetched page; five new 3.5 snapshots
(`SSL_CTX_set_security_level`, `SSL_CTX_set_info_callback`,
`SSL_CTX_set_verify`, `SSL_CTX_set_client_cert_cb`,
`CRYPTO_get_ex_new_index`, fetched 2026-11-18 with `fetch_docs.py --merge`); the pages already cited were re-read at their
recorded digests (each refetched page's SHA-256 equals the recorded one).
Prototypes: `pin_prototypes.py` over the three new setters; every existing
entry byte-identical. The checker reads an in-place function pointer
(`int (*cb)(SSL *ssl, void *arg)`) as `fnptr` and refuses every other
parenthesised spelling.

| Declaration | Before | Now |
|---|---|---|
| `SSL_CTX_set_default_passwd_cb` (new) | uncontracted | slot registration `ssl_ctx_st.default_passwd_cb` (the context's one writer: the page's `SSL_set_*` setters write the SSL's copy, which nothing declared invokes) |
| `SSL_CTX_set_cert_cb`, `SSL_set_cert_cb` (new) | uncontracted | slot registration `ssl_ctx_st.cert_cb`, each retaining `arg` in its holder (C04.14). NAME (3.5, 1.0.2) lists exactly these two writers. The SSL's copy shares the key: `SSL_set_cert_cb` writes it and `SSL_new` copies the context's ("inherits the settings of the underlying context"), so the run's set is every value either can hold |
| `SSL_CTX_use_PrivateKey_file` | premise | invokes `ssl_ctx_st.default_passwd_cb` with four supplied values (`pem_password_cb(buf, size, rwflag, u)`); **no premise** |
| `SSL_CTX_use_certificate_chain_file` | premise | invokes the password slot; keeps `openssl_unstated_callback_slots` for the security callback (SSL_CTX_set_security_level, 3.5: "Attempts to set certificates or parameters with insufficient security are also blocked"), whose setters are not declared |
| `SSL_write`, `SSL_read`, `SSL_connect`, `SSL_do_handshake` | premise | invoke `ssl_ctx_st.cert_cb(ssl, arg)` (SSL_CTX_set_cert_cb, 3.5: "called before a certificate will be used by a client or server"; the I/O calls negotiate when needed); keep the narrowed premise for the verify, info, message, security, session and client-certificate callbacks, whose setters are not declared (several are `SSL_CTX_ctrl` command arms, out of scope) |
| `SSL_shutdown` | silent | names the narrowed premise (the info callback "is called whenever ... an alert appears") |
| `SSL_free`, `SSL_CTX_free` | silent | name the narrowed premise (SSL_CTX_free, 3.5: the session-remove callback "will be called for each session being freed"; ex_data free callbacks); the NULL-accepting reference drop is kept |

The premise `openssl_registered_callbacks` is gone. Its replacement,
`openssl_unstated_callback_slots`, names only the callbacks a declaration may
run and does not state. A premise is metadata (C04.18): no verdict moves by
it. One declaration of the six is premise-free; five keep the narrower
premise beside a stated invocation, which is strictly stronger than before (a
run that registers an effectful or uncontracted certificate or password
callback now refuses those calls). The brief allowed this ("keep that
declaration on the named premise ... and record a follow-up"); the remaining
slots are follow-up `OPENSSL-UNSTATED-CALLBACK-SLOTS` (§6).

**The checker** (`documented.rs`, `check.rs`): new shape `RegistersSlot`
(registration, optional retention, every parameter borrowed, no result
custody), premises `SlotInvoked` and `UnstatedSlots` (replacing
`RegisteredCallbacks`), and the slot table `SLOTS` with each slot's page and
writers. `expected_clauses` derives each new clause set exactly; tests refuse
a dropped invocation, the wrong slot, a supplied value in place of the
connection, the member form in place of the slot form, the premise given back
to the premise-free loader, a dropped retention or registration, the frees'
premise dropped, an unknown slot and a page that does not name a writer.

**Lock** (`quirin-audit pin` with the tip binary; `providers/package.py
checkers` then `pin --verify-contracts` exit 0): 3 entries added
(`SSL_CTX_set_cert_cb`, `SSL_CTX_set_default_passwd_cb`, `SSL_set_cert_cb`),
the 9 re-stated declarations' digests change (`SSL_CTX_free`,
`SSL_CTX_use_PrivateKey_file`, `SSL_CTX_use_certificate_chain_file`,
`SSL_connect`, `SSL_do_handshake`, `SSL_free`, `SSL_read`, `SSL_shutdown`,
`SSL_write`), nothing else (`pinned`, `library_versions` and the other 48
entries unchanged). `providers/user-crates-load-path.json` regenerated
(`quirin-plugin load-path` over both user crates, `quirin-audit plugin views
--write`): three bindings added. `providers/contracts.py sync` wrote nothing
further.

## 3. Fail-closed arms, each with a test

`crates/qcore-c-gateway/tests/openssl_callback_slots.rs` (9 tests, Clang):

| Arm | Test | Refusal |
|---|---|---|
| a candidate with no contract in force | `a_candidate_with_no_contract_refuses` (on the premise-free `SSL_CTX_use_PrivateKey_file`) | `callback-uncontracted` |
| several registrations, one effectful | `an_effectful_candidate_among_several_registrations_refuses` | `callback-effectful` |
| unknown slot state: a formal or a call's result handed to the setter, an address-taken setter, an unread unit | `an_open_slot_refuses` | `callback-member-open` |
| a slot nothing registers (even a closed empty set) | `a_slot_nothing_registers_refuses` | `no declaration registers the slot` |
| a unit defining the record | `a_slot_whose_record_a_unit_defines_refuses` | `defines struct ssl_ctx_st` |
| no run facts | `a_closed_slot_decides_over_the_declared_setters_actuals` (second half) | `keeps opaque` |
| the member form over an undefined record | `the_member_form_still_never_decides_an_undefined_record` | `not a member the run's records define` |
| malformed slot clauses (a path below the field, a duplicate, `supplied_owned`, arity, the callback as holder) | `the_slot_forms_load_and_reject_their_malformed_spellings` | module rejected |

A row is `Safe` only where every candidate is the identity or the run
registers nothing (`a_closed_slot_decides…`, `an_unregistered_run_set_invokes_nothing`).

**Lean** `QCore.OpaqueCallbackSlots`, audited by
`proof/AuditOpaqueCallbackSlots.lean` (16 theorems, axioms within `propext`
and `Quot.sound`; one with none): the decision's four refusals, the member
form unchanged, `identity_needs_every_candidate_identity`,
`every_declared_writers_value_is_a_candidate`,
`a_refused_slot_never_publishes_safe`,
`a_row_resting_on_a_slot_is_safe_only_if_it_decides`, and the checker's slot
shapes (a registration states its slot and releases nothing; a stated
invocation and the unstated premise are in every accepted declaration).
`QCore.OpensslCalleeShapes` is untouched: its abstract `registeredCallbacks`
premise constructor models the renamed premise (the model carries no name).

## 4. The user path, with the real binaries

[`evidence/user-path.sh`](evidence/user-path.sh) (OPENSSL-PROVIDER-REMAINING-CALLEES'
script on this card's two-unit fixture [`evidence/fixture/`](evidence/fixture/)),
tip binaries ([logs](evidence/run-1/), [summary](evidence/run-1/run.json)),
every step exit 0: 60 bindings, 0 Safe / 60 Pending; `pin` 0 pinned / 60
flexible; `pin --verify-contracts` exit 0. (The script's step 4 lists only
the OpenSSL crate; the committed load path was then regenerated over both
user crates, §2.)

| Fixture function | Status | Why |
|---|---|---|
| `load_key`, `load_chain` | `Pending` | the password slot's run set is `{app_password}`, the identity, in force in this unit |
| `write_some`, `handshake` | `Pending` | the certificate slot's run set is `{app_certificate}`, the identity here |
| `shut` | `Pending` | the premise only |
| `read_some` (`tls_other.c`) | `Unknown` | the run's certificate slot holds `tls_slots.c`'s internal `app_certificate`, no contract in force in this unit: `callback-uncontracted` |
| `configure` | `Unknown` | `SSL_CTX_set_cert_cb(ctx, app_certificate, NULL)`: the retention route reads `(void *) 0` as a retained actual it cannot decide (`retention-undecided`); follow-up `RETENTION-NULL-CONSTANT-ACTUAL` |

## 5. Measurement

**Pinned routes: no move, by construction; arbiter deferred.** Corpus, system
and slice take no user load path, and the gateway change decides only the two
slot keywords, which no shipped module states (`providers/contracts.py
check` keeps the OpenSSL crate off the shipped load path). This host has no
prepared pinned-route inputs (no `build/cycle19`), so the prediction's
`measure-prediction` block (none on corpus, system, slice) is for the
integrator's `quirin-audit measure --declared` against `rev:974434463`.

**The opt-in `current` run (this card's arbiter).** `quirin-audit current
--monkey-linked` over Fluent Bit 046accf6 exactly as OPENSSL-PROVIDER-REMAINING-CALLEES
ran it ([script](evidence/opt-in/opt-in-current.sh); the user load path's
selection, `SOURCE_DATE_EPOCH=1767225600`, `--jobs 12`), inputs prepared on
this host by the base binary's `prepare` into `private/fb-046accf6`. Two runs,
each a `git archive` side tree with its checker installed by its own
`package.py checkers` (exit 0 both): base (base binary and tree
`974434463`) and tip (tip binary, tree `4b3174f75`; later commits change only
a comment and the report). A base binary cannot load the tip modules (it
rejects the slot keywords); the tip binary changes nothing for modules
without them, so no third run.

| Run | Files / configurations | Functions Safe / Pending / Unknown / Unsafe | Bindings | Wall |
|---|---|---|---|---|
| base | 180 / 181: 9 Safe, 0 Pending, 5 Unsafe, 163 Unknown, 4 data-only | 1098 / 2 / 3699 / 5 | 63, all `Pending` | 45 min |
| tip | same | 1098 / 2 / 3699 / 5 | 66, all `Pending` | 45 min |

Both exit 0, no unmatched row ([summaries](evidence/opt-in/), invocations,
[comparison](evidence/opt-in/comparison-base-tip.json) by
OPENSSL-PROVIDER-REMAINING-CALLEES' `evidence/compare.py`). **No row moved,
as predicted**; no row to `Safe`, no new `Unsafe`. Both slots' run sets are
closed and as predicted: `ssl_ctx_st.cert_cb` = {`rd_kafka_ssl_cert_callback`},
`ssl_ctx_st.default_passwd_cb` = {`rd_kafka_transport_ssl_passwd_cb`}
(`rdkafka_ssl.c:1714, 1946`; sidecar `callback_members`). Eleven rows rest on
a slot (`run_set_members`), all `Unknown`; 10 rows change content:

* `rd_kafka_ssl_ctx_init`, `rd_kafka_ssl_set_certs`: the setters' uncontracted
  diagnostics leave and their links arrive; `set_certs` gains the predicted
  retention refusal (`SSL_CTX_set_cert_cb` retains `rk` in `ctx`).
* Monkey `mk_tls_send_file`, `mk_tls_writev`: `SSL_write` now refuses
  (`callback rd_kafka_ssl_cert_callback (from ssl_ctx_st.cert_cb) has no
  contract in force`, `callback-uncontracted`): the run-wide set reaches a
  unit where the candidate is not in force, as predicted.
* librdkafka's TLS I/O (`rd_kafka_transport_ssl_connect`, `_handshake`,
  `_io_event`, `_recv`, `_send`; they live in `rdkafka_ssl.c`, not
  `rdkafka_transport.c` as the prediction guessed): the slot declarations go
  through the callback route, which reads the object operand
  `rktrans->rktrans_ssl` and refuses it as an indirect access
  (`indirect memory access is outside the resource projection`) where the
  plain declarations refused `unresolved resource identity rktrans` at the
  same call. A refusal at the same line either way.
* `tls_load_credentials` (Monkey): content only (no diagnostic added or lost).

Where the prediction was wrong in detail: Fluent Bit's own TLS rows did not
change (none rests on a slot), and the rdkafka I/O rows refuse on the object
operand before any candidate is classified rather than on the candidate.

## 6. Gate

`python3 scripts/gate_scope.py --base 974434463 --out private/gate-1`
through `private/toolchain/env.sh` (Clang 21.1.8, Lean 4.35.0) on
`5fc6a787d` (the report and amendments uncommitted, docs only); scoped, the
provider gate included (record [`gate/gate-1.json`](gate/gate-1.json), logs
`private/gate-1/`). `gate_scope: 14 steps passed`.

| Step | Exit | Counts |
|---|---:|---|
| submodules, `fmt`, `clippy` | 0, 0, 0 | 1 submodule |
| Rust suite (scoped, 24 packages) | 0 | 656 units, 701 summaries; 9650 passed, 0 failed, 21 ignored, 148 filtered |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| Lean (`check_lean_audits.py`) | 0 | 446 audits (`AuditOpaqueCallbackSlots.lean` among them) |
| Python suite | 0 | 779 tests |
| provider package build, verify-all, verify `qcore.libevent` | 0, 0, 0 | |
| provider tests | 0 | 49 summaries; 153 passed, 0 failed, 1 ignored |
| provider fmt, provider clippy | 0, 0 | |

Earlier, outside the gate (on `5fc6a787d`'s tree, while the opt-in runs
were in flight): a plain `python3 -m unittest discover -s scripts/tests` run
failed one test,
`test_the_derived_facts_are_recomputed_rather_than_carried` (the committed
matrix's `lean_audits` 445 against 446 produced: the new audit module), log
`private/python-pre.log`; the gate's Python step, later on the same tree,
passed (779), and the discrepancy is not investigated further: a stale
matrix check is the integrator's (the brief's known noise), who regenerates
the matrix with `lean_audits` 446.
`providers/package.py checkers` exit 0 (installs both checkers, `plugin
views`, `pin --verify-contracts`), `private/checkers-2.log`.

## 7. Follow-ups

[`integration-amendments.json`](integration-amendments.json) retires
`OPENSSL-SSL-CTX-CALLBACK-SLOTS` (vocabulary, the two setters and the
SSL twin declared, the premise replaced: removed from one declaration,
narrowed on five, made explicit on three) and registers:

* `OPENSSL-UNSTATED-CALLBACK-SLOTS`: the slots `openssl_unstated_callback_slots`
  still covers (verify, info, message, security, session, client
  certificate, ex_data), and `SSL_CTX_use_certificate`, whose security
  callback (SSL_CTX_set_security_level, 3.5, names it) its declaration reads
  silently today.
* `RETENTION-NULL-CONSTANT-ACTUAL`: a null pointer constant handed to a
  retaining position retains nothing, but the retention route refuses it.

Files likely to conflict (OPENSSL-PEM-CALLBACK-ARM edits the same crate):
`providers/quirin-provider-openssl/src/documented.rs` (the `Premise` enum and
the constants beside it), `src/check.rs` (`expected_clauses`, the imports, the
tests module), `contracts.lock` and `providers/user-crates-load-path.json`
(regenerate: `quirin-audit pin`, `quirin-plugin load-path` over
`quirin-provider-libcurl` then `quirin-provider-openssl`, `quirin-audit
plugin views --write`), `crates/qcore-contract/src/contract_callback.rs`,
`proof/QCore.lean` (rerun `scripts/gen_lean_roots.py`), `docs/agents/gateway.md`
and `docs/agents/README.md` (rerun `scripts/gen_rule_docs.py`).
