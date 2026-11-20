#!/usr/bin/env bash
# rerun-failed.sh NAME -- AG1: the gate-2 rust-suite failures run again alone
# after the deliberate binding-digest update (the reading-site pair) and the
# environment-dependent libcurl header test, with the gate's test-build config.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
cd "$repo" || exit 2
mapfile -t cfg < <(python3 scripts/test_link_config.py)
out="$repo/private/ag1/$1"
mkdir -p "$out"
cargo test "${cfg[@]}" -p qcore-c-gateway --test libc_reading_site_evidence --test reading_site_domain \
  > "$out/reading-site.log" 2>&1
echo $? > "$out/reading-site.exit"
python3 scripts/gate_slot.py rust-suite --receipt "$out/libcurl.receipt.json" -- \
  cargo test "${cfg[@]}" -p qcore-audit --test libcurl_user_crate_versions \
  > "$out/libcurl.log" 2>&1
echo $? > "$out/libcurl.exit"
