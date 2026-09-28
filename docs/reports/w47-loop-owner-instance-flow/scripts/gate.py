#!/usr/bin/env python3
"""W47-LOOP-OWNER-INSTANCE-FLOW: the standing gate and the provider gate.

Usage: gate.py LOG_DIR rust|records

Every step's output goes to LOG_DIR/NAME.log and its exit to LOG_DIR/NAME.exit
(never piped). providers/ changed (fluent_bit/input_instance.qv and its lock),
so the provider gate is due: provider cargo test, fmt and clippy,
contracts.py check, package build and verify of every package, the provider
Python suite (which pins the resource-view module count), and pin --check at
the three audited trees plus pin --verify-contracts.

  rust:    submodule, fmt, Rust suite, clean, clippy, both probes, Lean
           audits, clean; provider tests, fmt, clippy, contracts check,
           package build and verifies, provider Python, pins, provider clean
  records: Python suite and docs-index check (after the matrix and index
           regeneration), and the Clang recording
"""

import os
import pathlib
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
PACKAGES = pathlib.Path(
    "/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-implementation-"
    "wave-47-681db5/c8ef992e-a75f-4250-b4e1-07e6e5477601/scratchpad/if-packages")
PROVIDER = ["--manifest-path", "providers/Cargo.toml"]
STEPS = {
    "rust": [
        ("01-submodule", ["git", "submodule", "update", "--init", "--recursive"]),
        ("02-fmt", ["cargo", "fmt", "--all", "--", "--check"]),
        ("03-tests", ["python3", "scripts/run_test_binaries.py"]),
        ("04-clean", ["cargo", "clean", "--profile", "dev"]),
        ("05-clippy", ["cargo", "clippy", "--all-targets", "--", "-D", "warnings"]),
        ("06-probe", ["cargo", "run", "-p", "qcore-compiler", "--example", "reevaluation_probe"]),
        ("07-followup", ["cargo", "run", "-p", "qcore-compiler", "--example",
                         "reevaluation_followup"]),
        ("08-lean", ["python3", "scripts/check_lean_audits.py"]),
        ("09-clean", ["cargo", "clean", "--profile", "dev"]),
        ("13-provider-tests", ["cargo", "test", *PROVIDER, "--workspace", "--locked",
                               "--offline"]),
        ("14-provider-fmt", ["cargo", "fmt", *PROVIDER, "--all", "--", "--check"]),
        ("15-provider-clippy", ["cargo", "clippy", *PROVIDER, "--workspace", "--all-targets",
                                "--locked", "--offline", "--", "-D", "warnings"]),
        ("16-contracts-check", ["python3", "providers/contracts.py", "check"]),
        ("17-package-build", ["python3", "providers/package.py", "build", "--output",
                              str(PACKAGES)]),
        ("18-verify-chunkio", ["python3", "providers/package.py", "verify",
                               str(PACKAGES / "qcore.chunkio")]),
        ("18-verify-cmetrics", ["python3", "providers/package.py", "verify",
                                str(PACKAGES / "qcore.cmetrics")]),
        ("18-verify-ctraces", ["python3", "providers/package.py", "verify",
                               str(PACKAGES / "qcore.ctraces")]),
        ("18-verify-libevent", ["python3", "providers/package.py", "verify",
                                str(PACKAGES / "qcore.libevent")]),
        ("18-verify-librdkafka", ["python3", "providers/package.py", "verify",
                                  str(PACKAGES / "qcore.librdkafka")]),
        ("19-provider-python", ["python3", "-m", "unittest", "discover", "-s",
                                "providers/tests"]),
        ("21-pin-check", ["python3", "docs/reports/w47-loop-owner-instance-flow/scripts/pin.py",
                          "PINCHECK", "--check"]),
        ("22-pin-verify", ["target/release/quirin-audit", "pin", "--verify-contracts"]),
        ("20-provider-clean", ["cargo", "clean", *PROVIDER, "--profile", "dev"]),
    ],
    "records": [
        ("10-python", ["python3", "-m", "unittest", "discover", "-s", "scripts/tests"]),
        ("11-docs-index", ["python3", "scripts/build_docs_index.py", "--check"]),
        ("12-clang", ["/usr/lib/llvm-21/bin/clang", "--version"]),
    ],
}


def main():
    logs, phase = pathlib.Path(sys.argv[1]).resolve(), sys.argv[2]
    logs.mkdir(parents=True, exist_ok=True)
    environment = dict(os.environ, CARGO_INCREMENTAL="0",
                       PATH=f"{pathlib.Path.home() / '.elan/bin'}:{os.environ['PATH']}")
    for name, command in STEPS[phase]:
        command = [str(logs / "pin-check") if part == "PINCHECK" else part for part in command]
        with open(logs / f"{name}.log", "wb") as log:
            code = subprocess.run(command, cwd=REPO, env=environment, stdout=log,
                                  stderr=subprocess.STDOUT, check=False).returncode
        (logs / f"{name}.exit").write_text(f"{code}\n")
        print(name, code, flush=True)


if __name__ == "__main__":
    main()
