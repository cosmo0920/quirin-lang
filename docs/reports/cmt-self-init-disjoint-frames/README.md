# Checked initializer frames for same-owner child slots

This card adds a bounded ownership frame to the checked CMetrics/CFL initializer
transport. A live, precise Heap owner can retain existing child pointer slots
when source-bound Clang layout and the core footprint checker prove each slot
separate from every initializer write. For an embedded initializer the proof
checks the whole first enclosing member, conservatively. No different-allocation
identity supplies a physical separation or allocator-freshness premise.

Semantic BASE is `47fc52a32abe8b5e56d711ff4773343ac6537167`; measured semantic
TIP is `342ca74c4a2ffca611dbc1b6ff51e0fe53920490`. The fresh BASE corpus and
bottom-up planner completed before prediction `acb13489c`, with a narrower call
closure clarification in `b593500d9`, both before implementation. The
[brief](BRIEF.md), [pre-implementation design](DESIGN.md) and
[unchanged prediction](PREDICTION.md) retain that ordering.

The bounded feature and its complete attribution are closed. Production Safe
gain and getter closure gain are both **0**, matching the prediction. The broader
frame follow-up and getter ownership chain remain Open.

The source and flow boundary
----------------------------

The original default-frame route runs first. The new optional source proof
requires unique record/member declarations and complete layout, direct pointer
member slots, compatible owner pointer declarations, and a whole-caller seal on
pointer assignments. Same-type plain copies, ordinary contracted Owned results
and known NULL are admitted; loaded/retyped/complex pointer results, pointer-slot
addresses, Out/Stored/Removed effects and indirect or nonordinary calls decline.
Only the exact checked initializer declaration is exempt from ordinary contract
closure. Declined optional probes restore both reading and consultation state
before retaining the original tracked instruction.

The alternate transfer requires exactly LIVE, nonnullable, precise Heap owner
and child identities. Current or previous precise identities are admitted;
older summary identities and every non-Heap kind refuse. Field links belong only
to this owner, and partial-link markers must be a subset. All ledger, nullable,
field-link and partial-link obligations survive. Existing protected direct-slot
facts through compatible identity aliases survive; other recognized member facts
are forgotten conservatively. Plain and derived bindings survive. Foreign-owner
graphs, containment, membership, collections, borrows, caller custody, sanctioned
out slots and every other nondefault custody field still refuse. A residue check
against the default state makes future unrecognized state fields refuse too.

The existing indirect-access check must produce no diagnostic or state change;
otherwise the old fallback applies. Existing B08 aggregate-store alias refusal
is unchanged: source stores through copied owners stay refused. Compatible-alias
preservation is exercised in explicit flow states. Neither ordinary nor callback
nor cross-unit initializer summary permission is exported.

Rust, Lean and declared contracts
--------------------------------

`legacy_tracked_self_init.rs` owns the optional source proof and frame transfer;
`legacy_transfer.rs` composes it after the old transfer. Optional instruction
metadata preserves old no-proof serialization. The new Lean module
`TrackedSelfInitDisjointFrame.lean` and its registered audit cover numeric layout
separation, enclosing-member subwrites, finite writes, preservation of the full
ledger/graphs/partial markers, aliases, conservative invalidation and refusals.
Twenty flow vectors and four core layout vectors bind the Rust controls to the
Lean model. The audit has 23 theorem checks and no project axioms or placeholders.
These are model statements, not a C refinement proof.

The canonical CFL `intrusive_list.qv` initializer mirrors these guards and
effects: 18 requires, 9 ensures and 1 quantifier, deliberately changing the
clause pin from 18 to 28. It remains outside interpreted resource/bounds views
and grants no resource-v1 call permission. Provider inventory synchronization
leaves inventory/load paths unchanged. Existing map, opts and setter executable
contract adoption remains Open.

Twelve added root tests cover real root/embedded/nested source positives,
independent certificate replay and forged metadata rejection, overlap,
historical retyping and hidden rebinding, unprovided/guarded interfaces, partial
escape, child leaks, double release and parent lifetime failure. The deep source
positive measures depth 253 and 328 KiB. Explicit flow controls exercise previous
and older identities, six non-Heap kinds, graph/partial preservation, alias facts,
conservative invalidation and future-state refusal. The probe rollback test
proves that an actual `proof_only` consultation under `sizeof` is restored.

Full standing and provider gate
-------------------------------

All 20 commands exit 0 on semantic TIP. Exact argv, start/end times, private
log paths and hashes are in [command-receipt.json](gate/command-receipt.json);
[counts.json](gate/counts.json) and [gate.json](gate/gate.json) are the measured
records. The actual worktree Clang recording is beside that gate. Rust tests
hold a genuine `rust-suite` reservation; Lean runs separately in its own slot.

| Command | Exit | Private log |
|---|---:|---|
| `git submodule update --init --recursive` | 0 | `private/cmt-disjoint-frames/gate-1/submodules.log` |
| `/usr/lib/llvm-21/bin/clang --version` | 0 | `private/cmt-disjoint-frames/gate-1/clang-version.log` |
| `python3 scripts/run_test_binaries.py --jobs 6 --log-dir /home/cosmo/.codex/worktrees/808d/quirin-lang/private/cmt-disjoint-frames/gate-1/rust-binaries` | 0 | `private/cmt-disjoint-frames/gate-1/rust-tests.log` |
| `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline --no-fail-fast` | 0 | `private/cmt-disjoint-frames/gate-1/provider-tests.log` |
| `python3 scripts/gate_slot.py lean --receipt /home/cosmo/.codex/worktrees/808d/quirin-lang/private/cmt-disjoint-frames/gate-1/lean-slot.json -- python3 scripts/check_lean_audits.py --jobs 6` | 0 | `private/cmt-disjoint-frames/gate-1/lean-audits.log` |
| `cargo fmt --all -- --check` | 0 | `private/cmt-disjoint-frames/gate-1/fmt.log` |
| `cargo clippy --all-targets -- -D warnings` | 0 | `private/cmt-disjoint-frames/gate-1/clippy.log` |
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | `private/cmt-disjoint-frames/gate-1/reevaluation-probe.log` |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | `private/cmt-disjoint-frames/gate-1/reevaluation-followup.log` |
| `python3 providers/contracts.py check` | 0 | `private/cmt-disjoint-frames/gate-1/contracts-check.log` |
| `cargo fmt --manifest-path providers/Cargo.toml --all -- --check` | 0 | `private/cmt-disjoint-frames/gate-1/provider-fmt.log` |
| `cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings` | 0 | `private/cmt-disjoint-frames/gate-1/provider-clippy.log` |
| `python3 -m unittest discover -s providers/tests` | 0 | `private/cmt-disjoint-frames/gate-1/provider-python.log` |
| `python3 providers/package.py build --output /home/cosmo/.codex/worktrees/808d/quirin-lang/private/cmt-disjoint-frames/gate-1/packages` | 0 | `private/cmt-disjoint-frames/gate-1/package-build.log` |
| `python3 providers/package.py verify /home/cosmo/.codex/worktrees/808d/quirin-lang/private/cmt-disjoint-frames/gate-1/packages/qcore.chunkio` | 0 | `private/cmt-disjoint-frames/gate-1/verify-chunkio.log` |
| `python3 providers/package.py verify /home/cosmo/.codex/worktrees/808d/quirin-lang/private/cmt-disjoint-frames/gate-1/packages/qcore.cmetrics` | 0 | `private/cmt-disjoint-frames/gate-1/verify-cmetrics.log` |
| `python3 providers/package.py verify /home/cosmo/.codex/worktrees/808d/quirin-lang/private/cmt-disjoint-frames/gate-1/packages/qcore.ctraces` | 0 | `private/cmt-disjoint-frames/gate-1/verify-ctraces.log` |
| `python3 providers/package.py verify /home/cosmo/.codex/worktrees/808d/quirin-lang/private/cmt-disjoint-frames/gate-1/packages/qcore.libevent` | 0 | `private/cmt-disjoint-frames/gate-1/verify-libevent.log` |
| `python3 providers/package.py verify /home/cosmo/.codex/worktrees/808d/quirin-lang/private/cmt-disjoint-frames/gate-1/packages/qcore.librdkafka` | 0 | `private/cmt-disjoint-frames/gate-1/verify-librdkafka.log` |
| `python3 -m unittest discover -s scripts/tests` | 0 | `private/cmt-disjoint-frames/gate-1/python-tests.log` |

Root Rust: 554 units, 567 summaries, 8,686 passed, 0 failed, 15 ignored.
Provider Rust: 133 passed, 0 failed, 1 ignored. Lean: 373 audits passed under
pinned 4.35.0. Root Python: 460 tests; provider Python: 25. All five packages
build and verify. Root Python ran after the structural matrix commit. A fresh final confirmation
ran `python3 -m unittest discover -s scripts/tests` on committed publication
`6280e080d`, after the final matrix/index updates: 460 tests, exit 0. The
[confirmation receipt](publication/confirmation/receipt.json) retains the exact
command, head, environment, private log path and hash; the full output is retained
beside it. Intentional negative-fixture messages are followed by the final OK.

[preliminary-attempts.json](gate/preliminary-attempts.json) retains the earlier
nonzero Lean and focused Rust attempts and their corrections. Lean failures
were syntax/decidability issues; focused Rust failures were fixture spelling,
an overly broad consultation assertion and a wrong test enum path. None was
hidden or used to weaken a diagnostic, test, theorem or audit. All subsequent
focused controls and the complete gate pass. Private logs live under
`private/cmt-disjoint-frames/`, primarily `gate-1/` and `focused-3/`.

Native results, context and evidence custody
-------------------------------------------

The immutable BASE binary has SHA-256
`077014346fe09d2127fd2c28ad48156fe5e467a63840f50afa932e1bcf477404`;
TIP is `fe0c5c50fa8e9df9e16897834f22fa75b0880f9f153842b8efed99314a060e57`.
Both routes use the same frozen 314-file external snapshot and the original
prepared source/database/link-map/package inputs, with three native workers and
a 7,168 MiB audit planning budget. No mirror or namespace wrapper is used.
The [binary/model identities](gate/source-identity.json) disclose that TIP embeds
the changed canonical declared model while the external snapshot retains its
BASE copy; this is not a Rust-only binary isolation experiment.

The [BASE receipt](baseline-receipt.json) records corpus 4,834 = 1,013 Safe /
3,812 Unknown / 9 provisional Unsafe. The planner has 2,189 candidate bodies,
baseline same/cross closure 6/8 and conditional ceiling 384. Its getter
11-caller ceiling is planning evidence only. Configured BASE system is 448 =
101 Safe / 343 Unknown / 4 provisional Unsafe; tail-pause slice is 404 =
57 Safe / 342 Unknown / 4 Pending / 1 provisional Unsafe. Those configured
sources are their actual prepared checkout and do not establish historical
source/provider conformance. Provisional Unsafe rows are not validated upstream
defects. [Configured BASE custody](base-configured-receipt.json) retains the
identities, scopes and stable before/after hashes.

All three paired routes, both planners and the final attribution chain exit 0.
The BASE/TIP counts are identical:

| Route | Rows | Safe | Unknown | Pending | Provisional Unsafe |
|---|---:|---:|---:|---:|---:|
| Pinned corpus | 4,834 | 1,013 | 3,812 | 0 | 9 |
| Configured system | 448 | 101 | 343 | 0 | 4 |
| Tail-pause slice | 404 | 57 | 342 | 4 | 1 |

All 5,686 resource rows retain their statuses, complete diagnostic lists and
contract dependencies. All independent NULL dictionaries are unchanged across
4,921 / 454 / 406 enumerated NULL rows. Outer/runtime documents, NULL summary
payloads, chunk contracts and source effects are identical as complete parsed
documents. The complete planner difference list is empty. The six
selected CMetrics targets remain Unknown with own diagnostic counts 6 / 4 / 14 /
20 / 21 / 1 in the order named in the prediction. Complete target rows are in
[production-targets.json](native/production-targets.json).

Attribution retains every raw artifact and raw leaf difference before any
reconstruction. It validates all existing source modules, environments,
observations and captures, and independently rebinds each complete reading
receipt on both semantic versions. Exact Context evidence-version coordinates
are reconstructed only after checking the complete context shape. Two empty
lifetime-summary keys per route are the same duplicated source/configuration
identity: an exact-command fresh preprocess reproduces both keys by replacing
only `MONKEY_BUILT` date/time literals within the actual measurement windows.
Wrong-time and wrong-date controls reject. No nonempty summary or authority is
removed. Date rollover in corpus is explicitly reproduced, not normalized away.

| Route | Complete reading receipts | Canonical captures | Evidence-version coordinates | Remaining semantic leaves |
|---|---:|---:|---:|---:|
| Corpus | 122 | 45 | 18,855 | 0 |
| System | 24 | 4 | 2,154 | 0 |
| Slice | 2 | 1 | 1,572 | 0 |

All six BASE/TIP production projection replays reproduce the complete outer
projection, NULL tally fields, NULL row custody fields and production joins.
Wrong-custody-status and unprovided-NULL-premise controls reject on every replay.
Complete diagnostic counts are independently reconstructed. Measurement
provenance remains separately retained; fields erased = 0. The final
[completion](native/completion.json) and
[semantic review](native/semantic-review-reconstruction.json) discharge the
intermediate collectors' historical pending accounting labels. There is no
unrecognized remaining leaf and no real added consultation to explain away.

Published result context is `(artifact, property, semantic profile, scope,
bounds, assumptions, evidence version)`: artifacts are the retained per-function
resource/NULL/outer rows, property `resource-custody.v1` plus the separately
computed NULL and combined projections, semantic profile is the recorded legacy
pinned resource profile or configured profile digest
`37309028f4fa1d50682829be2cbca880931a1a65d57b47d1fccb78a9b74965f3`, scope is the
selected pinned/configured compilation configurations, bounds are the existing
gateway budgets with no new hypothesis discharge, assumptions are the complete
retained contract/Context assumption sets, and evidence version is
`contract-hypothesis-graph.v1:<semantic BASE or TIP>` with exact per-row source
and build identities. The raw artifacts retain every complete context rather
than substituting one context for all rows. Pinned Fluent Bit source is
`66910c10a4d7eafa810b84229cf2dcf7b0f26f97`; actual system checkout is
`046accf6aded7906f66a1baf0202da349e521736`, slice checkout
`34155955e4962a56d94eb23c03a002880899a0d1`. Configured historical source/native/
provider evidence stays Unknown, and profile pins are historical reference only.

Source binding remains Open and truth transport false. Clang extraction,
embedding, C bounds/no undefined behaviour, existing declared contracts and
Rust/Lean correspondence remain reviewed trust premises. Neither the model
proofs nor these observations establish whole-C or whole-Fluent-Bit safety.

[native/receipt.json](native/receipt.json) records every native command and
before/after identity. [native/manifest.json](native/manifest.json) covers raw
compressed artifacts, complete differences, replay specifications/results,
negative controls, exact source reconstruction, logs and scripts. Native and
replay private logs remain under `private/cmt-disjoint-frames/`; the corpus log
is `tip-corpus/audit.log`, configured logs `tip-system/` and `tip-slice/`, and
attribution command logs `attribution-1/` with per-replay logs in `replay-v3/`.
Corpus elapsed BASE 6,228.318 seconds, TIP 9,938.934 seconds on the uncontrolled
shared host; no speed or memory improvement is claimed. The 7,168 MiB figure is
a planning budget, not an RSS bound. All final input before/after hashes are
stable. Preliminary gate failures remain disclosed separately above.

The broader `W48-CMT-SELF-INIT-DISJOINT-FRAMES` and
`CMT-MAP-METRIC-GET-OWNERSHIP-CHAIN` stay Open: foreign-owner frames,
containment/member/collection relations, deeper or same-member paths,
loaded/retyped pointer source forms, nonordinary calls, B08 source aliases,
contract adoption, hash/cache/borrow/label effects and the other getter
obligations remain. The [tracking amendment](integration-amendments.json) qualifies the earlier
all-nondefault refusal claim with this bounded exception without retiring either
entry. A separate follow-on
is to investigate/document S3 pointer-type spelling sensitivity: the focused
fixture rejects `ptr<mut,u8>` while canonical `ptr<mut, u8>` is accepted. No
parser change is included here.
