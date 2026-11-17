#!/usr/bin/env python3
"""Fetch libcurl's official manual pages and record each page's AVAILABILITY section.

    python3 fetch_docs.py --out DIR --record doc-snapshots.json FUNCTION...

Card LIBCURL-PROVIDER-PLUGIN (user decision 2026-11-17, the template of
OPENSSL-PROVIDER-PLUGIN's decision of 2026-11-16): the `since` of every
libcurl function contract comes from libcurl's official API reference,
https://curl.se/libcurl/c/<function>.html, its AVAILABILITY section ("Added
in curl X.Y.Z"). Each page is fetched with curl into DIR (outside the
repository; the pages are not committed), its SHA-256 and fetch date are
recorded, and beside them the text of its AVAILABILITY, SYNOPSIS and RETURN
VALUE sections, its sentences mentioning NULL and its DESCRIPTION
sentences on allocation, copying and release (tags stripped), so a
reviewer can check every cited bound and custody reading against the
snapshot digest. libcurl publishes one manual (the current release's), so a
page has no series: `series` records the manual's own version as the page
footer states it, when it does, else `current`. A page without an
AVAILABILITY section records `availability: null`; the contract then states
`documented` at the manual's version, never an invented `since`.
"""
import argparse
import datetime
import hashlib
import html
import json
import re
import subprocess
from pathlib import Path

BASE = "https://curl.se/libcurl/c"


def section(page_html, name):
    """One roffit section's text (`<a name="NAME"></a><h2>...`), tags
    stripped, or None."""
    match = re.search(rf'<a name="{name}"></a><h2[^>]*>.*?</h2>(.*?)(?:<a name="|<p class="roffit">)',
                      page_html, re.S)
    if not match:
        return None
    text = re.sub(r"<[^>]+>", " ", match.group(1))
    return re.sub(r"\s+", " ", html.unescape(text)).strip()


def added_in(availability):
    """The version an AVAILABILITY text names ("Added in curl 7.62.0"), or
    None."""
    match = re.search(r"Added in curl (\d+\.\d+(?:\.\d+)?)", availability or "")
    return match.group(1) if match else None


def null_sentences(page_html):
    """The DESCRIPTION and RETURN VALUE sentences that mention NULL: the
    documented nullability each contract states."""
    sentences = []
    for name in ("DESCRIPTION", "RETURN"):
        text = section(page_html, name) or ""
        sentences += [s.strip() for s in re.split(r"(?<=[.])\s+", text) if "NULL" in s]
    return sentences


def custody_sentences(page_html):
    """The DESCRIPTION sentences that speak of allocation, copying or
    release: the documented custody each contract states."""
    text = section(page_html, "DESCRIPTION") or ""
    return [s.strip() for s in re.split(r"(?<=[.])\s+", text)
            if re.search(r"free|cop(?:y|ies)|allocat|destroy|cleanup", s, re.I)]


def fetch(function, out):
    url = f"{BASE}/{function}.html"
    target = out / f"{function}.html"
    target.parent.mkdir(parents=True, exist_ok=True)
    completed = subprocess.run(["curl", "-sS", "-f", "--max-time", "60", "-o", str(target), url],
                               capture_output=True, text=True)
    if completed.returncode != 0:
        return {"url": url, "fetched": False, "error": completed.stderr.strip()}
    data = target.read_bytes()
    text = data.decode("utf-8", "replace")
    availability = section(text, "AVAILABILITY")
    return {"url": url, "fetched": True,
            "sha256": hashlib.sha256(data).hexdigest(), "bytes": len(data),
            "availability": availability, "added_in": added_in(availability),
            "synopsis": section(text, "SYNOPSIS"), "return_value": section(text, "RETURN"),
            "null_sentences": null_sentences(text),
            "custody_sentences": custody_sentences(text)}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--record", required=True, type=Path)
    parser.add_argument("functions", nargs="+")
    args = parser.parse_args()
    date = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%d")
    record = {"schema_version": 1, "source": BASE, "fetched_on": date,
              "rule": "user decision 2026-11-17 (card LIBCURL-PROVIDER-PLUGIN): since bounds "
                      "come from the official manual pages' AVAILABILITY sections; pages are "
                      "not committed, only their SHA-256 as fetched and the sections' text",
              "pages": []}
    for function in args.functions:
        entry = fetch(function, args.out)
        entry["page"] = function
        record["pages"].append(entry)
    args.record.write_text(json.dumps(record, indent=1) + "\n")


if __name__ == "__main__":
    main()
