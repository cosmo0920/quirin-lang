#!/bin/sh
# Preserve the saved compile database's original absolute paths without editing it.
set -eu
root=$(pwd -P)
exec bwrap --unshare-user --die-with-parent --new-session --ro-bind / / --dev /dev \
  --bind "$root" "$root" --bind /tmp /tmp --bind /home/cosmo/.cargo /home/cosmo/.cargo \
  --ro-bind "$root/private/w48-cmt-map/namespace-github" /home/cosmo/Documents/GitHub \
  --ro-bind /media/Data/GitHub/quirin-lang /home/cosmo/Documents/GitHub/qcore-c \
  --chdir "$root" -- "$@"
