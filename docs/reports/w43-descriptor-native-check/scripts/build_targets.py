#!/usr/bin/env python3
"""Relink pinned Fluent Bit targets statically with the descriptor observer.

Usage: build_targets.py PINNED_SOURCE PRIVATE_SOURCE PRIVATE_BUILD OUT_DIR

PRIVATE_SOURCE must be a byte-identical copy of PINNED_SOURCE (excluding .git
and the untracked build/ directory) and PRIVATE_BUILD a build configured with
the pinned configure command; compare_compile_db.py checks the latter. The
pinned checkout is only read. Every target is relinked from its CMake objects
and the static archives of the `fluent-bit-bin` link line, adding
`-Wl,--wrap=NAME` for each observed API and the observer object. Writes
OUT_DIR/build.json with commands, hashes and the interposition audit.
"""
import hashlib
import json
import os
import pathlib
import re
import shlex
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
HARNESS = HERE.parent / "harness"
PIN = "66910c10a4d7eafa810b84229cf2dcf7b0f26f97"

# Every function the observer defines as __wrap_NAME.
WRAPPED = sorted(set(re.findall(r"__wrap_([A-Za-z0-9_]+)\s*\(", (HARNESS / "descriptor_observer.c").read_text())
                     + re.findall(r"(?:FD_USE|FILE_USE)\([^,]+,\s*([A-Za-z0-9_]+),", (HARNESS / "descriptor_observer.c").read_text())
                     + re.findall(r"SIMPLE_FD_CONSTRUCTOR\(([A-Za-z0-9_]+),", (HARNESS / "descriptor_observer.c").read_text())))

# libc/POSIX entry points that create, release or transfer a descriptor-like
# resource. An imported member of this set that is not wrapped is an
# interposition gap; it is reported, never ignored.
DESCRIPTOR_UNIVERSE = sorted("""
open open64 openat openat64 creat creat64 socket accept accept4 dup dup2 dup3 fcntl fcntl64
pipe pipe2 socketpair epoll_create epoll_create1 eventfd timerfd_create signalfd inotify_init
inotify_init1 memfd_create mkstemp mkstemp64 mkostemp mkostemp64 mkstemps mkostemps close
close_range closefrom fopen fopen64 fdopen freopen freopen64 tmpfile tmpfile64 popen pclose
fmemopen open_memstream fclose fcloseall opendir fdopendir closedir dlopen dlmopen dlclose
getaddrinfo freeaddrinfo getifaddrs freeifaddrs mmap mmap64 munmap mremap posix_openpt shm_open
mq_open syscall posix_spawn fork vfork
""".split())

TARGETS = {
    "fluent-bit": None,
    "flb-it-pipe": "tests/internal/CMakeFiles/flb-it-pipe.dir/pipe.c.o",
    "flb-it-network": "tests/internal/CMakeFiles/flb-it-network.dir/network.c.o",
    "flb-it-fstore": "tests/internal/CMakeFiles/flb-it-fstore.dir/fstore.c.o",
    "flb-it-utils": "tests/internal/CMakeFiles/flb-it-utils.dir/utils.c.o",
    "flb-it-file": "tests/internal/CMakeFiles/flb-it-file.dir/file.c.o",
}


def sha(path):
    return hashlib.sha256(pathlib.Path(path).read_bytes()).hexdigest()


def run(command, **kwargs):
    return subprocess.run(command, check=True, text=True, **kwargs)


# CMake's configure step writes these seven files into the source tree. They
# carry the checkout/build directory spelling, and flb_version.h the commit of
# whatever git repository encloses the checkout (for the private copy, the
# QCore worktree). They are compared after normalizing exactly those tokens.
CONFIGURE_GENERATED = {
    "include/fluent-bit/flb_info.h", "include/fluent-bit/flb_version.h",
    "lib/cfl/include/cfl/cfl_info.h", "lib/cmetrics/include/cmetrics/cmt_info.h",
    "lib/ctraces/include/ctraces/ctr_info.h", "lib/monkey/monkey.service",
    "tests/internal/flb_tests_internal.h",
}


def tree_digest(root, build):
    root = pathlib.Path(root)
    rows = []
    for path in sorted(root.rglob("*")):
        relative = path.relative_to(root)
        if relative.parts[0] in (".git", "build") or (path.is_dir() and not path.is_symlink()):
            continue
        if path.is_symlink():
            rows.append(f"L {relative} {os.readlink(path)}")
        elif str(relative) in CONFIGURE_GENERATED:
            text = path.read_text().replace(str(root), "SRC").replace(str(build), "BLD")
            text = re.sub(r'(#define FLB_GIT_HASH\s+)"[0-9a-f]{40}"', r'\1"GIT"', text)
            rows.append(f"G {relative} {hashlib.sha256(text.encode()).hexdigest()}")
        else:
            rows.append(f"F {relative} {sha(path)}")
    return hashlib.sha256("\n".join(rows).encode()).hexdigest(), len(rows)


def main() -> int:
    pinned_source, private_source, private_build, out = map(lambda p: pathlib.Path(p).resolve(), sys.argv[1:5])
    out.mkdir(parents=True, exist_ok=True)
    head = (pinned_source / ".git/HEAD").read_text().strip()
    if head != PIN:
        raise SystemExit(f"pinned checkout HEAD {head} is not {PIN}")
    pinned_tree, pinned_files = tree_digest(pinned_source, pinned_source.parent / "fluent-bit-clang-production")
    private_tree, private_files = tree_digest(private_source, private_build)
    if pinned_tree != private_tree:
        raise SystemExit("private source copy differs from the pinned checkout")
    flb_version = (private_source / "include/fluent-bit/flb_version.h").read_text()
    build = {"schema_version": 1, "fluent_bit_revision": head, "source_tree_sha256": pinned_tree,
             "source_files": pinned_files, "configure_generated_normalized": sorted(CONFIGURE_GENERATED),
             "private_flb_git_hash": re.search(r'FLB_GIT_HASH\s+"([0-9a-f]+)"', flb_version).group(1),
             "wrapped": WRAPPED, "descriptor_universe": DESCRIPTOR_UNIVERSE,
             "clang": run(["clang", "--version"], capture_output=True).stdout, "targets": {}}
    observer = out / "descriptor_observer.o"
    command = ["clang", "-std=gnu17", "-g", "-O1", "-fPIC", "-Wall", "-Wextra", "-Werror",
               "-Wno-unused-parameter", "-c", str(HARNESS / "descriptor_observer.c"), "-o", str(observer)]
    run(command)
    build["observer"] = {"command": command, "source_sha256": sha(HARNESS / "descriptor_observer.c"),
                         "object_sha256": sha(observer)}
    # Fixture plugin compiled with the pinned out_null include/define profile.
    database = json.loads((private_build / "compile_commands.json").read_text())
    entry = next(e for e in database if e["file"].endswith("/plugins/out_null/null.c"))
    flags = [a for a in shlex.split(entry["command"])[1:] if a.startswith(("-I", "-D", "-std="))]
    plugin = out / "flb-out_qdesc.so"
    command = ["clang", *flags, "-g", "-fPIC", "-shared", str(HARNESS / "flb_out_qdesc_plugin.c"), "-o", str(plugin)]
    run(command, cwd=entry["directory"])
    build["fixture_plugin"] = {"command": command, "source_sha256": sha(HARNESS / "flb_out_qdesc_plugin.c"),
                               "object_sha256": sha(plugin), "profile_entry": entry["file"]}
    # Monkey socket driver compiled with mk_socket.c's pinned profile.
    entry = next(e for e in database if e["file"].endswith("/lib/monkey/mk_server/mk_socket.c"))
    flags = [a for a in shlex.split(entry["command"])[1:] if a.startswith(("-I", "-D", "-std="))]
    driver = out / "monkey_socket_driver.o"
    command = ["clang", *flags, "-g", "-Wall", "-Werror", "-c", str(HARNESS / "monkey_socket_driver.c"),
               "-o", str(driver)]
    run(command, cwd=entry["directory"])
    build["monkey_socket_driver"] = {"command": command, "source_sha256": sha(HARNESS / "monkey_socket_driver.c"),
                                     "object_sha256": sha(driver), "profile_entry": entry["file"]}
    # ctraces random driver compiled with ctr_random.c's pinned profile (Phase B).
    entry = next(e for e in database if e["file"].endswith("/lib/ctraces/src/ctr_random.c"))
    flags = [a for a in shlex.split(entry["command"])[1:] if a.startswith(("-I", "-D", "-std="))]
    random_driver = out / "ctraces_random_driver.o"
    command = ["clang", *flags, "-g", "-Wall", "-Werror", "-c", str(HARNESS / "ctraces_random_driver.c"),
               "-o", str(random_driver)]
    run(command, cwd=entry["directory"])
    build["ctraces_random_driver"] = {"command": command,
                                      "source_sha256": sha(HARNESS / "ctraces_random_driver.c"),
                                      "object_sha256": sha(random_driver), "profile_entry": entry["file"]}
    targets = dict(TARGETS, **{"monkey-socket-driver": str(driver),
                               "ctraces-random-driver": str(random_driver)})
    link = (private_build / "src/CMakeFiles/fluent-bit-bin.dir/link.txt").read_text().split()
    wraps = ["-Wl,--wrap=" + name for name in WRAPPED]
    for name, test_object in targets.items():
        args = []
        for arg in link:
            if arg.startswith("--dependency-file"):
                args.append("--dependency-file=" + str(out / (name + ".link.d")))
                continue
            if arg.strip('"').endswith(("fluent-bit.c.o", "flb_dump.c.o")):
                if arg.strip('"').endswith("fluent-bit.c.o"):
                    source = "CMakeFiles/fluent-bit-bin.dir/fluent-bit.c.o" if test_object is None else None
                    if source:
                        args.append(str(private_build / "src" / source))
                        args.append(str(private_build / "src/CMakeFiles/fluent-bit-bin.dir/flb_dump.c.o"))
                    else:
                        args.append(str(private_build / test_object))
                continue
            if arg == "-o":
                args.append(arg)
                continue
            if arg == "../bin/fluent-bit":
                args.append(str(out / name))
                continue
            args.append(arg)
        command = [args[0], *wraps, str(observer), *args[1:]]
        run(command, cwd=private_build / "src")
        binary = out / name
        imported = set(run(["nm", "-D", "--undefined-only", "--format=just-symbols", str(binary)],
                           capture_output=True).stdout.split())
        imported = {symbol.split("@")[0] for symbol in imported}
        referenced = sorted(imported & set(DESCRIPTOR_UNIVERSE))
        gaps = sorted(set(referenced) - set(WRAPPED))
        build["targets"][name] = {"command": command, "binary_sha256": sha(binary),
                                  "test_object": test_object,
                                  "test_object_sha256": sha(private_build / test_object) if test_object else None,
                                  "descriptor_apis_imported": referenced, "unwrapped_descriptor_apis": gaps}
    # libc probe: exercises declared libc callees Fluent Bit's paths never reach.
    probe = out / "libc-descriptor-probe"
    command = ["clang", "-std=gnu17", "-g", "-O0", "-Wall", "-Wextra", "-Werror", *wraps, str(observer),
               str(HARNESS / "libc_descriptor_probe.c"), "-ldl", "-o", str(probe)]
    run(command)
    imported = {symbol.split("@")[0] for symbol in run(
        ["nm", "-D", "--undefined-only", "--format=just-symbols", str(probe)], capture_output=True).stdout.split()}
    build["targets"]["libc-descriptor-probe"] = {
        "command": command, "binary_sha256": sha(probe), "source_sha256": sha(HARNESS / "libc_descriptor_probe.c"),
        "descriptor_apis_imported": sorted(imported & set(DESCRIPTOR_UNIVERSE)),
        "unwrapped_descriptor_apis": sorted((imported & set(DESCRIPTOR_UNIVERSE)) - set(WRAPPED))}
    (out / "build.json").write_text(json.dumps(build, indent=2) + "\n")
    print(json.dumps({name: target["unwrapped_descriptor_apis"] for name, target in build["targets"].items()}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
