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

"""Enforce standing size, artifact-bloat, and archive-layout hygiene under `docs/reports/`."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
REPORTS = Path("docs/reports")
PRUNED_SCHEMA = "quirin-pruned-report-artifacts/1"
MAX_REPORT_FILE_BYTES = 512 * 1024  # 512 KiB per committed report file
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
ATTEMPT_SEG_RE = re.compile(r"^attempt-\d+$")

FORBIDDEN_BULK_NAMES = frozenset({
    "resource-diagnostics.json",
    "resource-diagnostics.json.gz",
    "resource-diagnostics.contracts.json",
    "resource-diagnostics.contracts.json.gz",
    "resource-diagnostics.components.json",
    "resource-diagnostics.components.json.gz",
    "resource-diagnostics.chunk-contracts.json",
    "resource-diagnostics.chunk-contracts.json.gz",
    "diagnostics.json",
    "diagnostics.json.gz",
    "parity.jsonl",
    "parity.jsonl.gz",
    "corpus-null.jsonl",
    "corpus-null.jsonl.gz",
    "null.jsonl",
    "null.jsonl.gz",
    "provenance-preimages.json",
    "provenance-preimages.json.gz",
    "raw-differences.json.gz",
    "five-artifacts.json.gz",
    "five-artifact-diff.json.gz",
    "standard-five-artifact-diff.json.gz",
    "normalized-field-differences.json.gz",
    "gate-scope.json",
    "corpus-reading-proof.json",
})

# Frozen, mechanically pinned files read directly by evaluation baselines, contribution ledgers,
# or test suites that predate the 512 KiB / bulk-dump rule. Never expand for new reports;
# publish compact diffs and SHA-256 digests instead.
FROZEN_PINNED_ALLOWLIST = frozenset({
    "docs/reports/INDEX.md",
    "docs/reports/archive/contracts-and-premises/sp1-symbolic-openssl-premises/integration/gate.json",
    "docs/reports/archive/language-and-tooling/arm-obligations-unit-grain/native-comparison.json",
    "docs/reports/archive/language-and-tooling/arm-obligations/native-comparison.json",
    "docs/reports/b-series-integration/artifacts/chunk-contracts.json.gz",
    "docs/reports/c-gateway-return-call-w4/artifacts/diagnostics.json.gz",
    "docs/reports/detach-lookup-membership/artifacts/corpus/after/chunk-contracts.json.gz",
    "docs/reports/detach-lookup-membership/artifacts/corpus/after/contracts.json.gz",
    "docs/reports/detach-lookup-membership/artifacts/corpus/after/corpus-null.jsonl.gz",
    "docs/reports/detach-lookup-membership/artifacts/corpus/after/corpus-null.summary.json.gz",
    "docs/reports/detach-lookup-membership/artifacts/corpus/after/diagnostics.json.gz",
    "docs/reports/detach-lookup-membership/artifacts/corpus/after/source-effects.json.gz",
    "docs/reports/gate-scope-precise-selection/read-census.json",
    "docs/reports/premise-symbolic-adoption/inventory-before.json",
    "docs/reports/rule35-pilot/native-comparison.json",
    "docs/reports/rule36-pilot/native-comparison.json",
    "docs/reports/rule37-pilot/native-comparison.json",
    "docs/reports/sp2-symbolic-librdkafka-premises/integration/gate.json",
    "docs/reports/w42-chunk-lifecycle-cert/artifacts/chunk-contracts.json.gz",
    "docs/reports/w42-priority-integration/artifacts/chunk-contracts.json.gz",
    "docs/reports/w48-cmt-list-self-init/gate.json",
})

FROZEN_PINNED_DIR_PREFIXES = (
    "docs/reports/cycle19-file-constructor-failures/",
    "docs/reports/cycle19-file-descriptor-failures/",
    "docs/reports/cycle19-file-existing/",
    "docs/reports/cycle19-file-os-constructors/",
    "docs/reports/cycle19-file-sync-failures/",
    "docs/reports/cycle19-file-transitions/",
    "docs/reports/cycle19-file-unmap-failures/",
    "docs/reports/cycle19-reopen-failures/",
    "docs/reports/detach-lookup-membership/artifacts/corpus/after/",
    "docs/reports/premise-guarded-decomposition/applications/",
    "docs/reports/quant-contract-hypotheses/plugin/before/",
    "docs/reports/sp10-formal-binding-descriptions/bindings/",
    "docs/reports/sp2-symbolic-librdkafka-premises/bindings/",
    "docs/reports/sp6-symbolic-hypothesis-atoms/bindings/",
    "docs/reports/w42-chunk-lifecycle-cert/",
    "docs/reports/w43-chunk-source-profile/",
    "docs/reports/wave13-corpus/artifacts/",
    "docs/reports/wave14-corpus/artifacts/",
    "docs/reports/wave15-corpus/artifacts/",
    "docs/reports/wave17-corpus/artifacts/",
)


def _is_frozen_allowed(rel_posix: str) -> bool:
    if rel_posix in FROZEN_PINNED_ALLOWLIST:
        return True
    return any(rel_posix.startswith(prefix) for prefix in FROZEN_PINNED_DIR_PREFIXES)


def validate_pruned_manifest(path: Path, rel_posix: str) -> list[str]:
    try:
        doc = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as err:
        return [f"{rel_posix}: invalid JSON ({err})"]
    if not isinstance(doc, dict) or doc.get("schema") != PRUNED_SCHEMA:
        return [f"{rel_posix}: must carry schema {PRUNED_SCHEMA!r}"]
    artifacts = doc.get("artifacts")
    if not isinstance(artifacts, list) or not artifacts:
        return [f"{rel_posix}: 'artifacts' must be a non-empty list"]
    errors: list[str] = []
    prev_path = None
    total_bytes = 0
    for idx, entry in enumerate(artifacts):
        if not isinstance(entry, dict):
            errors.append(f"{rel_posix}: artifacts[{idx}] is not an object")
            continue
        epath = entry.get("path")
        ebytes = entry.get("bytes")
        esha = entry.get("sha256")
        if not isinstance(epath, str) or not epath or epath.startswith("/") or ".." in epath.split("/"):
            errors.append(f"{rel_posix}: artifacts[{idx}].path {epath!r} must be a clean card-relative path")
        elif prev_path is not None and epath <= prev_path:
            errors.append(f"{rel_posix}: artifacts must be strictly sorted and unique ({epath!r} after {prev_path!r})")
        prev_path = epath if isinstance(epath, str) else prev_path
        if not isinstance(ebytes, int) or isinstance(ebytes, bool) or ebytes < 0:
            errors.append(f"{rel_posix}: artifacts[{idx}].bytes must be a non-negative integer")
        else:
            total_bytes += ebytes
        if not isinstance(esha, str) or not SHA256_RE.match(esha):
            errors.append(f"{rel_posix}: artifacts[{idx}].sha256 must be 64 lowercase hex digits")
    if doc.get("total_pruned_files") != len(artifacts):
        errors.append(f"{rel_posix}: total_pruned_files ({doc.get('total_pruned_files')}) != {len(artifacts)}")
    if doc.get("total_pruned_bytes") != total_bytes:
        errors.append(f"{rel_posix}: total_pruned_bytes ({doc.get('total_pruned_bytes')}) != {total_bytes}")
    return errors


def _list_report_files(root: Path, reports_dir: Path) -> list[Path]:
    if (root / ".git").exists():
        res = subprocess.run(
            ["git", "-C", str(root), "ls-files", "--cached", "--others", "--exclude-standard", "-z", "--", "docs/reports"],
            capture_output=True,
        )
        if res.returncode == 0:
            files = []
            for raw in res.stdout.split(b"\0"):
                if not raw:
                    continue
                candidate = root / raw.decode("utf-8", errors="replace")
                if candidate.is_file():
                    files.append(candidate)
            return sorted(set(files))
    return sorted(p for p in reports_dir.rglob("*") if p.is_file())


def check_reports(root: Path = ROOT) -> list[str]:
    root = root.resolve()
    reports_dir = root / REPORTS
    if not reports_dir.is_dir():
        return []

    problems: list[str] = []
    seen_cards: dict[str, str] = {}

    for top in sorted(p for p in reports_dir.iterdir() if p.is_dir() and p.name != "archive"):
        seen_cards[top.name] = f"docs/reports/{top.name}"

    archive_dir = reports_dir / "archive"
    if archive_dir.is_dir():
        for bucket in sorted(p for p in archive_dir.iterdir() if p.is_dir()):
            for card_dir in sorted(p for p in bucket.iterdir() if p.is_dir()):
                rel_card = f"docs/reports/archive/{bucket.name}/{card_dir.name}"
                if card_dir.name in seen_cards:
                    problems.append(
                        f"duplicate report card directory {card_dir.name!r}: "
                        f"{seen_cards[card_dir.name]} and {rel_card}"
                    )
                else:
                    seen_cards[card_dir.name] = rel_card

    for file_path in _list_report_files(root, reports_dir):
        rel_posix = file_path.relative_to(root).as_posix()
        parts = file_path.relative_to(reports_dir).parts
        name = file_path.name

        if name == "pruned-artifacts.json":
            problems.extend(validate_pruned_manifest(file_path, rel_posix))
            continue

        if "wrong-clang-attempt" in rel_posix or any(
            ATTEMPT_SEG_RE.match(seg) or seg == "failed" for seg in parts[:-1]
        ):
            problems.append(
                f"{rel_posix}: failed or superseded attempt artifact must stay in private/ (never committed)"
            )
            continue

        if _is_frozen_allowed(rel_posix):
            continue

        if (
            "/raw/before/" in rel_posix
            or "/raw/after/" in rel_posix
            or name in FORBIDDEN_BULK_NAMES
            or name.endswith(".prepared.c.gz")
        ):
            problems.append(
                f"{rel_posix}: raw bulk dump or gate-scope log must stay in private/; "
                "commit compact diffs and SHA-256 digests instead"
            )
            continue

        size = file_path.stat().st_size
        if size > MAX_REPORT_FILE_BYTES:
            problems.append(
                f"{rel_posix} is {size:,} bytes, over the {MAX_REPORT_FILE_BYTES:,}-byte "
                "report-artifact ceiling; keep raw outputs in private/ and commit compact summaries/digests"
            )

    return problems


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    args = parser.parse_args(argv)
    problems = check_reports(args.root)
    if problems:
        for problem in problems:
            print(f"report-hygiene: {problem}", file=sys.stderr)
        return 1
    print("Report hygiene checks passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
