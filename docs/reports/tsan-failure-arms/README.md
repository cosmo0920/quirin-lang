# `TSAN-FAILURE-ARMS` (wave 41) — failure-arm ThreadSanitizer rows, and a release trigger the join does not own

**Status: in progress (skeleton).** Base `463fa6e8` (= `origin/main` at start).

## 1. Scope

Three things, in the order `STABILITY-REEVAL-4` measured them:

1. a forcing-header release trigger the creator's join does not own, so that deleting a
   join can no longer hide every race;
2. TSan rows over the task programs' *failure* arms (a refusing `QCORE_TASK_CREATE`,
   a NULL-returning `QCORE_BUFFER_MALLOC`) on the write-lend flagship and on CT07's own
   `case.qcore` lowering, each row asserting which arm executed from the program's own output;
3. an enforcement of `TSAN-FORCED-HARNESS-SYNC-INVISIBILITY` rather than a documented claim.

## 2. The release trigger

*(to be written)*

## 3. The failure arms

*(to be written)*

## 4. Invisibility enforcement

*(to be written)*

## 5. Non-vacuity: catch-rate table

*(to be written)*

## 6. Byte identity of emission

*(to be written)*

## 7. Gate

*(to be written)*

## 8. Residuals and retirements

*(to be written)*

## 9. For the integrator

*(to be written)*
