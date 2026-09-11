# `qcore.obs-reordering.v1` — comparing an observation stream up to a declared reordering

Card `OBS-REORDERING` (wave 22). Design record, written before the
implementation and kept as the normative statement of what the capability is.

This document defines **one relation on observation streams**, states what
derives it, states what its equivalence means, and states — at the same length —
what it does **not** mean.

---

## 0. The problem, in one paragraph

Every route in this repository that writes compares a native run's bytes against
a reference execution's rendering **byte for byte, per stream**. That comparison
is well-defined exactly while each stream is written by one thread. Card
`COMPOSE-CONCURRENCY` measured that this holds for `source.std-io-tasks.v1` —
every write an emitted task program performs is performed by the parent's
thread, because a worker's trace events go to that worker's private array and
are replayed by the parent at the join — and therefore refused a source line
inside a **worker** body at `Q050`, with the ground stated: such a line would be
written by that worker's own thread, concurrently with the parent's writes and
with its siblings', and *no checked model in this repository orders them*, so the
per-stream byte comparison would compare against something that is not a function
of the program.

That refusal names its own repair. This document is that repair: a comparison
that is a function of the program even when the byte order is not, because the
set of admitted byte orders is **declared by the checked model** rather than
discovered by running the program.

## 1. What is being related, and by whom

### 1.1 An observation stream

An **observation stream** is what one C stream (`stdout` or `stderr`) of one run
carries: a finite sequence of complete lines. In this model a line is a
**record**:

```
record = (emitter, window, text)
```

* `text` — the bytes of the line, without its terminating newline.
* `emitter` — **the thread of control that wrote those bytes.**
* `window` — the index of the concurrency window the record was written in
  (§2.2), assigned by the derivation, never by the observer.

`emitter` is the load-bearing field and its definition is the opposite of F02's.
`qcore.task-trace-relation.v1` attributes an observed line to *the task whose
state change it reports* (its premise P2), because F02 relates **causal** order.
This relation attributes a record to *the thread that performed the write*,
because it relates **byte** order, and bytes are ordered by writers. The two
attributions disagree on exactly one family of records — a worker's buffered
trace events, replayed by the parent inside `qcore_task_collect` — and each is
right for its own question: F02 reads them as the child's actions, and this
relation reads them as the parent's writes, because the parent's thread is what
put them on the stream.

```
Emitter ::= Parent | Worker(task-id)
```

### 1.2 A relation is a declaration, not an observation

An `AdmittedReordering` is **the reference stream, annotated**. It is
constructed only from a checked model's own reference execution, and the
annotation (emitter, window) is read off the model's spawn/join structure. It is
never derived from, adjusted to, or validated against a native run. A native run
is only ever *compared to* it.

That is the whole reason this is a capability and not a test relaxation: the set
of admitted orders is fixed before any native run exists, and a native run either
lies inside it or is refused.

## 2. The relation

### 2.1 Commuting

Two records **commute** exactly when

1. they lie in the same window, **and**
2. their emitters differ.

Equivalently and more usefully as a rule: **two records never commute when they
share an emitter** (a thread performs its writes in its own program order, and
nothing in this model reorders one thread against itself), and **two records
never commute across a window boundary** (§2.2 makes a window boundary a real
happens-before edge).

`R`-equivalence of two streams is the reflexive–transitive closure of
transposing adjacent commuting records. §3 gives the closed form the checker
actually decides, and proves the two agree.

### 2.2 Windows, and where their boundaries come from

A window boundary is a point of the run at which **no worker is live** — no task
has been created and not yet collected. At such a point the parent is the only
thread of control in the program, so a record written there is ordered after
everything written before it and before everything written after it, by
`thrd_create`'s and `thrd_join`'s own synchronization.

The derivation therefore walks the reference run's records in order, maintaining
the live set, and:

* a record written while the live set is **empty** is a **barrier**: it is given
  a window of its own, so it commutes with nothing;
* a maximal run of records written while the live set is **non-empty** shares one
  window, and within it records of distinct emitters commute.

A worker's own lines all lie strictly between its creation and its collection, so
they always land inside a non-empty window; the parent's pre-spawn and post-join
lines always land on barriers. The conservative reading of `COMPOSE-CONCURRENCY`
— "before any spawn, between joins, after all joins" — is exactly the set of
barrier positions, so the strict comparison that card relies on is *the barrier
part of this relation*, unchanged.

Windows are numbered over the **merged** record sequence across both streams and
then projected, so a barrier that happens to be written on `stdout` still
separates two `stderr` windows. Numbering per stream would have merged two
concurrency regions separated by a quiescent point into one window and admitted
reorderings the program does not perform.

### 2.3 What derives it — the ingredients, and where each is read

| Ingredient | Read from | Not read from |
|---|---|---|
| the record sequence and its texts | the composed reference execution (`ComposedTaskObservation`) | any native run |
| a trace record's emitter | the emitted runtime's own structure: every trace line is written by the parent's thread, immediately or at collect | the task id inside the line |
| a source line's emitter | the task that was executing the line's function in the reference run | the line's text |
| the live set | `TaskEvent::Spawn` (opens) and the recorded index at which each `qcore_task_collect` replay begins (closes) | timing |

The collect boundary is **recorded by the reference runtime** at the moment its
model performs the join, rather than recovered afterwards by scanning the
rendered events for a pattern. A pattern scan was rejected: a worker that emits
no buffer event renders as `SPAWN k`, `RESULT k`, `JOIN k`, and the `SPAWN`
record carries `k` in exactly the field a backwards scan would read, so the scan
would attribute the parent's own spawn record to the collect.

## 3. What equivalence means

Let `s` be a declared stream and `t` an observed one. Write

* `sizes(s)` for the sequence of window lengths, in window order;
* `s|w,e` for the subsequence of window `w` written by emitter `e`, as texts.

**Definition (`R`-equivalence).** `s ≡ t` iff `sizes(s) = sizes(t)` and, for
every window `w` and every emitter `e`, `s|w,e = t|w,e`.

Because the window sizes agree, `t` decomposes into windows in exactly one way,
which is what makes this decidable by a single left-to-right pass.

**The canonical form.** `canon(s)` is, per window, the list of
`(emitter, texts)` pairs with the emitters in a fixed total order. Then
`s ≡ t ⟺ sizes(s) = sizes(t) ∧ canon(s) = canon(t)` — the "sort the commuting
window, byte-compare within an emitter" shape, stated as an equation.

### 3.1 Soundness — what an accepted native run has been shown to satisfy

1. **Per-emitter order is preserved.** `s ≡ t` implies that for every emitter
   `e`, the whole-stream projection of `s` onto `e` equals that of `t`. So an
   accepted run performed each thread's writes in that thread's program order,
   and the strict statement about a single-threaded emitter survives intact.
2. **Barrier structure is preserved.** `s ≡ t` implies the barrier records — the
   records alone in their window — are the same records in the same order, at the
   same window indices. So an accepted run performed every quiescent-point write
   in exactly the declared place.
3. **Per-window content is preserved.** `s ≡ t` implies each window carries the
   same multiset of lines; nothing is dropped, duplicated or invented.
4. **`≡` is an equivalence relation** — reflexive, symmetric and transitive — so
   "these two runs agree" is a property of the pair and not of which one was
   called the reference.
5. **The strict comparison is the degenerate case.** If every window of `s` has
   at most one emitter — in particular if the program spawns nothing, or writes
   from the parent only, which is every route in this repository before this card
   — then `s ≡ t ⟺ s = t` byte for byte. The existing comparison is not
   *replaced* by this one; it *is* this one, on the relation that admits nothing.

### 3.2 Fail-closed — every way out is a refusal

* **`AmbiguousAlphabet`** — refused **when the relation is declared**, before any
  run: two distinct emitters of one window would write the same text. Then a
  native line's writer is not a function of the bytes, per-emitter order is not
  checkable from the stream, and the relation would silently weaken to a multiset
  comparison. It refuses instead. This is a real restriction on which programs
  can be compared this way, and it is stated rather than engineered around.
* **`WindowsNotMonotone`** — a declared record sequence whose windows do not
  increase along the stream is not a stream this derivation can produce; a
  hand-built one is refused.
* **`LengthMismatch`**, **`Residue`**, **`EmitterOrder`** — an observed stream of
  the wrong length, carrying a line no emitter of its window could write, or
  carrying one emitter's lines out of that emitter's order. Each refuses with
  what it violated. Nothing is normalised, reordered or repaired, on the F02
  precedent.

## 4. What is not claimed

Stated in the premise vocabulary of standing constraint 28: these are premises
and boundaries, not proofs and not TODOs.

1. **This is not a memory model.** No claim is made about visibility, atomicity
   of any shared object, data races, or ordering of anything other than complete
   stream records. TSan is not run and no TSan-shaped claim is made anywhere.
2. **No cross-emitter order is claimed, anywhere.** Inside a window the relation
   asserts the *absence* of order between distinct emitters. It does not assert
   that any particular interleaving happens, is reachable, or was taken.
3. **No cross-stream order is claimed.** `stdout` and `stderr` remain two
   independent observations, exactly as on every route that writes. This relation
   uses the merged sequence only to *number windows*, which is a refinement of
   each stream's own relation and asserts nothing about how the two are
   interleaved.
4. **Premise `C11StreamLock`.** A record is a whole line because a single
   `fprintf` call is performed under the stream's own lock (C11 7.21.2p7–p8): two
   threads writing through `QCORE_IO_WRITE` cannot tear one line into another.
   This is a premise about the C implementation, of the same standing as the C
   layout and linker equalities. A stream without that lock is outside this
   relation, and no attempt is made to detect one.
5. **Premise `C11ThreadJoinSynchronizes`.** A window boundary is a happens-before
   edge because `thrd_join` synchronizes with the joined thread's completion
   (C11 7.26.5.6). Everything §2.2 says about quiescent points rests on that and
   on the profile's leaf-worker rule, not on timing.
6. **An accepted run is one run.** Accepting a native run says that run's stream
   is in the admitted set. It says nothing about the *other* admitted orders being
   reachable, nothing about a run not performed, and nothing about the scheduler.
   Running the comparison many times raises the number of orders observed; it
   does not turn the relation into a coverage claim.
7. **The relation is not a proof that the program is correct under reordering.**
   It states which byte orders the checked model admits. Whether admitting them is
   the right thing for a given program is that program's own question.

## 5. Where the relation is declared today

Nothing declares it by default, and the default everywhere stays the strict byte
comparison. On `source.std-io-tasks.v1` a program **declares** the relation
exactly by placing a source line in a spawned worker's body: the certificate then
carries `admits_reordering`, and a consumer that wants to compare that program's
streams derives the relation from its composed reference observation. A composed
program with no worker line declares nothing, its certificate says so, and its
streams keep the strict comparison byte for byte — which is §3.1's degenerate
case rather than a second code path.

No reserved word is added. Constraint 14's census does not move: the declaration
is a structural property of the program, not a new spelling.

## 6. Named consumers

| Consumer | What it needs from this document |
|---|---|
| `COMPOSE-WORKER-LINES` | §2 and §3, exactly as written. This card's acceptance. |
| `STD-LOGGER-ASYNC` (ordering half) | §2.1's commuting rule with a *queue* as an emitter, and a window whose boundary is the queue's declared drain point rather than a join. The relation carries over; the **derivation** does not, because no queue exists on any route today. |
| event nondeterminism (`COMPOSE-EPOCH-LINES`, a future `source.event.*`) | §2.2's live-set walk generalised to whatever that model's concurrency regions are. The comparator is model-independent already: it consumes records, emitters and windows. |
