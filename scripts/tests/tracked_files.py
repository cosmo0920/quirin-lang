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

"""Tracked-file discovery for tests that find their inputs by walking the
repository (card W45-TEST-HYGIENE; the rule is "Tracked test inputs" in
`docs/agents/validation.md`). The Python twin of `tests/support/tracked_files.rs`.

A test that walked the working tree read whatever lay there: ignored
`private/`, `build/` and `target/` content, untracked exports and other
worktrees. This lists what git tracks instead, so ignored and untracked
content never changes a result.

Fail-closed: when git cannot run, or the directory is not inside a git
checkout (an exported tarball), or nothing is tracked there, the helper raises
`TrackedFilesUnavailable`, which fails the calling test with the reason. It
never falls back to walking the filesystem.
"""
import os
import subprocess
from pathlib import Path


class TrackedFilesUnavailable(AssertionError):
    """Tracked-file discovery could not run; the test must fail, not walk."""


def tracked_files(directory):
    """Every file `git ls-files` lists under `directory`, as `directory / path`,
    sorted. The index is the source, so a file added with `git add` and not
    yet committed is listed; submodule entries are skipped, and so is a
    tracked path deleted from the working tree, as a walk would have missed it.
    """
    directory = Path(directory)
    environment = {key: value for key, value in os.environ.items()
                   if key not in ("GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE")}
    try:
        result = subprocess.run(
            ["git", "-C", str(directory), "ls-files", "-z", "--cached", "--", "."],
            capture_output=True, env=environment)
    except OSError as error:
        raise TrackedFilesUnavailable(
            f"this test discovers its inputs with `git ls-files`, which could not run ({error}); "
            "it needs git and a git checkout, and never walks the filesystem instead "
            "(W45-TEST-HYGIENE)") from error
    if result.returncode != 0:
        raise TrackedFilesUnavailable(
            f"`git ls-files` failed under {directory} (exit {result.returncode}): this test "
            "discovers its inputs from tracked files only and must run in a git checkout, "
            "not an export (W45-TEST-HYGIENE):\n" + result.stderr.decode(errors="replace"))
    files = []
    for entry in result.stdout.split(b"\0"):
        if not entry:
            continue
        path = directory / entry.decode()
        if (path.is_symlink() or path.exists()) and not (path.is_dir() and not path.is_symlink()):
            files.append(path)
    if not files:
        raise TrackedFilesUnavailable(
            f"git tracks no file under {directory}: an untracked or ignored directory, "
            "or the wrong path (W45-TEST-HYGIENE)")
    return sorted(files)


def tracked_relative(directory):
    """`tracked_files(directory)` as POSIX paths relative to `directory`."""
    directory = Path(directory)
    return [path.relative_to(directory).as_posix() for path in tracked_files(directory)]
