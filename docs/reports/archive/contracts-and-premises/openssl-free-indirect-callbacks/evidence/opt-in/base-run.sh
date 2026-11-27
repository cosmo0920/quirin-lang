#!/bin/sh
# Side tree of the base revision f916d3007, its release quirin-audit, the
# Fluent Bit 046accf6 inputs (prepared by the base binary), then the base
# opt-in run.
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-aaf5d432a4505fc36
ENV=/home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh
L=$W/private/logs
SCRIPT=$W/docs/reports/openssl-ssl-ctx-callback-slots/evidence/opt-in/opt-in-current.sh
mkdir -p "$W/private/side-base" "$L"
cd "$W" || exit 2
git archive f916d3007 | tar -x -C "$W/private/side-base"
echo "archive $?" > "$L/build-base.exit"
cd "$W/private/side-base" || exit 2
sh "$ENV" cargo build --release -p qcore-audit --bin quirin-audit > "$L/build-base.log" 2>&1
echo "build $?" >> "$L/build-base.exit"
BIN=$W/private/side-base/target/release/quirin-audit
cd "$W" || exit 2
sh "$ENV" "$BIN" prepare --fluent-bit /home/cosmo/Documents/GitHub/fluent-bit --revision 046accf6 \
  --output "$W/private/fb-046accf6" > "$L/prepare.log" 2>&1
echo $? > "$L/prepare.exit"
sh "$SCRIPT" "$W/private/side-base" "$W/private/opt-in/base-1" "$BIN" "$W/private/fb-046accf6"
echo "prepare $(cat "$L/prepare.exit") base $(cat "$W/private/opt-in/base-1/exit")" > "$L/base-run.done"
