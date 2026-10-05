# W43-ABI-ENUM-AND-CONST-PP: enumerations, pointers to pointers to const, guarded retentions

Card W43-ABI-ENUM-AND-CONST-PP (wave 48, card-high). Base: local `main` at
`4f55fa4c4`, which it stayed at. [PREDICTION.md](PREDICTION.md) was committed
at `c95487c56`, before any implementation code or contract. Authority:
`W43-LIBC-ABI-READER-SHAPES` and `CLASS1-UNDECLARABLE-RETAINERS`.

**Published property and scope.** Custody (`c.custody`) on the pinned corpus,
the system route and the tail-pause slice, as before. The card widens what a
declaration can be compared against (gateway rules 12u, 12v), adds one
value-conditioned clause (rule 12w) and fifteen declarations; it adds no
property.

## 1. The decisions

**USER DECISION 2026-10-05**, recorded verbatim in the contract
specification (C04.1) and in both ledger entries: (1) an enumeration
parameter or result is spelled as its underlying integer type, taken from
Clang for the target, never assumed `int`, exact width and signedness, refused
when Clang does not state it; (2) the shapes covered are enumerations and
pointers to pointers to const, with the const rule stated; transparent unions
and `va_list` stay registered; (3) a guard on an argument's value (`when
PARAM == CONST`, QUANT-INDEX-CHECKS' operators), applied only where the actual
is decided, the cautious reading otherwise, with total-or-reject grammar,
partition, Lean, and a body check where the body is available.

**A correction to decision 3's example.** `rd_kafka_confval_set_type`
(`rdkafka_conf.c:3763-3874`, byte-identical at `66910c10` and `046accf6`)
retains `valuep` in the `RD_KAFKA_CONFVAL_PTR` arm of `switch
(confval->valuetype)`, the **holder's own stored type**; that arm never reads
the `valuetype` argument. A caller passing the INT constant (0) for a
PTR-typed `confval` still has `valuep` retained, so a guard on `valuetype`
would be false of the body. The guard mechanism is built as decided; the
declaration carries the cautious, unguarded retention, and the in-unit body
check refuses exactly this shape (`tests/retention_guard.rs`,
`holder_switched`). No corpus declaration carries a guard
(`GUARDED-CLAUSE-WIDENING`).

## 2. BEFORE

Reproduced (binary `private/w43aecp/bin/before`, `0a83e728…`, equal to
`run.json`): corpus **812 / 9 / 4,013** (`a43d02cb…`, the brief's), system
87 / 4 / 357, slice 45 / 1 / 354 with 4 Pending (digests embed this worktree's
paths). Ranking population 625, 449 decided alone. The ledger's populations
reproduce exactly ([size-before-corpus.json](artifacts/size-before-corpus.json)):
enum 9 callees / 28 functions alone (`LZ4_decompress_generic` 9,
`rd_kafka_error_new_v` 4, `rd_kafka_error_new` 3, six more at 2); const-pp 3 / 7
(`rd_flags2str` 3, `rd_kafka_anyconf_dump` 2, `rd_slice_reader0` 2); class 1
`rd_kafka_confval_set_type` 5, `rd_kafka_interceptor_method_add` 4,
`add_item_to_object` 3, `cJSON_ParseWithLengthOpts` 2. `getrlimit`/`setrlimit`
block one function together (`mk_server_capacity`, Monkey), neither alone.

## 3. The reader (rules 12u, 12v)

**Enumerations (12u).** `legacy_enum_projection.rs`; the reader records an
untagged `typedef enum { … } NAME` as `TypedefTarget::UntaggedEnum` and reads
`enum TAG` and any chain to either. `AbiAgreement::read` reads the unit's
contracted declarations twice: a first pass records each enumeration spelling
met, one appended probe per unit asks Clang (with the unit's own source and
frontend request) for `sizeof(T) * __CHAR_BIT__`, the sign of `(T)-1` and
`__builtin_types_compatible_p(T, X)` for the ten standard integer types, and
the second pass reads with the answers. Clang's JSON states a fixed underlying
type but not a computed one, which is why the probe evaluates constants
instead (a measured probe: `{0,1}` → `unsigned int`, a negative value → `int`,
`0x100000000` → `unsigned long`, `enum : unsigned char` → `unsigned char`). The
projection is the one spelling of that width and signedness, admitted only for
exactly one compatible type whose signedness agrees; everything else refuses.

**Pointers to pointers to const (12v).** `ptr<mut, ptr<const, u8>>`: the outer
word qualifies the slot, the inner one the innermost pointee. Admitted for
`const char **`, `const void **`, an adjusted `const char *p[]` and a chain
whose innermost specifiers carry `const`; a qualifier between the `*`, on a
typedef name standing for a pointer, or on a pointer link of a chain refuses
(the last needed `chain_qualifies_a_pointer`: `resolve_typedef` folds such a
link away, harmless before because no depth-two reading carried `const`).
`POINTER_ABI_TYPES` gains the spelling; `resource_out` keeps its one.

Both widenings only add: every earlier reading is unchanged
(`the_pointer_widening_only_adds`; the enumeration branch was a refusal). One
row of `tests/loader_abi.rs` changed its refusal deliberately
(`const char **` declared `ptr<mut, ptr<mut, u8>>`: unreadable → disagreement,
withdrawn either way) and two enumeration rows moved from unreadable to a
disagreement with `i32` (Clang says `unsigned int`).

## 4. Guarded retentions (rule 12w)

`ensures normal resource_retained_by(P, H) when X OP K;` (C04.14.1, G05).
Loader: total-or-reject (`split_guard`, `parse_guard`, `guard_partitions`); `X`
an integer parameter other than `P` and `H`, `OP` one of `==` `<` `>` `<=`
`>=`, `K` a canonical decimal literal in `X`'s range, and the guard partitions
the range; a guard on any other clause rejects. Schema: `Retention::guard`
(`RetentionGuard` with the parameter's range; skipped when absent, so every
existing serialised retention is unchanged). Gateway
(`legacy_retention_guard.rs`): rule 12t's classifier drops a clause only for a
decided failure (a decimal constant, or an unshadowed file-scope enumeration
constant whose value an appended Clang probe states); the unit environment
removes every guard its own body does not validate (structural: every mention
of `P` outside an exact guard region is a test or a dereference, `X` never
written, no `goto`). Controls ([tests/retention_guard.rs](../../../crates/qcore-c-gateway/tests/retention_guard.rs)):
decided-off callers (`1`, `(0u)`, `K_INT`) are Safe; decided-on, a variable,
`-1` into `u32` and a shadowed `K_INT` are `retention-undecided`; a body
retaining only in `case K_PTR:` keeps the guard, a body switching on the
holder's own field loses it.

## 5. Contracts

Each body read at both revisions (byte-identical at `66910c10` and `046accf6`,
`scripts/body.py`), pinned with `quirin-audit pin` at both
(logs `private/w43aecp/pin-a1/`): every changed crate's
`pin --check` 0 at both, `--verify-contracts` 0, the whole-tree check 1 at both
as on the base (its one row the libevent crate's tip-mode `mk_vhost_set`).

| crate, module | declarations |
| --- | --- |
| `qcore-provider-librdkafka` `client.qv` (new, manifest-selected) | `rd_kafka_error_new_v`, `rd_kafka_error_new` (owned nullable; the second a data tail), `rd_kafka_err_action` (data tail), `rd_kafka_oauthbearer_set_token_failure`, `rd_kafka_toppars_pause_resume`, `rd_kafka_admin_add_config0` (borrowed, `none`), `rd_kafka_AclBinding_new0`, `rd_kafka_ConsumerGroupDescription_new`, `rd_kafka_anyconf_dump` (owned nullable; the last `ptr<mut, ptr<const, u8>>`), `rd_flags2str` (`desc` const-pp, an exact alias of `dst`), `rd_kafka_confval_set_type` (`valuep` retained by `confval`, cautious), `rd_kafka_interceptor_method_add` (`func`, `ic_opaque` retained by `list`) |
| `qcore-provider-librdkafka` `lz4_stream.qv` | `LZ4_decompress_generic` (two `u32` directives) |
| `quirin-provider-libc` `process.qv` (flexible) | `getrlimit`, `setrlimit` (`__rlimit_resource_t` as `u32`) |

`rd_kafka_resp_err_t` is `i32` (it starts at `-200`); every other enumeration
used is `u32`. Not declared: `rd_slice_reader0` (an interior pointer through an
out slot, `INTERIOR-OUT-SLOT-MODE`), `add_item_to_object`,
`cJSON_ParseWithLengthOpts`, `rd_kafka_assignor_add` (callbacks, class 1).

## 6. AFTER, attributed

Binary `private/w43aecp/bin/a2` (`552d86ab…`, equal to `run.json`). A first
run (`a2`) was +13 only: the corpus arbiter reads the Cycle 19 manifest
(`cycle19-resource-contracts.json`), which did not yet select the new module;
`a3` is the same binary with the module selected, the card's AFTER.

| route | BEFORE | AFTER | digest |
| --- | --- | --- | --- |
| corpus | 812 / 9 / 4,013 | **846 / 9 / 3,979** | `a43d02cb…` → `686fca16…` |
| system | 87 / 4 / 357 | unchanged, no row moves | `63bf8063…` → `f36275f7…` |
| slice | 45 / 1 / 354, 4 Pending | unchanged, no row moves | `1614b88b…` → `5a22a046…` |

Corpus **+34 Unknown → Safe**, no other status move
([comparison](artifacts/compare-corpus.json)): over `LZ4_decompress_generic` 9
direct and 3 through their summaries (the twelve `LZ4_decompress_safe*` and
`LZ4_uncompress_unknownOutputSize`); `rd_kafka_AclBinding_new`,
`_AclBindingFilter_new` and their copies (4), `rd_kafka_ConsumerGroupDescription_copy`,
`_copy_opaque`, `_new_error` (3); `rd_kafka_ConfigResource_set_config`,
`rd_kafka_NewTopic_set_config`; `rd_kafka_conf_dump`, `rd_kafka_topic_conf_dump`;
`rd_kafka_pause_partitions`, `rd_kafka_resume_partitions`; two
`rd_kafka_err_action` handlers; two oauthbearer failure paths; three
`rd_kafka_error_new` returners; `rd_kafka_event_debug_contexts`
(`rd_flags2str`); `mk_server_capacity` (`getrlimit`/`setrlimit`). **No Safe →
non-Safe move; `Unsafe` 9 → 9, no leak or violation added.**

85 more Unknown functions changed leaves only, each class read: 29 callers of
`rd_kafka_error_new` and 12 of `rd_kafka_err_action` now meet a later leaf the
uncontracted call hid (52 `nullable resource … used by a non-null contract`
leaves are the owned nullable error object passed on untested; 55 `unresolved
resource identity`); the 9 class-1 callers trade the uncontracted call for the
access gate on their holder actual (`&options->FIELD`, `&conf->interceptors.LIST`:
`indirect memory access is outside the resource projection`), so rule 12t
never decides them (the prediction said `retention-undecided`; Unknown
either way); `rd_kafka_error_new`, `_fatal`, `_retriable`,
`_txn_requires_abort` read `rd_kafka_error_new_v` withdrawn as unreadable in
`rdkafka_error.c`, whose units declare `va_list` twice
(`ABI-DUPLICATE-TYPEDEF-READING`); eight read a call in a branch condition
now that its callee's summary validates.

Remaining ([tally](artifacts/tally-a3.txt), [ranking](artifacts/rank-a3-corpus.json)):
**571** Unknown functions blocked by uncontracted calls alone, 403 by one
callee alone (625 / 449 before; system 26 / 18, slice 38 / 27 unchanged).
After the card ([size-after-corpus.json](artifacts/size-after-corpus.json)):
the enum shape blocks nothing alone; const-pp only `rd_slice_reader0` 2;
class 1 25 callees / 63 functions.

**Parity, compared in full** ([corpus](artifacts/parity-corpus.json),
[system](artifacts/parity-system.json), [slice](artifacts/parity-slice.json)):
system and slice multisets identical (139,962 and 50,426 rows); corpus 668,259
→ 668,005 rows, 136 functions differ. Outside the moving set: the declared
callees' own bodies (W4 no longer infers a summary where a declaration stands,
and a retaining body is never inferred: `rd_kafka_confval_set_type` 196 rows,
`rd_kafka_interceptor_method_add` 18), and seven functions
(`rd_kafka_assignment_add`, `rd_kafka_check_produce`,
`rd_kafka_DescribeTopicsResponse_parse`, …) whose statements reach
`rd_kafka_error_new` behind an earlier refusal of the same statement, so their
consultations change and their leaves do not.

**`mk_vhost_set` at the tip** ([rows](artifacts/tip-rows-after.txt)):
byte-identical to CALLEE-RETENTION-CLASS1's record.

## 7. Prediction against outcome

| item | predicted | measured |
| --- | --- | --- |
| corpus | +12 to +30 | **+34** (above the range: the three LZ4 summaries and four constructor-copy summaries cascade, and `rd_kafka_error_new_v`'s 4 did not move) |
| class-1 callers | `retention-undecided` | the access gate on the holder actual (Unknown either way) |
| new leaks or violations | 0 to 3 | 0 |
| Safe → non-Safe | none | none |
| system, slice | 0 to 1 / none | none / none |
| reader | no existing outcome changes | none in the corpus; three synthetic rows changed refusal (§3) |
| `rd_slice_reader0` | not declarable | not declared |

The first run's +13 (the manifest omission) is recorded, not hidden: it
measured the LZ4 and libc declarations alone.

## 8. Lean

Two modules, registered in `AuditAbiEnumConstPointer.lean` and
`AuditRetentionGuard.lean` and imported by `QCore.lean`, no placeholder, no
project axiom (291 → 293 audits):

* **`QCore.AbiEnumConstPointer`** (15 audited): a projection exists only for
  one compatible type of the probed signedness and has the probed width and
  signedness; never `isize`/`usize`; never `i32` for an unsigned enumeration;
  no facts, no projection; the four measured shapes; the pointer widening only
  adds, the new spelling is exactly the innermost-const two-level pointer, a
  qualified slot refuses, `resource_out` never admits it.
* **`QCore.RetentionGuard`** (11 audited): only a decided in-range failure
  takes a clause out of force; every reading lies within the cautious one and
  an undecided call reads exactly it; for any verdict more retention never
  improves, every reading is bounded by the cautious one, so **an undecided
  guard is `Safe` only where the cautious reading is**; an admitted guard
  partitions its range; a validated body keeps nothing when the guard fails;
  the Rust unit tests' vectors.

## 9. Ledger

`scripts/update_ledger.py` (follow-ups 710 → 715): `W43-LIBC-ABI-READER-SHAPES`
and `CLASS1-UNDECLARABLE-RETAINERS` amended with the decision verbatim, what
was carried out and what remains (the transparent union and `va_list`;
`add_item_to_object`, `cJSON_ParseWithLengthOpts`); `W48-CALLEE-DECISION-TABLE`,
`W45-CALLEE-CASCADE-RESIDUAL` (571 / 403) and `RETAINED-BY-BODY-CHECK`
amended. Registered: `ABI-ENUM-UNIT-FRONTEND-FLAGS`,
`ABI-DUPLICATE-TYPEDEF-READING` (`rd_kafka_error_new_v`, 4),
`GUARDED-CLAUSE-WIDENING`, `INTERIOR-OUT-SLOT-MODE` (`rd_slice_reader0` 2,
`cJSON_ParseWithLengthOpts` 2), `NATIVE-HEADER-ABI-ENUM-CONST-PP`.

## 10. Gate

See [gate.json](gate.json) and [gate/](gate/) (filled when the gate
completes).

## Files

* Rules 12u, 12v: `crates/qcore-c-gateway/src/legacy_enum_projection.rs` (new),
  `legacy_declared_abi.rs`, `typed_access_path.rs` (`appended_declarations`),
  `legacy_c.rs` (module mounts); `crates/qcore-contract/src/contract_loader.rs`
  (`POINTER_ABI_TYPES`).
* Rule 12w: `crates/qcore-schemas/src/retained_by.rs` (`RetentionGuard`,
  `GuardRelation`, `GuardDecision`), `crates/qcore-contract/src/contract_retained_by.rs`,
  `contract_loader.rs` (the guarded clause), `crates/qcore-c-gateway/src/legacy_retention_guard.rs`
  (new), hooks in `legacy_retained_by.rs` and `legacy_c.rs` (three entry
  points, beside the ABI gate).
* Lean: `proof/QCore/AbiEnumConstPointer.lean`, `proof/QCore/RetentionGuard.lean`,
  their `Audit*.lean`, `proof/QCore.lean`.
* Tests: `crates/qcore-c-gateway/tests/abi_enum_const_pp.rs`,
  `tests/retention_guard.rs`, `crates/qcore-contract/tests/retention_guard_clause.rs`;
  deliberate pin updates in `compiler/tests/contract_loader.rs`,
  `compiler/tests/qcore_contract_corpus.rs`, `crates/qcore-audit/tests/provider_selection.rs`,
  `crates/qcore-c-gateway/tests/{loader_abi,neighbours,prod_contracts}.rs`,
  `crates/qcore-contract/tests/retained_by_clause.rs`,
  `scripts/tests/test_contract_corpus.py`, `providers/tests/test_contract_load_path.py`.
* Contracts: `providers/qcore-provider-librdkafka/contracts/librdkafka/{client,lz4_stream}.qv`,
  `providers/quirin-provider-libc/contracts/libc/process.qv`, their locks,
  `contracts.json`, `providers/contract-load-path.json`,
  `cycle19-resource-contracts.json`.
* Records: contract spec C04.1, C04.4, C04.14, C04.14.1; grammar G05;
  `docs/agents/gateway.md` (12u–12w), `docs/agents/README.md`, `docs/STATUS.md`,
  `docs/development/increments/wave-48-part5.md`,
  `docs/reviews/r0_3/acceptance_matrix.json`, `docs/reports/INDEX.md`.
* Scripts: adapted from CALLEE-RETENTION-CLASS1 (build, measure, rank, tally,
  compare, parity, pin, sync, gate, provider gate, finish), plus
  `preprocess.py` and `census.sh` (the C side of the member declarations) and
  `update_ledger.py`.
