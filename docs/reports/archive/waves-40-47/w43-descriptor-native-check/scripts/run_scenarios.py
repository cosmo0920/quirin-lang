#!/usr/bin/env python3
"""Execute the pinned descriptor-observation scenarios and replay each run.

Usage: run_scenarios.py TARGETS_DIR OUT_DIR [--only SUBSTRING ...]

TARGETS_DIR is build_targets.py's output (observed binaries, build.json,
fixture plugin). OUT_DIR must not exist. Every scenario runs in its own
process with a fresh record file; each record file is replayed by
`check_descriptor_observations` under the repository policy. Writes
OUT_DIR/runs/<scenario>/{records.jsonl,stdout,stderr,run.json,report.json},
OUT_DIR/sites.json (symbolized call sites) and OUT_DIR/summary.json.

A scenario whose process does not exit by itself after SIGTERM, or whose
records the checker rejects, is reported as not evidence; nothing is
silently dropped. Injections are site-targeted: each names the function
whose call must fail, and the run is flagged when no injected record lands
inside that function.
"""
import argparse
import json
import os
import pathlib
import shutil
import signal
import socket
import subprocess
import sys
import tempfile
import time
import urllib.request

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
POLICY = REPO / "providers/quirin-provider-shared/contracts/library/descriptor_native_observation.qcore"
CHECKER = REPO / "target/release/examples/check_descriptor_observations"

EMFILE, EACCES, ENOENT, ENOMEM, ECONNABORTED, EAFNOSUPPORT = 24, 13, 2, 12, 103, 97
EAI_NONAME = -2


def free_port():
    with socket.socket() as probe:
        probe.bind(("127.0.0.1", 0))
        return probe.getsockname()[1]


def wait_port(port, process, timeout=15.0):
    deadline = time.time() + timeout
    while time.time() < deadline:
        if process.poll() is not None:
            return False
        try:
            with socket.create_connection(("127.0.0.1", port), timeout=0.5):
                return True
        except OSError:
            time.sleep(0.1)
    return False


def tcp_send(port, payload):
    with socket.create_connection(("127.0.0.1", port), timeout=5) as client:
        client.sendall(payload)


def http_get(port, path):
    with urllib.request.urlopen(f"http://127.0.0.1:{port}{path}", timeout=5) as response:
        return response.status


def function_ranges(binary):
    output = subprocess.run(["nm", "-S", "--defined-only", str(binary)], check=True, text=True,
                            capture_output=True).stdout
    ranges = {}
    for line in output.splitlines():
        parts = line.split()
        if len(parts) == 4 and parts[2] in ("t", "T"):
            ranges.setdefault(parts[3], (int(parts[0], 16), int(parts[0], 16) + int(parts[1], 16)))
    return ranges


def acutest_names(binary):
    output = subprocess.run([str(binary), "--list"], check=True, text=True, capture_output=True).stdout
    return [line.strip() for line in output.splitlines()[1:] if line.startswith("  ")]


def engine_config(run_dir, service="", inputs="", outputs=""):
    text = "[SERVICE]\n    Flush 1\n    Grace 1\n    Log_Level info\n" + service + inputs + outputs
    path = run_dir / "fluent-bit.conf"
    path.write_text(text)
    return path


def scenarios(targets):
    plugin = targets / "flb-out_qdesc.so"
    runs = []

    def test(binary, name, inject=None, note=""):
        runs.append({"name": f"test.{binary.removeprefix('flb-it-')}.{name}" + (f".inject-{inject[0]}" if inject else ""),
                     "target": binary, "argv": ["--no-exec", name], "inject": inject, "note": note,
                     "kind": "upstream_test", "timeout": 60})

    for binary in ("flb-it-pipe", "flb-it-file", "flb-it-network", "flb-it-fstore", "flb-it-utils"):
        for name in acutest_names(targets / binary):
            test(binary, name)
    # Injected constructor failures on upstream tests that handle them.
    test("flb-it-network", "accept_empty_nonblocking_listener", ("socket", 1, EMFILE, "flb_net_socket_create"),
         "listener socket creation fails; the test returns early")
    test("flb-it-network", "accept_empty_nonblocking_listener", ("getaddrinfo", 1, EAI_NONAME, "flb_net_server"),
         "listener address resolution fails")
    test("flb-it-network", "ipv6_bracketed_listen", ("socket", 1, EAFNOSUPPORT, "flb_net_socket_create"),
         "IPv6 unsupported branch of the upstream test")
    test("flb-it-fstore", "all", ("open", 1, EACCES, "cio_file_native_open"),
         "chunk file open fails")
    test("flb-it-fstore", "all", ("mmap", 1, ENOMEM, "cio_file_native_map"),
         "chunk file mapping fails")

    # libc probe: every declared callee on a success and a natural-failure
    # path, then once per injected constructor failure.
    runs.append({"name": "probe.libc", "target": "libc-descriptor-probe", "argv": ["{run}"], "inject": None,
                 "note": "direct libc callee executions", "kind": "probe", "timeout": 30})
    for call, error in (("open", EACCES), ("openat", EACCES), ("socket", EMFILE), ("accept4", ECONNABORTED),
                        ("dup", EMFILE), ("fcntl_dupfd", EMFILE), ("pipe", EMFILE), ("pipe2", EMFILE),
                        ("socketpair", EMFILE), ("epoll_create1", EMFILE), ("eventfd", EMFILE),
                        ("fopen", EACCES), ("fdopen", ENOMEM), ("tmpfile", EMFILE), ("opendir", EACCES),
                        ("fdopendir", ENOMEM), ("dlopen", ENOENT), ("getaddrinfo", -11), ("mmap", ENOMEM)):
        runs.append({"name": f"probe.libc.inject-{call}", "target": "libc-descriptor-probe", "argv": ["{run}"],
                     "inject": (call, 1, error, None), "note": "injected failure of the first call",
                     "kind": "probe", "timeout": 30})

    # Monkey server sockets: the actual mk_socket.c functions, called directly.
    def monkey(scenario, inject=None, note=""):
        runs.append({"name": f"monkey.{scenario}" + (f".inject-{inject[0]}" if inject else ""),
                     "target": "monkey-socket-driver", "argv": [scenario, "."], "inject": inject,
                     "note": note, "kind": "probe", "timeout": 30})

    monkey("tcp", note="server, connect, accept, peer address, close; then an empty nonblocking accept")
    monkey("unix", note="mk_socket_open to a listening and to a missing Unix socket")
    monkey("bind_in_use", note="second mk_socket_server on a listening port: bind fails with EADDRINUSE")
    monkey("tcp", ("socket", 1, EMFILE, "mk_socket_create"), "listener socket creation fails")
    monkey("tcp", ("getaddrinfo", 1, EAI_NONAME, "mk_socket_server"), "listener address resolution fails")
    monkey("tcp", ("accept4", 1, ECONNABORTED, "mk_socket_accept"), "first accept fails")

    # Phase B: the actual ctr_random_get (lib/ctraces/src/ctr_random.c), called directly.
    for scenario, note in (("stdin_open", "open(/dev/urandom) with descriptor 0 inherited and live"),
                           ("stdin_closed", "descriptor 0 closed first: open returns 0, the if (fd > 0) guard")):
        runs.append({"name": f"ctraces.random.{scenario}", "target": "ctraces-random-driver",
                     "argv": [scenario], "inject": None, "note": note, "kind": "probe", "timeout": 30})

    def engine(name, argv=None, service="", inputs="", outputs="", action=None, inject=None,
               note="", expect_exit_alone=False, duration=2.5, close_stdin=False):
        runs.append({"name": f"engine.{name}" + (f".inject-{inject[0]}" if inject else ""),
                     "target": "fluent-bit", "argv": argv, "config": (service, inputs, outputs),
                     "action": action, "inject": inject, "note": note, "kind": "engine",
                     "expect_exit_alone": expect_exit_alone, "duration": duration, "timeout": 40,
                     "close_stdin": close_stdin})

    null_out = "[OUTPUT]\n    Name null\n    Match *\n"
    dummy_in = "[INPUT]\n    Name dummy\n    Rate 5\n"
    engine("dummy_null", inputs=dummy_in, outputs=null_out)
    # Phase B: in_event_type's traces call ctr_id_create_random -> ctr_random_get.
    traces_in = "[INPUT]\n    Name event_type\n    Type traces\n"
    engine("event_type_traces", inputs=traces_in, outputs=null_out,
           note="trace generation reaches ctr_random_get's open(/dev/urandom)")
    engine("event_type_traces_stdin_closed", inputs=traces_in, outputs=null_out, close_stdin=True,
           note="the same with descriptor 0 closed at spawn, so open can return 0")
    engine("tcp_filesystem_storage",
           service="    storage.path {run}/storage\n    storage.sync normal\n",
           inputs="[INPUT]\n    Name tcp\n    Listen 127.0.0.1\n    Port {port}\n    Format none\n    storage.type filesystem\n",
           outputs=null_out, action="tcp")
    engine("http_server_monkey",
           service="    HTTP_Server On\n    HTTP_Listen 127.0.0.1\n    HTTP_Port {port}\n",
           inputs=dummy_in, outputs=null_out, action="http")
    qdesc_out = "[OUTPUT]\n    Name qdesc\n    Match *\n"
    engine("dynamic_plugin", argv=["-e", str(plugin), "-c", "{config}"], inputs=dummy_in, outputs=qdesc_out)
    engine("dynamic_plugin_twice", argv=["-e", str(plugin), "-e", str(plugin), "-c", "{config}"],
           inputs=dummy_in, outputs=qdesc_out, note="the same DSO loaded twice: counted dl references")
    engine("plugin_invalid_name", argv=["-e", "{run}/qdesc-invalid.so", "-c", "{config}"],
           inputs=dummy_in, outputs=null_out,
           note="a name without the flb- prefix routes to the Go proxy loader: dlopen succeeds, "
                "registration fails without an error and the engine runs on")
    engine("plugin_missing", argv=["-e", "{run}/flb-out_missing.so", "-c", "{config}"],
           inputs=dummy_in, outputs=null_out, note="dlopen fails", expect_exit_alone=True)
    # Injected failures in the engine.
    engine("tcp_filesystem_storage",
           service="    storage.path {run}/storage\n",
           inputs="[INPUT]\n    Name tcp\n    Listen 127.0.0.1\n    Port {port}\n    Format none\n",
           outputs=null_out, inject=("socket", 1, EMFILE, "flb_net_socket_create"),
           note="tcp listener socket fails; input initialization fails", expect_exit_alone=True)
    engine("tcp_filesystem_storage",
           service="    storage.path {run}/storage\n",
           inputs="[INPUT]\n    Name tcp\n    Listen 127.0.0.1\n    Port {port}\n    Format none\n    storage.type filesystem\n",
           outputs=null_out, action="tcp", inject=("accept4", 1, ECONNABORTED, "flb_net_accept"),
           note="first accept fails; the pending connection is accepted later")
    engine("tcp_filesystem_storage",
           service="    storage.path {run}/storage\n",
           inputs="[INPUT]\n    Name tcp\n    Listen 127.0.0.1\n    Port {port}\n    Format none\n    storage.type filesystem\n",
           outputs=null_out, action="tcp", inject=("open", 1, EACCES, "cio_file_native_open"),
           note="first chunk file open fails")
    engine("tcp_filesystem_storage",
           service="    storage.path {run}/storage\n",
           inputs="[INPUT]\n    Name tcp\n    Listen 127.0.0.1\n    Port {port}\n    Format none\n    storage.type filesystem\n",
           outputs=null_out, action="tcp", inject=("mmap", 1, ENOMEM, "cio_file_native_map"),
           note="first chunk mapping fails")
    engine("tcp_filesystem_storage",
           service="    storage.path {run}/storage\n",
           inputs="[INPUT]\n    Name tcp\n    Listen 127.0.0.1\n    Port {port}\n    Format none\n    storage.type filesystem\n",
           outputs=null_out, action="tcp", inject=("opendir", 1, EACCES, None),
           note="first storage directory scan fails")
    engine("dummy_null", inputs=dummy_in, outputs=null_out, inject=("epoll_create1", 1, EMFILE, None),
           note="first event loop creation fails", expect_exit_alone=True)
    engine("dummy_null", inputs=dummy_in, outputs=null_out, inject=("pipe", 1, EMFILE, None),
           note="first pipe creation fails", expect_exit_alone=True)
    engine("dynamic_plugin", argv=["-e", str(plugin), "-c", "{config}"], inputs=dummy_in, outputs=qdesc_out,
           inject=("dlopen", 1, ENOENT, "get_handle"), note="plugin dlopen fails", expect_exit_alone=True)
    engine("http_server_monkey",
           service="    HTTP_Server On\n    HTTP_Listen 127.0.0.1\n    HTTP_Port {port}\n",
           inputs=dummy_in, outputs=null_out, action="http", inject=("accept4", 1, ECONNABORTED, None),
           note="first Monkey accept fails")
    return runs


def execute(run, targets, out, ranges, env_base):
    run_dir = out / "runs" / run["name"]
    run_dir.mkdir(parents=True)
    records = run_dir / "records.jsonl"
    env = dict(env_base, QCORE_DESC_OUT=str(records))
    binary = targets / run["target"]
    port = free_port()
    injection = None
    if run["inject"]:
        call, index, error, function = run["inject"]
        spec = f"{call}:{index}:{error}"
        if function:
            lo, hi = ranges[run["target"]][function]
            spec += f":{lo}:{hi}"
        env["QCORE_DESC_INJECT"] = spec
        injection = {"call": call, "index": index, "error": error, "function": function, "spec": spec}
    if run["kind"] == "upstream_test":
        argv = [str(binary), *run["argv"]]
    elif run["kind"] == "probe":
        argv = [str(binary), *[arg.replace("{run}", str(run_dir)) for arg in run["argv"]]]
    else:
        service, inputs, outputs = (part.replace("{run}", str(run_dir)).replace("{port}", str(port))
                                    for part in run["config"])
        config = str(engine_config(run_dir, service, inputs, outputs))
        if run["argv"] is None:
            argv = [str(binary), "-c", config]
        else:
            if "invalid" in run["name"]:
                shutil.copy(targets / "flb-out_qdesc.so", run_dir / "qdesc-invalid.so")
            argv = [str(binary), *[arg.replace("{run}", str(run_dir)).replace("{config}", config)
                                   for arg in run["argv"]]]
    started = time.time()
    with open(run_dir / "stdout", "wb") as stdout, open(run_dir / "stderr", "wb") as stderr:
        # A run with `close_stdin` starts with descriptor 0 closed (as a service
        # launched without standard input); every other run inherits the runner's.
        spawn = {"preexec_fn": lambda: os.close(0)} if run.get("close_stdin") else {}
        process = subprocess.Popen(argv, cwd=run_dir, env=env, stdout=stdout, stderr=stderr, **spawn)
        action_result = None
        terminated = False
        try:
            if run["kind"] == "engine":
                if run.get("action") in ("tcp", "http"):
                    ready = wait_port(port, process)
                    action_result = {"port_ready": ready}
                    if ready and run["action"] == "tcp":
                        for index in range(3):
                            tcp_send(port, b"line %d\n" % index * 20)
                        action_result["sent_connections"] = 3
                    if ready and run["action"] == "http":
                        statuses = []
                        for path in ("/", "/api/v1/uptime", "/api/v1/metrics"):
                            try:
                                statuses.append(http_get(port, path))
                            except Exception as error:  # recorded, not hidden
                                statuses.append(f"error: {error}")
                        action_result["http"] = statuses
                deadline = time.time() + run["duration"]
                while time.time() < deadline and process.poll() is None:
                    time.sleep(0.1)
                if process.poll() is None:
                    process.send_signal(signal.SIGTERM)
                    terminated = True
            returncode = process.wait(timeout=run["timeout"])
            timed_out = False
        except subprocess.TimeoutExpired:
            process.kill()
            returncode = process.wait()
            timed_out = True
    record = {"name": run["name"], "kind": run["kind"], "target": run["target"], "argv": argv,
              "port": port, "injection": injection, "note": run.get("note", ""), "sigterm_sent": terminated,
              "expected_exit_without_signal": run.get("expect_exit_alone", False),
              "stdin_closed_at_spawn": run.get("close_stdin", False),
              "returncode": returncode, "timed_out": timed_out, "action": action_result,
              "seconds": round(time.time() - started, 2)}
    checked = subprocess.run([str(CHECKER), str(POLICY), str(records)], text=True, capture_output=True)
    record["checker_exit"] = checked.returncode
    if checked.returncode == 0:
        (run_dir / "report.json").write_text(checked.stdout)
    else:
        record["checker_error"] = checked.stderr.strip()[-2000:]
    if injection and records.exists():
        landed = []
        for line in records.read_text().splitlines():
            row = json.loads(line)
            if row.get("injected"):
                landed.append({"sequence": row["sequence"], "call": row["call"], "offset": row["site"]["offset"]})
        record["injected_records"] = landed
        if injection["function"]:
            lo, hi = ranges[run["target"]][injection["function"]]
            record["injection_reached_function"] = any(lo <= row["offset"] < hi for row in landed)
        else:
            record["injection_reached_function"] = bool(landed)
    (run_dir / "run.json").write_text(json.dumps(record, indent=2) + "\n")
    return record


def symbolize(out, targets, source_root):
    wanted = {}
    for records in (out / "runs").glob("*/records.jsonl"):
        for line in records.read_text().splitlines():
            row = json.loads(line)
            for site in [row["site"], *row.get("stack", [])]:
                if site["module"] != "observer":
                    wanted.setdefault(site["module"], set()).add(site["offset"])
    sites = {}
    for module, offsets in sorted(wanted.items()):
        binary = targets / module
        ordered = sorted(offsets)
        if not binary.exists():
            for offset in ordered:
                sites[f"{module}:{offset}"] = {"function": "?", "location": "?"}
            continue
        text = subprocess.run(["llvm-symbolizer", f"--obj={binary}", *[str(max(offset - 1, 0)) for offset in ordered]],
                              check=True, text=True, capture_output=True).stdout
        blocks = [block.splitlines() for block in text.strip().split("\n\n")]
        for offset, block in zip(ordered, blocks):
            # Inlined frames print innermost first; keep the full chain.
            frames = [{"function": block[i], "location": block[i + 1].replace(str(source_root) + "/", "").replace(str(REPO) + "/", "")}
                      for i in range(0, len(block) - 1, 2)]
            sites[f"{module}:{offset}"] = frames[0] | {"inlined_into": frames[1:]}
    (out / "sites.json").write_text(json.dumps(sites, indent=1, sort_keys=True) + "\n")
    return sites


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("targets", type=pathlib.Path)
    parser.add_argument("out", type=pathlib.Path)
    parser.add_argument("--only", action="append", default=[])
    parser.add_argument("--source-root", type=pathlib.Path, required=True)
    args = parser.parse_args()
    targets, out = args.targets.resolve(), args.out.resolve()
    out.mkdir(parents=True, exist_ok=False)
    if not CHECKER.exists():
        raise SystemExit(f"build the checker first: {CHECKER}")
    ranges = {name: function_ranges(targets / name) for name in
              ("fluent-bit", "flb-it-pipe", "flb-it-network", "flb-it-fstore", "flb-it-utils", "flb-it-file",
               "libc-descriptor-probe", "monkey-socket-driver", "ctraces-random-driver")}
    env_base = {key: value for key, value in os.environ.items() if not key.startswith("QCORE_")}
    env_base["LC_ALL"] = "C"
    tmp = tempfile.mkdtemp(prefix="w43-desc-", dir=str(out))
    env_base["TMPDIR"] = tmp
    results = []
    for run in scenarios(targets):
        if args.only and not any(fragment in run["name"] for fragment in args.only):
            continue
        result = execute(run, targets, out, ranges, env_base)
        results.append(result)
        print(json.dumps({k: result.get(k) for k in ("name", "returncode", "timed_out", "checker_exit",
                                                      "injection_reached_function")}), flush=True)
    symbolize(out, targets, args.source_root.resolve())
    shutil.rmtree(tmp, ignore_errors=True)
    (out / "summary.json").write_text(json.dumps({"schema_version": 1, "runs": results}, indent=2) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
