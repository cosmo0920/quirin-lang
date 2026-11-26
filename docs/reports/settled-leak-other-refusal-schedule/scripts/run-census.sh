#!/bin/sh
# run-census.sh NAME TAR -- run.sh with the off-by-default `indirect-census`
# feature built in and QCORE_LEAK_WITNESS naming run-NAME/witness.jsonl: the
# leak-witness census (W44-LEAK-PIN-AUDIT) over the same opt-in current route.
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a6303276c758fa656
N=$1
T=$W/private/slo/side-$N
O=$W/private/slo/run-$N
mkdir -p "$T" "$O"
tar -x -C "$T" -f "$2" || exit 2
cd "$T" || exit 2
sh /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh cargo build --release -p qcore-audit --bin quirin-audit --features qcore-c-gateway/indirect-census > "$O/build.log" 2>&1
echo $? > "$O/build.exit"
sha256sum "$T/target/release/quirin-audit" > "$O/binary.sha256"
QCORE_LEAK_WITNESS="$O/witness.jsonl"
export QCORE_LEAK_WITNESS
sh "$W/docs/reports/settled-leak-other-refusal-schedule/scripts/opt-in-current.sh" "$T" "$O" "$T/target/release/quirin-audit" /home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6
