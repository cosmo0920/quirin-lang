# Cycle 19: Monkey descriptor identity observations

The pinned Monkey harness now binds descriptor creation, I/O, event callbacks,
and successful closes to `(fd, generation)` identities. On 2026-09-06 the actual
Fluent Bit revision `66910c10a4d7eafa810b84229cf2dcf7b0f26f97` produced an accepted
**48-record runtime identity projection**. This advances the 19D descriptor
binding gate; it is not a source or whole-program safety certificate.

| Observed obligation | Result |
| --- | --- |
| Socketpair acquisitions | 4 descriptors from 2 successful pairs |
| Descriptor-number reuse | 2 descriptors reacquired at generation 2 |
| Failed socketpair | 1 injected failure through the actual channel wrapper; no custody created |
| Descriptor I/O | 3 observed sends/receives, including the timer's failed send |
| Successful closes | All 4 acquired descriptor identities discharged |
| Actual callbacks | `cb_event`, `cb_timeout`, `cb_wait_2_timeout` |
| Observed backends | `epoll` on both bases |
| Allocator regression | All 145 native QCore executions still checked |

The hooks wrap the actual Libevent socketpair, send/receive, event, and close
operations. Generations persist across event bases. Event observations retain
the descriptor identity captured at event creation; timeout-only events carry
no descriptor. The checker verifies the base/dispatcher/thread binding,
registered callback entry, matching callback exit, and event/base teardown.
It permits the actual timeout callback to free its own event before exit.
Closing a timer reader before unregistering its idle I/O event is retained;
subsequent I/O callback entry or registration on that closed identity is rejected.

The socketpair failure is injected before calling Libevent, with `EMFILE` and
no output descriptors. The actual Monkey channel wrapper handles the failure.
This does not simulate successful acquisition or claim coverage of event/base
allocation or registration failures. `socketpair: Too many open files` and
`write: Broken pipe` in stderr are the expected injected failure and timer
teardown diagnostics.

## Checker and evidence

`verifier_plugins::libevent::monkey_observation::check_monkey_observations`
reads schema-version-1 JSONL and returns a runtime report. The schema is separate
from verifier protocol v1. Older unversioned traces cannot supply the missing
identity evidence and are rejected. The checker rejects missing/gapped records,
unknown fields, stale generations, duplicate acquisition/close, failed-pair
custody, changed event bindings, mismatched threads/dispatchers, and outstanding
teardown obligations. Failed native closes are unresolved because their custody
semantics depend on the platform; they are not presumed to retain a live fd.

- [Recorded trace](monkey_observation.jsonl)
- [Checked runtime report](monkey_identity_result.json)
- [Native stderr](monkey_observation.stderr)
- [Source, build, harness, dependency and observation hashes](metadata.json)
- [Allocator regression result](flb_mem_contract_result.json)

The metadata records the disposable native output directory, including the
allocator observation hash. The allocator corpus and its interpretation are
also documented in the preceding [native report](../cycle19-native-contracts/README.md).

## Validation

Rust regression tests include the actual recorded trace, descriptor reuse,
stale-use and stale-close mutations, missing records, changed event identity,
thread/base mismatches, active-callback close, and unsupported schemas/fields.
`QCore.DescriptorObservation` adds executable generation-slot acquisition/use/
close semantics and six audited theorems for next-generation acquisition,
stale-use rejection, successful discharge, duplicate-close rejection,
unresolved failed close, and rejection of a previous generation after reuse.
These prove the abstract slot model, not the Rust/JSON or native hook bridge.

The native harness passed AddressSanitizer, UndefinedBehaviorSanitizer, and
LeakSanitizer. Textually included source is instrumented; prebuilt linked
archives retain their original flags. All 288 Rust tests, formatting, Clippy,
Lean build, Cycle 19 dependency audit, and the pinned external source audit
passed. See the accompanying logs.

The external audit remains at 185 units, 4,346/4,775 functions projected,
2 `Safe`, 54 provisional `Unsafe`, 125 `Unknown`, 4 data-only units, and
zero gateway failures. The two existing native field contracts still check.
These classifications are unchanged; provisional `Unsafe` is not a confirmed
upstream defect.

## Reproduction and remaining gates

```sh
python3 scripts/cycle19_native_observations.py \
  --fluent-bit /home/cosmo/Documents/GitHub/fluent-bit \
  --monkey-build /tmp/qcore-cycle19-monkey-build-codex-20260905 \
  --output /tmp/qcore-cycle19-descriptor-observations

cargo run --quiet -p qcore-compiler --example check_monkey_observations -- \
  docs/reports/cycle19-descriptor-observations/monkey_observation.jsonl
```

Native execution requires Unix socketpair and sanitizer access. The runner
checks the copied Monkey sources/dependencies against the pinned checkout and
automatically invokes the Rust observation checker.

Cycle 19 remains in progress. General 19B source-call identity mapping, 19C
nested teardown/failure imports, and 19D source-derived callback payload/map
access traces and allocation/registration failures remain open. Recorded
observations are supplied evidence, not authenticated traces. The checker does
not infer unrecorded I/O, prove Libevent scheduling, or promote these executions
to `check_monkey_composition` or plugin `CoreCertificate` values.
