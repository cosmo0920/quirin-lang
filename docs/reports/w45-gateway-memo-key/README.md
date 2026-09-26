# W45-GATEWAY-MEMO-KEY: no cached analysis answer is served on a hash match alone

Card W45-GATEWAY-MEMO-KEY, 2026-09-26. A soundness fix in the legacy-C gateway,
found by W45-AUDIT-CHECKPOINTS's independence audit.

**Base.** Local `main` at `2f9131a8` (W45-CONTRACT-REVISION-MODE integrated),
as the brief named. `main` moved to `e90de196` (W45-FB-BACKLOG-CUSTODY) while
this card was running. That change adds no memo or cache to the four crates,
so the card was not rebased.

**Commits.**

* `891902e1`: code, tests, Lean and harnesses. The arbiter binaries were
  built from this commit.
* `cf26c45f`: the records. The gate was measured here.
* `d05a4b4e`: rustfmt and the matrix regeneration.
* Records follow.

## 1. The defect

`member_releasing_functions` (`crates/qcore-c-gateway/src/legacy_member_release.rs`,
W45-CALLEE-SUMMARY-CASCADE) returns the functions of a token stream whose body
may release an object through a formal. The unit-callee guard is inserted
before every call to one of them, so this set decides where a caller's actual
that owns sub-custody is refused. W4 builds many CFGs from one unit's tokens,
so the answer is memoised in a one-entry `thread_local!` per thread.

The entry was keyed by a 64-bit `DefaultHasher` digest of the token count and
every token text, and it was served when the digest matched, without looking
at the tokens. Two streams with one digest would hand one unit the other's
set. A missing name drops that function's guard, and the refusal it would
have made becomes an admission: a hash match turned into a verdict, which is
not fail-closed. The memo is per thread and outlives a unit, so under
`--jobs N` the pairs that could collide depend on how units are scheduled onto
threads. That also breaks the job-count identity (W44-AUDIT-PARALLEL-UNITS)
and a checkpoint's independence from the units before it. No collision was
observed; the defect is that nothing excluded one.

## 2. The fix

The entry (`ReleasingEntry`) now keeps **every token text** of the stream it
answers, beside the digest and the answer. `ReleasingEntry::answers` serves it
only when the digest, the length and every text are equal; anything else is a
miss, which computes the answer and replaces the entry. The digest stays only
as a cheap reject. The key is the whole input: `compute_member_releasing_functions`,
`definition_name`, `matching` and `call_argument_spans` read `Token::text`
only, never `line`.

Two hooks are for measurement only. `inject_releasing_entry` (`#[cfg(test)]`)
files an arbitrary entry under an arbitrary digest. The Cargo feature
`member-release-memo-off`, off by default and never enabled by any workspace
member (the convention of `indirect-census`), compiles the memo out, so every
call computes. That gives the cache-off corpus run. No rule text changes: no
gateway constraint described the memo.

## 3. Cache inventory (criterion 2)

Swept `crates/qcore-c-gateway`, `crates/qcore-audit`, `crates/qcore-contract`
and `crates/qcore-core` (sources and tests) for `thread_local!`, `RefCell`,
`Cell`, `OnceCell`/`OnceLock`/`LazyLock`, `static` items, `DefaultHasher` and
any other `Hasher`/`.hash(`, `HashMap`/`HashSet`, `Rc`/`Arc` held in a table,
integer-keyed maps, and the words memo/cache/intern. Each table that stores
state and serves it again is listed below.

| # | Where | What it holds | Keyed on | Hit verified? | Action |
|---|---|---|---|---|---|
| 1 | gateway `legacy_member_release.rs` `RELEASING` (`thread_local!`, one entry per thread, lives across units) | the member-releasing function set of a token stream | **was** a 64-bit `DefaultHasher` digest of the token count and texts | **was no** (digest alone) | **Fixed**: key is every token text; a hit needs the digest and every text equal |
| 2 | gateway `legacy_member_release.rs` `guard_unit_callees` locals (`releasing`, `declared`, `guarded: BTreeMap<String, bool>`) | lazy per-call reads and each callee's guard decision | the callee name (full string); one CFG, one token stream, one contract set | yes (exact `BTreeMap` key) | none |
| 3 | gateway `clang.rs` `RESOLVED` (`static Mutex<Option<(ResolutionKey, PathBuf)>>`, per process) | the resolved Clang executable | the whole `ResolutionKey` (`QCORE_CLANG`, `PATH`) | yes (`==` on the key); a failure is never cached | none |
| 4 | gateway `legacy_counted_users.rs` `Unit::{derived, verified, consulted}` (`RefCell<BTreeMap<String, _>>`) | callee effects, verifications and consulted clause records | the callee name; one `check` call with fixed tokens and contracts | yes (exact key) | none |
| 5 | contract `contract_inclusion.rs` `Elaborator::done` (`BTreeMap<String, Rc<Elaborated>>`) | elaborated modules | the `crate:module` string, exact; not the version or SHA-256 | a hit is on an exact key, not a digest; the key determines one text only because one elaborator uses one resolver (`LoadPathResolver`: one crate per name; `RetainedResolver`: one module per key, checked against its SHA-256 on resolution) | none; see §8 |
| 6 | contract `contract_inclusion.rs` `Resolution::resolved` (`BTreeMap<usize, _>`) | each own declaration's resolved clauses and chain | the declaration's index within one module | yes (exact key) | none |
| 7 | contract `contract_inclusion.rs` `RetainedResolver::used` (`RefCell<BTreeSet<String>>`) | the retained modules served, to refuse an unused one | — | not a cache: it never serves an answer | none |
| 8 | gateway `lib.rs` `test_support::CLANG_AVAILABILITY` and `CLANG_WARNING`; `qcore-audit/tests/*` `CLANG`, `CLANG_AVAILABILITY` (`OnceLock`, `Once`) | whether Clang-backed tests can run | no key: one answer per test process | n/a (test-only, no analysis answer) | none |
| 9 | gateway census and witness sinks: `static Mutex<Option<Option<BufWriter<File>>>>` in `indirect_census.rs` (5), `indirect_chain_census.rs`, `sub_object_store_census.rs`, `destination_suffix_census.rs`, `condition_call_census.rs`, `traversal_census.rs`, `provenance_rounds.rs`, `legacy_leak_witness.rs`, `legacy_return_family.rs`, `legacy_call_argument_access.rs`, `legacy_parenthesised_callee.rs`, `legacy_callee_scope.rs` | an output stream opened once from an environment variable | — | not a cache: written, never read back | none |
| 10 | atomic counters: gateway `indirect_census::UNIT_SEQ`, `provenance_rounds::SEQ`, `legacy_leak_witness::INVOCATION`; audit `wait_failures`, `observed_wait`, `cycle18_inputs` `NEXT`; test `NEXT`s | sequence numbers and scratch names | — | not a cache | none |

Nothing else matched. `qcore-core` holds no memo: its `BTreeMap<u64, _>` and
`BTreeSet<u64>` (`capacity_boundary`, `task_execution/reference`,
`composed_tasks`, `inference`) are keyed by model identifiers (epochs,
allocation and buffer ids), not digests. The same holds for
`descriptor_observation::Replay::origins` in `qcore-contract`, keyed by an
observed pointer value. `qcore-audit` has no memo table, no `DefaultHasher`
and no `Hash` use. Its SHA-256 digests are recorded and compared as evidence,
never used as a lookup key that serves a stored answer. Only row 1 served on
a hash alone, and it is fixed.

## 4. Tests

In `crates/qcore-c-gateway/tests/unit/legacy_member_release.rs`, run by
`cargo test -p qcore-c-gateway --lib member_release` (22 pass, 3 new):

* `a_forced_digest_collision_is_a_miss_and_gives_the_uncached_answer`. The
  hook files, under the stream's own digest, a key of the same length that
  differs in the last text, holding a set that would drop every guard
  (`{"bogus"}`). The call returns the uncached answer, and so does a shorter
  key under the same digest.
* `the_hook_reaches_the_memo_so_the_collision_test_is_not_vacuous`. The whole
  key under its own digest is served whatever it holds, so the injected entry
  is the one consulted. The whole key under another digest is a miss. This
  test is compiled out under `member-release-memo-off`.
* `interleaved_streams_always_get_their_uncached_answer`. Two streams with
  different answers, alternated four times, each called twice per round.

**Mutations.** [`scripts/mutate_memo_key.py`](scripts/mutate_memo_key.py)
restores the source by bytes and asserts the round trip. Per-mutation logs are
in [`mutations/`](mutations/). **2 of 2 killed**: the replaced rule (digest
alone) and digest plus length without a text comparison. Each fails the
collision test; the restored file is byte-identical.

## 5. Lean

`QCore.MemoKey` ([`proof/QCore/MemoKey.lean`](../../../proof/QCore/MemoKey.lean),
audited in `proof/AuditMemoKey.lean`, 8 theorems) models a one-entry memo
generic in the key, the answer, the uncached function `f` and the digest, and
assumes nothing of the digest. The results:

* `a_hit_has_an_equal_key`.
* `a_colliding_entry_is_a_miss`.
* `a_mismatched_entry_gives_the_uncached_answer`, which needs no invariant, so
  it covers the test hook's arbitrary entry.
* Under the memo's own invariant (`Filled`), `call_eq_uncached` and
  `call_keeps_filled`.
* **`run_eq_map`**: any sequence of calls through the memo is `List.map f`, for
  every digest, a constant one included. This is observational equivalence
  with the uncached function.
* `the_digest_alone_serves_a_collision`: a witness that the replaced rule
  answers key 1 with key 0's answer.

That the Rust key is the whole input of the uncached function is a review
premise (trust boundary 28), pinned by the tests above.

## 6. Corpus identity and timing

Prediction: no verdict, leaf or artifact byte moves, because the memo is
observationally the uncached function and no collision was known. This was
not committed separately before implementation. The card cannot move a figure
unless a collision had occurred on the corpus, and the cache-off run below
rules that out directly.

The arbiter (`run_audit.py`, `--jobs 8`) ran three binaries: **base** (main
`2f9131a8`, sha256 `7981c726…`), **fix** (`891902e1`, `a0cc996f…`) and **off**
(`891902e1` with `member-release-memo-off`, `581fbe0b…`). Each ran three times,
interleaved. [`identity/comparison.json`](identity/comparison.json) is
`compare_runs.py`'s output, with base-1 as the reference:

* All nine runs exit 0.
* All five raw artifacts of every run are **byte-identical** to base-1.
* Every `audit.log` is identical once its own output path is replaced.

So fix = base (no movement), and **cache on = cache off**, byte for byte.

**Against the named baseline** (`docs/reports/w45-fb-retry-lifetime/integration/artifacts`,
`baseline_identity.py --root-alias <worktree>=…/hungry-thompson-1740da
--additive-field contracts:contract_revision`), the fix run exits 0 with
`verdict_bearing_identical: true`. The diagnostics, chunk-contracts and
monkey-source artifacts are byte-identical
([`identity/baseline-identity-fix.json`](identity/baseline-identity-fix.json)).
Two sidecars differ, and **base's own run reports exactly the same**
([`identity/baseline-identity-base.json`](identity/baseline-identity-base.json)):

* The contracts sidecar differs by the additive `contract_revision` key and by
  the libc texts that W45-CONTRACT-REVISION-MODE marked flexible.
* The source-effects sidecar differs in 53 leaves of 3 import results: build,
  configuration and task digests, and the obligation ids derived from them
  ([`identity/source-effects-difference.txt`](identity/source-effects-difference.txt)).

These differences come from main, not from this card.

**Timing.** The host was shared with other running cards, with a load
average of 15–35 on 16 CPUs, so the wall times scatter widely.

* **`run_audit.py` wall time** (`comparison.json`), seconds per run:

  | Binary | Runs | Median |
  |---|---|---:|
  | base | 127.5 / 135.0 / 109.8 | 127.5 |
  | fix | 123.8 / 188.8 / 117.7 | 123.8 |
  | off | 117.7 / 112.7 / 119.6 | 117.7 |

* **A second interleaved set** records the child's CPU time, Clang included
  ([`scripts/time_variants.py`](scripts/time_variants.py),
  [`identity/cpu-timing.json`](identity/cpu-timing.json)). Its 5 artifacts
  are again identical across all 9 runs. User CPU seconds:

  | Binary | Runs | Median |
  |---|---|---:|
  | base | 1,067 / 922 / 1,138 | 1,067 |
  | fix | 1,107 / 927 / 1,063 | 1,063 |
  | off | 969 / 965 / 1,098 | 969 |

  Within a round, fix minus base is +40, +5 and −75 s, a mean of −10 s
  (about −1%).

**The verification's cost is below what this host can resolve.** It is
bounded above by the scatter, about ±7% of CPU time. By construction it costs
one comparison of the token texts per hit (the length of the digest pass) and
one copy of the texts per miss (at most once per stream and thread). The memo
itself saves nothing measurable either: off minus fix is −138, +38 and +35 s,
with no consistent sign. That is disclosed, not acted on. Removing the memo is
out of scope, since the brief asks to fix it.

## 7. Gate

The standing gate ran at `cf26c45f` with `CARGO_INCREMENTAL=0`. Every exit was
captured to a file ([`gate.json`](gate.json); logs in the worktree's
`private/memo-key/gate/`). Results:

* Submodule 0, clippy 0, both reevaluation examples 0, docs index `--check` 0.
* **Lean 0: 199 audits.**
* **Rust exit 101: 302 units, 312 summaries, 5,842 passed / 1 failed / 11
  ignored** (`private/test-binaries/20260926T102652Z-1909017/`). The one
  failure is constraint 16's load-sensitive row
  `vp04_vp07_vp08_process_lifecycle_and_payload_budgets`, which exceeded its
  500 ms budget in the exclusive phase under a host load of 15–35.
* **fmt exit 1**: rustfmt layout of the new test.
* **Python exit 1**: the committed matrix's derived `lean_audits` was 198
  against 199 on disk.

Repaired at `d05a4b4e`: rustfmt applied, and the matrix regenerated without
`--gate`, so its measured numbers stay carried and labelled as carried. At
that tip:

* fmt 0.
* Python 0 (358 tests).
* clippy 0.
* The module's unit tests 0 (22 passed).
* The load-sensitive row alone passed 3/3, unchanged.

The provider gate did not run, because `providers/` is untouched.

## 8. Ledger and follow-ups

* **Discharged, to retire at integration:** `W45-GATEWAY-MEMBER-RELEASE-MEMO-KEY`.
  Its discharge condition is "compare the token stream (or a cryptographic
  digest of it) on a hit, with a test", which §§2 and 4 meet. The name is **not
  registered on this card's base**: W45-AUDIT-CHECKPOINTS registers it on its
  own branch, which is not yet merged. Removing it here would be a no-op. The
  integrator retires it once both cards are on `main`, naming this card as the
  one that discharged it.
* No follow-up is registered. Row 5 of §3 is not hash-keyed, and its key
  determines one text within its single-resolver scope. A defensive SHA-256
  comparison on a hit is possible but not needed by any path found. It becomes
  needed only if one elaborator is ever given modules from more than one
  resolver.

## 9. Boundaries

* The fix is about key identity. It does not change what the set means or how
  the guard uses it (gateway constraint 5k, W45-CALLEE-SUMMARY-CASCADE).
* The Lean model proves that a memo returning only on key equality is the
  uncached function. It does not prove that the Rust key is the whole input
  (trust boundary 28); the tests pin that.
* The sweep covers the four named crates at `2f9131a8`. Tables that other
  running cards add (W45-AUDIT-CHECKPOINTS, W45-USER-CONTRACT-PLUGINS) are
  outside it.
