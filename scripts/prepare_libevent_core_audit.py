#!/usr/bin/env python3
"""Prepare the pinned `libevent-core` configured-audit inputs (card G03-F).

The audited checkout is the repository's own pinned `third_party/libevent`
submodule, whose C sources sit at the checkout root. CMake is configured with
Clang so Libevent's generated `event2/event-config.h` and `evconfig-private.h`
exist, and `event_core_static` is built natively from the same tree.

Unlike the archived G03 attempt, **CMake's unmodified compilation database is
the audit input**. Nothing is rewritten, dropped or hand-authored: the
configured audit now transports every build option Libevent's `CMakeLists.txt`
appends, and `audit.json` names the target whose object directory it reads, so
the per-target duplicate commands are narrowed by a declared selection instead
of being chosen silently.

This prepares inputs and records provenance. It certifies no Libevent
behaviour: every verdict the audit publishes is a bounded resource projection
under declared contracts, and no raw unit is promoted.
"""

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]
PROFILE_DIR = ROOT / "contract/profiles/libevent-core"
OBJECT_PREFIX = "CMakeFiles/event_core_static.dir/"


def capture(*command):
    return subprocess.run(command, check=True, text=True, capture_output=True).stdout.strip()


def logged(command, path):
    result = subprocess.run(command, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    path.write_text(result.stdout)
    if result.returncode:
        raise RuntimeError(f"command failed ({result.returncode}); see {path}")


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def selected_units(entries, checkout):
    """The root-level `.c` files the selected target compiles."""
    units = set()
    for entry in entries:
        command = entry.get("command") or " ".join(entry["arguments"])
        output = entry.get("output")
        if output is None:
            words = command.split()
            output = next(
                (words[index + 1] for index, word in enumerate(words) if word == "-o"),
                "",
            )
        if not output.startswith(OBJECT_PREFIX):
            continue
        source = Path(entry["file"]).resolve()
        if source.parent == checkout and source.suffix == ".c":
            units.add(source.name)
    return sorted(units)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / "build/cycle19")
    parser.add_argument("--clang", default="clang")
    parser.add_argument("--clangxx", default="clang++")
    parser.add_argument("--cmake", default="cmake")
    parser.add_argument(
        "--write-profile",
        action="store_true",
        help="regenerate the audited-file digest table in profile.json from the "
        "verified pinned checkout (design Q2: generated digests are committed "
        "only alongside the record that validated them)",
    )
    args = parser.parse_args()

    profile = json.loads((PROFILE_DIR / "profile.json").read_text())
    pin = profile["pins"]["libevent"]["git_commit"]

    checkout = (ROOT / "third_party/libevent").resolve(strict=True)
    if capture("git", "-C", str(checkout), "rev-parse", "HEAD") != pin:
        raise ValueError("third_party/libevent HEAD does not match the configured source pin")
    if capture("git", "-C", str(checkout), "status", "--porcelain", "--untracked-files=no"):
        raise ValueError("third_party/libevent has tracked changes; the pin is not clean")

    clang = shutil.which(args.clang)
    clangxx = shutil.which(args.clangxx)
    if not clang or not clangxx:
        raise ValueError("both Clang C and C++ compilers are required")
    compiler_version = capture(clang, "--version")
    if "clang version" not in compiler_version:
        raise ValueError("C compiler does not identify itself as Clang")

    build = args.output.resolve() / "libevent-clang-build"
    build.mkdir(parents=True, exist_ok=True)
    cache = build / "CMakeCache.txt"
    if cache.exists():
        values = dict(
            line.split("=", 1)
            for line in cache.read_text().splitlines()
            if "=" in line and not line.startswith(("#", "//"))
        )
        if values.get("CMAKE_HOME_DIRECTORY:INTERNAL") != str(checkout):
            raise ValueError("build cache belongs to another source; choose a fresh output")
        cached = values.get("CMAKE_C_COMPILER:FILEPATH", values.get("CMAKE_C_COMPILER:STRING"))
        if not cached or Path(cached).resolve() != Path(clang).resolve():
            raise ValueError("build cache uses another compiler; choose a fresh output")

    configure = [
        args.cmake, "-S", str(checkout), "-B", str(build),
        f"-DCMAKE_C_COMPILER={clang}", f"-DCMAKE_CXX_COMPILER={clangxx}",
        "-DCMAKE_C_STANDARD=17", "-DCMAKE_EXPORT_COMPILE_COMMANDS=ON",
        "-DEVENT__LIBRARY_TYPE=STATIC", "-DEVENT__DISABLE_OPENSSL=ON",
        "-DEVENT__DISABLE_MBEDTLS=ON", "-DEVENT__DISABLE_TESTS=ON",
        "-DEVENT__DISABLE_REGRESS=ON", "-DEVENT__DISABLE_SAMPLES=ON",
        "-DEVENT__DISABLE_BENCHMARK=ON",
    ]
    logged(configure, build / "configure.log")
    native = [args.cmake, "--build", str(build), "--target", "event_core_static", "-j2"]
    logged(native, build / "native-build.log")
    if capture("git", "-C", str(checkout), "status", "--porcelain", "--untracked-files=no"):
        raise ValueError("the native build changed tracked source files")

    database = build / "compile_commands.json"
    entries = json.loads(database.read_text())
    units = selected_units(entries, checkout)
    if not units:
        raise ValueError("the selected target compiles no root-level C source")

    if args.write_profile:
        profile["libraries"]["libevent-core"]["audited_files"] = [
            {"path": name, "sha256": digest(checkout / name)} for name in units
        ]
        (PROFILE_DIR / "profile.json").write_text(json.dumps(profile, indent=2) + "\n")
    else:
        declared = {entry["path"]: entry["sha256"] for entry in
                    profile["libraries"]["libevent-core"]["audited_files"]}
        if sorted(declared) != units:
            raise ValueError(
                "the selected target's unit set differs from the declared audited files; "
                "rerun with --write-profile and review the diff"
            )
        for name, expected in declared.items():
            if digest(checkout / name) != expected:
                raise ValueError(f"audited file {name} does not match its declared digest")

    config = json.loads((PROFILE_DIR / "audit.json").read_text())
    config.update(
        profile_directory=str(PROFILE_DIR),
        checkout=str(checkout),
        compile_database=str(database),
        report=str(build / "libevent-core-audit.json"),
    )
    config_path = build / "libevent-core-audit.json.config"
    config_path.write_text(json.dumps(config, indent=2) + "\n")

    provenance = dict(
        schema_version=1,
        source_set="libevent-core",
        source_revision=pin,
        checkout=str(checkout),
        build=str(build),
        compiler_version=compiler_version,
        configure_command=configure,
        native_build_command=native,
        object_prefix=OBJECT_PREFIX,
        selected_units=units,
        cmake_database_sha256=digest(database),
        cmake_database_entries=len(entries),
        static_library_sha256=digest(build / "lib/libevent_core.a"),
        source_sha256={name: digest(checkout / name) for name in units},
        scope=(
            "CMake's unmodified compilation database is the audit input. The audit "
            "reads only the records whose object output lies under the declared "
            "target prefix; every remaining build option is transported into "
            "preprocessing, syntax and the typed AST. Bounded C resource projection "
            "only: not GCC equivalence, not code generation, not whole-library safety."
        ),
    )
    (build / "libevent-build-provenance.json").write_text(json.dumps(provenance, indent=2) + "\n")
    print(config_path)


if __name__ == "__main__":
    main()
