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
| current | untyped: the typedef defines the record | nullable | 1 | 1 | none (`cJSON_Delete`) |
| current | resolved | any | 17 | 15 | `rd_kafka_offset_file_close` |

Corpus totals: 166 leaves in 107 functions; the reader leaves the base untyped
for 96 leaves in 57 functions. Every untyped-base leaf with a nullable
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
