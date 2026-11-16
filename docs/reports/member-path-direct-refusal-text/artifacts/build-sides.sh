#!/bin/sh
# Build quirin-audit in the base and tip exports (MEMBER-PATH-DIRECT-REFUSAL-TEXT).
P=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-aa4af16c3e0cb085f/private
build() {
  side=$1 commit=$2
  cd "$P/$side-src" || exit 2
  QCORE_AUDIT_BUILD_COMMIT=$commit cargo build --release --locked -j 8 -p qcore-audit \
    > "$P/$side-build.log" 2>&1
  echo $? > "$P/$side-build.exit"
}
build base 0f6e17118 &
build tip 8e735f30f &
wait
cat "$P/base-build.exit" "$P/tip-build.exit"
