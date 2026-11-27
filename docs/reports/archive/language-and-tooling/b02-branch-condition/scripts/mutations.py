#!/usr/bin/env python3
"""B02-BRANCH-CONDITION mutation campaign.

Each mutation negates one load-bearing arm, runs the named end-to-end test,
rewrites the original bytes back (advancing mtime), and asserts a byte-identical
round trip.  Run from the repository root with `CARGO_TARGET_DIR` set outside a
tmpfs:

    python3 docs/reports/b02-branch-condition/scripts/mutations.py LOG
"""

import hashlib
import os
import pathlib
import subprocess
import sys
import time

ROOT = pathlib.Path.cwd()
LEGACY = ROOT / "crates/qcore-c-gateway/src/legacy_c.rs"
BOUNDARY = ROOT / "crates/qcore-c-gateway/src/legacy_condition_call.rs"


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def cargo(test):
    if test.startswith("legacy_condition_call::"):
        command = ["cargo", "test", "-p", "qcore-c-gateway", "--lib", test, "--", "--exact"]
    else:
        command = [
            "cargo", "test", "-p", "qcore-c-gateway", "--test",
            "branch_condition_calls", test, "--", "--exact",
        ]
    done = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
    output = done.stdout + done.stderr
    failed = [
        line[len("test ") : -len(" ... FAILED")]
        for line in output.splitlines()
        if line.startswith("test ") and line.endswith(" ... FAILED")
    ]
    compile_error = "error: could not compile" in output or "error[E" in output
    return done.returncode, failed, compile_error


MUTATIONS = [
    (
        "M1 call transfer omitted before the branch",
        LEGACY,
        """                condition_call.as_ref().map_or(branch, |call| {
                    self.push((**call).clone(), vec![branch], *line, false)
                })""",
        "                branch",
        "an_unconditional_release_runs_before_the_branch",
    ),
    (
        "M2 short-circuit condition treated as unconditional",
        BOUNDARY,
        '["&", "&"] | ["|", "|"] | ["+", "+"] | ["-", "-"]',
        '["__qcore_never", "__qcore_never"] | ["+", "+"] | ["-", "-"]',
        "a_short_circuit_release_stays_refused_and_cannot_hide_the_live_resource",
    ),
    (
        "M3 call under a type operator treated as evaluated",
        BOUNDARY,
        """        if crate::legacy_c::TYPE_OPERATORS.contains(&pair[0]) {
            // Refuse the whole expression, rather than risk selecting a call
            // whose evaluation the type operator suppresses.
            return None;
        }""",
        """        if crate::legacy_c::TYPE_OPERATORS.contains(&pair[0]) {
            continue;
        }""",
        "legacy_condition_call::tests::refuses_conditional_or_out_of_scope_evaluation",
    ),
    (
        "M4 residual expression skipped instead of checked",
        LEGACY,
        "if !self.supported_expression(&residual, ExpressionRole::Decides) {",
        "if false && !self.supported_expression(&residual, ExpressionRole::Decides) {",
        "an_indirect_residual_cannot_be_masked_by_the_modeled_call",
    ),
    (
        "M5 any isolated instruction accepted instead of ContractCall only",
        LEGACY,
        "if matches!(instruction, LegacyInstruction::ContractCall { .. }) {",
        "if true || matches!(instruction, LegacyInstruction::ContractCall { .. }) {",
        "an_uncontracted_call_stays_refused",
    ),
    (
        "M6 unknown Boolean result loses the else successor",
        LEGACY,
        "let branch = self.push(instruction, vec![then_entry, else_entry], *line, false);",
        "let branch = self.push(instruction, vec![then_entry], *line, false);",
        "the_unknown_call_result_keeps_both_branch_arms",
    ),
]


def main():
    if len(sys.argv) != 2:
        raise SystemExit(__doc__)
    if "CARGO_TARGET_DIR" not in os.environ:
        raise SystemExit("set CARGO_TARGET_DIR outside the worktree tmpfs")
    log_path = ROOT / sys.argv[1]
    originals = {path: path.read_bytes() for path in {LEGACY, BOUNDARY}}
    results = []
    with log_path.open("w", encoding="utf-8") as log:
        def emit(line):
            print(line, file=log, flush=True)
            print(line, flush=True)

        emit(f"# B02 mutations {time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())}")
        emit(f"# HEAD {subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip()}")
        code, failed, compile_error = cargo("an_unconditional_release_runs_before_the_branch")
        baseline = code == 0 and not compile_error
        emit(f"M0 baseline: exit={code}; {'GREEN' if baseline else 'RED'}; failures={failed or '-'}")
        results.append(baseline)
        try:
            for label, path, old, new, test in MUTATIONS:
                source = originals[path].decode()
                count = source.count(old)
                if count != 1:
                    emit(f"{label}: HARNESS ERROR anchor count={count}")
                    results.append(False)
                    continue
                before = sha(path)
                path.write_text(source.replace(old, new), encoding="utf-8")
                try:
                    code, failed, compile_error = cargo(test)
                finally:
                    path.write_bytes(originals[path])
                    # Rewriting after byte restoration advances mtime, avoiding
                    # a stale Cargo fingerprint while preserving exact bytes.
                    path.write_text(originals[path].decode(), encoding="utf-8")
                restored = sha(path) == before
                caught = code != 0 and test in failed and not compile_error and restored
                results.append(caught)
                emit(
                    f"{label}: exit={code}; {'RED as required' if caught else 'NOT AS REQUIRED'}; "
                    f"named_test={'yes' if test in failed else 'no'}; compile_error={compile_error}; "
                    f"restored_sha256_equal={restored}; failures={failed or '-'}"
                )
        finally:
            for path, original in originals.items():
                path.write_text(original.decode(), encoding="utf-8")
                assert path.read_bytes() == original
        code, failed, compile_error = cargo("an_unconditional_release_runs_before_the_branch")
        restored_green = code == 0 and not compile_error
        results.append(restored_green)
        emit(f"M7 restored baseline: exit={code}; {'GREEN' if restored_green else 'RED'}; failures={failed or '-'}")
        emit(f"# {sum(results)} of {len(results)} verdicts as required")
    return 0 if all(results) else 1


if __name__ == "__main__":
    sys.exit(main())
