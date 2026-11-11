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

"""Generate the numbered constraint sections and the rule router from rule files.

Each numbered or lettered constraint lives in one file,
`docs/agents/rules/<id>.md`, its single authority:

    ---
    rule: <id>
    topic: <page stem under docs/agents/>
    summary: <router opening sentence, navigation only; omit for no router row>
    ---

    <the rule text, exactly as rendered on its topic page>

The rule text is rendered into its topic page, so its relative links resolve
from `docs/agents/`. This script rewrites only the text between the
`generated:rules` markers of each topic page (an anchor
`<a id="constraint-<id>"></a>` and the rule text per rule, in natural rule
order: 4, 4a, 4b, ..., 10z, 11-series, 11a, ...) and the stable rule index
table between the `generated:router` markers of `docs/agents/README.md`.
Everything outside the markers stays hand-edited.

Regenerate with `python3 scripts/gen_rule_docs.py`; validate with `--check`.
On a merge conflict in a generated page, take either side and rerun the script.
"""
from pathlib import Path
import argparse
import re

from listed_files import listed_files

ROOT = Path(__file__).resolve().parents[1]
AGENTS = ROOT / "docs/agents"

PAGE_START = ("<!-- generated:rules:start — edit docs/agents/rules/<id>.md, "
              "then run python3 scripts/gen_rule_docs.py -->")
PAGE_END = "<!-- generated:rules:end -->"
ROUTER_START = ("<!-- generated:router:start — rows come from docs/agents/rules/<id>.md; "
                "run python3 scripts/gen_rule_docs.py -->")
ROUTER_END = "<!-- generated:router:end -->"
ROUTER_HEADER = ("| Rule | Full text | Opening sentence (navigation only) |\n"
                 "|---|---|---|\n")

RULE_ID = re.compile(r"^(\d+)([a-z]*)((?:-[a-z0-9]+)*)$")
ANCHOR = re.compile(r'<a id="constraint-([^"]+)"></a>')


class RuleError(ValueError):
    pass


def natural_key(rule_id):
    """4 < 4a < 4b < 5 < ... < 10z < 11 < 11-series < 11a < ... < 15z < 15aa."""
    match = RULE_ID.match(rule_id)
    if not match:
        raise RuleError(f"rule id {rule_id!r} is not <number><letters>[-<suffix>]")
    number, letters, suffix = match.groups()
    return (int(number), len(letters), letters, suffix)


def parse_rule(path):
    text = path.read_text(encoding="utf-8")
    if not text.startswith("---\n"):
        raise RuleError(f"{path}: missing front matter")
    try:
        front, body = text[4:].split("\n---\n", 1)
    except ValueError:
        raise RuleError(f"{path}: unterminated front matter") from None
    fields = {}
    for line in front.split("\n"):
        key, sep, value = line.partition(": ")
        if not sep or key in fields or key not in {"rule", "topic", "summary"}:
            raise RuleError(f"{path}: bad front matter line {line!r}")
        fields[key] = value
    for key in ("rule", "topic"):
        if key not in fields:
            raise RuleError(f"{path}: front matter lacks {key}")
    if fields["rule"] != path.stem:
        raise RuleError(f"{path}: rule {fields['rule']!r} differs from its file name")
    natural_key(fields["rule"])
    body = body.strip("\n")
    if not body:
        raise RuleError(f"{path}: empty rule text")
    if ANCHOR.search(body):
        raise RuleError(f"{path}: the rule text carries a constraint anchor; the generator writes it")
    if PAGE_START in body or PAGE_END in body:
        raise RuleError(f"{path}: the rule text carries a generator marker")
    return {"id": fields["rule"], "topic": fields["topic"],
            "summary": fields.get("summary"), "body": body, "path": path}


def render_rule_file(rule):
    """The canonical text of a rule file (used by the one-time split)."""
    front = f"---\nrule: {rule['id']}\ntopic: {rule['topic']}\n"
    if rule.get("summary") is not None:
        front += f"summary: {rule['summary']}\n"
    return front + "---\n\n" + rule["body"].strip("\n") + "\n"


def rule_paths(files, agents=AGENTS):
    """The rule files among `files` (paths under docs/agents/)."""
    return sorted(Path(path) for path in files
                  if Path(path).parent == agents / "rules" and Path(path).suffix == ".md"
                  and Path(path).name != "README.md")


def load_rules(files, agents=AGENTS):
    rules = [parse_rule(path) for path in rule_paths(files, agents)]
    ids = [rule["id"] for rule in rules]
    if len(set(ids)) != len(ids):
        raise RuleError("duplicated rule ids")
    return sorted(rules, key=lambda rule: natural_key(rule["id"]))


def replace_between(text, start, end, inner, where):
    if text.count(start) != 1 or text.count(end) != 1:
        raise RuleError(f"{where}: needs exactly one {start[:24]}... and one {end}")
    head, rest = text.split(start, 1)
    _, tail = rest.split(end, 1)
    return head + start + "\n\n" + inner + end + tail


def render_section(rules):
    blocks = [f'<a id="constraint-{rule["id"]}"></a>\n\n{rule["body"]}\n' for rule in rules]
    return "\n".join(blocks) + "\n"


def render_router(rules):
    rows = [f"| {rule['id']} | [{rule['topic']}]({rule['topic']}.md#constraint-{rule['id']}) "
            f"| {rule['summary']} |\n" for rule in rules if rule["summary"] is not None]
    return ROUTER_HEADER + "".join(rows) + "\n"


def render(files, agents=AGENTS):
    """Map each generated path to its expected text.

    `files` lists the files under docs/agents/ (`listed_files`, or the tracked
    list in tests).
    """
    rules = load_rules(files, agents)
    pages = {Path(path).stem: Path(path) for path in files
             if Path(path).parent == agents and Path(path).suffix == ".md"
             and Path(path).name != "README.md"
             and PAGE_START in Path(path).read_text(encoding="utf-8")}
    by_topic = {}
    for rule in rules:
        if rule["topic"] not in pages:
            raise RuleError(f"{rule['path']}: topic page docs/agents/{rule['topic']}.md "
                            "has no generated:rules section")
        by_topic.setdefault(rule["topic"], []).append(rule)
    expected = {}
    for topic, path in sorted(pages.items()):
        if topic not in by_topic:
            raise RuleError(f"{path}: generated:rules section with no rule files")
        text = replace_between(path.read_text(encoding="utf-8"), PAGE_START, PAGE_END,
                               render_section(by_topic[topic]), path)
        ids = ANCHOR.findall(text)
        duplicated = sorted({rule_id for rule_id in ids if ids.count(rule_id) > 1})
        if duplicated:
            raise RuleError(f"{path}: duplicated constraint anchors {duplicated}")
        expected[path] = text
    router = agents / "README.md"
    expected[router] = replace_between(router.read_text(encoding="utf-8"), ROUTER_START,
                                       ROUTER_END, render_router(rules), router)
    return expected


def router_rows(text):
    """(id, page, anchor) for every stable-index row of a router text."""
    return re.findall(r"^\| ([^|]+?) \| \[[^\]]+\]\(([a-z-]+\.md)#constraint-([^)]+)\) \| ",
                      text, re.M)


def stale(files, agents=AGENTS):
    return [path for path, text in render(files, agents).items()
            if path.read_text(encoding="utf-8") != text]


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--check", action="store_true",
                        help="fail if a committed page differs from the generated one")
    args = parser.parse_args()
    files = listed_files(AGENTS)
    if args.check:
        bad = stale(files)
        if bad:
            names = ", ".join(str(path.relative_to(ROOT)) for path in bad)
            raise SystemExit(f"stale generated rule pages {names}; run python3 scripts/gen_rule_docs.py")
        print(f"Rule pages and router match {len(load_rules(files))} rule files")
        return
    for path, text in render(files).items():
        if path.read_text(encoding="utf-8") != text:
            path.write_text(text, encoding="utf-8")
            print(f"Wrote {path.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
