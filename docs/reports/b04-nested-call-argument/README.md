# B04 — sibling nested-call arguments

Measured 2026-09-20 from base
`8e633261fd216a76d4d25390b0dadeda891c59aa`.

## Result

B04 now models any finite set of sibling calls that occupy whole argument
positions of one modeled outer call, provided every sibling independently
satisfies the existing neutral nested-call premises. A sibling must have an
in-force contract, exact arity, result mode `None`, only `Borrowed`
parameters, denotable/data actuals, and no call inside its own arguments. The
outer position must be borrowed or part of a declared variadic data tail.

This bound is insensitive to C's unspecified argument evaluation order. Every
admitted inner call has the identity abstract effect, and the Rust route checks
the liveness and may-alias obligations of every sibling before applying all
their neutral effects. A call with an effectful or owned result, an uncontracted
callee, a deeper call, a partial call expression, or an unsupported actual
still refuses the entire modeled statement.

The public Rust instruction representation changes
`ContractCall.nested` from an optional single child to a vector of children.
The relevant effect and liveness readers iterate that vector; consumers that
only gate on nested-call presence now preserve the prior refusal with an
emptiness check. This is an internal proof instruction API change; the
certificate wire representation is unchanged.

## Prediction and measured AFTER

The prediction was committed before the AFTER audit in
[`PREDICTION.md`](PREDICTION.md): **zero function movements, zero exchanges,
and no tracker retirement**. That prediction was met.

| corpus measure | BEFORE | AFTER |
|---|---:|---:|
| unit `Safe` | 2 | 2 |
| unit provisional `Unsafe` | 42 | 42 |
| unit `Unknown` | 137 | 137 |
| data-only units | 4 | 4 |
| function `Safe` | 213 | 213 |
| function provisional `Unsafe` | 84 | 84 |
| function `Unknown` | 4,537 | 4,537 |
| leak diagnostics | 272 | 272 |
| custody-summary conflicts | 1 | 1 |
| unknown-alias diagnostics | 4,309 | 4,309 |
| unsupported diagnostics | 49,753 | 49,753 |

The plan's census recorded 22 leaves in 13 functions, including four where the
normalized refusal was the sole blocker. Inspection before the prediction
showed that none meets this card's bound: `pack_string` nests undeclared
`cfl_sds_len`; the three `cJSON_DetachItem*` calls are undeclared and transfer
an owned result into `cJSON_Delete`. Those require modeling effects and
produced identity, so admitting them under a neutral rule would be unsound.
The synthetic two-sibling fixture did move from `Unknown` to `Safe`, as
predicted.

`NESTED-CALL-IN-ARGUMENT-POSITION` remains registered. B04 extends the existing
neutral route but does not discharge the effectful and undeclared corpus
class.

## Implementation and proof correspondence

`admitted_nested_calls` now returns the complete vector of sibling calls. The
empty vector means no nested call; failure to validate any observed sibling
returns `None` and preserves the old refusal. `legacy_c` converts the complete
vector into `NestedContractCall` instructions, checks every nested borrow in
the settled liveness/alias state, and applies every child's neutral effect.
The diagnostic probe was changed to traverse the same complete vector. The
joined-provenance route remains intentionally narrower and rejects any
nonempty vector, preserving its prior nested-call gate.

`QCore.VariadicNested` models a finite list of calls. The audited theorem
`admitted_sibling_calls_are_the_identity` proves identity for an arbitrary
list, and `sibling_evaluation_order_is_unobservable` proves that any two finite
neutral lists produce the same state. The structural premise is now
named `oneLevel`. The former theorem name about more than one nested call was
removed because that claim is no longer true; the retained refusal is
accurately named `a_two_level_nested_call_is_never_admitted`. The dependent
`QCore.DereferencedResult` theorem and audit use those names as well. The Lean
audit adds no project axiom or placeholder.

The correspondence boundary remains explicit: the proof establishes the
abstract list rule, while review establishes that Rust enumeration constructs
the same complete list and that the imported contracts faithfully describe C
callees. Neither the proof nor this card establishes undeclared C behavior.

## Tests and deliberate pin updates

The new constructor-residual tests establish:

* two neutral sibling calls reach `Safe` and the certificate rechecks;
* a dead borrowed actual in a non-first sibling refuses;
* a may-alias obligation in a non-first sibling refuses; and
* one effectful or uncontracted sibling refuses the whole outer call.

The historical O04 trace test previously grouped two different shapes under
`two_level_and_sibling_nesting_are_never_admitted`. It is deliberately split:
the two-level case remains negative, while two neutral siblings are positive,
carry no unsupported diagnostic, and recheck from the certificate. Its
contract fixture change is comment-only. The old one-call behavior and its
diagnostic wording remain pinned. Deeper, partial, owned-result, uncontracted,
and unsupported-actual cases remain negative.

Two shipped mutations each delete a non-first sibling at a different boundary:

| mutation | expected | measured |
|---|---:|---:|
| omit the non-first settled-borrow check | nonzero | 101 |
| omit the non-first instruction child | nonzero | 101 |
| restored-source control, 22 constructor-residual tests | zero | 0 |

The raw logs and rerunnable harness are in [`logs/`](logs/) and
[`scripts/mutations.py`](scripts/mutations.py). The harness always restores the
original bytes and accepts `CARGO_TARGET_DIR` and `QCORE_MUTATION_LOG_DIR` for
portable, non-overwriting reruns.

## Pinned corpus evidence

The BEFORE is the parent coordinator's successful base run at
`/home/cosmo/Documents/GitHub/qcore-c/private/b-series/baseline/`. Its
`run.json` binds base `8e633261`, the binary, package, and source inputs. B04
reuses it with that provenance rather than manufacturing a second baseline.

The published AFTER is
`/home/cosmo/Documents/GitHub/qcore-c/private/b-series/b04/after-pinned/` and
its audit exit is 0. It used package
`build/cycle19/provider-packages-wave20/qcore.chunkio`, source
`build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source`, and
`QCORE_CLANG=/usr/bin/clang`.

| artifact | BEFORE SHA-256 | AFTER SHA-256 | relation |
|---|---|---|---|
| diagnostics | `c54ad4219fa85793652760e94cdff48f145fc30f04f7ab2604f65da38a42cc2b` | same | verdict-bearing identical |
| chunk contracts | `c4f438e4ed3dab3c3b0e776af5c7c5181385353a72c67b6874ab4df5ba917980` | same | verdict-bearing identical |
| source | `ae38f6c93dfbe341b187077e9dd0074501000ea76d621feb481db2303930e529` | same | source identity identical |
| source effects | `0bda08ad86da01b133bcf0397ee39d7b7cd145a1265c1cb66df2747e6aa4fc9b` | `4295fdf29a58650e2f215d506319f02229c24b204776b6fc5cb6d02104cea877` | checkout/build identity differs |
| contracts | `9de748ce56d2c614f251411f88e50a8b8a50c7dbff935216515e873a3c081f32` | `019781aec4f77eccaa10027f6fab666942e350866166ef2b254ae5ac04d4f61d` | checkout/build identity differs |

The last two artifacts embed checkout-derived build and configuration
identities. Their semantic source and contract inputs remain the pinned ones;
they are disclosed rather than described as byte-identical. The parent
coordinator independently normalized the two JSON pairs: contracts has no
structural difference, while source-effects differs only at the enumerated
configuration, build, task, obligation, bound, and coverage identity paths.
That independently produced path list is preserved in
[`artifacts/normalized-diff-paths.json`](artifacts/normalized-diff-paths.json).

An earlier local invocation omitted `QCORE_CHUNKIO_PACKAGE`. Its directory
`private/b-series/b04/after/` contains diagnostics, chunk-contracts, contracts,
and source JSON but no source-effects artifact, plus its audit log and exit. It
was rejected as an incomplete, unpublished measurement. The table and counts
above come only from the corrected `after-pinned` run.

## Standing gate

The final isolated gate ran from restored source with
`CARGO_TARGET_DIR=/home/cosmo/Documents/GitHub/qcore-c/private/b-series/targets/b04`
and `CARGO_BUILD_JOBS=3`. Raw outputs and one exit per command are under
`/home/cosmo/Documents/GitHub/qcore-c/private/b-series/b04/gate-final/`.
`clang-version.txt` records the Clang identity. The workspace test step ran
outside the sandbox so LeakSanitizer could execute outside ptrace; sanitizers
were not disabled.

The machine-readable results are in [`gate.json`](gate.json). The parent
coordinator owns the combined B01–B04 gate and the final acceptance-matrix
regeneration.

The isolated measurements are 4,817 passed, 0 failed, and 10 ignored across
246 result summaries and 244 targets; 145 Lean audits passed; 305 Python tests
passed; format, clippy, both reevaluation probes, and Clang recording all
exited zero. Provider 122 is carried rather than remeasured because B04 changes
neither `providers/` nor `crates/qcore-schemas/`.

The measured gate was also supplied to
`docs/reports/remaining-d02/build_matrix.py` with `--out` to a private matrix;
the generator exited 0, the 305-test Python suite passed against that generated
matrix, and the committed matrix was restored byte-identically. Those logs and
three zero exits are under
`/home/cosmo/Documents/GitHub/qcore-c/private/b-series/b04/matrix/`. The
generated isolated matrix is not committed because the parent reconciles the
combined B01–B04 gate and tracker union once.

## Integrator reconciliation

The combined integration must preserve these facts:

1. keep the vector conversion in every `ContractCall.nested` producer and
   consumer, including the alias aggregate diagnostic probe;
2. keep the precise one-level Lean theorem names and remove the obsolete
   more-than-one-sibling refusal claim;
3. keep the deliberate O04 sibling-positive/deep-negative pin split;
4. record zero corpus movement, zero exchanges, and no tracker retirement;
5. leave `NESTED-CALL-IN-ARGUMENT-POSITION` open for effectful and undeclared
   nested calls; and
6. reconcile the union's increment, status, gate, and derived matrix once,
   using the combined measurements rather than this card's isolated counts.

No main-branch update or push is part of this card branch; the parent performs
the authorized merge and remote push after the combined gate passes.
