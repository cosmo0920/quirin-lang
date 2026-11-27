# LINK-RECORD-KILL-POLICY-SCALAR-CALLS: link records survive scalar-only calls

Card LINK-RECORD-KILL-POLICY-SCALAR-CALLS (the `flb_errno` follow-up of
LERI-HELD-FORMAL-LINK-OBLIGATION; user decision 2026-11-27). Base
`fb4adb697` (r83). Gateway rule **46** amended in place (its kill policy,
`link_member::reach`, which rule 91's carried records share); no new rule
id. Lean `QCore.ScalarCallKillPolicy`. Commits: probe `c03e9b792`,
[prediction](PREDICTION.md) `b3a45ca61`, semantic `b3e09974e` (first
measured tip), probe `c1e6b1669`, prediction amendment `f823446b4`,
semantic `b160cb304` (the measured tip).

## Headline

* **`flb_output_task_queue_enqueue` moves Unknown → Safe** on the
  fluent-bit-system route, both configurations (116 / 0 / 328 / 4 →
  118 / 0 / 326 / 4), as predicted. Its caller
  `flb_output_task_singleplex_enqueue` trades leaves and stays Unsafe (its
  own counted-user refutation). No other row of the system route or of r83's
  census units moved or changed a leaf; the census units read 34 / 1 / 165 /
  0 on both sides.
* **The first semantic commit moved nothing.** r83 read the blocker as
  `flb_errno()`; a debug trace of the measured row (section 4) showed the
  carried record already forgotten one statement earlier, at
  `queued_task = flb_malloc(sizeof (struct flb_task_enqueued))`, whose
  `sizeof` actual was undecided. At that tip `flb_errno_print` itself was
  already decided. The second semantic commit reads a whole call-free
  `sizeof ( … )` as a scalar, and the row moves.

## 1. Mechanism

Rule 46's kill policy forgets a link record (rule 46's `link_ready`
initialisation facts, rule 91's unspent `Ready`/`Owed` carried records) at
every statement that may write the object. Before this card a contract call
kept records only when it had no nested call and every actual was a place
(and only those of objects no place's root binding may denote); any nested
call also forgot every record before the call.

Each actual of a contract call now has a kind (`Kind`,
`legacy_link_member.rs`):

* **place**: the call may write what its root binding denotes (unchanged);
* **scalar**: it denotes no object. A literal or predefined identifier
  (premise T6, restated in `legacy_link_member.rs` because `variadic_nested.rs` is a pinned reading-site artifact; the reader
  spells a string literal as one identifier-shaped token), an admitted
  dereference of a neutral nested call's scalar result (premises N4-D and
  N10: `(*__errno_location ())`), or exactly `sizeof ( … )` whose operand
  carries no call, assignment, increment or decrement (`is_sizeof_actual`;
  C11 6.5.3.4p2: unevaluated unless variably modified, and a call-free,
  assignment-free operand writes nothing either way). Recorded at
  classification in `LegacyInstruction::ContractCall::scalar_actuals`;
* **undecided**: anything else (a cast, an address, an arithmetic
  expression, `sizeof (T) + p`, a scalar-typed name with no pointer binding,
  an undereferenced nested result, a nested actual read as data).

A call is **decided** when no actual is undecided and every nested call's
actuals are places; it then reaches what its places, its nested calls'
places and its written place may denote (`call_reach`). Otherwise it
reaches every object, as before. Before the call's own effect its nested
calls forget the records of what their places may denote, or every record
when one of their actuals is not a place (`nested_reach`,
`before_nested`), replacing the unconditional nested-call kill.

**The global-write premise.** A decided call keeps the records of objects no
actual denotes, so it relies on a contracted callee writing nothing it is
not handed (no write through a global that reaches the object). That is the
premise the old policy's contract-call arm already relied on (`q_use(other)`
and every zero-argument contracted call kept every unrelated record), and
the brief names "contracted" as establishing it; this card states it in
rule 46 and in the Lean frame premise instead of adding a new fact. A
`ContractCall` exists only for a callee with a contract in force; an
uncontracted callee is an `Unknown` instruction, which reaches everything.
No schema clause states "writes no global reaching its arguments' objects"
separately; see section 6.

**Hook points in shared files:** `legacy_classify.rs` (one call computing
`scalar_actuals` before the `ContractCall` is built), `legacy_c.rs` (the new
`scalar_actuals` field, `#[serde(default, skip_serializing_if =
"Vec::is_empty")]`), `legacy_transfer.rs` (the nested-call kill block
replaced by one `before_nested` call; the `ContractCall` destructuring
pattern names the field), and one `scalar_actuals: Vec::new()` line in each
constructor of `legacy_owned_out_member.rs`, `legacy_member_handoff.rs`,
`legacy_joined_provenance.rs` (two, tests), `tests/contract_dependencies.rs`,
`tests/unit/audit_ir_fragments.rs` and `tests/unit/legacy_c.rs`.

## 2. Controls

`crates/qcore-c-gateway/tests/link_member_premises.rs`, module
`carried_link_obligation`, `kill_policy_fixtures` and the r83 fixture
`errno-before-the-destroyer`. Base verdicts pinned by `c03e9b792` (at
`fb4adb697`) and `c1e6b1669` (the sizeof fixtures, at `b3e09974e`'s reading).

| Fixture | Pinned | Tip |
|---|---|---|
| r83 `errno-before-the-destroyer` (`q_note(*q_errloc(), "f.c", 1)`) | Unknown | **Safe** |
| carried, glibc spelling `q_note((*q_errloc()), "f.c", 1)` | Unknown | **Safe** |
| carried, literals only `q_note(0, "f.c", 12)` | Unknown | **Safe** |
| carried, predefined identifier `q_note(0, __func__, 12)` | Unknown | **Safe** |
| held (own `mk_list_init`), errno report | Unknown | **Safe** |
| carried, `q_try_alloc_n(sizeof(struct holder))` then the errno report (the measured shape) | Unknown | **Safe** |
| held, `q_try_alloc_n(sizeof(struct holder))` | Unknown | **Safe** |
| carried, `q_use(q_errloc())` (undereferenced pointer result) | Unknown | Unknown (held refusal) |
| carried / held, `q_note(*q_field(r), …)` (the nested call is handed the object) | Unknown | Unknown (held refusal) |
| carried, `q_note(k, …)` (scalar name, no pointer binding) | Unknown | Unknown (held refusal) |
| carried, `q_note(1 + 2, …)`, `q_note(-1, …)` | Unknown | Unknown (held refusal) |
| carried, `q_use((void *) r)` | Unknown | Unknown |
| carried / held, uncontracted `q_unknown(1)` | Unknown | Unknown |
| carried, `q_try_alloc_n(k * sizeof(…))`, `q_try_alloc_n(sizeof(…) + k)` | Unknown | Unknown (held refusal) |

No fixture is accused. Every r83 and rule 46 fixture keeps its verdict.
Unit tests: `KILL_VECTORS` (`decided_before` and `decided`), the refinement
check over them, and `only_a_whole_call_free_sizeof_is_scalar` (four
admitted, ten refused spellings).

## 3. Lean

`proof/QCore/ScalarCallKillPolicy.lean`, audited by
`proof/AuditScalarCallKillPolicy.lean` (9 theorems; no placeholder, no
project axiom). Under the frame premise (a contracted call writes only what
its actuals, its nested calls' actuals and its written place may denote; a
scalar denotes nothing, an undecided actual anything) the refined reading
over-approximates the call's writes (`the_refined_policy_is_sound`) and the
nested calls' reading theirs (`the_nested_kill_is_sound`), so rule 46's facts
and rule 91's ready records it keeps still hold
(`the_refined_policy_keeps_every_fact`,
`the_refined_policy_keeps_every_ready_record`, through
`QCore.ElementLinkPremises.retaining_preserves_every_fact` and
`QCore.CarriedLinkObligation.retaining_keeps_every_ready_record`). Every
call the old policy decided is decided with the same reach
(`every_old_reach_is_kept`); a call with an undecided actual or a nested
call with a non-place actual keeps its kill
(`a_pointer_carrying_call_keeps_its_kill`); a kill is removed only when no
actual is undecided, every nested actual is a place, and the call has a
nested call or a scalar actual (`a_kill_is_removed_only_for_scalar_calls`).
`the_errno_report_is_decided`; `vectors_hold` equals `KILL_VECTORS`. The
classification of actuals (T6, N10, the `sizeof` shape), the frame premise
and the Rust correspondence are trust boundary 28's.

## 4. Measurement

**Routes.** The fluent-bit-system route and r83's census units
(`flb_input.c`, `tail_file.c`, `tail_fs_inotify.c`), each run by r83's
[system.sh](../leri-held-exit-class-precondition/scripts/system.sh) and
[census-units.sh](../leri-held-exit-class-precondition/scripts/census-units.sh)
from its revision's own `git archive` tree and release binary
([scripts/side.sh](scripts/side.sh)) over this host's preparation
`private/fb-046accf6`; and the opt-in `current --monkey-linked` route
([scripts/optin.sh](scripts/optin.sh), [scripts/opt-in-current.sh](scripts/opt-in-current.sh):
FB-RETRY-OR6-ELEMENT-IDENTITY's script with the archived
`opt_in_providers.py` path). Base binary `f45e2da2…`, first tip
`60080c0a…`, tip `a6313719…`. Raw reports in `private/m/{sys,census,optin}-{base,tip,tip2}`
(all exit 0; digests in [artifacts/digests.txt](artifacts/digests.txt);
the opt-in runs were launched directly, not through `scripts/gate_slot.py`,
and an earlier base opt-in attempt that failed on the moved
`opt_in_providers.py` path is kept as `private/m/optin-base-attempt1-wrong-path`,
not evidence); row diffs by [scripts/diffrows.py](scripts/diffrows.py), keyed by
unit index (the system report holds two units per file, one per
configuration; r83's `moved.py` keys by file and counts one).

| Route | Base | First tip `b3e09974e` | Tip `b160cb304` |
|---|---|---|---|
| system | 116 / 0 / 328 / 4 | 116 / 0 / 328 / 4, no leaf changed | **118 / 0 / 326 / 4** |
| census units | 34 / 1 / 165 / 0 | unchanged, no leaf changed | unchanged, no leaf changed |
| opt-in current | 1205 / 2 / 3590 / 7 | not measured (started, stopped once the tip superseded it) | 1205 / 2 / 3590 / 7, no leaf changed ([artifacts/optin-base-tip2.txt](artifacts/optin-base-tip2.txt)) |

**Every moved row** ([artifacts/sys-base-tip2.txt](artifacts/sys-base-tip2.txt)):

* `flb_output_task_queue_enqueue` (`flb_output.c`), units 13 and 14:
  Unknown → Safe; its three leaves (`unresolved resource identity retry`
  twice and the held refusal at `flb_task_retry_destroy`) go. Justification:
  r83's exit-class summary (class `-1` an element destroyer of `retry`
  through `_head`, class `0` `Consumed` beside `resource_links_valid(retry,
  _head)`) now validates, because the entry's `Ready(_head)` record reaches
  the destroyer: `flb_malloc(sizeof (struct flb_task_enqueued))` is decided
  (one `sizeof` scalar, destination a plain name), `flb_errno_print((*__errno_location
  ()), "…", 293)` is decided (three scalars, one nested call with no actual),
  and the branches write nothing. The custody reading moved; the combined
  null reading did not (custody 118 / 326 / 4, combined 76 / 374 / 4 on both
  sides: the row's null candidates are unchanged).
* `flb_output_task_singleplex_enqueue`, units 13 and 14: Unsafe → Unsafe,
  `call to uncontracted function flb_output_task_queue_enqueue` traded for
  the precondition refusal (`requires retry's _head links valid on entry,
  which this function does not show`), the held refusal and an unresolved
  identity at the same call: its `retry` is its own formal, which its
  declared contract does not carry with `_head`. Its Unsafe is its own
  `resource_user_exit(task, -1, restored)` refutation, untouched; it is a
  provisional result and is not presented as an upstream defect.

No Safe → Unknown, no accusation added or removed.

**The first tip's miss.** [artifacts/enqueue-trace-tip1.txt](artifacts/enqueue-trace-tip1.txt)
is the carried-record trace of the enqueue at `b3e09974e` (a private copy
of that tip with debug prints, [scripts/dbg.sh](scripts/dbg.sh)): `{2:
Ready("_head")}` before `flb_malloc`, `{}` after it, so the record was gone
before `flb_errno_print` (whose `scalar_actuals` were `[0, 1, 2]`). r83's
fixture allocated with `q_try_alloc()`, which has no actual.

**Prediction against measured.** Predicted (frozen at `b3a45ca61`, kept by
the amendment `f823446b4` for the second tip): system
`flb_output_task_queue_enqueue` Unknown → Safe in both configurations,
Safe +2, Unknown −2, the singleplex leaf trade; census none; opt-in none.
Measured at `b160cb304`: exactly that on system and census; opt-in
none, as predicted. At the first tip the prediction failed (nothing moved), for
the reason above. Pinned routes (`quirin-audit measure --declared`) cannot
run on this host (no published live-route inputs); nothing is fabricated
for them.

## 5. Gate

GATE_SECTION

LLP64 fixture scan, as the brief spells it (`git diff --name-only
fb4adb697...HEAD -- '*tests*' | python3 docs/reports/archive/language-and-tooling/windows-ci-llp64-fixtures-2/scripts/llp64_fixture_scan.py
--stdin`): exit 0 (0 counted, 9 found). Two findings it counted at the
first attempt were pre-existing `unsigned long` parameters in files this
card touched only by one constructor line (`contract_dependencies.rs`'s
`malloc`, `tests/unit/legacy_c.rs`'s `q_realloc`); both are now spelled
`__SIZE_TYPE__`. `python3 scripts/check_report_hygiene.py`: HYGIENE.

## 6. Follow-ups

[integration-amendments.json](integration-amendments.json) amends
LERI-HELD-FORMAL-LINK-OBLIGATION: its nearest remainder (the enqueue) is
discharged; every other census row keeps the blocker r83 named (unmodelled
member stores into `ins`, uncontracted calls, refuted callees,
`flb_tail_file_remove`'s contract-pin mismatch), none of which this card
touches. It registers nothing new. Residuals recorded there: a scalar-typed
name actual (`q_note(k, …)`) and a size expression over names stay
undecided (deciding them needs the typed declaration facts and an integer
provenance premise); and the global-write premise is the contract-call
arm's existing one, not a schema fact. `profiles: []`.
