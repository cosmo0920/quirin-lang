#!/usr/bin/env python3
"""mutations.py OUT_DIR -- QUOTA-PROCS-DESCRIPTOR-DUP-FAIL-OPEN: reverts each
repair on its own and runs the test target that must catch it.

Each mutation rewrites one exact span of a source file, runs one cargo test
target, restores the original bytes and asserts the round trip. Logs go to
OUT_DIR/<id>.log with the exit beside them; a summary is printed and written to
OUT_DIR/summary.json. A mutation is caught when the target exits nonzero and
every test it names fails."""
import json
import os
import pathlib
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
LIB = "crates/qcore-verifier-host/src/lib.rs"
QUOTA = "crates/qcore-verifier-host/src/process_quota.rs"

MUTATIONS = [
    {
        "id": "m1-dup-failure-drops-the-join",
        "what": "the base behaviour: a failed duplicate is mapped to no join",
        "file": LIB,
        "old": "    quota\n        .as_ref()\n        .map(|session| session.procs_descriptor(probe))\n        .transpose()\n",
        "new": "    Ok(quota\n        .as_ref()\n        .and_then(|session| session.procs_descriptor(probe).ok()))\n",
        "target": ["--test", "execution_quota"],
        "caught_by": ["a_failed_join_descriptor_refuses_before_spawning"],
    },
    {
        "id": "m2-observation-unconditional",
        "what": "the base behaviour: every created cgroup is observed, spawned or not",
        "file": LIB,
        "old": "        let quota_observed = if observed.spawned {\n",
        "new": "        let quota_observed = if observed.spawned || quota.is_some() {\n",
        "target": ["--test", "execution_quota"],
        "caught_by": [
            "an_identity_refusal_after_the_cgroup_exists_records_no_observation",
            "a_failed_spawn_records_no_observation",
        ],
    },
    {
        "id": "m3-short-join-write-accepted",
        "what": "only a negative count fails the join, as at the base",
        "file": QUOTA,
        "old": "        if written.unsigned_abs() != expected {\n",
        "new": "        if written.unsigned_abs() != expected && false {\n",
        "target": ["--lib"],
        "caught_by": ["process_quota::tests::only_a_whole_join_write_is_a_join"],
    },
]


def main() -> int:
    out = pathlib.Path(sys.argv[1]).resolve()
    out.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, CARGO_BUILD_JOBS="4", CARGO_INCREMENTAL="0")
    results = []
    for mutation in MUTATIONS:
        path = REPO / mutation["file"]
        original = path.read_bytes()
        text = original.decode()
        assert text.count(mutation["old"]) == 1, mutation["id"]
        path.write_bytes(text.replace(mutation["old"], mutation["new"]).encode())
        try:
            log = out / f"{mutation['id']}.log"
            with log.open("wb") as handle:
                exit_code = subprocess.run(
                    ["cargo", "test", "-p", "qcore-verifier-host", *mutation["target"]],
                    cwd=REPO, env=env, stdout=handle, stderr=subprocess.STDOUT,
                ).returncode
        finally:
            path.write_bytes(original)
            assert path.read_bytes() == original, mutation["id"]
        (out / f"{mutation['id']}.exit").write_text(f"{exit_code}\n")
        text = log.read_text(errors="replace")
        failed = [name for name in mutation["caught_by"] if f"test {name} ... FAILED" in text]
        caught = exit_code != 0 and failed == mutation["caught_by"]
        results.append({"id": mutation["id"], "what": mutation["what"], "exit": exit_code,
                         "expected_failures": mutation["caught_by"], "failed": failed,
                         "caught": caught})
        print(f"{mutation['id']}: exit {exit_code}, caught {caught}, failed {failed}")
    (out / "summary.json").write_text(json.dumps(results, indent=1) + "\n")
    return 0 if all(result["caught"] for result in results) else 1


if __name__ == "__main__":
    raise SystemExit(main())
