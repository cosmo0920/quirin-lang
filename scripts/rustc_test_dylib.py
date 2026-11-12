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

"""``build.rustc-wrapper`` for dynamically linked test builds (Linux).

Card TEST-STORAGE-BUDGET.  Every test executable of the root workspace used to
link its own static copy of the analysis crates and their line tables (573
executables, about 125 MB each, about 71.5 GB in all).  Under this wrapper
each library of the root workspace is also emitted as a Rust ``dylib`` beside
its rlib, and every linked artifact of the root workspace (test harnesses,
binaries, examples and those dylibs) is linked with ``-C prefer-dynamic``
against them, so the code and its line tables exist once, in
``target/<profile>/deps/lib<crate>-<hash>.so``.

It is opt-in: ``scripts/test_link_config.py`` names it for the test
configuration only (the standing gate, the scoped gate and CI's Linux test
steps).  Nothing else -- release builds, ``quirin-audit measure``'s ``rev:``
sides, provider packages, the provider workspace, plain ``cargo build`` --
passes through it, and no manifest changes, so those builds are what they
were.

Cargo calls ``<this> <rustc> <args...>`` for every crate.  Per invocation:

* a probe (``-vV``, ``--print``), a compile that does not link (``--emit``
  without ``link``: check, clippy), a build script, a proc-macro, or a crate
  outside the root workspace (registry crates, ``providers/``): the arguments
  are unchanged;
* a root-workspace library (``--crate-type lib``/``rlib``): ``--crate-type
  dylib`` is added, so the same rustc run writes ``lib<crate>-<hash>.rlib``
  and ``lib<crate>-<hash>.so``;
* every linked root-workspace artifact: ``-C prefer-dynamic``; each
  ``--extern name=…/lib<crate>-<hash>.{rmeta,rlib}`` becomes the rlib plus
  the sibling ``.so`` when one exists (rustc considers only the exact paths
  cargo names for a direct dependency, and cargo names the ``.rmeta`` of a
  dependency it believes needs no linking); and absolute run paths to the
  dependency directory and the toolchain's ``target-libdir`` (``libstd-*.so``),
  so an executable started with a cleared environment still finds its
  libraries.

Cargo does not know about the ``.so`` outputs, so it would start a dependent
library as soon as rustc announces the dependency's ``.rmeta`` (pipelining),
before the rlib and ``.so`` it now links against exist.  The wrapper therefore
withholds that announcement (the ``"emit":"metadata"`` artifact line on
rustc's stderr) for every crate; cargo then treats the metadata as produced
when the rustc run ends, which is cargo's own fallback for a job that never
announced it.  Pipelining is lost; nothing else about scheduling changes.  A
dependency whose ``.so`` is missing is linked statically: correct, only larger.

ELF only: a Windows DLL is capped at 65,535 exported symbols, which the large
crates exceed, so ``test_link_config`` names this wrapper on Linux only.
"""

from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import sys
from typing import Optional, Sequence

REPO_ROOT = Path(__file__).resolve().parents[1]
# Not root-workspace members: their own workspace, built without this wrapper.
OUTSIDE = ("providers",)


def option_values(args: Sequence[str], flag: str) -> list[str]:
    """Values of `flag value` and `flag=value` forms."""
    values = []
    for index, arg in enumerate(args):
        if arg == flag and index + 1 < len(args):
            values.append(args[index + 1])
        elif arg.startswith(flag + "="):
            values.append(arg[len(flag) + 1:])
    return values


def links(args: Sequence[str]) -> bool:
    emits = option_values(args, "--emit")
    if not emits:
        return True  # rustc's default emit is link
    return any("link" in value.split(",") for value in emits)


def root_member(manifest_dir: Optional[str], root: Path = REPO_ROOT) -> bool:
    """Whether cargo is compiling a package of the root workspace."""
    if not manifest_dir:
        return False
    try:
        relative = Path(manifest_dir).resolve().relative_to(root)
    except ValueError:
        return False
    return not relative.parts or relative.parts[0] not in OUTSIDE


def dependency_dirs(args: Sequence[str]) -> list[str]:
    """Every `-L dependency=` directory, in order. Cargo before 1.100 passes
    one (`target/<profile>/deps`); its per-unit build-directory layout passes
    one per dependency (`target/<profile>/build/<pkg>/<hash>/out`), and the
    dylibs live in those, so each must be on the runpath."""
    return [value[len("dependency="):] for value in option_values(args, "-L")
            if value.startswith("dependency=")]


def dylib_dirs(args: Sequence[str]) -> list[str]:
    """The directory of every `.so` named by an `--extern` in `args`."""
    directories = []
    for index, arg in enumerate(args):
        value = (args[index + 1] if arg == "--extern" and index + 1 < len(args)
                 else arg[len("--extern="):] if arg.startswith("--extern=") else None)
        if value and "=" in value and value.endswith(".so"):
            directories.append(str(Path(value.split("=", 1)[1]).parent))
    return directories


def linkable_extern(value: str) -> list[str]:
    """`name=…/lib<crate>-<hash>.{rmeta,rlib}` -> the linkable forms that exist."""
    if "=" not in value:
        return [value]
    name, path = value.split("=", 1)
    stem = None
    for suffix in (".rlib", ".rmeta"):
        if path.endswith(suffix):
            stem = path[: -len(suffix)]
    if stem is None:
        return [value]
    rlib, shared = stem + ".rlib", stem + ".so"
    forms = [f"{name}={rlib}" if os.path.exists(rlib) else value]
    if os.path.exists(shared):
        forms.append(f"{name}={shared}")
    return forms


def with_dylib_externs(args: Sequence[str]) -> list[str]:
    """Rewrite every `--extern` to its linkable forms (rlib, plus `.so` if any)."""
    result: list[str] = []
    index = 0
    while index < len(args):
        arg = args[index]
        if arg == "--extern" and index + 1 < len(args):
            for form in linkable_extern(args[index + 1]):
                result += ["--extern", form]
            index += 2
        elif arg.startswith("--extern="):
            for form in linkable_extern(arg[len("--extern="):]):
                result += ["--extern", form]
            index += 1
        else:
            result.append(arg)
            index += 1
    return result


def probe(args: Sequence[str]) -> bool:
    return (not option_values(args, "--crate-name")
            or any(arg in ("-V", "-vV", "--version") or arg.startswith("--print")
                   for arg in args))


def rewrite(args: Sequence[str], manifest_dir: Optional[str], libdir: str) -> list[str]:
    """The rustc arguments this wrapper runs for `args`."""
    crate_names = option_values(args, "--crate-name")
    crate_types = option_values(args, "--crate-type")
    if (probe(args)
            or not links(args)
            or not root_member(manifest_dir)
            or any(name.startswith("build_script_") for name in crate_names)
            or "proc-macro" in crate_types):
        return list(args)
    rewritten = with_dylib_externs(args)
    if any(kind in ("lib", "rlib") for kind in crate_types):
        rewritten += ["--crate-type", "dylib"]
    rewritten += ["-C", "prefer-dynamic"]
    runpath: list[str] = []
    for directory in [*dependency_dirs(args), *dylib_dirs(rewritten), libdir]:
        if directory and str(Path(directory).resolve()) not in runpath:
            runpath.append(str(Path(directory).resolve()))
    for directory in runpath:
        rewritten += ["-C", f"link-arg=-Wl,-rpath,{directory}"]
    return rewritten


def withheld(line: bytes) -> bool:
    """rustc's announcement that the `.rmeta` is ready (cargo's pipelining cue)."""
    if b'"$message_type":"artifact"' not in line:
        return False
    try:
        message = json.loads(line)
    except ValueError:
        return False
    return message.get("$message_type") == "artifact" and message.get("emit") == "metadata"


def main(argv: Sequence[str]) -> int:
    if not argv:
        print("usage: rustc_test_dylib.py <rustc> [args...]", file=sys.stderr)
        return 2
    rustc, args = argv[0], list(argv[1:])
    if probe(args):
        os.execvp(rustc, [rustc, *args])
    libdir = ""
    if links(args) and root_member(os.environ.get("CARGO_MANIFEST_DIR")):
        libdir = subprocess.run([rustc, "--print", "target-libdir"], capture_output=True,
                                text=True, check=True).stdout.strip()
    command = [rustc, *rewrite(args, os.environ.get("CARGO_MANIFEST_DIR"), libdir)]
    # close_fds=False: rustc inherits cargo's jobserver descriptors
    # (CARGO_MAKEFLAGS), as it would without the wrapper.
    process = subprocess.Popen(command, stderr=subprocess.PIPE, close_fds=False)
    assert process.stderr is not None
    out = sys.stderr.buffer
    for line in process.stderr:
        if not withheld(line):
            out.write(line)
            out.flush()
    return process.wait()


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
