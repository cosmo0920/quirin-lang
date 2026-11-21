#!/bin/sh
# build.sh TAG : release `indirect-census` build of quirin-audit in this
# worktree, copied to private/sb/bin/quirin-audit-TAG (card CG10-SMALL-BUNDLE;
# round 4's scripts/build.sh with this worktree's paths).
W=$(cd "$(dirname "$0")/../../../.." && pwd)
cd "$W" || exit 2
mkdir -p private/sb/bin
sh /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh \
  cargo build --release -p qcore-audit --bin quirin-audit \
  --features qcore-c-gateway/indirect-census > "private/sb/build-$1.log" 2>&1
echo $? > "private/sb/build-$1.exit"
cp target/release/quirin-audit "private/sb/bin/quirin-audit-$1"
sha256sum "private/sb/bin/quirin-audit-$1" | tee "private/sb/bin/quirin-audit-$1.sha256"
