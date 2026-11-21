#!/bin/bash
# Installs the Rust AI's self-play worker on a Playmat server (docs/rust/Ai.md, "Self-play on the public server"):
# the playmat-rl account the trainer pulls through, its spool, the worker and its timer. Run as root from this
# directory; safe to run again. Nothing plays until the trainer sends a job (rust/rl/rounds.py --remote).
#
#   ops/web/host/rl-setup.sh 'ssh-ed25519 AAAA... trainer'   the trainer's public key (added once)
#   ops/web/host/rl-setup.sh --remove                       stops and removes it all (the spool too)
set -euo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
SPOOL=/srv/playmat-rl

if [[ ${1:-} == --remove ]]; then
  systemctl disable --now playmat-rl-worker.timer 2> /dev/null || true
  docker rm -f playmat-rl-worker > /dev/null 2>&1 || true
  rm -f /etc/systemd/system/playmat-rl-worker.{service,timer} /usr/local/bin/playmat-rl-{worker,transfer}
  systemctl daemon-reload
  userdel playmat-rl 2> /dev/null || true
  rm -rf "$SPOOL"
  echo "removed (/etc/playmat/rl.env kept)"
  exit 0
fi

id playmat-rl > /dev/null 2>&1 || useradd --system --home-dir "$SPOOL" --shell /bin/bash --user-group playmat-rl
passwd -l playmat-rl > /dev/null
mkdir -p "$SPOOL"/{jobs,job,out,state,work} "$SPOOL/.ssh"
chown -R playmat-rl:playmat-rl "$SPOOL"
chmod 750 "$SPOOL"
chmod 700 "$SPOOL/.ssh"
install -m 755 "$HERE/rl-worker.sh" /usr/local/bin/playmat-rl-worker
install -m 755 "$HERE/rl-transfer.sh" /usr/local/bin/playmat-rl-transfer
mkdir -p /etc/playmat
if [[ ! -e /etc/playmat/rl.env ]]; then
  cat > /etc/playmat/rl.env << 'EOF'
# the self-play worker's settings (ops/web/host/rl-worker.sh has every one and its default)
JOBS=3
CHUNK_GAMES=300
GAMES_MAX=0
CPU_MAX=25
QUIET_UTC=
EOF
fi
if [[ -n ${1:-} ]]; then
  [[ $1 =~ ^ssh-ed25519\ [A-Za-z0-9+/=]+(\ [^\"]*)?$ ]] || { echo "not an ssh-ed25519 public key" >&2; exit 1; }
  line="restrict,command=\"/usr/local/bin/playmat-rl-transfer\" $1"
  touch "$SPOOL/.ssh/authorized_keys"
  grep -qF "$1" "$SPOOL/.ssh/authorized_keys" || echo "$line" >> "$SPOOL/.ssh/authorized_keys"
  chown playmat-rl:playmat-rl "$SPOOL/.ssh/authorized_keys"
  chmod 600 "$SPOOL/.ssh/authorized_keys"
fi
install -m 644 "$HERE"/systemd-rl/playmat-rl-worker.{service,timer} /etc/systemd/system/
systemctl daemon-reload
systemctl enable --now playmat-rl-worker.timer
echo "installed; $(/usr/local/bin/playmat-rl-worker check)"
