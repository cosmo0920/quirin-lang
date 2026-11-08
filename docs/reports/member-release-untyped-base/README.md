# MEMBER-RELEASE-UNTYPED-BASE

The untyped-base arm of W45-MEMBER-RELEASE-UNTYPED-DECLARATIONS, as
MEMBER-EFFECT-COMPONENT-SCOPE recommended
([report §6](../member-effect-component-scope/README.md#6-measurement-and-follow-ups)).
Base `7cdd33906` (the tip of MEMBER-EFFECT-COMPONENT-SCOPE, not landed);
measurement side `rev:662dc54a4`.

## 1. Census: every rule-38i release leaf over a typedef-spelled base

[`scripts/census.py`](scripts/census.py), run by
[`scripts/run_census.sh`](scripts/run_census.sh) over the inputs
MEMBER-EFFECT-COMPONENT-SCOPE's census read (each route's
`resource-diagnostics.json` at `baseline:7b726587b`, the source tree that run
analysed, and for the user's `current` run on Fluent Bit 046accf6 the
MEMBER-ACCESS-PROJECTION premises census with MEMBER-EFFECT-SUMMARY's step-1
estimate). Outputs: [`census/`](census/). Each leaf records the base's
spelling, what the lexical declaration reader answers for it, the member's
spelling, the release callee's mode in the provider contracts (`unit`: no
provider declares it) and every other blocker of its function. A class is a
reading of the source tree, not the analyzer's answer; the reader column
reproduces `typed_field_access.rs` (`read_typedefs`, `classify_specifiers`)
over the whole tree, so a disagreement it finds may be absent from one
preprocessed unit.

**The reader resolves `typedef struct TAG T;`.** A base `T *B` is untyped
(`Refusal::UntypedBase`) only when the reader leaves `T` opaque: a typedef that
defines the record (`typedef struct [TAG] { … } T;`, which card
ABI-TYPEDEF-ADDRESS-OF-MEMBER records but deliberately classifies `Opaque`), a
typedef of a record pointer, or a name declared by disagreeing typedefs
(librdkafka declares many public types forward in `rdkafka.h` and defines them
in an internal header). The parent census's class "typedef of a record"
counted both; 70 of its 166 corpus leaves are over a base the reader
resolves, and their refusal lies elsewhere.

| route | reader | callee's release | leaves | functions | sole-blocker functions |
|---|---|---|---:|---:|---|
| corpus | untyped: the typedef defines the record | nullable | 35 | 29 | `mk_ptr_free`, `rd_kafka_mock_sgrp_record_release` |
| corpus | untyped: the typedef defines the record | nonnull | 8 | 6 | none |
| corpus | untyped: the typedef defines the record | unit | 24 | 14 | none |
| corpus | untyped: disagreeing typedefs | nullable | 2 | 1 | none (`rd_kafka_Node_destroy`, see below) |
| corpus | untyped: disagreeing typedefs | nonnull | 16 | 13 | none |
| corpus | untyped: disagreeing typedefs | unit | 11 | 4 | none |
| corpus | resolved (`typedef struct TAG T;`) | nullable | 38 | 23 | none |
| corpus | resolved | nonnull | 7 | 7 | `rd_kafka_offset_file_close` |
| corpus | resolved | unit | 25 | 20 | none |
| system | any | | 0 | 0 | |
| slice | any | | 0 | 0 | |
| current | untyped: the typedef defines the record | nullable | 37 | 31 | `mk_ptr_free`, `rd_kafka_mock_sgrp_record_release` |
| current | untyped: the typedef defines the record | unit | 8 | 6 | none |
| current | untyped: disagreeing typedefs | nullable | 2 | 1 | none (`rd_kafka_Node_destroy`) |
| current | resolved | any | 41 | 30 | `rd_kafka_offset_file_close` |

Corpus totals: 166 leaves in 107 functions; the reader leaves the base untyped
for 96 leaves in 57 functions. `current` (estimate): 88 leaves in 69
functions, 47 in 36 untyped. The `current` estimate counts more than the
parent's (18 in 16) because MEMBER-EFFECT-SUMMARY's selection reads only
written-out contracts and so missed every release through a wrapper declared
`contract inherit libc::free` (`rd_free`, `mk_mem_free`); `census.py` adds
the inherited positions (the signatures agree position by position). Every untyped-base leaf with a nullable
release has a `*`-spelled member (two `unit` leaves have a member of another
typedef), so "untyped base and member together" decides nothing on these
routes.

**The three bodies the parent named.** `mk_ptr_free` (`typedef struct { … }
mk_ptr_t;`) and `rd_kafka_mock_sgrp_record_release`
(`typedef struct rd_kafka_mock_sgrp_record_state_s { … } …_t;`) release a
`*`-spelled member through `mk_mem_free`/`rd_free`, which inherit
`libc::free` (nullable): they are this card's. `rd_kafka_offset_file_close`
is not: `rd_kafka_toppar_t` is declared once, forward
(`typedef struct rd_kafka_toppar_s rd_kafka_toppar_t;`), so the reader types
the base, and its release is `fclose`, whose contract releases a **non-null**
stream; rule 38j admits no `nonnull` member release whatever spells the base
(the body's `if (!rktp->rktp_offset_fp) return;` guard is not read as making
the position non-null). It stays refused (a follow-up below).

**Other blockers.** Of the 57 functions with an untyped-base leaf, the only
ones whose other blockers are a single kind and plausibly follow from the
refused release are rule-5k destructors whose other leaf is `unresolved
resource identity` of the base, which the refused member statement causes
(the fixture shapes below lose it once the member release is admitted):
`rd_kafka_Node_destroy` (disagreeing typedefs, two `rd_free`s),
`rd_kafka_mock_error_stack_destroy` and
`rd_kafka_share_ack_batch_entry_destroy` (each one `rd_free`). Their siblings
`rd_kafka_mock_sgrp_fetch_session_destroy` and `rd_kafka_assignor_destroy`
also release through `RD_IF_FREE(…, rd_kafka_topic_partition_list_destroy)`
(nonnull) or `rd_kafkap_str_destroy` (no provider contract), and the
remaining ones carry uncontracted calls, loop conditions, member stores or
callback premises (`census/corpus.json`, `other_blockers`).

## 2. The rule: Clang reads the base where the reader left it untyped

**[Rule 38j](../../agents/rules/38j.md), amended again in place** (no rule id
is added: ids 40, 41 and 42 are held by LINK-STANDING, SCALAR-STATEMENT-FORMS
and LOOP-REACQUISITION-LEAK-ACCUSATION). The call-argument rule's arm for the
parent's untyped member (`legacy_call_argument_access.rs::candidate_arguments`)
now also matches `Refusal::UntypedBase`, and asks
`legacy_member_effect.rs::Parser::typed_member_release` with `untyped_base`.
Admission needs, unchanged from the parent:

- the position is `Released { nullable: true }` of the contract in force;
- `(B, F)` is a place rule 5k's P1-P4 or rule 38j's P1-P3 admitted in this body
  (`AdmissibleReleases::admits`);
- Clang's actual, through parentheses, is an `LValueToRValue` read (never an
  `ArrayToPointerDecay`) of an arrow `MemberExpr` named `F`, of an object
  pointer type with no function type and no `volatile`/`_Atomic`, over a
  `DeclRefExpr` to the parameter `B` whose rvalue type spells no `union`;

and, for the base arm only:

- the base's rvalue type is a pointer (through any typedef chain: Clang has
  resolved it for `->`), no function type, no `volatile`/`_Atomic`;
- the member Clang resolved (`referencedMemberDecl`) is a member named `F` of
  a complete `struct` the unit defines (`RecordFacts::struct_members`). A
  `union`'s member (a union typedef, or an anonymous union inside the record,
  whose base is then no parameter), and an incomplete record (Clang resolves
  no member through it) are refused.

Two conditions are added for both arms, found by the fixtures:

- **The parameter's own qualifiers.** The rvalue read drops a qualifier of the
  parameter itself, so `r_t *volatile r` passed every check above and was
  `Safe`; the `DeclRefExpr`'s declared type must now spell no
  `volatile`/`_Atomic` either (fail-closed tightening, which also applies to
  the parent's arm).
- **The implicit conversion to the parameter type.** `free`, `rd_free` and
  `mk_mem_free` take `void *`, so Clang wraps the member's value in an
  implicit `BitCast` to `void *`, which the parent's peel refused: neither
  `mk_ptr_free` nor any rule-5k destructor over `rd_free` could be admitted.
  An implicit `BitCast` or `NoOp` to a plain object-pointer type is now peeled
  **outside** the value read (never an explicit cast, which the lexical
  recogniser refuses anyway, and never inside the read). It hands the same
  pointer value; the lexical reading of a `*`-spelled member never looked at
  the conversion either. This widens the parent's arm too; its only decided
  functions besides `cmt_opts_exit` release through
  `cfl_sds_destroy`/`flb_sds_destroy`, whose parameter has the member's own
  type, so nothing measured there changes.

The actual then reads as the lexical `B->F` does (base `B`, a pointer value,
no chain). Every lexical premise is unchanged, so a local base, a released
member used again, a second release, a release in a loop and a `nonnull`
release stay refused. Concurrent files: none of the waiting cards' files is
touched (`legacy_call_argument_access.rs` changes one match arm).

## 3. Fixtures and Lean

`crates/qcore-c-gateway/tests/member_release_untyped_base.rs` (Clang-backed,
every `Safe` certificate rechecked; real `libc/memory.qv`, `libc/stdio.qv`,
`cfl/sds_lifetime.qv`; `rd_free`/`mk_mem_free` with `libc::free`'s clauses
written out because the standalone loader does not resolve a provider
`#include`):

| fixture | shape | result |
|---|---|---|
| `the_corpus_bodies_with_a_typedef_spelled_base` | `mk_ptr_free` and `rd_kafka_mock_sgrp_record_release`, bodies verbatim | `Safe` |
| same | `rd_kafka_offset_file_close` verbatim, base `rd_kafka_toppar_t *` and `struct rd_kafka_toppar_s *` | `Unknown`, the rule-38i name (nonnull `fclose`) |
| `the_corpus_destructors_that_release_the_base_too` | `rd_kafka_Node_destroy` (disagreeing typedefs) with `rd_kafka_Node_free`; `rd_kafka_mock_error_stack_destroy`; `rd_kafka_share_ack_batch_entry_destroy` with its `_free` wrapper | `Safe`, wrappers too |
| `every_typedef_spelling_of_a_struct_pointer_base_is_admitted` | rule 5k over `r_t *`; a record-pointer typedef `rp_t r`; a typedef chain `r2_t *` with a `sds_t` member (base and member untyped together) | `Safe` |
| `the_base_reading_admits_only_a_complete_struct` | union-pointer typedef, union typedef, anonymous union member, incomplete record, array-typedef member, function-pointer member, volatile pointee, volatile pointer parameter | not `Safe` (the unions and array keep the rule-38i name; the incomplete record is rejected or refused) |
| `the_lexical_premises_still_decide` | local base (copied from the formal; from an allocator), member used again, two releases, parent released first, release in a loop, `mk_ptr_free`'s caller reading the released member | not `Safe` (the caller: `member-effect-call-refused`) |

`tests/member_effect_component_scope.rs`: the parent's `typedef_base` case,
which pinned a typedef-spelled base refused, is flipped deliberately to
`Safe`.

`proof/QCore/MemberReleaseUntypedBase.lean` (audited by
`proof/AuditMemberReleaseUntypedBase.lean`) extends
`QCore.MemberReleaseTypedMember`: `admitsBase` is rule 1 after this card, the
parent's `admits` before it (whose `an_untyped_base_is_never_admitted` stays
true of that earlier rule). Theorems: `a_previous_admission_is_kept`,
`a_new_admission_is_an_untyped_base`,
`an_admission_still_needs_the_place_and_a_nullable_release`,
`a_failed_place_check_is_refused`, `a_union_base_is_refused`,
`an_incomplete_record_is_refused`, `a_qualified_base_is_refused`,
`a_local_base_is_refused`, `a_decayed_array_is_refused`,
`a_function_pointer_is_refused`,
`an_admitted_actual_is_a_struct_pointer_member` (given the parent's
`LexicalSound`), `the_body_stays_sound` (rule 38j's `body_sound` unchanged)
and `the_unit_plays_no_part`. Premises stated, not proved (trust boundary
28): a `MemberExpr`'s `referencedMemberDecl` names a member of the record its
base points to, and its `type` is the member's declared type.

## 4. Gate

Card gate, `python3 scripts/gate_scope.py --base 7cdd33906 --out
private/gate-1` at `271b8ede3` (code as of `278f87412`), run once
([gate-run1.json](gate-run1.json)). Selection: the whole Rust suite
(`qcore-c-gateway` changes), all of Lean (a proof root changes), Python; no
provider gate. Every step exit 0 (`gate_scope: 8 steps passed`):

| step | exit | counts |
|---|---:|---|
| submodules | 0 | 1 submodule |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| Rust suite (`run_test_binaries.py`, rust-suite slot) | 0 | 618 units, 659 summaries; 9,297 passed; 0 failed; 16 ignored; 135 filtered |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| Lean (`check_lean_audits.py`) | 0 | 418 audits |
| Python (`unittest discover -s scripts/tests`) | 0 | 685 tests |

The Rust suite's 6,975 s include 5,749 s waiting for its shared slot (it ran
08:03:44-08:24:10 UTC). Logs: `private/gate-1/` in the card worktree
(per-unit logs under `rust-suite/`); console log
`/tmp/claude-1000/-media-Data-GitHub-quirin-lang--claude-worktrees-launch-cards-clean-worktrees-5365ff/6fcd7fbd-1cae-4a40-a37d-5bce071c02c6/scratchpad/gate-1.console.log`.

## 5. Measurement and follow-ups

Measured by the integrator:
`target/release/quirin-audit --plugin-path target/release --output <dir> --timeout-ms 36000000 measure --inputs crates/quirin-audit-plugin-measure/inputs/live.json --inputs-base /media/Data/GitHub/quirin-lang --side base=rev:662dc54a4 --side tip=rev:278f87412 --declared docs/reports/member-release-untyped-base/PREDICTION.md`.

Follow-ups (amendments in [integration-amendments.json](integration-amendments.json)):

- **W45-MEMBER-RELEASE-UNTYPED-DECLARATIONS**, amended: the base arm is
  discharged at a nullable release of an admitted place; unreadable
  declarations, the untyped base or member at every other position, and the
  shared reader's own resolution of defining, record-pointer and disagreeing
  typedefs stay open there. MEMBER-RELEASE-UNTYPED-BASE is registered and
  retired.
- Not registered, for the integrator's queue: a **non-null member release
  after a null guard** (`if (!B->F) return; fclose(B->F); B->F = NULL;`,
  `rd_kafka_offset_file_close` and the 31 nonnull leaves of the census, e.g.
  `RD_IF_FREE(B->F, rd_kafka_topic_partition_list_destroy)`): rule 38j admits
  only `Released { nullable: true }`; admitting `nonnull` where every path to
  the release has tested the place non-null is a new premise, a user-level
  choice.
- The parent's report §2 classes `rd_kafka_offset_file_close` as a
  typedef-base sole-blocker body and its `current` estimate omits releases
  through `contract inherit` wrappers; that report is frozen, and this
  report's §1 is the correction.

## Integrator measurement (2026-11-08)

Declared measurement, base `rev:662dc54a4`, tip `rev:278f87412`, run from the
main checkout (`private/measure/untyped-base-20261108T083424Z`): exit 0, all three routes
complete, no unmatched rows, declared result **match**.

- corpus: `rdkafka_aux.c::rd_kafka_Node_destroy` Unknown to Safe; `rdkafka_aux.c::rd_kafka_Node_free` Unknown to Safe; `rdkafka_mock.c::rd_kafka_mock_error_stack_destroy` Unknown to Safe; `rdkafka_mock_sharegrp.c::rd_kafka_mock_sgrp_record_release` Unknown to Safe; `rdkafka_share_acknowledgement.c::rd_kafka_share_ack_batch_entry_destroy` Unknown to Safe; `rdkafka_share_acknowledgement.c::rd_kafka_share_ack_batch_entry_destroy_free` Unknown to Safe; `mk_core/mk_memory.c::mk_ptr_free` Unknown to Safe; 12 rows changed content only.
- system: no row moved; 2 rows changed content only.
- slice: no row moved; 4 rows changed content only.

[Comparison](integration/native-comparison.json.gz) (gzip of
`native-comparison.json`, sha256
`da24baf43807a757b55ae8f704cc907152058e1f39c234d3b2916ab1401c7479`).

Base is MEMBER-EFFECT-COMPONENT-SCOPE's measured code tip, so the seven moves are this card's alone; MEMBER-EFFECT-COMPONENT-SCOPE's own measurement is recorded in its report.
