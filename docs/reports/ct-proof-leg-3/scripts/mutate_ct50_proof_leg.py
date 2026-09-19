#!/usr/bin/env python3
"""Non-vacuity harness for card `CT-PROOF-LEG-3`.

Three tiers, kept apart on purpose.

* **Fixture mutations (F)** — CT50's own `case.qcore` is mutated and the place
  the pipeline catches it is *measured*, so a mutation the **checker** refuses
  is never presented as evidence about the leg (`CT07-TWO-THREAD-PROOF-LEG`'s
  method). The rows that matter are the ones the checker **admits** and the
  binding catches.
* **Rule mutations (R)** — one clause of the Rust transcription, or one Lean
  term, is mutated in place and the named test is watched go red.
* **Digest mutations (D)** — one digit of a pinned digest is changed, on both
  sides and on a leg this card did not write, and
  `proof_leg_digests.rs` is watched go red. This is the control
  `PROOF-LEG-DIGESTS-UNCROSSCHECKED` measured missing.

Every mutation is restored by **rewriting the file's bytes** (never by an
mtime-preserving copy — `MUTATION-HARNESS-STALE-ARTIFACTS`), and every restore
is verified by sha256 against the pre-mutation digest. The baseline is proved
green before the first mutation and again after the last.

Run from the repository root:

    python3 docs/reports/ct-proof-leg-3/scripts/mutate_ct50_proof_leg.py
"""

import hashlib
import os
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]

LEG = ROOT / "compiler/tests/ct50_proof_leg.rs"
DIGESTS = ROOT / "compiler/tests/proof_leg_digests.rs"
FIXTURE = ROOT / "compiler/tests/fixtures/ct/CT50/case.qcore"
LEAN = ROOT / "proof/QCore/Ct50LifetimeIdentity.lean"
CT12 = ROOT / "compiler/tests/ct12_ct14_proof_reach.rs"

EXECUTOR = "ct50_artifact_is_the_lean_proved_lifetime_identity_shape"


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def cargo(test_binary, filter_name=None):
    command = ["cargo", "test", "-p", "qcore-compiler", "--test", test_binary]
    if filter_name:
        command += ["--", filter_name]
    finished = subprocess.run(
        command, cwd=ROOT, capture_output=True, text=True, check=False
    )
    return finished.returncode, finished.stdout + finished.stderr


def lake():
    environment = dict(os.environ)
    environment["PATH"] = os.path.expanduser("~/.elan/bin") + os.pathsep + environment["PATH"]
    finished = subprocess.run(
        ["lake", "build"],
        cwd=ROOT / "proof",
        capture_output=True,
        text=True,
        check=False,
        env=environment,
    )
    return finished.returncode, finished.stdout + finished.stderr


def panic_site(output):
    """The first `file:line:col` a panic names, which is where the run stopped."""
    for line in output.splitlines():
        if "panicked at " in line:
            return line.split("panicked at ", 1)[1].strip().rstrip(":")
    for line in output.splitlines():
        if line.strip().startswith("error"):
            return line.strip()
    return "(no panic line)"


def first_assertion(output):
    """The assertion or message text just after the panic line."""
    lines = output.splitlines()
    for index, line in enumerate(lines):
        if "panicked at " in line and index + 1 < len(lines):
            return lines[index + 1].strip()[:200]
    return ""


class Mutation:
    def __init__(self, tag, path, old, new, expectation, runner, expect_red=True):
        self.tag = tag
        self.path = path
        self.old = old
        self.new = new
        self.expectation = expectation
        self.runner = runner
        # A row may be *expected* green, where that is itself the measurement
        # (R5). The log labels it as such rather than as a surprise.
        self.expect_red = expect_red


def apply(path, old, new):
    text = path.read_text()
    if text.count(old) != 1:
        raise SystemExit(
            "%s: the mutation site occurs %d times, not once" % (path, text.count(old))
        )
    path.write_text(text.replace(old, new))


def restore(path, text):
    # Rewrite the bytes. Never `cp -p`, never a touch that preserves mtime:
    # a restored file whose mtime predates the mutated build leaves cargo and
    # lake serving stale artifacts (`MUTATION-HARNESS-STALE-ARTIFACTS`).
    path.write_text(text)
    os.utime(path, None)


def run_one(mutation, log):
    original = mutation.path.read_text()
    before = sha256(mutation.path)
    apply(mutation.path, mutation.old, mutation.new)
    code, output = mutation.runner()
    restore(mutation.path, original)
    after = sha256(mutation.path)
    log.append(
        "%s\t%s\t%s\texit=%d\t%s\t%s\trestore=%s"
        % (
            mutation.tag,
            mutation.path.relative_to(ROOT),
            mutation.expectation,
            code,
            ("RED" if mutation.expect_red else "RED (UNEXPECTED)")
            if code != 0
            else ("GREEN (UNEXPECTED)" if mutation.expect_red else "GREEN (as measured)"),
            panic_site(output) + (" | " + first_assertion(output) if code else ""),
            "sha256 EQUAL" if before == after else "MISMATCH",
        )
    )
    return code != 0 and before == after


def main():
    log = []
    failures = []

    def baseline(label):
        ok = True
        for binary in ("ct50_proof_leg", "proof_leg_digests"):
            code, _ = cargo(binary)
            log.append("BASELINE-%s\t%s\texit=%d" % (label, binary, code))
            ok = ok and code == 0
        code, _ = lake()
        log.append("BASELINE-%s\tlake build\texit=%d" % (label, code))
        return ok and code == 0

    if not baseline("BEFORE"):
        raise SystemExit("the baseline is not green; nothing measured")

    leg_runner = lambda: cargo("ct50_proof_leg")
    digest_runner = lambda: cargo("proof_leg_digests")

    mutations = [
        # ---- F: CT50's own fixture. The catch point is MEASURED, not assumed.
        Mutation(
            "F1-leaked-first-allocation",
            FIXTURE,
            "    drop old;\n",
            "",
            "expected: the CHECKER, before any artifact exists",
            leg_runner,
        ),
        Mutation(
            "F2-stale-read-after-reallocation",
            FIXTURE,
            "let answer = qcore_buffer_read(fresh);",
            "let answer = qcore_buffer_read(old);",
            "expected: the CHECKER, before any artifact exists",
            leg_runner,
        ),
        Mutation(
            "F3-different-fresh-payload",
            FIXTURE,
            "qcore_buffer_new(42)?",
            "qcore_buffer_new(43)?",
            "expected: ADMITTED by the checker, caught by the BINDING",
            leg_runner,
        ),
        Mutation(
            "F4-no-first-lifetime",
            FIXTURE,
            "    let old = qcore_buffer_new(7)?;\n    drop old;\n",
            "",
            "expected: ADMITTED by the checker, caught by the BINDING",
            leg_runner,
        ),
        # ---- R: the transcription's clauses, one at a time.
        Mutation(
            "R1-drop-the-stale-use-clause",
            LEG,
            "        && count(trace, is_stale_use) == 0\n",
            "",
            "expected: the escaping-capability mutant becomes accepted",
            leg_runner,
        ),
        Mutation(
            "R2-drop-the-old-lifetime-chain",
            LEG,
            """        && is_sublist(
            &[stale_allocation(), stale_release(), fresh_allocation()],
            trace,
        )
""",
            "",
            "expected: the overlapping-lifetimes mutant becomes accepted",
            leg_runner,
        ),
        Mutation(
            "R3-drop-the-read-window-chain",
            LEG,
            "        && is_sublist(&[fresh_allocation(), fresh_read(), fresh_release()], trace)\n",
            "",
            "expected: the read-after-release mutant becomes accepted",
            leg_runner,
        ),
        Mutation(
            "R4-drop-the-single-actor-clause",
            LEG,
            "        && projection(trace, 0) == trace\n",
            "",
            "expected: the second-actor mutant becomes accepted",
            leg_runner,
        ),
        Mutation(
            "R5-weaken-the-payload-clause",
            LEG,
            "        && count(trace, |event| reads_back(FRESH, 42, event)) == 1\n",
            "        && count(trace, is_read) == 1\n",
            "expected GREEN, and PROVED so: the payload clause is implied by the "
            "read-window chain and the read count "
            "(the_payload_clause_is_implied_by_the_read_window_and_the_read_count), "
            "so no trace separates them and no mutation of it can redden anything",
            leg_runner,
            expect_red=False,
        ),
        Mutation(
            "R6-rename-the-stale-generation",
            LEG,
            "const STALE: u64 = 1;",
            "const STALE: u64 = 3;",
            "expected: the artifact no longer matches the transcribed terms",
            leg_runner,
        ),
        Mutation(
            "R7-starve-the-enumeration-bound",
            LEG,
            "admitted_linearizations(&order, 100_000)",
            "admitted_linearizations(&order, 1)",
            "expected: the enumeration is really performed, so a bound it cannot "
            "meet stops the executor",
            leg_runner,
        ),
        Mutation(
            "R8-change-the-expected-admitted-set",
            LEG,
            "    assert_eq!(admitted, vec![reference_run()]);",
            "    assert_eq!(admitted, vec![reference_run(), reference_run()]);",
            "expected: the admitted set is really compared, not merely produced",
            leg_runner,
        ),
        # ---- L: the Lean terms. Checked by `lake build` under
        # `warningAsError`; a failed `decide` is a build error.
        Mutation(
            "L1-lean-digest-plus-one",
            LEAN,
            "mixTrace referenceRun = 1699395409980170611",
            "mixTrace referenceRun = 1699395409980170612",
            "expected: the Lean kernel refuses the digest theorem",
            lake,
        ),
        Mutation(
            "L2-lean-fresh-read-payload",
            LEAN,
            "def freshRead : Event := .read 0 fresh 42",
            "def freshRead : Event := .read 0 fresh 41",
            "expected: the Lean reference-run theorems stop deciding",
            lake,
        ),
        # Measured separately with the whole `lake` output kept
        # (`measure/l3_probe.py`): this mutation reddens
        # `an_escaping_stale_capability_is_refused` (the refusal), the
        # `countP isStaleUse = 1` conjunct of its isolation theorem, and the
        # derived `discharged_the_old_generation_is_never_read`. The log line
        # below shows the FIRST error Lean reports, which is the derived
        # theorem; the report names all three.
        Mutation(
            "L3-lean-blind-the-stale-use-predicate",
            LEAN,
            "  mentions stale event && !isAllocationOf stale event",
            "  mentions 99 event && !isAllocationOf stale event",
            "expected: the Lean escaping-capability refusal stops deciding, at "
            "the clause rather than by a structural break",
            lake,
        ),
        # ---- D: the digest cross-check this card landed.
        Mutation(
            "D1-rust-ct50-digest-one-digit",
            LEG,
            "const CT50_LEAN_TRACE_DIGEST: u64 = 1699395409980170611;",
            "const CT50_LEAN_TRACE_DIGEST: u64 = 1699395409980170612;",
            "expected: the cross-check reddens on the Rust side",
            digest_runner,
        ),
        Mutation(
            "D2-lean-ct50-digest-one-digit",
            LEAN,
            "mixTrace referenceRun = 1699395409980170611",
            "mixTrace referenceRun = 1699395409980170612",
            "expected: the cross-check reddens on the LEAN side, with no lake run",
            digest_runner,
        ),
        Mutation(
            "D3-rust-ct01-second-site-one-digit",
            CT12,
            "const CT01_LEAN_TRACE_DIGEST: u64 = 13023433800416048247;",
            "const CT01_LEAN_TRACE_DIGEST: u64 = 13023433800416048248;",
            "expected: the cross-check covers the EXISTING legs, including the "
            "second CT01 site the residual's own inventory missed",
            digest_runner,
        ),
        Mutation(
            "D4-drop-a-lean-digest-theorem",
            LEAN,
            "theorem ct50_trace_digest : mixTrace referenceRun = 1699395409980170611 := by\n  decide\n",
            "",
            "expected: a Lean pin removed is refused, not silently ignored",
            digest_runner,
        ),
    ]

    for mutation in mutations:
        red = run_one(mutation, log)
        # R5 is deliberately expected GREEN, and the reason is a theorem, not
        # a gap: the clause it weakens is implied by the two clauses beside it,
        # so no trace can separate them. Recorded rather than papered over.
        if mutation.tag.startswith("R5"):
            if red:
                failures.append(mutation.tag + " (reddened, but was expected green)")
            continue
        if not red:
            failures.append(mutation.tag)

    if not baseline("AFTER"):
        raise SystemExit("the baseline is not green after the restores")

    text = "\n".join(log) + "\n"
    (ROOT / "docs/reports/ct-proof-leg-3/mutations.log").write_text(text)
    print(text)
    if failures:
        print("MUTATIONS THAT DID NOT REDDEN:", failures)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
