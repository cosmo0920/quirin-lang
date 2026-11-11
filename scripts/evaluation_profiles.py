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

"""Evaluation profiles: records, cycle bindings and the frozen/live policy
(pluggable-cards seam A, card EVALUATION-PROFILES; user decision 2 of
2026-10-24).

This script is the single authority for three schemas and the policy check.

**Profile record** `contract/evaluations/<name>/evaluation.json`, schema
`quirin-evaluation-profile/1`, one JSON object with exactly these keys. (Not
`contract/profiles/<name>/`, the plan's spelling: the first-party provider
discovery in `compiler/src/first_party.rs` loads every directory there as a
source-set profile, and one without `profile.json` empties the registry.)

- `schema`: `quirin-evaluation-profile/1`.
- `name`: the directory name, `[a-z0-9][a-z0-9-]*`. An evaluation-profile
  directory never holds a source-set `profile.json`.
- `description`: what the profile evaluates.
- `policy`: `{"kind": "live"}`, or `{"kind": "frozen", "revision": <40-hex>}`
  where the revision is the analyzer revision the profile's last report
  measured. A frozen profile's figures describe the analyzer at that revision
  and say so; they are never re-measured.
- `route`: `{"kind": <ROUTE_KINDS>, "command": <text>}`, how the figures are
  produced.
- `source`: `{"repository", "revision", "submodules": {path: revision},
  "note"}`, the audited source (for the Fluent Bit profiles the Fluent Bit
  commit; vendored libraries are in-tree there, so `submodules` names only real
  git submodules the route reads).
- `inputs`: `{"tracked": [...], "untracked": [...]}`. A tracked input is
  `{"path", "sha256", "role"}`: for a live profile the digest is of the
  working tree's file; for a frozen profile it is the blob at
  `policy.revision` and the entry also carries `"revision"` equal to it. An
  untracked input (a prepared compile database, a private checkout) is
  `{"path", "sha256", "role"}` with `path` relative to the main checkout root
  as the route spells it, or `null` digest when the input is a directory or an
  external checkout identified by `source`; the measurement tool verifies these
  before a run, the tests cannot (constraint 29).
- `contract_set`: `{"lock_set", "references", "note"}`, the contracts the
  route loads. Pin-lock sets are tied to a Fluent Bit revision and only a
  window of them is retained (user decision 9a of 2026-10-26: the latest plus
  the two before it; CONTRACT-SUPERSESSION implements the window), so the
  record never makes a frozen profile depend on a live lock:
    - live: `{"state": "latest"}`, bound to the latest revision's lock set,
      that is the tree's provider locks; `references` names the tree's
      contract configuration and each path must exist;
    - frozen: `{"state": "retained", "fluent_bit_revision": R,
      "evidence_sha256": D}` while revision R's lock set is retained, or
      `{"state": "retired", "evidence_sha256": D}` once it is not (or never
      was); `references` is empty. `D` is `contract_evidence_digest` at
      `policy.revision`, read from git objects only, so retiring a lock set
      never breaks the check. Retiring is the one change a frozen record may
      make on its own: `retained` becomes `retired` with the same `D`.
- `baseline`: `{"path", "sha256"}`, the committed baseline record (schema
  below), normally `contract/evaluations/<name>/baseline.json`.
- `baseline_history` (live only): the append-only list of every baseline this
  profile has published, oldest first, each `{"sha256", "measured_revision",
  "reason", "source"}`; the last entry's digest equals `baseline.sha256`.
- `evidence_version`: `{"analyzer_revision", "semantic_evidence"}`, the
  evidence version of the baseline's figures.

**Baseline record** (`baseline.path`), schema `quirin-evaluation-baseline/1`:
`{"schema", "profile", "measured_revision", "source", "figures",
"artifacts"}`. `figures` is an object: for a live profile the exact output of
`summarize` (route totals plus row-level verdict digests); for a frozen
profile the figures transcribed from the report that measured it. `artifacts`
lists committed evidence files `{"path", "sha256"}` whose working-tree bytes
must stay identical.

**Bindings** `contract/evaluations/bindings.json`, schema
`quirin-evaluation-bindings/1`: `{"schema", "matrix", "pinned_cycle_documents",
"frozen_amendments"}`.
`matrix` classifies every key of the acceptance matrix's `corpus` section:
a profile name; `{"split": {"<json pointer below the key>": profile}}` for a
row that carries several routes (every key under a split's `routes` must be
bound); or `{"none": reason}` for a key that is not an evaluation result
(contract-inventory pins, for example). Every `docs/cycles/CYCLE_*.md` names
exactly one profile, or `none`, on one line of the form

    Evaluation profile: `<name>` (<policy>; [record](...)).
    Evaluation profile: none (<reason>).

A cycle document whose bytes another record pins (today
`CYCLE_18_LIBEVENT_VALIDATION.md`, whose digest `native_platform_matrix.qv`
pins) carries no line; `pinned_cycle_documents` binds it instead, as
`{path: {"profile": name or null, "reason"}}`.

`frozen_amendments` lists reviewed corrections of a frozen profile as
content-addressed transitions `{"profile", "path", "from_sha256",
"to_sha256", "reason"}`; nothing else may change a frozen profile's files.

**Policy** (checked over git history by `check --history`; every commit that
touches an evaluation-profile directory is compared with its first parent):

- a frozen profile's files never change, and it never becomes live or
  disappears, except by a listed frozen amendment or by its lock set's
  retirement (`retained` to `retired`, same evidence digest, nothing else);
- a live profile's baseline changes only in a commit that also appends to its
  `baseline_history` an entry with the new digest and a non-empty `reason`;
  history is append-only, an entry is appended only with a baseline change,
  and a change of a live profile's tracked-input digests (a re-base) is a
  baseline change too;
- a live profile may be frozen (`policy` becomes frozen at a revision); it is
  never deleted.

**Commands**

    evaluation_profiles.py check [--history] [--root DIR]
    evaluation_profiles.py summarize --report R --null N
    evaluation_profiles.py compare <profile> --report R --null N
    evaluation_profiles.py record <profile> --report R --null N --revision REV
                                  --reason TEXT --source PATH
    evaluation_profiles.py amendments <integration-amendments.json> [--base REV]

`compare` exits 0 when a run reproduces a live profile's baseline exactly, 1
when the results moved (record the new baseline in the same commit, with a
reason, or the move is a defect), and 2 for a frozen profile: its results never
move, and a run on another analyzer revision is not that profile. `record`
writes a live baseline and its history entry. `amendments` checks the
amendments field `profiles[]` (schema `quirin-amendments/1`): the live profiles
whose results the card's measured tip moves. Every name is a profile; none is
frozen; with `--base`, the listed set equals the set of live profiles whose
baseline changed since the base. `scripts/apply_amendments.py` runs the same
`check_amendments`, with the document's base, on every run (card
RECORDS-TOOLING).
"""
from __future__ import annotations

import argparse
import collections
import copy
import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parents[1]
PROFILES = PurePosixPath("contract/evaluations")
RECORD = "evaluation.json"
BINDINGS = PROFILES / "bindings.json"
MATRIX = PurePosixPath("docs/reviews/r0_3/acceptance_matrix.json")
CYCLES = PurePosixPath("docs/cycles")
PROFILE_SCHEMA = "quirin-evaluation-profile/1"
BASELINE_SCHEMA = "quirin-evaluation-baseline/1"
BINDINGS_SCHEMA = "quirin-evaluation-bindings/1"
NAME_RE = re.compile(r"[a-z0-9][a-z0-9-]*")
SHA_RE = re.compile(r"[0-9a-f]{64}")
REVISION_RE = re.compile(r"[0-9a-f]{40}")
SHORT_REVISION_RE = re.compile(r"[0-9a-f]{7,40}")
ROUTE_KINDS = ("pinned-corpus", "configured-selection", "configured-source-set",
               "source-contract-audit", "unrestricted-import-probe", "callback-coverage")
RECORD_KEYS = ("schema", "name", "description", "policy", "route", "source", "inputs",
               "contract_set", "baseline", "evidence_version")
LIVE_KEYS = RECORD_KEYS + ("baseline_history",)
BASELINE_KEYS = ("schema", "profile", "measured_revision", "source", "figures", "artifacts")
CYCLE_LINE_RE = re.compile(
    r"^Evaluation profile: (?:`(?P<name>[a-z0-9][a-z0-9-]*)` \((?P<policy>[^)]*)|none \((?P<reason>.+)\)\.$)")


class ProfileError(Exception):
    """A refusal of the profile records, bindings or policy."""


# --------------------------------------------------------------------------
# git access (read-only; fail-closed)


def _git(root, *args, check=True):
    environment = {key: value for key, value in os.environ.items()
                   if key not in ("GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE")}
    try:
        result = subprocess.run(["git", "-C", str(root), *args], capture_output=True,
                                env=environment)
    except OSError as error:
        raise ProfileError(f"git could not run ({error}); the profile check needs a git "
                           "checkout and never walks the working tree instead") from error
    if check and result.returncode != 0:
        raise ProfileError(f"`git {' '.join(args)}` failed (exit {result.returncode}): "
                           + result.stderr.decode(errors="replace").strip())
    return result


def tracked(root, pathspec):
    out = _git(root, "ls-files", "-z", "--cached", "--", pathspec).stdout
    return sorted(entry.decode() for entry in out.split(b"\0") if entry)


def blob_at(root, revision, path):
    """Bytes of `path` at `revision`, or None when the path is absent there."""
    result = _git(root, "cat-file", "blob", f"{revision}:{path}", check=False)
    if result.returncode != 0:
        if _git(root, "cat-file", "-e", f"{revision}^{{commit}}", check=False).returncode != 0:
            raise ProfileError(f"revision {revision} is not in this checkout's object store "
                               "(a shallow clone or an export cannot check frozen profiles)")
        return None
    return result.stdout


def sha256(data):
    return hashlib.sha256(data).hexdigest()


CONTRACT_SUFFIXES = (".qv", ".qcore", "contracts.lock", "contract-load-path.json")


def contract_evidence_digest(root, revision):
    """The contract set of `revision`, read from git objects only: sha256 of
    the sorted lines `<path>\\t<blob id>` of every file tracked at that
    revision outside third_party/ whose name ends in .qv, .qcore,
    contracts.lock (any lock prefix) or contract-load-path.json. It needs no
    lock file in the working tree, so a retired lock set never breaks it."""
    out = _git(root, "ls-tree", "-r", "--full-tree", revision).stdout.decode()
    lines = []
    for line in out.splitlines():
        meta, path = line.split("\t", 1)
        if path.startswith("third_party/") or not path.endswith(CONTRACT_SUFFIXES):
            continue
        lines.append(f"{path}\t{meta.split()[2]}")
    # A revision before any contract file existed (Cycle 17) has the empty
    # set, whose digest is that of the empty listing.
    return sha256("\n".join(sorted(lines)).encode())


# --------------------------------------------------------------------------
# schema validation


def _text(value, where):
    if not isinstance(value, str) or not value.strip() or value != value.strip():
        raise ProfileError(f"{where} must be a non-empty string without surrounding whitespace")


def _keys(value, required, where, optional=()):
    if not isinstance(value, dict):
        raise ProfileError(f"{where} must be an object")
    missing = [key for key in required if key not in value]
    extra = sorted(set(value) - set(required) - set(optional))
    if missing:
        raise ProfileError(f"{where} lacks {missing}")
    if extra:
        raise ProfileError(f"{where} has unexpected key(s) {extra}")


def _sha(value, where, nullable=False):
    if value is None and nullable:
        return
    if not isinstance(value, str) or not SHA_RE.fullmatch(value):
        raise ProfileError(f"{where} must be a sha256 hex digest")


def _relative(value, where):
    _text(value, where)
    path = PurePosixPath(value)
    if path.is_absolute() or ".." in path.parts or value.startswith("./"):
        raise ProfileError(f"{where} {value!r} must be a repository-relative path")


def validate_record(record, directory_name):
    """Check one profile record's shape; raise ProfileError on any defect."""
    if not isinstance(record, dict):
        raise ProfileError(f"{directory_name}: the record must be a JSON object")
    policy = record.get("policy")
    live = isinstance(policy, dict) and policy.get("kind") == "live"
    where = f"profile {directory_name}"
    _keys(record, LIVE_KEYS if live else RECORD_KEYS, where)
    if record["schema"] != PROFILE_SCHEMA:
        raise ProfileError(f"{where}: schema is {record['schema']!r}; expected {PROFILE_SCHEMA!r}")
    if record["name"] != directory_name or not NAME_RE.fullmatch(directory_name):
        raise ProfileError(f"{where}: name {record['name']!r} must equal its directory and "
                           f"match {NAME_RE.pattern}")
    _text(record["description"], f"{where}.description")
    if live:
        _keys(policy, ("kind",), f"{where}.policy")
    else:
        _keys(policy, ("kind", "revision"), f"{where}.policy")
        if policy["kind"] != "frozen":
            raise ProfileError(f"{where}.policy.kind must be 'live' or 'frozen'")
        if not isinstance(policy["revision"], str) or not REVISION_RE.fullmatch(policy["revision"]):
            raise ProfileError(f"{where}.policy.revision must be a 40-hex analyzer revision")
    route = record["route"]
    _keys(route, ("kind", "command"), f"{where}.route")
    if route["kind"] not in ROUTE_KINDS:
        raise ProfileError(f"{where}.route.kind {route['kind']!r} must be one of {list(ROUTE_KINDS)}")
    _text(route["command"], f"{where}.route.command")
    source = record["source"]
    _keys(source, ("repository", "revision", "submodules", "note"), f"{where}.source")
    _text(source["repository"], f"{where}.source.repository")
    if not isinstance(source["revision"], str) or not SHORT_REVISION_RE.fullmatch(source["revision"]):
        raise ProfileError(f"{where}.source.revision must be a git revision")
    if not isinstance(source["submodules"], dict):
        raise ProfileError(f"{where}.source.submodules must be an object")
    for path, revision in source["submodules"].items():
        _relative(path, f"{where}.source.submodules key")
        if not isinstance(revision, str) or not REVISION_RE.fullmatch(revision):
            raise ProfileError(f"{where}.source.submodules[{path!r}] must be a 40-hex revision")
    _text(source["note"], f"{where}.source.note")
    inputs = record["inputs"]
    _keys(inputs, ("tracked", "untracked"), f"{where}.inputs")
    for index, entry in enumerate(inputs["tracked"]):
        at = f"{where}.inputs.tracked[{index}]"
        if live:
            _keys(entry, ("path", "sha256", "role"), at)
        else:
            _keys(entry, ("path", "revision", "sha256", "role"), at)
            if entry["revision"] != policy["revision"]:
                raise ProfileError(f"{at}.revision must equal the frozen revision")
        _relative(entry["path"], f"{at}.path")
        _sha(entry["sha256"], f"{at}.sha256")
        _text(entry["role"], f"{at}.role")
    for index, entry in enumerate(inputs["untracked"]):
        at = f"{where}.inputs.untracked[{index}]"
        _keys(entry, ("path", "sha256", "role"), at)
        _relative(entry["path"], f"{at}.path")
        _sha(entry["sha256"], f"{at}.sha256", nullable=True)
        _text(entry["role"], f"{at}.role")
    contract_set = record["contract_set"]
    _keys(contract_set, ("lock_set", "references", "note"), f"{where}.contract_set")
    if not isinstance(contract_set["references"], list):
        raise ProfileError(f"{where}.contract_set.references must be a list")
    for index, path in enumerate(contract_set["references"]):
        _relative(path, f"{where}.contract_set.references[{index}]")
    _text(contract_set["note"], f"{where}.contract_set.note")
    lock_set = contract_set["lock_set"]
    at = f"{where}.contract_set.lock_set"
    if live:
        _keys(lock_set, ("state",), at)
        if lock_set["state"] != "latest":
            raise ProfileError(f"{at}: a live profile binds to the latest lock set")
        if not contract_set["references"]:
            raise ProfileError(f"{where}.contract_set.references must name the tree's contract "
                               "configuration")
    else:
        state = lock_set.get("state") if isinstance(lock_set, dict) else None
        if state == "retained":
            _keys(lock_set, ("state", "fluent_bit_revision", "evidence_sha256"), at)
            if not isinstance(lock_set["fluent_bit_revision"], str) or not REVISION_RE.fullmatch(
                    lock_set["fluent_bit_revision"]):
                raise ProfileError(f"{at}.fluent_bit_revision must be a 40-hex revision")
        elif state == "retired":
            _keys(lock_set, ("state", "evidence_sha256"), at)
        else:
            raise ProfileError(f"{at}.state must be 'retained' or 'retired' for a frozen profile")
        _sha(lock_set["evidence_sha256"], f"{at}.evidence_sha256")
        if contract_set["references"]:
            raise ProfileError(f"{where}.contract_set.references must be empty: a frozen profile "
                               "never depends on a live file")
    baseline = record["baseline"]
    _keys(baseline, ("path", "sha256"), f"{where}.baseline")
    _relative(baseline["path"], f"{where}.baseline.path")
    _sha(baseline["sha256"], f"{where}.baseline.sha256")
    evidence = record["evidence_version"]
    _keys(evidence, ("analyzer_revision", "semantic_evidence"), f"{where}.evidence_version")
    if not isinstance(evidence["analyzer_revision"], str) or not REVISION_RE.fullmatch(
            evidence["analyzer_revision"]):
        raise ProfileError(f"{where}.evidence_version.analyzer_revision must be a 40-hex revision")
    _text(evidence["semantic_evidence"], f"{where}.evidence_version.semantic_evidence")
    if not live and evidence["analyzer_revision"] != policy["revision"]:
        raise ProfileError(f"{where}: a frozen profile's evidence is the frozen revision's")
    if live:
        history = record["baseline_history"]
        if not isinstance(history, list) or not history:
            raise ProfileError(f"{where}.baseline_history must be a non-empty list")
        for index, entry in enumerate(history):
            validate_history_entry(entry, f"{where}.baseline_history[{index}]")
        if history[-1]["sha256"] != baseline["sha256"]:
            raise ProfileError(f"{where}: the last baseline_history entry must be the current "
                               "baseline")
        if history[-1]["measured_revision"] != evidence["analyzer_revision"]:
            raise ProfileError(f"{where}: evidence_version.analyzer_revision must be the current "
                               "baseline's measured revision")


def validate_history_entry(entry, where):
    _keys(entry, ("sha256", "measured_revision", "reason", "source"), where)
    _sha(entry["sha256"], f"{where}.sha256")
    if not isinstance(entry["measured_revision"], str) or not REVISION_RE.fullmatch(
            entry["measured_revision"]):
        raise ProfileError(f"{where}.measured_revision must be a 40-hex revision")
    _text(entry["reason"], f"{where}.reason")
    _relative(entry["source"], f"{where}.source")


def validate_baseline(baseline, record):
    where = f"baseline of {record['name']}"
    _keys(baseline, BASELINE_KEYS, where)
    if baseline["schema"] != BASELINE_SCHEMA:
        raise ProfileError(f"{where}: schema is {baseline['schema']!r}; expected {BASELINE_SCHEMA!r}")
    if baseline["profile"] != record["name"]:
        raise ProfileError(f"{where}: profile {baseline['profile']!r} is not {record['name']!r}")
    if baseline["measured_revision"] != record["evidence_version"]["analyzer_revision"]:
        raise ProfileError(f"{where}: measured_revision differs from the record's evidence version")
    _relative(baseline["source"], f"{where}.source")
    if not isinstance(baseline["figures"], dict) or not baseline["figures"]:
        raise ProfileError(f"{where}.figures must be a non-empty object")
    if not isinstance(baseline["artifacts"], list):
        raise ProfileError(f"{where}.artifacts must be a list")
    for index, entry in enumerate(baseline["artifacts"]):
        _keys(entry, ("path", "sha256"), f"{where}.artifacts[{index}]")
        _relative(entry["path"], f"{where}.artifacts[{index}].path")
        _sha(entry["sha256"], f"{where}.artifacts[{index}].sha256")


# --------------------------------------------------------------------------
# the committed tree


def load_json_bytes(data, where):
    try:
        return json.loads(data)
    except (ValueError, UnicodeDecodeError) as error:
        raise ProfileError(f"{where} is not JSON: {error}") from error


def profile_names(root):
    names = []
    for path in tracked(root, str(PROFILES)):
        parts = PurePosixPath(path).parts
        if len(parts) == 4 and parts[3] == RECORD:
            names.append(parts[2])
    return names


def load_profiles(root):
    root = Path(root)
    profiles = {}
    for name in profile_names(root):
        directory = root / PROFILES / name
        record = load_json_bytes((directory / RECORD).read_bytes(), f"{PROFILES}/{name}/{RECORD}")
        validate_record(record, name)
        profiles[name] = record
    return profiles


def check_tree(root):
    """Check every committed profile record against the working tree."""
    root = Path(root)
    profiles = load_profiles(root)
    if not profiles:
        raise ProfileError(f"no evaluation profile is tracked under {PROFILES}")
    for name, record in profiles.items():
        directory = PurePosixPath(PROFILES) / name
        if (root / directory / "profile.json").exists():
            raise ProfileError(f"{directory} holds a source-set profile.json; an evaluation "
                               "profile uses its own directory")
        frozen = record["policy"]["kind"] == "frozen"
        baseline_path = root / record["baseline"]["path"]
        if not baseline_path.is_file():
            raise ProfileError(f"{name}: baseline {record['baseline']['path']} is missing")
        data = baseline_path.read_bytes()
        if sha256(data) != record["baseline"]["sha256"]:
            raise ProfileError(f"{name}: baseline {record['baseline']['path']} does not match the "
                               "record's digest")
        baseline = load_json_bytes(data, record["baseline"]["path"])
        validate_baseline(baseline, record)
        for entry in baseline["artifacts"]:
            path = root / entry["path"]
            if not path.is_file() or sha256(path.read_bytes()) != entry["sha256"]:
                raise ProfileError(f"{name}: baseline artifact {entry['path']} is missing or its "
                                   "bytes changed")
        for entry in record["inputs"]["tracked"]:
            if frozen:
                data = blob_at(root, entry["revision"], entry["path"])
                if data is None or sha256(data) != entry["sha256"]:
                    raise ProfileError(f"{name}: input {entry['path']} at {entry['revision']} does "
                                       "not have the recorded digest")
            else:
                path = root / entry["path"]
                if not path.is_file() or sha256(path.read_bytes()) != entry["sha256"]:
                    raise ProfileError(
                        f"{name}: live input {entry['path']} changed without the profile: a "
                        "change of a live profile's inputs is a re-base and records a new "
                        "baseline in the same commit")
        for path in record["contract_set"]["references"]:
            if not (root / path).exists():
                raise ProfileError(f"{name}: contract reference {path} does not exist")
        if frozen and blob_at(root, record["policy"]["revision"], "Cargo.toml") is None:
            raise ProfileError(f"{name}: frozen revision {record['policy']['revision']} is not a "
                               "revision of this repository")
        if frozen and (contract_evidence_digest(root, record["policy"]["revision"])
                       != record["contract_set"]["lock_set"]["evidence_sha256"]):
            raise ProfileError(f"{name}: contract evidence digest differs from the contract set "
                               f"at {record['policy']['revision']}")
    return profiles


def check_bindings(root, profiles):
    """Every cycle document and every corpus row of the matrix names one profile."""
    root = Path(root)
    bindings = load_json_bytes((root / BINDINGS).read_bytes(), str(BINDINGS))
    _keys(bindings, ("schema", "matrix", "pinned_cycle_documents", "frozen_amendments"),
          str(BINDINGS))
    if bindings["schema"] != BINDINGS_SCHEMA:
        raise ProfileError(f"{BINDINGS}: schema must be {BINDINGS_SCHEMA!r}")
    for index, entry in enumerate(bindings["frozen_amendments"]):
        at = f"{BINDINGS}.frozen_amendments[{index}]"
        _keys(entry, ("profile", "path", "from_sha256", "to_sha256", "reason"), at)
        _sha(entry["from_sha256"], f"{at}.from_sha256")
        _sha(entry["to_sha256"], f"{at}.to_sha256")
        _relative(entry["path"], f"{at}.path")
        _text(entry["reason"], f"{at}.reason")
    used = collections.Counter()
    cycles = [path for path in tracked(root, str(CYCLES))
              if re.fullmatch(r"docs/cycles/CYCLE_[0-9A-Z_]+\.md", path)]
    if not cycles:
        raise ProfileError("no cycle document is tracked under docs/cycles")
    pinned = bindings["pinned_cycle_documents"]
    if not isinstance(pinned, dict):
        raise ProfileError(f"{BINDINGS}.pinned_cycle_documents must be an object")
    for path, entry in pinned.items():
        at = f"{BINDINGS}.pinned_cycle_documents[{path!r}]"
        _keys(entry, ("profile", "reason"), at)
        _text(entry["reason"], f"{at}.reason")
        if path not in cycles:
            raise ProfileError(f"{at} names no tracked cycle document")
        if entry["profile"] is not None:
            if entry["profile"] not in profiles:
                raise ProfileError(f"{at} names unknown profile {entry['profile']!r}")
            used[entry["profile"]] += 1
    for path in cycles:
        lines = [line for line in (root / path).read_text().splitlines()
                 if line.startswith("Evaluation profile:")]
        if path in pinned:
            if lines:
                raise ProfileError(f"{path} is bound in {BINDINGS} (its bytes are pinned "
                                   "elsewhere) and must not also carry a binding line")
            continue
        if len(lines) != 1:
            raise ProfileError(f"{path} must name exactly one evaluation profile (or none) on one "
                               f"`Evaluation profile:` line; found {len(lines)}")
        match = CYCLE_LINE_RE.match(lines[0])
        if not match:
            raise ProfileError(f"{path}: malformed binding line {lines[0]!r}")
        name = match.group("name")
        if name is not None:
            if name not in profiles:
                raise ProfileError(f"{path} names unknown profile {name!r}")
            policy = profiles[name]["policy"]
            expected = ("live" if policy["kind"] == "live"
                        else f"frozen at `{policy['revision'][:9]}`")
            if not match.group("policy").startswith(expected):
                raise ProfileError(f"{path}: the binding must state the policy {expected!r}")
            used[name] += 1
    matrix = load_json_bytes((root / MATRIX).read_bytes(), str(MATRIX))
    corpus = matrix.get("corpus")
    if not isinstance(corpus, dict):
        raise ProfileError(f"{MATRIX} has no corpus section")
    table = bindings["matrix"]
    if not isinstance(table, dict):
        raise ProfileError(f"{BINDINGS}.matrix must be an object")
    unbound = sorted(set(corpus) - set(table))
    stale = sorted(set(table) - set(corpus))
    if unbound:
        raise ProfileError(f"matrix corpus key(s) {unbound} bind to no evaluation profile; add "
                           f"each to {BINDINGS} (a profile, a split by route, or none with a reason)")
    if stale:
        raise ProfileError(f"{BINDINGS} binds matrix key(s) {stale} the matrix no longer has")
    for key, value in table.items():
        where = f"{BINDINGS}.matrix[{key!r}]"
        if isinstance(value, str):
            if value not in profiles:
                raise ProfileError(f"{where} names unknown profile {value!r}")
            used[value] += 1
        elif isinstance(value, dict) and set(value) == {"none"}:
            _text(value["none"], f"{where}.none")
        elif isinstance(value, dict) and set(value) == {"split"}:
            split = value["split"]
            if not isinstance(split, dict) or not split:
                raise ProfileError(f"{where}.split must be a non-empty object")
            for pointer, name in split.items():
                target = corpus[key]
                for part in pointer.split("/"):
                    if not isinstance(target, dict) or part not in target:
                        raise ProfileError(f"{where}: pointer {pointer!r} is absent from the row")
                    target = target[part]
                if name not in profiles:
                    raise ProfileError(f"{where}[{pointer!r}] names unknown profile {name!r}")
                used[name] += 1
            routes = corpus[key].get("routes") if isinstance(corpus[key], dict) else None
            if isinstance(routes, dict):
                missing = sorted(f"routes/{route}" for route in routes
                                 if f"routes/{route}" not in split)
                if missing:
                    raise ProfileError(f"{where}: route(s) {missing} bind to no profile")
        else:
            raise ProfileError(f"{where} must be a profile name, {{'split': ...}} or "
                               "{'none': reason}")
    orphans = sorted(set(profiles) - set(used))
    if orphans:
        raise ProfileError(f"profile(s) {orphans} are bound by no cycle or matrix row")
    return bindings


# --------------------------------------------------------------------------
# the policy over history


def _profile_dir(path):
    parts = PurePosixPath(path).parts
    if len(parts) >= 4 and PurePosixPath(*parts[:2]) == PROFILES:
        return parts[2]
    return None


def evaluation_directories(root):
    """Every directory under contract/evaluations that has ever held a record."""
    out = _git(root, "log", "--format=", "--name-only", "--diff-merges=first-parent",
               "--no-renames", "--", f":(glob){PROFILES}/*/{RECORD}").stdout.decode()
    names = {_profile_dir(line) for line in out.splitlines() if line.strip()}
    names.update(profile_names(root))
    return sorted(name for name in names if name)


def commits_touching(root):
    """(commit, first parent or None, changed paths) for every commit, merges
    against their first parent, that touches an evaluation-profile directory
    (source-set directories, which never hold a record, are not walked)."""
    names = evaluation_directories(root)
    if not names:
        return []
    out = _git(root, "log", "--format=%x01%H %P", "--name-only", "--diff-merges=first-parent",
               "--no-renames", "--", *(f"{PROFILES}/{name}" for name in names)).stdout.decode()
    commits = []
    for block in out.split("\x01")[1:]:
        lines = [line for line in block.splitlines() if line.strip()]
        head = lines[0].split()
        commits.append((head[0], head[1] if len(head) > 1 else None, lines[1:]))
    return list(reversed(commits))


def _record_at(root, revision, name):
    if revision is None:
        return None
    data = blob_at(root, revision, f"{PROFILES}/{name}/{RECORD}")
    return None if data is None else load_json_bytes(data, f"{name}/{RECORD}@{revision}")


def _is_retirement(old, new):
    """True when `new` differs from frozen `old` only by its lock set moving
    from retained to retired with the same evidence digest (decision 9a)."""
    before = old.get("contract_set", {}).get("lock_set", {})
    after = new.get("contract_set", {}).get("lock_set", {})
    if before.get("state") != "retained" or after != {
            "state": "retired", "evidence_sha256": before.get("evidence_sha256")}:
        return False
    rest_old, rest_new = copy.deepcopy(old), copy.deepcopy(new)
    del rest_old["contract_set"]["lock_set"], rest_new["contract_set"]["lock_set"]
    return rest_old == rest_new


def check_transition(name, old, new, changed, blobs, amendments, where):
    """Apply the frozen/live policy to one profile across one commit.

    `old`/`new` are the records before/after (None when absent), `changed`
    the paths of the profile's directory that the commit changed, `blobs` a
    function (side, path) -> bytes or None, `amendments` the frozen
    amendments in force."""
    if old is None:
        if new is not None:
            validate_record(new, name)
        return
    if old["policy"]["kind"] == "frozen":
        record_path = f"{PROFILES}/{name}/{RECORD}"
        if new is not None and changed == [record_path] and _is_retirement(old, new):
            validate_record(new, name)
            return
        for path in changed:
            before, after = blobs("old", path), blobs("new", path)
            allowed = any(entry["profile"] == name and entry["path"] == path
                          and before is not None and after is not None
                          and entry["from_sha256"] == sha256(before)
                          and entry["to_sha256"] == sha256(after) for entry in amendments)
            if not allowed:
                raise ProfileError(f"{where}: frozen profile {name} changed ({path}); a frozen "
                                   "profile's results, inputs and record never move")
        return
    if new is None:
        raise ProfileError(f"{where}: live profile {name} was removed; freeze it instead")
    validate_record(new, name)
    if new["policy"]["kind"] == "frozen":
        return
    old_history, new_history = old["baseline_history"], new["baseline_history"]
    if new_history[:len(old_history)] != old_history:
        raise ProfileError(f"{where}: {name}'s baseline_history was rewritten; it is append-only")
    appended = new_history[len(old_history):]
    baseline_moved = old["baseline"]["sha256"] != new["baseline"]["sha256"]
    old_inputs = {entry["path"]: entry["sha256"] for entry in old["inputs"]["tracked"]}
    new_inputs = {entry["path"]: entry["sha256"] for entry in new["inputs"]["tracked"]}
    if old_inputs != new_inputs and not baseline_moved:
        raise ProfileError(f"{where}: {name}'s inputs changed without a new baseline; a re-base "
                           "records the baseline measured on the new inputs in the same commit")
    if baseline_moved and not appended:
        raise ProfileError(f"{where}: live profile {name}'s baseline moved without a "
                           "baseline_history entry carrying the reason")
    if appended and not baseline_moved:
        raise ProfileError(f"{where}: {name} appended baseline history without a baseline change")
    baseline_path = new["baseline"]["path"]
    if baseline_path in changed and not baseline_moved:
        raise ProfileError(f"{where}: {name}'s baseline file changed but its record did not; "
                           "a live baseline moves only with a baseline_history entry")
    if baseline_moved and baseline_path not in changed:
        data = blobs("new", baseline_path)
        if data is None or sha256(data) != new["baseline"]["sha256"]:
            raise ProfileError(f"{where}: {name}'s new baseline is not in the commit")


def check_history(root):
    root = Path(root)
    bindings_now = load_json_bytes((root / BINDINGS).read_bytes(), str(BINDINGS))
    amendments = bindings_now.get("frozen_amendments", [])
    checked = 0
    for commit, parent, paths in commits_touching(root):
        by_profile = collections.defaultdict(list)
        for path in paths:
            name = _profile_dir(path)
            if name is not None:
                by_profile[name].append(path)
        for name, changed in by_profile.items():
            old = _record_at(root, parent, name)
            new = _record_at(root, commit, name)
            if old is None and new is None:
                continue  # a source-set directory, not an evaluation profile

            def blobs(side, path, commit=commit, parent=parent):
                revision = parent if side == "old" else commit
                return None if revision is None else blob_at(root, revision, path)

            check_transition(name, old, new, changed, blobs, amendments, f"commit {commit[:12]}")
            checked += 1
    return checked


# --------------------------------------------------------------------------
# route results


def summarize(report_path, null_path):
    """Route totals and row-level verdict digests of one run (deterministic).

    Rows are keyed by (library, unit file, unit occurrence, function,
    occurrence), as the identity comparisons of AUDIT-CANONICAL-PATHS key
    them; a null row by (library, file, function, occurrence)."""
    report = json.loads(Path(report_path).read_bytes())
    units, rows, seen = [], [], collections.Counter()
    for unit in report["units"]:
        key = (unit["library"], unit["file"])
        occurrence = seen[key]
        seen[key] += 1
        units.append([unit["library"], unit["file"], occurrence, unit["status"],
                      unit.get("reason_code")])
        local = collections.Counter()
        for row in unit.get("functions", []):
            rows.append([unit["library"], unit["file"], occurrence, row["function"],
                         local[row["function"]], row["status"]])
            local[row["function"]] += 1
    nulls, null_seen = [], collections.Counter()
    for line in Path(null_path).read_text().splitlines():
        if not line.strip():
            continue
        row = json.loads(line)
        key = (row.get("library"), row.get("file"), row.get("function"))
        null = row.get("null") or {}
        nulls.append([*key, null_seen[key], row.get("custody"), row.get("combined"),
                      null.get("verdict"), null.get("scope"), null.get("returns_non_null"),
                      [check.get("outcome") for check in null.get("checks", [])]])
        null_seen[key] += 1

    def digest(items):
        return sha256(json.dumps(sorted(items, key=json.dumps), separators=(",", ":"))
                      .encode())

    def tally(values):
        return dict(sorted(collections.Counter(values).items(), key=lambda item: str(item[0])))

    return {
        "units": len(units),
        "unit_statuses": tally(unit[3] for unit in units),
        "unit_verdicts_sha256": digest(units),
        "rows": len(rows),
        "row_statuses": tally(row[5] for row in rows),
        "row_verdicts_sha256": digest(rows),
        "null_rows": len(nulls),
        "null_verdicts": tally(str(row[6]) for row in nulls),
        "null_verdicts_sha256": digest(nulls),
    }


def _load_profile(root, name):
    profiles = load_profiles(root)
    if name not in profiles:
        raise ProfileError(f"unknown profile {name!r}; profiles: {sorted(profiles)}")
    return profiles[name]


def compare(root, name, figures):
    """(exit code, differences) of a run's figures against the profile."""
    record = _load_profile(root, name)
    if record["policy"]["kind"] == "frozen":
        return 2, [f"{name} is frozen at {record['policy']['revision']}: its results never "
                   "move, and a run on another analyzer revision is not this profile"]
    baseline = json.loads((Path(root) / record["baseline"]["path"]).read_bytes())
    expected = baseline["figures"]
    differences = [f"{key}: baseline {expected.get(key)!r}, run {figures.get(key)!r}"
                   for key in sorted(set(expected) | set(figures))
                   if expected.get(key) != figures.get(key)]
    return (1 if differences else 0), differences


def record_baseline(root, name, figures, revision, reason, source):
    """Write a live profile's new baseline and append its history entry."""
    root = Path(root)
    record = _load_profile(root, name)
    if record["policy"]["kind"] != "live":
        raise ProfileError(f"{name} is frozen; its baseline never moves")
    if not REVISION_RE.fullmatch(revision):
        raise ProfileError("--revision must be a 40-hex analyzer revision")
    _text(reason, "--reason")
    _relative(source, "--source")
    baseline = {"schema": BASELINE_SCHEMA, "profile": name, "measured_revision": revision,
                "source": source, "figures": figures, "artifacts": []}
    data = (json.dumps(baseline, indent=2) + "\n").encode()
    digest = sha256(data)
    if digest == record["baseline"]["sha256"]:
        raise ProfileError(f"{name}: the run reproduces the current baseline; nothing to record")
    (root / record["baseline"]["path"]).write_bytes(data)
    record["baseline"]["sha256"] = digest
    record["evidence_version"]["analyzer_revision"] = revision
    record["baseline_history"].append({"sha256": digest, "measured_revision": revision,
                                       "reason": reason, "source": source})
    validate_record(record, name)
    (root / PROFILES / name / RECORD).write_text(json.dumps(record, indent=2) + "\n")
    return digest


# --------------------------------------------------------------------------
# integration amendments: profiles[]


def check_amendments(root, document, base=None):
    """Check `profiles[]` of a quirin-amendments/1 document (see module doc)."""
    root = Path(root)
    profiles = load_profiles(root)
    listed = document.get("profiles", [])
    if not isinstance(listed, list) or len(set(listed)) != len(listed):
        raise ProfileError("profiles must be a list of distinct profile names")
    for name in listed:
        if name not in profiles:
            raise ProfileError(f"profiles names unknown evaluation profile {name!r}")
        if profiles[name]["policy"]["kind"] == "frozen":
            raise ProfileError(f"profiles names frozen profile {name!r}; a card never moves a "
                               "frozen profile's results")
    if base is None:
        return sorted(listed)
    moved = []
    for name, record in profiles.items():
        if record["policy"]["kind"] != "live":
            continue
        old = _record_at(root, base, name)
        if old is None or old["baseline"]["sha256"] != record["baseline"]["sha256"]:
            moved.append(name)
    if sorted(moved) != sorted(listed):
        raise ProfileError(f"profiles lists {sorted(listed)}, but the live profiles whose "
                           f"baseline changed since {base} are {sorted(moved)}")
    return sorted(listed)


# --------------------------------------------------------------------------
# command line


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--root", type=Path, default=ROOT)
    sub = parser.add_subparsers(dest="command", required=True)
    check = sub.add_parser("check", help="check records, bindings and (with --history) policy")
    check.add_argument("--history", action="store_true")
    summary = sub.add_parser("summarize", help="print a run's figures")
    for command in (summary, sub.add_parser("compare"), sub.add_parser("record")):
        command.add_argument("--report", type=Path, required=True)
        command.add_argument("--null", type=Path, required=True)
    for name in ("compare", "record"):
        sub.choices[name].add_argument("profile")
    sub.choices["record"].add_argument("--revision", required=True)
    sub.choices["record"].add_argument("--reason", required=True)
    sub.choices["record"].add_argument("--source", required=True)
    amend = sub.add_parser("amendments", help="check an amendments file's profiles[]")
    amend.add_argument("file", type=Path)
    amend.add_argument("--base")
    args = parser.parse_args(argv)
    try:
        if args.command == "check":
            profiles = check_tree(args.root)
            check_bindings(args.root, profiles)
            counts = collections.Counter(p["policy"]["kind"] for p in profiles.values())
            line = f"evaluation profiles: {counts['live']} live, {counts['frozen']} frozen; bindings ok"
            if args.history:
                line += f"; policy ok over {check_history(args.root)} profile change(s)"
            print(line)
            return 0
        if args.command == "summarize":
            print(json.dumps(summarize(args.report, args.null), indent=2))
            return 0
        if args.command == "compare":
            code, differences = compare(args.root, args.profile, summarize(args.report, args.null))
            for line in differences:
                print(line)
            if code == 1:
                print(f"{args.profile}: results moved; record the new baseline with a reason in "
                      "the same commit (record), or treat the move as a defect")
            elif code == 0:
                print(f"{args.profile}: the run reproduces the baseline")
            return code
        if args.command == "record":
            digest = record_baseline(args.root, args.profile, summarize(args.report, args.null),
                                     args.revision, args.reason, args.source)
            print(f"{args.profile}: baseline {digest}")
            return 0
        if args.command == "amendments":
            document = load_json_bytes(args.file.read_bytes(), str(args.file))
            names = check_amendments(args.root, document, args.base)
            print(f"profiles[] ok: {names}")
            return 0
    except ProfileError as error:
        print(f"refused: {error}", file=sys.stderr)
        return 1
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
