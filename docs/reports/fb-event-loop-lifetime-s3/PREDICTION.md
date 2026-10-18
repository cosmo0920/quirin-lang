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

## Implementation direction after the scoped-condition proposal

Before semantic code, the implementation is rebased onto `854920e3e`.
The user's local-condition blocks are implemented for conditional
prerequisites and lifetime-protocol clauses, with explicit read-only typed
condition bindings. Unsupported consumers and clause combinations refuse;
unknown conditions preserve both cases. Type-wide footprints do not inherit
one host call's instance condition. This extends parser/model scope but does
not predict extra corpus Safe movements or any null-property movement.
The earlier source census remains a source observation, not a fresh verdict
baseline on the rebased source.

## Integration addendum — 2026-10-18

The first production footprint is limited to the collector record; scheduler
footprints are not added by this batch. The selected S3 route permits an
explicitly attributed unchecked protocol assumption, while contradictions
withdraw it. Holder equality/stability is never supplied by that assumption.
The final immediate baseline includes the lexical integration and the user's
outer-envelope correction; their effects must be attributed separately from
S3. The historical854 baseline remains cumulative evidence only.
