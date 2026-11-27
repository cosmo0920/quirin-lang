"""The card's own oracle: the AFTER run differs from the BEFORE run in
**attribution presentation only**.

Card GATEWAY-C-LOCATION. This is stricter than the standard five-artifact
itemizer, which reports a unit row's `functions` value as "changed" without
saying what inside it moved. Here every claim the corpus carries is compared
directly, and the only differences permitted are:

* a diagnostic row gaining an `origin` key;
* a diagnostic row's `description` with its `preprocessed line N` site replaced
  by that same line's presented coordinate;
* a unit row gaining an `origin_coordinates` account;
* the report envelope's `line_coordinates` text and its new `origin_meaning`.

Anything else — a verdict, a status, a leaf, a count, a reason, an enumeration
row — is printed as a STOP finding.

    python3 -B measure/leaf_identity.py measure/runs/before measure/runs/after
"""
import json
import pathlib
import re
import sys

BEFORE = pathlib.Path(sys.argv[1]) / "resource-diagnostics.json"
AFTER = pathlib.Path(sys.argv[2]) / "resource-diagnostics.json"

before = json.loads(BEFORE.read_text())
after = json.loads(AFTER.read_text())

findings = []
stats = {
    "units": 0,
    "functions": 0,
    "diagnostics": 0,
    "origin_present": 0,
    "origin_null": 0,
    "descriptions_changed": 0,
    "descriptions_unchanged": 0,
    "origin_coordinates_established": 0,
    "origin_coordinates_refused": 0,
    "origin_coordinates_null": 0,
    "withdrawn_lines": 0,
    "sites_presented": 0,
    "sites_agreeing_with_a_sibling_row": 0,
    "sites_without_a_sibling_row": 0,
}

SITE = re.compile(r"preprocessed line (\d+)")
COORDINATE = re.compile(r"[^\s]+:\d+")

# ---------------------------------------------------------------------------
# envelope
# ---------------------------------------------------------------------------
ENVELOPE_ALLOWED = {"line_coordinates", "origin_meaning"}
for key in sorted(set(before) | set(after)):
    if key in ("units",):
        continue
    if before.get(key) != after.get(key) and key not in ENVELOPE_ALLOWED:
        findings.append(f"STOP envelope key `{key}` changed")
for key in ENVELOPE_ALLOWED:
    if key not in after:
        findings.append(f"STOP envelope key `{key}` missing from the AFTER run")

# ---------------------------------------------------------------------------
# units
# ---------------------------------------------------------------------------
if len(before["units"]) != len(after["units"]):
    findings.append(
        f"STOP unit count {len(before['units'])} -> {len(after['units'])}"
    )

UNIT_ALLOWED = {"functions", "origin_coordinates"}

for old, new in zip(before["units"], after["units"]):
    stats["units"] += 1
    where = f"{old.get('library')}/{old.get('file')}"
    if (old.get("library"), old.get("file")) != (new.get("library"), new.get("file")):
        findings.append(f"STOP unit order moved at {where}")
        continue
    for key in sorted(set(old) | set(new)):
        if key in UNIT_ALLOWED:
            continue
        if old.get(key) != new.get(key):
            findings.append(f"STOP {where}: unit key `{key}` changed")
    if "origin_coordinates" in old:
        findings.append(f"STOP {where}: the BEFORE run already carries origin_coordinates")
    account = new.get("origin_coordinates")
    if account is None:
        stats["origin_coordinates_null"] += 1
    elif account.get("status") == "established":
        stats["origin_coordinates_established"] += 1
        stats["withdrawn_lines"] += account.get("withdrawn_lines", 0)
    else:
        stats["origin_coordinates_refused"] += 1
        findings.append(
            f"NOTE {where}: origin-coordinate derivation refused: "
            + ", ".join(r["ground"] for r in account.get("refusals", []))
        )

    old_functions = old.get("functions", [])
    new_functions = new.get("functions", [])
    if len(old_functions) != len(new_functions):
        findings.append(f"STOP {where}: function count moved")
        continue
    for old_function, new_function in zip(old_functions, new_functions):
        stats["functions"] += 1
        if old_function["function"] != new_function["function"]:
            findings.append(f"STOP {where}: function order moved")
            continue
        if old_function["status"] != new_function["status"]:
            findings.append(
                f"STOP {where}:{old_function['function']}: status "
                f"{old_function['status']} -> {new_function['status']}"
            )
        old_rows = old_function["diagnostics"]
        new_rows = new_function["diagnostics"]
        if len(old_rows) != len(new_rows):
            findings.append(
                f"STOP {where}:{old_function['function']}: "
                f"{len(old_rows)} -> {len(new_rows)} diagnostics"
            )
            continue
        # The map from preprocessed line to presented coordinate, as this unit
        # published it. Used to check every rewritten description.
        origins = {}
        for row in new_rows:
            if row.get("origin") is not None:
                origins[row["line"]] = row["origin"]
        for old_row, new_row in zip(old_rows, new_rows):
            stats["diagnostics"] += 1
            leaf = f"{where}:{old_function['function']}@L{old_row['line']}"
            if old_row["line"] != new_row["line"]:
                findings.append(f"STOP {leaf}: leaf line moved to {new_row['line']}")
            if old_row["category"] != new_row["category"]:
                findings.append(f"STOP {leaf}: category moved")
            if "origin" in old_row:
                findings.append(f"STOP {leaf}: the BEFORE run already carries an origin")
            if new_row.get("origin") is None:
                stats["origin_null"] += 1
            else:
                stats["origin_present"] += 1
            if old_row["description"] == new_row["description"]:
                stats["descriptions_unchanged"] += 1
                if SITE.search(old_row["description"]) and origins:
                    # A site text that could have been rewritten and was not is
                    # only admissible when that line has no coordinate.
                    line = int(SITE.search(old_row["description"]).group(1))
                    if line in origins:
                        findings.append(
                            f"STOP {leaf}: description keeps `preprocessed line {line}` "
                            f"although that line presents {origins[line]}"
                        )
                continue
            stats["descriptions_changed"] += 1
            match = SITE.search(old_row["description"])
            if not match:
                findings.append(
                    f"STOP {leaf}: description changed without a preprocessed site: "
                    f"{old_row['description']!r} -> {new_row['description']!r}"
                )
                continue
            line = int(match.group(1))
            prefix = old_row["description"][: match.start()]
            suffix = old_row["description"][match.end() :]
            if not (
                new_row["description"].startswith(prefix)
                and new_row["description"].endswith(suffix)
            ):
                findings.append(
                    f"STOP {leaf}: description changed outside its site: "
                    f"{old_row['description']!r} -> {new_row['description']!r}"
                )
                continue
            substituted = new_row["description"][
                len(prefix) : len(new_row["description"]) - len(suffix)
            ]
            if not COORDINATE.fullmatch(substituted):
                findings.append(
                    f"STOP {leaf}: site is not a `file:line` coordinate: {substituted!r}"
                )
                continue
            stats["sites_presented"] += 1
            # An acquisition line that *also* carries a diagnostic row of its own
            # publishes an `origin`; the two channels must agree.
            published = origins.get(line)
            if published is None:
                stats["sites_without_a_sibling_row"] += 1
            elif published != substituted:
                findings.append(
                    f"STOP {leaf}: site {substituted!r} disagrees with the row "
                    f"origin {published!r} published for the same line"
                )
            else:
                stats["sites_agreeing_with_a_sibling_row"] += 1

print(json.dumps(stats, indent=2))
print()
stops = [f for f in findings if f.startswith("STOP")]
notes = [f for f in findings if not f.startswith("STOP")]
for note in notes[:40]:
    print(note)
if len(notes) > 40:
    print(f"... {len(notes) - 40} more notes")
for stop in stops[:60]:
    print(stop)
if len(stops) > 60:
    print(f"... {len(stops) - 60} more STOP findings")
print()
print(f"STOP findings: {len(stops)}; notes: {len(notes)}")
sys.exit(1 if stops else 0)
