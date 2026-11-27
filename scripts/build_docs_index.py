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

"""Build/check the report lookup index and archive category hubs; titles are not interpreted as status."""
from __future__ import annotations

import argparse
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]

ARCHIVE_BUCKETS = (
    ("early-waves", "Waves 1–19 and early `remaining-*` conformance/remediation cards"),
    ("waves-20-39", "Waves 20–39 source profiles, standard modules, and close records"),
    ("cycles-18-19", "Cycle 18 & Cycle 19 evaluations, custody, and OS failure suites"),
    ("waves-40-47", "Waves 40–47 C-gateway, audit tooling, and provider cards"),
    ("wave-48", "Completed Wave 48 historical cards and integration landings"),
    ("gateway-and-clang", "C-gateway rules, Clang-gap (`CG*`), and alias/access cards"),
    ("contracts-and-premises", "Contract loaders, symbolic premises (`SP*`), bounds, null-deref, and cell protocols"),
    ("language-and-tooling", "Quirin `.qr` language profiles, concurrency, FFI, gate/audit tooling, and compactions"),
)


def _title_of(readme: Path, fallback: str) -> str:
    match = re.search(r"^# (.+)$", readme.read_text(encoding="utf-8"), re.M)
    return (match[1] if match else fallback).replace("|", "&#124;")


def discover_reports(reports_dir: Path) -> tuple[list[tuple[str, str, str]], dict[str, list[tuple[str, str]]]]:
    """Return `(all_entries, by_bucket)` where `all_entries` is sorted
    `(card_name, rel_readme_from_reports, title)` across top-level and `archive/*/*`."""
    seen: dict[str, str] = {}
    entries: list[tuple[str, str, str]] = []
    by_bucket: dict[str, list[tuple[str, str]]] = {}

    if reports_dir.is_dir():
        for readme in sorted(reports_dir.glob("*/README.md")):
            card = readme.parent.name
            if card == "archive":
                continue
            rel = f"{card}/README.md"
            seen[card] = rel
            entries.append((card, rel, _title_of(readme, card)))

        archive_dir = reports_dir / "archive"
        if archive_dir.is_dir():
            for bucket_dir in sorted(p for p in archive_dir.iterdir() if p.is_dir()):
                bucket = bucket_dir.name
                bucket_rows: list[tuple[str, str]] = []
                for readme in sorted(bucket_dir.glob("*/README.md")):
                    card = readme.parent.name
                    rel = f"archive/{bucket}/{card}/README.md"
                    if card in seen:
                        raise SystemExit(
                            f"duplicate report directory {card!r}: {seen[card]} and {rel}"
                        )
                    seen[card] = rel
                    title = _title_of(readme, card)
                    entries.append((card, rel, title))
                    bucket_rows.append((card, title))
                by_bucket[bucket] = bucket_rows

    entries.sort(key=lambda item: item[0])
    return entries, by_bucket


def render_index(entries: list[tuple[str, str, str]]) -> str:
    rows = [
        "# Report lookup index\n",
        "[Documentation hub](../README.md) · [Reports hub](README.md) · [Wave history](../development/increments/README.md)\n"
        if any(rel.startswith("archive/") for _, rel, _ in entries)
        else "[Documentation hub](../README.md) · [Wave history](../development/increments/README.md)\n",
        "Generated directory/title lookup, **not a status ledger**. Search an exact\n"
        "card id here; read its report for evidence and the acceptance matrix's\n"
        "`named_followups` for registered open work. Historical measurements retain their original revision; maintained inputs are attributed in migration reports.\n",
        "Regenerate with `python3 scripts/build_docs_index.py`; validate with `--check`.\n",
    ]
    group = None
    for card, rel, title in entries:
        letter = card[0].upper()
        if letter != group:
            rows += [f"## {letter}\n", "| Card directory | Original report title |\n|---|---|\n"]
            group = letter
        rows.append(f"| [{card}]({rel}) | {title} |\n")
    return "\n".join(rows)


def render_archive_hub(
    entries: list[tuple[str, str, str]], by_bucket: dict[str, list[tuple[str, str]]]
) -> dict[Path, str]:
    """Render `docs/reports/README.md` and `docs/reports/archive/<bucket>/README.md`."""
    outputs: dict[Path, str] = {}
    top_count = sum(1 for _, rel, _ in entries if not rel.startswith("archive/"))
    arch_count = len(entries) - top_count
    known_desc = dict(ARCHIVE_BUCKETS)

    hub_lines = [
        "# Verification & card reports hub\n",
        "[Documentation hub](../README.md) · [Alphabetical card lookup (`INDEX.md`)](INDEX.md) · "
        "[Current status](../STATUS.md) · [Increment history](../development/increments/README.md)\n",
        "Card and integration reports are organized into two tiers so active/test-pinned landmarks stay "
        "immediately accessible while historical wave reports are grouped by era and subsystem:\n",
        f"- **Top-level active & mechanically pinned reports (`docs/reports/<card>/`)**: **{top_count}** directories "
        "referenced directly by Rust/Python test harnesses, evaluation baselines, contribution ledgers, or rule gates.\n"
        f"- **Categorized historical archive (`docs/reports/archive/<category>/<card>/`)**: **{arch_count}** directories "
        "grouped into era and subsystem buckets below.\n",
        "## Historical archive categories\n",
        "| Archive directory | Reports | Scope |\n|---|---:|---|\n",
    ]

    ordered_buckets = [b for b, _ in ARCHIVE_BUCKETS if b in by_bucket] + [
        b for b in sorted(by_bucket) if b not in known_desc
    ]
    for bucket in ordered_buckets:
        rows = by_bucket[bucket]
        desc = known_desc.get(bucket, "Archived card reports")
        hub_lines.append(
            f"| [`archive/{bucket}/`](archive/{bucket}/README.md) | {len(rows)} | {desc} |\n"
        )
        b_lines = [
            f"# Archived reports — `{bucket}`\n",
            f"[Reports hub](../../README.md) · [Full alphabetical lookup (`INDEX.md`)](../../INDEX.md)\n",
            f"{desc} (**{len(rows)}** reports). Titles are historical checkpoint titles, not current status.\n",
            "| Card directory | Original report title |\n|---|---|\n",
        ]
        for card, title in rows:
            b_lines.append(f"| [{card}]({card}/README.md) | {title} |\n")
        outputs[Path("archive") / bucket / "README.md"] = "\n".join(b_lines)

    hub_lines.append(
        "\n## Artifact hygiene & provenance manifests\n\n"
        "Unreferenced multi-megabyte raw corpus dumps, per-step execution logs, and superseded attempt "
        "directories are pruned from git working trees into per-card `pruned-artifacts.json` manifests "
        "(`schema: \"quirin-pruned-report-artifacts/1\"`) recording each file's original relative path, "
        "byte count, and SHA-256 digest. New reports are checked by `python3 scripts/check_report_hygiene.py` "
        "(wired into `python3 scripts/land.py check`).\n"
    )
    outputs[Path("README.md")] = "\n".join(hub_lines)
    return outputs


def render_all(root: Path = ROOT) -> dict[Path, str]:
    reports_dir = root / "docs/reports"
    entries, by_bucket = discover_reports(reports_dir)
    files: dict[Path, str] = {reports_dir / "INDEX.md": render_index(entries)}
    if (reports_dir / "archive").is_dir():
        for rel_path, content in render_archive_hub(entries, by_bucket).items():
            files[reports_dir / rel_path] = content
    return files


def render() -> str:
    entries, _ = discover_reports(ROOT / "docs/reports")
    return render_index(entries)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--root", type=Path, default=ROOT)
    args = parser.parse_args()
    root = args.root.resolve()
    expected_files = render_all(root)
    if args.check:
        stale = [
            str(target.relative_to(root))
            for target, content in expected_files.items()
            if not target.exists() or target.read_text(encoding="utf-8") != content
        ]
        if stale:
            raise SystemExit(
                f"report index is stale ({', '.join(stale)}); run python3 scripts/build_docs_index.py"
            )
        print("Report index matches all report directories")
    else:
        for target, content in expected_files.items():
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(content, encoding="utf-8")
        print(f"Wrote {len(expected_files)} report index file(s)")
