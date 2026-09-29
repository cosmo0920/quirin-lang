# W48-QR-SCALAR-TYPES-R4: frozen prediction

Frozen after the BEFORE fixture freeze and before any source, test or Lean
change. Base: local `main` at `0691a1cc` (= origin/main); the worktree's HEAD
was already that commit, so neither `git reset --hard main` nor
`git merge --ff-only main` was needed.

## BEFORE (measured)

`quirin` debug binary built at the base (sha256 `fe2acd81f3e3cc86…`). The first
half's `freeze_fixtures.py` over the 629 tracked `.qr`/`.qv`/`.qrlib` files
([BEFORE](artifacts/fixtures-before.json)): check exit 0 on 357, exit 1 on 272;
emit exit 0 on 152, exit 1 on 477. 145 tracked files declare
`concurrency checked;`.

Probes at the base (`private/`, not tracked):

* a worker `fn worker(x: u8) -> u8` refuses at `Q020` ("task parameters require
  i32, own<Buffer> use one, …");
* `let y = x + 1;` in a worker refuses at `Q500` ("task scalar expressions
  require an i32 literal or immutable scalar binding");
* a task module declaring its own `error ArithmeticError { Overflow; }` and
  raising it from a worker is **admitted** (`scoped copy tasks`).

## Mechanism (planned)

1. **`W48-QR-TYPES-TASK-CAPTURES`.** The task IR gains the §G02t table as
   value types (`i32` keeps its representation, every other type one new
   variant), typed literals, and the scalar route's value statements — one
   §G02t operation per `let` (arithmetic, negation, comparison, `as`), decided
   by the extern route's value-statement decision moved into a shared module
   rather than forked: over values known at check time it folds and a fault
   refuses; over a run-time value an operation that can fault needs the
   function's `throws<ArithmeticError>` and lowers to a checked terminator
   whose error edge raises the classified variant through the task route's
   existing error edge (exit instructions, then `raise`). `ArithmeticError`
   becomes a builtin, payload-free error type of the task route beside
   `SpawnError`/`AllocError` (so a user declaration of that name refuses, the
   one planned admission → refusal; no tracked file declares it). A worker's
   `ArithmeticError` is an ordinary typed worker error: collected by `join`,
   reaching the creator through `?` or `match`. Worker parameters, results and
   `copy` captures admit every §G02t type; the transferability judgment gains
   one duplicated kind for them (Rust and `QCore.Transferability`). The C
   runtime's task value gains a typed union only in a module that names a
   non-`i32` type, so every existing program emits the bytes it emitted.
   `isize`/`usize` need the recorded target, published as the named
   assumption. The §G02j operands render at their own type.
2. Then `W48-QR-TYPES-LOOP-BOUNDS` and `W48-QR-TYPES-PAYLOADS` in order as time
   allows; a position not finished keeps its refusal and its follow-up.
   `W48-QR-BUFFER-ROUTE-ARITHMETIC` only if the shared value statements make it
   fall out.

## Predictions

| Measure | Prediction | Confidence |
|---|---|---|
| fixture freeze AFTER vs BEFORE (629 files) | every existing file byte-identical in both commands; only new files differ | 80% |
| existing tests changed by item 1 | none except tests that pin a now-widened refusal (typed worker parameter or task arithmetic) | 60% |
| transferability totality tests | their pinned constructor counts change (16 → 17) and nothing else | 75% |
| native tables, UBSan `-O0`/`-O2` + plain `-O2` on the real thread backend | every stream and exit equals the reference, worker `ArithmeticError` reaches the creator through join, spawn-failure control takes its edge, 0 sanitizer reports | 75% |
| Lean | new theorems audit with no placeholder or project axiom | 80% |
| corpus arbiter `resource-diagnostics.json` | byte-identical `ae17152b…` | 97% |
| standing gate | green; test count rises by new tests only | 70% |
| items completed | 1 (70%); 2 (20%); 3 (5%); `W48-QR-BUFFER-ROUTE-ARITHMETIC` (10%) | as stated |
