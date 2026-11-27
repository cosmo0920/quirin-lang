#!/usr/bin/env bash
# measure-binary.sh NAME BINARY [ROUTES] -- UNION-OPAQUE-RECORD-WRITES-BY-TYPE (from HANDOFF-DIRECT-CALL-READING, adapted from HANDOFF-CANDIDATE-OWN-CALLBACKS, from OP-CALLBACK-MEMBER-PATH-RELEASE) (adapted from SUPPLIED-OWNED-RETURN-CODE-HANDOFF) (adapted from RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION) (adapted from OP-APP-RK-CALLBACKS-VIA-POLL-CB) (adapted from OP-CONSTRUCTOR-DECLARATIONS, from CG22-OP-CALLBACKS-AND-SERVED-QUEUES, from CG22-CALLBACK-REGISTRY-CONTRACT, adapted from REFCOUNT-FAMILY-BLOCKING-CALLEES, adapted from REFCOUNT-REFERENCE-RETURNING-DECLARATIONS, adapted from REFCOUNT-RESOURCE-KIND, adapted from SLOT-MOVE-AND-INTERIOR-BORROW, from W43-ABI-ENUM-AND-CONST-PP, from CALLEE-RETENTION-CLASS1) (adapted from CHUNKIO-META-LEN-BOUNDS-CONTRACTS):
# measure.sh with a saved binary (private/uorw/bin/...). run_pinned_audit.py
# always runs target/release/quirin-audit, so BINARY's bytes are written there
# first under the shared heavy lock and its digest recorded beside the run;
# the system and slice routes run BINARY itself. run.json's binary_sha256 is
# the check that the corpus ran these bytes.
set -u
name="$1"; binary="$(cd "$(dirname "$2")" && pwd)/$(basename "$2")"; routes="${3:-corpus,system,slice}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="${REPO:-$(cd "$here/../../../.." && pwd)}"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
case ",$routes," in *,corpus,*)
  mkdir -p "$repo/target/release" || exit 2
  flock "$lock" /usr/bin/env bash -c 'cat "$1" > "$2" && chmod +x "$2" && sha256sum "$2" > "$3"' _ \
    "$binary" "$repo/target/release/quirin-audit" "$repo/private/uorw/$name.corpus-binary.sha256" || exit 3;;
esac
BINARY="$binary" "$here/measure.sh" "$name" "$routes"
