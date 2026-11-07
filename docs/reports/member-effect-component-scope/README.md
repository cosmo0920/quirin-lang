# MEMBER-EFFECT-COMPONENT-SCOPE

Registered follow-up of MEMBER-EFFECT-SUMMARY
([report §5](../member-effect-summary/README.md#5-gate-and-measurement)):
`cmt_opts.c::cmt_opts_exit` was predicted `Unknown -> Safe` under rule 38j and
stayed `Unknown`, its five `cfl_sds_destroy(opts->F)` releases still named
`member-release-through-formal`; the entry attributed this to the row's scope,
the component `[cmt_opts_exit, cmt_opts_init]`. Base `1e522ca5d`
(origin/main); measurement side `baseline:7b726587b`.

## 1. Diagnosis: no component premise; the member is spelled through a pointer typedef

**The scope is not a component.** A row's `hypothesis_context.scope` is the
list of its unit's source functions, which every row of the unit shares. On
the corpus at `baseline:7b726587b`, 4,829 of 4,834 rows have a scope of
several functions; `rd_kafka_metadata_partition_clear`, the row that did
move, has a scope of 34 functions (its whole `rdkafka_metadata.c`), as do the
other rows rule 38j moved (`metric_release_storage`: 18,
`cmt_metric_exp_hist_snapshot_destroy`: 29, `free_chunk_header`: 26).
`cmt_opts_init` and `cmt_opts_exit` call neither each other nor anything in
common, so they are two singleton components of the summary derivation's call
graph (`derive_custody_summaries`), and rule 38j's `component.len() == 1`
condition holds for both. None of the three premises the brief names (mutual
recursion, a shared summary table, a joint fixed point) is in play.

**The premise that refuses it.** `struct cmt_opts` declares its five members
`cfl_sds_t ns;` … with `typedef char *cfl_sds_t;` (`cfl_sds.h:32`). Every row
that moved releases a member spelled with a `*` (`uint64_t *hist_buckets`,
`int32_t *isrs`, `char *buffer`, `void *fs_backend`). The path of the
statement `cfl_sds_destroy(opts->ns);`:

1. Rule 38j's body check (`BodyReading::read`) is lexical and reads no type:
   it admits all five places `(opts, F)` and adds them to rule 5k's
   `AdmissibleReleases` (`with_places`).
2. The statement is refused by the lexical gate and reaches the call-argument
   rule (`legacy_call_argument_access.rs::candidate_arguments`). Its actual
   `opts->ns` sits at a `Released { nullable: true }` position, so
   `position.admits()` is false and only the lexical recogniser `recognise`
   is asked. Its `pointer_read` arm for `B -> F` requires
   `pointer_member(model, tag, field)`, which is false for a member the
   declaration reader does not classify: the reader records a pointer
   typedef unresolved (`typed_field_access.rs::read_typedefs`), so a member
   spelled through one is in no field class. The answer is
   `Err(Refusal::UntypedMember)`.
3. Rule 1 of W45-CALLEE-SUMMARY-CASCADE (`member_release::admits_release`)
   admits a member handed to a nullable release only on that recogniser's
   pointer read, so nothing is admitted; the typed (Clang) reading of rule
   10k, which would read `opts->ns` as a `char *`, is asked only at a position
   that admits a read.
4. The statement keeps the gate's refusal, which rule 38i then names
   `member-release-through-formal` (rule 38i's own reading reads every actual
   as borrowed, where the typed reading is asked, so it does name it).

This is the untyped-member arm of W45-MEMBER-RELEASE-UNTYPED-DECLARATIONS
("a member declared through a typedef of a pointer, e.g. `cfl_sds_t buf`"),
not a property of the row's scope. MEMBER-EFFECT-SUMMARY's fixture spelled
the members `char *a`, so its `cmt_opts_exit` shape was `Safe`.

**Fixture** (`crates/qcore-c-gateway/tests/member_effect_component_scope.rs`,
`the_deciding_premise_is_the_member_spelling_not_the_unit`): the
`cmt_opts` record, `cmt_opts_exit` verbatim and a shortened `cmt_opts_init`,
under the real `libc/memory.qv` and `cfl/sds_lifetime.qv`. With the members
spelled `cfl_sds_t` the exit is `Unknown` with the five rule-38i names, alone
and beside the initialiser; with `char *` it is `Safe`, alone and beside it.

## 2. Census: every rule-38i release leaf (every scope is a unit's)

[`scripts/census.py`](scripts/census.py) over each route's
`resource-diagnostics.json` at `baseline:7b726587b`, and
[`scripts/census_current.py`](scripts/census_current.py) over the user's
`current` run (Fluent Bit 046accf6; its report predates rule 38i, so its
release leaves are selected with MEMBER-EFFECT-SUMMARY's step-1 estimate over
MEMBER-ACCESS-PROJECTION's census). The base and member spellings are read
lexically from the analysed source tree (the parameter declaration, the first
`struct TAG {` definition), so a class is a reading of the source, not the
analyzer's answer. "Sole" is a function every one of whose leaves is a rule-38i
release leaf. Outputs: [`census/`](census/).

| route | base | member | leaves | functions | sole-blocker functions |
|---|---|---|---:|---:|---|
| corpus | `struct T *` | pointer typedef | 24 | 17 | `cmt_opts_exit` |
| corpus | `struct T *` | `*`-spelled | 43 | 25 | `rd_kafka_snappy_free_env` |
| corpus | typedef of a record | `*`-spelled | 88 | 63 | `rd_kafka_offset_file_close` |
| corpus | typedef of a record | untagged or not found | 78 | 45 | `mk_ptr_free`, `rd_kafka_mock_sgrp_record_release` |
| system | `struct T *` | pointer typedef | 1 | 1 | none |
| system | `struct T *` | `*`-spelled | 14 | 5 | none |
| slice | `struct T *` | pointer typedef | 5 | 2 | none |
| slice | `struct T *` | `*`-spelled or unresolved | 24 | 5 | none |
| current | `struct T *` | pointer typedef | 26 | 18 | `cmt_opts_exit` |
| current | `struct T *` | `*`-spelled | 29 | 20 | (`metric_release_storage`, `cmt_metric_exp_hist_snapshot_destroy`: rule 38j already) |
| current | typedef of a record | any | 18 | 16 | `rd_kafka_offset_file_close` |

Corpus totals: 233 leaves in 149 functions; system 15 in 6; slice 29 in 6;
`current` (estimate) 73 in 54. Every one of these rows has a several-function
scope. The sole-blocker functions other than `cmt_opts_exit` are refused on
other premises: `rd_kafka_snappy_free_env` ends with `clear_env(env)`, a
whole-base call to a function with no member effect (P1); the three bodies
with a typedef-spelled base are the base arm of
W45-MEMBER-RELEASE-UNTYPED-DECLARATIONS.

## 3. Decision: admit the untyped member at an admitted release; no component premise to state

The registered alternatives were to admit the effect inside a component with
its premises stated, or to record the premise that keeps it refused. Neither
fits: there is no component premise, and the premise that refuses
`cmt_opts_exit` is a reading defect fail-closed settles. Clang types
`opts->ns` as the value of a `char *` member, exactly the fact the lexical
reader gives a `*`-spelled member, and rule 1 needs nothing more. The card
therefore admits that one reading, at that one position, and keeps every
other premise:

**[Rule 38j](../../agents/rules/38j.md), amended in place.** A release
event's actual `B->F` whose member the lexical declaration reader leaves
untyped is read by Clang instead
(`legacy_call_argument_access.rs::candidate_arguments`, the
`Refusal::UntypedMember` arm; `legacy_member_effect.rs::Parser::typed_member_release`),
only when:

- the position is `Released { nullable: true }` of the contract in force;
- `(B, F)` is a place rule 5k's P1-P4 or rule 38j's P1-P3 admitted in this
  body (`AdmissibleReleases::admits`);
- the lexical refusal is the untyped **member**, so the reader already typed
  `B` as a record pointer (an untyped base, the other arm of
  W45-MEMBER-RELEASE-UNTYPED-DECLARATIONS, is not read);
- Clang's actual, through parentheses, is an `LValueToRValue` read (never an
  `ArrayToPointerDecay`, which would hand an interior address to the
  release) of an arrow `MemberExpr` named `F` whose type is an object
  pointer with no `volatile`/`_Atomic`, over a `DeclRefExpr` to the
  parameter `B` that points to no `union`.

The actual then reads as the lexical `B->F` does (base `B`, a pointer value,
no chain) and rule 1's lowering is unchanged (the position read `Borrowed`).
No rule id is added: the amendment is a reading inside rule 38j's own event
(ids 40, 41, 42 reserved or held elsewhere, checked).

**Scope of the reading.** Rule 1's admission is shared by rule 5k and rule
38j, so a rule-5k body over a pointer-typedef member
(`cfl_sds_destroy(opts->ns); free(opts);`) is admitted too; nothing else asks
Clang at a release position. Concurrent files: none of the waiting cards'
files is touched (`legacy_call_argument_access.rs` gains one match arm).

## 4. Fixtures and Lean

`crates/qcore-c-gateway/tests/member_effect_component_scope.rs` (Clang-backed,
every `Safe` result's certificate rechecked; the diagnosis fixture of §1 is
flipped deliberately by the amendment and replaced):

| fixture | shape | result |
|---|---|---|
| `the_cmt_opts_pair_is_admitted_whatever_spells_the_members` | `cmt_opts_exit` with `cfl_sds_t` and with `char *` members, alone and beside `cmt_opts_init` | `Safe`; `cmt_opts_init` keeps `member-store-through-formal` |
| same | rule 5k over a `cfl_sds_t` member: `cfl_sds_destroy(opts->ns); free(opts);` | `Safe` |
| same | `cmt_counter_destroy`'s shape in one unit: `cmt_opts_exit(&c->opts); free(c);` | `Safe` (the embedded cascade) |
| `a_member_one_function_releases_and_another_still_uses_stays_refused` | `cmt_opts_exit(opts);` then `opts->ns` read, or `cfl_sds_destroy(opts->name)` | `Unknown`, `member-effect-call-refused` |
| same | in one body: a use after the release; two releases | `Unknown`; the second keeps both rule-38i names |
| `a_recursive_pair_stays_refused` | `exit_a`/`exit_b` releasing a member each and calling each other with the base; a self-recursive exit | `Unknown` (no effect is published in a recursive component, so the whole-base call fails P1) |
| `the_typed_reading_admits_only_a_pointer_members_value` | an array-typedef member (`free(r->buf)`), a function-pointer typedef member, a typedef-spelled base `r_t *`, a `union` base | not `Safe` (the array and the base keep rule 38i's name) |
| same | `if (r->s) { cfl_sds_destroy(r->s); }`, `sds_t s` | `Safe` |

`proof/QCore/MemberReleaseTypedMember.lean` (audited by
`proof/AuditMemberReleaseTypedMember.lean`, axioms `propext`, plus
`Classical.choice` and `Quot.sound` through `body_sound`; no placeholders):
`an_old_admission_is_kept`, `a_new_admission_is_an_untyped_member`,
`an_admission_needs_the_place_and_a_nullable_release`,
`an_untyped_base_is_never_admitted`, `a_decayed_array_is_refused`,
`a_function_pointer_is_refused`, `a_qualified_pointer_is_refused`,
`a_union_base_is_refused`, `typedPointer_type`,
`an_admitted_actual_is_an_object_pointer` (given the lexical reader's own
premise for a `*`-spelled member, `LexicalSound`), `the_body_stays_sound`
(rule 38j's `body_sound` applies to the admitted event unchanged) and
`the_unit_plays_no_part`. Premises stated, not proved: Clang's
`MemberExpr` type is the member's declared type, and the lexical reader's
`*`-spelled reading (trust boundary 28).
