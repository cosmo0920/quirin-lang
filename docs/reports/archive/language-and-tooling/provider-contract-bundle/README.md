# Provider contract bundle packaging handoff

Date: 2026-09-21. Base: `10aafcf8b1c17f11ee49d6cb909fca4a6b9ecee1`.
Provider-owned contract source commit in this worktree: `d717db9a` (the
integrator owns the corresponding canonical-source commit). Tested
implementation commit: `5d71e8fb`.

This is a bounded implementation handoff. It does not change the provider
protocol, plugin manifest schema, core host, source-contract selection, or any
verdict.

## Result

All five first-party provider binaries embed their crate's `contracts.json`
inside one distinct `qcore.provider-contracts.v1` envelope. Package construction
runs `providers/contracts.py check` before Cargo, then copies the inventory and
its declared `contracts/**/*.qcore` files into the installed package and
deterministic archive. `packages.json` records each installed contract
inventory digest.

`providers/package.py verify` accepts a bundled provider only when the
installed and embedded inventories agree semantically and the installed
recursive asset set is exactly the declared set. Inventory paths must be
canonical contained POSIX paths below `contracts/`; every declared SHA-256 must
match; missing, tampered, extra, colliding, malformed and symbolic-link assets
refuse. An asset and its installed digest cannot be changed together to evade
the check because the unchanged executable inventory remains the comparison
authority. Directory traversal raises rather than silently skipping an
unreadable subtree.

Old and test-only providers that carry neither an embedded inventory nor
installed contract files keep their previous behavior. The shared runtime's
old entry points remain, while bundled providers use the additive wrappers and
may print the validated embedded inventory with `--contract-manifest`.

The existing CMetrics process conformance test now reads
`contracts/cmetrics/counter_reset.qcore` from the built package. Its Safe result,
certificate, two checked states and hostile-proposal refusals are unchanged.

## Trust boundary

This change gives the standalone packaging tool a contract-asset identity
check. The core host still verifies the executable digest, installed plugin
manifest, path containment and executable-embedded plugin manifest. It does
not acquire contract-bundle verification in this card. Configured audits still
select their contracts explicitly and replay evidence in the trusted receiver;
shipping a contract with a provider grants no authority and proves no contract
premise.

## Focused controls and package evidence

The five Python controls cover the exact recursive bundle, legacy absence,
canonical path containment, missing/tampered/extra/symlink assets, simultaneous
asset-plus-installed-hash tampering, and recursive deterministic archiving.
The first run exited 1 because its assertion expected a different deterministic
member order; the implementation order was stable. The expected order was
corrected and the rerun passed 5/5. Both logs are retained.

Two complete package builds exited 0 using the same reused Cargo target. All
five corresponding `.tar.gz` archives compared byte-for-byte equal (`cmp` exit
0 for each). This is repeated deterministic packaging with a reused compilation
cache, not an independent fresh-target recompilation. A CMetrics archive was
then extracted into a new directory and `package.py verify` exited 0 against
the extracted standalone package.

## Validation

Private logs are under
`private/provider-contract-packaging/` in the worktree; large build outputs and
the disk-backed Cargo target are under
`/home/cosmo/Documents/GitHub/qcore-c/private/provider-contract-packaging/`.

| Check | Exit / result | Log |
|---|---:|---|
| Fresh-worktree submodule initialization | 0 | tool transcript |
| `python3 providers/contracts.py check` | 0; 5 providers / 72 contracts | `contracts-check.log` |
| `python3 -m unittest discover -s providers/tests -v` initial | 1; one member-order assertion | `python-tests.log` |
| Same focused Python suite, corrected and final | 0; 5 passed | `python-tests-rerun.log`, `python-tests-final.log` |
| `python3 providers/package.py build` run 1 | 0 | `package-build.log` |
| Same build run 2, reused target | 0 | `package-build-run2.log` |
| Five run-1/run-2 archive comparisons | 0 each | hashes in `archive-sha256.log`; command exits in transcript |
| Extracted CMetrics `package.py verify` | 0 | `extracted-verify.log` |
| CMetrics package-sourced semantic conformance | 0; 1 passed | `cmetrics-unit.log` |
| Provider workspace, managed sandbox | 101; 121 passed / 1 failed | `provider-workspace-test.log` |
| Provider workspace, native-host rerun | 0; 122 passed / 0 failed / 0 ignored | `provider-workspace-test-escalated.log` |
| `real_packages_preserve_unknown_and_reject_mutations`, isolated native-host | 0; 1 passed | `package-mutation-isolated.log` |
| Provider workspace `cargo check --locked --offline` | 0 | `check.log` |
| Provider workspace `cargo fmt --all -- --check` | 0 | `fmt-final.log` |
| Provider workspace Clippy, all targets, warnings denied | 0 | `clippy.log` |
| `git diff --check` before implementation commit | 0 | tool transcript |

The managed-sandbox full-suite failure is retained rather than reclassified:
`real_packages_answer_identically_under_the_confined_profile` reached the
existing Linux confinement path and the sandbox denied
`unshare(CLONE_NEWUSER | CLONE_NEWNET)` with `EPERM`. The complete native-host
rerun passed that row and all 122 provider tests. The load-sensitive real
package mutation row also passed separately with one test thread.

## Integrator reconciliation list

1. Merge implementation commit `5d71e8fb` after the canonical provider-owned
   contract commit; retain the integrator's stricter final `contracts.py` where
   the histories differ.
2. Record the provider workspace as 122 passed / 0 failed / 0 ignored only from
   the native-host run. Preserve the sandbox `EPERM` run as excluded platform
   evidence and the initial Python assertion failure as corrected test work.
3. Make no status, coverage, conformance, protocol, manifest-schema, core-host,
   acceptance-matrix or follow-up retirement change from this lane.
4. Keep the published boundary explicit: `package.py` verifies shipped contract
   assets; the unchanged host verifies executable/plugin-manifest identity;
   configured audits keep explicit contract selection and trusted replay.
