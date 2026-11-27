# B02-BRANCH-CONDITION prediction — published before AFTER

Published from the unchanged baseline at commit `8e633261`, before any changed
corpus audit was run.  The baseline is the integrator's fresh exit-0 audit in
`private/b-series/baseline/`; its verdict-bearing `resource-diagnostics.json`
and `resource-diagnostics.chunk-contracts.json` reproduce the archived
W37-CORPUS-CONTRACTS-4 baseline byte for byte.

The implementation admits exactly one ordinary contracted call in an `if`
condition when C evaluates it unconditionally, its residual expression is
already custody-neutral, and the existing statement classifier produces an
ordinary `ContractCall`.  It rejects `&&`, `||`, `?:`, assignment,
increment/decrement, a call under a type operator, a second ordinary call, an
uncontracted call, a call with a discarded custody result, and any residual
indirect read the existing expression rule refuses.  Loop and `switch`
conditions are B03; generalized nested-call arguments are B04; destination-side
indirect access is W42-INDIRECT-ACCESS.

## Prediction

* **Function movement: 1 Unknown to Safe.**  Of the baseline's 44 Unknown
  functions whose only distinct blocker is the branch-condition family,
  `cio_file.c::cio_file_is_up` is the one whose blocking condition is a single
  call to the selected, declared `cio_chunk_is_up` contract in this scope.  The
  other 43 are indirect/aggregate reads, uncontracted calls, short-circuit or
  multi-call conditions, and remain outside this card.
* **Exchanges: 0 predicted.**  A syntactically declined decomposition keeps its
  existing branch refusal and restores site allocation.  An admitted call uses
  the existing `ContractCall` transfer; this prediction found no sole-blocker
  candidate expected to exchange the branch leaf for a liveness, alias or
  contract-effect refusal.
* **Retirements: none.**  B02 alone does not meet a registered entry's complete
  discharge condition.  In particular the 196-function indirect+branch pair
  remains because W42-INDIRECT-ACCESS is deliberately not implemented here.

The AFTER measurement may legitimately find more removed leaves in functions
that retain other blockers.  Those are projection movement, not additional
function verdict movement, and will be attributed separately rather than
silently folded into this prediction.
