# W44-CALLEE-CROSS-UNIT-SUMMARY

Card base `fa04aab0fd43b242d2a996da2c6c507f1eb2a113`; prediction
[`PREDICTION.md`](PREDICTION.md) committed first at `d92349a56`.

## Route boundary found before implementation

The W4 inference (`qcore-c-gateway::legacy_summary`) derives validated
summaries inside one preprocessed translation unit. Its `ResourceSummaryRecord`
has a function, trust label and contract, but no home unit or source identity.
`LegacySafetyCertificate` re-derives that same unit only. The audit's binding
adds a compile-task and loaded-environment identity, again for one unit.

Transporting a record by name and source digest would be unsound. A C external
call can resolve to an interposed definition; a C99 external `inline` text can
be bypassed; and multiple configurations can define the same name. The
current compile database is a set of units, not a linked-program symbol table.
The run must therefore take **an explicit conditional premise** identifying
the one external definition that each call resolves to. That premise must be
retained in the verdict's assumptions and in the certificate's expected
context. Absent, ambiguous, static-only and external-inline-only identities
must remain uncontracted.

The second boundary is certificate containment: candidate home functions can
be Safe while their translation unit is Unknown, so a whole-unit Safe
certificate cannot authenticate them. A route needs per-function home evidence
holding the preprocessed source, request and loaded environment, and must
re-run W4 over that home unit. The caller certificate must then independently
re-derive the home summary before it re-derives the caller under its imported
copy. `ResourceContracts.functions` cannot carry that copy: source-assumption
replay checks it exactly against the `.qv` declarations. Imports need their
own channel and identity.

The W48 BEFORE/AFTER calibration is 6 call-only corpus functions at the route
ceiling and 10 with held position calls; system and slice 0. It is a dated
upper bound, not this card's current-base measurement. The current-base
arbiter, held-call census and all gate outcomes will be recorded below.

## Measurement

Pending the scheduled private heavy-run slot.
