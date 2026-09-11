#!/bin/sh
# Constraint-14 keyword collision census for card HTTP-QUIRIN-V1.
# Run from the worktree root:  sh measure/census.sh
set -e
root="$(pwd)"
find "$root" -path "$root/target" -prune -o \( -name '*.qcore' -o -name '*.qc' \) -print > measure/qcore_files.txt
find "$root" -path "$root/target" -prune -o -name '*.rs' -print > measure/rust_files.txt
echo "qcore/qc fixtures: $(wc -l < measure/qcore_files.txt)"
echo "rust sources:      $(wc -l < measure/rust_files.txt)"
# Strip // comments from the QCore side, as the socket card's census did.
xargs sed 's://.*::' < measure/qcore_files.txt > measure/qcore_nocomment.txt
xargs cat < measure/qcore_files.txt > measure/qcore_raw.txt
xargs grep -h -o -E '"[^"]*"' < measure/rust_files.txt > measure/rust_strings.txt || true
echo "rust string literals: $(wc -l < measure/rust_strings.txt)"
echo
printf '%-12s %8s %8s %8s\n' word qcore-raw qcore-src rust-str
for w in request response header body http message method status version reason field via head content payload entity trailer target
do
  a=$(grep -o -E "(^|[^A-Za-z0-9_.])$w([^A-Za-z0-9_.]|$)" measure/qcore_raw.txt | wc -l)
  b=$(grep -o -E "(^|[^A-Za-z0-9_.])$w([^A-Za-z0-9_.]|$)" measure/qcore_nocomment.txt | wc -l)
  c=$(grep -o -E "(^|[^A-Za-z0-9_.])$w([^A-Za-z0-9_.]|$)" measure/rust_strings.txt | wc -l)
  printf '%-12s %8s %8s %8s\n' "$w" "$a" "$b" "$c"
done
