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

"""Produce one attributable Linux row for the Cycle 18 platform matrix."""

import argparse
import hashlib
import json
import os
import pathlib
import platform
import shutil
import subprocess

REVISION = "d82464a277d0f42703702c4dfd9af6af38595a83"


def digest_bytes(data):
    return hashlib.sha256(data).hexdigest()


def digest(path):
    return digest_bytes(path.read_bytes())


def run(command, *, env=None, capture=False):
    result = subprocess.run(
        command,
        check=True,
        text=True,
        env=env,
        stdout=subprocess.PIPE if capture else None,
        stderr=subprocess.PIPE if capture else None,
    )
    return result.stdout.strip() if capture else ""


def history(result, history_digest):
    return {
        "source_document_sha256": history_digest,
        "source_revision": REVISION,
        "recorded_result": result,
    }


def dimensions(host, backend, sanitizers, threads):
    return {
        "host": host,
        "backend": backend,
        "sanitizers": sanitizers,
        "thread_adapter": threads,
        "observer": "bounded_generic_libevent_probe",
    }


def unknown(host, backend, sanitizers, threads, reason, prior, history_digest):
    return {
        "dimensions": dimensions(host, backend, sanitizers, threads),
        "status": "unknown",
        "reason": reason,
        "evidence": None,
        "historical_native_result": history(prior, history_digest) if prior else None,
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=pathlib.Path, required=True)
    parser.add_argument("--compiler", default="clang")
    args = parser.parse_args()
    root = pathlib.Path(__file__).resolve().parents[1]
    source = root / "third_party/libevent"
    harness = root / "crates/qcore-audit/tests/fixtures/native_platform_probe.c"
    history_doc = root / "docs/cycles/CYCLE_18_LIBEVENT_VALIDATION.md"
    output = args.output.resolve()
    build = output / "libevent-build"
    output.mkdir(parents=True, exist_ok=True)
    if run(["git", "-C", str(source), "rev-parse", "HEAD"], capture=True) != REVISION:
        raise SystemExit("pinned Libevent source revision mismatch")
    source_status_command = [
        "git", "-C", str(source), "status", "--porcelain=v1",
        "--untracked-files=all", "--ignored",
    ]
    if run(source_status_command, capture=True):
        raise SystemExit("pinned Libevent source worktree is not clean")
    compiler_path = shutil.which(args.compiler)
    if compiler_path is None:
        raise SystemExit(f"compiler not found: {args.compiler}")
    compiler_version = run([compiler_path, "--version"], capture=True)
    sanitizer_flags = "-fsanitize=address,undefined -fno-omit-frame-pointer"
    configure = [
        "cmake", "-S", str(source), "-B", str(build), "-DCMAKE_BUILD_TYPE=Debug",
        "-DEVENT__LIBRARY_TYPE=STATIC", "-DEVENT__DISABLE_OPENSSL=ON",
        "-DEVENT__DISABLE_MBEDTLS=ON", "-DEVENT__DISABLE_TESTS=ON",
        "-DEVENT__DISABLE_REGRESS=ON", "-DEVENT__DISABLE_BENCHMARK=ON",
        "-DEVENT__DISABLE_SAMPLES=ON", "-DEVENT__DISABLE_THREAD_SUPPORT=OFF",
        "-DCMAKE_EXPORT_COMPILE_COMMANDS=ON",
        f"-DCMAKE_C_COMPILER={compiler_path}", f"-DCMAKE_C_FLAGS={sanitizer_flags}",
        f"-DCMAKE_EXE_LINKER_FLAGS={sanitizer_flags}",
    ]
    run(configure)
    run(["cmake", "--build", str(build), "--parallel", "2", "--target", "event_core_static", "event_pthreads_static"])
    core = build / "lib/libevent_core.a"
    pthreads = build / "lib/libevent_pthreads.a"
    executable = output / "native-platform-probe"
    compile_command = [
        compiler_path, "-std=gnu17", "-Wall", "-Wextra", "-Werror", "-g",
        *sanitizer_flags.split(), "-I", str(build / "include"), "-I", str(source / "include"),
        str(harness), str(pthreads), str(core), "-pthread", "-o", str(executable),
    ]
    run(compile_command)
    build_commands = {"configure": configure, "build": ["cmake", "--build", str(build), "--parallel", "2", "--target", "event_core_static", "event_pthreads_static"], "compile_link": compile_command}
    (output / "build-commands.json").write_text(json.dumps(build_commands, indent=2) + "\n")
    runtimes = {}
    runtime_paths = []
    for expected, disabled in {
        "epoll": ["EVENT_NOPOLL", "EVENT_NOSELECT"],
        "poll": ["EVENT_NOEPOLL", "EVENT_NOSELECT"],
        "select": ["EVENT_NOEPOLL", "EVENT_NOPOLL"],
    }.items():
        runtime_env = os.environ.copy()
        runtime_env["ASAN_OPTIONS"] = "detect_leaks=1:halt_on_error=1"
        runtime_env["UBSAN_OPTIONS"] = "halt_on_error=1:print_stacktrace=1"
        for variable in disabled:
            runtime_env[variable] = "1"
        runtime = json.loads(run([str(executable)], env=runtime_env, capture=True))
        runtime["process_exit"] = 0
        if runtime["backend"] != expected or runtime["dispatch_thread"] == runtime["activating_thread"]:
            raise SystemExit(f"probe did not select {expected} with a distinct activating thread")
        if runtime["callback_thread"] != runtime["dispatch_thread"] or runtime["activation_count"] != 1 or runtime["callback_count"] != 1 or runtime["callback_flags"] != 2:
            raise SystemExit(f"{expected} probe callback/thread accounting mismatch")
        runtime_path = output / f"runtime-{expected}.json"
        runtime_path.write_text(json.dumps(runtime, indent=2) + "\n")
        runtime_paths.append(runtime_path)
        runtimes[expected] = runtime
    tsan_build = output / "libevent-tsan"
    tsan_flags = "-fsanitize=thread"
    tsan_configure = [
        "cmake", "-S", str(source), "-B", str(tsan_build), "-DCMAKE_BUILD_TYPE=Debug",
        "-DEVENT__LIBRARY_TYPE=STATIC", "-DEVENT__DISABLE_OPENSSL=ON",
        "-DEVENT__DISABLE_MBEDTLS=ON", "-DEVENT__DISABLE_TESTS=ON",
        "-DEVENT__DISABLE_REGRESS=ON", "-DEVENT__DISABLE_BENCHMARK=ON",
        "-DEVENT__DISABLE_SAMPLES=ON", "-DEVENT__DISABLE_THREAD_SUPPORT=OFF",
        "-DCMAKE_EXPORT_COMPILE_COMMANDS=ON",
        f"-DCMAKE_C_COMPILER={compiler_path}", f"-DCMAKE_C_FLAGS={tsan_flags}",
        f"-DCMAKE_EXE_LINKER_FLAGS={tsan_flags}",
    ]
    run(tsan_configure)
    tsan_build_command = ["cmake", "--build", str(tsan_build), "--parallel", "2", "--target", "event_core_static", "event_pthreads_static"]
    run(tsan_build_command)
    tsan_core = tsan_build / "lib/libevent_core.a"
    tsan_pthreads = tsan_build / "lib/libevent_pthreads.a"
    tsan_executable = output / "native-platform-probe-tsan"
    tsan_compile = [compiler_path, "-std=gnu17", "-Wall", "-Wextra", "-Werror", "-g", tsan_flags,
                    "-I", str(tsan_build / "include"), "-I", str(source / "include"), str(harness),
                    str(tsan_pthreads), str(tsan_core), "-pthread", "-o", str(tsan_executable)]
    run(tsan_compile)
    tsan_env = os.environ.copy()
    tsan_env.update({"EVENT_NOPOLL":"1", "EVENT_NOSELECT":"1", "TSAN_OPTIONS":"halt_on_error=1"})
    tsan_runtime = json.loads(run([str(tsan_executable)], env=tsan_env, capture=True))
    tsan_runtime["process_exit"] = 0
    if tsan_runtime["backend"] != "epoll" or tsan_runtime["dispatch_thread"] == tsan_runtime["activating_thread"] or tsan_runtime["callback_thread"] != tsan_runtime["dispatch_thread"] or tsan_runtime["callback_flags"] != 2:
        raise SystemExit("ThreadSanitizer probe identity mismatch")
    tsan_commands = {"configure":tsan_configure,"build":tsan_build_command,"compile_link":tsan_compile}
    (output / "build-commands-tsan.json").write_text(json.dumps(tsan_commands, indent=2) + "\n")
    (output / "runtime-epoll-tsan.json").write_text(json.dumps(tsan_runtime, indent=2) + "\n")
    history_digest = digest(history_doc)
    linux_history = "linux_full_native_84_of_84_passed"
    rows = []
    for backend in ["epoll", "poll", "select"]:
        runtime = runtimes[backend]
        rows.append({
            "policy": f"linux_{backend}_asan_profile",
            "dimensions": dimensions("linux", backend, "address,undefined,leak", "libevent_pthreads_initialized"),
            "status": "observed", "reason": None,
            "evidence": {
                "source": {"libevent_revision": REVISION, "harness_sha256": digest(harness)},
                "build": {"compiler_path": compiler_path, "compiler_version": compiler_version,
                          "command_sha256": digest(output / "build-commands.json"),
                          "libevent_core_sha256": digest(core), "libevent_pthreads_sha256": digest(pthreads),
                          "executable_sha256": digest(executable)},
                "runtime": {"system": platform.system(), "machine": platform.machine(), "release": platform.release(),
                            "selected_backend": runtime["backend"], "dispatch_thread": runtime["dispatch_thread"],
                            "activating_thread": runtime["activating_thread"], "callback_thread": runtime["callback_thread"],
                            "process_exit": 0, "activation_count": runtime["activation_count"], "callback_count": runtime["callback_count"], "callback_flags":runtime["callback_flags"]},
                "property": "generic_libevent_cross_thread_activation", "observer_scope": "generic_libevent_only",
                "observer_transparency": "not_claimed",
            },
            "historical_native_result": history(linux_history, history_digest),
        })
    rows += [
        {"policy":"macos_kqueue_asan_profile", **unknown("macos", "kqueue", "address,undefined", "libevent_pthreads_initialized", "missing_current_kqueue_sanitizer_observer_artifact_join", "macos_kqueue_21_of_21_and_full_65_of_65_passed", history_digest)},
        {"policy":"windows_win32_asan_profile", **unknown("windows", "win32", "address", "libevent_windows_threads_initialized", "missing_current_win32_sanitizer_observer_artifact_join", "windows_full_native_44_of_44_passed", history_digest)},
        {"policy":"windows_iocp_asan_profile", **unknown("windows", "iocp_profile", "address", "libevent_windows_threads_initialized", "missing_current_iocp_sanitizer_observer_artifact_join", "windows_iocp_21_of_21_passed", history_digest)},
        {"policy":"linux_epoll_tsan_profile","dimensions":dimensions("linux","epoll","thread","libevent_pthreads_initialized"),
         "status":"observed","reason":None,"historical_native_result":None,
         "evidence":{"source":{"libevent_revision":REVISION,"harness_sha256":digest(harness)},
         "build":{"compiler_path":compiler_path,"compiler_version":compiler_version,"command_sha256":digest(output / "build-commands-tsan.json"),
         "libevent_core_sha256":digest(tsan_core),"libevent_pthreads_sha256":digest(tsan_pthreads),"executable_sha256":digest(tsan_executable)},
         "runtime":{"system":platform.system(),"machine":platform.machine(),"release":platform.release(),"selected_backend":"epoll",
         "dispatch_thread":tsan_runtime["dispatch_thread"],"activating_thread":tsan_runtime["activating_thread"],"callback_thread":tsan_runtime["callback_thread"],
         "process_exit":0,"activation_count":tsan_runtime["activation_count"],"callback_count":tsan_runtime["callback_count"],"callback_flags":tsan_runtime["callback_flags"]},
         "property":"generic_libevent_cross_thread_activation","observer_scope":"generic_libevent_only","observer_transparency":"not_claimed"}},
    ]
    matrix = {"schema_version": 1, "artifact": "libevent/generic-cross-thread-activation", "rows": rows}
    (output / "matrix.json").write_text(json.dumps(matrix, indent=2) + "\n")
    manifest = {"schema_version":1,"status":"completed_linux_native_probe","source_revision":REVISION,"sanitizers":"address,undefined,leak",
                "artifacts_sha256":{"harness":digest(harness),"build_commands":digest(output / "build-commands.json"),"cmake_cache":digest(build / "CMakeCache.txt"),"compile_commands":digest(build / "compile_commands.json"),
                "libevent_core":digest(core),"libevent_pthreads":digest(pthreads),"executable":digest(executable)},
                "runtime_sha256":{path.stem.removeprefix("runtime-"):digest(path) for path in runtime_paths}}
    (output / "run-manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    tsan_manifest={"schema_version":1,"status":"completed_linux_native_probe","source_revision":REVISION,"sanitizers":"thread",
                   "artifacts_sha256":{"harness":digest(harness),"build_commands":digest(output / "build-commands-tsan.json"),"cmake_cache":digest(tsan_build / "CMakeCache.txt"),"compile_commands":digest(tsan_build / "compile_commands.json"),
                   "libevent_core":digest(tsan_core),"libevent_pthreads":digest(tsan_pthreads),"executable":digest(tsan_executable)},
                   "runtime_sha256":{"epoll":digest(output / "runtime-epoll-tsan.json")}}
    (output / "run-manifest-tsan.json").write_text(json.dumps(tsan_manifest, indent=2) + "\n")
    config = {"schema_version":1,"matrix":"matrix.json",
              "policy":str(root / "providers/qcore-provider-libevent/contracts/monkey/native_platform_matrix.qv"),
              "harness":str(harness),"historical_validation":str(history_doc),"libevent_source":str(source),
              "artifact_sets":[
              {"sanitizers":"address,undefined,leak","build_commands":"build-commands.json","cmake_cache":str(build / "CMakeCache.txt"),"compile_commands":str(build / "compile_commands.json"),"libevent_core":str(core),"libevent_pthreads":str(pthreads),
              "executable":str(executable),"runtime_records":[path.name for path in runtime_paths],"run_manifest":"run-manifest.json"},
              {"sanitizers":"thread","build_commands":"build-commands-tsan.json","cmake_cache":str(tsan_build / "CMakeCache.txt"),"compile_commands":str(tsan_build / "compile_commands.json"),"libevent_core":str(tsan_core),"libevent_pthreads":str(tsan_pthreads),
              "executable":str(tsan_executable),"runtime_records":["runtime-epoll-tsan.json"],"run_manifest":"run-manifest-tsan.json"}],
              "report":"checked-report.json"}
    (output / "audit-config.json").write_text(json.dumps(config, indent=2) + "\n")
    if run(["git", "-C", str(source), "rev-parse", "HEAD"], capture=True) != REVISION or run(source_status_command, capture=True):
        raise SystemExit("pinned Libevent source identity changed during measurement")
    print(output / "matrix.json")


if __name__ == "__main__":
    main()
