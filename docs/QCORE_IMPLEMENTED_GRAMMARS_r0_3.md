# Quirin implemented input grammars — revision 0.3

> **Name.** The language is **Quirin**; its command is `quirin`. The `QCore`
> spelling is retained for the core component — Rust namespaces, the emitted-C
> surface, the diagnostics codes, the `.qcore` extension and this file's name —
> and every such identifier in this document is unchanged by the branding.

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
extern_stmt   = acquisition | scalar_binding | scalar_literal | plain_call
              | "return", ";" ;
acquisition   = "let", NAME, "=", CALLEE, "(", [ arg, { ",", arg } ], ")",
                "succeeds", BOOL, "else", { disposition },
                "raise", ERROR_NAME, ";" ;
disposition   = CALLEE, "(", [ arg, { ",", arg } ], ")" ;
scalar_binding = "let", NAME, "=", CALLEE, "(", [ arg, { ",", arg } ], ")",
                 [ "succeeds", [ "-" ], NAT ], ";" ;
scalar_literal = "let", NAME, "=", [ "-" ], NAT, ";" ;
plain_call    = CALLEE, "(", [ arg, { ",", arg } ], ")", ";" ;
arg           = [ "-" ], NAT | NAME | "move", NAME | "out" ;
EFFECT        = "io" | "alloc" | "mut" ;
```

Two of those forms are card `STD-IO-COMPOSE`'s and both stay inside the value
namespace card `PIPE-SCALAR-RESULT` opened. `scalar_literal` binds a number
written at the site: it calls nothing, so it derives no effect, admits no
argument, opens no obligation and can carry no custody in any position — the
narrowest widening that lets a rendered value originate in the module that
renders it (`EXTERN-SCALAR-LITERAL`). The optional `succeeds` tail on
`scalar_binding` is the site's **declared outcome**, the same concept the
acquisition adapter already spells with that word: `succeeds BOOL` declares
which arm a nullable owned return takes and `succeeds INTEGER` declares which
number a scalar result carries. The two are told apart by the literal alone, so
no malformed adapter tail can be read as the other and `Q025` keeps its exact
shape; no reserved word was added. Neither control is believed by the emitted
program, which compares the real result and exits 70 on disagreement.

A bound value whose number the source did **not** declare is still a value: it
may be passed at a declared scalar parameter exactly as before. What it may not
do is be *rendered*, which is a `source.std-io-extern.v1` rule (§G02i) and not
one of this route's — this route renders nothing.

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
`resource_owned(owner, nonnull)` parameter on `ptr<mut, u8>` — and, card
`EXTERN-MODES`, `resource_released(p, nonnull)` on `ptr<mut, u8>` plus
`resource_return_reallocated(p, nullable)` returning `ptr<mut, u8>` over a
subject declared `resource_owned(p, nonnull)` on `ptr<mut, u8>`. Contract spec
§C04.2 requires a stored parameter's owner to be a distinct `Owned` parameter
and §C04.3 requires a reallocation subject to be `Owned`, so `resource_owned` is
callable in exactly those two designated positions and nowhere else.
`resource_removed`, a nonnull owned return, a nonnull reallocated return, a
reallocation subject that is out of range, not `Owned`, nullable or on any other
pointer, a reallocated result on any other type, a reallocation beside an out
slot, a nullable released parameter, a release on any other pointer, a nonnull
out slot, an out slot on any other ABI type, two
out slots, an out slot beside an owned return, a store beside either, a nullable
stored parameter, a store on a `const` pointer, a store beneath itself or
beneath a non-`Owned` parameter, a nullable or `const`-pointer declared owner, a
`resource_owned` parameter that owns nothing, a nullable consumed parameter, a
custody mode on a scalar, a borrowed `ptr<mut, ptr<mut, u8>>` and a consumed
`ptr<const, u8>` are each refused by their own name (`Q024`).

`resource_removed` is refused on a measured ground rather than by omission: a
removal names a child this profile stopped tracking when it was stored, because
`EXTERN-F1` records the residual beneath a declared owner as a premise and not
as a modeled owner-to-child index, so admitting it would mint a caller identity
out of a premise. `resource_released(p, nullable)` is refused for the reason the
nullable consumed mode is: a nullable release discharges nothing when the
modeled value is null (§C04.2), and this profile does not model that value.

`out` marks the position of the declared out slot; it names a parameter
position, never a binding, and is reserved. The `let` binder names the identity
the callee writes into that slot. `move NAME` at a declared store removes the
binding from the body exactly as a consumption does, so every later use is
refused; where the custody went is declared by the contract, not by the call
site, and is recorded on the certificate as a premise. Card `EXTERN-MODES` adds
no argument syntax and no reserved word: `move NAME` also spells a declared
release and a declared reallocation subject, and which of the four departing
modes ran comes from the declaration alone. The certificate keeps them apart —
a release is published under `releases`, a replacement as one `(replaced,
established)` pair — because contract spec §C04.2 warns that a false release
declaration can invalidate a safety conclusion while a false consumption only
relocates an obligation.

A reallocation is the one departing argument an acquiring call may carry.
Contract spec §C04.3 states both of its arms — a null failure preserves the
incoming custody, a success replaces it exactly once — so the subject stays on
the error arm's live list and must be dispositioned there like any other live
identity. Every other departing mode beside an acquisition stays refused, for
the reason that refusal exists: an ordinary declaration constrains only its
normal exit.

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

Card `PIPE-SCALAR-RESULT` adds `scalar_binding` and one meaning to a bare
`NAME` argument, and adds no reserved word. `let NAME = CALLEE(ARGS);` binds a
**non-acquiring call's scalar result** to a local, and a bound scalar may then
be passed at a declared scalar (data) parameter. The admission is exactly:
the callee establishes fresh custody through none of the three acquisition
sources, and its declared result is a custody-free scalar of the §C04.1 ABI
vocabulary. `unit` is refused — there is no value — and every pointer result is
refused, including a pointer declared `resource_return_none()`, because such a
result is an address the declaration says nothing about (`Q024`, naming the
declared result). Binding an acquiring call's result refuses at `Q025`: an
acquisition has exactly one spelling, the mandatory adapter, and this card adds
no second one.

Scalars and identities are **separate namespaces**. A bound scalar is admitted
at a declared data parameter and at no other mode; a resource binding is refused
at a data parameter (`Q024`, "a declared scalar parameter carries data, never
custody"); a scalar binding removes nothing from and adds nothing to the ledger,
so an unread one is not a leak; and a `let` whose name already means the other
kind of thing is refused rather than resolved by lookup order — measured, the
reachable refusal is name resolution's own `Q012` (duplicate local binding), and
the lowering keeps its `Q026` arm as an unreachable fail-closed backstop. The
certificate publishes `(callee, value)` per binding in trace order, beside the
custody fields and never inside them. A **scalar literal** binding
(`let n = 42;`) is deliberately *not* added here and stays refused; it is
follow-on `EXTERN-SCALAR-LITERAL`, and `STD-IO-COMPOSE` is the card that needs
it. The emitted C declares the result at its ABI spelling, discards it to
`void` so the translation unit stays warning-clean, and prints an observation
that names the callee, the value and the arguments —
`value CALLEE vN ARG…`.

Card `EXTERN-SCALAR-OUTCOME` (trace half) appends the number to that line, and
only where a `succeeds INTEGER` outcome is declared: `value CALLEE vN ARG… -> N`.
A reference interpreter still cannot compute a C function's result; what it
prints is the *declared* number, and the emitted program compares the real
result against that same number and exits 70 **before** reaching the
observation write, so a run only ever publishes the line having established
that the declared number is the returned one. A binding with no declared
outcome keeps the card-`PIPE-SCALAR-RESULT` line byte for byte, because nothing
in either model computes a number for it — that remainder is deliberate, not an
omission. The ordering of the comparison and the write is load-bearing rather
than incidental.

The derived effect row is `{io}`, plus `alloc` exactly when the call establishes
fresh custody through any of the three acquisition sources and `mut` exactly
when some parameter is a `mut` pointer; the entry's declared row must admit it, and an
absent row therefore admits no extern call at all. Length
and extent are not transported, so no declaration here constrains how many bytes
a callee touches. Custody liveness, leaks and resource flow are checked by the
established sequential passes over the same lowering, not by a second extern
code. Emission declares each callee from the ABI table and calls it directly;
which object file supplies the symbol is the builder's premise. See
[the extern-call record](reports/extern-call-v1/README.md) and
[the out/stored extension](reports/extern-call-f1/README.md),
[the error-arm release record](reports/remaining-extern-err-rel/README.md) and
[the released/reallocated extension](reports/remaining-extern-modes/README.md).

### G02d.1. Unchecked linkage

Card `UNSAFE-LINKAGE`. An external library with **no contract at all** may be
linked, under an explicit recorded waiver. This widens G02d's *declaration* form
and nothing else: the statement grammar, the argument forms and every refusal
above are unchanged, and without the waiver an uncontracted callee still refuses
at `Q023`.

```ebnf
unsafe_module   = "import", "std.extern", ";",
                  "import", "std.extern.unsafe", ";",
                  { error_decl | extern_decl | unsafe_extern_decl | entry } ;
unsafe_extern_decl = "unsafe", "extern", "fn", NAME,
                     "(", [ param, { ",", param } ], ")",
                     "->", SCALAR_ABI_TYPE, ";" ;
SCALAR_ABI_TYPE = "bool" | "i8" | "i16" | "i32" | "i64" | "isize"
                | "u8" | "u16" | "u32" | "u64" | "usize" | "unit" ;
```

Both imports are required and neither implies the other: `std.extern` is the
call vocabulary and grants no waiver, `std.extern.unsafe` is the waiver and
grants no call. The waiver import is required by the *declaration* (`Q032`),
not by a call to it — declaring an uncontracted symbol is itself the trust act,
so a file that links one and never calls it still names the module. `unsafe`
was already a reserved source word (it introduces the `unsafe { … }` statement),
so this form adds none.

Parameters take `SCALAR_ABI_TYPE` minus `unit`, and the result takes it whole;
**every pointer spelling is refused in every position** (`Q033`), including
`ptr<const, u8>`, and so is any spelling outside the §G05 ABI table. The
contract is waived; the ABI is not. The admitted signature therefore carries no
custody in either direction, which is why an uncontracted call cannot change the
caller's ledger whatever the callee does. A waiver may not sit beside a
checkable declaration (`Q034`): not an inline `contract { … }` block, and not a
name the selected contract environment also declares — there the contracted
route wins whether or not the two signatures agree.

The derived effect row of an uncontracted callee is `{io, alloc, mut}`
unconditionally, because nothing is known about it, so an entry calling one
declares the whole row. A module may mix contracted and uncontracted callees; a
module all of whose callees are uncontracted needs no contract selection at all,
which is what `quirin check` and `quirin emit` can compile today.

Every certificate names each uncontracted callee, with its declared ABI, as an
unchecked-linkage premise, on the same channel that carries a trusted declared
contract; a published certificate missing one is refused (`Q035`). See
`docs/design/UNSAFE_LINKAGE_V1.md` and
[the unchecked-linkage record](reports/wave18-unsafe-linkage/README.md).

### G02d.2. The native-library manifest

Cards `UNSAFE-LINKAGE-ARTIFACT`, `NATIVE-LIBRARY-CONTRACT-SLICE` and
`NATIVE-LIBRARY-HEADER`. A **manifest** is not source: it is a separate
declarative file a builder selects (`quirin --native-library MANIFEST`,
`CompileOptions::native_libraries`), and it adds no form to G02d's or G02d.1's
source grammar and no reserved word. What it changes is the *premise* a call
publishes: a G02d.1 waiver moves from "symbol X, unchecked" to "symbol X,
unchecked, from artifact Y@hash under ABI premises Z", and — card
`NATIVE-LIBRARY-CONTRACT-SLICE` — a G02d **contracted** call gains a second
premise beside its declared-contract one, "contracted X per contract C, from
artifact Y@hash under ABI premises Z". The second premise **stacks with** the
declared-contract premise and never replaces it: one says what the code does,
the other which code it is.

```ebnf
manifest     = marker, { field | entry } ;
marker       = "native_library", "manifest", "v1", ";" ;
field        = ( "library" | "version" | "artifact" ), TOKEN, ";"
             | "hash", "sha256:", 64 * LOWER_HEX, ";"
             | "data_model", "lp64", ";"
             | "unwinding", "abort", ";"
             | "allocator_custody", "closed", ";"
             | "thread_posture", "single_caller", ";"
             | "header", TOKEN, ";"                        (* card `NATIVE-LIBRARY-HEADER` *)
             | "header_hash", "sha256:", 64 * LOWER_HEX, ";" ;
entry        = waived_entry | contract_entry ;
waived_entry = "unchecked", "fn", NAME,
               "(", [ C_ABI_TYPE, { ",", C_ABI_TYPE } ], ")",
               "->", C_ABI_TYPE, ";" ;
contract_entry
             = "contract", "fn", NAME,                     (* card `NATIVE-LIBRARY-CONTRACT-SLICE` *)
               "(", [ C_ABI_TYPE, { ",", C_ABI_TYPE } ], ")",
               "->", C_ABI_TYPE, "from", MODULE_REF, ";" ;
C_ABI_TYPE   = "_Bool" | "int8_t" | "int16_t" | "int32_t" | "int64_t"
             | "intptr_t" | "uint8_t" | "uint16_t" | "uint32_t"
             | "uint64_t" | "uintptr_t" | "void"
             | C_POINTER ;                                 (* a waived entry refuses C_POINTER *)
C_POINTER    = "uint8_t *" | "const uint8_t *" | "uint8_t **" ;
MODULE_REF   = TOKEN ;   (* the contract selection's own key, compared exactly *)
```

`#` begins a comment; statements are `;`-terminated and may span lines. The
eight identity and ABI-premise fields are mandatory and each may be stated once.
The two **header-binding** fields (card `NATIVE-LIBRARY-HEADER`) are optional
**as a pair**: both, or neither. One without the other is a `Q039` missing-field
rejection, on the same ground as every mandatory field — an identity missing a
field is not a weaker identity but an unusable one — and naming neither is the
default, which leaves every manifest written before that card byte-valid and
byte-unchanged. Entry types are
the **C** spellings of the exported header declaration, not the QCore ones, so
the agreement check compares a projection rather than a string. The pointer
spellings `uint8_t *`, `const uint8_t *` and `uint8_t **` are recognised
(the space before `*` optional) and then **refused in a contract-less entry**,
which is a rejection with its own name rather than an unreadable spelling: a
library's pointer symbols belong in a **contract slice entry**, and card
`NATIVE-LIBRARY-CONTRACT-SLICE` is where they now go. The two entry kinds are
told apart by their keyword and never by their shape: a `contract fn` without
its `from MODULE` tail, and an `unchecked fn` carrying one, are each a named
rejection rather than a reading of the other.

A `contract fn` entry binds its symbol to a declared contract **module
reference** — the contract selection's own key, compared exactly and never
normalised. The contract machinery is unchanged: the manifest selects and
attributes, it states no mode and no effect of its own, and the contract is
loaded, admitted and bridged by the unedited resource-contract v1 route.
The binding is total in **both** directions (`Q047`): every entry must resolve to
a declared contract in the module it names, and every declaration of a module the
slice names must be bound by an entry. The slice is derived from the entries
themselves rather than declared by a second statement, because a second spelling
would be a second place the slice could be wrong.

Loading is total or reject (`Q039`), the selection's binding must be a function
(`Q040`), a **waived** declaration the manifest contradicts is refused (`Q041`),
a name the manifest declares contract-less may not also be reached through a
declared contract (`Q042`), a contract slice binding must resolve in both
directions (`Q047`), a **contracted** declaration the manifest contradicts is
**withdrawn for the unit** and its call sites fall to `Q023` (`Q048`), and a
published certificate must carry both stacked premises of every manifest-bound
contracted callee (`Q049`). Selecting no manifest is the default and leaves every
G02d and G02d.1 program byte-identical.

A `header` field binds the library's **exported header** and `header_hash` its
digest (card `NATIVE-LIBRARY-HEADER`, spec §L14.4). Where they are present, every
entry's transcribed declaration above is read out of that header's own text and
compared against it, for both entry kinds alike, rather than believed: the
header's text is supplied to the checker (`CompileOptions::native_library_headers`;
`quirin` resolves the reference against the manifest's own directory), its digest
is measured over exactly those bytes and **verified** against `header_hash`, and
a declaration outside the bounded reader subset refuses the manifest by name
(`Q063`). A transcription the header contradicts refuses (`Q064`) — the `Q041`
disposition, not `Q048`'s, because the weaker admissible reading here is this
same manifest without a header and that is accepted. Declarations the manifest
does not name are ignored: a header is the library's whole exported surface.

See `docs/design/NATIVE_LIBRARY_LINKAGE_V1.md`,
`docs/design/NATIVE_LIBRARY_HEADER_V1.md`,
[the manifest record](reports/wave19-unsafe-linkage-artifact/README.md),
[the contract slice record](reports/wave20-native-library-contract-slice/README.md)
and [the header record](reports/wave22-native-library-header/README.md).

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

## G02g. Rendered lines: `std.fmt`, `std.io` and `std.logger`

`source.std-io.v1` (card `STD-MODULES`) is the whole implemented surface of the
three output modules. A file selects it by using one of its three statements.

```ebnf
module    = { import }, { error_decl | entry } ;
import    = "import", ( "std.fmt" | "std.io" | "std.logger" ), ";" ;
entry     = "fn", NAME, "(", ")", "->", "unit",
            [ "effects", "{", EFFECT, { ",", EFFECT }, "}" ],
            "{", { stmt }, "}" ;
stmt      = "let", NAME, "=", [ "-" ], INTEGER, ";"
          | "print", RENDERING, OPERAND, ";"
          | "log", LEVEL, RENDERING, OPERAND, ";"
          | "flush", ";"
          | "return", ";" ;
RENDERING = "dec" | "hex" ;
LEVEL     = "info" | "warn" | "fatal" ;
OPERAND   = [ "-" ], INTEGER | NAME | "borrow", NAME ;
EFFECT    = "io" | "alloc" | "mut" ;
```

```qcore
import std.fmt;
import std.io;
import std.logger;

fn main() -> unit effects { io } {
    let total = 42;
    print dec total;            // stdout: 42
    print hex 255;              // stdout: 0x000000FF
    log warn dec total;         // stderr: [WARN] 42
    flush;                      // sink handoff; writes no line
    return;
}
```

**The operand rule is the effect discipline.** A *bare* `OPERAND` is an integer
literal or a name the entry bound with `let` to an integer literal. Neither form
can denote an owned buffer, a share or a handle, so a bare rendering cannot
consume, release, store or retain custody of anything — it is not that the
checker refuses those uses, it is that the profile cannot express them. A name
that is not one of the entry's integer bindings is `Q028`; a `let` bound to
anything but an integer literal is `Q030`. The derived effect row is
exactly `{Io}` for a program that writes and empty for one that does not; it is
never `Alloc` and never `Mut`. The entry's declared `effects { … }` row must
admit the derived one (`Q029`).

**The borrowed render** (card `STD-FMT-BORROW`) is the one operand form that
names a resource. `borrow NAME` renders that resource's **model identity** — the
number the emitted C's own call trace spells `rN` — and nothing else: no byte of
the object, no address, no extent, no descriptor, because no model a rendering
can reach carries one. It is admitted only where a checked custody model both
supplied the identity **and** holds it live at that statement, which
`source.std-io.v1` never does: this route has no custody model, so every
`borrow NAME` written on it refuses at `Q028` — structurally, for want of a
model, rather than by a rule the route states. §G02i is where the form is
reachable, and the liveness rule is that route's join.

`borrow` was **already** a reserved source word (the checked mutex-guard
profile's), so the form adds none and moves no pinned surface. The marker is
also what keeps the pipe out: `x |> print dec` desugars to `print dec x;` with
no marker, and a sink stage naming an extra selector word refuses at `Q036`
(§G02h), so no pipeline can spell a borrowed render in this revision.

Each module gates its own capability: `print` needs `std.io`, `log` and `flush`
need `std.logger`, and naming a rendering needs `std.fmt`. `flush` names no
rendering, so a logger-only file that never renders needs no `std.fmt`. A
missing import is `Q015` naming the module.

The severity is spelled `fatal`, not `error`: `error` is the reserved
error-type declaration keyword, and this card renamed its own syntax rather than
edit the pinned declaration surface, exactly as `work` became `effort` in G02e.
Eight words are newly reserved — `print`, `log`, `flush`, `dec`, `hex`, `info`,
`warn`, `fatal`. An **unreserved** word in a rendering or level position is
collected as a referenced name and refuses at `Q013` before this route's own
`Q030` message; the refusal is correct and fail-closed, and only the message is
less specific.

The route emits C11 and has a reference execution, compared **per stream**: the
run's stdout against the reference's stdout projection and its stderr against
the stderr projection. **No order between the two streams is modeled or
claimed.** `flush` emits `fflush` and produces no line — sink handoff, not
durability: no `fsync` is performed and no crash survival is claimed. There is
no format-string language, no string or character value, no file or descriptor,
and no logger configuration model. A `print` inside a `thread scope` or beside
an extern call stays with the more specific route and keeps its refusal. See
[the module record](reports/wave17-std-modules/README.md).

## G02k. An owned byte buffer: `std.buffer`

`source.buffer.v1` (card `STD-BUFFER`) is the whole implemented surface of the
byte-buffer module. A file selects it by using one of its four constructs. It is
the **source-level** tier of the two `std.buffer` tiers
[`STD_BUFFER_V1.md`](design/STD_BUFFER_V1.md) §1 distinguishes; the
native-library-backed tier of `NATIVE_LIBRARY_LINKAGE_V1.md` §6 is untouched by
it and neither retires the other.

```ebnf
module    = { import }, { error_decl | entry } ;
import    = "import",
            ( "std.buffer" | "std.fmt" | "std.io" | "std.logger" ), ";" ;
entry     = "fn", NAME, "(", ")", "->", "unit",
            [ "effects", "{", EFFECT, { ",", EFFECT }, "}" ],
            "{", { stmt }, "}" ;
stmt      = "let", NAME, "=", [ "-" ], INTEGER, ";"
          | "let", NAME, "=", "bytes", "(", INTEGER, ")", ";"
          | "let", NAME, "=", "extent", "(", NAME, ")", ";"
          | "let", NAME, "=", NAME, "[", OPERAND, "]", ";"
          | NAME, "[", OPERAND, "]", "=", OPERAND, ";"
          | "drop", NAME, ";"
          | "print", RENDERING, OPERAND, ";"
          | "log", LEVEL, RENDERING, OPERAND, ";"
          | "flush", ";"
          | "return", ";" ;
OPERAND   = [ "-" ], INTEGER | NAME ;
EFFECT    = "io" | "alloc" | "mut" ;
```

```qcore
import std.buffer;
import std.fmt;
import std.io;

fn main() -> unit effects { io, alloc, mut } {
    let b = bytes(8);           // acquire eight zeroed bytes; the obligation opens
    b[0] = 65;                  // write one byte at a checked index
    let n = extent(b);          // the declared length, as an ordinary integer
    let first = b[0];           // read one byte
    print dec n;                // stdout: 8
    print dec first;            // stdout: 65
    drop b;                     // release; the obligation closes
    return;
}
```

**The extent is transported.** `bytes(N)` lowers to an ownership acquisition
whose footprint is `N` on a byte axis, so the declared length lives in the
custody model's own ledger for the resource's whole lifetime and `extent(b)`
reads it back out of there rather than out of a side table. This is the
capability `ExternAssumption::NoExtentTransport` names as missing, supplied **on
the source side only**: a declared C04 contract still cannot say how many bytes
a callee touches, and this profile does not change that.

**Custody is inherited, not reimplemented.** The lowering hands the event
sequence to the unedited cycle-3 ownership checker, which owns every custody
refusal at `Q054`: a leak at the normal exit, a double release, an access after
release, a rebound live name. This profile adds only the rules about *indices*,
because nothing in the language had ever indexed anything.

**Every index, length and stored byte is a check-time constant** — an integer
literal or a name bound to one, which includes a name bound by `extent(…)` or by
a byte read. A non-constant operand refuses at `Q052`. The rule is load-bearing
rather than conservative: this route's evidence is a byte comparison between a
reference execution and a native run, and a value the reference interpreter
cannot compute is one it cannot predict a line for. `Q053` is the bounds and
range rule — an index outside `[0, extent)`, a stored byte outside `0..=255`, an
extent outside `1..=4096` — decided **before emission**, so an out-of-range
access is a refusal and not a run-time trap. `Q055` refuses a buffer at a
position it may not cross: a rendering operand (bare or `borrow`), an index, or a
stored byte.

The derived effect row carries `Alloc` and `Mut`, where `source.std-io.v1`'s is
exactly `{Io}` because its operand grammar admits no resource. §G02d derives
those two as well, but from a **declared contract**'s projections — `Alloc` when
the contract says the callee acquires — whereas here they follow from an
acquisition and a store this profile checks and discharges itself. An
acquisition and a release derive `Alloc`, a store derives `Mut`, a read derives
nothing, and the declared `effects { … }` row must admit each (`Q057` — this
profile's own code, not a reuse of §G02g's `Q029`, because §L21 forbids a second
semantic pass reusing one and this row can carry effects `Q029` never reports).

**Two words are newly reserved: `bytes` and `extent`.** The release is the
**existing** `drop` — the operation is the ownership IR's own drop, so a new
word would have been a synonym — and the byte accessors are the **existing** `[`
and `]`, consumed elsewhere only in usage-interval positions (§G04) and never in
a statement body, so `b[0] = 65;` needs no parser change at all. The acquisition
is spelled `bytes` and **not** `buffer`: `buffer` was measured at 555
occurrences across the source-side corpus, in *binding* position in shipped
fixtures (`let buffer = qcore_buffer_new(1)?;` in
`examples/threads/shared_readers.qcore`), so reserving it would refuse them and
this card renamed its own syntax instead — the `work` → `effort` (§G02e) and
`error` → `fatal` (§G02g) precedent. The *module* keeps the name `std.buffer`.

The route emits **self-contained** C11 — `calloc`, `free` and two bounds-checked
accessors defined in the emitted file, with no extern declaration, no library
and no boundary — and has a reference execution, compared per stream exactly as
§G02g's is. `calloc` rather than `malloc` so a read before a write has a defined
answer. The emitted bounds comparison and allocation check **abort**, and both
are *witnesses*: every index is a constant the checker already proved in range,
so the comparison is provably never taken, and if one ever fired that would be a
defect in the checker. Every claim on this route is therefore
**partial-correctness**: if the emitted program produces an observation at all,
it is the reference observation; an aborting run produces none.

**No string, and no crossing.** There is no text, no encoding, no literal and no
concatenation — a byte buffer is not a string (follow-ons
`STD-BUFFER-STRING-LITERAL`, `STD-BUFFER-RENDER`). A module that buffers *and*
extern-calls stays with §G02d and keeps that route's refusal; one that buffers
*and* spawns stays with the concurrent route and keeps its `Q600`; one that
buffers and prints reaches this route, whose line statements are §G02g's own.
See [the module record](reports/wave22-std-buffer/README.md).

## G02i. Composed extern calls and rendered lines

`source.std-io-extern.v1` (card `STD-IO-COMPOSE`). Selected when a module both
calls a declared `extern fn` (§G02d) and writes one of the standard-IO
statements (§G02g) — the conjunction of those two routes' own selectors, and
nothing else.

```ebnf
composed_module = "import", "std.extern", ";",
                  { "import", STD_IO_MODULE, ";" },
                  { error_decl | extern_decl | entry } ;
composed_stmt   = extern_stmt | io_stmt ;   (* §G02d and §G02g, verbatim *)
```

**This route adds no statement.** Every statement of a composed entry is lowered
by one of the two existing lowerings, unedited, chosen by the statement head
alone: `print`, `log` and `flush` are the standard-IO model's, `return;` is the
single return *both* models require, and everything else is the extern model's.
Each lowering runs with the other's statements skipped, so neither ever sees a
statement it does not model and neither refusal surface moves. A module using
one feature set is not composed and keeps that route byte for byte.

Route selection places this composition **after every concurrent route and
before both routes it composes**. A module that prints and spawns, or
extern-calls and spawns, therefore keeps its concurrent route's existing
refusal: this card composes two *sequential* models and no other pair.

What the composition decides, and all it decides, is **which value a rendering
reads**. The extern model owns the value namespace, so the operand names of
`print`/`log` resolve against its bindings — a bound scalar, never a resource
identity, which has no operand form at all (§G02g's effect discipline is
structural and is unchanged here). The operand must be bound by an earlier
composed step, must be of the one ABI width the two renderings are total on, and
must be a value whose number the source **determined**: a `scalar_literal`, or a
`scalar_binding` carrying its declared `succeeds INTEGER` outcome. A bound call
result with no declared outcome is refused at `Q043`, because rendering it would
publish a line no model computed.

Card `STD-FMT-BORROW` adds one rule to that join and no code: a `borrow NAME`
operand resolves against the extern model's **custody** namespace — never its
value namespace — and is admitted only where that model's own published ledger
holds the identity live at the rendering's composed step. An identity never
established (including one whose acquisition took its declared null arm), or one
that departed by consumption, release, storage or reallocation before the
rendering, is refused at `Q043` rather than rendered. The rendering opens no
obligation and discharges none: the composed ledger is still exactly the extern
projection's, and the identity is as live after the line as before it. A
rendering is not a *publication* in §L21's constraint-10 sense — the destination
is a byte stream rather than an object of the custody graph, the bytes carry a
copy of the identity number rather than the address, and no model operation
reads a stream back — so no second reachable path to the resource exists. The
emitted program reads nothing out of the object: the identity is a number the
checker determined, and what the borrow buys is the checked precondition, not a
run-time read.

The composed program's derived effect row is the **union** of the two models'
rows and must be admitted by the one row the entry declares. Its observation is
per stream: standard output carries this route's `print` lines *and* the extern
route's synthesized call trace, in one real order on that one stream, and no
order between stdout and stderr is modeled — the boundary §G02g already stated.

## G02j. Composed scoped tasks and rendered lines

`source.std-io-tasks.v1` (card `COMPOSE-CONCURRENCY`) is selected for a module
that uses **both** the scoped-task forms (§G02c) and the line-producing
statements (§G02g), and that uses no extern call — the conjunction
`has_source_tasks ∧ has_source_std_io ∧ ¬has_source_extern`, and nothing else.
It is selected **after** the composed-epoch, epoch and mutex routes and
**before** the task route it composes, so every other concurrency exclusion is
unchanged: a module with an epoch scope or a mutex generation keeps that model's
refusal, a module that extern-calls and spawns keeps the task route's `Q600`,
and a module using one of the two composed feature sets falls through to that
route byte for byte.

The grammar adds **no statement and no reserved word**. The three line-producing
statements are §G02g's, read by that route's own parser; the scoped-task forms
are §G02c's, lowered by that route's own lowering. One thing is widened: a task
function's `effects { … }` row admits `io`, and *only* on this route — off it,
`io` is still the unsupported task effect that refuses at `Q020`. The entry must
declare it whenever the composed program writes at all, and since card
`OBS-REORDERING` a **worker** that writes must declare it in its own row too.

```ebnf
task_stmt   = ... | io_stmt ;          (* §G02c plus §G02g's three, verbatim *)
task_effect = "alloc" | "spawn" | "join" | "may_block"
            | "throws", "<", NAME, ">"
            | "io" ;                   (* this route only *)
```

**Placement.** A line stands in the **parent's** body, or — since card
`OBS-REORDERING` — in the body of a function this entry spawns as a worker.

In the parent it may stand anywhere: before any `spawn`, between two `join`s,
inside a `match` arm on the creation or allocation outcome, or after every join.
The reason is a measurement of the emitted runtime rather than a convention:
every *trace* write an emitted task program performs is performed by the
parent's thread, because a worker's trace events are appended to that worker's
private array and replayed by the parent at the join. So a parent line's
position relative to every trace line is determined, and such a program's
streams are compared **byte for byte**.

In a **worker** it is performed by that worker's own thread, concurrently with
the parent's writes and with its siblings'. Card `COMPOSE-CONCURRENCY` refused
that and named its own repair — the comparison-up-to-an-admitted-reordering
capability — and card `OBS-REORDERING` landed it
(`qcore.obs-reordering.v1`, `docs/design/OBS_REORDERING_V1.md`). A worker line
is admitted, and placing one is what **declares** the relation: that program's
streams are compared window by window and emitter by emitter, where a window
boundary is a point of the run at which no worker is live and an emitter is the
thread that wrote the bytes. Two extra conditions come with it, both `Q050`:

* the worker's **own** `effects { … }` row must admit `io` — the effect
  discipline is per function, and a worker body is a function body;
* a line in a function that is neither the entry nor a spawned worker is refused
  outright: nothing in this profile makes such a function run, so its line has
  no thread to attribute it to.

A program that places **no** worker line declares nothing, and its comparison is
unchanged: a relation whose windows each have one emitter admits exactly one
stream, its own. All four published `examples/threads` programs are of that
kind.

**Operands.** A rendering operand is an integer literal or a name the *task*
model bound to an `i32`:

```ebnf
OPERAND     = [ "-" ], INTEGER | NAME ;   (* no `borrow NAME` on this route *)
```

Unlike §G02g's own `let`-bound literals and §G02i's declared outcomes, the number
is **computed by the task model** at the rendering's own step — the value a
`qcore_buffer_read` read out of a live buffer, or the outcome a `join`
collected — so the reference execution reads it out of the slot the task model
wrote. Whether the name may be read at that position is decided by
`check_task_module`, over an ordinary copy of the scalar inserted at exactly that
position; the composition states no liveness rule of its own. A name denoting an
owned Buffer, a read loan or a task handle is refused, and `borrow NAME` is
refused on this route entirely (§G02i is where a borrowed render is admitted,
over a model that publishes a ledger the join can read).

The composed observation is per stream, exactly as elsewhere: standard output
carries this route's `print` lines *and* the synthesized task trace, on that one
stream, and no order between stdout and stderr is modeled. That stream's order
is one real order when every line on it is the parent's — which it is unless a
worker `print`s — and otherwise is the declared reordering relation's.
Removing this route's own lines from standard output leaves exactly the task
route's rendering of the same module, which is checked over the four published
`examples/threads` programs and proved in general as
`QCore.ComposedTasks.erasing_the_source_lines_leaves_the_task_trace`.

## G02h. The pipe operator, and the operator token family

`source.pipe.v1` (card `PIPE-OPERATOR`) is a **parse-time desugaring, not a
profile**: a file does not select it, and no route knows it exists. A
well-formed pipeline is rewritten into the tokens of the statement a reader
would have written without it, before the delimiter map is built and before any
parsing, so name resolution, custody, effects, linearity, concurrency,
whole-program checking, reference execution and C emission all see an ordinary
statement. A piped program and its hand-written twin are `syntax_equivalent`
and emit byte-identical C11.

```ebnf
stmt       = [ "let", NAME, "=" ], OPERAND, ( "|>", STAGE )+, ";" ;
OPERAND    = token, { token } ;            (* verbatim: `x`, `-7`, `move b` *)
STAGE      = [ "move" ], STAGE_BODY ;
STAGE_BODY = "print", RENDERING            (* a G02g sink *)
           | "log", LEVEL, RENDERING       (* a G02g sink *)
           | NAME, [ "(", [ arg, { ",", arg } ], ")" ], [ ADAPTER ] ;  (* a G02d call *)
ADAPTER    = "succeeds", BOOL, "else", { disposition }, "raise", NAME ;
```

| Written | Desugars to |
|---|---|
| `total \|> print dec;` | `print dec total;` |
| `total \|> log fatal dec;` | `log fatal dec total;` |
| `x \|> f;` and `x \|> f();` | `f(x);` |
| `b \|> f(7);` | `f(b, 7);` |
| `move b \|> release;` | `release(move b);` |
| `8 \|> alloc succeeds true else raise E \|> move free;` | `let ⟨t⟩ = alloc(8) succeeds true else raise E;` then `free(move ⟨t⟩);` |
| `let r = x \|> f succeeds true else raise E;` | `let r = f(x) succeeds true else raise E;` |

**A chain is a statement sequence** (card `PIPE-CHAIN`). A pipeline of `n`
stages desugars to `n` statements: each non-final stage binds its result to a
hygienic temporary the next stage receives, and the final stage keeps the
statement form the reader wrote around the whole pipeline. Custody flows through
the temporaries under the unedited checkers — by the time any of them runs a
temporary is an ordinary binding — so a chain that drops a live result refuses
at the pre-existing `Q202`, and a value consumed mid-chain cannot be used after
the pipeline.

**"Produces a value" is decided by stage kind**, syntactically and with no
contract knowledge in the desugaring: a call stage produces a value, and a sink
(`print`, `log`) or an operandless statement (`flush`) does not. Card
`PIPE-CHAIN` decided it by the presence of `succeeds`, because the acquisition
adapter was then the only `let`-with-call the language admitted; card
`PIPE-SCALAR-RESULT` adds the other one — a non-acquiring call's scalar result
(§G02d) — so the keyword no longer separates the two. Whether a particular
callee's declared result is bindable is the **route's** question, answered from
the declaration and reported at that stage's own span: a `unit`-returning callee
in a non-final position still refuses, at `Q024` naming its result rather than
at `Q037` naming the pipe. This is what makes `x |> render |> scale`
expressible, which card `PIPE-CHAIN` measured as the first of the two boundaries
under the brief's `value |> render |> log`; the second, a module that both calls
an extern function and prints, is unchanged and remains `STD-IO-COMPOSE`.

**A model intrinsic is a call stage** (card `PIPE-INTRINSIC-STAGE`). Card
`COMPOSE-CONCURRENCY` filed the brief's literal `buffer |> read |> render |> log`
as needing "a pipeline stage for a task intrinsic, which is a pipe-grammar
change". It is not, and the census is the reason: the source language's model
intrinsics fall into two shapes, and this grammar already decides both.

| Shape | Intrinsics | Where | As a stage |
|---|---|---|---|
| **call-shaped**, `NAME(ARGS)` | `qcore_buffer_new`, `qcore_buffer_read`, `qcore_buffer_write`, `qcore_error_payload` | §G02b task routes and every route that composes them (`ComposedStdIoTasksV1`, `ComposedEpochTasksV1`) | **admitted** by the `NAME [ "(" … ")" ] [ ADAPTER ]` alternative above, with no table entry of its own |
| **call-shaped**, `NAME(ARGS)` | `qcore_acquire`, `qcore_check` | the sequential route | **admitted**, likewise |
| **keyword-shaped** | `spawn S W(ARGS)?`, `join move H?`, `share N`, `read N`, `write N`, `drop N`, `grow`, `open`, `close`, `bound`, `overhead` | their own routes | **refused**: their operands are not a parenthesized argument list, so no call stage produces them, and the closed sink table deliberately does not name them |

So `buffer |> qcore_buffer_read |> log info dec;` is the statement sequence
`let ⟨t0⟩ = qcore_buffer_read(buffer); log info dec ⟨t0⟩;` under the ordinary
chain rule, emitting byte-identical C against that hand-written pair. An
intrinsic whose result the surrounding statement discards, or whose result the
next stage's route cannot bind, refuses at **that route's** own code and span —
`qcore_buffer_write` returns `unit` and refuses mid-chain at `Q050`, not at
`Q037` — which is the rule §G02d already states for a `unit`-returning callee.
A chain whose intermediate stage *allocates* binds an owned resource to a
temporary no reader can name and therefore cannot drop; that leaks and refuses
at the route's pre-existing leak diagnostic (follow-on `PIPE-DROP-STAGE`).

The same card makes "no later pass can tell a pipe was written" true of **every**
route. The sequential route recovered each statement's tokens by re-lexing its
source bytes rather than reading the program's own post-desugaring stream, which
for a pipeline recovers the pipeline: the operand sits left of the callee in the
bytes and right of it in the statement, so that route's own `qcore_acquire`
reader saw the operand where it wanted the callee name. Reading the parsed
stream is identical on every pipe-free file — it is the lex of that same source —
and it is what admits the two sequential intrinsics above.

**Two spellings name the value a previous stage produced**, because a hygienic
temporary has no name a reader could write: `|> move STAGE` consumes it,
mirroring `move OPERAND |> STAGE` for the first stage (where `move` on the stage
is refused, one spelling per meaning); and a bare `move` inside a stage's adapter
tail names it, so an acquisition's error arm can dispose of the value piped into
it. Without the second, `Q025` would cap a chain at one acquisition.

**Hygiene is enforced by refusal.** Temporaries are named `qcore__pipeN`; a file
that needs them and also writes that prefix is refused at `Q036` rather than
silently α-renamed, so no diagnostic can name a binding the reader did not
write. A file whose pipelines are all single-stage generates none and is
untouched.

**Insertion is at the stage's first operand position** — Elixir-style
first-argument insertion, chosen because QCore is subject-first and has no
currying, so there is no partial application for a last-argument rule to
compose with. For a parenthesized call that position is immediately after `(`,
before any argument already written. For a statement stage the head and its
closed selector words are not arguments, so the operand follows them; both v1
statement stages take exactly one operand, so "first operand position" and
"append" coincide and no selector-arity table is needed.

**Attribution.** Every token the reader wrote keeps its own span, unmoved, so a
refusal inside a desugared statement points at the bytes that were typed: an
operand fault at the operand, a stage fault at *that stage*. Only `let`, `=`,
`;`, `(`, `)`, `,` and the temporaries are synthesized, each anchored zero-width
at the `|>` it came from — at that pipe's start for the first stage and at its
end for a later one, which is what keeps consecutive statements' byte spans
strictly disjoint where a `|>` abuts its neighbours, since a statement's
synthesized terminator sits at the next pipe's start and `|>` is two bytes wide.
The one exception is recorded rather than hidden: in a **multi-stage** bound
pipeline the `let NAME =` the reader wrote sits to the left of every stage, so
keeping its spans would make the last statement's span swallow the ones before
it; the binder is re-anchored at the final `|>` with the reader's own name
preserved. A one-stage bound pipeline keeps its binder tokens where they were.

**Refusals.** `Q036` a pipeline shape this profile does not model — an empty
side, a stage that is not a name, a sink stage of the wrong selector arity, an
unbalanced call stage, a malformed or value-less `let NAME =` binder, an
assignment pipeline (follow-on `PIPE-ASSIGN`), `move` on the first stage, a bare
`move` whose incoming value is not a binding, or a file that needs pipeline
temporaries and writes the `qcore__pipe` prefix itself; `Q037` a stage that
cannot receive the operand — `flush`, which takes none, or a **sink** in a
non-final position, since only a call stage binds a result;
`Q038` an operator token used where this revision gives it no meaning — one of
the five operator spellings `|`, `&`, `^`, `||`, `&&` **outside an `if`
condition** (see the table below). Card `BITWISE-CONDITIONS` gave the three
single-byte spellings a meaning in that one position, so the gate is now
positional for all five and no longer holds any of them as a pure reservation.
Dropping the `;` of the last statement in a body is a parse failure (`Q000`)
before the pipe pass runs.

### The operator token family

Two-byte spellings are tried before every one-byte spelling, so tokenization is
longest-match at a fixed width of two. The vertical-bar-family spellings are
pairwise distinct byte pairs (or a single byte), so their order in the table is
immaterial and the resolution is decided by width alone.

| Spelling | Token | Status |
|---|---|---|
| `[\|` / `\|]` | `QuantOpen` / `QuantClose` | **unchanged** — the G04 quant-index delimiter alias |
| `\|>` | `PipeInto` | **landed** — desugared away before parsing (§G02h above) |
| `\|\|` | `PipePipe` | **logical or** — one token; admitted in an `if` condition and lowered there (§G02i) |
| `&&` | `AndAnd` | **logical and** — same, binding **tighter** than `\|\|` |
| `\|` | `Pipe` | **bitwise or** — admitted only as a parenthesized comparison operand in an `if` condition (§G02i); `Q038` elsewhere |
| `&` | `Ampersand` | **bitwise and** — same position, binding **tighter** than `^` |
| `^` | `Caret` | **bitwise xor** — same position, between `&` and `\|` |

**This table supersedes card `PIPE-OPERATOR`'s, by explicit user direction**
(card `OR-AND-LOWERING`). That card had decided the single `|` was logical or
and `&&` logical and, with `&` held for binary use and `||` refused as a
misspelling of `|`. The retable inverts the or half: **both** logical operators
are double-character and short-circuit, and **all three** single characters are
freed for a future bitwise wave. That also answers the open note wave 18 left —
bitwise or had no spelling left — by giving it `|` back.

`[|` is matched at the `[`, so `[|>` is always `[|` then `>` and never `[` then
`|>`, and `[||]` is the two quant delimiters and never an or-operator. `|>` wins
over `||` and over `|`, so the ambiguity wave 18 recorded (`a |> b` against a
hypothetical `a | > b`) was inherited by **`BITWISE-CONDITIONS`**, which
measured it and recorded the answer: `|| >` is unambiguous, and a bitwise `|`
adjacent to a `>` must be spaced — but the adjacency can never produce a wrong
*acceptance*, only a refusal, because `>` has no left operand inside an integer
expression. `(1 | 2) > 0` is the spaced spelling and it parses; `1 |> 2` is a
pipeline in a condition and refuses at `Q045`.
`|||` is `||` then the reserved `|`; `&&&` is `&&` then the reserved `&`.

**Two deliberate pin updates, neither changing any program's verdict.** A bare
`|` was a lex error before card `PIPE-OPERATOR` and a `Q038` refusal after it; it
stays a `Q038` refusal outside a condition, with the message naming the position
it *is* admitted in rather than claiming the token. A bare `^` was a lex error
(`unexpected character '^'`) and became a token whose occurrences refuse at
`Q038` — the same move one card later, for the same reason: an actionable
message. A bare `&` refused downstream in whichever route claimed the module and
now refuses at the gate. In all three cases the retable itself left the set of
accepted programs unchanged and moved only the layer and the message; card
`BITWISE-CONDITIONS` then *widened* the set, in one position and by one
grammar production each (§G02i). The census behind the retable
is in [the card record](reports/wave19-or-and-lowering/README.md): `[|`, `|]`,
`||`, `&` outside comments and `^` appear in **no** `.qcore` file at all, and
`|>` only in the four pipe fixtures.

## G02i. The source branching construct

```ebnf
stmt     = "if", "(", cond, ")", block, [ "else", ( block | stmt ) ] ;
cond     = conj, { "||", conj } ;                 (* 1, left-associative  *)
conj     = neg,  { "&&", neg  } ;                 (* 2, left-associative  *)
neg      = "!", neg | cmp ;                       (* 3, right-associative *)
cmp      = int, [ op, int ] ;                     (* 4, non-associative   *)
int      = xor, { "|", xor } ;                    (* 5, left-associative  *)
xor      = and, { "^", and } ;                    (* 6, left-associative  *)
and      = atom, { "&", atom } ;                  (* 7, left-associative  *)
atom     = "(", cond, ")" | "true" | "false"
         | [ "-" ], INTEGER | NAME ;
op       = "==" | "!=" | "<" | "<=" | ">" | ">=" ;
```

The grammar is **two-sorted**: levels 1–4 produce a *condition* and levels 5–7
an *integer*, `atom` produces either, and one parser reads both — so a sort
mismatch is a refusal rather than a second parse. There is no coercion in either
direction: `if (mask & 4)` and `if (true | false)` are both refused (`Q045`).

`source.branch.v1` (card `IF-STATEMENT`), on the **sequential route only**. The
statement shape is the one the parser has always accepted; what the card added
is that the condition's bytes are now *recorded* rather than discarded, so a
semantic pass can read them. `else if` flattens into one statement with one
condition per header, and lowers to a right-nested branch. Nesting an `if`
inside an arm is admitted — it is the shape a short-circuit lowering needs.

**No new reserved word and no new token.** `if`, `else`, `true` and `false` were
already reserved source words, and `==`, `!=`, `<=`, `>=`, `<` and `>` were
already single tokens. The census across all 369 `.qcore`/`.qc` files (317 of
them source-side, where wave 18 counted 294) found `if`
as a word in four files — three of them `//` comment text quoting C inside
`contract/qcore/fluent_bit/`, and one an archived probe the standing gate does
not replay — and `else` in sixteen, every one of them the extern adapter's
`succeeds BOOL else … raise ERROR` tail, which is a `;`-terminated statement and
never follows a `}`.

**The condition is decidable at check time**, exactly as the `succeeds` and
`passes` literal outcome controls beside it are, because the translation
certificate compares one reference observation, one CFG observation and one
observation parsed from the compiled program's own output. Every leaf is closed
over literals, so evaluation stays state-free however the leaves are combined.

**The connectives (card `OR-AND-LOWERING`).** Card `IF-STATEMENT` shipped the
tier with one leaf and no connectives, naming the successor card that would add
them. `||` and `&&` are now admitted **in a condition and nowhere else**, and
they are a pure desugaring onto this construct:

```text
if (A || B) T else E   ==>   if (A) { T } else if (B) { T } else { E }
if (A && B) T else E   ==>   if (A) { if (B) { T } else { E } } else { E }
```

The arms are lowered **once** and their operation vectors duplicated into nested
branches, so a connective adds no statement, no acquisition and no checker rule;
`qcore_core` is byte-unchanged by the card, and the emitted C is byte-identical
to the hand-lowered twin's. Short-circuit is structural rather than asserted —
the right operand is the condition of a branch nested inside an arm of the left
operand's — though it is not *observable*, because every leaf is a pure literal
comparison; `IF-RUNTIME-CONDITION` is what would make it so. Every connective
level runs the same four-model custody join, so a connective cannot launder a
`Q046` disagreement: `if (A || B) { drop a; }` is the conditional leak exactly as
`if (A) { drop a; }` is.

**The negation and the integer tier (card `BITWISE-CONDITIONS`).** Unary `!` is
eliminated by an **arm swap**: `if (!C) { A } else { B }` is the branch a reader
writes as `if (C) { B } else { A }`, so `!!A` is literally `A`'s own lowering,
the emitted C is byte-identical to the hand-swapped twin's, and no checked model
sees a negation. The three bitwise operators `|`, `^` and `&` are admitted in
**one** position — a parenthesized comparison operand — over signed 32-bit
literals and names bound to one. A `NAME` atom is a **check-time constant
binding**, `let NAME = INTEGER;`, admitted at the top level of a sequential
entry body and substituted for its value here; a name that denotes a resource
identity is refused, so no custody-bearing value gains a position in this tier.

**Precedence is declared, and the rule for declaring it is stated once: a
precedence is declared only where it agrees with C, and where this tier's two
sorts would force a different ranking than C's, the unparenthesized spelling is
refused rather than silently redefined.** Levels 1, 2, 5, 6 and 7 are C's own
order and are declared: `&&` tighter than `||`, `&` tighter than `^` tighter
than `|`, all left-associative, parentheses group. Levels 3 and 4 are not — C
ranks `!` tighter and `|`/`^`/`&` looser than the comparisons, while a tier in
which a comparison yields a condition and a condition is never an integer
operand must rank them the other way — so `!A < B` and `A | B == C` are
**refused** (`Q045`) and `!(A < B)` and `(A | B) == C` are the spellings. A
comparison is non-associative, so `1 < 2 < 3` is refused too.

Two measured consequences are recorded rather than designed. For a *homogeneous*
chain the lowering is associative on the nose — `(A || B) || C` and
`A || (B || C)` emit identical C, and so do the two groupings of `1 | 2 | 4` —
so each associativity declaration is load-bearing only where operators of
different levels mix. And a negation over two *identical* arms is unobservable,
for the same reason a swap of equals is: `!` is load-bearing exactly when the
arms differ.

`|>` is **not** in the tier and is never given a precedence relative to anything
in it: a condition mixing them is refused at `Q045`.

**The branch is nonetheless real.** Both arms are validated in full, both are
emitted as basic blocks, and the emitted C computes the comparison itself —
`if (INT32_C(3) < INT32_C(5)) { goto …; } else { goto …; }` — rather than
receiving the decided literal, so the reference evaluator and a C compiler are
cross-checked by the translation-validation triangle and by a sanitized native
run. Only the *observation* is single-valued; follow-on `IF-RUNTIME-CONDITION`.

**The custody join is agreement between the arms that survive it, never
widening.** Both arms are validated from the same incoming state, and an arm
that **returns** discharged its obligations at its own exit and does not reach
the join at all (card `IF-ARMS`); the join compares the arms that fall through,
and where there are two of them it admits only equal ordered live lists and equal
transferred sets. **Four** models check that agreement — the cleanup lowering,
ownership (`OwnershipOp::Branch`), typed errors (`ErrorOp::Branch`) and resource
flow (`Flow::join_branches`) — and a fifth, usage (`UsageExpr::Branch`), joins
the arms' *demands*. Only `SourceOp::Branch`, in the cleanup lowering, is new:
the other four were all already shipped, each with its own mismatch error.

### The arm statement subset

| Arm statement | Admitted | Ground |
|---|---|---|
| `qcore_check(BOOL, Error);` | yes | custody-neutral |
| `drop NAME;` / `move NAME;` | yes | the join's own subject |
| a nested `if` | yes | the shape `\|\|`/`&&` lower onto |
| an empty arm; `if` with no `else` | yes | the missing arm is the empty arm |
| `return …;` | **yes** (card `IF-ARMS`) | an exit from an arm carries exactly the obligations the sequential route imposes at any exit, so the returning arm discharges at its own exit and leaves the join |
| a zero-argument call `f();` | **yes** (card `IF-ARMS`) | a source call emits no C at all in this subset; it composes the callee's summary flow into the caller's, and a *conditional* call is admitted only when that composition is the identity |
| `let NAME = …;` | no — `Q044` | an arm-local binding has no position after the join |
| `let n = qcore_acquire(…);` | no — `Q103` | two arms leave different resource-identity ledgers; `IF-ARM-ACQUIRE` |
| `while`, `match`, `defer`, `raise`, … | no | each keeps its own code; `while` inside an arm is still `Q500` |

**The continuation moves with the exit.** Four of the five models cannot express
"this arm exited", so the driver does not hand them a one-sided join: the
statements written *after* a branch one arm of which returns are lowered inside
the arm that survives to reach them, which is where they run. The rewrite is a
**move**, never a duplication — the continuation is lowered exactly once — so the
shape that would need it in two arms (an early return in one arm of a nested
branch whose sibling also falls through) is refused rather than served by
duplicating acquisitions, calls and demands.

**Refusals.** `Q044` a shape this profile does not model — inconsistent block and
condition counts; a `let` inside an arm; a branch with one returning arm and no continuation for the
survivor to reach, or one whose continuation still reaches no `return`
(`IF-ARM-RETURN-NESTED`); statements after a branch every arm of which returns;
or a conditional call whose callee's summary flow is not neutral
(`IF-ARM-CALL-FLOW`). `Q045` a
condition outside the tier — a chained comparison, an arithmetic expression, a
shift, a complement, a pipeline, an empty condition, an unbalanced parenthesis,
a connective missing an operand, an integer where a condition is required or a
condition where an integer is required, an **ungrouped** bitwise comparison
operand or `!` operand, a name bound to a resource identity, or a non-integer
operand.
`Q046` two surviving arms do not reach the join in the same custody state.
`Q202` an arm exit with an owned resource still live — the same condition, and
the same code, as the program's own exit, reported at the exit that leaks. A name
written in a condition that is bound to nothing is collected as a referenced
name and refuses earlier at `Q013`. An operator token written **outside** a
condition refuses at `Q038` before the tier is consulted — all five of `|`,
`&`, `^`, `||` and `&&`. An acquisition
inside an arm — or in a continuation hoisted into one — is refused by the
pre-existing ownership join at `Q103`, inherited unedited and only re-attributed
(`IF-ARM-ACQUIRE`). Every other route keeps its own refusal, measured: `Q030` on
`source.std-io.v1`, the extern route's nested-block refusal on
`source.extern-call.v1`, and `Q500` on the copy-scalar task profile — and neither
a connective in the condition nor a widened arm statement changes which refusal
answers. See
[the branch record](reports/wave19-if-statement/README.md),
[the connective record](reports/wave19-or-and-lowering/README.md),
[the integer-tier record](reports/wave20-bitwise-conditions/README.md),
[the arm record](reports/wave20-if-arms/README.md) and
[the design note](design/SOURCE_BRANCH_V1.md).

## G02k. The checked event surface

Card `STD-EVENT`. Design record: [`design/STD_EVENT_V1.md`](design/STD_EVENT_V1.md).
Example page: [`../examples/events/README.md`](../examples/events/README.md).

```ebnf
module    = { import }, entry ;
import    = "import", "std.event", ";" ;
entry     = "fn", NAME, "(", ")", "->", "i32",
            [ "effects", "{", effect, { ",", effect }, "}" ], "{", { stmt }, "}" ;
stmt      = "event", "open", ";"
          | "event", "close", ";"
          | "endpoints", NAME, NAME, ";"
          | "watch", NAME, NAME, interest, ";"
          | "unwatch", NAME, ";"
          | "wake", NAME, ";"
          | "dispatch", "ready", NAME, ";"
          | "dispatch", "idle", ";"
          | "close", NAME, ";"
          | "return", [ "-" ], INTEGER, ";" ;
interest  = "read" | "write" ;
effect    = "io" | "alloc" | "mut" ;
```

There is deliberately **no `let`** and no scalar operand: this grammar has no
value vocabulary at all, so every name it resolves denotes an endpoint or a
registration — a custody name. That is why the derived effect row is exactly
`{Io}` for any program that opens a context and why `Alloc` and `Mut` are
unreachable here by construction rather than by a rule.

Eight reserved words are added — `event`, `endpoints`, `watch`, `unwatch`,
`wake`, `dispatch`, `ready`, `idle` — and four existing ones are reused rather
than respelled: `open`, `close`, `read`, `write`. The context pair is spelled
`event open;` / `event close;` for exactly that reason. The standing constraint
14 census over every `.qcore`/`.qc` fixture and every inline source string found
no collision: the corpus occurrences of `event` are ghost-field names inside
`contract { … }` clauses, never a statement head and never a binding.

**The custody half is the existing model.** Each statement projects onto
`qcore_core::event_reactor::EventOperation`s and the projection is handed to
that model's own checker unmodified; a refusal at `Q060` carries its `L00`–`L12`
diagnostic verbatim. This card is what made that model reachable from source at
all.

| Statement | Projection |
|---|---|
| `event open;` / `event close;` | `CreateBase` / `FreeBase` |
| `endpoints A B;` | `CreateSocketPair` (success branch) |
| `watch E D INTEREST;` | `Assign` naming the descriptor, then `Add` — the custody edge |
| `unwatch E;` | `Delete` then `FreeEvent`, retiring the registration's cookie |
| `wake D;` | *(none — the reactor models no byte flow)* |
| `dispatch ready E;` | `BeginDispatch`, `Activate`, `CallbackEnter`, `CallbackExit`, `EndDispatch` |
| `dispatch idle;` | `BeginDispatch`, `EndDispatch` |
| `close D;` | `CloseSocket` (success branch) |

**The determinism boundary is this route's own.** A certificate compares one
observation, so v1 waits only on readiness the program itself induced with
`wake D;`, and each turn declares what it reports — a declaration the emitted
program verifies against what `poll(2)`/`WSAPoll` really reported, exiting
non-zero on disagreement. A turn at which two or more registrations are ready is
refused **by name** at `Q061`: `poll(2)` reports a set, and the callback order
within one ready set is the implementation's rather than one the program
induced. Every registration carries priority `0` so that a priority order is not
smuggled in as an undeclared ordering premise. A `write` interest is accepted
and checked and makes the module emit no C at all, on the `accept_check_only`
precedent.

Diagnostics: `Q058` shape, `Q059` an unbound custody name, `Q060` the reactor
model's own refusal, `Q061` the determinism boundary, `Q062` the effect row, and
`Q015` for a missing `import std.event;`.

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
                | "requires", literal_argument_premise, ";"
                | "ensures", "normal", resource_predicate, ";" ;
literal_argument_premise =
    "resource_literal_argument", "(", IDENT, ",", integer_literal, ")" ;
integer_literal = ["-"], DIGITS ;
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

Additional static conditions are mandatory: exactly one marker, exactly one mode per parameter, exactly one result mode, unique names, compatible pointer/scalar types, valid distinct owned parent references, an owned reallocation source, and — for `literal_argument_premise` — a parameter name the same declaration declares (of any admitted type, pointer included: see contract specification C04.5), at most one premise per parameter, and a `DIGITS` run the lexer produces as a single integer token with no leading zero, which excludes suffixed, alternate-base and octal spellings; `-0` is likewise rejected, so the declared literal is canonical. These conditions cannot be represented by context-free productions alone. Signature usage annotations, extra declarations, unsupported predicates and other exit selectors are rejected atomically, not ignored. See contract specification C04–C06 and `crates/qcore-contract/src/contract_loader.rs`.

`literal_argument_premise` (card HASH-CLAUSE) states where the declaration is applied, not what it means: a consumer treats the callee as undeclared at every call site whose actual at the named parameter is not exactly that integer constant. It never widens a declaration and never changes a mode. Which call-site *spellings* are that constant is fixed by contract specification C04.5.1 (card HASH-REFINE): matched enclosing parentheses and an integer suffix are erased structurally, and every spelling needing arithmetic, a base, a cast or symbol resolution is refused.

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
