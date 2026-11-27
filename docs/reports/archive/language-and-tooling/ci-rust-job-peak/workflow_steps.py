#!/usr/bin/env python3
"""Write one job's steps of a GitHub workflow as shell files, for ci_rust_job.sh.

    workflow_steps.py <workflow.yml> <job> <out dir>

Prints `index<TAB>kind<TAB>name` per step (kind `run` or `uses`), writes each
`run:` body to `<out dir>/<index>.sh` and the job's `env:` to `env.sh`.  A step
with `if:`, `shell:`, `working-directory:` or its own `env:` is refused: the
Linux `rust` job has none, and running one without its condition would not be
the job.  Card CI-RUST-JOB-PEAK.
"""

import shlex
import sys
from pathlib import Path

import yaml


def main(workflow, job, out):
    document = yaml.safe_load(Path(workflow).read_text())
    spec = document["jobs"][job]
    out = Path(out)
    env = spec.get("env", {}) or {}
    (out / "env.sh").write_text("".join(f"export {name}={shlex.quote(str(value))}\n"
                                        for name, value in env.items()))
    for index, step in enumerate(spec["steps"], 1):
        unsupported = {"if", "shell", "working-directory", "env"} & set(step)
        if unsupported:
            sys.exit(f"step {index} has {sorted(unsupported)}; not reproducible here")
        name = step.get("name", step.get("uses", f"step {index}")).replace("\t", " ")
        if "run" in step:
            (out / f"{index:02d}.sh").write_text(step["run"])
            print(f"{index:02d}\trun\t{name}")
        else:
            print(f"{index:02d}\tuses\t{name}")


if __name__ == "__main__":
    main(*sys.argv[1:])
