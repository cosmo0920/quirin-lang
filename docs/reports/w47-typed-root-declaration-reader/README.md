# W47-TYPED-ROOT-DECLARATION-READER

Discharges the follow-up of the same name that W47-ACCESS-PATH-POSITIONS
registered. Base `3ba72320` (local `main`, "Merge runtime imports and explicit
contract checks after complete gate"). The prediction and the negative
controls were frozen and run at the base before any analyzer change
([PREDICTION.md](PREDICTION.md), commit `148afe83`). Gateway rule
[10l](../../agents/gateway.md#constraint-10l); Lean model
`QCore.TypedRootDeclarations`.

## The cause of the twelve misses (step 1)

The follow-up said twelve frozen-forecast Safe predictions of P3 miss because
the never-bound, `may_bind` and declared-local premises read a lexical
declaration reader that answers nothing for their functions. A temporary trace
(`scripts/trace_twelve_patch.py`, never committed; run by `scripts/diag.py`;
output in [artifacts/cause/](artifacts/cause/)) printed, at every typed hook of
the twelve, the lexical answers and the typed reader's answer with **every
premise granted**. For all twelve the lexical reader is readable and every
root is never bound and not `may_bind`; the typed reader still refuses. The
exact reasons, from the matched Clang nodes
([twelve-node-chains.txt](artifacts/cause/twelve-node-chains.txt)):

| Functions | Reason |
|---|---|
| `rd_kafka_Node_host`, `rd_kafka_Node_rack`, `rd_kafka_mock_cluster_bootstraps` | value `char *const` (a pointer member through a `const` pointee): `is_pointer` requires a trailing `*` |
| `rd_kafka_mock_cluster_handle`, `rd_kafka_opaque`, `rd_kafka_handle_mock_cluster` | value `rd_kafka_t *const`, `void *const`, `rd_kafka_mock_cluster_t *const` |
| `rd_kafka_messages_get`, `rd_kafka_share_partition_offsets_list_get` | element of a `T *const[]` member: `T *const` |
| `rd_kafka_ConfigEntry_name`, `rd_kafka_share_partition_offsets_offsets` | array member returned by decay (`char[1]`, `const int64_t[]`): an interior address, rule 5j by design |
| `rd_kafka_topic_partition_get_leader_epoch`, `…_get_current_leader_epoch` | `parpriv = rktpar->_private`: the lexical declared-local answer is `Opaque` (a real premise gap), **and** the value is a `BitCast` from `void *` over a `void *const` member, which the statement reader refuses |

P3's control `a_redeclared_typedef_formal_keeps_the_pointer_return_refusal`
isolates the same `char *const` cause, not a declaration one: with a
non-`const` formal the doubly declared typedef certifies at the base
(`the_redeclared_typedef_control_refuses_for_its_const_pointee`); its comment
is corrected. The typed-reader causes are registered as
`W47-TYPED-CONST-POINTER-VALUE`, outside this card's mechanism, which the brief
fixed; this card implements the premise reader as asked and moves none of the
twelve, as predicted.

## Mechanism (rule 10l)

`typed_root_declarations.rs` reads the function's `FunctionDecl` from the same
Clang AST the typed reader matches; `legacy_typed_roots.rs` serves its answers
to the P3 hooks (`legacy_typed_positions.rs` changes only which premise
function each hook calls).

- **Where the lexical declaration reader read the function, every answer is
  its own, unchanged.** Where it did not, a name is answered only when it is
  declared exactly once as a formal or an automatic local and no reference
  names another object of its spelling.
- **Never bound**: neither the AST closure nor the lexical closure (now
  computed there too, `UntrackedBases::lexical_risky`, with `never_bound`
  still `false`) binds it. The AST closure seeds the object root of a `&`
  operand, an increment, a compound assignment and a sub-object store; every
  variable of an actual at a position its contract does not declare
  `Borrowed` (all of an uncontracted, indirect or atomic/`va_arg` builtin),
  except an `->` base; protected formals; and closes over assignments and
  initialisers whose value holds a call, an address, a statement expression, a
  bound or an undeclared name. The loop settles within `assignments + 1`
  rounds or refuses the function.
- **Pointer root** (rule 5b): also named by neither the lexical nor the AST
  `may_bind` (every name written, address-taken or in any actual).
- **Declared local**: a lexical `yes` stands; otherwise one automatic,
  non-formal declaration of a type the typed reader reads.
- **Refused functions**: no body or AST, a K&R definition (a `;` between
  declarator and body), inline assembly, a block, a block-scope `typedef`, an
  unsettled closure.
- **Parity**: a parity run builds the AST index for every analysed function
  and writes one `root-declarations` row per analysis: each declared name
  with the lexical, lexical-closure, AST and served answers, the AST binding
  reason and every disagreement.

## Measurement

Arbiter: `scripts/run_pinned_audit.py OUT COMMIT --inputs-root
/home/cosmo/Documents/GitHub/qcore-c/build/cycle19 --jobs 5` (the brief's
`w43-return-expression/scripts/run_audit.py` calls the example `b64be69e`
retired; PREDICTION.md cites `e4a73427`, which added the replay, and is
corrected here) and `w46-contract-gap-expansion/scripts/run_system.py OUT
--jobs 5` at `046accf6`, each with `QCORE_TYPED_PATH_PARITY`. Private runs:
`private/w47tr/` of this worktree.

**BEFORE** (binary at the base, `5d4eb06c…`): both exit 0. Corpus
diagnostics, monkey source and chunk contracts byte-identical to
`w47-access-path-positions/artifacts/after/corpus/`; sorted parity and
contexts identical; the contracts and source-effects sidecars differ only by
the `.qcore` → `.qv` migration (`f4018a92`, EXT-02): paths, lock/module digests
and the digests derived from them
([before-identity/](artifacts/before-identity/)). System: 0 changed rows
(63 / 4 / 381 configuration rows, unique functions 33 / 2 / 199); raw
differences are contract-manifest and inventory digests of the same
migration. The census card's independent post-merge measurement
(`w47-alias-residuals-census`, commit `063f0911`) reports the same identity.
The BEFORE system run records `qcore_revision` `148afe83` (the prediction
commit, docs and tests only); its binary is the base binary.

**AFTER** at `7e5389be` (binary `b005c6e1…`,
[archive](artifacts/after/corpus/)); three earlier AFTER runs at `455117e7`,
`b082e819` and `80559dd8` (the last recorded with a mistyped commit argument
`80559dd8d`) differ from it only in parity fields; their diagnostics are
byte-identical to the final ones.

| Measure | BEFORE | AFTER |
|---|---:|---:|
| Corpus functions Safe / provisional Unsafe / Unknown | 507 / 9 / 4,318 | **507 / 9 / 4,318** |
| Corpus units Safe / Unsafe / Unknown / data-only | 6 / 8 / 167 / 4 | same |
| Leak / unknown_alias | 34 / 7,440 | 34 / **7,441** |
| COND / RET / IND / UNC / OTHER leaves | 6,369 / 284 / 15,037 / 8,902 / 5,670 | **6,331** / 284 / **15,034** / 8,902 / 5,670 |
| System configuration rows / unique functions | 63 / 4 / 381; 33 / 2 / 199 | identical; diagnostics byte-identical |

No function or unit moves, so there is no Safe → non-Safe move and no
correction; no leak, violation or provisional Unsafe is added, so there is
nothing to classify as real, artifact or premise gap. **39 functions change
leaves** ([comparison](artifacts/after/corpus-comparison/)), every changed leaf
at an admitted typed row of the same function and line (`compare.py`
`direct`): 38 COND (12 condition, 5 condition/call-actual, 21
condition-assignment) and 3 IND (call-actual) refusals removed; 27
`unknown_alias` leaves added at condition-assignment rows (rule 10b's
identity refusal on the assigned local, and read obligations on roots the
lexical closure binds); 26 `unknown_alias` leaves removed at condition rows
whose roots the AST now serves never bound, so their per-hop obligations no
longer need an identity. 24 of the functions are AST-served (lexical reader
unreadable); 15 are readable functions whose `NAME = PATH` destination the
lexical reader types `Opaque` and the AST answers a declared local
(`rd_kafka_subscribe`, `rd_kafka_assignment`, `rd_kafka_committed`, …). The
older parity rows change only at those positions
([parity delta](artifacts/after/root-declarations/corpus-parity-delta.json.gz)).

## Prediction against outcome

1. None of the twelve Safe — **holds**; the ten typed returns and two
   assignment operands refuse as tabled.
2. Lexical answers unchanged — **holds**: every changed function is AST-served
   or gained only an AST declared-local answer.
3–4. No status move; COND/IND/RET removed only at servable rows; aliases
   possibly added — **holds** (38 + 3 removed, 27 added). **Missed**: the 26
   `unknown_alias` removals and 12 changed functions outside the forecast set
   (`rd_list_is_sorted`, `rd_kafka_topic_partition_list_cmp`, …). The census
   counted rows the typed reader *refused*; these rows were already admitted
   with a never-bound root read as bound, and now carry the served never-bound
   root, dropping the identity obligation. Each root is a formal or local the
   joined closures prove unbound.
5. System slice unchanged — **holds** (byte-identical diagnostics).
6. Parity rows — **holds**: 17,832 `root-declarations` rows (6,969 distinct
   function records; 94 differ between a function's analyses, whose token
   streams differ by contract holds).

## Both readers side by side

Of 6,969 corpus function records, 6,731 are served by the lexical reader, 230
by the AST and 8 by neither
([root rows](artifacts/after/root-declarations/corpus-root-rows.json);
system: 379 / 2 / 4). Disagreements, by the AST's own reason
([classification](artifacts/after/root-declarations/corpus-disagreements.json)):

- **AST binds, lexical closure says never bound** (1,118 name records in
  readable functions, where the lexical answer is served): the AST is the
  conservative side for `&` values (296, e.g. `TAILQ_FOREACH`'s
  `&head->tqh_first`), statement expressions (179, rule 5l's `container_of`),
  sub-object stores (82), undeclared names (5), copies of those (305), direct
  seeds the lexical closure reads differently (21: 17 custody actuals, 2
  addresses, 2 compound assignments), names not answered (111). **One class is a lexical blind spot**: 119 names bound
  from a call in an assignment nested in a condition or a `for` clause, which
  the lexical closure never reads. 33 are roots of admitted typed rows, all in
  non-Safe functions ([exposure](artifacts/after/root-declarations/corpus-exposure.json)).
  The only Safe functions with any disputed root are `mk_fifo_queue_get` and
  `mk_plugin_cap` (rule 5l `container_of`) and `rd_kafka_Uuid_str` (a scalar
  array sub-object store), where the AST over-approximates. Registered as
  `W47-LEXICAL-CLOSURE-NESTED-ASSIGNMENT`; the lexical answer is not changed
  here.
- **Lexical closure binds, AST says never bound** (449, all masked by the
  join): the lexical closure takes a comma expression's tail as the value
  (`it = it->next, ret++` in `cfl_list_size`, `mk_list_size`); every other
  record is listed per name in the parity stream.
- **`may_bind`** (768 lexical-only, 175 AST-only) and **declared local**
  (1,405 lexical-only: record types and shadowed names the AST does not type
  as one local; 3,044 AST-only: names the lexical reader types `Opaque`) are
  recorded per name with the binding reason.

## Lean

`proof/QCore/TypedRootDeclarations.lean`, registered in
`proof/AuditTypedRootDeclarations.lean` (28 theorems; audit total 217). It
imports `QCore.TypedAccessPathPositions` and proves: a seeded, settled set
contains every binding of the model (`bound_within_a_settled_set`), with
executable checks (`seeded_of_check`, `settled_of_check`), so a name outside
the computed set has no binding (`an_unlisted_name_is_never_bound`,
`a_never_bound_root_has_no_binding`); the lexical answer stands where it
answers; the served answer implies the lexical closure's and the AST's; an
unanswered function or a name declared twice serves no root; a pointer root is
named by neither `may_bind` and is never bound; an actual at a `Borrowed`
position is `may_bind`, never bound; a pointer return through a root either
closure binds refuses (reusing P3's
`a_returned_pointer_from_a_tracked_root_refuses`), and a served return hands
nothing back; a declared local is one automatic, non-formal, typed
declaration and a lexical `yes` stands; with non-vacuity witnesses. The
Rust-to-Lean correspondence (that the Rust loop returns a seeded, settled set,
and the extraction) is a review premise (constraint 28).

## Tests

`crates/qcore-c-gateway/tests/typed_root_declarations.rs` (8 tests, written and
run at the base first; [base outcomes](artifacts/cause/controls-at-base.txt)):
an untouched root certifies when only the AST answers (Unknown at the base);
the lexical answer is kept where it answers, and a statement route keeps the
lexical answer; `&root`, a write from a call, a copy, a release, a `Borrowed`
callee (`may_bind`), an increment and a macro-expanded assignment keep the
return refusal; a shadowed root keeps it; a C11-compatible redeclared typedef
certifies while a block-scope typedef of the same name refuses; the P3
control's `const` cause; a K&R definition, an implicitly declared callee
(rejected by C11 Clang) and a file-scope root refuse; the declared-local
operand is answered, and a `static` or shadowed local is not. Unit tests in
`typed_root_declarations.rs` (6) cover every binding form, `Borrowed` and
`->`-base actuals, not-answered names and whole-function refusals.

## Gate

[gate.json](gate.json), exits in [gate/](gate/). Rust, Lean and census steps
on the code at `9ff02185` (later commits change docs only): submodule 0;
`run_test_binaries.py` 0 — **340 units, 353 summaries; 6,136 passed, 0
failed, 11 ignored** (`private/test-binaries/20260928T013332Z-592808/summary.json`);
`cargo clean --profile dev` 0; fmt 0; clippy 0; both reevaluation examples 0;
`check_lean_audits.py` 0 (**217** audits); Clang 21.1.8 recorded; the
census-feature gateway clippy 0 and library tests 0 (623). After the matrix
and index regeneration: `unittest discover -s scripts/tests` 0 (**365**) and
`build_docs_index.py --check` 0. No `providers/` path is touched, so the
provider gate does not apply; `provider_tests` is carried in gate.json.

## Follow-ups

- Retired: `W47-TYPED-ROOT-DECLARATION-READER` (its discharge is implemented
  and measured; its twelve motivating misses are re-attributed).
- Registered ([followups.json](followups.json)): `W47-TYPED-CONST-POINTER-VALUE`
  (ten of the twelve: eight `T *const` values, two `void *` bit-casts) and
  `W47-LEXICAL-CLOSURE-NESTED-ASSIGNMENT` (119 name records, 33 admitted-row
  roots, no Safe dependence). Ledger 523 → 524.
- Stays open: `W47-ACCESS-PATH-POSITION-RESIDUALS`, `W46-ACCESS-PATH-PARITY`.

## Files likely to conflict

`crates/qcore-c-gateway/src/{legacy_c.rs,legacy_typed_positions.rs,typed_access_path.rs,borrowed_actual.rs,lib.rs}`
(thin hooks), `crates/qcore-c-gateway/tests/access_path_positions.rs` (one
comment), `proof/QCore.lean`, `docs/agents/{gateway.md,README.md}`,
`docs/STATUS.md`, `docs/plans/unsupported-coverage/README.md`,
`docs/development/increments/wave-47.md`,
`docs/reviews/r0_3/acceptance_matrix.json`, `docs/reports/INDEX.md`.
