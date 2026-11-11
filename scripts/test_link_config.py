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

"""The cargo configuration of the root workspace's test builds.

Card TEST-STORAGE-BUDGET; the policy is docs/agents/validation.md "Test build
storage".  One function, ``cargo_config_args()``, names the ``--config``
arguments that ``scripts/run_test_binaries.py`` (the standing gate),
``scripts/gate_scope.py`` (the scoped gate, through the runner and its
reevaluation examples) and CI pass to every cargo command that builds or runs
the root workspace's tests, so local gates and CI build the same thing:

* ``build.incremental=false`` everywhere: a test build in CI or a gate starts
  from an empty or foreign ``target/``, and incremental state is several GB
  that nothing reads again;
* on Linux, ``build.rustc-wrapper`` = ``scripts/rustc_test_dylib.py``: the
  workspace libraries become Rust dylibs and every test executable, binary
  and example links them dynamically (see that file).  Other platforms build
  statically, as before.

These are ``--config`` arguments, never environment variables, so cargo runs
started by tests (``quirin-audit measure``'s ``rev:`` sides) and the provider
workspace never inherit them.  Run as a script it prints the arguments one
per line, for ``mapfile -t config < <(python3 scripts/test_link_config.py)``
in bash or ``$config = python scripts/test_link_config.py`` in PowerShell.
"""

from __future__ import annotations

import json
from pathlib import Path
import sys
from typing import Optional

REPO_ROOT = Path(__file__).resolve().parents[1]
WRAPPER = REPO_ROOT / "scripts" / "rustc_test_dylib.py"

LINK_MODES = ("dynamic", "static")


def default_link(platform: str = sys.platform) -> str:
    """Dynamic test linking on Linux (ELF); static elsewhere."""
    return "dynamic" if platform.startswith("linux") else "static"


def cargo_config_args(link: Optional[str] = None, platform: str = sys.platform,
                      wrapper: Path = WRAPPER) -> list[str]:
    """`--config` arguments for a root-workspace test build."""
    link = link or default_link(platform)
    if link not in LINK_MODES:
        raise ValueError(f"unknown link mode {link!r}; expected one of {LINK_MODES}")
    args = ["--config", "build.incremental=false"]
    if link == "dynamic":
        args += ["--config", f"build.rustc-wrapper={json.dumps(str(wrapper))}"]
    return args


def main(argv: list[str]) -> int:
    link = argv[0] if argv else None
    try:
        args = cargo_config_args(link)
    except ValueError as error:
        print(f"error: {error}", file=sys.stderr)
        return 2
    for arg in args:
        print(arg)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
