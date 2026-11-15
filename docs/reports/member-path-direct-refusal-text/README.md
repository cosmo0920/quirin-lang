# MEMBER-PATH-DIRECT-REFUSAL-TEXT

Base main `0f6e17118`. Ledger entry `MEMBER-PATH-DIRECT-REFUSAL-TEXT`
(recorded by SCHEDULER-DESTROY-MEMBER-STATES).

## Prediction

Committed before any code change; nothing had been built or measured from
this card.

The card changes only the text of the `member-path-direct` refusal of a
direct call (`callback_operands` in
`crates/qcore-c-gateway/src/legacy_callback_route.rs`) so that it names the
clause that makes the declaration non-plain. When the refusal fires, its code
and every verdict stay as they are.

- No status moves on any route (system, slice, corpus).
- Corpus: exactly one diagnostic's text changes, in `mk_http_thread_destroy`'s
  own row (`mk_server/mk_http_thread.c`, origin `mk_http_thread.c:269`, call
  to `mk_thread_destroy`). Its text changes from "beside a hand-off, which only
  a callback invocation composes" to text that names the member invocations
  of the formal. The code `member-path-direct`, the category `unsupported`,
  the status (Unknown) and every other diagnostic in that row stay the same.
  Hash-like fields that are derived from the binary (`build_identity`) may
  differ by rebuild.
- System and slice: no change. Neither has a Monkey scheduler unit, and no
  other declaration in force reaches this refusal: every other member-path
  declaration in force is plain or stands beside a hand-off, and the text for
  a hand-off is unchanged.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

## Change

`callback_operands` in `crates/qcore-c-gateway/src/legacy_callback_route.rs`
used to refuse every non-plain member-path declaration at a direct call with
one fixed text, "beside a hand-off". It now takes the text from a new
function, `member_path_direct_reason`, next to `member_path_plain`, which
names the clause beside the states:

| Declaration | Detail (after ``call to `F`:``) |
|---|---|
| a hand-off (unchanged) | it states what it leaves at member paths of a formal's object beside a hand-off, which only a callback invocation composes |
| member invocations (a helper destroyer, rule 50) | it states what it leaves at member paths of a formal's object beside member invocations, which no direct call composes |
| anything else (the loader admits nothing else; fail-closed wording) | it states what it leaves at member paths of a formal's object beside a clause other than a plain predicate, which no direct call composes |

The condition for firing is unchanged (`declaration.handoff.is_some() ||
!member_path_plain(declaration)`), and so are the code (`member-path-direct`,
`member_path_route::DIRECT`) and the category. The loader keeps a hand-off and
member invocations apart (`helper_invocations` requires no hand-off), so the
first two rows never overlap. The doc comment of `DIRECT` in
`legacy_member_path_route.rs` and contract specification §C04.19.6 ("A helper
destroyer") now say which clause the text names. No rule file quoted the old
text. Lean is not changed: the change is diagnostic text only, with no
semantic change.

Test: `a_direct_call_of_a_helper_with_an_invocation_beside_its_states_refuses`
(`crates/qcore-c-gateway/tests/member_value_helper.rs`) now pins the exact
refusal text and checks that it does not mention a hand-off.

## Measurement

**What was run.** The narrowest equivalent run, not `quirin-audit measure
--declared`. This host refuses user namespaces, so no `rev:` side can run.
The corpus preparations that earlier cards used (LERI's `corpus-inputs`) have
been deleted. Rebuilding the whole pinned corpus would mean two runs of about
87 minutes each, so instead the two sides ran `quirin-audit current` over the
row's own unit at the corpus's source revision:

- Sides: `git archive` exports of base `0f6e17118` and tip `8e735f30f` (this
  card's code commit) under `private/`, each built with `cargo build --release
  --locked -p qcore-audit`
  ([build-sides.sh](artifacts/build-sides.sh)). Binary sha256: base
  `667c1641…`, tip `edf717d8…`.
- Inputs: `quirin-audit prepare` at Fluent Bit `66910c10` (the corpus
  revision) from the local clone, with system Clang 21
  ([prepare.sh](artifacts/prepare.sh)). This gives the production compile
  database (`5ffa2b4a…`) and the forced-Libevent Monkey database
  (`1cccbd5e…`), the configuration the corpus's `monkey-libevent` units use.
- Run: [current.sh](artifacts/current.sh), with the fluent-bit profile, the
  run's member-facts pre-pass over the whole production database, `--jobs 8`
  and `SOURCE_DATE_EPOCH=1767225600`. The selection
  ([monkey-only-selection.json](artifacts/monkey-only-selection.json)) names
  one chunkio file, so the run analyses that file plus the 29 standalone Monkey
  units. Both sides exited 0. Outputs are in the worktree-private
  `measure/current-base-1` and `measure/current-tip-1`.
- Comparison: [compare_rows.py](artifacts/compare_rows.py), with its output in
  [comparison.txt](artifacts/comparison.txt) (exit 0, no unmatched row).

**Result.** 349 function rows on each side: 98 Safe, 249 Unknown, 2 Unsafe on
both. No row moved status. Two rows changed content:

- `monkey-libevent/lib/monkey/mk_server/mk_http_thread.c`,
  `mk_http_thread_destroy` (Unknown on both sides). This is the predicted
  change and the whole semantic difference:

  ```diff
  - call to `mk_thread_destroy`: it states what it leaves at member paths of a formal's object beside a hand-off, which only a callback invocation composes (member-path-direct)
  + call to `mk_thread_destroy`: it states what it leaves at member paths of a formal's object beside member invocations, which no direct call composes (member-path-direct)
  ```

  The diagnostic keeps its category (`unsupported`), its line (6138) and its
  origin (`lib/monkey/mk_server/mk_http_thread.c:269`). Every other diagnostic
  and every contract dependency in the row is the same on both sides.
- `mk_core/mk_rconf.c`, `mk_rconf_read` (Unknown on both sides). This change
  comes from the environment only. Its three `reading_site_evidence` entries
  are identical once `context.evidence_version` is removed. That field spells
  the build commit (`0f6e17118` against `8e735f30f`). The fields derived from
  it are also removed: `context.assumptions`, `context.build_identity`, the
  entry `id` and `observation.environment_sha256`.

**The prediction holds.** No status moved. The only semantic content change
is `mk_http_thread_destroy`'s `member-path-direct` text at
`mk_http_thread.c:269`, and its code is kept.

**What was not measured.** The pinned corpus, system and slice routes were not
run, and `measure --declared` was not run, for the reasons above. The system
and slice routes contain no Monkey unit. The run's paths differ from the
pinned corpus's (`lib/monkey/…` against `mk_server/…`), but the configuration
is the same: standalone Libevent Monkey at `66910c10`.

## Follow-ups

None new. `MEMBER-PATH-HELPER-AT-A-CALL` and `RULE50-CONSUMER-COMPOSITION`
remain open and may change whether this refusal fires at
`mk_http_thread.c:269` at all.
