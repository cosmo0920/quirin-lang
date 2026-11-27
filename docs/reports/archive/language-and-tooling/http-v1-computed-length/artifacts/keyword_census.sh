#!/bin/sh
# Constraint-14 keyword census for card HTTP-V1-COMPUTED-LENGTH.
#
# This card adds ZERO reserved words. The census is run anyway, because the
# constraint asks for a measurement and not for an argument, and because the
# card *reads* four words at new positions: `concat` and `len` (contextual
# clause words of `source.string.v1`) and `dec`/`hex` (reserved since
# STD-MODULES).
#
# Run from the repository root:  sh docs/reports/http-v1-computed-length/artifacts/keyword_census.sh
set -eu

echo "=== corpus size ==="
printf '.qcore/.qc files: '
find . -name '*.qcore' -o -name '*.qc' | grep -v '/target/' | grep -v '/.git/' | wc -l
printf 'tracked .rs files: '
git ls-files '*.rs' | wc -l

echo
echo "=== the global reserved-word list is unchanged ==="
git diff --stat origin/main -- crates/qcore-frontend/src/frontend.rs
printf 'reserved words in is_reserved_source_word, before: '
git show origin/main:crates/qcore-frontend/src/frontend.rs \
  | sed -n '/^fn is_reserved_source_word/,/^}/p' | grep -c '^ *| *"'
printf 'reserved words in is_reserved_source_word, after:  '
sed -n '/^fn is_reserved_source_word/,/^}/p' crates/qcore-frontend/src/frontend.rs \
  | grep -c '^ *| *"'
printf 'HTTP_KEYWORDS arity: '
grep -o 'HTTP_KEYWORDS: \[&str; [0-9]*\]' crates/qcore-frontend/src/source_http.rs

echo
echo "=== the words this card reads at new positions ==="
for word in concat len dec hex; do
  printf '%-12s bare-word occurrences in .qcore/.qc (comments stripped): ' "$word"
  find . \( -name '*.qcore' -o -name '*.qc' \) ! -path './target/*' -print0 \
    | xargs -0 sed 's://.*::' 2>/dev/null \
    | grep -Eo "\\b$word\\b" | wc -l
  printf '%-12s "%s(" shape in .qcore/.qc (comments stripped):          ' "$word" "$word"
  find . \( -name '*.qcore' -o -name '*.qc' \) ! -path './target/*' -print0 \
    | xargs -0 sed 's://.*::' 2>/dev/null \
    | grep -Eo "\\b$word\\(" | wc -l
done

echo
echo "=== the contextual rule is one predicate, consulted once ==="
grep -rn 'is_string_clause_word' crates/qcore-frontend/src/ | sed 's/:.*fn /: fn /'

echo
echo "=== candidate spellings this card considered and did NOT take ==="
for word in length size body value render format; do
  printf '%-10s bare-word occurrences in .qcore/.qc source: ' "$word"
  find . \( -name '*.qcore' -o -name '*.qc' \) ! -path './target/*' -print0 \
    | xargs -0 sed 's://.*::' 2>/dev/null \
    | grep -Eo "\\b$word\\b" | wc -l
done
