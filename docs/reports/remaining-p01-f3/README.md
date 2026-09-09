# P01-F3 — the unverified loader, and the check-to-dispatch window

Work card `P01-F3` (wave 10 follow-on to `P01-F2`), 2026-09-09. It closes the
two residuals [`docs/reports/remaining-p01-f2/README.md`](../remaining-p01-f2/README.md)
§9 recorded open and could reach:

* **`ProcessProvider::from_package` remained an unverified loader** — a plain
  read of `verifier-plugin.json` that checked neither the declared digest nor
  the embedded-template binding, and whose result was a fully registrable
  provider;
* **time of check to time of use** — package files could change between
  verification and dispatch, and nothing looked again.

The third residual, *a rebuilt or forged executable carries its builder's
template*, is **out of scope and unchanged**. It is not a gap this card could
narrow: an actor who produces the binary produces its embedded template too, so
only signature verification over a trusted build record would decide it. None is
added here, and none is claimed.

No verdict, provider semantics, diagnostic code or wire version changes. All
five shipped packages remain protocol-v1 `Unknown` providers; no raw Cycle 19
unit is promoted; `qcore-schemas` is untouched.

## 1. What changed, in one paragraph

`ProcessProvider` — the only thing a `VerifierRegistry` accepts — can no longer
be constructed without verification. Its two constructors are crate-private:
one takes a `VerifiedExecutable` and is reachable only from
`verify_executable_package`, the other is reachable only from the new
`UnverifiedProvider`, which is **not** a `VerifierProvider` and so cannot be
registered. `verify_executable_package` now opens the executable once and
*keeps the descriptor*; every spawn re-reads the file through that descriptor,
re-compares the verified digest, re-binds the embedded template, and requires
the executable path to still name the same file — refusing, with a distinct
reason and nothing spawned, when any of those moved.

## 2. The loader: a type that says "unverified"

### 2.1 Disposition

| Route | Before | After |
|---|---|---|
| `verify_executable_package(path)` | verified: containment, digest, template binding | unchanged, **plus** it keeps the descriptor for the spawn-time re-check |
| `ProcessProvider::from_package(path)` | **unverified plain loader**, result registrable | delegates to `verify_executable_package` |
| `ProcessProvider::new(manifest, root)` | **public, unverified**, result registrable | removed from the public API; two crate-private constructors replace it |
| `VerifierRegistry::discover_packages(root)` | registered whatever carried a parseable manifest — **including a package with no executable at all** | verifies each package; an unverifiable one is an error and is not registered |
| `qcore_compiler::load_first_party_registry` | built the executable arm with `ProcessProvider::new` from *profile metadata alone* | verifies the package, and additionally requires the installed manifest to equal the profile's declaration |
| `qcore_audit::source_contract::SourceContractRegistry` | verified (`P01-F2`) | unchanged |
| CLI `plugins verify` / `enable` / registry loads | verified (`P01-F2`) | unchanged |
| *unverified loading, deliberately* | — | `UnverifiedProvider::load` / `::new` |

Two of those rows are the real finding. `discover_packages` and
`load_first_party_registry` were production routes that registered an executable
provider on the strength of a JSON file: the discovery test this card inherited
installed *no executable whatsoever* and a placeholder digest, and discovery
registered it. That test now asserts both halves — the manifest-only directory
is refused, and the installed package is discovered — so the repair is pinned in
the direction it moved.

### 2.2 Why a type, not a flag

`UnverifiedProvider` exists because one caller genuinely needs unverified
loading: the conformance harness drives *deliberately mutated* packages to check
that the executable's own self-check refuses them, which is a property of the
package, separate from the host's gate. Expressing that as a boolean on
`ProcessProvider` would leave the unverified state one field away from a
registry. As a distinct type it is one `impl` away, and that `impl` has to be
written by hand:

* `UnverifiedProvider` does not implement `VerifierProvider`, so
  `registry.register(Arc::new(provider))` **does not compile** — pinned as a
  `compile_fail` doctest;
* it exposes no owning accessor, and `Deref` yields only `&ProcessProvider`, so
  `let taken: ProcessProvider = *provider;` **does not compile** — a second
  `compile_fail` doctest;
* `ExecutionPolicy::apply` keeps taking and returning `ProcessProvider`; the new
  `apply_unverified` takes and returns `UnverifiedProvider`, so applying a
  policy is not a laundering route;
* `UnverifiedProvider::into_verified` is the one bridge, and it runs the same
  gate, preserving the execution profile, confinement policy and mechanism
  selection chosen before it.

The escape that remains is explicit and visible: a caller can write a newtype
that implements `VerifierProvider` by delegation. One exists, in the adversarial
harness, named `DeliberatelyUnverified` and commented as such
(`providers/conformance/tests/composed_five_providers.rs`) — those cases register
manifests that are *deliberately* mutated, to drive the registry's own
`QP004`/`QP006` gates, and such a manifest cannot bind to a shipped executable's
template by construction. Nothing in production writes one. This is a type-level
discouragement of an accident, not a capability boundary.

## 3. TOCTOU: the mechanism, and the window it leaves

### 3.1 What is checked, and where

`crates/qcore-verifier-host/src/executable_identity.rs`. At verification the
executable is opened once; the same bytes decide the digest and the template
binding, and the `File` is kept in the provider together with the verified
digest and the file's `(device, inode)`. Immediately before every spawn —
after the limit plan, after the confinement and quota objects are built, as the
last act before `Command::spawn` — `VerifiedExecutable::revalidate` runs:

1. **read the whole file back through the held descriptor** (`pread` from
   offset 0, so the provider's own cursor is untouched) and compare the digest.
   The read goes through the descriptor, not the path: a path substitution
   cannot redirect it. A rewrite of the verified inode is refused here.
2. **re-bind the embedded template** against the installed manifest. Retained as
   defense in depth; see §3.4 for why it is unreachable behind step 1.
3. **`stat` the executable path** and require it to be a regular file with the
   same `(device, inode)`. A removal, an unlink-and-recreate, a rename over the
   path, or a directory in its place is refused here — including a
   *byte-identical* replacement, which step 1 cannot see and which is exactly
   what an `execve` on the path would have loaded.

The spawn then executes **the canonical path verification resolved**, not the
spelled entrypoint. Those differ whenever the entrypoint is a symbolic link, and
checking one path while `execve`-ing another would be no check at all. With the
canonical path, re-pointing the link cannot substitute the executable: the
verified file is what runs (measured — see the refusal matrix).

### 3.2 The measured residual window

The window is **narrowed, not closed**, and the code records its own upper
bound. `ProcessExecutionRecord::identity_window_us` is the elapsed time from the
completed re-check to `Command::spawn` returning — and `spawn` returns only
after the child has reported a successful `execve`, so the kernel has already
read the image. Anything a writer could do to the file after that point is not a
substitution of the executed image.

Measured on the five real shipped packages (x86_64 Linux, five dispatches each,
`providers/conformance/tests/packages.rs`):

| Package | Executable | Residual window (us) min / median / max |
|---|---|---|
| `qcore.chunkio` | 2,394,576 B | 2709 / 4012 / 5010 |
| `qcore.cmetrics` | 2,288,752 B | 1133 / 4034 / 6323 |
| `qcore.ctraces` | 2,256,976 B | 1034 / 2623 / 4745 |
| `qcore.libevent` | 2,284,480 B | 590 / 3372 / 4774 |
| `qcore.librdkafka` | 2,261,032 B | 361 / 2219 / 6351 |

So the residual is on the order of **1–6 milliseconds** per dispatch, and it is
the `fork`/`exec` itself. Before this card the window was the whole interval
from verification to dispatch — unbounded, and spanning every later dispatch of
the same provider. That is the honest size of the improvement: from "unbounded
and unmeasured" to "a few milliseconds, measured and published per execution".

### 3.3 Why the window is not closed

Executing the *held file description* would close it: `fexecve`, or
`execve("/proc/self/fd/N")`. It is deliberately not done, on two grounds, the
first measured:

1. **Measured.** An interpreted (`#!`) package cannot be executed through
   `/proc/self/fd/N` with a close-on-exec descriptor: the kernel starts the
   interpreter with that path as `argv[0]`, and by then the descriptor is gone.
   A scratch reproduction returns exit 2 with
   `/bin/sh: 0: cannot open /proc/self/fd/3: No such file`. Several fixtures and
   the CLI package fixtures are shell scripts; keeping the descriptor open past
   `exec` instead would leak a descriptor into every package.
2. **Policy.** Under `linux-confined-v1` the Landlock policy admits the package
   root and the runtime paths a provider needs — not `/proc`. Executing through
   `/proc/self/fd` would require admitting `/proc` to the confined child, which
   widens a confinement boundary (card `P03`) in order to narrow a race. That
   trade is not made here.

Rust's `std::process::Command` also offers no `fexecve` hook — `pre_exec` runs
*before* the exec that `Command` itself performs — so the alternative is a
hand-written `fork`/`exec` path that would have to reproduce the pipe,
process-group, `setrlimit`, Landlock and cgroup machinery. Recorded open as
**`P01-F3-FDEXEC`**.

### 3.4 One check is unreachable, and says so

Step 2 (the template re-bind) cannot fire while step 1 runs first: reaching it
needs bytes whose SHA-256 equals the verified digest but whose embedded template
differs — a preimage collision. It is retained because it costs a scan of bytes
already in memory and because it fails closed if the digest comparison is ever
weakened, and `IdentityRefusal::TemplateUnbound`'s doc comment records the
unreachability rather than implying coverage. No test claims to reach it; the
in-place-rewrite test asserts that a *stripped envelope* is named as changed
content, which is what actually happens.

### 3.5 Cost

The re-check is one SHA-256 over the executable plus one `stat`, per dispatch,
and it is not free. Measured on the same five ~2.3 MB packages, median per
re-check:

| Package | Release | Debug |
|---|---|---|
| `qcore.chunkio` | 107 ms | 674 ms |
| `qcore.cmetrics` | 80 ms | 1715 ms |
| `qcore.ctraces` | 142 ms | 898 ms |
| `qcore.libevent` | 54 ms | 1273 ms |
| `qcore.librdkafka` | 37 ms | 1118 ms |

`qcore-schemas` ships a pure-Rust SHA-256 with no hardware path, so the cost is
dominated by the build profile: **tens of milliseconds per dispatch in release,
around a second per dispatch in an unoptimized build**. That matters, because
the audit routes run through `cargo run --example`, which is a debug build by
default: a route that dispatches ~100 requests to one provider pays on the order
of a hundred seconds there, against a few seconds in release. This is a recorded
cost, not a hidden one, and it is the price of not trusting the file between
dispatches. Recorded open as **`P01-F3-COST`**: caching keyed on anything weaker
than the content (size, mtime, a kernel change notification) would reintroduce
exactly the substitution this card refuses, so no cache is added here.

**A regression this cost caused, and the repair.** The first implementation left
the wall deadline where it was, computed before the re-check, so the host's own
SHA-256 was charged against the *package's* `limits.timeout_ms`. On a debug
build that silently broke the configured process audit: two conformance cases
went from two and one checked function contracts to zero, with the provider
timing out and the audit correctly recording `Unknown`. The deadline now starts
after the re-check, because `limits.timeout_ms` bounds how long the *package*
may run, not how long the host spends deciding whether to run it. The
five-package measurement case above dispatches the real 2.3 MB executables under
their declared timeouts in a debug build, so it is also the regression test for
this.

Providers now also hold one open descriptor each for their lifetime (five, for
the five shipped packages).

## 4. Refusal matrix

Measured. Rows A–E use the shell-package fixtures in
`crates/qcore-verifier-host/tests/package_identity.rs`; row F uses the five real
shipped executables. Every row asserts, separately from the returned error, that
the substituted package **never ran** — each substitution writes a marker into
the package directory, so a spawn that happened at all is visible.

| Case | Before | Now |
|---|---|---|
| Z. unchanged package, repeated dispatches | answers | **answers**, and each record carries its measured window |
| A. the same inode rewritten in place after verification | **spawned the new content** | refused: `verified executable changed since verification: the same file now hashes to … not the verified …` (`RefusedByIdentity`) |
| B. unlink + recreate with *different* content | **spawned the new file** | refused: `verified executable … was replaced since verification: the path now names device …, inode …, not the verified …` |
| C. unlink + recreate with *byte-identical* content | spawned the new file | refused — content alone cannot see this, which is why file identity is checked |
| D. replaced by a *different but self-consistent* package (it verifies on its own) | spawned it | refused; the substitute verifying from scratch does not make it this provider's package |
| E. executable removed | spawn failed with an OS error | refused before spawning, naming the path |
| F. the five shipped packages, replaced after verification | spawned the replacement | refused, and the restored package verifies again — what moved was identity, not content |
| G. entrypoint is a symbolic link, re-pointed after verification | would have spawned the new target | the verified canonical file still answers; the decoy never runs |
| H. entrypoint is a symbolic link, *resolved* file rewritten | spawned the new content | refused as changed content |
| I. through a `VerifierRegistry` | — | scoped `Unknown` under `QP006`, no certificate, no downgrade |
| J. package with no executable, through `discover_packages` | **registered** | error, not registered |
| K. manifest-only or digest-mismatched package, through `from_package` | **loaded and registrable** | refused; unverified loading requires `UnverifiedProvider` by name |
| L. registering an `UnverifiedProvider` | — | **does not compile** |
| M. moving the `ProcessProvider` out of an `UnverifiedProvider` | — | **does not compile** |
| N. rebuilt or forged executable | out of scope | **still out of scope** — needs signatures |

A refused identity is recorded as the new `ProcessOutcome::RefusedByIdentity`,
distinct from `RefusedByProfile`: the profile was satisfiable and the *package
identity* was not. Like every other non-completed outcome it is a scoped
`Unknown` with no certificate, never a program `Unsafe`, and it reports no
enforced limits because nothing was spawned.

## 5. Tests

| Where | What |
|---|---|
| `crates/qcore-verifier-host/src/executable_identity.rs` (7 unit tests) | round trip and repeatability; in-place rewrite; stripped envelope; unlink-and-recreate; removal; a directory in place of the executable; opening a directory as an executable |
| `crates/qcore-verifier-host/tests/package_identity.rs` (10 integration tests) | the refusal matrix rows A–E and G–I end to end, each asserting the ran-marker is absent; the registry row; the window measurement; the unverified loader's absent identity and its verifying bridge; `apply_unverified` |
| `crates/qcore-verifier-host/src/lib.rs` (2 `compile_fail` doctests) | rows L and M |
| `providers/conformance/tests/packages.rs` | row F on the five real shipped executables, with the published per-package window and re-check measurements; the digest-intact manifest mutations now assert that `verify_executable_package` refuses *and* drive the package's own self-check through `UnverifiedProvider::load` |
| `providers/conformance/tests/composed_five_providers.rs` | the four deliberately-mutated registrations move to the explicit `DeliberatelyUnverified` wrapper |
| `compiler/tests/cycle20_verifier_plugins.rs` | `package_discovery_verifies_before_it_registers` pins both halves of row J; two registry fixtures install real verifiable packages |
| `crates/qcore-verifier-host/tests/{enforced_process_limits,confined_process_execution,execution_quota}.rs` | the four registration sites install verifiable packages; every other fixture moves to `UnverifiedProvider` with its assertions unchanged |

## 6. Lean

`proof/QCore/PackageIdentity.lean`, audited by
`proof/AuditPackageIdentity.lean` — **24 theorems**, no new axiom (every
declaration depends only on `propext`, `Quot.sound` and, for the one
contradiction argument, `Classical.choice`, all already allowed by
`AuditAll.lean`).

The judgment on whether Lean carries content here: it does, because the claim is
about a *state machine* and a *decision procedure with two independent
questions*, and both have directions worth proving.

* **What acceptance means.** `revalidate_ok_iff` (acceptance is exactly the
  conjunction), `ok_loads_the_rechecked_content` and
  `ok_loads_content_of_the_verified_digest` (acceptance is not merely "nothing
  looked wrong": the loaded content is the re-checked content), and
  `ok_loads_the_verified_bytes` under an explicit injectivity hypothesis —
  SHA-256's collision resistance is assumed *by name*, never proved.
* **The refusals, one per substitution class.**
  `an_in_place_rewrite_is_refused`, `a_replacement_is_refused`,
  `a_byte_identical_replacement_is_refused` (which also computes what the path
  would have loaded, so the case is not vacuous),
  `a_removed_executable_is_refused`, `no_substitution_is_admitted`, and
  `both_refusals_are_reachable` — neither branch is dead.
* **The state machine.** `never_dispatched_without_an_accepted_verification`:
  over *any* event sequence containing no accepted verification, a handle that
  started unverified never dispatches; `a_dispatch_requires_an_accepted_verification`
  is its contrapositive over traces. With `dispatch_predecessor_is_verified`,
  `a_refusal_never_dispatches`, `dispatch_carries_its_verification` and
  `dispatch_iff_ok`, that is the modelled form of "no path from Unverified to
  Dispatched".
* **Non-vacuity and the frame.** `an_unchanged_package_dispatches` (the honest
  package runs), `revalidate_depends_only_on_the_two_observations` (nothing else
  about the filesystem moves the answer — and, read the other way, no further
  observation is available to narrow the window).
* **The residual, proved open.** `a_post_check_substitution_is_not_refused`
  exhibits, for any verified handle, a world that passes the re-check and a
  later world whose load differs. The model states the gap rather than papering
  over it.

Deliberate reductions, stated in the module header: bytes are one natural number
per file and the digest is an arbitrary parameter; the embedded-template binding
is `QCore.PackageTemplateBinding`'s subject, not re-modelled here; and the model
says nothing about `execve` semantics beyond "the path's content is what loads".

## 7. Gate results

| Gate | Result |
|---|---|
| `cargo test --workspace` | GATE_WORKSPACE |
| `cargo fmt --all -- --check` | GATE_FMT |
| `cargo clippy --all-targets -- -D warnings` | GATE_CLIPPY |
| `cargo run -p qcore-compiler --example reevaluation_probe` | GATE_PROBE1 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | GATE_PROBE2 |
| `python3 scripts/check_lean_audits.py` | **49 Lean audits passed** (48 before; `AuditPackageIdentity.lean` is the new one) |
| `cargo test --manifest-path providers/Cargo.toml --workspace --offline` | GATE_PROVIDERS |
| `cargo fmt --manifest-path providers/Cargo.toml --all -- --check` | GATE_PROV_FMT |
| `cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --offline -- -D warnings` | GATE_PROV_CLIPPY |
| `python3 -B providers/package.py build` + `verify` | GATE_PACKAGES |
| `python3 -m unittest discover -s scripts/tests` | GATE_PYTHON |

### 7.1 One derived number the card may not update

`scripts/tests/test_build_matrix_sections.py::test_the_derived_facts_are_recomputed_rather_than_carried`
fails, and is **expected to**: it compares the committed
`docs/reviews/r0_3/acceptance_matrix.json` against the live sources, and this
card adds `proof/AuditPackageIdentity.lean`, so the derived
`validation.lean_audits` moves 48 -> 49 and `validation.lean_audit_files`
gains the new name. The matrix is integrator-owned and this card is instructed
not to edit it, so the failure is left standing and handed over. It clears with
the regeneration the wave-9 generator already supports:

```sh
python3 -B docs/reports/remaining-d02/build_matrix.py . --gate GATE.json
```

No other Python test fails (124 of 125 pass), and nothing about the assertion
was weakened to make it green.

## 8. Compatibility and deliberate decisions

* **Public API break.** `ProcessProvider::new` is gone; `ProcessProvider` is
  produced by verification alone. `qcore-verifier-host` moves 0.2.0 → **0.3.0**
  and `qcore-compiler` 0.3.0 → **0.4.0** (it re-exports the type and now also
  re-exports `UnverifiedProvider`). Callers that were constructing a provider
  from a manifest they had in hand move to `UnverifiedProvider::new`, and get a
  compile error rather than a silently unverified registration if they were
  registering it.
* **`ProcessExecutionRecord` gains `identity_window_us`**, `Option<u64>`,
  `skip_serializing_if = "Option::is_none"`, so records written before this
  field serialize byte-identically. `ProcessOutcome` gains
  `RefusedByIdentity`; no existing variant changes meaning.
* **No `qcore-schemas` change**, no manifest schema change, no wire version
  change, no package rebuild required by this card's semantics (the packages
  are rebuilt in the gate because the harness builds them).
* **Behaviour that got stricter, deliberately**: `discover_packages` and the
  compiler's first-party executable arm now verify. Both were fail-open routes;
  both now fail closed, and both directions are pinned by tests.
* **No fail-closed gate was weakened.** The one test whose expectation moved
  (`package_discovery_loads_a_versioned_manifest`) moved because the behaviour
  it pinned was the defect; it is replaced by a test that pins the refusal *and*
  the acceptance.

## 9. Open boundaries

* **`P01-F3-FDEXEC` — the last few milliseconds.** Executing the held file
  description would close the residual measured in §3.2. It is refused here on
  the two grounds in §3.3, one of them measured. Reopening it means either a
  hand-written `fork`/`exec` path or admitting `/proc` to the confined child.
* **Forged or rebuilt executables remain undetected.** Restated because it is
  the boundary most easily misread: this card ties *which file runs* to *the
  file that was checked*. It says nothing about who built that file. Signature
  verification and a trusted build record remain absent by design; the
  operational rule in `providers/README.md` still carries that weight.
* **The re-check is a snapshot, not a lock.** No `O_EXLOCK`, no immutable
  attribute, no read-only bind mount. A package directory a writer controls is
  still a package directory a writer controls.
* **The template re-bind at spawn is unreachable behind the digest comparison**
  (§3.4), and is recorded as such rather than counted as coverage.
* **The explicit delegation escape** described in §2.2 is a discouragement, not
  a boundary: a caller who writes the newtype can register an unverified
  package. Making that impossible would require sealing `VerifierProvider`,
  which would also lock out the legitimate in-process providers.
* **Cost**: one SHA-256 over the executable per dispatch. Every configured-audit
  route that dispatches many requests to one provider pays it once per request;
  the measured per-dispatch figures are in §3.5 and §7.
* Nothing here promotes a verdict, changes CT coverage (30/50 mapped), the
  contract corpus (60 modules) or the pinned 185-unit artifacts.
