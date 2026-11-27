# CONNECT-LIBC-EFFECT-CLAIMS: prediction

Base `9903b027a`. Written at the hand-back, before any option is applied. Every
row is a prediction for the integrator to measure; none of it has been
measured.

## This commit (census and report only)

This commit adds only report files, so all four pinned routes (corpus,
twelve-home, system, slice) are byte-identical in verdicts, coverage and
contract identity.

## Per option, if applied

| Route | A (move literal declarations to libc, owner read directly) | B (resource-v1 task/defer vocabulary) | C (A plus `linkage_contract_v2`) | D (keep in libevent, refusal with grandfathering) |
|---|---|---|---|---|
| corpus | no move | no move | no move | no move |
| twelve-home | no move | no move | no move | no move |
| system | no move | no move | no move | no move |
| slice | no move | no move | no move | no move |

The reason is the same for every route: neither
`monkey/callback_task_handoff.qv` nor `monkey/timeout_teardown.qv` is in
libevent's resource view, and a new libc literal module would be in no view.
The pinned route reports each Monkey callback's Cycle 18 composition as
`Unknown` with a reason and does not run it, so no route's verdict reads these
declarations.

Identity changes that are not verdict moves:

- **A and C.** The libc and libevent module digests change, along with their
  `contract-inventory.lock` files, `contracts.json` and the module list in
  `contract-load-path.json`.
  - Any route whose identity binds the whole load-path manifest records a new
    input identity.
  - `quirin-audit callback-task-handoff` keeps its result ("bounded
    obligations checked"). Its `policy_digest` changes because the libevent
    policy text shrinks, and it gains the libc module's digest.
- **B.** The resource-v1 declarations of `send`, `perror` and
  `__errno_location` change, so their `contracts.lock` `contract_sha256`
  values change (flexible pins; a review and re-pin are required).
  - The custody effects are unchanged, so no gateway verdict moves.
  - The Lean audit count grows by the new vocabulary's theorems.
- **D.** No contract text changes. Only `providers/contracts.py check` gains a
  refusal.
