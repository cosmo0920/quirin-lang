# W48-QR-SCALAR-TYPES-R6: frozen prediction

Frozen after the BEFORE fixture freeze and before any source, test or Lean
change. Base: local `main` at `8a378bf3` (= origin/main); the worktree's HEAD
was already that commit, so neither `git reset --hard main` nor
`git merge --ff-only main` was needed.

## BEFORE (measured)

`quirin` debug binary built at the base (sha256 `11891c958c01b411…`). The first
half's `freeze_fixtures.py` over the 632 tracked `.qr`/`.qv`/`.qrlib` files
([BEFORE](artifacts/fixtures-before.json)): check exit 0 on 360, exit 1 on 272;
emit exit 0 on 154, exit 1 on 478.

Probes at the base (`private/w48-r6/probes/`, not tracked):

* a worker parameter `own<Buffer<u8>>` refuses at `Q020` ("task parameters
  require i32, own<Buffer> use one, …");
* `let seed: u8 = 200; let buffer = qcore_buffer_new(seed)?;` refuses at
  `Q500` ("buffer constructor requires i32");
* `let buf = protected u16;` on `source.mutex.guard.v1` refuses at `Q013`
  (name resolution collects `u16`).

## Mechanism (planned)

1. **`W48-QR-TYPES-PAYLOADS`.** The payload type becomes part of the Buffer's
   type: `own<Buffer<T>>`, `borrow<Buffer<T>>` and `lend<Buffer<T>>` for every
   §G02t type `T`, with `own<Buffer>` (etc.) the `i32` payload exactly as
   before. The task IR's `OwnedBuffer`/`ReadLoan`/`WriteLease` carry the
   payload type; an allocation's payload type is its initializer's type (the
   type "named at acquisition"); a read yields the payload type and a write
   takes exactly it. A lease's byte extent, and an owner byte read's, is the
   payload type's size (`bool` 1, `isize`/`usize` the recorded width / 8),
   never 4; a byte write into a `bool` payload is refused (a byte other than
   0/1 is not a `bool`). The emitted C types the payload per type, asserts
   `sizeof` of each payload type the module names, and logs a typed payload
   event under a `=TYPE` name through an adapter arm installed only where
   needed, so every `i32`-payload program keeps its bytes and the F02 traces do
   not move; the F02 relation refuses a typed payload event (fail-closed).
   `protected TYPE` declares a mutex-protected payload whose extent is the
   type's size; `protected N` is unchanged. A typed error payload
   (`Failed(own<Buffer<T>>)`) stays refused by name. Lean: the typed extent,
   the typed lease admission, `WRITE-LEND-TASKS` exclusivity re-proved over
   typed extents (every reachable typed state is a reachable state of the
   untyped ledger, and every held byte lies inside the payload), and the `i32`
   extent is 4.
2. **`W48-QR-TYPES-EPOCH-TASKS`**, 3. **`W48-QR-F02-FIXED-OUTCOMES`**,
   4. **`W48-QR-TYPES-COMPOSED-STRING`**, 5. the two emitted-C defects, in the
   brief's order as time allows; a position not finished keeps its refusal
   and follow-up.

## Predictions

| Measure | Prediction | Confidence |
|---|---|---|
| fixtures AFTER vs BEFORE (632) | byte-identical except new examples | 75% |
| existing tests changed | only IR constructions of the Buffer types (a new payload field) | 60% |
| F02-pinned traces and digests | unmoved | 90% |
| native tables under UBSan (`-O0`/`-O2` + UBSan, plain `-O2`) | all agree, 0 reports | 75% |
| Lean | one new audit file, the exclusivity theorem re-proved over typed extents | 85% |
| corpus arbiter | byte-identical to the base's own archived AFTER | 97% |
| standing gate | green, new tests only | 60% |
| items completed | 1 (70%), 2 (30%), 3 (25%), 4 (15%), 5 (20%) | |
