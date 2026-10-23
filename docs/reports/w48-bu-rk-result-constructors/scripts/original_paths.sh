#!/bin/sh
set -eu
root=/home/cosmo/.codex/worktrees/cf0d/quirin-lang
exec bwrap --unshare-user --die-with-parent --new-session --ro-bind / / --bind "$root" "$root" --bind /tmp /tmp --ro-bind "$root/private/w48-rk-constructors/namespace-github" /home/cosmo/Documents/GitHub --ro-bind /media/Data/GitHub/quirin-lang /home/cosmo/Documents/GitHub/qcore-c --chdir "$root" -- "$@"
