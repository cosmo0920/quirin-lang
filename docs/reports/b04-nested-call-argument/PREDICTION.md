# B04 prediction, published before AFTER

Base: `8e633261fd216a76d4d25390b0dadeda891c59aa`. The parent run at
`/home/cosmo/Documents/GitHub/qcore-c/private/b-series/baseline/` exited 0 and
is the BEFORE for this card; its `run.json` binds the binary and source commit.

The planning census records 22 nested-call refusal leaves in 13 functions and
four functions for which that normalized refusal is the sole blocker. This
card predicts **zero function movements and zero exchanges**, and it retires
**no acceptance-matrix entry**.

The difference from the plan's apparent four-function opportunity was checked
before this prediction. `pack_string` calls undeclared `cfl_sds_len` inside a
modeled call. The other three sole-blocker functions call undeclared
`cJSON_DetachItem*` functions whose results carry custody into
`cJSON_Delete`; modeling that requires an effectful produced-identity transfer,
outside this card's neutral bound. B04 admits sibling inner calls only when
each has an in-force contract with all parameters `Borrowed`, result `None`,
exact arity, and denotable/data actuals. Their effects are identities, so C's
unspecified sibling-argument evaluation order cannot affect the abstract
state. The existing single-call rule already admits every corpus call meeting
those premises; the new sibling composition is expected to add no corpus
reach.

The synthetic prediction is positive: a call with two sibling, whole-argument
neutral nested calls changes from `Unknown` to `Safe` relative to its declared
test contracts. If either sibling is effectful, undeclared, deeper, partial, or
has a dead/may-alias borrowed actual, the enclosing statement remains refused.
