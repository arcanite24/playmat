#!/bin/bash
# Installs the host tools of a Playmat server: the watchdog (every 5 minutes) and the off-site backup packer (daily).
# Run as root from this directory; it is safe to run again after an update.
#
#   ops/web/host/install.sh [PLAYMAT_DIR]
#
# PLAYMAT_DIR is the checkout holding ops/web (default: two levels up from here). Settings go to /etc/playmat/host.env;
# the pull key for off-site copies is added by hand (see ops/web/README.md, "Off-site backups").
set -euo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
DIR=${1:-$(cd "$HERE/../../.." && pwd)}
command -v restic >/dev/null || apt-get -o DPkg::Lock::Timeout=600 install -y -qq restic >/dev/null
install -m 755 "$HERE/watchdog.sh" /usr/local/bin/playmat-watchdog
install -m 755 "$HERE/offsite.sh" /usr/local/bin/playmat-offsite
mkdir -p /etc/playmat
chmod 755 /etc/playmat
if [[ ! -e /etc/playmat/host.env ]]; then
  printf 'PLAYMAT_DIR=%s\n' "$DIR" > /etc/playmat/host.env
fi
install -m 644 "$HERE"/systemd/playmat-*.service "$HERE"/systemd/playmat-*.timer /etc/systemd/system/
systemctl daemon-reload
systemctl enable --now playmat-watchdog.timer playmat-offsite.timer
echo "installed; PLAYMAT_DIR=$(sed -n 's/^PLAYMAT_DIR=//p' /etc/playmat/host.env)"
