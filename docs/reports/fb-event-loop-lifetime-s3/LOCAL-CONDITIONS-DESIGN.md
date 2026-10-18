# Scoped conditional prerequisites: implementation direction

The user proposed a brace context that switches prerequisites:
`quant #threaded in { … }: [[local]];` and
`quant not #threaded in { … }: [[local]];`, with `[[local]]` representing
local scope for the clause condition. The coordinator subsequently directed
implementation using that exact block form, with an explicit read-only alias
binding as a routine grammar choice. This supersedes the earlier pending
syntax decision; it does not approve declared equalities or arbitrary effects.

Conditional prerequisites are useful for S3. The threaded collector needs a
proved ending of its worker-loop registration before release; the
non-threaded collector can use removal at the engine loop when the paths are
proved equal. A condition must select the matching obligations and effects,
without assuming the condition or its prerequisites merely because they are
written in a local block.

## Grammar before this card and source binding

`SourceContractClauseKind` in `qcore-frontend/src/frontend.rs` distinguishes
`requires`, exit-selected `ensures`/`quant`, checks, deref/live and outer
clauses. `contract_block` reads a flat semicolon-terminated list. It has no
condition block, `quant not` or generic `[[local]]` attribute. The lexer's
ordinary brackets can spell `[[local]]`; outer_contract.rs recognizes
`[[contract]]` and `[[runtime]]` by exact token patterns, exclusively in the
outer vocabulary. Neither grants a general scope-attribute interpretation.
`[| … |]` attributes for loop control and C imports are a different form.

Current `when` guards are attached to individual effects: retained pointers,
member states, result handoffs or an outer loader condition. They do not
provide an enclosing context for `requires`. Existing `where` uses bind
quantitative type indices, id forwarding or outer reachability; none already
binds a local Boolean name to a source member.

Source grounding at the prepared slice revision `34155955e4962a56d94eb23c03a002880899a0d1` (the historical report called its export lineage `90978ac6`):

* `include/fluent-bit/flb_input.h:395`: `int is_threaded` in
  `struct flb_input_instance`.
* `flb_input.h:733–736`: the inline `flb_input_is_threaded(ins)` returns
  exactly `ins->is_threaded`. Its C truth condition is nonzero, not equality
  to one.
* `src/flb_input.c:2246–2258`: `coll->instance = ins`, then the predicate's
  true arm stores `thi->evl` and its false arm stores `config->evl` in
  `coll->evl`.
* The field also has initialization and property-setter writes. Rule 10p's
  immutable-after-init premise must remain explicit; absence of later writes
  is not established merely by the two collector stores.

## Chosen initial surface

The initial implementation grammar is:

```text
binding := quant #C = (#P.MEMBER_PATH != 0);
block   := quant [not] #C in { (prerequisite | protocol)+ }: [[local]];
prerequisite := requires EXISTING_OBLIGATION;
protocol := quant registered #B: TAG.INTERIOR |-> TAG.HOLDER;
```

For a declaration whose pointer formal is named `ins`, the concrete binding
would be:

```qcore
quant #threaded = (#ins.is_threaded != 0);
```

For a collector formal `coll`, it would instead bind
`#coll.instance.is_threaded`. There is no implicit global `#threaded`, and
no inference from that English name. The condition is read from the typed
formal/member at the call's entry; the checked inline predicate provides the
matching in-body branch fact. The condition's storage must stay stable over
the region that uses it, or the fact is invalidated. Predicate calls with side
effects, unreadable types, volatile/atomic fields or unknown aliases do not
silently supply that fact.

That `coll` must be an actual formal of the hosting function. The `#coll`
introduced inside a type-wide `quant registered` clause is bound only by
that clause and is unavailable to an enclosing alias. Likewise a setter's
`#ins.is_threaded` governs that call's instance, not every object of type
`flb_input_collector`. Keep generic function-local contexts separate from
type-wide protocol predicates. Relating their conditions requires a checked
root-specific `coll.instance == ins` derivation for the exact record being
created or processed; a different record, an unresolved list lookup or an
unproved instance relation preserves an unresolved obligation. This initial implementation
does not choose a new syntax for universal per-record conditional protocols.

Initially admit conditional prerequisite and registered-footprint syntax;
consumers must implement scoped discharge/correlation and attribution before
relying on their effects. Keep the vocabulary selector, ABI, parameter and
result custody modes, revision/review markers, global protocol declarations
and outer envelopes outside conditional blocks. Unsupported inner clauses
reject the module; they are not skipped. An unguarded S3 footprint is type-wide. A footprint inside a local block
retains its condition and requires checked per-record/host-instance
correlation; absent correlation stays unresolved. Conditional `ensures`
remains outside this initial implementation; it needs guarded state transfer
semantics as well as syntax.

The alias is contract-local, has exactly one Boolean definition, and names a
formal or a proved member path. Aliases cannot shadow formals, other binders
or reserved value words. Reject an unbound alias, a duplicate definition,
cyclic binding, empty block, nested block in the first version, wrong scope
attribute or unsupported condition expression. One positive and one negative
block for an alias are allowed; a missing complementary block means there
are no *additional* prerequisites on that side, not that the side is
unreachable. Unconditional clauses still apply on both sides.

## Meaning and scope

`quant #C in { requires P; }: [[local]];` means `C ⇒ P`.
The negative block means `¬C ⇒ P`. A proven true condition activates the
positive block; a proven false condition activates the negative block.
An unknown condition leaves both cases feasible. Check each feasible case,
then join the outcomes; if either has an undischarged obligation, Safe is
withheld. Never interpret unknown as false, never use absence of proof of C
as proof of its negation, and never conjoin incompatible branch assumptions
to manufacture a contradiction.

Inside a body, the positive context is available only beneath a matching
proved branch, and the negative context only beneath its proved complement.
On rejoining branches the condition fact is forgotten unless both establish
it. A `requires` clause is still a caller obligation: the checker cannot
assume it while checking its caller. Declared premises remain explicitly
named assumptions under the matching condition, with their evidence/pin
dependencies. `[[local]]` prevents the branch fact from becoming an
unconditional contract/run premise; it does not erase a conditional
prerequisite from the function's interface.

Interprocedural summaries preserve the implication and substitute the exact
actual root/path, or retain an unresolved guarded obligation. A local fact
for instance A cannot discharge an obligation on instance B. An unread or
external caller cannot inherit a body-local branch fact. No local condition
affects C04.20's static/runtime loaded-code scenario, or grants a global
outer envelope.

For S3, the false branch permits considering the engine-loop removal only
after the member equality has been **derived** from the stores and stated
stability premises. The condition does not declare that equality. The true
branch retains its worker-loop ending obligation; unchecked/absent joins
remain unresolved. A poisoned registration still needs its own loop to end.

## Direction resolved and boundaries retained

The user's local-context direction is authorized. The explicit alias spelling
is the routine binding choice recorded before semantic code. Initial blocks
contain conditional prerequisites and registered lifetime protocols; ordinary
custody modes, arbitrary `ensures`, nested local blocks and other attributes
remain rejected. The vocabulary selector remains unconditional.

`[[local]]` denotes scoped branch facts with conditional caller obligations
preserved. Callee bodies may be checked under their declared conditional
preconditions, as in ordinary contract reasoning. At the caller/use boundary,
the annotation itself establishes neither the condition nor its prerequisite.
A body-only proof context that erases caller obligations is not implemented.

A conditional type-wide protocol needs a derived root-specific correlation to
the host's exact instance. Missing correlation is a named unresolved boundary,
not a reason to ask again about the chosen syntax. Holder equalities remain
derived-only. Interprocedural must-removals whose order or incoming poison is
lost remain refused; only must-destroys discharge through that abstraction.
New closed-world lifetime/stability facts must appear in dependencies and the
runtime outer projection; absent an applicable loaded-code envelope, runtime
admission stays withheld. No envelope is broadened in passing.

## Implementation and proof obligations

Add structured condition bindings/blocks to `frontend.rs`, with recursive
clause parsing and precise spans rather than flattening an inner `requires`
into opaque text. Add portable condition/path/guarded-obligation schema data;
teach the resource loader total-or-reject validation, inclusion/normalization
and the exact supported inner-clause whitelist. Keep review deferrals
declaration-wide. Consumers without conditional support must withhold the
contract instead of applying only its unconditional portion.

Ground predicates and conditions in the typed member reader and run store
census; carry immutable-field and instance-identity premises. Extend S3
summary substitution/fold and clause-level body standing, and emit condition
provenance in dependency records. Existing `RetentionGuard` does not suffice
on its own: its conservative unknown-means-retain policy is a useful boundary
for effects but does not implement paired prerequisite branches.

Lean and shared executable vectors must establish: branch activation requires
matching evidence; unknown preserves both cases; checking both cases is
sound for every concrete truth value; guarded requirements remain caller
obligations; substitution preserves instance identity; local assumptions
cannot escape scope; branch merge forgets one-sided facts; contradictory
unproved assumptions yield no discharge; declared premises remain named;
an unproved holder equality ends no record; and the final caller fold only
demotes. Retain the existing poisoning/holder-end theorems. Parser tests must
exercise rejected unknown/nested/unsupported blocks, not just accepted syntax.

The report's gate record, when complete, owns commands and counts. This
design document is not evidence of a full standing gate or a safety baseline.

## Source extraction premises and conservative controls

The abstract condition model uses numeric binder/root identifiers. Its Lean
proof does not prove production canonicalization: the correspondence from each
abstract identifier to a declaration-qualified condition and its exact canonical
member path is an explicit extraction premise. Production matching uses the
source/configuration/function-qualified actual root plus the complete normalized
member chain, rather than a local alias spelling or a formal index alone.
Bindings of two different members on the same actual object are distinct.

A structural branch is accepted only in the supported structured-control subset.
A function containing labels, goto, indirect goto, case/default labels, assembly,
or recognized nonlocal jumps contributes no local condition facts. Effectful or
unsupported argument/callee expressions invalidate prior facts before a call.
Checked getter bodies must be a single member-return statement, and every getter
actual/designator must be effect free; otherwise its enclosing conditional
forgets inherited facts as well. This is a conservative source rule, not a proof
that arbitrary C ancestry implies CFG dominance.

Syntactically different formal/result roots remain possible aliases unless a
separate checked disjointness fact exists. A lifted closed type footprint covers
may effects only; destroying one unresolved holder never ends all records in
that footprint, and lifted paths supply no must endings. The source summary
retains unresolved effects and does not convert a linearized operation census
into a proof of complete CFG exit coverage. Admission requires the independent
body/source standing boundary, not merely an empty diagnostic census.

The bounded active named-prerequisite path uses ordinary Hoare reasoning from
the current function's own conditional `requires`, with full actual/member
substitution and polarity preserved. It never reads an `ensures` as an entry
premise. Entry-to-use correspondence additionally requires the explicit
unconditional named premise
`resource_premise(s3_condition_snapshot_stable, "Every member read by this function's condition bindings preserves its entry value until each guarded use.")`.
This is not inferred from absence of local writes. The source subset still
rejects intervening calls, stores, formal reassignment, effectful operands and
unsupported control. Each used entry obligation and snapshot premise is
qualified by the source/configuration/object identity in dependency output;
loaded-code projection requires coverage and no existing envelope is extended.
This general entry premise does not derive Fluent Bit's post-init flag
immutability or holder equalities: those remain separately named D3 obligations.
