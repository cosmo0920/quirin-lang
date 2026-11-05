#!/usr/bin/env python3
"""LIBRDKAFKA-HEADER-INLINES census.

Recomputes, from one `quirin-audit current` report, every callee the report
calls "uncontracted" that a librdkafka unit calls and that has no analysed
body anywhere in the run (no unit lists it among its functions), and records
for each where its definition is in the librdkafka source at each audited
Fluent Bit revision, its storage class, and why the run has no body for it.

    python3 census.py --report REPORT.json --adoption ADOPTION.json \
        --tree REV=PATH [--tree REV=PATH ...] --out DIR [--decisions FILE]

REPORT is the run's `resource-diagnostics.json`. ADOPTION is
CONTRACT-ADOPTION-NOT-IN-FORCE's census.json (its rows are the callees that
some provider module already declares; their owner entries keep them). Each
--tree names a Fluent Bit checkout at one revision (its `lib/librdkafka-*`
directory is read, files only, never executed). The first tree is the run's
revision. Run from the repository root; reads tracked provider files through
`git ls-files`. Writes DIR/census.json and DIR/census.md (the in-scope table,
whose last column comes from the optional --decisions JSON).
"""
import argparse
import collections
import hashlib
import json
import re
import subprocess
from pathlib import Path

UNCONTRACTED = re.compile(r"^call to uncontracted function `([^`]+)`$")
EXTERN = re.compile(r"^\s*extern\s+fn\s+([A-Za-z_][A-Za-z0-9_]*)", re.M)
IDENT = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")

# Library prefixes that are not librdkafka's own code. Each is an external
# library with its own provider/plugin owner (user decision: never libc, never
# this crate), or the compiler. Checked by hand against the run's calls.
EXTERNAL_LIBRARY = [
    (re.compile(r"^(X509|BIO|EVP|SSL|ERR|PEM|OPENSSL|OSSL|RAND|HMAC|PKCS12|d2i|i2d|ASN1|BN|CRYPTO|ENGINE|OBJ|DH|EC|RSA|SHA\d*|MD5)_|^(ssl|crypto)_"), "OpenSSL"),
    (re.compile(r"^(inflate|deflate|crc32$|adler32|zlibVersion|compress|uncompress)"), "zlib"),
    (re.compile(r"^(ZSTD|ZDICT)_"), "zstd"),
    (re.compile(r"^sasl_"), "Cyrus SASL"),
    (re.compile(r"^curl_|^Wcurl_"), "libcurl"),
]


def tracked(prefix):
    out = subprocess.run(["git", "ls-files", "-z", prefix], capture_output=True, check=True).stdout
    return [p for p in out.decode().split("\0") if p]


def strip(text):
    """Blank comments and string/char literals, keeping offsets and newlines."""
    out = list(text)
    i, n = 0, len(text)
    while i < n:
        c = text[i]
        if text.startswith("//", i):
            j = text.find("\n", i)
            j = n if j < 0 else j
            for k in range(i, j):
                out[k] = " "
            i = j
        elif text.startswith("/*", i):
            j = text.find("*/", i + 2)
            j = n if j < 0 else j + 2
            for k in range(i, j):
                if out[k] != "\n":
                    out[k] = " "
            i = j
        elif c in "\"'":
            j = i + 1
            while j < n and text[j] != c:
                j += 2 if text[j] == "\\" else 1
            for k in range(i + 1, min(j, n)):
                if out[k] != "\n":
                    out[k] = " "
            i = j + 1
        else:
            i += 1
    return "".join(out)


def blank_preprocessor(text):
    """Blank preprocessor lines (with continuations) and every `#ifdef
    __cplusplus` arm (its `extern "C" {` braces); returns text and macro defs."""
    lines = text.split("\n")
    macros = []
    out = []
    # stack of booleans: True while inside a __cplusplus arm being skipped
    stack = []
    i = 0
    while i < len(lines):
        line = lines[i]
        skipping = any(stack)
        if line.lstrip().startswith("#"):
            start = i
            block = [line]
            while block[-1].endswith("\\") and i + 1 < len(lines):
                i += 1
                block.append(lines[i])
            directive = re.match(r"\s*#\s*(\w+)(.*)", block[0])
            word = directive.group(1) if directive else ""
            rest = directive.group(2) if directive else ""
            if word in ("if", "ifdef", "ifndef"):
                stack.append(word == "ifdef" and "__cplusplus" in rest
                             or word == "if" and re.search(r"defined\s*\(?\s*__cplusplus", rest) is not None)
            elif word in ("else", "elif") and stack:
                stack[-1] = False
            elif word == "endif" and stack:
                stack.pop()
            m = re.match(r"\s*#\s*define\s+([A-Za-z_][A-Za-z0-9_]*)(\()?", block[0])
            if m and not skipping:
                macros.append((m.group(1), start + 1, bool(m.group(2)), "\n".join(block)))
            out.extend(" " * len(b) for b in block)
        elif skipping:
            out.append(" " * len(line))
        else:
            out.append(line)
        i += 1
    return "\n".join(out), macros


def matching(text, i, open_c, close_c):
    depth = 0
    for j in range(i, len(text)):
        if text[j] == open_c:
            depth += 1
        elif text[j] == close_c:
            depth -= 1
            if depth == 0:
                return j
    return -1


class SourceFile:
    def __init__(self, path, rel):
        raw = path.read_text(errors="replace")
        self.rel = rel
        self.raw = raw
        code, self.macros = blank_preprocessor(strip(raw))
        self.code = code
        self.line_starts = [0]
        for m in re.finditer("\n", raw):
            self.line_starts.append(m.end())
        # depth[i] = brace depth before offset i, sampled at identifiers on demand
        self._depth = None

    def line(self, offset):
        lo, hi = 0, len(self.line_starts) - 1
        while lo < hi:
            mid = (lo + hi + 1) // 2
            if self.line_starts[mid] <= offset:
                lo = mid
            else:
                hi = mid - 1
        return lo + 1

    def depth_map(self):
        if self._depth is None:
            depth = 0
            marks = []
            for i, c in enumerate(self.code):
                if c == "{":
                    depth += 1
                elif c == "}":
                    depth -= 1
                marks.append(depth)
            self._depth = marks
        return self._depth

    def definitions(self, name):
        """File-scope function definitions and prototypes of `name`."""
        found = []
        for m in re.finditer(r"\b" + re.escape(name) + r"\s*\(", self.code):
            start = m.start()
            if self.depth_map()[start] != 0:
                continue
            close = matching(self.code, m.end() - 1, "(", ")")
            if close < 0:
                continue
            k = close + 1
            tail = self.code[k:k + 400]
            stripped = re.sub(r"__attribute__\s*\(\(.*?\)\)|RD_[A-Z_]+(\([^)]*\))?|\s+", "", tail, flags=re.S)
            # prefix: back to the previous `;`, `}` or `{` at file scope
            p = start - 1
            while p >= 0 and self.code[p] not in ";{}":
                p -= 1
            prefix = self.code[p + 1:start]
            tokens = IDENT.findall(prefix)
            if not tokens or tokens[-1] in ("return", "sizeof", "if", "while", "for", "switch"):
                continue
            if "=" in prefix or "," in prefix.split("\n")[-1] and "(" not in prefix:
                continue
            kind = "definition" if stripped.startswith("{") else "declaration" if stripped.startswith(";") else None
            if kind is None:
                continue
            spec = set(tokens)
            found.append({
                "file": self.rel,
                "line": self.line(start),
                "kind": kind,
                "static": "static" in spec,
                "inline": bool(spec & {"inline", "__inline", "__inline__", "RD_INLINE"}),
                "extern": "extern" in spec,
                "start": start,
                "body_open": None,
            })
        return found


def body_text(source, definition):
    start = definition["start"]
    close = matching(source.code, source.code.find("(", start), "(", ")")
    brace = source.code.find("{", close)
    end = matching(source.code, brace, "{", "}")
    definition["end_line"] = source.line(end)
    return source.code[brace:end + 1], source.raw[start:end + 1]


def load_tree(root):
    libs = sorted(Path(root, "lib").glob("librdkafka-*"))
    assert len(libs) == 1, libs
    base = libs[0]
    files = {}
    for path in sorted((base / "src").rglob("*")):
        if path.suffix in (".c", ".h") and path.is_file():
            rel = str(path.relative_to(root))
            files[rel] = SourceFile(path, rel)
    return files


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--report", required=True)
    parser.add_argument("--adoption", required=True)
    parser.add_argument("--tree", action="append", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--decisions", help="optional JSON {callee: outcome text} for the table's last column")
    args = parser.parse_args()
    report_bytes = Path(args.report).read_bytes()
    report = json.loads(report_bytes)
    adoption = json.loads(Path(args.adoption).read_text())
    adoption_owner = {row["callee"]: row["owner"] for row in adoption["rows"]}
    libc_names = {line.strip() for line in Path("providers/quirin-provider-libc/libc-names.txt").read_text().splitlines()
                  if line.strip() and not line.startswith("#")}

    pairs = collections.defaultdict(set)
    sites = collections.Counter()
    bodied = set()
    unit_files = set()
    for unit in report["units"]:
        unit_files.add(unit["file"])
        bodied.update(unit.get("source_functions") or [])
        bodied.update(f["function"] for f in unit["functions"])
        for function in unit["functions"]:
            for diagnostic in function["diagnostics"]:
                match = UNCONTRACTED.match(diagnostic["description"])
                if match:
                    pairs[match.group(1)].add((unit["library"], unit["file"], function["function"]))
                    sites[match.group(1)] += 1

    # "only uncontracted" functions: every diagnostic is an uncontracted call
    only = {}
    for unit in report["units"]:
        for function in unit["functions"]:
            ds = function["diagnostics"]
            names = [UNCONTRACTED.match(d["description"]) for d in ds]
            if ds and all(names):
                only[(unit["file"], function["function"])] = {m.group(1) for m in names}

    declared = collections.defaultdict(list)
    for path in tracked("providers"):
        if path.endswith(".qv") and "/contracts/" in path and "/tests/fixtures/" not in path:
            for match in EXTERN.finditer(Path(path).read_text()):
                declared[match.group(1)].append(path)

    trees = []
    for spec in args.tree:
        rev, root = spec.split("=", 1)
        trees.append((rev, load_tree(root)))

    run_rev, run_files = trees[0]
    candidates = sorted(
        (n for n in pairs if n not in bodied and any(p[0] == "librdkafka" for p in pairs[n])),
        key=lambda n: (-len(pairs[n]), n))

    rows = []
    for name in candidates:
        callers = sorted(pairs[name])
        row = {
            "callee": name,
            "pairs": len(callers),
            "sites": sites[name],
            "caller_libraries": sorted({c[0] for c in callers}),
            "declared_in": sorted(set(declared.get(name, []))),
        }
        # ownership outside this card
        owner = None
        if name in adoption_owner:
            owner = "declared already; owned by " + ", ".join(adoption_owner[name])
        elif name.startswith("__builtin_") or name in libc_names:
            owner = "LIBC-GAP-DECLARATIONS (libc or compiler builtin)"
        else:
            for pattern, library in EXTERNAL_LIBRARY:
                if pattern.search(name):
                    owner = f"external library {library} (own provider/plugin crate; user decision)"
                    break
        per_rev = {}
        for rev, files in trees:
            defs, macros = [], []
            for rel, source in files.items():
                for d in source.definitions(name):
                    d = dict(d)
                    if d["kind"] == "definition":
                        body, raw = body_text(source, d)
                        d["body_sha256"] = hashlib.sha256(raw.encode()).hexdigest()
                        d["direct_calls"] = sorted({m.group(1) for m in re.finditer(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*\(", body)}
                                                   - {"if", "while", "for", "switch", "return", "sizeof", name})
                    d.pop("start")
                    d.pop("body_open")
                    defs.append(d)
                for mname, line, fnlike, text in source.macros:
                    if mname == name:
                        macros.append({"file": rel, "line": line, "function_like": fnlike,
                                       "text_sha256": hashlib.sha256(text.encode()).hexdigest()})
            per_rev[rev] = {"definitions": [d for d in defs if d["kind"] == "definition"],
                            "declarations": [{"file": d["file"], "line": d["line"]} for d in defs if d["kind"] == "declaration"],
                            "macros": macros}
        here = per_rev[run_rev]
        defs = here["definitions"]
        if not defs and not here["macros"]:
            where = "no definition in the librdkafka tree"
        elif not defs:
            where = "macro only"
        else:
            kinds = set()
            for d in defs:
                header = d["file"].endswith(".h")
                if d["static"] and not d["extern"]:
                    kinds.add(("header" if header else "unit") + " static" + (" inline" if d["inline"] else ""))
                elif d["inline"]:
                    kinds.add(("header" if header else "unit") + " external-linkage inline")
                else:
                    kinds.add(("header" if header else "unit") + " external definition")
            where = "; ".join(sorted(kinds))
        defining_units = sorted({d["file"] for d in defs if d["file"].endswith(".c")})
        fnptr = []
        if not defs:
            for caller_file in sorted({c[1] for c in callers}):
                source = run_files.get(caller_file)
                if source and re.search(r"\(\s*\*\s*" + re.escape(name) + r"\s*\)", source.code):
                    fnptr.append(caller_file.split("/")[-1])
        if owner is None:
            if any(k.startswith("header static") for k in where.split("; ")):
                why = ("header static body in every including unit; gateway constraint 4a offers it to the W4 "
                       "derivation; the run published no validated summary for it")
            elif any("external-linkage inline" in k for k in where.split("; ")):
                why = "external-linkage inline definition: refused on linkage (W44-CALLEE-EXTERNAL-INLINE-DEFINITION)"
            elif defining_units and not any(u in unit_files for u in defining_units):
                why = "defining unit(s) " + ", ".join(defining_units) + " not among the run's configurations"
            elif defining_units:
                why = "defined in a configured unit but not analysed there (preprocessed out or not enumerated)"
            elif fnptr:
                why = None
                owner = ("not a function: a call through the function-pointer object `" + name + "` ("
                         + "; ".join(fnptr) + "); indirect-call rules own it")
            elif where == "macro only":
                why = "a macro name; a call to it is not a call after preprocessing unless defined elsewhere"
            else:
                why = "no definition in the librdkafka source"
        else:
            why = None
        identical = len({tuple(d.get("body_sha256") for d in per_rev[r]["definitions"]) for r, _ in trees}) == 1
        # Reading aid for "why no summary": the body's direct calls that the
        # run itself reports uncontracted somewhere (never the derivation's
        # own record, which the run does not publish).
        body_uncontracted = sorted({c for d in defs for c in d.get("direct_calls", []) if c in pairs})
        row.update({
            "body_calls_uncontracted_in_run": body_uncontracted,
            "owner_elsewhere": owner,
            "where_at_run_revision": where,
            "why_no_body": why,
            "bodies_identical_across_revisions": identical,
            "revisions": per_rev,
            "callers": [[c[1], c[2]] for c in callers],
            "only_uncontracted_callers": sorted(f"{f}:{fn}" for (f, fn), names in only.items() if name in names),
        })
        rows.append(row)

    in_scope = [r for r in rows if r["owner_elsewhere"] is None]
    document = {
        "card": "LIBRDKAFKA-HEADER-INLINES",
        "report_sha256": hashlib.sha256(report_bytes).hexdigest(),
        "report_fluent_bit_revision": report["fluent_bit_revision"],
        "revisions_read": [rev for rev, _ in trees],
        "uncontracted_callees": len(pairs),
        "external_callees_called_from_librdkafka": len(rows),
        "external_pairs_called_from_librdkafka": sum(r["pairs"] for r in rows),
        "in_scope_callees": len(in_scope),
        "in_scope_pairs": sum(r["pairs"] for r in in_scope),
        "only_uncontracted_functions": len(only),
        "rows": rows,
    }
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    (out / "census.json").write_text(json.dumps(document, indent=1) + "\n")
    decisions = json.loads(Path(args.decisions).read_text()) if args.decisions else {}
    lines = ["| # | Callee | Pairs | Sites | Definition (046accf6) | Same at all three | Body calls uncontracted in the run | Outcome |",
             "|---|---|---:|---:|---|---|---|---|"]
    for i, row in enumerate(in_scope, 1):
        defs = row["revisions"][run_rev]["definitions"]
        where = ", ".join(f"`{d['file'].split('/')[-1]}:{d['line']}`" for d in defs)
        calls = ", ".join(f"`{c}`" for c in row["body_calls_uncontracted_in_run"]) or "none"
        outcome = decisions.get(row["callee"], "(step 2)")
        lines.append(f"| {i} | `{row['callee']}` | {row['pairs']} | {row['sites']} | {where} ({row['where_at_run_revision']}) "
                     f"| {'yes' if row['bodies_identical_across_revisions'] else 'NO'} | {calls} | {outcome} |")
    lines += ["", "Owned elsewhere (external, no body in the run, called from librdkafka):", "",
              "| Owner | Callees | Pairs | Largest |", "|---|---:|---:|---|"]
    groups = collections.defaultdict(list)
    for row in rows:
        if row["owner_elsewhere"] is not None:
            key = row["owner_elsewhere"]
            if key.startswith("not a function"):
                key = "not a function: a call through a function-pointer object (indirect-call rules)"
            groups[key].append(row)
    for key, members in sorted(groups.items(), key=lambda kv: -sum(r["pairs"] for r in kv[1])):
        top = ", ".join(f"`{r['callee']}` {r['pairs']}" for r in members[:4])
        lines.append(f"| {key} | {len(members)} | {sum(r['pairs'] for r in members)} | {top} |")
    (out / "census.md").write_text("\n".join(lines) + "\n")


if __name__ == "__main__":
    main()
