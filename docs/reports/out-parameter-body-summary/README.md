# OUT-PARAMETER-BODY-SUMMARY

Round 26 card F. Base `532e03c97` (origin/main); measurement side
`baseline:7b726587bdf6a8213f451fa1c78297bb5e476ef7`. The user's `current` run
at Fluent Bit 046accf6 refuses 130 functions with
`store through `*X` without an out-parameter contract`. The card asked for a
census of those stores and for an out-parameter summary read off the body
wherever every path decides the slot.

Status: **two readings land; one shape stops on a semantic choice** (see
"Stop"). [PREDICTION.md](PREDICTION.md) was committed before the semantic
change (`c024796e2`).

## Census

Population: every leaf with that description on the user's report (130
functions, 243 leaves over all configurations; 127 distinct functions, 238
distinct sites). The corpus baseline has 126 rows (235 leaves), system 10
(16), slice 5 (10) ([`census/baseline-rows.json`](census/baseline-rows.json),
[`census/rows.py`](census/rows.py)). Each site's original line and its
function's definition were read lexically
([`census/sites.py`](census/sites.py), [`census/classify.py`](census/classify.py),
per site in [`census/census.json`](census/census.json)); a macro site is read
on the line the report names.

| Slot root | Pointee | What is stored | Sites | Functions | Sole-blocked |
|---|---|---|---:|---:|---:|
| formal | pointer | a fresh local (allocator or constructor result) | 52 | 48 | 4 |
| formal | pointer | NULL | 52 | 37 | 0 |
| macro local (`_vp` in `rd_kafka_buf_read_*`) | scalar | a call's value (`be64toh(_v)`, `_v`) | 50 | 7 | 0 |
| formal | enumeration | an enumerator or an enum value | 22 | 10 | 2 |
| formal | pointer | a borrowed interior or list element | 10 | 8 | 1 |
| formal | builtin scalar | `0`, a scalar local | 10 | 4 | 0 |
| local | scalar | `'\0'`, `0` through a local pointer or array | 10 | 10 | 0 |
| formal | pointer | a local or formal with no assignment seen | 7 | 7 | 0 |
| local | pointer | NULL | 6 | 3 | 0 |
| formal | pointer | another local | 5 | 4 | 0 |
| formal | pointer | a string literal | 4 | 3 | 0 |
| other (conditional, expression, record, opaque typedef) | | | 10 | 9 | 0 |
| **Total** | | | **238** | **127** | **7** |

Paths: 26 of the 127 functions return only integer literals (rule 11r(4)'s
shape); 40 return `rd_kafka_resp_err_t` enumerators and 4 `rd_kafka_error_t *`.
The fresh-pointer slots are typically NULL on failure or untouched on
failure; the census does not separate the two by path (a lexical reading).

Callers ([`census/callers.py`](census/callers.py),
[`census/callers.json`](census/callers.json)): 119 formal slots, 314 call
sites found, 146 in the defining file. The actual at the slot is `&local` at
176, `&member` at 54, a forwarded pointer at 20, NULL at 8 and another
expression at 56; 10 slots have no call site in the tree (API entry points).

The seven sole-blocked functions:

| Function | Shape | Outcome |
|---|---|---|
| `rd_kafka_message_timestamp` | `*tstype = RD_KAFKA_TIMESTAMP_NOT_AVAILABLE` (typedef enum) | part 1 admits; predicted `Safe` |
| `rd_kafka_toppar_get_avail` | four `*errp = RD_KAFKA_RESP_ERR__…` | part 1 admits; predicted `Safe` |
| `cio_utils_read_file`, `cio_file_content_copy`, `cio_memfs_content_copy` | `int`, fresh on `0`, unwritten on `-1`, then `*size = n` through a second formal | part 2's shape, held by the scalar frame (Stop) |
| `mk_vhost_get` | borrowed list elements into two slots | out of scope (follow-up) |
| `rd_kafka_consumer_group_metadata_write` | a possibly-null `rd_malloc` result | out of scope (follow-up) |

Why the enumeration stores were refused: in the same function
`*errp = rkt->rkt_err` already certified, because its `->` makes the lexical
classifier refuse the statement and the W46 AST reader (rule 10k, enum arm)
then admits it; `*errp = RD_KAFKA_RESP_ERR__X` looks like a pointer copy, is
lowered as an out-slot `Alias`, and never reached that reader. The lexical
reader keeps a typedef-defined enumeration `Opaque` by design.

## The readings

**Part 1, rule 10k(5): a scalar store named for an out slot.** After every
lexical route of the statement position declined, a `*B = V` lowered as an
out-slot `Alias`/`Clear` is read by the W46 statement reader and replaced
only by its scalar store (`IndirectScalarAccess`). It admits a store whose
destination type is a scalar, the owner-aware enum arm included; the store
keeps its base's dereference obligation (a released base is a
`use_after_release` candidate). It refuses, by keeping the out-slot reading:
a pointer store (`*pp = 0`, `*pp = q`), a reader refusal, a plan with no
access. A sanctioned slot is never written by the scalar route.
(`legacy_classify.rs`.)

**Part 2, rule 11r(6): an unwritten return-code class.** Rule 11r(4)
re-reads an `int` body with literal returns once per returned value. When the
ordinary route publishes nothing for a per-value body, the body is probed
with every stored-through formal an `Out` slot, and the slots `Unwritten` at
every reachable exit form `U`. The class contract reads each slot of `U` as
`resource_borrowed` (C04.13's unwritten slot beside `resource_out` in another
class) and is validated under the analysis contract (each slot of `U` `Out`,
so stores on the other classes' paths are sanctioned) by the W4 gate with
`Unwritten` demanded at every exit, then by rule 4b's re-check. The relation
stands only when each class's `U` is exactly its slots `Borrowed` there and
`Out` elsewhere. Rule 12s(2)'s row analyses each class and the whole body under
the analysis contracts (`row_entries_agree`). It admits: fresh on one code,
untouched on another, the caller testing the code where it stands. It
refuses: a slot written on some exit of the class and not another, a stored
site released, escaped, possibly-null or shared by two slots, an untested
call (rule 11r's join), a non-literal return, a non-`int` result.
(`legacy_unwritten_class.rs`, `legacy_summary.rs::observations_match_class`,
`legacy_relational_summary.rs`, `legacy_relational_forwarding.rs`,
`legacy_summary_premises.rs`.)

The caller side is rule 11r(2)'s, unchanged: on the unwritten class the
actual must be `&NAME`, and `NAME` then holds an untracked value, so a caller
that releases it there refuses (fixture below), and a caller whose variable
held a live resource that the success class overwrites reports the leak.

User policy of 2026-11-07 (rows whose verdict depends on C++ lifetime are
`Unknown`; no out-parameter summary from a C++ body): both readings run only
in the C gateway's C units; nothing here reads a C++ body.

## Fixtures and Lean

[`crates/qcore-c-gateway/tests/out_parameter_body_summary.rs`](../../../crates/qcore-c-gateway/tests/out_parameter_body_summary.rs),
13 tests, each asserting every function's status and complete diagnostic
vector: enumerator stores through typedef and tagged enumerations (`Safe`),
the `rd_kafka_toppar_get_avail` shape (`Safe`), a caller of an enumerator
writer (`Safe`), a store through a released base (`Unsafe`,
`use_after_release`), pointer stores (refused); fresh-on-success /
unwritten-on-failure with a testing caller (both `Safe`), a caller that drops
or overwrites the delivered resource (`Unsafe`, leak), a caller that releases
the untracked variable on the failure class and an untested call (`Unknown`),
a class written on one exit only, a store after release, two slots given one
site, the chunkio shape with the scalar store after the slot (all refused),
and the ordinary `Out(nullable)` summary unchanged.

Two older pins moved with part 2 (gate-1 failed on them, as designed):
`call_argument_access::an_unsanctioned_out_slot_store_is_withdrawn_by_the_hand_on_rule`
and `exposed_leak_mechanisms::mechanism_1_an_unsanctioned_out_slot_store_is_not_a_leak_claim`
pinned `int w(char **out, …) { … if (!p) return -1; *out = p; return 0; }`
as an unsanctioned store. That body now validates its own relation (`-1`
unwritten, `0` fresh) and its row is `Safe`; both mechanisms are re-pinned on
the `void` form of the same body, which no relation describes and which keeps
the refusal, and the first test also pins the `int` form's new `Safe`.

`QCore.OutParameterBodySummary` ([`proof/QCore/OutParameterBodySummary.lean`](../../../proof/QCore/OutParameterBodySummary.lean),
audited by `proof/AuditOutParameterBodySummary.lean`): `class_mode_sound`
(a class's published mode admits the body's effect on every exit of the
class), `unwritten_class_keeps`, `written_exit_refuses_unwritten`,
`other_exit_refuses`, `exit_in_its_class`,
`summary_effect_equals_body_on_every_path` (over a whole body, every exit's
effect is the one its value's class admits), `relation_requires_a_writer`,
`scalar_store_preserves_custody` and `released_base_refuses`. Premises stated,
not proved (trust boundary 28): the analyzer's exit observations are what the
body does on each path, `for (;;);` removes exactly the other classes' paths,
and Clang's reading of the destination type.

## Stop: a scalar store through a second formal

The chunkio shape stores the resource and then its size:
`*buf = data; *size = st.st_size;`. The scalar store's frame may write any
object `size` points to, including the caller's `char *` that `*buf`
designates, unless `buf` and `size` are known not to alias; the slot is then
`Ambiguous` at the success exit and the class does not validate (fixture
`a_scalar_store_through_a_second_formal_after_the_slot_stays_refused`). The
brief keeps aliasing between out formals refused, and fail-closed does not
settle whether a premise may discharge it. Options:

- **A (recommended).** Keep a pointer-typed out slot across a non-character
  scalar store through another formal under the listed premise
  `effective-type:` (C11 6.5p7: an lvalue of an integer or enumeration type
  cannot modify a pointer object), the premise rule 14m already lists for the
  null property, recorded in the row's assumptions. A character store
  (`*s = '\0'`) still poisons the slot. Frees the three chunkio rows on the
  corpus and on `current` (predicted, unmeasured).
- **B.** A caller-side precondition: the summary holds when the out formals
  designate distinct objects, discharged at a call whose actuals are `&` of
  distinct locals (176 of the 314 caller actuals are `&local`). No premise,
  but the callee's own row stays `Unknown`, and the precondition is a new
  clause with its own Lean model.
- **C.** Keep the refusal.

## Prediction and measurement

[PREDICTION.md](PREDICTION.md): corpus `rd_kafka_message_timestamp` and
`rd_kafka_toppar_get_avail` `Unknown -> Safe` (`delta Safe +2`); system and
slice none; on `current` the same two functions `Safe`, no unit status
change, no callee leaving the uncontracted list. The integrator runs the
declared measurement:

```sh
target/release/quirin-audit --plugin-path target/release --output <dir> --timeout-ms 36000000 measure \
  --inputs crates/quirin-audit-plugin-measure/inputs/live.json --inputs-base /media/Data/GitHub/quirin-lang \
  --side base=baseline:7b726587bdf6a8213f451fa1c78297bb5e476ef7 --side tip=rev:41d1b48a5 \
  --declared docs/reports/out-parameter-body-summary/PREDICTION.md
```

Overlap checked: `CG19-ATOMIC-FORMAL-VALUE-STORE-OUT-PARAMETER` is the
`_Atomic` spelling the older routes' pointer test refuses; it has 0 corpus
leaves and no census site spells `_Atomic`, so it is untouched and stays
open.

## Gate

`python3 scripts/gate_scope.py --base 532e03c97 --out <worktree>/private/gate-N`
(full mode: the diff reaches `qcore-c-gateway`; Lean all, a proof root
changed; no provider gate).

- gate-1 (tip `f636992a7`): exit 1. Rust suite exit 101, 616 units, 9,289
  passed, 2 failed (the two re-pinned mechanisms above), 16 ignored; every
  other step exit 0 (fmt, Clippy, both reevaluation probes, Lean 415 audits,
  Python 685 tests). Record [gate-1/gate.json](gate-1/gate.json).
- gate-2 (tip `61c58db35`): **exit 0**. Submodules, fmt, Clippy, Rust suite
  (616 units, 9,291 passed, 0 failed, 16 ignored, 135 filtered), both
  reevaluation probes, Lean (415 audits), Python (685 tests). Record
  [gate.json](gate.json).
