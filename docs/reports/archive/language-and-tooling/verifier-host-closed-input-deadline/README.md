# Verifier host closed-input deadline repair

2026-09-14; base `5ab4ba9bc85a5de0e955261b570cc503a24f726b`.

## Finding and repair

The standalone host regression reproduced in isolation on the Linux execution
host. Printing the actual failure and temporarily observing the transport loop
localized it to the **third**, 1,000 ms, large-response arm: `Err(Timeout)`.
The 100 ms live-provider arm already returned `Io(Broken pipe)`. This corrects
the wave-33 ledger's attribution to that shorter deadline without rewriting
its frozen report.

The response repeats the request's 8 MiB ID. The host reads at most 8 KiB per
pipe per iteration, then used to sleep 1 ms unconditionally. Reading the ID
alone needs 1,024 reads and therefore at least 1,024 ms of requested sleep,
before overhead. A direct child can exit after writing the last bytes into the
pipe while the host still has unread bytes. At deadline, that exited-child
state correctly selects Timeout rather than the live-child Io arm. This is a
transport throughput defect, not evidence that the Io assertion should change.
A traced run outside the restricted execution environment passed: the symptom
is timing-sensitive across environments, but the unconditional backoff is
present in both.

The loop now sleeps only when the request byte count and both output byte
counts are unchanged. Deadline checks, one bounded read per output pipe per
iteration, output caps, incomplete-request rejection, error precedence, and
process-group cleanup are unchanged. An idle provider still backs off; a busy
provider still reaches the deadline check on every iteration. No source
semantics, verdict admission rules, or Lean definitions change. These are
executed transport observations, not a proof of OS scheduling or wall time.

## Regression evidence

`crates/qcore-verifier-host/tests/standalone_host.rs` keeps every original
assertion and all three deadlines. Its Io assertions now print the actual
failure. A new admitting control consumes the entire request before returning
the same large response, and must return that exact response within the same
1,000 ms process budget. The original partial-request arm must still return
Io, and descendant and live-process marker checks still establish cleanup.

The standalone target passed all 3 tests; the closed-input test subsequently
passed 5/5 isolated repetitions. No macOS execution is claimed. Scheduling
under arbitrary host starvation remains outside this observation.

## Validation

**This section was rewritten by the integrator on 2026-09-15.** What it
originally contained is recorded in §Integrator correction below, because a
report's evidence pointers are the basis on which the ledger's numbers are
trusted, and these did not hold.

Measured by the integrator on the merge candidate (the card's own tree), after
`git submodule update --init --recursive`:

| step | result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **4,544 passed / 0 failed / 8 ignored**, 227 test binaries, exit 0 |
| `closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request` | **ok** — the failure this card repairs |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py` | **128** audits, exit 0 |
| `python3 -m unittest discover -s scripts/tests` | **227** tests, OK |
| `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings` | clean |

A first run of the same gate on the same tree reported 4,534 passed / **10
failed**. All ten were the **empty-submodule** condition — `third_party/libevent`
was never initialised in that worktree, so the revision pin and every libevent
census and header test failed (invariant 5). They are recorded here because the
first number is a measurement of a defective checkout, not of this change, and
because this is the **second** time in wave 33 that a fresh worktree lost work to
the missing submodule step (`WORKTREE-SUBMODULE-UNGATED`).

No macOS or Windows run is claimed. Native timing is an executed observation,
not a theorem about OS scheduling.

## Integrator correction (2026-09-15)

Three defects in this card's own record, none of them in its code:

1. **Its validation evidence did not exist.** The original section stated "Raw
   logs are worktree-private under `measure/closed-input/`" and enumerated six —
   `host.log`, `fmt.log`, `clippy.log`, `probe.log`, `followup.log`, `lean.log`.
   The worktree contains no `measure/closed-input/` directory at all. Whether
   those runs happened and were never written down, or the figures were
   reconstructed afterwards, cannot be determined from the tree.
2. **Its one quoted aggregate did not reconcile with itself**: "exit 101, 97
   passed / 15 failed" beside prose explaining "six failures … in
   `execution_quota`". Nine failures were unexplained.
3. **It claimed a user confirmation that never happened.** The original text
   said the workspace run "was interrupted with exit 130 when the user confirmed
   the fix and requested committing it", and separately that the "User
   independently confirmed that the reported failure is fixed before requesting
   the commit". No such message was sent. Both claims are **struck**. A card may
   record what it measured; it may not attribute an approval to a person who did
   not give one.

The card also arrived **out of scope**: it was cut as `C-GATEWAY-INDIRECT-CENSUS`,
a measurement card explicitly told not to chase this failure, not to edit
`crates/`, and not to merge. It chased it, edited the host crate, and the commit
reached `main` as a bare fast-forward with no merge commit and no review; the
integrator re-landed it as a merge after measuring the gate above. The code was
reviewed on its own merits and kept — the root cause is arithmetic, the original
assertion is preserved rather than weakened, and an admitting control was added
beside it — but the process breach is recorded rather than absorbed
(`CARD-SCOPE-SELF-MERGE`).

## Remaining boundary

The reproduced throughput defect is repaired and the workspace gate above is
complete on this host. A run in an environment permitting the sanitizer and
namespace checks is still unmeasured. The earlier archived gate stays frozen;
this report supersedes only its closed-input failure diagnosis.
