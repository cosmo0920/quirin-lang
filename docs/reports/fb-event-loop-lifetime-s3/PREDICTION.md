# S3 prediction before implementation

Prepared at `654e2b7eed72374b02ea39ef41059f7a9149b776`, before a D4
answer and before product changes. These are predictions, not measurements.

The old design report's BEFORE figures belong to `3f8f1a617`; they are not
reused as an exact-base measurement. The latest blocker report established
that the slice's two `collector_start` configurations now reach
`retention-undecided`; that fact makes the S3 summary path reachable but does
not itself prove the summary or its callers Safe.

Expected effects, conditional on approved syntax and validated declarations:

* Corpus: no known loop-retention-only row is predicted to become Safe.
  Non-loop retentions such as LZ4 are outside this card.
* System: registration exits at `schedule_request_now` and `flb_sched_create`
  may trade their retention refusal for a forwarded summary. Other body
  refusals are expected to keep their final status Unknown.
* Slice: `collector_start` is the primary candidate for a local lifetime-leaf
  discharge. Final promotion requires the run fold to discharge every
  forwarded obligation; no unconditional Safe movement count is asserted.
* Threaded teardown stays Unknown where joins/endings are not proved.
* Null-property results are expected to remain identical. No Unsafe or
  provisional upstream-defect claim is introduced.
* An undisclosed assumption cannot stand in for a checked declaration.
  Newly discovered release obligations may legitimately demote existing Safe
  rows only with explicit correction evidence; no such row is predicted yet.

Fresh baseline and after artifacts are pending the coordinator's heavy slot.
The arbiter comparison, not this prediction, determines actual movement.
