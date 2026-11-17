#!/usr/bin/env python3
"""Fetch OpenSSL's official manual pages and record each page's HISTORY section.

    python3 fetch_docs.py --out DIR --record doc-snapshots.json PAGE[@SERIES...] ...

User decision 2026-11-16: the `since` (and any removal or deprecation bound)
of every OpenSSL function contract comes from OpenSSL's official API
reference, https://docs.openssl.org/<series>/man3/<page>/, its HISTORY
section. Each page is fetched with curl into DIR (outside the repository; the
pages are not committed), its SHA-256 and fetch date are recorded, and the
text of its HISTORY section (tags stripped) is written beside them in the
record so a reviewer can check every cited bound against the snapshot digest.
A page with no HISTORY section records `history: null`; the contract then
states `documented` at the oldest series checked, never an invented `since`.

Card REFERENCE-DROP-NULL-ACCEPTING: `--append` adds the fetched pages to an
existing record instead of rewriting it, each new entry carrying its own
`fetched_on` (the record's top-level date stays the first fetch's), so a
digest recorded earlier is never silently replaced by a later fetch; asking
for a page/series the record already holds is an error.

Card OPENSSL-PROVIDER-REMAINING-CALLEES: `--merge` likewise keeps the record
and stamps each new entry's `fetched_on`, but silently skips every
page/series already held (never refetching it), so a whole page list can be
re-run to fill only the gaps. The two flags are exclusive; both leave every
recorded digest as reviewed.
"""
import argparse
import datetime
import hashlib
import html
import json
import re
import subprocess
from pathlib import Path

BASE = "https://docs.openssl.org"


def section(page_html, name):
    """One `<h2 id=NAME>` section's text (tags stripped), or None."""
    match = re.search(rf'<h2 id="?{name}"?[^>]*>.*?</h2>(.*?)(?:<h2 |<footer|</article>)',
                      page_html, re.S | re.I)
    if not match:
        return None
    text = re.sub(r"<[^>]+>", " ", match.group(1))
    return re.sub(r"\s+", " ", html.unescape(text)).strip()


def history(page_html):
    """The HISTORY section's text, or None."""
    return section(page_html, "history")


def null_sentences(page_html):
    """The DESCRIPTION and RETURN VALUES sentences that mention NULL: the
    documented nullability of each argument a contract states."""
    sentences = []
    for name in ("description", "return-values"):
        text = section(page_html, name) or ""
        sentences += [s.strip() for s in re.split(r"(?<=[.])\s+", text) if "NULL" in s]
    return sentences


def reference_sentences(page_html):
    """The DESCRIPTION sentences that speak of a reference count: the
    documented reading of a release as a counted drop."""
    text = section(page_html, "description") or ""
    return [s.strip() for s in re.split(r"(?<=[.])\s+", text)
            if re.search(r"reference count|structural reference|up_ref", s)]


def fetch(series, page, out):
    url = f"{BASE}/{series}/man3/{page}/"
    target = out / series / f"{page}.html"
    target.parent.mkdir(parents=True, exist_ok=True)
    completed = subprocess.run(["curl", "-sS", "-f", "--max-time", "60", "-o", str(target), url],
                               capture_output=True, text=True)
    if completed.returncode != 0:
        return {"url": url, "series": series, "fetched": False,
                "error": completed.stderr.strip()}
    data = target.read_bytes()
    text = data.decode("utf-8", "replace")
    return {"url": url, "series": series, "fetched": True,
            "sha256": hashlib.sha256(data).hexdigest(), "bytes": len(data),
            "name": section(text, "name"), "history": history(text), "null_sentences": null_sentences(text),
            "reference_sentences": reference_sentences(text)}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--record", required=True, type=Path)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--append", action="store_true",
                      help="add to an existing record; refuse a page/series it already holds "
                           "(card REFERENCE-DROP-NULL-ACCEPTING)")
    mode.add_argument("--merge", action="store_true",
                      help="keep the record's existing pages and add only the (page, series) "
                           "pairs it lacks, each with its own `fetched_on` (card "
                           "OPENSSL-PROVIDER-REMAINING-CALLEES: a page already cited is never "
                           "refetched, so its recorded digest stays the one reviewed)")
    parser.add_argument("pages", nargs="+", help="PAGE@SERIES[,SERIES...]")
    args = parser.parse_args()
    date = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%d")
    if args.append or args.merge:
        record = json.loads(args.record.read_text())
    else:
        record = {"schema_version": 1, "source": BASE, "fetched_on": date,
                  "rule": "user decision 2026-11-16: since/removal bounds come from the official "
                          "manual pages' HISTORY sections; pages are not committed, only their "
                          "SHA-256 as fetched and the HISTORY text",
                  "pages": []}
    present = {(entry["page"], entry["series"]) for entry in record["pages"]}
    for spec in args.pages:
        page, series = spec.split("@", 1)
        for one in series.split(","):
            if (page, one) in present:
                if args.append:
                    raise SystemExit(f"{page}@{one} is already recorded")
                continue
            entry = fetch(one, page, args.out)
            entry["page"] = page
            if args.append or args.merge:
                entry["fetched_on"] = date
            record["pages"].append(entry)
            present.add((page, one))
    args.record.write_text(json.dumps(record, indent=1) + "\n")


if __name__ == "__main__":
    main()
