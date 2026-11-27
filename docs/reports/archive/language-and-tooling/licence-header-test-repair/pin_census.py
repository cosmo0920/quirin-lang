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
"""Digest census of LICENCE-HEADER-TEST-REPAIR.

For every file a commit modified, hash its parent blob (raw and with CRLF
normalized to LF) and list each tracked file of the working tree that names
one of those SHA-256 digests. A hit is a pin of the pre-change bytes; whether
anything replays it is decided by reading its readers, not by this census.

Usage: pin_census.py <checkout> [commit ...]   (default: 838bd777d 74105dd0e)
"""
import hashlib
import os
import re
import subprocess
import sys


def git(*args):
    return subprocess.run(["git", *args], capture_output=True, check=True).stdout


def main():
    os.chdir(sys.argv[1])
    commits = sys.argv[2:] or ["838bd777d", "74105dd0e"]
    digests = {}
    for commit in commits:
        names = git("diff", "--name-only", "--diff-filter=M", f"{commit}^", commit)
        for name in filter(None, names.decode().split("\n")):
            old = git("show", f"{commit}^:{name}")
            for kind, data in (("raw", old), ("lf", old.replace(b"\r\n", b"\n"))):
                digests[hashlib.sha256(data).hexdigest()] = (name, commit, kind)
    hits = {}
    pattern = re.compile(rb"[0-9a-f]{64}")
    for path in filter(None, git("ls-files").decode().split("\n")):
        try:
            data = open(path, "rb").read()
        except OSError:
            continue
        for match in set(pattern.findall(data)):
            found = digests.get(match.decode())
            if found:
                hits.setdefault(found[0], set()).add((path, found[1], found[2]))
    for source, pins in sorted(hits.items()):
        print(source)
        for pin in sorted(pins):
            print("   ", pin)


if __name__ == "__main__":
    main()
