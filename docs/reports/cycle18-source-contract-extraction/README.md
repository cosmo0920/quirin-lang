# Cycle 18 timed-wait source contract extraction

This card moves the bounded Monkey timed-wait source acceptance policy out of
`monkey_wait_projection.rs` and into the executable QCore module
`monkey/wait_source_policy.qcore`. It preserves the constructor-private source
projection boundary and does not issue an event certificate or a Safe verdict.

## Extracted policy

The new module is the sole runtime authority for the pinned source revision,
callback name, callback signature alternatives, four wrapper signatures,
profile identity, import roles, path counts, complete per-path operation counts
and call sequences, selected path indices, selected path shapes, return kinds,
and event flags/descriptor/priority observation requirements. It also owns the
five wrapper/composition declaration headers and exact clause sequences; the
existing `wrapper_paths.qcore` and `composition.qcore` files remain byte
unchanged.

The module admits two complete canonical digest vectors: the focused fixture
and the production translation unit. The common three digests are checked
individually and the two varying digests are checked as a paired disjunction,
so a fixture/production mixture is refused. Rust still traverses typed source
summaries, derives facts, serializes complete paths, hashes them, checks that
all summaries share the same profile assumptions, and requires exactly one
policy-accepted candidate path. Expected values are not repeated in Rust.

`MonkeyWaitContracts::parse` keeps its two-argument public interface and loads
the bundled policy. The source revision and callback accessors read unique
literal clauses through the generic policy engine; malformed bundled policy is
an invariant failure in the convenience accessors, while construction and
projection return an error. The policy override is private to the module's
mutation test, so callers cannot supply a weaker contract as verification
authority. A successful projection now carries the policy digest under the
stable `monkey/wait_source_policy.qcore` key.

## Parity and mutation evidence

[`parity.json`](parity.json) records the old and new five-function set,
selected path indices, two whole-artifact alternatives, eleven observation
requirements, and the intentional addition of the policy digest. The existing
focused source suite remains the behavior parity check: the accepted fixture
projects, missing/foreign/duplicate summaries refuse, wrapper/composition
contract mutations refuse, callback/lifecycle mutations refuse, and all three
same-count operand/field/guard substitutions still reach canonical artifact
refusal.

The policy contains 13 declarations and 102 clauses. The in-tree mutation test
renames each declaration and replaces each individual clause occurrence with
`false`, re-running construction and projection against one imported baseline;
all 115 mutants refuse. This includes repeated clause text in different
declarations as separate mutations. The production preprocessed-input test
remains explicit and ignored in ordinary runs; this card did not claim a new
production observation.

`QCore.MonkeyWaitSourcePolicy` mirrors the accepted summary facts, exact
wrapper/composition declarations, observation identity, and whole-vector
artifact choice. Its audited theorems accept the exact fixture and reject a
mixed digest vector, selected-path substitution, missing summary, changed call
sequence, and changed contract clause without project axioms or placeholders.

## Integrator reconciliation list

- Add `monkey/wait_source_policy.qcore` to the global contract-corpus pins and
  selection/count documentation. This card deliberately does not edit the
  root-owned corpus table, status ledger, acceptance matrix, QCore aggregate
  audit, or increment record.
- Reconcile the public accessor migration wherever sibling work still refers
  to `MONKEY_SOURCE_REVISION` or `WAIT_CALLBACK`: use
  `monkey_source_revision()` and `wait_callback()` so the contract remains the
  only literal authority.
- Preserve the new `monkey/wait_source_policy.qcore` digest when composing this
  projection with the identity and observation cards.
- The two predicate-engine commits (`b4228aa9`, `27ab484c` in this branch) are
  prerequisites and may already be present through the shared engine card;
  resolve them once rather than duplicating their report/audit files.
