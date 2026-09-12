#!/bin/sh
# Emit every published program of the routes this card touches with a BASELINE
# compiler (this card's two source files reverted to origin/main) and with the
# card's compiler, then diff. "Every existing program byte-identical" is then a
# measurement rather than an argument.
#
# Worktree-private; writes under measure/ only. Run from the repository root.
set -u
cd "$(dirname "$0")/.."

PROGRAMS="examples/http/exchange.qcore examples/http/raw_request.qcore
examples/http/computed_length.qcore
examples/sockets/tcp_echo.qcore examples/sockets/udp_echo.qcore
examples/strings/status_line.qcore examples/strings/computed_length.qcore
examples/events/self_pipe.qcore examples/events/socket_server.qcore
examples/events/wake_payload.qcore examples/events/write_interest.qcore"

mkdir -p measure/emit-baseline measure/emit-card

emit_all() {
  out="$1"
  for program in $PROGRAMS; do
    [ -f "$program" ] || continue
    name=$(echo "$program" | tr '/' '_')
    cargo run -q -p qcorec --bin quirin -- emit -o "$out/$name.c" "$program" || echo "  (not emittable: $program)"
  done
}

echo "--- card emission ---"
emit_all measure/emit-card

echo "--- reverting this card's two source files to origin/main ---"
cp crates/qcore-core/src/std_http.rs measure/std_http.rs.card
cp crates/qcore-frontend/src/source_http.rs measure/source_http.rs.card
git show origin/main:crates/qcore-core/src/std_http.rs > crates/qcore-core/src/std_http.rs
git show origin/main:crates/qcore-frontend/src/source_http.rs > crates/qcore-frontend/src/source_http.rs

echo "--- baseline emission ---"
emit_all measure/emit-baseline

echo "--- restoring ---"
cp measure/std_http.rs.card crates/qcore-core/src/std_http.rs
cp measure/source_http.rs.card crates/qcore-frontend/src/source_http.rs

echo "--- diff ---"
for program in $PROGRAMS; do
  [ -f "$program" ] || continue
  name=$(echo "$program" | tr '/' '_')
  if [ ! -f "measure/emit-baseline/$name.c" ]; then
    echo "$name: NOT EMITTABLE BY THE BASELINE (new with this card)"
  elif diff -q "measure/emit-baseline/$name.c" "measure/emit-card/$name.c" > /dev/null; then
    echo "$name: byte-identical"
  else
    echo "$name: DIFFERS"
    diff "measure/emit-baseline/$name.c" "measure/emit-card/$name.c" | head -20
  fi
done
