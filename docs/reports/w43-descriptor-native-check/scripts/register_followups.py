#!/usr/bin/env python3
"""Append this card's registered follow-ups to the acceptance matrix ledger.

Usage: register_followups.py MATRIX_JSON

Idempotent: an id already present is left unchanged. `named_followups` is
hand-maintained (build_matrix.py carries it verbatim); regenerate the matrix
afterwards so the derived dimension agrees.
"""
import json
import pathlib
import sys

SOURCE = "docs/reports/w43-descriptor-native-check/README.md"
CARD = "W43-DESCRIPTOR-NATIVE-CHECK"

FOLLOWUPS = [
    ("W43-DESC-NATIVE-PHASE-B",
     "Native descriptor observations are not yet bound to the libc descriptor contracts",
     "Card W43-DESCRIPTOR-NATIVE-CHECK pinned 77 native runs (76 accepted, 6,730 records) of the pinned Fluent Bit "
     "build, Monkey sockets and a libc probe, replayed under library/descriptor_native_observation.qcore. Its Phase B "
     "(check each declared libc descriptor contract of W43-DESCRIPTOR-RESOURCE-TRACKING against every recorded "
     "execution of its callee: sentinel validity, kind, dup/fdopen/fileno relations) could not run because main at "
     "f8b41209 lacks that card. Discharge: on a main containing W43-DESCRIPTOR-RESOURCE-TRACKING, derive each landed "
     "contract's clauses as facts over its callee's pinned records and report every contradiction as a finding, "
     "without weakening the observation."),
    ("W43-DESC-NATIVE-COVERAGE-GAPS",
     "Several descriptor callees and kinds are observed only through the libc probe or not at all",
     "In the W43-DESCRIPTOR-NATIVE-CHECK measurement the observed Fluent Bit paths never call dup, dup2, "
     "fcntl(F_DUPFD), fdopen, fdopendir, dirfd, openat, pipe2, socketpair, eventfd, tmpfile or mremap (executed "
     "only by the libc probe), and nothing calls accept (the build uses accept4), dup3, memfd_create, mkstemp, "
     "popen/pclose or getifaddrs/freeifaddrs, so the process_stream and ifaddrs kinds are unobserved. Targets not "
     "exercised: upstream (client) connections with TLS, c-ares asynchronous DNS sockets, in_tail inotify, in_exec, "
     "librdkafka. References inside shared libraries (libssl, libcurl, libsystemd, libyaml, libsasl2, libc "
     "internals) are not interposed by --wrap. Discharge: a pinned scenario reaching each named callee from a "
     "Fluent Bit path with measured records, or a recorded determination that the pinned build does not reach it; "
     "and an LD_PRELOAD complement or a declared boundary for shared-library callers."),
    ("W43-DESC-FLB-CONFIG-CHANNEL-DOUBLE-CLOSE",
     "flb_config_exit closes the engine manager channel's read descriptor twice, and closes -1 when the engine never started",
     "Observed in every run reaching full engine teardown (11 runs): flb_config_exit closes config->ch_event.fd "
     "(src/flb_config.c:607) and then config->ch_manager[0] (:617), one descriptor generation (flb_engine.c:1201 "
     "stores ch_manager[0] in ch_event), and the second close returns EBADF. When the engine never started, "
     "ch_event.fd is -1 and the nonzero guard calls close(-1) (37 runs). Only EBADF was observed; loss of a "
     "descriptor reused by another thread between the closes is inferred, not observed. Native observation, "
     "source-confirmed; not a static verdict. Discharge: an upstream change that closes the generation once "
     "(pinned rerun showing release_at_most_once and operand_identity_valid hold), or an explicit decision to "
     "leave it unreported."),
    ("W43-DESC-FLB-SCHED-TIMER-DOUBLE-CLOSE",
     "flb_sched_destroy closes every scheduler timer descriptor twice",
     "Observed 38 times in 10 runs: flb_sched_destroy (src/flb_scheduler.c:843) calls mk_event_timeout_destroy on "
     "each timer, which closes its descriptor, then flb_sched_timer_destroy -> flb_sched_timer_cb_disable closes "
     "it again because timer->timer_fd was never reset (mk_event_epoll.c:350); the second close returns EBADF. It "
     "runs on output worker threads while other threads are live, so a reuse race is possible but was not "
     "observed. Native observation, source-confirmed; not a static verdict. Discharge: an upstream change and a "
     "pinned rerun with release_at_most_once holding, or an explicit decision to leave it unreported."),
    ("W43-DESC-FLB-STORAGE-SCAN-DOUBLE-FREE",
     "A storage root that cannot be read makes Fluent Bit double-free its ChunkIO context at shutdown",
     "Surfaced by W43-DESCRIPTOR-NATIVE-CHECK's injected opendir failure and reproduced without injection "
     "(storage.path directory with mode 0300): glibc aborts with 'free(): double free detected' in cio_destroy <- "
     "flb_storage_destroy (src/flb_storage.c:1000) <- flb_engine_shutdown. flb_storage_create's cio_load failure "
     "branch (src/flb_storage.c:727-732) destroys ctx->cio without clearing it, unlike the storage_backlog branch "
     "below it. Heap lifetime, outside the descriptor scope; the aborted run is kept as a rejected run. "
     "Discharge: an upstream fix with the pinned injection scenario accepted on rerun, or a recorded decision on "
     "routing it upstream."),
    ("W43-DESC-MONKEY-BIND-FAILURE-LEAK",
     "mk_socket_server leaks its socket when bind or listen fails",
     "The Monkey socket driver of W43-DESCRIPTOR-NATIVE-CHECK called the actual mk_socket_server on a listening "
     "port: bind fails with EADDRINUSE and the socket from mk_socket_create stays live at exit; the failure path "
     "(lib/monkey/mk_server/mk_socket.c:336-340) returns without closing it. No pinned Fluent Bit configuration "
     "reaches Monkey's server sockets, so this is a Monkey library error path. Discharge: an upstream Monkey fix "
     "with the pinned driver scenario showing no live socket at exit, or a recorded decision."),
    ("W43-DESC-FLB-PROXY-HANDLE-UNRELEASED",
     "A proxy plugin whose registration fails keeps its dl handle for the life of the process",
     "fluent-bit -e with a DSO lacking the flb- prefix routes to flb_plugin_proxy_create, which dlopens it and "
     "ignores flb_plugin_proxy_register's result; the proxy stays on config->proxies, which flb_config_exit never "
     "walks, so the handle is live at exit (observed once). Harmless for the CLI; for an embedding that calls "
     "flb_destroy and continues, the handle and proxy allocations would outlive the context (inferred). "
     "Discharge: a teardown of config->proxies or a refusal on registration failure, observed releasing the "
     "handle; or a decision that the process-lifetime handle is intended."),
    ("W43-DESC-OBSERVER-ORDERING-BOUNDARY",
     "The native descriptor observer can miss a multithreaded use after release and cannot tell the intended generation on reuse",
     "descriptor_observer.c records uses before the call and releases under its lock, so a use recorded before a "
     "release but executed after it is missed (false negative, never false positive); poll/select descriptor "
     "arrays and raw syscalls other than the recorded descriptor numbers are not observed; and, as with "
     "CT50-PROOF-ADDRESS-REUSE-UNPROVED, a reused number or address is a new generation whose intended owner the "
     "records cannot show. Discharge: a stronger ordering (for example per-descriptor sequencing across the "
     "call), interposition of the remaining use channels, or an explicit decision that these stay declared "
     "boundaries of the native evidence."),
]


def main() -> int:
    path = pathlib.Path(sys.argv[1])
    matrix = json.loads(path.read_text())
    present = {entry["id"] for entry in matrix["named_followups"]}
    added = []
    for identifier, title, why in FOLLOWUPS:
        if identifier in present:
            continue
        matrix["named_followups"].append({"id": identifier, "title": title, "why": why, "source": SOURCE,
                                          "recorded_by": CARD})
        added.append(identifier)
    path.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(json.dumps({"added": added, "total": len(matrix["named_followups"])}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
