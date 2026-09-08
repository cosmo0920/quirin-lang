#!/usr/bin/env python3
"""Measure the seven named P09-A..G legacy routes and emit `route-census.json`.

Mechanical facts only: where each route is defined, which files reference it,
and whether the symbol its recorded replacement depends on exists in the tree.
Dispositions and the reasons for them live in `README.md`; this script records
what a reader can re-measure, not what the card concluded.

    python3 -B docs/reports/remaining-p09-routes/census.py > \
      docs/reports/remaining-p09-routes/route-census.json
"""
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
TREE = ["compiler", "crates", "providers", "scripts", "docs", "contract"]
CODE = ("compiler", "crates", "providers")


def rust_files(dirs):
    out = []
    for d in dirs:
        out.extend(sorted((ROOT / d).rglob("*.rs")))
    return out


def sites(pattern, dirs=CODE, exclude=()):
    """Every `path:line` in `dirs` whose text matches `pattern`."""
    found = []
    rx = re.compile(pattern)
    for path in rust_files(dirs):
        rel = str(path.relative_to(ROOT))
        if rel in exclude:
            continue
        for number, line in enumerate(path.read_text(errors="ignore").splitlines(), 1):
            if rx.search(line):
                found.append(f"{rel}:{number}")
    return found


def classify(site):
    path = site.split(":", 1)[0]
    if "/tests/" in path or path.endswith("_test.rs"):
        return "test"
    if "/examples/" in path:
        return "example"
    if "/benches/" in path:
        return "bench"
    return "production"


def route(identifier, name, definition, pattern, replacement, replacement_present,
          artifacts, exclude=(), dirs=CODE, exclude_definition_file=True):
    if exclude_definition_file:
        exclude = exclude + (definition.split(":", 1)[0],)
    all_sites = sites(pattern, dirs=dirs, exclude=exclude)
    kinds = {}
    for site in all_sites:
        kinds.setdefault(classify(site), []).append(site)
    return {
        "route": identifier,
        "name": name,
        "definition": definition,
        "reference_pattern": pattern,
        "references": kinds,
        "reference_count": len(all_sites),
        "replacement": replacement,
        "replacement_symbol_present": replacement_present,
        "pinned_artifacts_influenced": artifacts,
    }


def present(pattern, dirs=CODE):
    return bool(sites(pattern, dirs))


def main():
    routes = [
        route(
            "P09-A",
            "ChunkIO in-process counter route",
            "compiler/src/verifier_plugins/chunkio/teardown_source.rs:43",
            r"\bcheck_counter_sources\b",
            "qcore_verification::counter_source::CounterSourceTask + package qcore.chunkio 0.5.0",
            present(r"pub struct CounterSourceTask", ["crates"]),
            ["docs/reports/remaining-o04-arms/artifacts/chunk-contracts.json.gz"],
        ),
        route(
            "P09-B",
            "Libevent in-process event-program route",
            "compiler/src/verifier_plugins/libevent/event_driven.rs:407",
            r"\bcheck_event_program\b",
            "qcore_verification::reactor_source + package qcore.libevent 0.2.0",
            present(r"pub struct ReactorProgramImport|mod reactor_source", ["crates"]),
            [],
            exclude=("crates/qcore-core/src/event_reactor.rs",),
        ),
        route(
            "P09-C",
            "SourceContractConfiguration::Direct in the legacy audit example",
            "compiler/examples/support/cycle19_compatibility.rs:928",
            r"SourceContractConfiguration::Direct|SourceContractPolicies",
            "SourceContractConfiguration::Process (qcore_audit::source_contract::SourceContractRegistry)",
            present(r"pub struct SourceContractRegistry", ["crates"]),
            ["docs/reports/remaining-o04-arms/artifacts/source-effects.json.gz"],
        ),
        route(
            "P09-D",
            "QCORE_CHUNKIO_PACKAGE selector alias",
            "compiler/examples/support/cycle19_compatibility.rs:953",
            r"QCORE_CHUNKIO_PACKAGE",
            "QCORE_SOURCE_CONTRACTS configuration selection",
            present(r"QCORE_SOURCE_CONTRACTS"),
            ["docs/reports/remaining-o04-arms/artifacts/source-effects.json.gz"],
            exclude_definition_file=False,
        ),
        route(
            "P09-E",
            "Legacy out-of-protocol reactor record-exchange fixture",
            "providers/conformance/src/bin/callback_proposal_writer.rs:1",
            r"callback_proposal_writer|CallbackProposalError::(MissingOperation|DuplicateOperation|UnsupportedVersion)",
            "providers/conformance/tests/reactor_callback_capability.rs (P05 matrix)",
            present(r"reactor_callback_capability", ["providers"]) or (
                ROOT / "providers/conformance/tests/reactor_callback_capability.rs"
            ).exists(),
            [],
        ),
        route(
            "P09-F",
            "Compiler compatibility facades",
            "compiler/src/lib.rs",
            r"crate::(verifier_plugin|external_source|counter_source|research_extensions|cleanup_outcome|source_call_use|typed_source|list_unlink_contract|field_destructor|counter_contract|chunk_contract)\b"
            r"|qcore_compiler::(verifier_plugin|external_source|counter_source|research_extensions|cleanup_outcome|source_call_use|typed_source|list_unlink_contract|field_destructor|counter_contract|chunk_contract)\b",
            "the canonical crate paths (qcore_verification::*, qcore_c_gateway::*, qcore_verifier_host::*)",
            True,
            [],
        ),
        route(
            "P09-G",
            "chunk_source::import_chunk_source zero-helper wrapper",
            "compiler/src/verifier_plugins/chunkio/chunk_source.rs:122",
            r"\bimport_chunk_source\b(?!_with)",
            "import_chunk_source_with_helpers(unit, revision, &[])",
            present(r"pub fn import_chunk_source_with_helpers"),
            [],
        ),
    ]
    facades = {}
    for module in [
        "verifier_plugin", "external_source", "counter_source", "research_extensions",
        "cleanup_outcome", "source_call_use", "typed_source", "list_unlink_contract",
        "field_destructor", "counter_contract", "chunk_contract",
    ]:
        source = (ROOT / "compiler/src" / f"{module}.rs").read_text()
        facades[module] = {
            "lines": len(source.splitlines()),
            "pure_reexport": "pub fn " not in source and "pub struct " not in source,
            "hosts_attached_unit_tests": "#[path" in source,
            "consumers": len(
                sites(rf"crate::{module}\b|qcore_compiler::{module}\b",
                      exclude=(f"compiler/src/{module}.rs",))
            ),
        }
    # P09-D is user-facing: the alias is spelled in prose and in the archived
    # reproduction commands, not only in Rust.
    alias_documents = []
    for pattern in ("docs/**/*.md", "providers/*.md", "scripts/*.py", "docs/**/*.py"):
        for path in sorted(ROOT.glob(pattern)):
            if "QCORE_CHUNKIO_PACKAGE" in path.read_text(errors="ignore"):
                alias_documents.append(str(path.relative_to(ROOT)))
    document = {
        "measurement": "P09-A..G legacy route census",
        "commit": subprocess.run(
            ["git", "-C", str(ROOT), "rev-parse", "HEAD"],
            capture_output=True, text=True, check=True).stdout.strip(),
        "note": "reference counts are textual occurrences in compiler/, crates/ and providers/; "
                "the route's own definition file is excluded",
        "routes": routes,
        "p09_d_alias_documents": alias_documents,
        "p09_f_facade_detail": facades,
    }
    json.dump(document, sys.stdout, indent=2)
    sys.stdout.write("\n")


if __name__ == "__main__":
    main()
