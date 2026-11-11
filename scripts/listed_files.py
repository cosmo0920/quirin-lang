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

"""File listing for the generators of committed files (`gen_lean_roots.py`,
`gen_rule_docs.py`).

A generator reads tracked files plus untracked files git does not ignore, so a
card's new module or rule file is seen before `git add`, while ignored content
(`private/`, `proof/.lake/`, exports) never enters a generated file. Tests pass
the tracked list from `scripts/tests/tracked_files.py` instead (constraint 29).
Fails closed: without git or outside a checkout it raises, never walks.
"""
import os
import subprocess
from pathlib import Path


def listed_files(directory):
    directory = Path(directory)
    environment = {key: value for key, value in os.environ.items()
                   if key not in ("GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE")}
    try:
        result = subprocess.run(
            ["git", "-C", str(directory), "ls-files", "-z", "--cached", "--others",
             "--exclude-standard", "--", "."], capture_output=True, env=environment)
    except OSError as error:
        raise SystemExit(f"cannot list {directory} with git ls-files: {error}") from error
    if result.returncode != 0:
        raise SystemExit(f"git ls-files failed under {directory} (exit {result.returncode}): "
                         + result.stderr.decode(errors="replace"))
    paths = {directory / entry.decode() for entry in result.stdout.split(b"\0") if entry}
    return sorted(path for path in paths if path.is_file())
