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
source        = [module_header], {module_import | declaration} ;
module_import = "import", IDENT, {".", IDENT}, ";" ;
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

Bundled semantic modules require an explicit top-level `import` in each source
file. The current registry contains `std.concurrency`:

```qcore
import std.concurrency;
concurrency checked;
```

Import paths are dot-separated identifiers terminated by a semicolon. Imports
may occur between top-level declarations, after the optional module header, and
apply to the whole file. Repeated imports are idempotent. Unknown module names
reject with `Q014`; concurrency forms without their import reject with `Q015`,
including in uncalled bodies and non-executable modules. Each input file is
checked before combining declarations: another file's import cannot grant it
concurrency. Importing vocabulary does not discharge operation checks or make an
unsupported profile executable. The task profile still requires its
`concurrency checked;` marker. User-file/package resolution, import aliases,
wildcards, reexports and implicit/transitive imports are not implemented.

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

`qcore_acquire` and `qcore_check` take literal Boolean controls in this profile. They are model operations, not general malloc calls or arbitrary runtime conditions. General expressions, argument-taking calls, indirect calls, calls through a shadowing local, calls to extern declarations, nested source control flow, source lambdas and source threading are not accepted by this driver body path. Local `use` annotations are explicitly rejected here rather than ignored. A bounded extern call is accepted by the separate `source.extern-call.v1` path of §G02d, which is selected before this one and does not widen it: this body path still refuses every call whose target is not a zero-parameter source function.

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
import std.concurrency;
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
The native driver supports Linux C11 threads, macOS through an emitted pthread
adapter, and Windows through an emitted winpthreads-derived C11 thread subset;
it explicitly rejects other targets. Its start record survives until successful native join; failed
join aborts without reclaiming possibly accessed storage. These rules do not
establish general C11/shared-memory or cross-platform refinement. See the
[recovery execution record](reports/w6-task-recovery/README.md).

## G02b. Owned-buffer task execution

`ScopedOwnedTasksV1` extends G02a with one concrete builtin type:
`own<Buffer>`. Each Buffer is a malloc-backed allocation with a fresh generation
and one immutable i32 payload. Its resource identity is separate from source
slot and scalar identities. This is not arbitrary `own<T>` elaboration.

```qcore
import std.concurrency;
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
workers, general control flow and targets other than Linux, macOS and Windows remain unsupported.
One bounded borrowing form is admitted by G02bb below; every other loan form
remains unsupported.
See [the owned task report](reports/w6-owned-source-tasks/README.md).

## G02bb. Scoped read loan

`ScopedReadLoansV1` extends G02b with one bounded borrowing form: a **scoped
read loan** of one parent-owned Buffer to one child, returned by the creator's
join. It is selected whenever any checked function has a `borrow<Buffer>`
parameter; a module that lends nothing keeps its G02a/G02b profile unchanged.

```qcore
import std.concurrency;
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
loans held past a scope and targets other than Linux, macOS and Windows remain unsupported. See
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
import std.concurrency;
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
lent resources at once, nested workers and targets other than Linux, macOS and Windows remain
unsupported. See [the read-share report](reports/remaining-t05/README.md).

## G02bc. Tagged error payload

`ScopedErrorPayloadV1` extends G02b with one bounded failure-edge transfer: an
explicitly tagged error variant carrying exactly one owned Buffer out of one
child, through its completion capsule and its creator's join. It is selected
whenever any checked function declares an error payload; a module whose error
variants are all payload-free keeps its G02a/G02b/G02bb profile unchanged.

```qcore
import std.concurrency;
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
nested workers, loans inside payloads and targets other than Linux, macOS and Windows remain
unsupported. See [the error-payload report](reports/remaining-t06/README.md).

## G02c. Guarded fixed-footprint mutex

`CheckedMutexGuardsV1`, profile id `source.mutex.guard.v1`, lowers a lexical
guard onto the `CheckedMutexInvariantV1` model checker
(`crates/qcore-core/src/mutex_invariant.rs`). It is selected by the presence of
a `mutex guards` declaration, ahead of the scoped-task routes. It composes with
none of them: a module using the scoped-task `spawn`/`join` expressions is
refused. Its own bounded child-task form is a nested `thread scope` block
(card T03-F).

```qcore
import std.concurrency;
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
        let f = fact buf;               // a guard-scoped refinement fact
        with lock m as g3 {
            assert f with g3;
            open g3;
            read buf with g3;
            use f;
            close g3 unchanged;         // restored, and *checked*: no write
        }
        thread scope c {                // one child task, joined at `}`
            with lock m as gc { read buf with gc; }
        }
    }                                   // scope exit destroys m, then buf
}
```

```ebnf
module      = { error_declaration } , entry ;
entry       = "fn" , NAME , "(" , ")" , "->" , "unit" , "{" , scope , "}" ;
scope       = "thread" , "scope" , NAME , "{" , { outer } , "}" ;
outer       = "let" , NAME , "=" , "protected" , NAT , ";"
            | "let" , NAME , "=" , "mutex" , "guards" , NAME , ";"
            | "let" , NAME , "=" , "mutex" , "rank" , NAT , "guards" , NAME , ";"
            | "let" , NAME , "=" , "fact" , NAME , ";"
            | ( "read" | "write" ) , NAME , ";"
            | "wait" , NAME , ";"
            | "use" , NAME , ";"
            | "thread" , "scope" , NAME , "{" , { child } , "}"
            | "with" , "lock" , NAME , "as" , NAME , "{" , { inner } , "}" ;
inner       = "open" , NAME , ";"
            | "close" , NAME , [ "unchanged" ] , ";"
            | "grow" , NAME , "guards" , NAME , ";"
            | ( "read" | "write" ) , NAME , "with" , NAME , ";"
            | "let" , NAME , "=" , "borrow" , NAME , "with" , NAME , ";"
            | "drop" , NAME , ";"
            | "assert" , NAME , "with" , NAME , ";"
            | "use" , NAME , ";"
            | "wait" , NAME , ";"
            | "thread" , "scope" , NAME , "{" , { child } , "}"
            | "raise" , NAME , "::" , NAME , ";" ;
child       = (* any `outer` or `inner` form except a declaration and except
                 `drop`; a child inherits the enclosing critical section
                 lexically and holds none of its authority *) ;
```

`mutex guards R;` declares the protected footprint syntactically, and the only
way to change it afterwards is card CT34-GROWTH's `grow` statement below, under
its own rule; a dynamic protected shape is otherwise not expressible rather than
being detected later. Creation consumes the declaring task's `FULL_ACCESS` share over
`R`, so a second `mutex guards R;` cannot obtain it (`Q617`) and an unguarded
access to `R` has no fallback authority (`Q611`). A guard obtained over one
generation authorizes only that generation's footprint (`Q610`).

Guard names bind only inside their own `with lock` body, so a guard-derived
name mentioned after the block is `Q013`, not a concurrency diagnostic. A write
requires the invariant to be declared broken first, and both the normal block
end and a `raise` require it restored and every guard-derived borrow released
(`Q613`, `Q614`, `Q615`). `close g;` supplies a *declared* witness, so a
certificate covering it carries `InvariantPredicateDeclared`;
`close g unchanged;` supplies the **checked** witness and carries none.
`DeclaredFootprintShape` is on every certificate from this route, and
`GuardFootprintAuthority` and `SynchronizedCustodyEdges` never are. Unless the
module declares a lock rank (card CT33-RANKS below), a nested `with lock` lowers
to an explicit unsupported-construct event and is refused at `Q622`; `defer`
inside a critical section, and every other statement outside the grammar above,
is refused at `Q600`. Scope exit destroys each generation and then each
resource, under the model's quiescence rules.

Card T03-F's three completions, each bounded by a stated rule:

* **`close g unchanged;`** is admitted only when every statement since this
  guard's `open` was a read or a fact use — the shape the lowering can decide.
  Any other shape is refused at `Q600` and never silently downgraded to the
  declared witness.
* **`let f = fact r;` / `assert f with g;` / `use f;`** are guard-scoped
  refinement facts. Each assertion mints a fresh model identity, so relying on
  a fact after its guard was released is `Q616`, relying on one that was never
  asserted is `Q600 UnknownFact`, and a child task relying on its parent's fact
  is `Q616`.
* **A nested `thread scope c { … }`** is one child task, created where the
  block opens and joined where it closes. The child receives no share and no
  custody, so a protected footprint is reachable to it only through its own
  acquisition (`Q611` otherwise); naming the enclosing guard is `Q612`; and a
  child block written *inside* a critical section is refused because its join
  lands while the creator holds the guard (`Q600 JoinWhileGuardHeld`). A child
  declares no resource, generation or fact, and may not release a borrow it did
  not take (`Q600`). **`wait m;`** announces an acquisition and, if never
  performed, blocks the generation's destruction (`Q619`).

Card T03-GUARD-JOIN adds one opt-in form to that last rule and changes nothing
else about it:

* **`thread scope c under g { … }`**, written inside `g`'s own section, lowers
  its join to the core's `GuardLocalJoin` rather than `Join`. `under` names the
  enclosing section explicitly; outside one, or naming a different guard, it is
  a lowering refusal (`Q600`) or an unresolvable name (`Q013`). Admission is the
  core's decision, not the grammar's: the child must have been created strictly
  inside this same section, the section must be the only one held, with its
  invariant closed and no live borrow, and the child *and every task under it*
  must have named no mutex, guard or guard-scoped fact anywhere in the trace and
  hold no share or custody. Anything else keeps the same
  `Q600 JoinWhileGuardHeld` refusal, so a plain `thread scope c { … }` inside a
  section is unchanged. Because a child in this profile holds no share and no
  custody, a guard-local child cannot touch a declared resource at all: the
  admission is about the *join*, not about giving the child authority. It adds
  no premise to the certificate — every condition is checked — and each admitted
  join is named on it. `under` is reserved. See
  [the T03-GUARD-JOIN record](reports/remaining-t03-guard-join/README.md).

Card CT33-RANKS adds the ranked declaration and nothing else to the grammar:

* **`let m = mutex rank N guards r;`** declares `m`'s position in a lock order.
  `N` is one decimal literal in `0 ..= 4294967295`; any other spelling is a
  lowering refusal (`Q600`). Declaring at least one rank turns on the core's
  opt-in discipline for the whole module, and two things follow. Every mutex the
  module declares must then be ranked, or the module is refused at `Q625` — a
  partly ranked module has no order, so it is not checked over its ranked part.
  And a nested `with lock` becomes expressible: you may nest exactly where you
  have declared in which order, and the core then refuses any acquisition made
  while holding an equal-or-higher rank (`Q624`), which is what makes a
  lock-order cycle writable and refused rather than inexpressible. Nesting buys
  the right to hold two guards, not to conflate them: the inner body's frame
  carries only the inner section, so naming the enclosing guard from inside is
  `Q600`, and a nested acquisition invalidates the decidable region behind
  `close g unchanged;`. A task re-locking a generation it holds keeps its
  existing `Q600 DuplicateLockCapability`, since the rank test runs last and
  never displaces an existing refusal. Declaring no rank leaves every module
  written before this card byte-identical, including the `Q622` nesting refusal.
  A passing rank check is not a deadlock-freedom proof — see
  `docs/QCORE_SPEC_r0_3.md` for exactly what it does and does not establish.
  `rank` is reserved. See
  [the CT33-RANKS record](reports/remaining-ct33-ranks/README.md).

Card CT34-GROWTH adds one statement and nothing else to the grammar:

* **`grow g guards r;`**, written inside `g`'s own section, adds `r` to the
  protected footprint of the generation `g` guards. It names the guard, and the
  core reads the generation off it, so a growth can never reach a mutex its
  holder does not hold. Admission is the core's decision, not the grammar's: the
  invariant must be open at the growth (`Q626`); `r` must be protected by no
  live generation, this one included (`Q617`); the growing task must be the
  generation's creator and hold the full share over `r` (`Q621`); and the growth
  must move the generation's declared invariant to an identity it has never
  carried (`Q627`), which the lowering mints. A growth is a change to the
  protected region, so `close g unchanged;` is refused in a section that grew
  (`Q600`) rather than downgraded, and the section must close by declaration.
  Growth is permanent: `r` is behind the guard from then on — an unguarded
  access to it becomes `Q611` — the originally declared footprint stays
  protected, and there is no shrink or replacement form. A refused growth leaves
  the generation's shape, its declared identity and the ledger untouched, so the
  section's own `close` then names an identity the generation never took; that
  fallout is reported rather than repaired. Growth adds no premise beyond the
  `InvariantPredicateDeclared` the new identity carries, and each admitted
  growth is named on the certificate. The unconditional `Q618` fixed-shape
  refusal is unchanged and has no source form. `grow` is reserved. See
  [the CT34-GROWTH record](reports/remaining-ct34-growth/README.md).

Native emission uses one thread and C11 `mtx_t` with real heap footprints on
Linux, macOS and Windows (the latter two through the emitted compatibility layer);
the reference execution of the checked event list must reproduce the same
observation. There is no condition variable, no try/timed lock and no
reader/writer lock in this profile. A module whose trace
carries task structure (`spawn`, `join`, a custody transfer or an announced
acquisition) is checked but has **no native rendering**: the backend emits one
thread, so the module is published with no C rather than with C the backend did
not produce. See [the T03 record](reports/remaining-t03/README.md) and
[the T03-F record](reports/remaining-t03-f/README.md).

## G02d. Bounded extern calls

`source.extern-call.v1`. Selected when a source file declares an `extern fn`
and a function body calls it, after the concurrent routes: a module that also
spawns or locks stays with those routes and keeps their existing refusals.

```ebnf
extern_module = "import", "std.extern", ";",
                { error_decl | extern_decl | entry } ;
extern_decl   = "extern", "fn", NAME, "(", [ param, { ",", param } ], ")",
                "->", ABI_TYPE, ";" ;
entry         = "fn", NAME, "(", ")", "->", "unit",
                [ "effects", "{", EFFECT, { ",", EFFECT }, "}" ],
                "{", { extern_stmt }, "}" ;
extern_stmt   = acquisition | plain_call | "return", ";" ;
acquisition   = "let", NAME, "=", CALLEE, "(", [ arg, { ",", arg } ], ")",
                "succeeds", BOOL, "else", { disposition },
                "raise", ERROR_NAME, ";" ;
disposition   = CALLEE, "(", [ arg, { ",", arg } ], ")" ;
plain_call    = CALLEE, "(", [ arg, { ",", arg } ], ")", ";" ;
arg           = [ "-" ], NAT | NAME | "move", NAME | "out" ;
EFFECT        = "io" | "alloc" | "mut" ;
```

`ABI_TYPE` is exactly the resource-contract v1 projection of §G05. The callee's
contract is **not** written here: it comes from an explicit `CompileOptions`
selection loaded by the same total-or-reject loader, and an inline
`contract { … }` block on a called extern declaration is refused (`Q027`) rather
than enforced. The source declaration's parameter names, parameter types and
result type must equal the selected module's, or the call is refused.

Callable C04 shapes are exactly: scalars by value (data only), `resource_borrowed`
on `ptr<mut, u8>` or `ptr<const, u8>`, `resource_consumed(p, nonnull)` on
`ptr<mut, u8>`, `resource_return_none()` with a `unit` or scalar result,
`resource_return_owned(nullable)` returning `ptr<mut, u8>`, and — card
`EXTERN-F1` — `resource_out(p, nullable)` on `ptr<mut, ptr<mut, u8>>` plus
`resource_stored(p, owner, nonnull)` on `ptr<mut, u8>` beneath a distinct
`resource_owned(owner, nonnull)` parameter on `ptr<mut, u8>`. Contract spec
§C04.2 requires a stored parameter's owner to be a distinct `Owned` parameter,
so `resource_owned` is callable in that position and nowhere else.
`resource_released`, `resource_removed`, `resource_return_reallocated`, a
nonnull owned return, a nonnull out slot, an out slot on any other ABI type, two
out slots, an out slot beside an owned return, a store beside either, a nullable
stored parameter, a store on a `const` pointer, a store beneath itself or
beneath a non-`Owned` parameter, a nullable or `const`-pointer declared owner, a
`resource_owned` parameter that owns nothing, a nullable consumed parameter, a
custody mode on a scalar, a borrowed `ptr<mut, ptr<mut, u8>>` and a consumed
`ptr<const, u8>` are each refused by their own name (`Q024`).

`out` marks the position of the declared out slot; it names a parameter
position, never a binding, and is reserved. The `let` binder names the identity
the callee writes into that slot. `move NAME` at a declared store removes the
binding from the body exactly as a consumption does, so every later use is
refused; where the custody went is declared by the contract, not by the call
site, and is recorded on the certificate as a premise.

`succeeds BOOL` is the fixture's literal outcome control, exactly as in
`qcore_acquire`/`qcore_check`: a controlled model operation, not a claim about
when a production allocator fails. Both arms are exercisable, and the emitted C
compares the real result's nullness against the declared control and exits with
status 70 if they disagree — for an out slot the emitted local starts `NULL`, so
a callee that never wrote it is observed as the failure arm rather than
believed. The `else … raise ERROR;` arm is mandatory for each acquisition
source, an owned nullable return and an out slot alike (`Q025`).

Card `EXTERN-ERR-REL` gives that arm its dispositions. Between `else` and
`raise` the handler names one call per still-live identity, and the arm is
admitted only when it *is* this acquisition's established cleanup edge: the
sequential lowering `qcore_core::cleanup_lowering` releases every identity live
before a failing acquisition in reverse acquisition order, and the arm must
release the same identities, each exactly once, in that order. This is a
rendering, not a second rule — an extern-acquired identity is released only by
calling a declared `resource_consumed` callee, and the profile cannot invent
one. A disposition that acquires, that stores, that discharges other than
exactly one identity, that names an identity not live on the arm, that runs in
another order, or whose derived effect row the entry's declared row does not
admit is refused (`Q025`), as is an identity that reaches the raise still live
— the v1 abandonment refusal, narrowed to the undispositioned case and not
weakened. An acquiring call that also moves custody out is refused there too,
because a declaration constrains only its normal exit. The arm is a path: a
disposition never removes a binding from the success path. That is what makes a
store reachable — it needs two simultaneously live identities, the child and its
declared owner, which the second acquisition's arm now accounts for.

The derived effect row is `{io}`, plus `alloc` exactly when the call establishes
fresh custody through either acquisition source and `mut` exactly when some
parameter is a `mut` pointer; the entry's declared row must admit it, and an
absent row therefore admits no extern call at all. Length
and extent are not transported, so no declaration here constrains how many bytes
a callee touches. Custody liveness, leaks and resource flow are checked by the
established sequential passes over the same lowering, not by a second extern
code. Emission declares each callee from the ABI table and calls it directly;
which object file supplies the symbol is the builder's premise. See
[the extern-call record](reports/extern-call-v1/README.md) and
[the out/stored extension](reports/extern-call-f1/README.md) and
[the error-arm release record](reports/remaining-extern-err-rel/README.md).

## G02e. Flat-epoch quantitative bound

`source.epoch.bound.v1`. Selected before every other concurrent route by the
`epoch` marker on a thread scope, because an epoch scope is also a
`thread scope` and a module declaring a quantitative epoch must not be handed to
a route that models no accounting. Lowers onto the unedited `FlatEpochBoundV1`
checker (`qcore_core::epoch_accounting`, diagnostics `Q630`–`Q641`).

```ebnf
epoch_module = "import", "std.concurrency", ";", entry ;
entry        = "fn", NAME, "(", ")", "->", "unit", "{", epoch_scope, "}" ;
epoch_scope  = "thread", "scope", NAME, "epoch", "{", { header | worker }, "}" ;

header       = cell | allocation | overhead | latency_claim ;
cell         = "let", NAME, "=", "share", NAT, ";" ;
allocation   = "bound", NAT, "effort", NAT, ";" ;
overhead     = "overhead", "scope", NAT, "thread", NAT, [ "measured", NAT ], ";" ;
latency_claim= "bound", "effort", "as", "latency", ";" ;

worker       = "thread", "scope", NAME, "{", { clause }, "}" ;
clause       = "read", NAME, ";" | "write", NAME, ";"
             | "peak", NAT, ";" | "effort", NAT, ";"
             | "credit", NAT, "effort", NAT, ";"
             | "retained", NAT, ";"
             | "join", "(", ( "move" | "drop" | "close" ), ")", ";"
             | worker ;
```

Two axes are declared: `memory` carries every byte figure and `effort` carries
the profile's work quantity. `allocation` and `overhead` each appear exactly
once; `credit` appears exactly once per worker; `peak`, `effort`, `retained` and
the join clause at most once each. Every `NAT` is read at `u64` and an
out-of-range literal is refused, never wrapped.

The join clause is the creator's act, written in the child's block:
`join(move)` transfers the completed child's result to the creator,
`join(drop)` releases it there, and `join(close)` is the representable claim
that the result stopped being live at completion — refused by the core at
`Q634`. A worker with no join clause lowers to an unjoined task (`Q635`). The
parentheses are load-bearing: a bare `join` is claimed by the scoped-task
profile's expression scanner, so this profile's join is always written
`join(...)`.

Shapes the profile does not model become *program content* the core refuses
rather than grammar the lowering hides: `write NAME;` is
`UnsupportedEpochShape::SharedMutation` and a nested worker block is
`NestedTask` (both `Q632`); a shared entry naming something other than a
declared cell keeps an identity the storage list does not contain (`Q631`); a
repeated `read` of one cell in one worker is not deduplicated (`Q630`); and
`overhead … measured RUNS;` offers a native measurement in place of the declared
figures, which is `Q640`. Everything outside this grammar — a clause word the
profile does not have, a missing header, a second declaration, a `spawn`/`join`
task expression, `join(close)` with nothing retained — is refused at `Q600`.

`bound effort as latency;` is a request, not a grant: the lowering records it
and the driver puts it to `epoch_wall_clock_bound`, which always refuses
(`Q639`). This profile has no schedule model, so no quantity it derives is ever
a time.

Every quantity here is a **declared premise** of `FlatEpochBoundV1`
(`DeclaredTaskFootprints`, `DeclaredRuntimeOverhead`,
`DisjointSharedStorageCells`, `NoScheduleModel`); nothing on this route imports
a peak, a result, a credit or an overhead figure from C, from an allocator or
from a measured run. The route is **check-only**: an admitted epoch is a checked
accounting statement about a declared shape, so it emits no C, runs no reference
execution and implies no native run. See
[the epoch source record](reports/remaining-t08-epoch-src/README.md).

## G02f. Flat-epoch bound over a checked task region

`source.epoch.tasks.v1`. Selected before G02e by an `epoch` marker **and** the
scoped-task profile's own `concurrency checked;` declaration: a module carrying
both is accounting for a real task region, not for declared worker shapes. The
entry's first statement is an epoch *plan*; the rest of the entry is an ordinary
scoped-task region. No diagnostic code is added by this route.

```ebnf
composed_entry = "fn", NAME, "(", ")", "->", result, "{", plan, { statement }, "}" ;
plan           = "thread", "scope", NAME, "epoch", "{", { header | row }, "}" ;
header         = allocation | overhead | latency_claim ;   (* G02e's, minus `cell` *)
row            = "thread", "scope", WORKER, "{", { clause }, "}" ;
clause         = "move", ";" | "copy", ";"
               | "peak", NAT, ";" | "effort", NAT, ";"
               | "credit", NAT, "effort", NAT, ";"
               | "retained", NAT, ";"
               | [ "join", "(", ( "move" | "drop" | "close" ), ")", ";" ] ;
```

`WORKER` names a source function the region spawns. Rows naming `WORKER`, in
declaration order, correspond one-to-one with that worker's `TrySpawn` sites in
the checked task IR, in source order; a declared row with no matching creation
and a creation with no declared row are both refused, per worker. A row's
ordered `move;`/`copy;` clauses must equal that creation's ordered argument
operands, and `retained BYTES;` is required exactly when the spawned worker's
checked result type is an owned Buffer.

The `join` clause is **optional here**: the creator's disposal is derived from
the checked IR (the handle's joins, the outcome's eliminations, and whether the
bound value is released), and a written clause must equal what was derived. The
task count, each task's identity and `joined` are derived and cannot be
declared at all.

No byte or work figure is derivable: this backend's `Buffer` carries no extent
(`TryAllocateBuffer` takes an initial scalar value, not a size) and the profile
has no cost model, so `peak`, `effort`, `credit`, `bound` and `overhead` stay
declared premises exactly as in G02e.

Refused at `Q600`, each with a named follow-on: shared storage cells and
`read`/`write` clauses (`T08-COMPOSE-SHARED`), read-loan captures
(`T08-COMPOSE-LOAN`), a mutex generation anywhere in the composed body
(`T08-COMPOSE-GUARD`, ranked or not), a joined owned result the creator hands on
rather than releasing or keeping (`T08-COMPOSE-RESULT-REUSE`), a plan that is
not the entry's first statement, and a second epoch scope. The profile's own
refusals still come from the core's `Q630`-`Q641` band. Emission and reference
execution are the scoped-task route's own, byte for byte; the epoch is
check-only on top. See
[the composition record](reports/remaining-t08-compose/README.md).

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
quant_type = "quant", "<", (scalar | vector), [where_clause], ">"
           | "quant", "[|", (scalar | vector), [where_clause], "|]" ;
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

`quant[| ... |]` is a delimiter alias for `quant< ... >` over the identical interior grammar: `[|` and `|]` are single lexer symbols, both spellings resolve to the same closed-index type, and the opener fixes the required closer, so a mixed pair such as `quant[| ... >` is refused, never repaired. A bare `|` remains a lexer error. The alias exists because `<`/`>` collide visually with where-clause comparison operators; it adds no semantic surface.

`quant<cost,...>` is rejected by this resolver as a future total/peak profile. `quant<access,...>` is skipped by this closed-index parser for separate handling; skipping it is not source-level access verification. The axis environment is now built and validated unconditionally, including modules without quantitative type occurrences (R05).

Source: `crates/qcore-frontend/src/quant_index.rs`, `frontend.rs:1519–1548`. A supported canonical example is:

```qcore
module contract;
quant axis chunks : credit<Count<Chunk>>;
extern fn keep(q: quant<n where n := 5> use one) -> unit;
extern fn hold(q: quant[|n where n := 5|] use one) -> unit;
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
