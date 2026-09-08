# QCore implemented input grammars — revision 0.3

> Repository integration: 2026-09-07. Normative requirements are adopted from
> the supplied review; implementation status is reconciled with main in the
> [acceptance ledger](reviews/r0_3/REVIEW.md). The original archive findings
> are historical observations, not a claim that later merged repairs are absent.
> Full revision-0.3 conformance is not claimed.

**Source scope:** supplied archive review, reconciled with the implementation records in the [acceptance ledger](reviews/r0_3/REVIEW.md).  
**Purpose:** distinguish exact consumer inputs from the broader [design BNF](QCORE_DESIGN_GRAMMAR_r0_3.bnf).

The grammars below use EBNF: quoted tokens are literals; `{ X }` means repetition; `[ X ]` means optional; `|` means alternatives. `IDENT` is an ASCII identifier and `NAT` is an unsigned decimal token. Lexical recognition alone does not determine semantic admissibility. Historical line references name the reviewed files; the integration source inventory separately binds available current files.

## G01. Structural source envelope

```ebnf
module_header = "module", ("executable" | "library" | "contract"), ";" ;
source        = [module_header], {declaration} ;
contract      = "contract", "{", {clause}, "}" ;
clause        = "requires", NONEMPTY_EXPRESSION_TOKENS, ";"
              | "ensures", selector, NONEMPTY_EXPRESSION_TOKENS, ";"
              | "quant", selector, NONEMPTY_EXPRESSION_TOKENS, ";" ;
selector      = "normal" | "any" | "error", "(", NONEMPTY_TYPE_TOKENS, ")" ;
```

`declaration` and the two uppercase token-sequence placeholders above deliberately denote structural parser regions, not a claim of full expression/type checking. The parser tracks balanced regions and clause spans. The relevant implementation is `crates/qcore-frontend/src/frontend.rs`, particularly declaration validation, contract parsing, parameter parsing and `elaborate_source`.

Recognized declaration categories are concurrency profile, verifier scope, dimension, axis, type alias, resource type, error, constant, extern function and function. Broad statement/type classification includes forms that the source compiler will subsequently reject. The structural grammar is therefore not a sound shortcut for discovering executable capabilities by trying tokens until parsing succeeds.

A header may occur at most once and only before declarations. The default is executable mode. A contract module is entry-free; it is not automatically an observation policy or a resource table. Multiple input modes combine as executable first, library second, contract last. Each file still has its own header constraint, and declarations share one namespace.

Task syntax now has structured records in `frontend_tasks.rs`: `spawn S worker(...)`
retains ordered `copy`, `move`, `read ... share ...` and `write` place captures;
`join move H` retains its handle place. An immediately following `?` is retained.
Name resolution identifies a live scope, a source worker with matching arity,
capture roots and the handle binder. Typed statements preserve new binder IDs
after resolving their initializers. Place/share syntax is not proof of capture
types, permissions or live handles. These records do not extend G02's sequential executable profile. The separate
G02a path now checks and executes its supported task subset; the earlier
[task foundations record](reports/w6-source-task-foundations/README.md) remains historical.

## G02. Source execution profile

The following is the supported canonical body syntax, not a claim that every possible structurally accepted spelling of a declaration has ordinary type semantics:

```ebnf
body          = "{", {statement}, "}" ;
statement     = acquisition | check | direct_call | drop | move | return ;
acquisition   = "let", IDENT, "=", "qcore_acquire", "(", RESOURCE_ID,
                ",", BOOL, ",", ERROR_NAME, ")", ";" ;
check         = "qcore_check", "(", BOOL, ",", ERROR_NAME, ")", ";" ;
direct_call   = FUNCTION_NAME, "(", ")", ";" ;
drop          = "drop", RESOURCE_BINDING, ";" ;
move          = "move", RESOURCE_BINDING, ";" ;
return        = "return", [ ["-"], NAT ], ";" ;
BOOL          = "true" | "false" ;
```

`RESOURCE_ID` must fit `u32`. `ERROR_NAME` must be declared; when no error declarations exist the driver provides its default `Failure` case. Resource bindings must satisfy ownership and cleanup checks. A `move` transfers to the abstract result payload; it does not simply release an external endpoint. A unit result uses bare return, represented as zero in the reduced ABI. An i32 result requires its signed literal; the separate custody payload is explicitly identified in compilation result metadata.

`qcore_acquire` and `qcore_check` take literal Boolean controls in this profile. They are model operations, not general malloc calls or arbitrary runtime conditions. General expressions, argument-taking calls, indirect calls, calls through a shadowing local, calls to extern declarations, nested source control flow, source lambdas and source threading are not accepted by this driver body path. Local `use` annotations are explicitly rejected here rather than ignored.

A direct call must resolve to a zero-parameter source function. For C emission, it must also have supported acyclic call structure and an interpreted observation with no resource trace, a normal result, and no owned payload. It is not enough that the function's aggregate net resource change is zero. Declared user effects and general contract predicates are not comprehensively checked by this slice.

The executable driver admits explicit i32/unit result annotations and validates each return form. Signed i32 literals include both extrema; the signed value is range-checked in a wider representation. Other result annotations, nonliteral returns and scalar/type mismatches reject. Structural tooling preserves richer result syntax, but full owned-result/per-exit typing is still unsupported. A separate custody identity ledger retains the reduced model's transfer payload; it is not proof that an i32 annotation describes owned values.

An example within the canonical slice is:

```qcore
fn main() -> i32 {
    let item = qcore_acquire(1, true, Failure);
    drop item;
    return 0;
}
```

This example does not demonstrate arbitrary allocation ABI support. Source references: `compiler/src/driver.rs:390–610, 724–767, 775–932`.

## G02a. Scoped-copy task execution

`source.tasks.copy.v1` uses the ordinary compiler commands and requires exactly
one `concurrency checked;` marker. All definitions are checked. The entry has no
parameters; leaf workers accept named i32 parameters and return i32/unit.
Local scalar bindings are immutable. A return may use an i32 literal or live
scalar slot; unit uses bare return or a live unit result slot. Explicit supported effect rows permit
`spawn`, `join`, `may_block` and `throws<E>`; every actual effect/error must be
permitted. Payload-free error variants preserve their exact `E::Variant` tags.

```qcore
concurrency checked;
fn worker(x: i32) -> i32 { return x; }
fn main() -> i32 effects { throws<SpawnError>, spawn, join, may_block } {
    let x = 42;
    thread scope s {
        let h = spawn s worker(copy x)?;
        let collected = join move h;
        let y = collected?;
        return y;
    }
}
```

Creation succeeds or raises the distinguished SpawnError dynamically. `join`
collects the worker's typed Result; `?` is a separate propagation operation.
`let y = join move h?;` combines those operations. A bare join may discard only
an infallible unit result. Scope exits, returns and propagation reject live
handles or unconsumed Results, including the failure edge of a later spawn.
There is no automatic join. Sequential spawn/join pairs are supported.
An infallible `?` consumes the Result and extracts its scalar value without
inventing an error exit; the core requires an empty declared error set.

Explicit recovery uses exactly one of each canonical arm, in either order:

```qcore
match spawn s worker(copy x) {
    Result::Ok { value: h } => { let y = join move h?; return y; }
    Result::Err { error: failure } => { return 0; }
}
```

The scrutinee may instead be `join move h` or an existing Result binding.
Scrutinees are whole expressions without `?`. Each arm binds one local name;
its checked body may return, raise or continue with outer identities. Arm
bindings do not escape. `raise failure;` rethrows a bound error with its exact
tag; `raise E::Variant;` remains supported. Handled creation errors need no
`throws<SpawnError>` permission unless propagated. Spawn itself requires its
spawn effect. Prior live siblings remain obligations on every arm and must be
explicitly joined before exit. Other patterns, guards, arm values, arbitrary
match scrutinees and general branching remain unsupported. Explicit matching
of an infallible Result conservatively checks both arms; only infallible `?`
uses the empty-error extraction rule.

Captures must be `copy` of direct immutable i32 local/parameter names. General
expressions, assignments, loops, general branches, resources, loans, move/read/write
captures, mutexes, externs, globals, user contracts and task-bearing workers
reject in every definition. Entry-free task modules are not admitted yet.
The native driver currently supports Linux C11 threads and macOS through an
emitted pthread-backed compatibility implementation of the required C11 thread
subset; it explicitly rejects other targets. Its start record survives until successful native join; failed
join aborts without reclaiming possibly accessed storage. These rules do not
establish general C11/shared-memory or cross-platform refinement. See the
[recovery execution record](reports/w6-task-recovery/README.md).

## G02b. Owned-buffer task execution

`ScopedOwnedTasksV1` extends G02a with one concrete builtin type:
`own<Buffer>`. Each Buffer is a malloc-backed allocation with a fresh generation
and one immutable i32 payload. Its resource identity is separate from source
slot and scalar identities. This is not arbitrary `own<T>` elaboration.

```qcore
concurrency checked;
fn worker(buffer: own<Buffer> use one) -> own<Buffer> {
    return move buffer;
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
    let buffer = qcore_buffer_new(42)?;
    thread scope s {
        match spawn s worker(move buffer) {
            Result::Ok { value: handle } => {
                let returned = join move handle?;
                let value = qcore_buffer_read(returned);
                drop returned;
                return value;
            }
            Result::Err { error: failure } => {
                let value = qcore_buffer_read(buffer);
                drop buffer;
                return value;
            }
        }
    }
}
```

Owned parameters require exactly `use one`. Captures must use `move` for owned
Buffer parameters and `copy` for i32 parameters, with exact arity and no duplicate
owned capture. Successful creation consumes the original parent bindings;
failed creation restores them, without altering prior siblings. A later join
returns custody through a fresh Result/value binding; it never revives the
original moved binding. Reading does not consume ownership; `drop name;` and
`return move name;` consume it. Repeated reads require the same live owner.

`qcore_buffer_new(I32_OPERAND)` requires `alloc` in the root or a leaf worker. It must either
be propagated with `?` under `throws<AllocError>`, or directly matched using the
same canonical Result arms as G02a. A failed allocation produces no Buffer or
new generation. Earlier owners must be released on that failure path. Local
annotations may be i32, unit or `own<Buffer>` and must match the initializer;
local quantitative annotations remain unsupported. `qcore_buffer_read(NAME)`
requires one live owned binding and returns i32. The builtin function names and
AllocError cannot be shadowed by source declarations.

Workers may read/drop owned inputs and locally allocated Buffers, return exactly
one remaining input or fresh local, or drop all of them before returning
scalar/unit or raising a payload-free error. The core derives this relation from
checked exits and applies exact caller capture identities or a checked fresh
result; return annotations alone cannot create custody. Multiple owned inputs
are supported. Owned root-entry results reject, including in uncalled definitions.
Workers remain nonrecursive task-free leaves. Every scope/return/error exit must
account for owned bindings, Results and handles; there is no implicit cleanup
or join in this profile. Local move aliases and general assignment are not
part of the current source grammar.

The native and reference allocation identity is `task * 2^32 + ordinal`, with
task in `0..2^32-1` and a per-task successful-allocation ordinal in `1..2^32-1`.
Root task identity is zero. Failed allocation consumes no generation; native
worker scheduling cannot change another task's counter. Native execution aborts
if these bounds are exhausted. Task-local fault scripts select allocation
attempts by successful runtime task identity. See the
[worker-allocation record](reports/w6-worker-allocation/README.md).

The finite task checker rejects unreachable blocks. Owned Result analysis uses
actual checked worker exits rather than inventing resource-bearing alternatives:
explicit matches with an impossible owned-result arm, and propagation from an
owned-return worker with an unused nonempty error row, may therefore reject.
Use infallible `?` for the empty-error owned-return case. General branch reasoning
and dead-code validation remain open.

Native worker buffer observations are retained privately and printed by the
parent after successful join. The transcript is a normalized collection order,
not a wall-clock schedule. C11 publication/join, allocator correctness and
source/core/native correspondence remain explicit refinement boundaries.
Lean model lemmas and fixed native pointer/sanitizer observations are separate
evidence. Owned graphs/error payloads, shared memory, mutexes, nested
workers, general control flow and non-Linux native targets remain unsupported.
One bounded borrowing form is admitted by G02bb below; every other loan form
remains unsupported.
See [the owned task report](reports/w6-owned-source-tasks/README.md).

## G02bb. Scoped read loan

`ScopedReadLoansV1` extends G02b with one bounded borrowing form: a **scoped
read loan** of one parent-owned Buffer to one child, returned by the creator's
join. It is selected whenever any checked function has a `borrow<Buffer>`
parameter; a module that lends nothing keeps its G02a/G02b profile unchanged.

```qcore
concurrency checked;
fn reader(payload: borrow<Buffer>) -> i32 {
    let seen = qcore_buffer_read(payload);
    return seen;
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
    let buffer = qcore_buffer_new(42)?;
    thread scope s {
        match spawn s reader(read buffer share FULL) {
            Result::Ok { value: handle } => {
                let during = qcore_buffer_read(buffer);
                let seen = join move handle?;
                let after = qcore_buffer_read(buffer);
                drop buffer;
                return seen;
            }
            Result::Err { error: failure } => {
                let restored = qcore_buffer_read(buffer);
                drop buffer;
                return restored;
            }
        }
    }
}
```

A loan parameter is spelled exactly `NAME: borrow<Buffer>`, with **no** usage
grade: the borrower may read it any number of times and may do nothing else
with it. Grading loan reads is not part of this profile. The only capture that
produces one is `read PLACE share FULL`, against a place whose type is
`own<Buffer>`. A `share` expression names how much read authority the borrower
receives; `share FULL` is the whole of it. Fractions are card T05's grammar,
G02bc below.

A read loan is not a value. It is refused as a function result, as an entry
parameter, as a copy source, as a move source, as a `drop` operand and as a
re-loan source; `qcore_buffer_read(NAME)` is the only operation that accepts
one. A worker cannot lend onward for two independent reasons: a loan is not an
`own<Buffer>` place, and a spawned worker is a first-order leaf.

While a loan is outstanding, the lender **keeps read authority** over its own
Buffer and loses every exclusive authority over it — mutation, release, move
and re-loan are all refused, naming the loan rather than a custody transfer.
The bounds are structural: at most one loan per creation, at most one
lent *resource* at a time, and one creation may not both move and lend the
same resource. It may move one Buffer and lend another.

Creation failure restores the lender exactly and opens no loan; the reservation
window before that outcome is known grants nobody anything, read included.
Only the creator's successful join closes a loan. A child that completed but
has not been joined still holds its loan. A path that never joins its borrower
cannot leave the scope: the scope-busy rule reaches that first, and the exit
rule's own loan check is a redundant backstop.

Native emission passes the same pointer for a move and for a loan and differs
only in what the caller keeps: a moved capture is consumed, a lent one is not.
`QCORE_BUFFER_LOAN borrower generation` is announced only after a creation that
actually returned a child, and `QCORE_BUFFER_RETURN borrower generation` only
after a collection that actually returned. Concurrent reads by lender and
borrower are admitted because nothing may write a loaned resource; that is a
checker rule, not a dynamic measurement — no race detector covers this backend,
because ThreadSanitizer does not intercept C11 `thrd_create`.

Write loans, loans of anything but a Buffer, loans across nested workers,
loans held past a scope and non-Linux native targets remain unsupported. See
[the read-loan report](reports/remaining-t04/README.md). Fractional and
multi-reader shares are G02bc.

## G02bc. Counted read shares and the checked Buffer write

`ScopedReadSharesV1` extends G02bb by *counting* the lent authority. Each
`read PLACE share S` capture transfers `S` out of a per-resource budget of
`FULL` (1024, the same unit as the concurrency model's `FULL_ACCESS`), so one
allocation may be lent to at most two borrowers whose shares sum to no more
than `FULL`. It is selected when some capture lends a share other than `FULL`,
or when the module writes a Buffer; a module that does neither keeps its
G02a/G02b/G02bb profile unchanged.

```qcore
concurrency checked;
fn reader(payload: borrow<Buffer>) -> i32 {
    let seen = qcore_buffer_read(payload);
    return seen;
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
    let buffer = qcore_buffer_new(1)?;
    let prepared = qcore_buffer_write(buffer, 42);   // exclusive authority
    thread scope s {
        match spawn s reader(read buffer share 512) {
            Result::Ok { value: first } => {
                match spawn s reader(read buffer share 512) {
                    Result::Ok { value: second } => {
                        let during = qcore_buffer_read(buffer);
                        let a = join move first?;
                        let b = join move second?;
                        let restored = qcore_buffer_write(buffer, 7);
                        let after = qcore_buffer_read(buffer);
                        drop buffer;
                        return after;
                    }
                    /* … creation-failure arms elided … */
                }
            }
        }
    }
}
```

A share expression is `FULL`, a decimal literal, a parenthesised share, or a
sum of shares. It must evaluate to a positive value at most `FULL`; zero, an
over-full value and a named share are represented refusals, and a name that
does not resolve is refused earlier still by name resolution. This profile has
no declared share constants.

The budget is over the authority handed **out**. Lending transfers `S` from the
resource's unlent authority to the borrower; the outstanding total may never
exceed `FULL`, and an over-allocation is refused as a whole — no partial share
is ever granted. At most two holders exist at once; a third is refused by the
declared bound even when authority remains. The owner's own read is *not*
budgeted: it is retained across committed lending exactly as G02bb retains it,
and is sound only because a write demands the reconstructed exclusive
authority. The separation-logic reading, in which the owner's read is itself a
consumed fraction, is not this profile.

`qcore_buffer_write(BUFFER, VALUE)` evaluates to `unit`, takes one exact
`own<Buffer>` name and one i32, and demands exclusive authority. It is
therefore refused for the whole window in which any share is outstanding —
including after only one of two borrowers has been joined — and a borrower
cannot reach it at all, because a read loan is not an `own<Buffer>` binding.
Exclusive authority, and with it the write, the release and the move, return
only when the *last* share comes back: one join out of two reconstructs
nothing.

Two borrowers of one allocation are two holders of one identity. The backend
hands both the same pointer, so the allocation's live bytes are counted once,
and lending never consumes the lender's binding — only its release does.
`QCORE_BUFFER_WRITE task generation value` is announced by an admitted write;
the loan and return trace lines are unchanged from G02bb.

Disjoint slicing, write shares, more than two simultaneous readers, several
lent resources at once, nested workers and non-Linux native targets remain
unsupported. See [the read-share report](reports/remaining-t05/README.md).

## G02bc. Tagged error payload

`ScopedErrorPayloadV1` extends G02b with one bounded failure-edge transfer: an
explicitly tagged error variant carrying exactly one owned Buffer out of one
child, through its completion capsule and its creator's join. It is selected
whenever any checked function declares an error payload; a module whose error
variants are all payload-free keeps its G02a/G02b/G02bb profile unchanged.

```qcore
concurrency checked;
error Recovery { Failed(own<Buffer>); }
fn worker(payload: own<Buffer> use one) -> i32 effects { alloc, throws<Recovery> } {
    match qcore_buffer_new(7) {
        Result::Ok { value: scratch } => {
            let seen = qcore_buffer_read(payload);
            drop payload;
            drop scratch;
            return seen;
        }
        Result::Err { error: exhausted } => {
            raise Recovery::Failed(payload);
        }
    }
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
    let buffer = qcore_buffer_new(42)?;
    thread scope s {
        match spawn s worker(move buffer) {
            Result::Ok { value: handle } => {
                match join move handle {
                    Result::Ok { value: seen } => {
                        return seen;
                    }
                    Result::Err { error: failure } => {
                        let recovered = qcore_error_payload(failure);
                        let value = qcore_buffer_read(recovered);
                        drop recovered;
                        return value;
                    }
                }
            }
            Result::Err { error: refused } => {
                drop buffer;
                return 0;
            }
        }
    }
}
```

A payload variant is declared exactly `VARIANT(own<Buffer>);` inside an
`error NAME { … }` declaration. Scalar payloads, `borrow<Buffer>` payloads,
empty parameter lists and several payload fields are each refused at the
declaration. A declared error row is **uniform**: either every variant carries a
payload or none does, so a joined error always holds exactly one Buffer and the
parent's take is total. A mixed row is a represented refusal, not a widening.

`raise NAME::VARIANT(BUFFER);` moves one owned Buffer into the raised error, and
`raise NAME::VARIANT;` stays the payload-free spelling. The declaration decides
which is required, in both directions. `AllocError` and `SpawnError` never carry
a payload — the runtime raises them after a failure that acquired nothing — and
the task entry may not declare a payload row at all, because it has no external
custodian, exactly as it may not declare an `own<Buffer>` result.

`let NAME = qcore_error_payload(ERROR);` is the only way a payload leaves an
error. The error keeps its tag and loses its payload, so a second take is
refused, and until the take happens the bound error may neither be discarded at
an exit nor re-raised. A taken payload is an ordinary owned Buffer with the
ordinary duty: forgetting it is a leak on the path that forgot it.

A worker's error exit must account for every identity it acquired: locals it did
not carry must already be released, and the completion names exactly the one
identity the outcome carries. A creation that failed transfers nothing. A second
join of the same handle is refused, so the payload can never be collected twice.
One Buffer may be a normal owned result on one path and a payload on another;
the checker keeps the two exits apart.

Two core gates cannot be reached from this source surface, because the uniform
row rule and the entry rule refuse earlier: propagating a payload-carrying
Result with `?` from a function that also declares a builtin failure, and
re-raising a bound error that still holds its payload. Both are checked on
mutated IR so that widening the surface later cannot silently admit them.

Native emission carries the payload in the result value alongside the error tag
and clears every stale copy as it moves, printing
`QCORE_TASK_RESULT task ERROR_BUFFER tag generation` distinctly from a
payload-free `ERROR tag`.

Several payloads, recursive owned graphs, arbitrary error ABIs, payloads on
nested workers, loans inside payloads and non-Linux native targets remain
unsupported. See [the error-payload report](reports/remaining-t06/README.md).

## G02c. Guarded fixed-footprint mutex

`CheckedMutexGuardsV1`, profile id `source.mutex.guard.v1`, lowers a lexical
guard onto the `CheckedMutexInvariantV1` model checker
(`crates/qcore-core/src/mutex_invariant.rs`). It is selected by the presence of
a `mutex guards` declaration, ahead of the scoped-task routes, and it composes
with none of them: a module that also creates a task is refused.

```qcore
error Fault { Broken; }
fn main() -> unit {
    thread scope s {
        let buf = protected 4;          // one shared resource, 4 bytes
        let m = mutex guards buf;       // protected footprint fixed here
        with lock m as g {
            open g;                     // the invariant may now be broken
            write buf with g;
            close g;                    // restored, by a declared predicate
            let b = borrow buf with g;
            read buf with g;
            drop b;                     // the borrow dies inside its section
        }
        with lock m as g2 { read buf with g2; }
    }                                   // scope exit destroys m, then buf
}
```

```ebnf
module      = { error_declaration } , entry ;
entry       = "fn" , NAME , "(" , ")" , "->" , "unit" , "{" , scope , "}" ;
scope       = "thread" , "scope" , NAME , "{" , { outer } , "}" ;
outer       = "let" , NAME , "=" , "protected" , NAT , ";"
            | "let" , NAME , "=" , "mutex" , "guards" , NAME , ";"
            | ( "read" | "write" ) , NAME , ";"
            | "with" , "lock" , NAME , "as" , NAME , "{" , { inner } , "}" ;
inner       = ( "open" | "close" ) , NAME , ";"
            | ( "read" | "write" ) , NAME , "with" , NAME , ";"
            | "let" , NAME , "=" , "borrow" , NAME , "with" , NAME , ";"
            | "drop" , NAME , ";"
            | "raise" , NAME , "::" , NAME , ";" ;
```

`mutex guards R;` fixes the protected footprint syntactically at the
declaration, so a dynamic protected shape is not expressible rather than being
detected later. Creation consumes the declaring task's `FULL_ACCESS` share over
`R`, so a second `mutex guards R;` cannot obtain it (`Q617`) and an unguarded
access to `R` has no fallback authority (`Q611`). A guard obtained over one
generation authorizes only that generation's footprint (`Q610`).

Guard names bind only inside their own `with lock` body, so a guard-derived
name mentioned after the block is `Q013`, not a concurrency diagnostic. A write
requires the invariant to be declared broken first, and both the normal block
end and a `raise` require it restored and every guard-derived borrow released
(`Q613`, `Q614`, `Q615`). `close g;` supplies a *declared* witness: every
certificate from this route carries `InvariantPredicateDeclared` and
`DeclaredFootprintShape`, and never `GuardFootprintAuthority` or
`SynchronizedCustodyEdges`. A nested `with lock` lowers to an explicit
unsupported-construct event and is refused at `Q622`; `defer` inside a critical
section, and every other statement outside the grammar above, is refused at
`Q600`. Scope exit destroys each generation and then each resource, under the
model's quiescence rules.

Native emission is one Linux thread using C11 `mtx_t` and real heap footprints;
the reference execution of the checked event list must reproduce the same
observation. There is no contention, no second thread, no condition variable,
no try/timed lock, no reader/writer lock and no lock ranking in this profile,
and the invariant predicate itself is declared, never proved. See
[the T03 record](reports/remaining-t03/README.md).

## G03. Source parameter usage grades

```ebnf
usage_annotation = "use", grade ;
grade            = "zero" | "one" | "affine" | "relevant" | "many"
                 | NAT
                 | "[", NAT, ",", (NAT | "inf"), "]" ;
```

A natural grade denotes `[n,n]`. Bounds must fit `u64`; finite lower bounds must not exceed upper bounds. Named meanings are `[0,0]`, `[1,1]`, `[0,1]`, `[1,inf]` and `[0,inf]`, respectively. Symbolic grade expressions are not evaluated by this source parameter parser. The independent usage algebra has checked addition, multiplication, interval hull and containment semantics; that algebra does not broaden this surface grammar. See `frontend.rs:838–878` and `crates/qcore-core/src/usage.rs`.

## G04. Closed quantitative indices

```ebnf
quant_type = "quant", "<", (scalar | vector), [where_clause], ">" ;
scalar     = expr ;
vector     = ("credit" | "live"), ",", "{", field, {",", field}, "}" ;
field      = IDENT, "=", expr ;
where_clause = "where", where_item, {("," | "and"), where_item} ;
where_item = IDENT, ":=", expr | expr, relation, expr | "true" ;
relation   = "==" | "!=" | "<" | "<=" | ">" | ">=" ;
expr       = product, {"+", product} ;
product    = atom, ["*", atom] ;
atom       = NAT | IDENT | "(", expr, ")"
           | ("min" | "max" | "monus"), "(", expr, ",", expr, ")" ;
```

The product rule has an additional semantic restriction: one operand must parse to a literal node. Two names are rejected even if later definitions would resolve both to constants. A parenthesized literal may still be a literal node. An unparenthesized product chain is not a general multiplication expression under this grammar.

Scalar shorthand is credit-only and requires exactly one declared credit axis. Vector fields are nonempty, uniquely named, mode-compatible and dimension-checked. Every local definition is pure, fresh and closed; forward definitions are allowed, dependency cycles and unknown/runtime names are rejected. Constraints must evaluate true. Unreferenced definitions without an inferable dimension are not silently assigned a dimension.

Numbers and arithmetic use checked `u128` natural semantics. `monus(a,b)` requires `b <= a` and computes the checked difference; it does not saturate an invalid subtraction to zero. There is no free subtraction, division or nonlinear symbolic solver. Overflow and structural limits are errors, not wraparound or assumed inequalities.

The parser rejects atom nesting beyond its depth-32 condition and addition beyond its term-limit condition of 64 added terms. Resolution/evaluation impose additional depth/dependency guards in the same implementation. These are explicit implementation limits, not mathematical restrictions on natural numbers.

`quant<cost,...>` is rejected by this resolver as a future total/peak profile. `quant<access,...>` is skipped by this closed-index parser for separate handling; skipping it is not source-level access verification. The axis environment is now built and validated unconditionally, including modules without quantitative type occurrences (R05).

Source: `crates/qcore-frontend/src/quant_index.rs`, `frontend.rs:1519–1548`. A supported canonical example is:

```qcore
module contract;
quant axis chunks : credit<Count<Chunk>>;
extern fn keep(q: quant<n where n := 5> use one) -> unit;
```

## G05. Resource loader v1

```ebnf
resource_module = "module", "contract", ";", resource_fn, {resource_fn} ;
resource_fn = "extern", "fn", IDENT, "(", [parameter, {",", parameter}], ")",
              "->", result_abi, "contract", "{", resource_clause,
              {resource_clause}, "}", ";" ;
parameter   = IDENT, ":", parameter_abi ;
parameter_abi = scalar_abi | pointer_abi ;
result_abi  = "unit" | parameter_abi ;
scalar_abi  = "bool" | "i8" | "i16" | "i32" | "i64" | "isize"
            | "u8" | "u16" | "u32" | "u64" | "usize" ;
pointer_abi = "ptr", "<", "mut", ",", "u8", ">"
            | "ptr", "<", "const", ",", "u8", ">"
            | "ptr", "<", "mut", ",", "ptr", "<", "mut", ",", "u8", ">", ">" ;
resource_clause = "requires", "resource_contract_v1", ";"
                | "ensures", "normal", resource_predicate, ";" ;
resource_predicate = "resource_borrowed", "(", IDENT, ")"
  | ("resource_owned" | "resource_consumed" | "resource_released" | "resource_out"),
    "(", IDENT, ",", nullability, ")"
  | ("resource_stored" | "resource_removed"),
    "(", IDENT, ",", IDENT, ",", nullability, ")"
  | "resource_return_none", "(", ")"
  | "resource_return_owned", "(", nullability, ")"
  | "resource_return_reallocated", "(", IDENT, ",", nullability, ")" ;
nullability = "nullable" | "nonnull" ;
```

Additional static conditions are mandatory: exactly one marker, exactly one mode per parameter, exactly one result mode, unique names, compatible pointer/scalar types, valid distinct owned parent references, and an owned reallocation source. These conditions cannot be represented by context-free productions alone. Signature usage annotations, extra declarations, unsupported predicates and other exit selectors are rejected atomically, not ignored. See contract specification C04–C06 and `crates/qcore-contract/src/contract_loader.rs`.

The grammar describes tokens, not required whitespace. It provides no general predicate-call nesting. It also does not turn an admitted trusted declaration into proof that a C function obeys it.

## G06. Observation predicate grammar

```ebnf
predicate = or_expr ;
or_expr   = and_expr, {"or", and_expr} ;
and_expr  = comparison, {"and", comparison} ;
comparison = sum, {relation, sum} ;
sum       = product, {("+" | "-"), product} ;
product   = unary, {"*", unary} ;
unary     = ("-" | "!" | "not"), unary | primary ;
primary   = NAT | IDENT | "true" | "false" | "(", predicate, ")" ;
relation  = "==" | "!=" | "<" | "<=" | ">" | ">=" ;
```

Reserved operator/literal words take their parser meanings rather than binding arbitrary facts. The interpreter uses signed `i128`, checked arithmetic, nonzero truth, eager evaluation and left-associated binary operators, including repeated comparisons. Thus `a < b < c` is not mathematical chained comparison syntax; write `(a < b) and (b < c)` to express that intended predicate.

A predicate is limited to 512 non-EOF tokens and the parser's depth-64 guard. Signed decimal literals are range-checked with sufficient magnitude capacity, including the full i128 minimum (R07). Out-of-range literals and checked arithmetic overflow reject. Negating an already minimal supplied fact still overflows; literal support does not authorize unchecked arithmetic.

Only `requires` and `ensures normal` envelopes are admitted by this consumer, with at least one postcondition. Function-call predicates, `old(...)`, `resource_*` calls, quant clauses, `&&`/`||`, typed errors and `any` exits do not extend this grammar. A missing fact or overflow errors even in `true or missing_fact`, and a malformed postcondition errors even when a precondition is false. Source: `crates/qcore-contract/src/contract_observation.rs`.

## G07. Extended grammar and conformance

`QCORE_DESIGN_GRAMMAR_r0_3.bnf` collects the nineteen BNF blocks from revision 0.2, adding the current optional module header. It preserves the intended full language vocabulary, including source constructs without driver lowering. Its lexical terminal placeholders are `IDENTIFIER-TOKEN`, `INTEGER-TOKEN`, `FLOAT-TOKEN`, `STRING-TOKEN` and `CHAR-TOKEN`.

That target grammar is neither a parser generator input already used by the compiler nor the grammar of every policy consumer. In particular, retained policy token expressions and resource predicate calls are defined at their consumer boundary above; their structural acceptance is not a new general-purpose contract proof rule. Future implementation work MUST keep the profile grammars, type rules, lowering coverage and negative tests synchronized instead of conflating the target with today's supported syntax.

## G08. Source-contract service configuration v1

This JSON format configures the audit service; it adds no QCore source syntax.
The base fields below are required; `frame_helpers` is optional on each binding.
Unknown fields reject:

```json
{
  "schema_version": 1,
  "packages": ["packages/example"],
  "bindings": [
    {"library": "example", "provider": "example.boolean", "contract": "policy.qcore"}
  ]
}
```

Relative package and policy paths resolve against the configuration directory.
There must be 1–256 distinct nonempty package paths and 1–4096 bindings with
unique nonempty library names and valid provider IDs. Configuration is bounded
at 1 MiB, each UTF-8 policy at 8 MiB, and retained policy bytes together at
32 MiB. Policy bytes are snapshotted once. Package identities, manifest target
support and the existing Boolean-field checker capability must validate before
dispatch. The sealed import supplies the model-library identity; configuration
cannot manufacture it or expand import coverage. Missing bindings/packages,
incompatible targets and rejected witnesses fail without provider fallback.

`SourceContractPolicies::load` loads policies for direct comparison without
requiring installed packages. `SourceContractRegistry::load` additionally verifies
and registers explicit process packages. Its `verify` method accepts an opaque
Clang-imported summary and returns a checked certificate plus contract evaluation
only after existing trusted replay succeeds. Source: `crates/qcore-audit/src/source_contract.rs`.
This implementation supports the existing bounded Boolean-field semantic profile;
it is not a general contract solver. See the [execution record](reports/w5-configured-source-contracts/README.md).

A binding may additionally declare `"frame_helpers": [{"source": "reset.c",
"function": "reset_metadata", "record": "widget", "contract": "reset.qcore"}]`.
Omission means no helper selection. At most 256 helpers are allowed per binding,
with unique C function names; function/record identifiers are at most 128 ASCII
bytes. Source paths must be contained relative paths within the selected library
root. Policy paths resolve from the configuration directory and their snapshots
share the existing 32 MiB aggregate policy budget.

`import_configured_frame_helpers` requires exactly one selected compilation entry
per source, canonical source containment, an absolute compilation directory, and
strict preservation of supported preprocessing options. Unrepresented options
reject. Source reads are bounded at 8 MiB each and retained imported helper source
at 32 MiB per call; this is not a new bound on transient Clang preprocessing.
`import_configured_frame_helpers_compatibility` explicitly preserves the existing
pinned audit's historical option projection; there is no automatic fallback.
`configured_frame_helper_policies` in the audit sidecar records snapshotted
selection metadata, not proof that every configured helper was executed.

## Configured C audit composition

`qcore-audit` schema-1 configuration now accepts an optional `source_contracts`
object with `configuration`, `source_rules` and explicit `libraries` entries
(`library`, checkout-relative `source_root`). All are required within that object;
unknown fields reject. The exact configuration and helper-resolution rules are
in the [audit command documentation](../crates/qcore-audit/README.md#configured-source-contract-processes).
This input selects existing bounded Boolean source import and evidence-v2 replay;
it does not extend QCore source grammar or promote the C resource verdict.
