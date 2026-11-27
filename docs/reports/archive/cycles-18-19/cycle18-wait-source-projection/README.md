# Cycle 18 timed-wait source projection

This card adds the source-and-contract half of one bounded Monkey callback
composition for `cb_wait_2_timeout`. It does not create an `EventProgram`, a
composition certificate, or a `Safe` verdict. A caller must still join every
returned requirement to an authenticated native observation and run the
existing lifecycle checker.

## Checked boundary

`MonkeyWaitContracts::parse` reads the existing declarations in
`monkey/wrapper_paths.qcore` and `monkey/composition.qcore`. It requires the
exact selected declaration headers and clause sequences for
`cb_wait_2_timeout`, `_mk_event_loop_create`, `_mk_event_wait_with_flags`,
`_mk_event_wait_2`, and `_mk_event_loop_destroy`. Changing the callback borrow
to ownership, adding an effect row, removing a clause, or substituting a clause
fails closed. The sealed value carries SHA-256 digests under the canonical
contract keys. These declarations state the intended facts; their presence is
not proof that the C source implements them.

`project_monkey_timed_wait_source` supplies that second check. It accepts only
constructor-private typed summaries imported by `qcore-c-gateway`, requires
the Monkey/Fluent Bit profile and pinned source revision, checks the exact five
signatures and all imported path shapes, and selects paths 0, 3, 0, 2, and 0 in
the order above. Primitive matching of typed C operations to the declarations
is a trusted Rust verification boundary. The complete serialized summaries,
including operands, stores and guards, must equal either the approved fixture
digest vector or the approved production digest vector; vectors cannot be
mixed. Canonical summary digests and profile assumptions are exposed for the
outer identity check.

The callback must be a sole `Return`; together with its exact borrowed/no-
resource-effect contract this yields the sealed
`BorrowedNoEffectNoTasks` summary. The timed path must call `event_new`,
`event_add`, `_mk_event_wait_with_flags`, and `event_free` in order. Projection
returns these observation requirements:

1. successful base creation;
2. successful timeout event creation for `cb_wait_2_timeout`, flags `0x01`, no
   descriptor, and an explicitly observed priority;
3. successful event registration;
4. dispatch begin;
5. timeout activation;
6. callback enter and exit;
7. dispatch end;
8. event free;
9. argument-scope end;
10. base free.

No default priority is assumed. The independent observer must report
`event_get_priority`. The source leaves `event_new` and `event_add` failures
unchecked; those executions remain `Unknown` and this success-path projection
does not cover them.

## Production convergence

The ordinary focused test uses the existing compiler fixtures. An explicit
ignored test imports a preprocessed translation unit from the configured
Monkey build when `QCORE_MONKEY_PREPROCESSED` is set. It is ignored in the
standing workspace run so absence of that external input cannot be reported as
a production pass. The separate recorded run used the `mk_event.c` entry in
`build/cycle19/c-gateway-wave-pin/monkey/monkey-build/compile_commands.json`,
replacing its compile-only output arguments with `-E -P -o
/tmp/qcore-monkey-mk-event.preprocessed.c`, then ran:

```sh
QCORE_MONKEY_PREPROCESSED=/tmp/qcore-monkey-mk-event.preprocessed.c \
  CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=4 \
  cargo test -p qcore-verification --test monkey_wait_projection \
  configured_pinned_translation_unit_matches_the_fixture_projection \
  -- --ignored --nocapture
```

The production source, compile database, preprocessed artifact, summary-vector
and contract hashes are recorded in `production-source.json`. The same test
also passed against the Cycle 18 native observer's preprocessed input, whose
digest is identical despite its pinned headers and sanitizer flags. The
regular focused suite passed five tests and explicitly ignored that one test.
Its same-count regressions change the `event_new` descriptor, the copied result
field, and the timeout guard; all three are refused by full artifact identity.

## Lean parity and limits

`QCore.MonkeyWaitProjection` models exact source selection, exact declarative
contract facts and the ordered native obligations. It proves accepted
projections contain both exact inputs, accepts the intended pair, and rejects
missing lifecycle source, a callback effect, missing event free, and a missing
callback contract fact. Its audit contains no project axioms.

This card closes a prerequisite projection boundary only. It does not change
the three-callback/eight-path coverage denominator. Coverage may advance for
the one timed wait path only after the native identity/context join discharges
all observation requirements and the existing event lifecycle replay accepts
the assembled program. Missing, reordered, duplicated, unauthenticated or
identity-mismatched observations remain `Unknown`.

## Integrator reconciliation list

No global acceptance matrix, status ledger, history page, provider, schema or
profile changed. Existing QCore contracts were sufficient, so the pinned
contract-corpus counts did not change. The integration card owns the eventual
composition result and any coverage update.

Validation details and private log paths are in `gate.json`. The local full
workspace runs were cancelled after later semantic review made their results
obsolete. The final combined integration gate owns the full-workspace result
for the corrected tip.
