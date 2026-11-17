#!/usr/bin/env python3
"""OPENSSL-PROVIDER-PLUGIN census.

Every OpenSSL function a route's report calls "uncontracted", with call-site
counts, (unit, function) pairs and the rows it blocks, for each route given;
plus a lexical census of the OpenSSL calls in the Fluent Bit and Monkey TLS
sources (which units the routes analyse is read from the reports).

    python3 census.py --route NAME=REPORT.json [--route ...] \
        --tree CHECKOUT --headers INCLUDE_ROOT --library SO [--library SO] \
        --compile-database DB [--compile-database DB] --clang CLANG --out DIR

The pinned identity (user decision 2026-11-16: a VERSION) is read from the
headers the prepared build uses: `#include <openssl/opensslv.h>` preprocessed
by CLANG with every `-I`/`-isystem` directory the compile databases pass (so a
bundled header set would win over the system one, as in the build), its
OPENSSL_VERSION_MAJOR/MINOR/PATCH and the number they compose, the header chain
`-H` lists with each file's SHA-256 as provenance, and every opensslv.h found
under those directories and the checkout (more than one is ambiguous).

A callee counts as OpenSSL only when its name has an OpenSSL prefix AND
either the pinned shared libraries export it (`nm -D --defined-only` over each
--library, e.g. libssl.so.3 and libcrypto.so.3; many OpenSSL 3 functions such as
`X509_free` are declared only through `DECLARE_ASN1_FUNCTIONS`) or the pinned
headers under INCLUDE_ROOT (`openssl/*.h`, plus the multiarch
`x86_64-linux-gnu/openssl/*.h`) declare it lexically or define it as a macro;
which one held is recorded per row. A row (one function of one unit)
is blocked by OpenSSL alone ("sole") when every diagnostic it carries is an
uncontracted call to an OpenSSL function, and jointly ("co") when it carries
such a call beside any other diagnostic. Files are read, never executed.
"""
import argparse
import collections
import hashlib
import json
import re
import subprocess
from pathlib import Path

UNCONTRACTED = re.compile(r"^call to uncontracted function `([^`]+)`$")
PREFIX = re.compile(
    r"^(X509|BIO|EVP|SSL|ERR|PEM|OPENSSL|OSSL|RAND|HMAC|PKCS12|PKCS7|d2i|i2d|ASN1|BN|CRYPTO|"
    r"ENGINE|OBJ|DH|EC|ECDSA|RSA|DSA|SHA\d*|MD5|TLS|DTLS|CONF|UI|GENERAL_NAME|sk)_")
CALL = re.compile(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*\(")

TLS_SOURCES = [
    "src/tls/openssl.c",
    "src/flb_hmac.c",
    "src/flb_hash.c",
    "src/flb_crypto.c",
    "src/flb_utils.c",
    "src/flb_oauth2.c",
    "src/flb_fips.c",
    "plugins/out_azure_kusto/azure_kusto_conf.c",
    "lib/monkey/plugins/tls/tls_openssl.c",
    "lib/monkey/tls/tls_openssl.c",
]


def strip(text):
    """Blank comments and string/char literals, keeping newlines."""
    out = list(text)
    i, n = 0, len(text)
    while i < n:
        if text.startswith("//", i):
            j = text.find("\n", i)
            j = n if j < 0 else j
            out[i:j] = " " * (j - i)
            i = j
        elif text.startswith("/*", i):
            j = text.find("*/", i + 2)
            j = n if j < 0 else j + 2
            for k in range(i, j):
                if out[k] != "\n":
                    out[k] = " "
            i = j
        elif text[i] in "\"'":
            quote, j = text[i], i + 1
            while j < n and text[j] != quote:
                j += 2 if text[j] == "\\" else 1
            for k in range(i + 1, min(j, n)):
                if out[k] != "\n":
                    out[k] = " "
            i = j + 1
        else:
            i += 1
    return "".join(out)


def header_index(root):
    """name -> {"kind": function|macro, "header": relative path} from the
    pinned headers, and the per-header SHA-256."""
    root = Path(root)
    headers = sorted(root.glob("openssl/*.h")) + sorted(root.glob("x86_64-linux-gnu/openssl/*.h"))
    digests, names = {}, {}
    for path in headers:
        relative = path.relative_to(root).as_posix()
        data = path.read_bytes()
        digests[relative] = hashlib.sha256(data).hexdigest()
        text = strip(data.decode("latin-1"))
        for match in re.finditer(r"^\s*#\s*define\s+([A-Za-z_][A-Za-z0-9_]*)", text, re.M):
            names.setdefault(match.group(1), {"kind": "macro", "header": relative})
        flat = re.sub(r"\s+", " ", re.sub(r"^\s*#.*$", "", text, flags=re.M))
        for match in re.finditer(r"([A-Za-z_][A-Za-z0-9_]*)\s*\(", flat):
            name = match.group(1)
            before = flat[max(0, match.start() - 160):match.start()]
            # A declaration: the name follows a type (not an operator or a
            # call's argument list) and the statement ends with `;`.
            if PREFIX.match(name) and re.search(r"[A-Za-z0-9_*]\s*$", before) and \
                    not re.search(r"(return|=|,|\()\s*$", before):
                names.setdefault(name, {"kind": "function", "header": relative})
    return digests, names


def exported(libraries):
    """name -> library file name for every defined dynamic text symbol."""
    names = {}
    for library in libraries:
        listing = subprocess.run(["nm", "-D", "--defined-only", library], check=True,
                                 capture_output=True, text=True).stdout
        for line in listing.splitlines():
            parts = line.split()
            if len(parts) == 3 and parts[1] in ("T", "W", "i"):
                names.setdefault(parts[2].split("@")[0], Path(library).name)
    return names


def include_directories(databases):
    found = collections.Counter()
    for database in databases:
        for entry in json.loads(Path(database).read_text()):
            arguments = entry.get("arguments") or entry.get("command", "").split()
            for index, argument in enumerate(arguments):
                for flag in ("-I", "-isystem"):
                    if argument == flag and index + 1 < len(arguments):
                        found[arguments[index + 1]] += 1
                    elif argument.startswith(flag) and len(argument) > len(flag):
                        found[argument[len(flag):]] += 1
    return found


def version_identity(clang, databases, tree):
    directories = include_directories(databases)
    flags = []
    for directory in sorted(directories):
        flags += ["-I", directory]
    source = "#include <openssl/opensslv.h>\n"
    macros = subprocess.run([clang, "-E", "-dM", "-x", "c", *flags, "-"], input=source,
                            capture_output=True, text=True, check=True).stdout
    chain = subprocess.run([clang, "-E", "-H", "-x", "c", *flags, "-"], input=source,
                           capture_output=True, text=True, check=True).stderr
    values = {}
    for name in ("OPENSSL_VERSION_MAJOR", "OPENSSL_VERSION_MINOR", "OPENSSL_VERSION_PATCH",
                 "OPENSSL_VERSION_STR", "OPENSSL_VERSION_PRE_RELEASE", "OPENSSL_VERSION_NUMBER"):
        match = re.search(rf"^#define {name} (.*)$", macros, re.M)
        values[name] = match.group(1).strip() if match else None
    major, minor, patch = (int(values[k]) for k in
                           ("OPENSSL_VERSION_MAJOR", "OPENSSL_VERSION_MINOR", "OPENSSL_VERSION_PATCH"))
    headers = []
    for line in chain.splitlines():
        match = re.match(r"^(\.+) (/\S+)$", line)
        if match:
            path = Path(match.group(2))
            headers.append({"depth": len(match.group(1)), "path": str(path),
                            "sha256": hashlib.sha256(path.read_bytes()).hexdigest()})
    candidates = {headers[0]["path"]}
    for directory in list(directories) + [str(tree)]:
        root = Path(directory)
        if root.is_dir() and root != Path("/usr/include"):
            candidates.update(str(p) for p in root.rglob("opensslv.h") if p.parent.name == "openssl")
    package = subprocess.run(["dpkg-query", "-W", "-f=${Package} ${Version} ${Architecture}",
                              "libssl-dev"], capture_output=True, text=True)
    return {
        "rule": "user decision 2026-11-16: the OpenSSL identity is the version the build's "
                "headers report; header digests are provenance",
        "include_flags": dict(sorted(directories.items())),
        "macros": values,
        "version": f"{major}.{minor}.{patch}",
        "version_number": f"0x{(major << 28) | (minor << 20) | (patch << 4):08x}",
        "header_chain": headers,
        "opensslv_candidates": sorted(candidates),
        "header_sets": "one" if len(candidates) == 1 else "ambiguous",
        "package": package.stdout.strip() if package.returncode == 0 else None,
    }


def route_census(name, path, openssl_names):
    data = Path(path).read_bytes()
    report = json.loads(data)
    sites = collections.Counter()
    pairs = collections.defaultdict(set)
    rows = []
    bodied = set()
    for unit in report["units"]:
        bodied.update(unit.get("source_functions") or [])
        bodied.update(f["function"] for f in unit["functions"])
    for unit in report["units"]:
        for function in unit["functions"]:
            diagnostics = function["diagnostics"]
            callees = []
            other = 0
            for diagnostic in diagnostics:
                match = UNCONTRACTED.match(diagnostic["description"])
                if match and match.group(1) in openssl_names:
                    callees.append(match.group(1))
                else:
                    other += 1
            if not callees:
                continue
            for callee in callees:
                sites[callee] += 1
                pairs[callee].add((unit["file"], function["function"]))
            rows.append({
                "library": unit["library"],
                "file": unit["file"],
                "function": function["function"],
                "status": function["status"],
                "openssl_callees": sorted(set(callees)),
                "openssl_sites": len(callees),
                "other_diagnostics": other,
                "blocking": "sole" if other == 0 else "co",
            })
    callees = []
    for callee in sorted(sites, key=lambda c: (-len(pairs[c]), c)):
        mine = [r for r in rows if callee in r["openssl_callees"]]
        callees.append({
            "callee": callee,
            "sites": sites[callee],
            "pairs": len(pairs[callee]),
            "rows_sole": sum(r["blocking"] == "sole" for r in mine),
            "rows_sole_single_callee": sum(r["blocking"] == "sole" and len(r["openssl_callees"]) == 1
                                           for r in mine),
            "rows_co": sum(r["blocking"] == "co" for r in mine),
            "has_body_in_run": callee in bodied,
            "libraries": sorted({r["library"] for r in mine}),
        })
    return {
        "route": name,
        "report": str(path),
        "report_sha256": hashlib.sha256(data).hexdigest(),
        "fluent_bit_revision": report.get("fluent_bit_revision"),
        "units": len(report["units"]),
        "openssl_callees": len(callees),
        "openssl_sites": sum(sites.values()),
        "openssl_pairs": sum(len(p) for p in pairs.values()),
        "rows_blocked": len(rows),
        "rows_sole": sum(r["blocking"] == "sole" for r in rows),
        "rows_co": sum(r["blocking"] == "co" for r in rows),
        "units_analysed_tls": sorted({u["file"] for u in report["units"]
                                      if u["file"] in TLS_SOURCES or "tls" in u["file"]}),
        "callees": callees,
        "rows": rows,
    }


def source_census(tree, openssl_names, analysed):
    out = []
    for relative in TLS_SOURCES:
        path = Path(tree) / relative
        if not path.is_file():
            out.append({"file": relative, "present": False})
            continue
        text = strip(path.read_text(errors="replace"))
        text = re.sub(r"^\s*#.*$", "", text, flags=re.M)
        counter = collections.Counter(m.group(1) for m in CALL.finditer(text)
                                      if m.group(1) in openssl_names)
        out.append({
            "file": relative,
            "present": True,
            "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            "analysed_by": sorted(route for route, files in analysed.items() if relative in files),
            "openssl_functions": len(counter),
            "openssl_sites": sum(counter.values()),
            "calls": dict(sorted(counter.items(), key=lambda kv: (-kv[1], kv[0]))),
        })
    return out


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--route", action="append", required=True)
    parser.add_argument("--tree", required=True)
    parser.add_argument("--headers", required=True)
    parser.add_argument("--library", action="append", required=True)
    parser.add_argument("--compile-database", action="append", required=True)
    parser.add_argument("--clang", required=True)
    parser.add_argument("--out", required=True)
    args = parser.parse_args()
    digests, declared = header_index(args.headers)
    for name, library in exported(args.library).items():
        if PREFIX.match(name):
            entry = declared.setdefault(name, {"kind": "exported", "header": None})
            entry["exported_by"] = library
    openssl_names = {n for n in declared if PREFIX.match(n)}
    library_sha256 = {Path(l).name: hashlib.sha256(Path(l).read_bytes()).hexdigest()
                      for l in args.library}
    routes = []
    analysed = {}
    for spec in args.route:
        name, path = spec.split("=", 1)
        census = route_census(name, path, openssl_names)
        routes.append(census)
        report = json.loads(Path(path).read_text())
        analysed[name] = {u["file"] for u in report["units"]}
    for census in routes:
        for row in census["callees"]:
            row["declared_as"] = declared[row["callee"]]
    sources = source_census(args.tree, openssl_names, analysed)
    for census in routes:
        census.pop("units_analysed_tls")
    result = {
        "card": "OPENSSL-PROVIDER-PLUGIN",
        "openssl_identification": "OpenSSL prefix and declared (function) or defined (macro) "
                                  "in the pinned headers",
        "pinned_identity": version_identity(args.clang, args.compile_database, args.tree),
        "headers_root": args.headers,
        "header_sha256": digests,
        "library_sha256": library_sha256,
        "routes": routes,
        "tls_sources": sources,
    }
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    (out / "census.json").write_text(json.dumps(result, indent=1, sort_keys=False) + "\n")
    lines = ["| Route | Units | OpenSSL callees | Sites | Pairs | Rows blocked | Sole | Co |",
             "|---|---:|---:|---:|---:|---:|---:|---:|"]
    for c in routes:
        lines.append(f"| {c['route']} | {c['units']} | {c['openssl_callees']} | {c['openssl_sites']} | "
                     f"{c['openssl_pairs']} | {c['rows_blocked']} | {c['rows_sole']} | {c['rows_co']} |")
    lines += ["", "| Source | Analysed by | OpenSSL functions | Sites |", "|---|---|---:|---:|"]
    for s in sources:
        if s["present"]:
            lines.append(f"| `{s['file']}` | {', '.join(s['analysed_by']) or 'none'} | "
                         f"{s['openssl_functions']} | {s['openssl_sites']} |")
    for c in routes:
        if not c["callees"]:
            continue
        lines += ["", f"### {c['route']}", "",
                  "| Callee | Sites | Pairs | Rows sole (single callee) | Rows co | Libraries |",
                  "|---|---:|---:|---:|---:|---|"]
        for row in c["callees"]:
            lines.append(f"| `{row['callee']}` | {row['sites']} | {row['pairs']} | {row['rows_sole']} "
                         f"({row['rows_sole_single_callee']}) | {row['rows_co']} | "
                         f"{', '.join(row['libraries'])} |")
    (out / "census.md").write_text("\n".join(lines) + "\n")


if __name__ == "__main__":
    main()
