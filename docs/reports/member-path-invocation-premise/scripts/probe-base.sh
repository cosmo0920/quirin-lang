#!/bin/sh
# probe-base.sh TAR LOG -- extract TAR (a `git archive` of base 98c51878f) into
# private/ctmp/probe-base, copy this card's pin target into it, and run it
# there: the probe, failing at base.
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-ad28b60a7514cefed
T=$W/private/ctmp/probe-base
mkdir -p "$T"
tar -x -C "$T" -f "$1" || exit 2
cp "$W/crates/qcore-c-gateway/tests/member_path_invocation_premise.rs" "$T/crates/qcore-c-gateway/tests/"
cd "$T" || exit 2
sh /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh cargo test -p qcore-c-gateway --test member_path_invocation_premise > "$2" 2>&1
echo "exit $?" >> "$2"
