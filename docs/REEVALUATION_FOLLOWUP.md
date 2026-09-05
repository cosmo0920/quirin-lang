# Reevaluation follow-up: remaining verifier boundaries

**Historical audit:** 2026-09-05, commit `3827db1c0be3bafdb7d069729c311bfa3632d0d9`.
The original six audit expectations passed at that baseline. This follow-up identifies five
new issues and supplies executable reproductions for two previously suspected
gaps (VP04 and VP05). Seven findings produce eight failed probe expectations;
three positive controls pass. Two probes exercise the same mutex root cause.

The [new probe](../compiler/examples/reevaluation_followup.rs) calls public APIs
and includes the production Clang gateway for the legacy-C example. It does not
change verifier behavior. [JSON evidence](reevaluation/followup/probes.json)
and [execution metadata](reevaluation/followup/metadata.json) are archived.

The findings and archived logs below describe the audit baseline. The
[superseding repair record](#repair-and-containment-on-2026-09-05) documents the
current implementation and remaining unsupported features.

## Findings in priority order

### LC01 — P1: an allocation wrapper hides a resource leak

In `analyze_legacy_c_translation_unit_selected_partial`,
[`legacy_c.rs`](../compiler/src/legacy_c.rs), each internal function is inserted
into the contract environment as `ResourceApiKind::Neutral`. This replaces any
supplied contract for that name and does not propagate inferred ownership of
returned resources to callers.

Under the explicit trusted contract that `q_alloc` returns a fresh non-null
owned resource, this translation unit leaks it:

```c
void *q_alloc(void);
void *make(void) {
    void *p = q_alloc();
    return p;
}
int main(void) {
    void *p = make();
    return 0;
}
```

**Observed:** `analyze_unrestricted_c11` produces `Safe(LegacySafetyCertificate)`
covering both `main` and `make`. The retained caller CFG labels `make`'s result
as `None`, while the callee escapes its owned result. Consequently neither
function retains the obligation to discharge that resource. Moving the same
allocation directly into `main` produces `Unsafe` with a leak diagnostic.

This is a resource-safety false positive, not merely missing syntactic coverage.
Reanalyzing the certificate repeats the same missing call effect and accepts it.
The counterexample relies on the declared resource contract, not unspecified
behavior of a real allocator or a new native execution.

**Required fix:** infer and validate interprocedural parameter/return custody
summaries, preserving identities and costs at calls. Until supported, any call
whose effects are not independently proven neutral must keep the unit Unknown.
Do not overwrite explicit contracts optimistically. Add caller-discard,
caller-release, nested-wrapper, parameter-consumption and recursion cases.

### WM05 — P1: mutex histories are not validated before certifying ordering

[`weak_memory.rs`](../compiler/src/weak_memory.rs),
`expected_synchronizes_with`, accepts a supplied unlock-to-lock edge based on
matching mutex IDs. No mutex owner state or complete acquisition order is
validated by `check_c11_execution`.

**Observed:** two successful acquisitions by different threads without an
intervening unlock receive a certificate. A second candidate contains a write,
an unlock without a prior acquisition, a matching lock in another thread, and
another write. Its supplied edge establishes happens-before, and it also
receives a certificate. These are invalid complete mutex histories for an
initially unlocked mutex. The API supplies neither initial ownership nor a
validated external mutex-history premise.

C11 requires unlock by the owning thread and defines synchronization through
mutex operations; matching identities alone does not establish their validity.
[WG14 N1570, §7.26.4](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf)

**Required fix:** model successful lock/unlock transitions, owner thread,
initial state and mutex order, or require a separately checked mutex-history
certificate. Reject unsupported histories before using their edges. These
probes target the standalone C11 checker, not a demonstrated bypass of the
separate source concurrency checker.

### VP04 — P1 for the bounded process-provider contract: timeout overrun

[`verifier_plugin.rs`](../compiler/src/verifier_plugin.rs),
`ProcessProvider::verify`, kills the direct child at the deadline but then
performs blocking joins on I/O threads. Descendants retain inherited pipe ends.

**Observed:** a provider shell starts a bounded 400 ms sleep child and waits.
A 20 ms provider deadline returns `Err(Timeout)` only after **403 ms** in the
archived run. The test allows 200 ms scheduling tolerance and still fails.
This promotes the earlier inspection hypothesis to an executed finding.
An indefinitely running descendant could prevent EOF indefinitely; that
unbounded variant was not executed.

**Required fix:** terminate the owned process tree and bound pipe completion
using the same deadline. Make cleanup bounded on successful exits and errors
as well as timeouts. Keep legitimate slow output and direct-child timeout
controls; run the descendant case under an outer harness deadline.

### VP08 — P1 for process lifecycle containment: rejected input leaves a provider running

In the same function, `.spawn()` precedes `encode_protocol_frame(...)`. An
encoding error returns through `?` before timeout monitoring or child cleanup
is installed.

**Observed:** with a one-byte output/request budget, the request immediately
returns `Err(OutputLimit)`. A marker is absent at return, but the provider
creates it after its bounded 50 ms delay. Its configured timeout is 20 ms.
The process therefore continues after the invocation has returned an error.
The marker is confined to the probe's unique temporary directory; the helper
exits and the probe removes that directory.

**Required fix:** encode and validate the complete request before spawning.
Install a child/process-tree cleanup guard immediately after spawn and use it
on all error paths. A regression should prove that rejected requests never
launch a helper and that post-spawn I/O failures terminate and reap it.

### VP05 — P2 integration gap: source verification declarations never invoke the verifier

[`driver.rs`](../compiler/src/driver.rs), `compile_files_with_registry`, calls
`resolve_source_scopes`, discards its returned scopes, and proceeds with normal
compilation. It does not generate or dispatch verification requests.

**Observed:** a valid source with `verify scope` and a registered counting
provider compiles successfully with **zero provider invocations**. A direct
registry request invokes that same provider, confirming the counter is live.
The source fixture has an empty `main`; this establishes missing invocation,
not an unsafe application receiving a source certificate.

**Required fix:** implement obligation generation and request dispatch, or
explicitly expose this feature as configuration validation until integration
is complete. Test the effect of unavailable, unknown and rejecting providers
on nonempty source obligations. This confirms the earlier VP05 inspection.

### VP06 — P2 evaluation integrity: unsupported negative verdicts are trusted

[`verifier_plugin.rs`](../compiler/src/verifier_plugin.rs), `check_response`,
passes `ProposedVerdict::Unsafe` through after envelope checks without checking
a violation witness. The new built-in provider correctly returns Unknown for
unsupported assertions, but custom providers do not share that protection.

**Observed:** the custom provider returns Unsafe with no evidence and no
diagnostics for an empty-fact request. The registry reports Unsafe. This is
an unvalidated allegation, not a demonstrated violation of a source program.
Composition also gives this status precedence over Unknown results.

**Required fix:** validate a reachable counterexample, or expose a separate
provider-allegation status and exclude it from verified unsafe counts. If
trusting negative provider verdicts is intentional, document that trust
explicitly. This finding concerns the reevaluation verdict contract; it does
not bypass the newly closed Safe-certificate path.

### VP07 — P2 boundary inconsistency: byte limits use different units

The public frame encoder/decoder apply their limit to JSON payload bytes.
`ProcessProvider` passes its output limit directly to `read_limited`, which
counts the four-byte header too, and later passes that same limit to decoding.

**Observed:** a 4,285-byte payload / 4,289-byte frame succeeds through both
codec helpers with limit 4,285. The actual process transport returns
`Err(OutputLimit)` with that limit. Raising the process limit by four makes
the same response succeed.

**Required fix:** define whether the manifest budget counts payload or wire
bytes and use that convention consistently, with checked header arithmetic.
Give input and output explicit budgets if they differ. This is a transport
interoperability issue, not a false-safe result.

## Reproduction and validation

```sh
cargo run -p qcore-compiler --example reevaluation_followup
```

At the audit baseline, the example exited **1** for eight unresolved
expectations. The repaired example exits zero when its executed expectations
pass, including explicit unsupported-integration rejection for VP05. Process probes are Unix-only, use
`/bin/sh`, `/bin/cat` and `/bin/sleep`, and complete in under a second on the
audited machine. The legacy-C probe requires Clang. Consult individual probe
results, not just the aggregate, when a tool is unavailable or a profile is
unsupported. No unbounded child process or external network call is used.

The existing workspace suite passed **208 library + 17 integration tests**.
Workspace formatting and Clippy (`--all-targets -- -D warnings`) passed. The
original reevaluation example passed all eight expectations (six regressions,
two controls). These passes demonstrate that the new cases expose gaps in the
existing coverage; they do not establish that the follow-up findings are fixed.

The LC01 and WM05 findings should be addressed before expanding safety claims.
Process lifecycle fixes should precede running third-party providers as a
bounded service. Record VP05 as an integration milestone and VP06–VP07 as
verdict/transport contract decisions with matching tests.


## Repair and containment on 2026-09-05

Implemented against `9edc9708cac7ed0692211a35b9f4f17629c93730`. These changes
close the reproduced acceptance and lifecycle gaps without claiming that the
missing interprocedural or plugin semantic infrastructure is implemented.

| Finding | Current behavior | Remaining boundary |
|---|---|---|
| LC01 | Internal definitions no longer receive automatic neutral contracts. Explicit trusted contracts are preserved; uncontracted calls retain Unknown instructions. | Inferred parameter/return custody, identity, and cost summaries remain unimplemented. A detected independent leak can still make the aggregate Unsafe. |
| WM05 | `C11Execution::mutex_order` supplies a complete order per mutex. The checker validates exact coverage, successful acquisitions, owning-thread unlocks, and agreement with happens-before before using handoff edges. | The supported profile starts with unlocked non-recursive mutexes; a finite prefix may end with a held lock. Missing orders, recursion, and initial ownership are unsupported. |
| VP04 | Nonblocking pipe I/O shares one deadline. A Unix process-group guard terminates ordinary descendants on success, timeout, and errors; direct-child reaping does not block the caller. | This is process-group containment, not a sandbox for deliberately detached descendants or OS memory/network enforcement. Non-Unix process execution is rejected. |
| VP08 | Encoding and request-size validation happen before spawn; the cleanup guard is installed immediately after successful spawn. | Request and response payloads currently share the manifest budget. |
| VP05 | Configuration validation remains available through `resolve_source_scopes`. Compilation rejects resolved scopes with `Q701`; unavailable or incompatible providers retain `Q700`. | Source obligation generation and dispatch remain unimplemented. No provider runs through a source scope, and no source verification success is implied. |
| VP06 | Every unchecked Unsafe proposal becomes Unknown with `QP010`; original diagnostics are preserved as allegations. Composition therefore cannot count it as verified Unsafe. | Reachable counterexample witnesses and their core checker remain unimplemented. |
| VP07 | Codec and transport limits both count JSON payload bytes. The transport allows four additional frame-header bytes using checked arithmetic; stderr has a separate 64 KiB cap. | Separate request/output manifest limits would require a future contract revision. |

The follow-up probe now passes all **11 expectations** (eight regressions and
three controls). VP05 explicitly accepts `Q701` as the documented containment
alternative to dispatch; the mutex probes now supply complete candidate orders
so they exercise invalid transitions rather than merely missing order data.
The inherited-pipe timeout returned in **20 ms** with a 20 ms deadline in the
recorded repair run. The original reevaluation probe still passes all eight
expectations. Baseline evidence is preserved; repair evidence and source hashes
are under [`reevaluation/followup/repair/`](reevaluation/followup/repair/).

Validation passed **212 library + 20 integration tests**, workspace formatting,
and Clippy with `--all-targets -- -D warnings`. Added cases cover caller-discard,
caller-release, nested wrappers, parameter consumption, recursion, preserved
explicit return contracts, valid mutex handoff, invalid/missing mutex orders,
unowned unlocks, held-lock prefixes, source-scope rejection, unsupported
negative verdicts, inherited pipes after timeout and normal child exit,
pre-spawn rejection, post-spawn I/O failure, successful-exit cleanup, delayed
valid output, and exact/over-limit response payloads. Process regressions run
under a two-second outer harness deadline with bounded helpers.

`lake build` and `lake env lean AuditCycle20.lean` pass. The new
`rejected_proposal_is_unknown` theorem uses no axioms. This audit does not claim
a Lean refinement proof for the new Rust mutex or operating-system code.
