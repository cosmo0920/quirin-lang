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
