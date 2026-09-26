#!/usr/bin/env python3
"""Run the tests the W45-TEST-HYGIENE census converted, one command each.

    run_controls.py ROOT OUT_DIR GROUP [GROUP ...]

GROUP is `compiler`, `gateway`, `providers`, `python` or `contracts`. Each
command's output goes to OUT_DIR/<name>.log and its exit to OUT_DIR/<name>.exit
(never through a pipe); `summary.txt` lists name, exit and libtest/unittest
totals. The script only runs tests: `decoys.py` places and removes decoys.
"""
import os
import re
import subprocess
import sys
from pathlib import Path

COMPILER_TESTS = [
    "std_socket_inet", "examples_suite", "ct_corpus", "proof_leg_digests",
    "first_party_discovery", "contract_inherit_qcore_provider", "composed_five_providers",
    "tracked_inputs",
]
CONTRACT_TESTS = [
    "qcore_contract_corpus", "contract_loader", "contract_view_census",
    "contract_inherit_adoption", "contract_inherit_parity", "contract_inherit_qcore_provider",
]
PROVIDER_CRATES = [
    ("quirin-provider-libc", "--lib"), ("quirin-provider-shared", "--lib"),
    ("quirin-provider-cfl", "--lib"), ("quirin-provider-mpack", "--lib"),
    ("quirin-provider-msgpack", "--lib"), ("quirin-provider-fluent-bit", "--lib"),
    ("qcore-provider-chunkio", "--bins"), ("qcore-provider-cmetrics", "--bins"),
    ("qcore-provider-ctraces", "--bins"), ("qcore-provider-libevent", "--bins"),
    ("qcore-provider-librdkafka", "--bins"),
]
PROVIDERS = ["cargo", "test", "--manifest-path", "providers/Cargo.toml", "--locked", "--offline"]


def commands(group):
    if group == "compiler":
        for test in COMPILER_TESTS:
            yield f"compiler-{test}", ["cargo", "test", "-p", "qcore-compiler", "--test", test]
        yield "qcorec-examples_cli", ["cargo", "test", "-p", "qcorec", "--test", "examples_cli"]
        yield "verification-composition_bounds", [
            "cargo", "test", "-p", "qcore-verification", "--test", "composition_bounds"]
    elif group == "contracts":
        for test in CONTRACT_TESTS:
            yield f"compiler-{test}", ["cargo", "test", "-p", "qcore-compiler", "--test", test]
        for crate, kind in PROVIDER_CRATES:
            yield f"provider-{crate}", PROVIDERS + ["-p", crate, kind]
        yield "python-test_contract_corpus", [
            sys.executable, "-m", "unittest", "discover", "-s", "scripts/tests",
            "-p", "test_contract_corpus.py"]
    elif group == "gateway":
        yield "gateway-census_sink_inventory", [
            "cargo", "test", "-p", "qcore-c-gateway", "--features", "indirect-census", "--lib",
            "census_sink_inventory"]
    elif group == "providers":
        yield "conformance-packages-template", PROVIDERS + [
            "-p", "qcore-provider-conformance", "--test", "packages",
            "only_template_carrying_crates_are_packaged"]
        for crate, kind in PROVIDER_CRATES:
            yield f"provider-{crate}", PROVIDERS + ["-p", crate, kind]
    elif group == "python":
        for pattern in ["test_retired_scripts.py", "test_contract_corpus.py", "test_tracked_files.py"]:
            if (Path("scripts/tests") / pattern).exists():
                yield f"python-{pattern[:-3]}", [
                    sys.executable, "-m", "unittest", "discover", "-s", "scripts/tests",
                    "-p", pattern]
    else:
        sys.exit(f"unknown group {group}")


def totals(text):
    passed = sum(int(n) for n in re.findall(r"(\d+) passed", text))
    failed = sum(int(n) for n in re.findall(r"(\d+) failed", text))
    ran = re.findall(r"Ran (\d+) tests?", text)
    verdict = re.findall(r"^(OK.*|FAILED.*)$", text, re.M)
    if ran:
        return f"python ran {sum(map(int, ran))}: {'; '.join(verdict)}"
    return f"passed {passed}, failed {failed}"


def main():
    root, out = Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve()
    out.mkdir(parents=True, exist_ok=False)
    environment = dict(os.environ, CARGO_INCREMENTAL="0")
    summary = []
    for group in sys.argv[3:]:
        for name, command in commands(group):
            log = out / f"{name}.log"
            with log.open("w") as handle:
                handle.write("$ " + " ".join(command) + "\n")
                handle.flush()
                exit_code = subprocess.run(command, cwd=root, stdout=handle,
                                           stderr=subprocess.STDOUT, env=environment).returncode
            (out / f"{name}.exit").write_text(f"{exit_code}\n")
            summary.append(f"{name}\texit {exit_code}\t{totals(log.read_text(errors='replace'))}")
            print(summary[-1], flush=True)
    (out / "summary.txt").write_text("\n".join(summary) + "\n")


if __name__ == "__main__":
    main()
