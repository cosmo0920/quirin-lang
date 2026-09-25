#!/usr/bin/env python3
"""Independently verify that the runner replays cargo's own launch of a test.

Usage (worktree root, after a build):
  verify_env.py <outdir> <package> <test-target>

1. Runs plain `cargo test -p <package> --test <test-target>` under
   `strace -f -v -s 65536 -e trace=execve,chdir`, and extracts the execve of
   the test executable (argv and complete envp) and the last chdir that the
   same pid made before it: this is what cargo really launched, observed from
   the kernel's side, with no runner configured.
2. Runs the runner's capture pass for the same target (cargo with the capture
   `target.<host>.runner`) and loads the record.
3. Compares argv, cwd and the environment variable by variable. Exit 0 only if
   they are identical.
"""

import json
import os
from pathlib import Path
import re
import subprocess
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[4] / "scripts"))
import run_test_binaries as runner  # noqa: E402

STRING = r'"((?:[^"\\]|\\.)*)"'
EXECVE_RE = re.compile(r'^(\d+)\s+execve\(' + STRING + r', \[(.*?)\], \[(.*)\]\) = 0')
CHDIR_RE = re.compile(r'^(\d+)\s+chdir\(' + STRING + r'\) = 0')


def unescape(text: str) -> str:
    return bytes(text, "utf-8").decode("unicode_escape").encode("latin-1").decode("utf-8")


def strings(text: str) -> list[str]:
    return [unescape(m) for m in re.findall(STRING, text)]


def main() -> int:
    out = Path(sys.argv[1]).resolve()
    package, target = sys.argv[2], sys.argv[3]
    out.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ)
    env.setdefault("CARGO_INCREMENTAL", "0")
    trace = out / "strace.log"
    subprocess.run(["strace", "-f", "-v", "-s", "65536", "-e", "trace=execve,chdir",
                    "-o", str(trace), "cargo", "test", "-p", package, "--test", target],
                   env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True)
    observed = None
    last_chdir: dict[str, str] = {}
    for line in trace.read_text(encoding="utf-8", errors="replace").splitlines():
        chdir = CHDIR_RE.match(line)
        if chdir:
            last_chdir[chdir[1]] = unescape(chdir[2])
            continue
        execve = EXECVE_RE.match(line)
        if execve and re.search(rf"/deps/{re.escape(target)}-[0-9a-f]{{16}}$", unescape(execve[2])):
            envp = strings(execve[4])
            observed = {
                "argv": strings(execve[3]),
                "cwd": last_chdir.get(execve[1]),
                "env": dict(item.split("=", 1) for item in envp),
            }
    if observed is None:
        print("no execve of the test executable was observed")
        return 1
    capture_dir = out / "capture"
    capture_dir.mkdir(exist_ok=True)
    triple = runner.host_triple(env)
    command = runner.capture_command(capture_dir)
    config = f"target.{triple}.runner=[{', '.join(json.dumps(p) for p in command)}]"
    subprocess.run(["cargo", "test", "-p", package, "--test", target, "--config", config],
                   env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True)
    [record_path] = list(capture_dir.glob("*.json"))
    record = json.loads(record_path.read_text())
    problems = []
    # strace shows argv[0] as cargo passed it; the runner replays argv as captured.
    if observed["argv"] != record["argv"]:
        problems.append(f"argv differs: {observed['argv']} vs {record['argv']}")
    if observed["cwd"] is None or os.path.realpath(observed["cwd"]) != os.path.realpath(record["cwd"]):
        problems.append(f"cwd differs: {observed['cwd']} vs {record['cwd']}")
    keys = sorted(set(observed["env"]) | set(record["env"]))
    for key in keys:
        if observed["env"].get(key) != record["env"].get(key):
            problems.append(f"env {key}: {observed['env'].get(key)!r} vs {record['env'].get(key)!r}")
    report = {"observed": observed, "captured": record, "problems": problems,
              "env_vars_compared": len(keys)}
    (out / "verify.json").write_text(json.dumps(report, indent=1) + "\n")
    print(f"compared argv, cwd and {len(keys)} environment variables: "
          f"{'IDENTICAL' if not problems else str(len(problems)) + ' differences'}")
    for problem in problems:
        print("DIFF", problem)
    return 0 if not problems else 1


if __name__ == "__main__":
    sys.exit(main())
