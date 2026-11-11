#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Build named Lean modules and run named audits, the scoped card gate's Lean step.

Card GATE-ELASTIC.  ``scripts/gate_scope.py`` calls this with the changed
``proof/QCore`` modules (and the modules importing them) and the ``Audit*.lean``
files that import any of them; when ``proof/`` roots change it runs the whole
``scripts/check_lean_audits.py`` instead.  This reuses that runner's own pieces
unchanged -- the pinned-toolchain guard, the bounded build step and the
parallel audit runner with ``-DwarningAsError=true`` -- so a scoped Lean step
checks what the full one checks, for fewer modules.  ``AuditAll`` (the
whole-project axiom closure) belongs to the full run only.
"""

from __future__ import annotations

import argparse
from pathlib import Path
import shutil
import subprocess
import sys
from typing import Optional, Sequence

sys.path.insert(0, str(Path(__file__).resolve().parent))
import check_lean_audits as lean  # noqa: E402

PROOF = Path(__file__).resolve().parents[1] / "proof"


def main(argv: Optional[Sequence[str]] = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--module", action="append", default=[],
                        help="a module to build, e.g. QCore.ListLookup (repeatable)")
    parser.add_argument("--audit", action="append", default=[],
                        help="an audit file to check, e.g. AuditListLookup.lean (repeatable)")
    parser.add_argument("--build-timeout", type=lean.positive_seconds, default=600)
    parser.add_argument("--audit-timeout", type=lean.positive_seconds, default=120)
    parser.add_argument("--heartbeat", type=lean.positive_seconds, default=15)
    parser.add_argument("--jobs", type=lean.positive_jobs, default=lean.default_jobs())
    args = parser.parse_args(argv)
    if not args.module and not args.audit:
        parser.error("nothing to build or audit")
    audits = []
    for name in sorted(set(args.audit)):
        path = PROOF / name
        if not path.is_file() or not path.name.startswith("Audit") or path.suffix != ".lean":
            print(f"Lean checks failed: no audit file {path}", file=sys.stderr)
            return 1
        if path.name == "AuditAll.lean":
            print("Lean checks failed: AuditAll belongs to the full check_lean_audits.py run",
                  file=sys.stderr)
            return 1
        audits.append(path)
    try:
        if shutil.which("lake") is None:
            raise lean.missing_lake_error()
        lean.check_pinned_toolchain(PROOF, args.build_timeout)
        # The audits' own imports are built too, so an audit never reads a stale
        # or missing .olean.
        targets = sorted(set(args.module) | {
            line.split()[1]
            for path in audits
            for line in path.read_text(encoding="utf-8").splitlines()
            if line.startswith("import ") and len(line.split()) == 2
        })
        if targets:
            lean.run_step(["lake", "build", *targets], cwd=PROOF,
                          label=f"Lean build of {len(targets)} targets",
                          timeout=args.build_timeout, heartbeat=args.heartbeat)
        if audits:
            jobs = min(args.jobs, len(audits))
            if jobs > 1:
                lean.run_audits_in_parallel(PROOF, audits, args, jobs)
            else:
                for index, audit in enumerate(audits, 1):
                    lean.run_step(lean.audit_command(audit.name), cwd=PROOF,
                                  label=f"{audit.name} ({index}/{len(audits)})",
                                  timeout=args.audit_timeout, heartbeat=args.heartbeat)
        print(f"Built {len(targets)} Lean targets", flush=True)
        print(f"Passed {len(audits)} Lean audits", flush=True)
    except (OSError, ValueError, TimeoutError, subprocess.CalledProcessError) as error:
        print(f"Lean checks failed: {error}", file=sys.stderr, flush=True)
        return 1
    except KeyboardInterrupt:
        print("Lean checks interrupted; stopped active process tree", file=sys.stderr,
              flush=True)
        return 130
    return 0


if __name__ == "__main__":
    sys.exit(main())
