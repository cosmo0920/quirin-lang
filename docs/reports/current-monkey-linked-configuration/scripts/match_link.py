#!/usr/bin/env python3
"""The linked Monkey units the route analyses, matched to the link artifact.

usage: match_link.py ARTIFACT RUN_ROOT USER_PREPARATION OUT_JSON

ARTIFACT is `link-resolution.json` written by this card's `prepare --link`
under RUN_ROOT (the prepared output whose second configure equals the user's
database; FLUENT-BIT-LINK-PREPARE, "The configuration the user's run reads").
USER_PREPARATION is the user's preparation, whose compile database the
`current` run reads.

For every configuration that the artifact keys as a `monkey/` route unit, the
script records whether it is linked, whether its object was reproduced byte
for byte from its database command, and the digest of the linked input. It
then finds the user's database record of the same checkout-relative file and
checks three facts: its command, with the user's path prefix rewritten to
RUN_ROOT's, has the artifact's `command_sha256`; its directory is the
artifact's after the same rewrite; and the source bytes at the user's
checkout equal those of the run's clone. When all three hold, the unit the
user's run analyses has exactly the compile input whose object is the linked
one. Auxiliary reading of prepared records; it decides nothing.
"""
import hashlib
import json
import sys
from pathlib import Path

USER_PREFIX = "/home/cosmo/Documents/GitHub/qcore-c"
REPO = "/media/Data/GitHub/quirin-lang"

artifact_path, run_root, user_preparation, out = sys.argv[1:5]
artifact = json.load(open(artifact_path))
run_root = str(Path(run_root))
user_preparation = str(Path(user_preparation))
user_spelling = user_preparation.replace(REPO, USER_PREFIX)
user_database = json.load(open(Path(user_preparation) / "fluent-bit-clang-production/compile_commands.json"))
run_source = Path(run_root) / "fluent-bit-clang-source"
user_source = Path(user_preparation) / "fluent-bit-clang-source"


def checkout_relative(file, source_root):
    return str(Path(file).relative_to(source_root))


def to_run(text):
    return text.replace(user_spelling, run_root).replace(user_preparation, run_root)


inputs_by_configuration = {}
for linked_input in artifact["inputs"]:
    for configuration in linked_input["configurations"]:
        inputs_by_configuration.setdefault(configuration, []).append(linked_input)

user_by_file = {}
for record in user_database:
    file = record["file"].replace(USER_PREFIX, REPO)
    try:
        relative = checkout_relative(file, user_source)
    except ValueError:
        continue
    user_by_file.setdefault(relative, []).append(record)

rows = []
for configuration in artifact["configurations"]:
    unit = configuration.get("route_unit") or ""
    if not unit.startswith("monkey/"):
        continue
    relative = unit.split("/", 1)[1]
    records = user_by_file.get(relative, [])
    linked_inputs = inputs_by_configuration.get(configuration["id"], [])
    row = {
        "route_unit": unit,
        "linked": configuration["linked"],
        "reproduction": configuration["reproduction"],
        "linked_input_sha256": [item["sha256"] for item in linked_inputs],
        "artifact_command_sha256": configuration["command_sha256"],
        "user_records": len(records),
    }
    if len(records) == 1:
        record = records[0]
        command = record.get("command") or " ".join(record["arguments"])
        rewritten = to_run(command)
        row["user_command_sha256"] = hashlib.sha256(command.encode()).hexdigest()
        row["user_command_rewritten_sha256"] = hashlib.sha256(rewritten.encode()).hexdigest()
        row["command_matches"] = row["user_command_rewritten_sha256"] == configuration["command_sha256"]
        row["directory_matches"] = to_run(record["directory"]) == configuration["directory"]
        user_bytes = (user_source / relative).read_bytes()
        run_bytes = (run_source / relative).read_bytes()
        row["source_sha256"] = hashlib.sha256(user_bytes).hexdigest()
        row["source_matches"] = user_bytes == run_bytes
    row["holds"] = bool(
        row["linked"]
        and row["reproduction"] == "equal"
        and len(row["linked_input_sha256"]) == 1
        and row.get("command_matches")
        and row.get("directory_matches")
        and row.get("source_matches")
    )
    rows.append(row)

summary = {
    "artifact": artifact_path,
    "artifact_sha256": hashlib.sha256(Path(artifact_path).read_bytes()).hexdigest(),
    "artifact_compile_database_sha256": artifact["compile_database"]["sha256"],
    "run_root": run_root,
    "user_preparation": user_preparation,
    "route_selection": artifact.get("route_selection"),
    "monkey_route_units": len(rows),
    "all_hold": all(row["holds"] for row in rows) and bool(rows),
    "units": rows,
}
Path(out).write_text(json.dumps(summary, indent=1, sort_keys=True) + "\n")
print(f"monkey route units: {len(rows)}; all hold: {summary['all_hold']}")
for row in rows:
    print(row["route_unit"], "holds" if row["holds"] else "FAILS", row["reproduction"], row.get("command_matches"))
