# CG22-CALLBACK-CANDIDATE-LINK-PREMISE: the twelve-home link premise

Card CG22-CALLBACK-CANDIDATE-LINK-PREMISE (launched by the user 2026-11-18),
follow-up of [CG22-CALLBACK-CANDIDATE-CONTRACTS](../cg22-callback-candidate-contracts/README.md).
Base `974434463` (main). The integrator chose the discharge: extend the stated
link premise as a **new evaluation profile**, `twelve-home`, without touching
`eight-home` or any other pinned profile. The card is **data only**: no
analyzer, rule or Lean change (rule 4c's import already reaches a callback
position; `a_candidate_from_another_unit_is_in_force_only_under_its_import`).
The [prediction](PREDICTION.md) was committed (`60323ff46`) before the
profile, its map or any test existed.

## 1. The premise, for approval

Rule 4c imports an external home's validated W4 summary only under a stated
linked-program premise: every external call to a listed symbol has a
compatible function type and calling convention and resolves to the listed
definition in the actual link and interposition environment. The source audit
cannot establish that; a human states it. `twelve-home` states **the
eight-home premise unchanged (same text, same eight symbols) plus four
symbols**:

| symbol | stated home | handed to |
| --- | --- | --- |
| `rd_kafka_metadata_broker_cmp` | `librdkafka/rdkafka_metadata.c` | `bsearch` in `rd_kafka_Node_new_from_brokers` |
| `rd_kafka_metadata_broker_internal_cmp` | `librdkafka/rdkafka_metadata.c` | `bsearch` in `rd_kafka_Node_new_from_brokers`, `ut_populate_internal_topic_metadata` |
| `rd_strcmp3` | `librdkafka/rdstring.c` | `bsearch` in `rd_kafka_topic_assignment_state_rack_search`, `rd_kafka_partition_internal_find_rack` |
| `rd_kafka_assignor_topic_cmp` | `librdkafka/rdkafka_assignor.c` | `qsort` in `rd_kafka_roundrobin_assignor_assign_cb` |

The registering card's unmeasured variant (`link-map.eleven-home.json`)
omitted the fourth; the roundrobin leaf needs it, hence twelve homes.

**Data.** [`link-map.json`](../../../contract/evaluations/twelve-home/link-map.json)
(sha256 `d338e379…`, written by [`twelve_home.py`](artifacts/twelve_home.py)
from the eight-home map) is what the route reads.
[`link-premise.json`](../../../contract/evaluations/twelve-home/link-premise.json)
(written by [`link_premise.py`](artifacts/link_premise.py)) states the
premise for review: the map it extends (by digest), the four added homes,
and per symbol (all twelve) what one static link records.

**Link evidence.** `quirin-audit prepare clang-build --link` (this card's
preparation, [prep.sh](artifacts/prep.sh): base binary, system Clang 21, GNU
ld 2.46, Fluent Bit `66910c10`) relinked the `fluent-bit` executable with a
map and cross-reference table (`relinked_equal: true`) and wrote a
`quirin-link-resolution/1` artifact (sha256 `cc70e14a…`) over the same
compile database the corpus route reads (`0b9c765b…`). For each of the
twelve symbols rule 40's checks **derive** the stated home: one strong
global, default-visibility function definition, in the executable (an
`librdkafka.a` member for the four added: `rdkafka_metadata.c.o`,
`rdstring.c.o`, `rdkafka_assignor.c.o`), the only linked definition, no
dynamic relocation against the name, and referenced by the callers' objects
(`rdkafka_aux.c.o`, `rdkafka_range_assignor.c.o`,
`rdkafka_sticky_assignor.c.o`, `rdkafka_roundrobin_assignor.c.o`, ...). This
supports the premise; it does not replace it. It does not establish the
call-site type or calling convention, it covers one link on this host, and
the executable is linked with `--export-dynamic`, so run-time interposition
by a preloaded object is outside it. The pinned corpus route reads only the
stated map (rule 40 is the `current` route's).

**Approval asked:** that every external call to these four symbols in the
audited Fluent Bit link resolves to the listed librdkafka definition. Without
it the profile should not land; the eight-home figures are unaffected either
way.

## 2. The profile

`contract/evaluations/twelve-home/evaluation.json` (live) is `eight-home`'s
record with its own name, description, link map and premise file as tracked
inputs; route, source, untracked inputs and contract set are the same
([make_profile.py](artifacts/make_profile.py)). Wiring, so the policy holds:

* `crates/quirin-audit-plugin-measure/inputs/live.json` gains route
  `twelve-home` (the corpus route under the twelve-home map), because
  `land.py publish` refuses a live profile it did not measure;
* `scripts/tests/test_evaluation_profiles.py` pins four live profiles;
* `contract/evaluations/bindings.json` binds the profile to the matrix corpus
  key `conditional_twelve_home_callback_comparators`, which the **integrator
  adds** ([integration/matrix_corpus_row.py](integration/matrix_corpus_row.py)):
  `evaluation_profiles.py check` refuses an unbound profile, the amendments
  schema has no corpus-row operation, and a card does not edit the matrix;
* `docs/development/contributions.md` re-rendered (one more profile section);
  `docs/development/audit-subcommand-plugins.md` names the profile.

**Baseline.** The first baseline is this card's measured tip figures (base
`974434463`), on its **worktree-private preparation**, not the
`build/cycle19` preparation the untracked inputs name (that preparation is
not on this host; the eight-home baseline is equally stale here: 4,834 rows
at `556460e31` against 4,704 on this preparation). The history entry says so;
a landing measurement on the recorded inputs re-records it.

## 3. Measurement

`quirin-audit measure` with the base export's host and plugin
([arbiter.sh](artifacts/arbiter.sh), `--no-slots --jobs 8`), corpus route,
both inputs files differing only in the link map
([inputs.eight-home.json](artifacts/inputs.eight-home.json),
[inputs.twelve-home.json](artifacts/inputs.twelve-home.json)).

* **BEFORE** `measure/before`: eight-home map, sides `base` and `again` (both
  built in place from `private/base-src`, binary `492d3ec1…`), exit 0, wall
  5,848 s. Both **1,083 / 6 / 3,615** (4,704 rows, 179 units; null 2,476 /
  2,303 / 17 of 4,796), no move, no content change. The comparison is marked
  incomplete only for `composition-conflict`: the two directory sides spell
  different working directories, which the pinned route digests into every
  row's `hypothesis_context.build_identity`; the rows are otherwise equal
  ([record](artifacts/native-comparison-before.json.gz)).
* **Arbiter** `measure/arbiter-1`: twelve-home map, `base=run:measure/before/sides/base`
  against `tip=private/base-src` rebuilt with `--build-commit 974434463`
  (binary `d6dc7720…`; the same source, only the embedded identity differs:
  the BEFORE build read the enclosing worktree's `HEAD`, `60323ff46`), exit 0,
  route **complete**, no unmatched row, no conflict, **declared result
  `match`**, tip wall 3,468 s.
  [Record](artifacts/native-comparison-arbiter-1.json.gz) (gzip of
  `native-comparison.json`, sha256 `f0ea930c…`), [rows](artifacts/changes-arbiter-1.txt).

| map | functions Safe / Unsafe / Unknown | units | null | `callback-uncontracted` (leaves / functions / alone) |
| --- | --- | --- | --- | --- |
| eight-home | 1,083 / 6 / 3,615 | 9 / 6 / 160 / 4 data-only | 2,476 / 2,303 / 17 | 50 / 48 / 17 |
| twelve-home | **1,083 / 6 / 3,615** | unchanged | unchanged (digest equal) | **44 / 44 / 17** |

**Against the prediction.** (1) No status move: confirmed, the row, unit and
null verdict digests are equal. (2) All six leaves discharged, including the
two uncertain ones (`rd_strcmp3`, `rd_kafka_assignor_topic_cmp`); each row
now lists the imported summary and the comparator's null premises
(`null-deref:<cmp>:…:a`, `:b`). (3) Traded leaves, as predicted in kind:
three of the six invocations now refuse **`arguments do not match resource
contract for bsearch`** (`rdkafka_aux.c:269`, `:274`,
`rdkafka_assignor.c:867`), and `ut_populate_internal_topic_metadata` gains a
`retention-undecided` leaf on `rd_list_add` that the refusal masked. (4)/(5)
No `Safe -> non-Safe`, no `Unsafe` added or removed; home rows unchanged.

**Each caller's remaining blockers** (tip leaves):

| caller | leaves | what remains |
| --- | --- | --- |
| `rd_kafka_Node_new_from_brokers` | 6 | 2 `bsearch` argument mismatches, 2 indirect accesses outside the projection, unresolved `broker`, `broker_internal` |
| `ut_populate_internal_topic_metadata` | 18 | `bsearch` argument mismatch, `rd_list_add`/`rd_list_destroy` registry undecided, `rd_list_deduplicate` uncontracted, unresolved identities, loop re-acquisition, ... |
| `rd_kafka_topic_assignment_state_rack_search` | 2 | unresolved `partition_racks`, unmodeled call in the return expression |
| `rd_kafka_partition_internal_find_rack` | 1 | unmodeled call in the return expression (the `bsearch` result) |
| `rd_kafka_roundrobin_assignor_assign_cb` | 8 | the other `qsort` (`rd_kafka_group_member_cmp`, Unknown in its unit), `rd_kafka_topic_partition_list_add` uncontracted, unresolved identities, ... |

**Other content changes (20 rows, all Unknown -> Unknown, no leaf change
except one).** `rd_kafkap_leader_discovery_set_brokers` (`qsort` with both
metadata comparators, not a census leaf) trades two out-of-projection leaves
for four unresolved identities and lists both imported summaries;
`verifyNumPartitionsWithRackMismatch0` lists the internal comparator's
summary and premises with its leaves unchanged. The other rows change only
their rule 13f `contract_dependencies`: in the four caller units now holding
home replays,
`rd_calloc` (and `rd_strdup` in `rdkafka_sticky_assignor.c`) move from
`declarations` to `withdrawn` (`C06.3-no-body`), and rows that call
`rd_strcmp` or `rd_kafka_metadata_destroy` list them as consulted summaries
with their leaves unchanged. These are the home units' replay records
written into the caller's recording (rule 4c), attributed to the caller's
rows; the labels move in the conservative direction and no verdict reads
them. Follow-up `CROSS-UNIT-REPLAY-DEPENDENCY-ATTRIBUTION`.

**Routes not measured: system and slice.** Not prepared here; neither route
has a `callback-uncontracted` leaf, and the profile is a corpus route.

## 4. Tests

`crates/qcore-audit/tests/link_premise_profile.rs` (Clang; rule 4c's own
`LinkMap::select` over the **committed** twelve-home map, synthetic comparator
bodies and the gateway's `callbacks.qv` `c_each`): the map is the eight-home
premise plus exactly the four homes; `rd_strcmp3` exported from its stated
home is in force and the caller is `Safe` and re-certifies; **a comparator
the map does not name (`rd_kafka_group_member_cmp`, exported Safe) is never
imported and its invocation keeps `callback-uncontracted`**; a listed
comparator exported from another unit is not imported either.
`crates/quirin-audit-plugin-measure/tests/units.rs`
(`the_twelve_home_profile_is_the_corpus_route_under_its_own_link_map`): the
adapter reads the profile as the eight-home route on the same inputs under its
own map, and `live.json` measures it.

## 5. What remains

* **Approval of the premise** (section 1) and the integrator's matrix row
  (`integration/matrix_corpus_row.py`): until the row exists,
  `evaluation_profiles.py check` refuses, so the Python suite fails on this
  branch (section 6).
* **First landing after this one**: `land.py publish` verifies the previous
  published baseline route by route; that baseline has no `twelve-home`
  route, so the first publication needs a bootstrap of that route. Every
  live-routes measurement now runs a fourth corpus route (about an hour here)
  unless `--routes` narrows it. Follow-up `EVALUATION-PROFILES-ADDED-PROFILE`.
* **The callers' other blockers** (table above), including the newly visible
  `bsearch` argument mismatch; `rd_kafka_partition_internal_find_rack` is one
  leaf from decided. Follow-up `CG22-COMPARATOR-CALLER-RESIDUALS`.
* The 41 candidates Unknown in their own unit stay with
  CG22-CALLBACK-CANDIDATE-CONTRACTS.

## 6. Gate

See [gate/gate.json](gate/gate.json) once recorded; the scoped run and its
known failures are stated there and in the card's reply.
