#!/usr/bin/env python3
"""Write the first evaluation-profile records (card EVALUATION-PROFILES).

Usage: build_profiles.py --runs DIR

Run once from the card worktree. DIR holds the BASE measurement of the three
live routes (`baseline.sh`): corpus/corpus/resource-diagnostics.json and
corpus/corpus-null.jsonl, system/ and slice/ likewise. Every digest of a
frozen profile's input is read from git at the frozen revision; every
figure of a frozen profile is transcribed from the record that published it
(the acceptance matrix row at this card's base, or the cycle document at the
frozen revision). The output is contract/evaluations/<name>/{evaluation.json,
baseline.json}, contract/evaluations/eight-home/link-map.json and
contract/evaluations/bindings.json; `scripts/evaluation_profiles.py
check --history` then checks all of it.
"""
import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / "scripts"))
import evaluation_profiles as ep  # noqa: E402

BASE = "710021bd713b90037e803d4a1f87d95eba6476c9"
REPORT = "docs/reports/evaluation-profiles/README.md"
FLUENT_BIT = "66910c10a4d7eafa810b84229cf2dcf7b0f26f97"
LIBEVENT = "d82464a277d0f42703702c4dfd9af6af38595a83"
LINK_MAP_SOURCE = "docs/reports/w48-bu-single-leaf-bodies/link-map.corpus.json"
LINK_MAP = "contract/evaluations/eight-home/link-map.json"
LIVE_CONTRACTS = {
    "lock_set": {"state": "latest"},
    "references": ["providers/contract-load-path.json",
                   "contract/profiles/fluent-bit/library-providers.json",
                   "contract/profiles/fluent-bit/source-contracts.json"],
    "note": "live: bound to the latest Fluent Bit revision's pin-lock set (decision 9a), the tree's provider contracts, which each provider crate's "
            "lock files pin (CONTRACT-LOCKS); a contract change is reviewed there and moves this "
            "profile's results only through a recorded baseline",
}


def git(*args):
    return subprocess.run(["git", "-C", str(ROOT), *args], check=True,
                          capture_output=True).stdout


def blob_sha(revision, path):
    return hashlib.sha256(git("cat-file", "blob", f"{revision}:{path}")).hexdigest()


def tree_sha(path):
    return hashlib.sha256((ROOT / path).read_bytes()).hexdigest()


def full(revision):
    return git("rev-parse", f"{revision}^{{commit}}").decode().strip()


def semantic_evidence(revision):
    try:
        matrix = json.loads(git("cat-file", "blob", f"{revision}:{ep.MATRIX}"))
        return str(matrix["versions"]["semantic_evidence"])
    except (subprocess.CalledProcessError, KeyError, ValueError):
        return "unrecorded at this revision (before the acceptance matrix)"


def matrix_row(key):
    return json.loads((ROOT / ep.MATRIX).read_text())["corpus"][key]


def write(path, value):
    path = ROOT / path
    path.parent.mkdir(parents=True, exist_ok=True)
    data = (json.dumps(value, indent=2) + "\n").encode()
    path.write_bytes(data)
    return hashlib.sha256(data).hexdigest()


def frozen(name, revision, description, route, source, tracked, untracked, figures, artifacts,
           figures_source, contract_note):
    revision = full(revision)
    baseline_path = f"contract/evaluations/{name}/baseline.json"
    baseline = {"schema": ep.BASELINE_SCHEMA, "profile": name, "measured_revision": revision,
                "source": figures_source, "figures": figures,
                "artifacts": [{"path": path, "sha256": tree_sha(path)} for path in artifacts]}
    record = {
        "schema": ep.PROFILE_SCHEMA, "name": name, "description": description,
        "policy": {"kind": "frozen", "revision": revision},
        "route": route, "source": source,
        "inputs": {"tracked": [{"path": path, "revision": revision,
                                "sha256": blob_sha(revision, path), "role": role}
                               for path, role in tracked],
                   "untracked": untracked},
        "contract_set": {
            "lock_set": {"state": "retired",
                         "evidence_sha256": ep.contract_evidence_digest(ROOT, revision)},
            "references": [],
            "note": contract_note + "; no pin-lock set was retained for this revision (it predates "
                    "the lock-set window of decision 9a), so the contract set is identified by "
                    "its evidence digest at the frozen revision"},
        "baseline": {"path": baseline_path, "sha256": write(baseline_path, baseline)},
        "evidence_version": {"analyzer_revision": revision,
                             "semantic_evidence": semantic_evidence(revision)},
    }
    write(f"contract/evaluations/{name}/evaluation.json", record)


def live(name, description, route, source, tracked, untracked, figures, reason):
    baseline_path = f"contract/evaluations/{name}/baseline.json"
    baseline = {"schema": ep.BASELINE_SCHEMA, "profile": name, "measured_revision": BASE,
                "source": REPORT, "figures": figures, "artifacts": []}
    digest = write(baseline_path, baseline)
    record = {
        "schema": ep.PROFILE_SCHEMA, "name": name, "description": description,
        "policy": {"kind": "live"}, "route": route, "source": source,
        "inputs": {"tracked": [{"path": path, "sha256": tree_sha(path), "role": role}
                               for path, role in tracked],
                   "untracked": untracked},
        "contract_set": LIVE_CONTRACTS,
        "baseline": {"path": baseline_path, "sha256": digest},
        "baseline_history": [{"sha256": digest, "measured_revision": BASE, "reason": reason,
                              "source": REPORT}],
        "evidence_version": {"analyzer_revision": BASE, "semantic_evidence": semantic_evidence(BASE)},
    }
    write(f"contract/evaluations/{name}/evaluation.json", record)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runs", type=Path, required=True)
    args = parser.parse_args()
    runs = args.runs
    (ROOT / LINK_MAP).parent.mkdir(parents=True, exist_ok=True)
    (ROOT / LINK_MAP).write_bytes((ROOT / LINK_MAP_SOURCE).read_bytes())

    first = ("first published baseline of the live profile: BASE 710021bd7 measured by card "
             "EVALUATION-PROFILES with the AUDIT-CANONICAL-PATHS route driver on the original "
             "pinned inputs")
    fb_profile = [("contract/profiles/fluent-bit/profile.json", "source-set pins and audited files"),
                  ("contract/profiles/fluent-bit/member-facts-scope.json",
                   "member-facts pre-pass exclusions")]
    link = [(LINK_MAP, "cross-unit link map (QCORE_CROSS_UNIT_LINK_MAP), byte-identical to "
             + LINK_MAP_SOURCE)]
    live(
        "eight-home",
        "The pinned Fluent Bit corpus (185 units of the fluent-bit source set's audit.json "
        "selection) under the conditional eight-home cross-unit link map, null and outer "
        "projections on, no source contracts or census: the corpus route every W48 card "
        "measures as BASE and TIP.",
        {"kind": "pinned-corpus",
         "command": "QCORE_CROSS_UNIT_LINK_MAP=<link map> QCORE_CLANG=/usr/lib/llvm-21/bin/clang "
                    "QCORE_NULL_PROPERTY=<out>/corpus-null.jsonl QCORE_OUTER_PROJECTION="
                    "<out>/corpus-outer.json python3 scripts/run_pinned_audit.py <out>/corpus "
                    "66910c10 --inputs-root build/cycle19 --jobs N "
                    "(docs/reports/audit-canonical-paths/scripts/measure.py corpus)"},
        {"repository": "fluent-bit", "revision": FLUENT_BIT,
         "submodules": {"third_party/libevent": LIBEVENT},
         "note": "Fluent Bit v5.1.1-127-g66910c10a with its vendored libraries in-tree; the Monkey "
                 "gate reads the Cycle 18 libevent submodule at its pinned revision"},
        [("contract/profiles/fluent-bit/audit.json", "unit selection")] + fb_profile + link,
        [{"path": "build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source/build/compile_commands.json",
          "sha256": "5e6868d9af26eaec97bc8020ed5f23496f2490abf44532443239bd4d365f7e78",
          "role": "Fluent Bit compile database"},
         {"path": "build/cycle19/c-gateway-wave-pin/monkey/monkey-build/compile_commands.json",
          "sha256": "85cc996712bbd35675340b8b863b6ebc716cb94552253f2a6ad0288db920f331",
          "role": "Monkey compile database"},
         {"path": "build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source", "sha256": None,
          "role": "prepared Fluent Bit checkout at the source revision"},
         {"path": "build/cycle19/provider-packages-wave20/qcore.chunkio", "sha256": None,
          "role": "ChunkIO provider package (QCORE_CHUNKIO_PACKAGE)"}],
        ep.summarize(runs / "corpus/corpus/resource-diagnostics.json",
                     runs / "corpus/corpus-null.jsonl"),
        first)
    for name, label, selection, prepared, revision, database_sha, prepare_sha, note in (
            ("fluent-bit-system", "system", "contract/profiles/fluent-bit/system-selection.json",
             "private/w45-fb/w46-external-callee-contracts/attempt-1",
             "046accf6aded7906f66a1baf0202da349e521736",
             "2475e6374b14152538463f35a3457ffa98fbe30b2639d5df916ef9a216cb6876",
             "22b5156af56b3764d7031d6ea6ace7c7f372175384321e6614a1af7c59e97d11",
             "the Fluent Bit revision quirin-audit prepare recorded for the system preparation"),
            ("fluent-bit-tail-pause", "slice", "contract/profiles/fluent-bit/tail-pause-selection.json",
             "private/w47-tail/attempt-1/prep",
             "34155955e4962a56d94eb23c03a002880899a0d1",
             "faf446759b31f892c9ffe03c76719875e9910682515cdd9acc908dc5142f8157",
             "cb80a291f85fd0b985d6512cfacaebb52fb505462642a9910d12629e12c6ac74",
             "the revision quirin-audit prepare recorded for the tail-pause preparation, a clone "
             "of the export of upstream 90978ac6a59e96d19621108f63e115eb4cd1c576 (tree "
             "8d1f3cd78a00fcfb4095a39d3d0b3e7eaac80be4, per tail-pause-selection.json)")):
        live(
            name,
            f"The Fluent Bit {'engine, backlog and retry files (system-coverage census)' if label == 'system' else 'in_tail pause/resume slice'} "
            f"audited through `quirin-audit current` with the explicit selection {selection}, "
            "every distinct compilation configuration kept, under the eight-home link map with "
            "null and outer projections on.",
            {"kind": "configured-selection",
             "command": f"quirin-audit current --checkout {prepared}/fluent-bit-clang-source "
                        "--profile-directory contract/profiles/fluent-bit --compile-database "
                        f"{prepared}/fluent-bit-clang-production/compile_commands.json "
                        f"--selection {selection} --jobs N --report <out>/resource-diagnostics.json "
                        f"(docs/reports/audit-canonical-paths/scripts/measure.py {label})"},
            {"repository": "fluent-bit", "revision": revision, "submodules": {}, "note": note},
            [(selection, "unit selection")] + fb_profile + link,
            [{"path": f"{prepared}/fluent-bit-clang-production/compile_commands.json",
              "sha256": database_sha, "role": "prepared Fluent Bit compile database"},
             {"path": f"{prepared}/prepare.json", "sha256": prepare_sha,
              "role": "quirin-audit prepare provenance"},
             {"path": f"{prepared}/fluent-bit-clang-source", "sha256": None,
              "role": "prepared Fluent Bit checkout at the source revision"}],
            ep.summarize(runs / f"{label}/resource-diagnostics.json",
                         runs / f"{label}/{label}-null.jsonl"),
            first)

    frozen(
        "cycle-17-cfl", "b26b4aa35a17508f2cdb7af1ac147913a33696c8",
        "Cycle 17's unrestricted-C importer probe of the adjacent CFL checkout: "
        "src/cfl_atomic_generic.c entry cfl_atomic_initialize, and src/cfl.c.",
        {"kind": "unrestricted-import-probe",
         "command": "cargo run -p qcore-compiler --example cycle17_legacy_audit -- "
                    "../cfl/src/cfl_atomic_generic.c cfl_atomic_initialize ../cfl/include"},
        {"repository": "cfl", "revision": "6e35cca", "submodules": {},
         "note": "the adjacent CFL checkout (../cfl from the QCore-C root), recorded by its "
                "abbreviated revision only"},
        [("compiler/examples/cycle17_legacy_audit.rs", "probe driver"),
         ("docs/cycles/CYCLE_17_CFL_VALIDATION.md", "the record that published the result")],
        [],
        {"src/cfl_atomic_generic.c": {
            "verdict": "Unknown",
            "diagnostic": "the conservative C11 projection contains an unsupported operation; "
                          "5 partially analyzed function(s)"},
         "src/cfl.c": {"verdict": "Unknown",
                       "diagnostic": "umbrella-header xxHash inline functions exceed the "
                                     "conservative projection before a partial program summary"}},
        [], "docs/cycles/CYCLE_17_CFL_VALIDATION.md",
        "frozen: the importer had no external contracts for CFL; the analyzer at the frozen "
        "revision defines the result")
    frozen(
        "cycle-18-libevent", "bb76178d5ecdc309667863238c75e611f3ff25f4",
        "Cycle 18's pinned-Libevent source-contract audit: the twelve pinned files and their "
        "behaviour markers at the libevent submodule revision.",
        {"kind": "source-contract-audit",
         "command": "cargo run --quiet --example cycle18_libevent_audit"},
        {"repository": "libevent", "revision": LIBEVENT,
         "submodules": {"third_party/libevent": LIBEVENT},
         "note": "release-2.2.2-alpha-11-gd82464a2, the third_party/libevent submodule"},
        [("compiler/examples/cycle18_libevent_audit.rs", "audit driver"),
         ("compiler/tests/libevent_submodule.rs", "the same audit as an integration test"),
         ("docs/cycles/CYCLE_18_LIBEVENT_VALIDATION.md", "the record that published the result")],
        [],
        {"verified": f"Libevent release-2.2.2-alpha-11-gd82464a2 at {LIBEVENT}",
         "checked": ["include/event2/event.h", "event.c", "bufferevent.c",
                     "include/event2/thread.h", "include/event2/util.h", "evutil.c", "epoll.c",
                     "kqueue.c", "win32select.c", "event_iocp.c", "buffer_iocp.c",
                     "bufferevent_async.c"],
         "scope": "a reproducible source-contract audit, not a proof of the upstream C "
                  "implementation"},
        [], "docs/cycles/CYCLE_18_LIBEVENT_VALIDATION.md",
        "frozen: the pinned behaviour markers live in the audit driver at the frozen revision")
    frozen(
        "cycle-19-fluent-bit", "b92688d220e1e62cffdcdb6ccc46476df0f406fd",
        "Cycle 19's pinned Fluent Bit corpus row (acceptance matrix corpus.cycle19_raw_units), "
        "last measured by DETACH-LOOKUP-MEMBERSHIP on the batch-2 link map.",
        {"kind": "pinned-corpus",
         "command": "QCORE_CROSS_UNIT_LINK_MAP=docs/reports/w48-bottom-up-batch2/link-map.corpus.json "
                    "python3 scripts/run_pinned_audit.py <out>/corpus 66910c10 --inputs-root "
                    "build/cycle19 --jobs 4 (docs/reports/detach-lookup-membership/scripts/measure.py "
                    "corpus)"},
        {"repository": "fluent-bit", "revision": FLUENT_BIT,
         "submodules": {"third_party/libevent": LIBEVENT},
         "note": "Fluent Bit v5.1.1-127-g66910c10a with its vendored libraries in-tree"},
        [("contract/profiles/fluent-bit/profile.json", "source-set pins and audited files"),
         ("contract/profiles/fluent-bit/audit.json", "unit selection"),
         ("contract/profiles/fluent-bit/member-facts-scope.json", "member-facts pre-pass exclusions"),
         ("docs/reports/w48-bottom-up-batch2/link-map.corpus.json", "cross-unit link map"),
         ("scripts/run_pinned_audit.py", "route runner")],
        [{"path": "build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source/build/compile_commands.json",
          "sha256": "5e6868d9af26eaec97bc8020ed5f23496f2490abf44532443239bd4d365f7e78",
          "role": "Fluent Bit compile database"},
         {"path": "build/cycle19/c-gateway-wave-pin/monkey/monkey-build/compile_commands.json",
          "sha256": "85cc996712bbd35675340b8b863b6ebc716cb94552253f2a6ad0288db920f331",
          "role": "Monkey compile database"}],
        matrix_row("cycle19_raw_units"),
        ["docs/reports/detach-lookup-membership/artifacts/corpus/after/" + name
         for name in ("chunk-contracts.json.gz", "contracts.json.gz", "corpus-null.jsonl.gz",
                      "corpus-null.summary.json.gz", "corpus-outer.json.gz")],
        "docs/reports/detach-lookup-membership/README.md",
        "frozen: the provider contracts of the tree at the frozen revision")
    frozen(
        "cycle-19-monkey-callbacks", "016df072c89b8d9439da45293fd5b9f8374b2020",
        "Cycle 19's profile-selected Monkey callback coverage (acceptance matrix "
        "corpus.cycle19_monkey_callback_coverage): three selected callbacks, eight paths.",
        {"kind": "callback-coverage",
         "command": "the pinned corpus route with the Monkey callback selection and the "
                    "focused replay (docs/reports/w42-monkey-callback-coverage/README.md)"},
        {"repository": "fluent-bit", "revision": FLUENT_BIT,
         "submodules": {"third_party/libevent": LIBEVENT},
         "note": "Fluent Bit v5.1.1-127-g66910c10a; Monkey in-tree under lib/monkey"},
        [("contract/profiles/fluent-bit/profile.json", "source-set pins and audited files"),
         ("contract/profiles/fluent-bit/audit.json", "unit selection")],
        [],
        matrix_row("cycle19_monkey_callback_coverage"),
        ["docs/reports/w42-priority-integration/attribution.json"],
        "docs/reports/w42-priority-integration/README.md",
        "frozen: the provider contracts of the tree at the frozen revision")
    frozen(
        "cmetrics-counter-configured", "55145be4f17aeace12734032dfb1b24a201d36ee",
        "The configured CMetrics counter audit of the cmetrics-counter source set (acceptance "
        "matrix corpus.configured_cmetrics_audit), last measured by card C02.",
        {"kind": "configured-source-set",
         "command": "cargo run -p qcore-audit -- contract/profiles/cmetrics-counter/audit.json"},
        {"repository": "fluent-bit", "revision": FLUENT_BIT, "submodules": {},
         "note": "lib/cmetrics/src/cmt_counter.c of Fluent Bit 66910c10a"},
        [("contract/profiles/cmetrics-counter/profile.json", "source-set pins and audited files"),
         ("contract/profiles/cmetrics-counter/audit.json", "audit configuration"),
         ("contract/profiles/cmetrics-counter/source-contracts.json", "source-contract bindings")],
        [],
        matrix_row("configured_cmetrics_audit"),
        ["docs/reports/remaining-c02/cmetrics-after.json"],
        "docs/reports/remaining-c02/README.md",
        "frozen: the source contracts bound by source-contracts.json at the frozen revision")
    frozen(
        "chunkio-core-configured", "60eef72ac8572a6ffee99f442d57b975dcb2f9b2",
        "The configured ChunkIO core audit of the chunkio-core source set (acceptance matrix "
        "corpus.configured_chunkio_core_audit), last measured by card G03.",
        {"kind": "configured-source-set",
         "command": "cargo run --quiet -p qcore-audit -- contract/profiles/chunkio-core/audit.json"},
        {"repository": "fluent-bit", "revision": FLUENT_BIT, "submodules": {},
         "note": "lib/chunkio/src of Fluent Bit 66910c10a"},
        [("contract/profiles/chunkio-core/profile.json", "source-set pins and audited files"),
         ("contract/profiles/chunkio-core/audit.json", "audit configuration")],
        [],
        matrix_row("configured_chunkio_core_audit"),
        ["docs/reports/remaining-g03/audit/chunkio-core-audit.json.gz",
         "docs/reports/remaining-g03/audit/chunkio-core-audit.assumptions.json"],
        "docs/reports/remaining-g03/README.md",
        "frozen: the provider contracts of the tree at the frozen revision")

    none = {"none": "a contract-inventory pin (CONTRACT-LOCKS), not an evaluation result"}
    bindings = {
        "schema": ep.BINDINGS_SCHEMA,
        "matrix": {
            "contract_modules": none,
            "contract_modules_pinned_by": none,
            "contract_clause_pin": none,
            "contract_modules_per_directory": none,
            "libevent_submodule_pin": {"none": "the Cycle 18 submodule identity pin, checked by "
                                                "compiler/tests/libevent_submodule.rs; an input "
                                                "of cycle-18-libevent, not a result"},
            "cycle19_raw_units": "cycle-19-fluent-bit",
            "configured_cmetrics_audit": "cmetrics-counter-configured",
            "configured_chunkio_core_audit": "chunkio-core-configured",
            "cycle19_monkey_callback_coverage": "cycle-19-monkey-callbacks",
            "conditional_eight_home_cycle3": {"split": {
                "routes/corpus": "eight-home",
                "routes/system": "fluent-bit-system",
                "routes/slice": "fluent-bit-tail-pause"}},
        },
        "pinned_cycle_documents": {
            "docs/cycles/CYCLE_18_LIBEVENT_VALIDATION.md": {
                "profile": "cycle-18-libevent",
                "reason": "its LF bytes are the historical_validation digest pinned by "
                          "providers/qcore-provider-libevent/contracts/monkey/"
                          "native_platform_matrix.qv, so the binding lives here, not in the "
                          "document"}},
        "frozen_amendments": [],
    }
    write(str(ep.BINDINGS), bindings)


if __name__ == "__main__":
    main()
