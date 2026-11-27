# The F02 trace alphabet's lease boundary — proved and exercised (TRACE-ALPHABET-LEASE-BOUNDARY)

Wave 32, card `TRACE-ALPHABET-LEASE-BOUNDARY`. This card discharges
`TRACE-ALPHABET-LEASE-REFUSAL-UNEXERCISED`, the residual that recorded an
**honesty debt the integrator incurred** at the wave-31 close and refused to let
age into an assumed truth. It is the first card run under `CLAUDE.md`'s new
`STABILITY-REEVAL` discipline, whose first method step — *hunt vacuous
greenness* — names this residual as its occasioning case.

Base: `main` at `6caea680` ("Open wave 32's increment record"). The card touches
one production file, adds one Lean module, one audit file and one test file, and
allocates no diagnostic code, reserves no word and changes no route.

**Headline.** The residual named three alternative discharges — (a) prove the
boundary in Lean, (b) exercise the arm, (c) show it unreachable. The
measurement **decided against (c)** and the card took **(a) and (b) together**,
plus a fourth finding the residual did not anticipate: dropping the lease events
instead of refusing them would have been a move in the **admitting** direction,
not merely a lossy one. The residual is **fully discharged**; a narrowed
successor, `TRACE-ALPHABET-LEASE-GROWTH-UNDECIDED`, replaces its forward-looking
half.

---

## 1. The debt, restated from the evidence rather than from the claim

`qcore_core::task_trace_relation::reference_trace` crosses a boundary: from the
reference run's **observation** alphabet (`qcore_core::task_execution::TaskEvent`
— what the reference interpreter records) into the F02 relation's **observable**
alphabet (`ObservedEvent` — what the relation compares). Wave 31 merged two
cards that disagreed about where that boundary sits:

* `WRITE-LEND-TASKS` added three observation constructors — `BufferWritten`,
  `BufferLeased`, `BufferLeaseReturned` — and refused all three at the crossing,
  through `ReferenceTraceError::UnsupportedObservedEvent`.
* `CT03-PROOF-ALPHABET` admitted `BufferWritten` into the observable alphabet,
  proved that extension conservative, and **deleted** the
  `UnsupportedObservedEvent` variant, which its change left with no producer.

The integrator resolved the conflict as a union — the write encoded (CT03's
proved arm), the two lease events still refused through a **restored**
`UnsupportedObservedEvent` — and reported it verified because both cards' suites
plus `f02_trace_relation` ran green. The residual recorded, correctly, that this
evidence is **vacuous for the arm in question**: `reference_trace` had no caller
outside its own module, and no test anywhere in the workspace constructed a
reference run carrying a lease event. The green suites were evidence about the
paths they took.

What bounded the risk was direction only — the arm refuses rather than admits.
What was *not* established: that the arm is reachable at all; that it is the only
entry point by which a lease event could reach an `AdmittedCausalOrder`; and
whether refusing is even the right answer.

---

## 2. What was measured, before anything was chosen

Each of the following is a measurement, reported as one. None is inferred from
the diff.

### 2.1 Can an emitted program today produce a lease event in a run that reaches `reference_trace`? — **Yes, and the arm is reachable**

Card `WRITE-LEND-TASKS`' own admitted flagship (`DISJOINT_WRITE_LEASES`: one
allocation, two children with disjoint write leases over the payload's two-byte
halves, both live at once) compiles through the real pipeline today, and its
reference run is fourteen events, of which **index 2** is
`BufferLeased { borrower: 1, buffer: 1, low: 0, high: 2 }`. Driving that
observation into `reference_trace` refuses with
`UnsupportedObservedEvent { index: 2 }`; `AdmittedCausalOrder::derive` carries
the same refusal out unchanged and never reaches its own relation checks.

So the residual's discharge **(c) is measured false.** The arm is not a
defensive guard against an unconstructible shape; it is a live path that an
admitted program of the current language reaches.

What that path is *not* doing today is blocking any claim: `WRITE-LEND-TASKS`'
native evidence is a sanitizer run plus an allocation count, not an F02
relational comparison (`the_two_lease_program_runs_sanitized_with_one_allocation_counted_once`).
Write-lend programs therefore sit **outside F02 relational comparison by
construction** — a measured boundary, recorded in §5.

### 2.2 Is `reference_trace`/`derive` the only route into an `AdmittedCausalOrder`? — **No, there are three routes; but it is the only one a lease can take**

The `compile_fail` doctest on `AdmittedCausalOrder` shows the struct's field is
private, so an unchecked sequence cannot be forged. It does **not** show that
`derive` is the only constructor, and it is not. Verified rather than trusted:

| Route | Public? | Can it carry a lease? |
|---|---|---|
| `AdmittedCausalOrder::derive` → `reference_trace` | yes | only by this arm — which refuses |
| `AdmittedCausalOrder::declare(Vec<ObservedEvent>)` | yes | **no**: `ObservedEvent` has eleven constructors and none is a lease. Unexpressible at the type level |
| `parse_observed_trace(&str)` → `Vec<ObservedEvent>` | yes | **no**: the parser's tag table has no `QCORE_BUFFER_LEASE` or `QCORE_BUFFER_LEASE_RETURN`, so the rendered lease lines refuse at their own line |

The renderer *does* print those lines (`render_task_observation` emits
`QCORE_BUFFER_LEASE 1 1 0 2` and `QCORE_BUFFER_LEASE_RETURN 1 1`), so the parser
is genuinely offered a lease and genuinely refuses it — this is a real refusal,
not an absence of input. Both facts are now pinned
(`the_rendered_lease_lines_are_outside_the_parsers_alphabet_too`).

### 2.3 Is refusing the right answer? — **Yes, and the alternative is worse than "lossy"**

The plausible alternative to refusing is dropping: filter the lease events out
and encode the rest, which a `filter` in place of the `return Err` would do. It
looks like a harmless simplification — report less, block nothing. It is not.

Measured: the flagship's lease-free remainder is **ten events**, it encodes, and
`AdmittedCausalOrder::declare` **accepts it** as an admitted causal order. That
is an admitted order for a run in which two workers held outstanding write
authority over a payload the order never mentions. Dropping is therefore a move
in the **admitting** direction — exactly what invariant 1 forbids — while
refusing only blocks a comparison.

This is the finding the residual did not anticipate, and it is what turns "the
arm refuses, so the direction bounds the risk" from a consolation into a
positive reason. It is now pinned in Rust
(`dropping_the_lease_events_would_have_produced_an_admitted_order`) and proved in
Lean (`the_dropped_encoding_would_have_been_admitted`), so a later reader cannot
re-derive the dropping variant as the tidy-up it resembles.

### 2.4 Should the alphabet grow instead, by CT03's conservative method? — **Not decidable today**

CT03's method has two halves: a conservativity theorem (the extension moves no
existing verdict or digest) *and* a demonstration that the rule the new
constructor extends is **non-vacuous in both directions** — CT03 supplied
`a_rewritten_payload_is_admitted` beside `a_stale_read_is_refused`.

A lease extension can supply the first half. It cannot supply the second today:
in this slice a worker **holds** write authority but does not **exercise** it
(the standing residual `WRITE-LEND-TASKS-EXERCISE`), so there is no worker-side
range write for a lease-window rule to admit or refuse. Admitting the two events
now would grow the alphabet by two constructors carrying no constrainable fact —
a larger alphabet and not one rule with more force. The question is therefore
genuinely open rather than answered in the refusing direction by default, and it
is what the narrowed successor residual carries (§5).

---

## 3. The discharge taken: (a) and (b), together

### 3.1 (b) The arm, exercised — `compiler/tests/trace_alphabet_lease_boundary.rs`

Five tests, all driving a **real compiled program** rather than a hand-built
observation, except where the program route cannot reach the shape:

| Test | What it pins |
|---|---|
| `a_real_lease_program_reaches_the_refusal_at_the_offending_events_own_index` | the flagship's run carries a lease at index 2 (located by shape, so the index is checked *against the observation*, not against itself); `reference_trace` refuses there; `derive` carries the refusal out unchanged |
| `the_lease_boundary_refuses_rather_than_dropping_the_event` | the lease run does not encode at all; the lease-free control encodes with its event count preserved and `derive`s cleanly — so neither direction elides an event |
| `dropping_the_lease_events_would_have_produced_an_admitted_order` | §2.3: the ten-event remainder is **admitted**, which is why dropping is not an option |
| `a_lease_return_refuses_on_its_own_at_its_own_index` | the arm's second variant, which the program route can never reach (a `BufferLeaseReturned` is always preceded by the `BufferLeased` that refuses first), refuses at **its own** index 4 |
| `the_rendered_lease_lines_are_outside_the_parsers_alphabet_too` | §2.2's text route, both lease line kinds |

The control (`DISJOINT_READ_SHARES`) is the same program with its two write
leases replaced by read shares, so the refusal is attributable to the lease
rather than to the spawn/join shape.

**Non-vacuity, measured by mutation.** A green test over a fail-closed arm is
exactly the shape this card exists to distrust, so the tests were checked the
way the residual says evidence should be checked — by breaking the arm three
ways and confirming the tests go red:

| Mutation of the arm | Tests red |
|---|---|
| encode leases as `Loaned`/`Returned` (an *admitting* mis-encode) | 3 of 5 |
| pre-filter lease events out and encode the rest (the silent drop) | 3 of 5 |
| report `index: 0` instead of the event's own index | 2 of 5 |

The mutated file was restored from a byte copy taken before the first mutation
and re-verified green.

### 3.2 (a) The boundary, proved — `proof/QCore/TaskTraceLeaseBoundary.lean`

CT03's `QCore.TaskTraceWrites` is the transcription of the F02 relation over the
**observable** alphabet, and its conservativity theorems are about that alphabet.
This module is about the **observation** alphabet on the other side of the same
crossing, and about the crossing function itself. It transcribes `TaskEvent` (in
the shape `WRITE-LEND-TASKS` left it), `ReferenceTraceError` (both variants —
modelling only one would overstate the encoder's totality, since they share an
index space), and `reference_trace`'s `enumerate().map(...).collect()`.

| Theorem | Statement |
|---|---|
| `succeeds_iff_no_event_is_refusable` | the crossing is decided by the events alone, **exactly**: a lease-carrying run is always refused (no lease slips through on any shape of run), and a refusable-free one always encodes (the refusal blocks nothing else) |
| `a_lease_carrying_observation_is_refused` | the forward half on its own — the statement the unexercised arm was asserting |
| `no_lease_reaches_an_encoded_trace` | the negative half — an encoded trace's source carries no lease, so none can reach `AdmittedCausalOrder` by this route |
| `encoding_is_positionwise`, `encoding_preserves_length` | on success the output is, position for position, the per-event encoding of the input: the formal content of "refused rather than silently dropped" — nothing elided, reordered, or invented |
| `encodeEventAt_error_index`, `refusal_points_at_a_refused_event`, `refusal_is_the_first` | the index a refusal carries is the offending event's **own** position, and the first such. The Rust pin `index: 2` is an instance, not a coincidence |
| `the_flagship_run_is_refused_at_its_lease` | non-vacuity, refusing direction: the card's real fourteen-event flagship, transcribed, refuses at index 2 (`rfl`) |
| `the_lease_free_twin_is_an_admitted_causal_order` | non-vacuity, admitting direction: the read-share twin crosses, and `TaskTraceWrites.declare` of the result is `0` |
| `the_dropped_encoding_would_have_been_admitted` | §2.3, proved: the lease-dropped remainder is ten events and **`declare` admits it** |
| `the_dropped_run_loses_exactly_the_four_lease_events` | and the two runs differ in exactly the four lease events, so the theorem above is about *this* refusal |

Registered in `proof/AuditTaskTraceLeaseBoundary.lean` (new), which
`scripts/check_lean_audits.py` runs. Every printed name depends only on
`propext`, `Quot.sound` and `Classical.choice`; several depend on no axiom at
all. No project axiom, no placeholder.

### 3.3 The production change

`crates/qcore-core/src/task_trace_relation.rs` is the only production file
touched, and **only its documentation changes** — the arm's behaviour is
byte-identical. The UNPROVEN caveat is replaced by what is now true: the pointers
to the proof and the exercise, the explicit "do not turn this refusal into a
drop" with its measured reason, and the narrowed open question. The
`ReferenceTraceError` doc gains the index guarantee; `reference_trace`'s doc
names the lease beside the T06 payload and states positionwise encoding.

---

## 4. What is proved versus what is observed, afterwards

The `STABILITY-REEVAL` discipline asks for this separation in these words.

**Proved** (Lean, over arbitrary runs of the transcribed model): that the
crossing succeeds exactly when no event is refusable; that a lease is never
encoded, on any shape of run; that a successful encoding is positionwise and
length-preserving; that the refusal's index is the first offending event's own;
that the dropped variant would have been admitted. These are statements about the
**model**, and the model is a transcription — Lean parity, not extraction.

**Observed** (Rust, over the real pipeline): that the transcription's flagship is
the pipeline's actual reference run for card `WRITE-LEND-TASKS`' actual admitted
program; that the arm is reachable from that program; that the three mutations
are caught. The correspondence between the Lean model and the Rust
implementation is the ordinary transcription premise this project carries
everywhere — it is not proved here and is not claimed to be.

**Neither proved nor observed, and named as such**: whether the observable
alphabet should grow to carry leases (§2.4, §5); anything about native
executions of write-lend programs beyond the sanitizer evidence
`WRITE-LEND-TASKS` already published.

---

## 5. Residual: retirement, and the narrowed replacement

### 5.1 `TRACE-ALPHABET-LEASE-REFUSAL-UNEXERCISED` — **fully discharged, retire it**

For the integrator, in terms usable against `named_followups`: every one of the
residual's three "what is NOT established" clauses is now established.

* *"that the arm is reachable at all"* — established, and in the direction the
  residual did not assume: it **is** reachable, from card `WRITE-LEND-TASKS`'
  own admitted flagship (§2.1). Discharge option (c) is measured false.
* *"that it is the only entry point by which a lease event could reach an
  `AdmittedCausalOrder`"* — established (§2.2): three public routes exist, and
  the other two cannot express a lease at all, one by the parser's tag table and
  one at the type level.
* *"that refusing is the right answer"* — established for today (§2.3) and, more
  strongly than the residual asked, with the alternative measured to be an
  admitting move rather than a lossy one.

The residual's stated discharge condition — "a card that either (a) proves the
lease boundary in Lean beside CT03's write conservativity, or (b) exercises the
arm with a test that drives a lease-carrying reference run into
`reference_trace` and pins the refusal, or (c) states it as a measured boundary
and shows the arm unreachable" — is met by **(a) and (b) both**. The closing
clause, "an integrator-made semantic edit with no card, no Lean parity and no
report is itself the finding", is answered by this card, this report and
`QCore.TaskTraceLeaseBoundary`.

### 5.2 `TRACE-ALPHABET-LEASE-GROWTH-UNDECIDED` — the narrowed replacement

Replacement text, for `named_followups`:

> **id:** `TRACE-ALPHABET-LEASE-GROWTH-UNDECIDED`
> **title:** Whether the F02 observable alphabet should grow to carry write leases is undecided, and becomes load-bearing when workers exercise them
> **why:** Card `TRACE-ALPHABET-LEASE-BOUNDARY` (wave 32) discharged
> `TRACE-ALPHABET-LEASE-REFUSAL-UNEXERCISED`: `reference_trace`'s refusal of
> `BufferLeased`/`BufferLeaseReturned` is now proved
> (`QCore.TaskTraceLeaseBoundary`) and exercised on a real reference run
> (`compiler/tests/trace_alphabet_lease_boundary.rs`), the arm is measured
> **reachable** today, `derive` is measured the only route by which a lease could
> reach an `AdmittedCausalOrder`, and dropping instead of refusing is measured to
> be an **admitting** mistake (`the_dropped_encoding_would_have_been_admitted`).
> What is NOT decided is the forward question. Growing the alphabet takes CT03's
> method — a conservativity theorem *plus* a rule shown non-vacuous in both
> directions — and the second half is unavailable while a worker **holds** but
> does not **exercise** its lease (`WRITE-LEND-TASKS-EXERCISE`): there is no
> worker-side range write for a lease-window rule to admit or refuse, so the two
> constructors would carry no constrainable fact. The operative consequence
> today is a measured boundary, not a defect: write-lend programs sit outside
> F02 relational comparison by construction, their native evidence being
> sanitizer and allocation-count rather than a trace relation
> (`the_two_lease_program_runs_sanitized_with_one_allocation_counted_once`), so
> the refusal blocks no claim anyone is making. **Condition:** when
> `WRITE-LEND-TASKS-EXERCISE` lands a worker-side range write, this arm either
> fires correctly on programs that card wants compared, or blocks it. That card
> must re-decide the boundary explicitly and record which: either keep the
> refusal and state the comparison gap it accepts, or grow the alphabet by CT03's
> method with both halves supplied. **Discharge:** the growth decision taken and
> recorded, with a conservativity theorem beside
> `write_free_traces_relate_exactly_as_before` if it grows, or a stated and
> measured comparison gap if it does not. Standing constraint 15n governs the
> arm until then.

### 5.3 Standing constraint landed

`AGENTS.md` gains **15n** (the F02 trace boundary refuses a range write lease,
and refusing is the measured answer — do not turn it into a drop), carrying the
three facts that must survive refactors: the arm is reachable, dropping is an
admitting mistake, and the index is the first offender's own.

### 5.4 For the integrator, at the wave-32 close

`AGENTS.md`'s **Merged gate** row is deliberately **left untouched**. It is
sourced to `docs/reports/wave31-close/gate.json` and labelled "measured at the
integration tip over the wave's six merged cards"; rewriting it with one card's
numbers would misattribute a card measurement as a wave-tip one. What this card
moves, for the close to fold in:

* workspace tests **4,511 → 4,516** (this card's five, no other delta);
* Lean audits **125 → 126**;
* Python **200**, with the `lean_audits` tripwire red until the matrix is
  regenerated (§7.1);
* provider (122) and every corpus row **carried** — no arbiter ran.

---

## 6. Discipline

* **No refusal weakened.** The arm's behaviour is byte-identical; the diff to
  the production file is documentation only. Nothing moves from `Unknown` toward
  `Safe`; nothing is admitted that was refused.
* **No `Q` codes allocated** (`Q-NAMESPACE-DECISION` is iceboxed, and this card
  needs no code — it touches no diagnostic).
* **No word reserved**, no route selected differently, no keyword census
  engaged.
* **No arbiter, and the corpus was not run** — per the card brief. No gateway,
  contract, audit or provider file is touched, so no corpus row can move; the
  headline corpus numbers are carried, not re-measured.
* **Lean parity** (invariant 6): the semantic content this card adds is stated
  in Lean and registered in an audit, not only in a test.

---

## 7. Gate

Run at the card tip. Measured, not asserted; the Lean line is quoted by its own
text.

| Check | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **4,516 passed, 0 failed, 8 ignored** (4,524 counted across 226 result lines). Wave 31's tip was 4,511 passed / 8 ignored; the delta is exactly this card's **five** new tests |
| `cargo fmt --all -- --check` | clean (exit 0) |
| `cargo clippy --all-targets -- -D warnings` | clean (exit 0) |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **exit 0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **exit 0** |
| `PATH="$HOME/.elan/bin:$PATH" python3 scripts/check_lean_audits.py` | exit 0, final line verbatim: `Passed 126 Lean audits` (**125 → 126**; `AuditTaskTraceLeaseBoundary.lean` is the new one, and its printed axioms are `propext`, `Quot.sound`, `Classical.choice` or none) |
| `python3 -m unittest discover -s scripts/tests` | **200 tests, 199 pass, 1 expected failure — the disclosed Lean tripwire** (see §7.1) |

### 7.1 The one red row, disclosed

`test_build_matrix_sections.py` compares the committed
`acceptance_matrix.json`'s `validation.lean_audits` against the live count of
`proof/Audit*.lean`:

```
AssertionError: Lists differ: ['14.0.0', 125] != ['14.0.0', 126]
```

This is the tripwire firing **by design**, exactly as it did for
`WRITE-LEND-TASKS` at wave 31 (`lean_audits: 123 != 124`, "left for the
integrator"). A card that adds an audit cannot make it green: the committed
matrix is regenerated by the integrator (constraint 25's `section_ownership`
discipline forbids a card regenerating the whole file), so the row is red from
the moment the audit lands until that regeneration. **It is a disclosure, not a
failure to fix here** — no test was loosened, and the audit was not withheld to
keep the row green.

Nothing in constraint 16's five load-sensitive rows tripped: the workspace run
reports zero failures across all 226 result lines, so no isolation re-run was
needed and none is claimed.

### 7.2 Not run, deliberately

No arbiter and no corpus run — the card brief's instruction, and correct on the
measurement: the diff touches no gateway, contract, audit or provider file, so
no corpus row can move. Provider packages were not rebuilt (constraint 18 is not
engaged: no template and no `qcore-schemas` major version changed).
