# SP1-SYMBOLIC-OPENSSL-PREMISES

Base `3c0f8e474` (integrate/ag-gaps). User decision 2026-11-20 (verbatim):
"we don't want to write down the sentences on premise clauses. We really
dislike this kind of results so we must write down as condition symbols like
|-> or other types of logical symbols to represent the prerequisites which
compose for premise conditions. This must be applied for OpenSSL's premises."
Same day: ASCII tokens reusing the language's existing tokens; convert every
OpenSSL premise, then the OpenSSL provider refuses a prose statement; other
providers' prose premises become registered follow-ups.

[Prediction](PREDICTION.md) (committed first, `431131947`): no status move on
any route, content changes only where a premise is rendered, 30 lock digests.
Its plan sentence spells difference `-`. The user later chose `\`, which
changes no predicted effect, so the prediction is kept as committed.

## 1. What changed

* **The statement form** (contract specification C04.18.9, grammar
  G05-formula). `requires resource_premise(NAME, FORMULA);` beside the string
  form, read by one reader, `qcore_frontend::premise_formula`, which the
  contract format (`frontend.rs`), the loader (`contract_loader::premise_clause`)
  and the S3 prerequisite reader (`local_condition::named_prerequisite`) share.
  The reader is syntax then sorts, both total-or-reject, bounded at 1024
  tokens, depth 48 and 512 nodes.
* **The statement** of a formula premise is its canonical printing, so the
  premise's identity is the name plus that text and every rendering prints
  it. A string statement that is itself a formula is refused, so the two
  forms never share a statement.
* **Semantics unchanged.** A formula premise is metadata exactly as a string
  one: no consumer that decides a verdict reads it.
* **OpenSSL.** All 53 `resource_premise` clauses of
  `quirin-provider-openssl` state formulas (`documented.rs`'s four
  `*_STATEMENT` constants, the 30 declarations in 13 modules). The checker
  refuses a premise whose statement is a string (`check::prose_premise`,
  before the documented-clause comparison).
* **Rule 13h** amended; no new rule id (64 stays free).

## 2. Grammar and vocabulary

Loosest to tightest: `->` (right-associative), `&&`, `|->` (no chaining),
`in` and `==` (no chaining), `|` and `\` (a union chain or one difference,
never mixed; `-` refused), then `( F )`, `quant all #x in SET: F` (the body extends as far
as it can), `#name`, `#name(A)`, `#name(A, B)`, an upper-case constant, `{}`.
Every token but `\` is an existing one.

| Term | Sort | Meaning |
|---|---|---|
| `#args` | set(obj) | the objects the call's pointer arguments designate |
| `#created` | set(obj) | the objects the call allocates, directly or as a temporary inside the call |
| `#copied` | set(obj) | the objects the call copies (the source of a dup) |
| `#freed` | set(obj) | the objects whose last reference the call drops: directly, in turn (an object a freed object owns), or as a replaced or temporary object it frees inside the call |
| `#class(o)` | obj → cls | the object's class |
| `#of(k, S)` | cls, set(obj) → set(obj) | the members of `S` of class `k` |
| `#ctx(S)` | set(obj) → set(obj) | the SSL_CTX of each SSL object of `S` |
| `#bios(S)` | set(obj) → set(obj) | the BIOs each SSL object of `S` holds |
| `#chain(S)` | set(obj) → set(obj) | the BIOs of `S` and every BIO chained after one, transitively |
| `#ssl_of(b)` | obj → set(obj) | the SSL object a BIO_f_ssl BIO holds |
| `#slots(S)` | set(obj) → set(fn) | every callback slot of an object of `S`, the `SSL_CTX_callback_ctrl` arms included |
| `#stated` | set(fn) | the callbacks a clause of the declaration states the call runs |
| `#ex_data(k)` | cls → set(fn) | the new, dup and free callbacks `CRYPTO_get_ex_new_index` registers for `k` |
| `#ex_data_free(k)` | cls → set(fn) | the free callbacks registered for `k` |
| `#method(b)` | obj → meth | the method table of a BIO (its BIO_METHOD) or of an X509_LOOKUP (its X509_LOOKUP_METHOD) |
| `#builtin` | set(meth) | the method tables the library defines itself, which no setter is handed: the BIO_METHOD `BIO_s_socket`, `BIO_s_mem`, `BIO_f_buffer` or another built-in `BIO_s_` or `BIO_f_` function returns, and the X509_LOOKUP_METHOD `X509_LOOKUP_file`, `X509_LOOKUP_hash_dir` or `X509_LOOKUP_store` returns |
| `#fns(b)` | obj → set(fn) | the functions of the object's method table (BIO_METHOD, X509_LOOKUP_METHOD), each as run on it |
| `#store(S)` | set(obj) → set(obj) | the X509_STORE each SSL_CTX of `S` holds |
| `#lookups(S)` | set(obj) → set(obj) | the X509_LOOKUPs each X509_STORE of `S` holds |
| `#teardown(o)` | obj → set(fn) | the callbacks the free of `o` runs from its method tables, its provider and its engine, and the callbacks `o` holds that its free runs |
| `#settable` | set(fn) | the callbacks a setter some declaration of the declaration's crate states can write |
| `#in_turn(f, S)` | fn, set(obj) → set(fn) | the callbacks a run of `f` runs in turn on an object of `S` |
| `#caller_held` | set(res) | the resources the caller holds when the run starts |
| `#takes(R)`, `#releases(R)`, `#retains(R)` | set(res) → set(res) | the members of `R` the run takes over, releases, keeps |
| `#inert` | eff | `#takes(#caller_held) == {} && #releases(#caller_held) == {} && #retains(#caller_held) == {}` |

Constants: the fifteen ex_data classes (sort cls).
`F |-> E`: every run of callback `F` (of every member of set `F`) during the
call has effect `E`.

**`|->` in the tokenizer.** The lexer has one `MapsTo` token: three bytes,
matched before every two-byte spelling, so `A |-> B` never lexes as a union
followed by an implication, and `|>`, `||`, `||->`, `->|` stay their own
tokens (each refused inside a formula). Its meaning is the delimited
context's: a state clause, a `resource_premise_flow_v1` expression, or this
formula, which is exactly the tokens between the comma after `NAME` and the
clause's closing parenthesis. The two other meanings are unchanged (their
readers and tests are untouched).

**Set difference is `\`** (USER DECISION 2026-11-20, "Add '\'", answering
this card's first hand-back, which had used the existing `-`).
* **The token.** `Symbol::Backslash`, one byte, no two-byte spelling. A bare
  backslash was an `unexpected character` lex error. The census
  (`private/logs`, script in the scratchpad, reproduced by the lexer's own
  refusal) found zero backslashes in code position across the 781 tracked
  `.qcore`/`.qc`/`.qr`/`.qv`/`.qcorelib` files at base, so the promotion
  accepts nothing new outside a formula: every other reader refuses the token
  as an unexpected symbol. Inside a string or character literal a backslash is
  still the literal's escape (`quoted` never reaches the symbol rule).
  Against the existing spellings: `\|` is `Backslash` then `Pipe`; `\|->` is
  `Backslash` then one `MapsTo`; `|->`, `||->`, `->|`, `!->`, `|>`, `||` keep
  their longest-match identities (`lexes_the_set_difference_apart_from_every_bar_spelling`).
* **One spelling.** Inside a formula `-` is refused ("set difference is spelled
  `\`, not `-`"), with tests in the reader and the loader.
* **The hypothesis graph.** A formula premise erases to a legacy hypothesis
  atom whose statement is the canonical printing, now possibly holding `\`.
  `hypothesis::validate` admitted an opaque statement only when `plain` (no
  backslash). It now also admits one when the atom ID starts `legacy_atom_`
  (generated from a named premise) **and** the statement is exactly the
  canonical printing of a well-formed formula (`canonical_formula`: the
  premise reader accepts it and re-prints it byte for byte). Every other
  opaque statement keeps `plain`: a `resource_hypothesis` atom, a string
  premise's legacy atom, a non-canonical or ill-formed formula, or any other
  text with a backslash is refused as before. It stays fail-closed because the
  added case is a strict, decidable check of the same reader that admitted the
  premise; the wire reader goes through the same `validate`.

## 3. The four formulas

* `openssl_ex_data_callbacks` (30):
  `quant all #o in #created | #copied | #freed: #ex_data(#class(#o)) |-> #inert`
* `openssl_unstated_callback_slots`, handshake statement (5):
  `#slots(#of(SSL, #args) | #ctx(#of(SSL, #args)) | #of(SSL_CTX, #args)) \ #stated |-> #inert && (quant all #l in #lookups(#store(#ctx(#of(SSL, #args)) | #of(SSL_CTX, #args))): #fns(#l) \ #stated |-> #inert) && quant all #o in #created | #copied | #freed: #ex_data(#class(#o)) \ #stated |-> #inert`
* `openssl_builtin_bio_methods` (10):
  `quant all #b in #chain(#of(BIO, #args) | #bios(#of(SSL, #args))): #method(#b) in #builtin -> #fns(#b) |-> #inert && quant all #f in #fns(#b): #in_turn(#f, #ssl_of(#b) \ #args) |-> #inert`
* `openssl_bio_free_callbacks` (2):
  `quant all #b in #of(BIO, #args) | #chain(#bios(#of(SSL, #args))): #b in #freed -> #ex_data_free(BIO) |-> #inert`
* `openssl_unstated_callback_slots`, statement of a free (3: `SSL_free`, `SSL_CTX_free`, `X509_STORE_free`):
  `quant all #o in #freed: #teardown(#o) \ (#stated | #settable) |-> #inert`
* `openssl_builtin_lookup_methods` (3: the same frees):
  `quant all #l in #lookups(#of(X509_STORE, #freed)): #method(#l) in #builtin -> #fns(#l) |-> #inert`

The modules lay each out one step per line (a break before every `->` and
`&&`); the checker and the loader read whitespace-insensitively.

## 4. Faithfulness: prose to formula

| Premise | Prose | Formula |
|---|---|---|
| ex_data | "an ex_data callback" | `#ex_data(…)` |
| | "the call may run" | `\|->`: only runs during the call count; a callback not run satisfies it |
| | "a new, dup or free callback CRYPTO_get_ex_new_index registers" | `#ex_data(k)`: the new, dup and free callbacks registered for `k` |
| | "for the class of an object" | `#class(#o)` |
| | "the call creates, copies or frees" | `quant all #o in #created \| #copied \| #freed` |
| | "takes, releases or retains nothing the caller holds" | `#inert` |
| unstated | "a callback the call may run" | `\|->` |
| | "that no clause of this declaration states" | `\ #stated`, on both conjuncts |
| | "a callback slot of" | `#slots(…)` |
| | "the SSL object" | `#of(SSL, #args)` |
| | "or its SSL_CTX" | `#ctx(#of(SSL, #args))`; on `SSL_CTX_free`, the SSL_CTX argument, `#of(SSL_CTX, #args)` (R3) |
| | "whose invocation the declaration does not state" | `\ #stated` |
| | "among them the SSL_CTX_callback_ctrl arms such as the servername, status and ticket key callbacks" | members of `#slots` (its meaning names the arms); "among them" adds no set |
| | "the certificate transparency validation callback, and the not-resumable session and session ticket extension callbacks, whose setters no manual page documents" (r38) | members of `#slots` too, under the same "among them": `#slots`' meaning names them; no set is added |
| | "a function of an X509_LOOKUP_METHOD of a lookup the certificate store of the SSL_CTX holds" (r38) | `&& (quant all #l in #lookups(#store(#ctx(#of(SSL, #args)) \| #of(SSL_CTX, #args))): #fns(#l) \ #stated \|-> #inert)`: the SSL_CTX as above (the SSL object's or the SSL_CTX argument), its store, the store's lookups, each lookup method's functions as run on that lookup, less what a clause states |
| | "or an ex_data callback" | `&& quant all #o in #created \| #copied \| #freed: #ex_data(#class(#o)) \ #stated \|-> #inert` (R2) |
| | "takes, releases or retains nothing the caller holds" | `\|-> #inert`, each conjunct |
| builtin | "a function of a BIO method" | `#fns(#b)` |
| | "the library defines itself (the BIO_METHOD that BIO_s_socket, BIO_s_mem, BIO_f_buffer or another built-in BIO_s_ or BIO_f_ function returns, which no setter is handed)" | `#method(#b) in #builtin ->`; `#builtin`'s meaning carries the parenthetical |
| | "run on a BIO" | `#fns(#b)`: each function as run on `#b` |
| | "the call reads or writes through or frees (its BIO argument or a BIO the SSL object holds)" | `#of(BIO, #args) \| #bios(#of(SSL, #args))` |
| | "or one chained to it" | `#chain(…)` |
| | "or frees" (r36, also on `BIO_free` and `SSL_free`) | no new term: the parenthetical names the same BIOs (its BIO argument, the SSL object's BIOs) whether the call reads, writes or frees through them, so the domain `#chain(#of(BIO, #args) \| #bios(#of(SSL, #args)))` is unchanged; a built-in method's destroy function run at the free is among `#fns(#b)` |
| | "takes, releases or retains nothing the caller holds" | `#fns(#b) \|-> #inert` |
| | "and so does any callback it runs in turn" | `&& quant all #f in #fns(#b): #in_turn(#f, …) \|-> #inert` |
| | "on an object other than the arguments of the call" | `… \ #args` |
| | "(the SSL object of a BIO_f_ssl BIO)" | `#ssl_of(#b)` (R4) |
| BIO free | "when the call drops the last reference to a BIO" | `quant all #b in …: #b in #freed ->` |
| | "its BIO argument" | `#of(BIO, #args)` |
| | "or a BIO the SSL object holds and the BIOs chained to it" | `\| #chain(#bios(#of(SSL, #args)))` (R5) |
| | "the ex_data free callbacks of the BIO class" (r36: the callback and destroy function are stated slots now, not premised) | `#ex_data_free(BIO)` |
| | "take, release or retain nothing the caller holds" | `\|-> #inert` |
| unstated (free, r39) | "a callback the call may run" | `\|->` |
| | "that no clause of this declaration states" | `\ (#stated \| …)` |
| | "a function of a method, provider or engine of an object" | `#teardown(#o)` (its method tables, provider and engine) |
| | "the call frees in turn" | `quant all #o in #freed`: `#freed` counts objects freed in turn; it also counts the argument, because the listed QUIC TLS callbacks "of the SSL object" run at `SSL_free` |
| | "whose setter no declaration of this crate states or which no setter is handed" | `\ (… \| #settable)`: less what a declared setter can write |
| | "among them" | an open list: each item below is a member, and the formula states the class |
| | "the free function of the key management of the provider of a key" | in `#teardown(key)` (provider) |
| | "the free function of the EVP_PKEY_ASN1_METHOD of a key" | in `#teardown(key)` (method table) |
| | "the finish function of the RSA_METHOD, DSA_METHOD, DH_METHOD or EC_KEY_METHOD of a key" | in `#teardown` of the key, or of its RSA, DSA, DH or EC_KEY object freed in turn (method table) |
| | "the finish and destroy functions of the ENGINE of a key or of the client certificate engine of the SSL_CTX" | in `#teardown(key)` and `#teardown(ctx)` (engine) |
| | "the free function of the X509_CRL_METHOD of a CRL a certificate store holds" | in `#teardown(crl)`, the CRL freed in turn (method table) |
| | "a cleanup function of the async wait context of the SSL object" | in `#teardown` of the wait context freed in turn (a callback it holds) |
| | "the QUIC TLS callbacks of the SSL object" | in `#teardown(ssl)` (callbacks it holds) |
| | "and any callback the free of a QUIC object runs in turn" | in `#teardown` of each QUIC object the call frees, and of what it frees in turn |
| | "takes, releases or retains nothing the caller holds" | `\|-> #inert` |
| builtin lookup (r39) | "a function of an X509_LOOKUP_METHOD" | `#fns(#l)` |
| | "the library defines itself (the method that X509_LOOKUP_file, X509_LOOKUP_hash_dir or X509_LOOKUP_store returns, which no setter is handed)" | `#method(#l) in #builtin ->`; `#builtin` now also holds these three methods (a lookup's method is never a BIO_METHOD, so the BIO premise reads `#builtin` as before) |
| | "run on a lookup of a certificate store the call frees" | `quant all #l in #lookups(#of(X509_STORE, #freed))`, each function as run on `#l` |
| | "takes, releases or retains nothing the caller holds" | `\|-> #inert` |

**Readings recorded** where a phrase admitted more than one parse:

* **R1** (ex_data). "a new, dup or free callback … for the class of an object
  the call creates, copies or frees" is read as written: any of the three
  kinds, for an object in any of the three relations. The pairwise reading
  (new with creates, dup with copies, free with frees) is narrower in the
  abstract model. Under `CRYPTO_get_ex_new_index`'s documented run discipline
  the two agree, since `#created` and `#freed` include temporaries.
* **R2** (unstated). "an ex_data callback" is read as the ex_data premise
  defines it, by the objects the call creates, copies or frees. With it the
  unstated premise entails the ex_data premise (§6).
* **R3** (unstated on `SSL_CTX_free`, which has no SSL object). "the SSL object
  or its SSL_CTX" is the SSL_CTX argument there. C04.18 records that the premise
  stands on `SSL_CTX_free` for the session-remove callback, a slot of the
  SSL_CTX. On every other declaration, `#of(SSL_CTX, #args)` is empty.
* **R4** (builtin). "(the SSL object of a BIO_f_ssl BIO)" is read as defining
  "an object other than the arguments", as every other parenthetical in these
  statements defines. Examples are marked "such as", "among them" and "or
  another". The wider reading, any callback on any object that is not an
  argument, would also assume something of, for example, a chained
  `BIO_s_connect` BIO's info callback (it would need a new term).
  **User decision 2026-11-20: "Narrow (Recommended)"**, the reading above.
* **R5** (BIO free). "and the BIOs chained to it" attaches to the SSL
  object's BIOs only: `BIO_free` frees one BIO (`BIO_free_all` frees a chain).
  The `#b in #freed` guard makes the other grouping assume nothing more of a
  BIO the call does not free.

## 5. Fail-closed arms, each with a test

`crates/qcore-frontend/tests/premise_formula.rs` (7 tests) and
`crates/qcore-contract/tests/contract_premise.rs` (2 new):

* an unknown term or constant, an ill-sorted application, a term without its
  arguments, an unbound variable, a variable shadowing a term or a variable, a
  non-proposition, and a quoted formula each reject the module through the
  loader (`every_malformed_formula_rejects_the_module`) and the reader
  (`a_premise_clause_is_prose_or_a_formula`);
* 28 malformed token sequences, the depth and token bounds
  (`malformed_syntax_is_refused`);
* the canonical printing reads back as the same tree, with parentheses exactly
  where precedence needs them (`the_printing_parenthesizes_exactly_where_needed`);
* a formula premise changes no contract, and two spellings of one tree are one
  premise (`a_formula_premise_is_admitted_with_its_canonical_statement`);
* the OpenSSL checker refuses each of the 53 clauses rewritten as prose
  (`check::tests::a_prose_premise_is_refused`); its drift tests
  (`a_drifted_bio_slot_reading_is_refused`, the unstated-slot and ex_data
  drifts) now drift the formulas.

## 6. Lean

`QCore.PremiseFormula`, audited by `proof/AuditPremiseFormula.lean` (12
theorems). It has the syntax (`Raw`), sorts (`Srt`), the vocabulary (`vocab`,
`classes`), the judgment (`check`, `wf`), and a denotation
(`eval`) over an abstract `Model` of the call's objects, callback sets, runs
and per-run resource effects.

* `check_denotes`, `wf_denotes`, `quant_body_denotes`: a well-sorted term
  denotes a value of its sort, and a well-formed formula a proposition.
* `decision_vectors_agree`: the judgment decides the 22 shared vectors as
  the Rust reader does. The Rust test `the_decision_vectors_are_the_lean_table`
  binds each row's tree and decision. `the_vocabulary_is_the_lean_table` binds
  the vocabulary and classes.
* `openssl_formulas_wf`: the four formulas are well-formed. The Rust test
  `the_openssl_formulas_are_the_lean_definitions` binds the Lean definitions
  to every one of the 53 clauses as read from the modules.
* `inert_definition`: `#inert` denotes its definition.
* `unstated_entails_ex_data`: in every model where no clause states an
  ex_data callback (`Model.Coherent`), `openssl_unstated_callback_slots`
  entails `openssl_ex_data_callbacks`. `unstated_entails_ex_data_for_ssl`
  specializes it to the SSL object's ex_data class.
  `ex_data_does_not_entail_unstated` shows the converse fails.

Placeholder-free, no project axioms (gate log below).

## 7. Lock diff, reviewed

Order: `package.py checkers` (exit 1, the expected stale lock;
`private/logs/regen-checkers-1.log`), `quirin-audit pin --source-root
docs/reports/openssl-provider-remaining-callees/evidence/fixture --load-path
providers/user-crates-load-path.json --crate quirin-provider-openssl` (exit 0,
`regen-pin-2.log`), `contracts.py sync` (exit 0), `package.py checkers`
(exit 0, `regen-checkers-2.log`), `plugin views --write` ("every user crate's
resource view is current").

`contracts.lock` changes exactly the 30 `contract_sha256` and `pinned_at`
values of the declarations naming a premise. The other 77 entries and every
top-level key are unchanged, and no entry is added or removed.
`providers/user-crates-load-path.json`, `providers/contract-load-path.json`
and `quirin-plugin.json` are byte-identical to base. The prediction expected
changed module digests in `user-crates-load-path.json`, but that file lists
modules and bindings, not digests, so that line of the prediction missed. The
lock count matched.

**Re-pin after the switch to `\`**, in the same order (`regen-checkers-3.log`
exit 1 listing exactly the stale declarations; `regen-pin-3.log`,
`regen-sync-3.log`, `regen-checkers-4.log`, `regen-views-3.log`, each exit 0):
against the previous tip, exactly the 10 declarations whose formulas hold a set
difference change `contract_sha256` and `pinned_at`. Those are the seven
`openssl_unstated_callback_slots` declarations (`SSL_connect`,
`SSL_do_handshake`, `SSL_read`, `SSL_write`, `SSL_shutdown`, `SSL_free`,
`SSL_CTX_free`) and three more naming `openssl_builtin_bio_methods`
(`PEM_read_bio_X509`, `PEM_read_bio_PrivateKey`, `d2i_PKCS12_bio`). The other
20 premised declarations print no difference and keep their digests. Against
base, the lock still changes exactly the 30 premised declarations, adds or
removes nothing, and leaves every top-level key and both load paths unchanged.

**After merging main `7c026b527`** (r36, OPENSSL-FREE-SESSION-REMOVE-INVOCATION),
the lock was regenerated, not merged, in the same order (`regen-checkers-5.log`
exit 1 listing the 30 stale declarations; then `regen-pin-5.log`,
`regen-sync-5.log`, `regen-checkers-6.log` and `regen-views-5.log`, each exit 0).
Against `7c026b527`, `contracts.lock` changes exactly the 30 premised
declarations' `contract_sha256` and `pinned_at`. The other 77 entries and every
top-level key are unchanged, nothing is added or removed, and both load paths
and `quirin-plugin.json` are byte-identical to main.

**After merging main `ffb41c748`** (r38, OPENSSL-REMAINING-HANDSHAKE-SETTERS)
the lock was regenerated the same way (`regen-checkers-7.log` exit 1 with 30
stale entries; `regen-pin-7.log`, `regen-sync-7.log`, `regen-checkers-8.log`,
`regen-views-7.log` each exit 0). Against `ffb41c748`, `contracts.lock` changes
exactly the 30 premised declarations' `contract_sha256` and `pinned_at` (the
changed set equals the set of declarations naming a premise). The other 105 of
its 135 entries and every top-level key are unchanged, and nothing is added or
removed. Both load paths and `quirin-plugin.json` are byte-identical to main.
The crate has 49 premise clauses on those 30 declarations.

**After merging main `b7f86a0de`** (r39, OPENSSL-FREE-INDIRECT-CALLBACKS) the
lock was regenerated the same way:
* `regen-checkers-9.log` exit 1, with 30 stale entries;
* `regen-pin-9.log`, `regen-sync-9.log`, `regen-checkers-10.log` and
  `regen-views-9.log`, each exit 0.

Against `b7f86a0de`, `contracts.lock` changes exactly the 30 premised
declarations' `contract_sha256` and `pinned_at`. The other 114 of its 144
entries and every top-level key are unchanged, and nothing is added or removed.
Both load paths and `quirin-plugin.json` are byte-identical to main. The crate
has 53 premise clauses on those 30 declarations.

On the integrator's instruction, the card's own full gate was not rerun after
this merge. Fast checks only, each exit 0:
* the OpenSSL provider tests: 21 and 2 passed;
* `qcore-frontend` lib and `premise_formula`: 61 and 7 passed;
* `qcore-contract` `contract_premise` and `hypothesis`: 7 and 15 passed;
* `lake build QCore.PremiseFormula` with `AuditPremiseFormula.lean`: 13 theorems, standard axioms only;
* `contracts.py check` and `package.py checkers`;
* `apply_amendments --check`.

The integrator's landing gate and measurement cover the rest, the fixture user
path included.

## 8. Measurement

**Pinned routes.** None loads the OpenSSL user crate, so no move is predicted
on corpus, twelve-home, system or slice. The `measure-prediction` block is for
the integrator's `quirin-audit measure --declared`.

**The user path** on the predecessor's fixture: `quirin-audit contracts
--load-path providers/user-crates-load-path.json --provider
quirin-provider-libc --provider quirin-provider-openssl --source-root
docs/reports/openssl-provider-remaining-callees/evidence/fixture`, run with the
tip's debug binary through an `audit` slot (`private/logs/userpath-tip-2.log`,
exit 0; a first attempt hit its own wall timeout while queued for a slot and
never ran, `userpath-tip.log`). All 12 fixture functions have the status
OPENSSL-EX-DATA-REGISTRY recorded for its tip, so none moved. `read_cert`,
`pkcs12_cycle` and seven others are `Pending`; `read_cert_reuse`,
`keep_userdata` and `verify_depth` are `Unknown`. All 113 bindings are
`Pending` ([comparison](evidence/user-path/fixture-r34-tip.txt)). The
report and its contract sidecar print the canonical formula
(`quant all #o in #created | …`) and no longer the English sentence: the
predicted content change. This comparison is against the predecessor's
recorded tip, not a base binary rebuilt at `3c0f8e474`.

## 9. Gate

`python3 scripts/gate_scope.py --base 3c0f8e474 --out private/gate-1` on
`ca39093c0` ([record](gate/gate-1.json); mode full, because the diff reaches
`qcore-contract`). The gate exited 1:

| Step | Exit | Counts |
|---|---|---|
| fmt, clippy | 0, 0 | |
| rust-suite | 101 | 667 units, 9770 passed, **1 failed**, 21 ignored |
| reevaluation probe, follow-up | 0, 0 | |
| lean (`check_lean_audits.py`) | 0 | 459 audits |
| python | 1 | 779 tests, **2 failed** |
| provider package build, verify (all, libevent) | 0, 0, 0 | |
| provider test | 0 | 161 passed, 1 ignored |
| provider fmt, clippy | 0, 0 | |

* **Rust.** The one failure is the known host failure the brief records,
  `qcore-audit::test::libcurl_user_crate_versions::the_libcurl_ranges_follow_the_audited_curlver_header`
  (the system `curlver.h`). It is not fixed here.
* **Python.** The two failures were this card's:
  * `test_tracked_files::…every_remaining_tree_walk_in_the_tests_is_a_reviewed_one`
    refused the formula test's `read_dir` of the OpenSSL contract directory
    (rule 29's tripwire);
  * `test_gate_scope_census::…every_walker_entry_is_walked` refused the walker
    entry declaring it.

  Fixed in `35fd35136`: the 17 modules are `include_str!`ed and the walker
  entry is gone. Targeted reruns on that tip are in `private/gate-2/`:
  * `cargo test -p qcore-frontend --test premise_formula`: 7 passed, exit 0;
  * `cargo clippy -p qcore-frontend --all-targets -- -D warnings`: exit 0;
  * `cargo fmt --all -- --check`: exit 0;
  * the full Python suite: 779 tests, exit 1. Both earlier failures pass.
    `test_gate_slot_queue`'s stale-waiter test timed out under load and passed
    alone (39 tests, exit 0). `test_build_matrix_sections`'s derived-facts test
    fails only on the matrix's Lean-audit count, 458 committed against 459
    with this card's audit: the expected stale count, which the integrator's
    matrix rebuild carries.

**Gate 3, the gate of record** (after the user's decisions: `\` and the
narrow R4 reading). `python3 scripts/gate_scope.py --base 3c0f8e474 --out
private/gate-3` on `a0d9ce838` ([record](gate/gate-3.json), mode full) exited
1, from the known host failure only:

| Step | Exit | Counts |
|---|---|---|
| fmt, clippy | 0, 0 | |
| rust-suite | 101 | 667 units, 9771 passed, 1 failed (`libcurl_user_crate_versions`, the recorded host failure), 21 ignored |
| reevaluation probe, follow-up | 0, 0 | |
| lean | 0 | 459 audits |
| python | 0 | 779 tests |
| provider package build, verify (all, libevent) | 0, 0, 0 | |
| provider test | 0 | 161 passed, 1 ignored |
| provider fmt, clippy | 0, 0 | |

The user path (§8) ran on the `-` spelling. The switch changes only the text
of the ten statements holding a difference, and premises are metadata, so it
was not rerun.

**Gate 4, the gate of record after merging main `7c026b527`.**
`python3 scripts/gate_scope.py --base 7c026b527 --out private/gate-4` on
`31ef7f1df` ([record](gate/gate-4.json), mode full). It exited 1, and the only
failure is the known host failure:

| Step | Exit | Counts |
|---|---|---|
| fmt, clippy | 0, 0 | |
| rust-suite | 101 | 667 units, 9779 passed, 1 failed (`libcurl_user_crate_versions`, the recorded host failure), 21 ignored |
| reevaluation probe, follow-up | 0, 0 | |
| lean | 0 | 461 audits |
| python | 0 | 779 tests |
| provider package build, verify (all, libevent) | 0, 0, 0 | |
| provider test | 0 | 162 passed, 1 ignored |
| provider fmt, clippy | 0, 0 | |

**The user path after the merge** (`private/logs/userpath-tip-3.log`, exit 0):

* All 12 fixture functions have the status that r36 recorded for its tip
  ([comparison](evidence/user-path/fixture-r36-tip.txt)), so none moved.
* All 113 bindings are `Pending`.
* The canonical formulas are printed and no English premise text remains.

## 10. Follow-ups

These are registered in [`integration-amendments.json`](integration-amendments.json).
Prose `resource_premise` clauses remain only outside OpenSSL. Counted per clause,
comments excluded:

* **SP2-SYMBOLIC-LIBRDKAFKA-PREMISES**: `qcore-provider-librdkafka` has 220
  clauses under 12 names in 9 files: `client.qv` 119, `null_deref.qv` 71,
  `header_inlines.qv` 12, `log_caller_frame.qv` 6, `cjson.qv` 4, `logging.qv`
  3, `memory.qv` 3, `kafka_mem.qv` 1 and `queue_destructor.qv` 1. `cjson.qv`
  also has 9 prose `resource_hypothesis` atoms.
* **SP3-SYMBOLIC-FLUENT-BIT-PREMISES**: `quirin-provider-fluent-bit` has 3
  clauses (`input_chunk_name.qv`, `loop_owner.qv`, `output_singleplex.qv`).
* **SP4-SYMBOLIC-LIBEVENT-PREMISES**: `qcore-provider-libevent` has 7 clauses
  (`monkey/intrusive_list.qv`).
* `quirin-provider-libc` has no prose `resource_premise` clause. Its two
  `nested_call_admission` premises are `resource_premise_all` conjunctions, so
  no libc follow-up is registered. The brief counted one.

Each will need terms beyond this vocabulary. Adding one is a specification row
(C04.18.9), a Rust vocabulary entry and a Lean `vocab` row.

**Files likely to conflict:**

* the OpenSSL lane: the 13 modules, `documented.rs`, `check.rs` and
  `contracts.lock` (regenerate the lock, do not merge it);
* C04.18 of the contract specification;
* `crates/qcore-frontend/src/frontend.rs` (`premise_clause`) and
  `crates/qcore-contract/src/contract_loader.rs` (`premise_clause`);
* the generated `proof/QCore.lean`, `docs/agents/gateway.md` and
  `docs/agents/README.md` (rerun their generators);
* `scripts/gate_scope_walkers.json` (one appended entry).
