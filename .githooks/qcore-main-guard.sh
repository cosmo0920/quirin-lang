#!/bin/sh
# QCore-C main-ref guard — shared decision logic.
#
# Sourced by .githooks/reference-transaction and .githooks/pre-push. It is not
# itself a hook: git only runs files whose names match a hook name, so this file
# is inert until sourced.
#
# Card CARD-SCOPE-GUARD (wave 36), discharging CARD-SCOPE-SELF-MERGE. See
# docs/reports/card-scope-guard/README.md for the design, the measurements
# behind it, the boundary and the activation command.
#
# The question this file answers is exactly one: may THIS process advance
# refs/heads/main? The answer has two layers.
#
#   1. Provenance veto. If the process is rooted in a DELEGATED CARD WORKTREE of
#      this repository (`<main>/.claude/worktrees/agent-*`), the answer is no and
#      no authorisation overrides it. This is what makes an inherited or copied
#      authorisation value useless to a card.
#   2. Positive authorisation. Otherwise QCORE_ALLOW_MAIN_UPDATE must equal the
#      repository-local token written by scripts/install_main_guard.py. Absent
#      or wrong, the answer is no. The default is refusal, for everyone.
#
# Three measurements shaped this, all in the card report §3:
#   * git REWRITES PWD in the hook environment, and a POSIX shell re-derives it
#     at startup anyway, so a caller's PWD cannot reach a shell hook — which
#     rules out the obvious detector for `git -C <main checkout>`, the exact
#     incident shape. What does survive it is process ancestry (/proc, Linux).
#   * A deletion of main reaches the hook as 0000...->0000..., so an old==new
#     filter that does not exclude the all-zeroes new value lets a deletion past.
#   * Every worktree the Agent tool creates is named `agent-<hex>`, while a
#     named worktree beside them may be the integrator's own session.
#
# A hook is bypassable by intent (--no-verify, core.hooksPath, writing
# .git/refs by hand). This guard is against a delegated agent's MISTAKE.

QCORE_PROTECTED_REF="refs/heads/main"
QCORE_ZERO_OID="0000000000000000000000000000000000000000"

# Absolute path of the common git directory, or empty. Read-only; performs no
# ref transaction, so it cannot recurse into the reference-transaction hook.
qcore_common_dir() {
    _c=$(git rev-parse --path-format=absolute --git-common-dir 2>/dev/null) || _c=""
    if [ -z "$_c" ]; then
        _c=$(git rev-parse --git-common-dir 2>/dev/null) || return 1
    fi
    [ -n "$_c" ] || return 1
    case "$_c" in
        /*) printf '%s\n' "$_c" ;;
        *) printf '%s\n' "$(pwd -P)/$_c" ;;
    esac
}

# The directory delegated card worktrees live under, e.g.
# /home/u/qcore-c/.claude/worktrees. Empty when it cannot be determined.
qcore_card_worktree_root() {
    _common=$(qcore_common_dir) || return 1
    case "$_common" in
        */.git) printf '%s\n' "${_common%/.git}/.claude/worktrees" ;;
        *) return 1 ;;
    esac
}

# 0 when $1 names a path inside a DELEGATED card worktree of this repository:
# under <main>/.claude/worktrees/ and in a directory whose name starts "agent-".
#
# The `agent-` requirement is deliberate and measured. Every worktree the Agent
# tool creates for a delegated card is `.claude/worktrees/agent-<hex>`; a
# human-named worktree beside them is where an integrator session may itself be
# working, and vetoing that would leave nobody able to advance main at all. A
# named worktree is therefore gated by the token alone (MAIN-GUARD-NAMED-WORKTREE).
qcore_is_card_path() {
    _path="${1:-}"
    [ -n "$_path" ] || return 1
    _root="${2:-}"
    [ -n "$_root" ] || return 1
    case "$_path" in
        "$_root"/agent-*) return 0 ;;
        *) return 1 ;;
    esac
}

# Comma-separated list of delegated-worktree provenance signals, or empty.
# Each signal is named so that a refusal says which one fired.
qcore_provenance_signals() {
    _signals=""
    _root=$(qcore_card_worktree_root) || _root=""

    if [ -n "$_root" ]; then
        # The physical working directory of this git process. Fires when a card
        # runs git inside its own worktree (a direct commit, push or update-ref).
        _cwd=$(pwd -P 2>/dev/null) || _cwd=""
        if qcore_is_card_path "$_cwd" "$_root"; then
            _signals="${_signals:+$_signals,}cwd-in-card-worktree"
        fi

        # There is deliberately NO signal on the caller's exported PWD. It was
        # written, tested, and removed as unreachable: git rewrites PWD for its
        # own hooks, and a POSIX shell re-derives PWD from getcwd() at startup,
        # so a caller's PWD reaches a shell hook under neither mechanism. An arm
        # no input can drive is not a guard (card report §3.2).

        # Process ancestry. This is the arm that catches the wave-33 incident
        # shape — a card running `git -C <main checkout> merge` — because the
        # card's own shell and session processes remain ancestors of that git
        # process whatever directory git itself was pointed at. Linux /proc only.
        _anc=$(qcore_ancestor_card_cwd "$_root")
        if [ -n "$_anc" ]; then
            _signals="${_signals:+$_signals,}ancestor-in-card-worktree"
        fi
    fi

    # Any linked worktree of this repository, wherever it sits on disk.
    _common=$(qcore_common_dir) || _common=""
    if [ -n "$_common" ] && [ -n "${GIT_DIR:-}" ]; then
        case "${GIT_DIR}" in
            "$_common"/worktrees/agent-*)
                _signals="${_signals:+$_signals,}git-dir-is-card-worktree" ;;
        esac
    fi

    # An explicit marker a card brief or wrapper may export.
    if [ -n "${QCORE_CARD_WORKTREE:-}" ]; then
        _signals="${_signals:+$_signals,}qcore-card-worktree-marker"
    fi

    # The harness's delegated-session marker. Calibrated at activation time:
    # install_main_guard.py measures whether the INTEGRATOR's own environment
    # sets it, and records the signal disabled if it does, rather than assuming.
    _veto_child=$(git config --get qcore.mainguard.vetochildsession 2>/dev/null)
    [ -n "$_veto_child" ] || _veto_child=true
    if [ "$_veto_child" = "true" ] && [ -n "${CLAUDE_CODE_CHILD_SESSION:-}" ]; then
        _signals="${_signals:+$_signals,}delegated-session-marker"
    fi

    printf '%s' "$_signals"
}

# Walk up the process tree; print the first ancestor working directory that is
# inside a delegated card worktree. Silent where /proc is unavailable (macOS,
# Windows), which is a stated boundary rather than a fallback.
qcore_ancestor_card_cwd() {
    _root="$1"
    [ -r /proc/self/stat ] || return 0
    _pid=$$
    _depth=0
    while [ "$_depth" -lt 12 ]; do
        [ -r "/proc/$_pid/stat" ] || return 0
        _acwd=$(readlink "/proc/$_pid/cwd" 2>/dev/null) || _acwd=""
        if [ -n "$_acwd" ] && qcore_is_card_path "$_acwd" "$_root"; then
            printf '%s' "$_acwd"
            return 0
        fi
        # `pid (comm) state ppid ...`; comm may contain spaces and parentheses,
        # so parse after the final ')'.
        _stat=$(sed 's/.*) //' "/proc/$_pid/stat" 2>/dev/null) || return 0
        _ppid=$(printf '%s' "$_stat" | cut -d' ' -f2)
        case "$_ppid" in
            ''|*[!0-9]*) return 0 ;;
        esac
        [ "$_ppid" -gt 1 ] || return 0
        _pid="$_ppid"
        _depth=$((_depth + 1))
    done
    return 0
}

# 0 when QCORE_ALLOW_MAIN_UPDATE carries the configured token.
qcore_is_authorised() {
    _expected=$(git config --get qcore.mainguard.token 2>/dev/null)
    [ -n "$_expected" ] || return 1
    [ "${QCORE_ALLOW_MAIN_UPDATE:-}" = "$_expected" ] || return 1
    return 0
}

# qcore_guard_log <verdict> <detail>. Best effort: a logging failure never
# fails a transaction.
qcore_guard_log() {
    _verdict="$1"
    _detail="$2"
    _dir=$(qcore_common_dir) || return 0
    _when=$(date -u '+%Y-%m-%dT%H:%M:%SZ' 2>/dev/null) || _when="unknown-time"
    printf '%s\t%s\t%s\tpwd=%s\tcwd=%s\tuser=%s\n' \
        "$_when" "$_verdict" "$_detail" "${PWD:-}" "$(pwd -P 2>/dev/null)" \
        "${USER:-unknown}" >>"$_dir/qcore-main-guard.log" 2>/dev/null || true
    return 0
}

# qcore_refusal_message <what> <signals>
qcore_refusal_message() {
    _what="$1"
    _sig="$2"
    printf '\n' >&2
    printf 'QCore-C main-ref guard: REFUSED %s\n' "$_what" >&2
    printf '\n' >&2
    if [ -n "$_sig" ]; then
        printf '  ground: delegated card-worktree provenance (%s)\n' "$_sig" >&2
        printf '  A delegated card may not advance refs/heads/main, with or\n' >&2
        printf '  without authorisation. Commit on your own branch and report;\n' >&2
        printf '  the integrator merges. See CLAUDE.md, worktree hygiene.\n' >&2
    else
        printf '  ground: no valid QCORE_ALLOW_MAIN_UPDATE authorisation\n' >&2
        printf '  The integrator authorises one command at a time:\n' >&2
        printf '    QCORE_ALLOW_MAIN_UPDATE="$(git config --get qcore.mainguard.token)" <command>\n' >&2
        printf '  If no token is configured, run: python3 scripts/install_main_guard.py\n' >&2
        printf '  Do not export the token: everything launched from that shell\n' >&2
        printf '  would inherit it.\n' >&2
    fi
    printf '\n' >&2
}
