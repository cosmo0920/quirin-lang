# W43-INDEX-TRAVERSAL: the frozen forecast

Frozen before the admission was committed or measured on the corpus. Nothing
here is edited after the AFTER run; corrections live in the report.

## Base and census

* Base: `0a8f59ac` (local `main` `ec70709a` with W43-INDIRECT-ACCESS-GENERAL
  `6bb838c8` merged). BEFORE run of the base's release binary
  (`db895e6e…`): diagnostics, chunk-contracts and source byte-identical to
  `docs/reports/w43-indirect-access-general/artifacts/`; `contracts` and
  `source-effects` differ by the checkout path and main's verdict-neutral
  contract records, as the brief lists.
* Census: commit `42ee628f` (instrumentation and the pure recognisers only,
  no rule consults them), release binary with the `indirect-census` feature
  (`e9f840b1…`). All five artifacts of the census run are byte-identical to
  BEFORE: the instrumentation is inert. Traversal stream sha256 `4dd29e70…`
  (not committed, 71 MB with the analyzer's repeated passes).
* A first census run died of a full disk (other worktrees filled the shared
  volume); it is set aside as `private/runs/aborted-census-enospc` and is not
  evidence.

## Starting point (Unknown functions; the base census is
`docs/reports/w43-indirect-access-general/blocker-census.txt`, byte-identical)

| Family | Leaves | Functions | Sole |
|---|---:|---:|---:|
| branch condition | 8,712 | 2,408 | 50 |
| for-loop condition | 927 | 602 | 1 |
| loop condition | 318 | 203 | 3 |
| switch expression | 96 | 90 | 1 |
| do-while condition | 31 | 29 | 0 |
| unmodeled increment or decrement | 678 | 345 | 1 |
| unrecognized assignment destination | 772 | 385 | 2 |
| unsupported assignment expression | 1,645 | 569 | 5 |
| formal-mutation gate | 118 | 69 | 0 |
| unresolved resource identity | 4,512 | 1,446 | 5 |

## The planned rules (recognisers in `42ee628f`)

* **R1 typed condition.** A refused controlling expression with no call whose
  every access is typed (`typed_scalar_condition`) lowers to the typed read,
  with each base's dereference obligation at the header.
* **R2 pointer step.** `P++`, `P += E`, `P = Q ± E` over declared pointers
  (`pointer_step`) lowers to `PointerStep`: interior-derived, custody-neutral.
* **R3 unbound typed copy.** `P = Q` between declared pointers (or declared
  scalars) with `Q` unbound gives an unbound `P`, no `unknown_alias`.
* **R4 index facts.** A declared scalar's fact joined with no fact is no fact;
  returning a scalar's target-free binding escapes nothing.
* **R5 stepped access.** `*P++ = E`, `X = *P++`, `B[X++] = E` lower to the
  access over the old values, then the step or the index's forget.
* **R6 repair.** A typed access whose index writes refuses (the hole below).

## Forecast

From `scripts/predict.py` over the census ([census/predict.txt](census/predict.txt)):

| Class | Leaves (all statuses) | In Unknown functions |
|---|---:|---:|
| condition (R1) | 580 | 524 |
| step (R2, R5) | 302 | 292 |
| gated-step (R2, R5 past the gate; upper bound) | 32 | 30 |
| scalar-alias (R3, R4; name heuristic) | 698 | 650 |

By family, in Unknown functions: branch condition −394, for-loop condition
−86, loop condition −24, switch −18, do-while −2, increment −149,
unrecognized destination −104, unsupported assignment −39, gate −30,
unresolved identity −650. Condition leaves may be **exchanged** for an
`unknown_alias` where the base is bound and possibly null; the forecast cannot
see that, and expects most condition removals over caller-provided bases to be
clean.

**Functions Unknown → Safe: at most 13** (8 from the lexical classes alone),
before exchanges:

* `cio_chunk_is_file`, `cio_file_native_filename_check` (chunkio)
* `decode_hex_digit` (ctraces)
* `gf2_matrix_times`, `bitLen`, `rd_kafka_q_check_yield`, `rd_string_hash`,
  `ut_write_varint` (librdkafka)
* `char_lookup`, `reverse_char_lookup`, `str_searchr`, `mk_kernel_features`,
  `mk_server_lib_notify_event_loop_break` (monkey)

Other forecasts:

* **Unit moves: 0.**
* **Violation candidates: 0.** No census statement steps and then releases a
  tracked pointer, and the accusing arms need a release or a use after the
  base's release through a walker.
* **Leak pin 271: unchanged**, low confidence. R6 can add a genuine leak where
  a pruned branch returns, R2's refusal of a lost base withdraws a leak it
  cannot claim, and R4 changes which loop-exit paths are feasible.
* **R6 blast radius: 23 statements in 11 functions** were admitted by the
  typed route with a writing index; all 23 are `B[X++]` shapes that R5
  re-admits with the forget, so R6 is forecast to move no leaf by itself, and
  any verdict movement in those 11 functions is the repair.

The function count is small because traversal functions in this corpus carry
other blockers (calls, member chains, list-iteration macros) that no rule here
touches; the leaf movement is the measure of the rules themselves.
