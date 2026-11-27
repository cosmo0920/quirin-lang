# SCALAR-STATEMENT-FORMS prediction

Committed after the census (`ed3753a6e`) and before the semantic change.
**Base side: `baseline:7b726587bdf6a8213f451fa1c78297bb5e476ef7`** (published
2026-11-07); the affected rows below are relisted from its corpus, system and
slice reports.

**What changes.** Gateway rule 41 reads, from the Clang node, a statement or
`for` clause every older route refused with one of the five refusals, when
every value is a non-pointer scalar: steps and compound assignments of a
scalar object or typed path, several scalar declarators, a type-only
declaration, a braced scalar initializer, an unevaluated `sizeof`. Pointer
steps, pointer arithmetic, `volatile`, aggregate, inline-record, variably
modified, static and shadowed objects keep their refusal byte for byte. The
lowering is `ForgetScalar` or the typed reader's `IndirectScalarAccess` with
its per-hop obligations, so no custody, binding or owed set changes; a newly
read store through a released base is reported (`use_after_release`).

**How the rows were chosen.** A function moves only when every leaf it has
is one rule 41 admits, or a leaf the refused statement's frame caused (the
`unresolved resource identity` of a name its refused writes poisoned).
Candidates: the baseline rows whose leaves are only the five families (25 in
the corpus, 4 in system, 4 in slice) or the five families plus such identity
leaves (12 in the corpus, none in system or slice), each checked against the
census statement and against the census driver run with and without the rule
(`QCORE_GATEWAY_RULES_OFF=41`; the driver loads no contracts, so a libc call
stays refused there: `XXH32_canonicalFromHash`'s `memcpy`,
`rd_kafka_version_str`'s `snprintf`).

**Corpus (eight-home), predicted: nine rows Unknown to Safe.**

| Row | Leaf removed | Form |
|---|---|---|
| `librdkafka::lz4.c::LZ4_stream_t_alignment` | braced (lz4.c:1505) | type declaration `typedef struct {…} t_a;` |
| `librdkafka::lz4hc.c::LZ4_streamHC_t_alignment` | braced (lz4hc.c:930) | type declaration |
| `librdkafka::lz4frame.c::LZ4F_returnErrorCode` | braced (lz4frame.c:311) | static-assert `enum` |
| `librdkafka::rdxxhash.c::XXH32_canonicalFromHash` | braced (rdxxhash.c:567) | static-assert `enum` |
| `librdkafka::rdxxhash.c::XXH64_canonicalFromHash` | braced (rdxxhash.c:1020) | static-assert `enum` |
| `librdkafka::lz4.c::LZ4_getIndexOnHash` | braced, and the two identities its frame poisoned | static-assert `enum` |
| `librdkafka::lz4hc.c::LZ4HC_rotatePattern` | sizeof (lz4hc.c:169) | unevaluated `sizeof(pattern)` |
| `librdkafka::rdkafka.c::rd_kafka_version_str` | assignment (rdkafka.c:6586) | `size_t of = 0, r;` |
| `monkey-libevent::mk_server/mk_scheduler.c::_next_target` | assignment (mk_scheduler.c:60) | `unsigned long long tmp = 0, cur = 0;` |

No unit status moves: every unit holding one of them keeps other `Unknown`
rows (lz4.c 45, lz4frame.c 40, lz4hc.c 41, rdkafka.c 127, rdxxhash.c 14,
mk_scheduler.c 21). Callers of the moved functions lose their `call to
uncontracted function` leaf (`LZ4F_returnErrorCode` in thirteen lz4frame.c
rows, `LZ4HC_rotatePattern` in `LZ4HC_InsertAndGetWiderMatch`, `_next_target`
in `mk_sched_next_target`) and keep their status: each has another leaf.
Two of them have only an identity leaf besides the call
(`LZ4F_getBlockSize`: `LZ4F_max64KB`; `LZ4F_headerSize`: `src`); the driver
keeps both identities, so they are predicted to stay `Unknown`. The 16 other
sole-family rows keep their refusal by name (pointer-element stores,
compound literals and initializer lists with pointers or calls, a string
pointer, `volatile`, pointer difference, a cast pointer store, a `for`-clause
comma, and `rd_kafka_offset2str`'s `- -2000`). No row is predicted to become
`Unsafe`: the driver surfaced no new finding in the 103 units.

**System and slice, predicted: no status move.** Their sole-family rows are
compound literals holding pointers (`update_logs_tag_records_untracked`,
`flb_input_ingestion_paused`, `flb_input_ingestion_resumed`) and a `sizeof` in
pointer arithmetic (`flb_output_coro_add`'s `FLB_CORO_DATA`); none moves.
Leaves inside still-`Unknown` rows may be removed in all three routes
(content changes, not moves).

```measure-prediction
route corpus
move librdkafka::lz4.c::LZ4_stream_t_alignment Unknown -> Safe
move librdkafka::lz4hc.c::LZ4_streamHC_t_alignment Unknown -> Safe
move librdkafka::lz4frame.c::LZ4F_returnErrorCode Unknown -> Safe
move librdkafka::rdxxhash.c::XXH32_canonicalFromHash Unknown -> Safe
move librdkafka::rdxxhash.c::XXH64_canonicalFromHash Unknown -> Safe
move librdkafka::lz4.c::LZ4_getIndexOnHash Unknown -> Safe
move librdkafka::lz4hc.c::LZ4HC_rotatePattern Unknown -> Safe
move librdkafka::rdkafka.c::rd_kafka_version_str Unknown -> Safe
move monkey-libevent::mk_server/mk_scheduler.c::_next_target Unknown -> Safe
delta Safe +9
route system
none
route slice
none
```

**The user's `current` run** (Fluent Bit `046accf6`): the same nine rows
move Unknown to Safe (each is in the current report with the same leaves);
`LZ4F_returnErrorCode`, `LZ4HC_rotatePattern` and `_next_target` leave the
uncontracted-callee list; no function leaves the "only uncontracted" set; no
unit status changes. Of the 1,741 five-family leaves, the driver comparison
removes 245, in 93 functions the report lists (an estimate: the driver loads no contracts, so a
statement with a contracted call in the run may be decided differently).
