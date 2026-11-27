# CAST-BEFORE-STAR-READ-AS-BINARY: prediction

Committed before the implementation commit. Base `7450d69af` (main, r61).
USER DECISION 2026-11-23: "launch the cast-before-star card".

## What was known when this was written

No implementation was measured. Known from reading the base:

* `legacy_refused_operands::handed_on_among` (W44's refused hand-on) reads a
  `*` before a lone name as a dereference only when `ends_operand` says the
  token before the `*` does not end an operand. It counts `)` as an operand
  end, so `(char *) *p` hands on `p`, not `*p`. The reader only **adds**
  unresolved-effect sites at a refused node: it withdraws leak accusations,
  never makes one and never makes a row `Safe`.
* `aggregate_store::store_bases` premise 4 refuses a base written `* BASE`,
  but its `dereferenced` test excludes a `*` after `)`. So `(T) *b` reads as
  no dereference and leaves `b` admissible. Admitting a base lets a store
  through it be modelled, which is the direction toward `Safe`. Reading the
  `*` as a dereference refuses the base: the fail-closed direction.
* A rough `grep` of the Fluent Bit 046accf6 `src/` and `plugins/` finds 33
  lines with a parenthesised type-like group before `* name` (not counting
  `sizeof`). Almost all are `(unsigned char) *p` character reads. The 11
  `* ) * name` spellings are `sizeof (T *) * n`, which the shared detector
  reads as an argument list (binary), as now.

## Planned change

Both readers route the `) *` decision through the shared detector
`ampersand_runs::may_close_a_cast` (rule 74f).

* **Refused hand-on.** After a `)` that may close a cast, the `*` before a
  lone name hands on both `*p` and `p`. A unary `*` hands on `*p` and a binary
  one hands on `p`, and neither set contains the other. Handing on both is a
  superset of either reading, so it holds for an undecided `(a) * p` too.
* **Premise 4.** A `*` after a `)` that may close a cast is a dereference, so
  the base is refused. The detector is called with no declared objects: a
  candidate base is a record pointer, and C11 6.5.5p2 forbids a pointer
  operand of a binary `*`.

## Predicted movements (opt-in route, base vs tip)

* **Refused hand-on:** a `*p` hand-on matters only where `*p` is a bound place
  that holds a live tracked identity at a refused statement. The corpus
  spellings are character reads, so no move is expected. Any move would be an
  `Unsafe` → `Unknown` leak withdrawal, or a leak leaf gone.
* **Premise 4:** a `Safe` → `Unknown` move would need a record-pointer local
  dereferenced whole under a cast. No move is expected.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**Pinned routes** (`quirin-audit measure --declared`) cannot run on this host:
it has no `build/cycle19` inputs or baselines. The integrator should expect
the same as on the opt-in route. Any move is unpredicted by name and is
reported.
