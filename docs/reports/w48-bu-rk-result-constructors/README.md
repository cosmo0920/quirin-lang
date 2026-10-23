# W48-BU-RK-RESULT-CONSTRUCTORS — validation pending

Rule 37 is proposed for checked, non-VLA `sizeof` mentions in constructor
ownership scans and checked non-null allocator-wrapper results. Rust/Lean implementation is prepared; execution of its gates is pending. No TIP gains are claimed.

The own fresh BASE corpus passed (audit and input round-trip exit 0):
999 Safe, 3,826 Unknown, 9 provisional Unsafe custody rows out of 4,834.
Configured BASE system and slice runs are continuing with the held BASE binary.
These are selected property observations, not whole-program safety coverage.

Read the pre-implementation [prediction](PREDICTION.md),
[exact BASE/planner receipts](BASE-PLANNER-RECEIPT.json),
[three-source allocator review](allocator-source-review.json), and
[failed/preflight attempts](setup-attempts.json). Full standing/provider gates,
matching audited Lean, fresh TIP measurements and per-body/leaf attribution
remain required before this card can close.
