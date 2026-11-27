# W48-BU-RK-RESULT-CONSTRUCTORS — validation pending

Rule 37 is proposed for checked, non-VLA `sizeof` mentions in constructor
ownership scans and checked non-null allocator-wrapper results. Rust/Lean implementation is prepared; execution of its gates is pending. No TIP gains are claimed.

The own fresh BASE corpus passed (audit and input round-trip exit 0):
999 Safe, 3,826 Unknown, 9 provisional Unsafe custody rows out of 4,834.
Configured BASE system and slice runs also passed, with stable before/after identities and the same held BASE binary:
system 99 Safe / 0 Pending / 345 Unknown / 4 provisional Unsafe out of 448;
slice 57 Safe / 4 Pending / 342 Unknown / 1 provisional Unsafe out of 404.
These are selected property observations, not whole-program safety coverage.

Read the pre-implementation [prediction](PREDICTION.md),
[exact BASE/planner receipts](BASE-PLANNER-RECEIPT.json),
[completed three-route BASE receipt](BASE-COMPLETE-RECEIPT.json),
[three-source allocator review](allocator-source-review.json), and
[failed/preflight attempts](setup-attempts.json). Full standing/provider gates,
matching audited Lean, fresh TIP measurements and per-body/leaf attribution
remain required before this card can close.

The first focused run recorded17passes/1failure: the enabled assertion fixture
needed a declared filename for `__FILE__`. The source-identity guard refused it
correctly; `fd6a0e15c` supplies an actual source file. Focused retry, audited Lean,
the full standing/provider gates and fresh TIP remain pending.

The filename-corrected retry ran three controls: two passed and the positive
constructor stayed Unknown (exit 101). Its default strict C11 dialect selects
glibc's unread conditional-expression assertion. The pinned `rdkafka_admin.c`
command uses GNU17. `90e72fd58` matches that dialect in the fixture and retains
strict C11 as a fourth refusal control (19 focused controls in total).
No analyzer rule was broadened. Retry2 was cancelled while still waiting for
the lock (exit 143, no validation started) so `532a930fb` could scope the two
real-glibc controls to Linux GNU targets. Retry3 passed all four controls
(0 failures, 0 ignored), then Lean4.35.0 rejected an unsolved Option equality
in the wrapper theorem. `ff2f55698` supplies an explicit rewrite without changing
the statement or runtime code. Retry4 starts with the Lean companion and then
the full root/provider gates; full-gate counts and TIP remain pending.

The repaired Lean file passes a bounded single-file elaboration (exit 0, no
warnings; `private/w48-rk-constructors/lean-repair-preflight.json`). This uses
cached imports and does not replace the queued build, theorem audit or full
standing gate.
