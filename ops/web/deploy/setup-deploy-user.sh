#!/bin/bash
# One-time (and re-runnable) setup of image-based deploys on the production host, as root, from a checkout of this
# repository that is up to date (its ops/web holds the compose file this setup starts from):
#
#   sudo ops/web/deploy/setup-deploy-user.sh [--from DIR] [--key 'ssh-ed25519 AAAA... comment']
#
#   --from DIR   the checkout the stack ran from until now (default: the one this script is in). Its ops/web/.env
#                is copied to $COMPOSE_DIR/.env the first time, and its compose kit becomes the "local" release:
#                what a first rollback returns to (the images built on the host, xmage-web-server:local).
#   --key KEY    the public half of the GitHub Actions deploy key (secret DEPLOY_SSH_KEY); written to
#                /home/deploy/.ssh/authorized_keys behind the forced command.
#
# What it sets up (each step leaves what is already there alone, except the two scripts, which it updates):
#   user deploy        no password, a member of the docker group and nothing else. Being in the docker group is as
#                      good as root on this host, so the forced command in authorized_keys is the real limit.
#   /usr/local/bin     playmat-deploy (the forced command) and playmat-compose (compose for people on the host)
#   /etc/playmat-deploy.conf   settings (COMPOSE_DIR, the project name, images, snapshots), root:deploy 640
#   /srv/playmat       the stack's home, deploy:deploy 750: .env (from --from), releases/local, current -> it
#   /var/lib/playmat-deploy    state (current and previous versions), lock, deploy.log
#   /var/backups/playmat       pre-deploy database snapshots
# After it, run the stack with playmat-compose (not from the checkout) and edit settings in /srv/playmat/.env.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
from=$(cd "$here/../../.." && pwd)
key=

while [[ $# -gt 0 ]]; do
  case "$1" in
    --from) from=${2:?--from needs a directory}; shift 2 ;;
    --key) key=${2:?--key needs a public key}; shift 2 ;;
    *) sed -n '2,25p' "$0" >&2; exit 2 ;;
  esac
done

say() { echo "setup: $*"; }
die() { echo "setup: $*" >&2; exit 1; }

[[ $(id -u) == 0 ]] || die "run it as root"
command -v docker > /dev/null || die "docker is not installed"
docker compose version > /dev/null || die "the docker compose plugin is missing"
for tool in flock curl timeout; do
  command -v "$tool" > /dev/null || die "$tool is needed (apt-get install util-linux curl coreutils)"
done
[[ -f $here/playmat-deploy && -f $here/playmat-deploy.conf ]] || die "run it from ops/web/deploy in a checkout"

# --- user -----------------------------------------------------------------------------------------------------------
if ! id deploy > /dev/null 2>&1; then
  useradd --create-home --shell /bin/bash --user-group deploy
  say "created user deploy"
fi
passwd -l deploy > /dev/null
usermod -aG docker deploy

# --- scripts and settings -------------------------------------------------------------------------------------------
install -m 755 -o root -g root "$here/playmat-deploy" /usr/local/bin/playmat-deploy
install -m 755 -o root -g root "$here/playmat-compose" /usr/local/bin/playmat-compose
say "installed /usr/local/bin/playmat-deploy and playmat-compose"

CONF=/etc/playmat-deploy.conf
if [[ ! -f $CONF ]]; then
  # the project the stack already runs as keeps its volumes (accounts, decks, stats)
  project=$(docker ps --filter label=com.docker.compose.service=xmage --format '{{.Label "com.docker.compose.project"}}' | head -n 1)
  project=${project:-web}
  sed "s/^COMPOSE_PROJECT=.*/COMPOSE_PROJECT=$project/" "$here/playmat-deploy.conf" > "$CONF"
  say "wrote $CONF (compose project: $project)"
fi
chown root:deploy "$CONF"
chmod 640 "$CONF"

COMPOSE_DIR=/srv/playmat
STATE_DIR=/var/lib/playmat-deploy
SNAPSHOT_DIR=/var/backups/playmat
# shellcheck source=/dev/null
. "$CONF"

# --- directories ----------------------------------------------------------------------------------------------------
install -d -m 750 -o deploy -g deploy "$COMPOSE_DIR" "$COMPOSE_DIR/releases" "$STATE_DIR"
install -d -m 700 -o deploy -g deploy "$SNAPSHOT_DIR"

if [[ ! -f $COMPOSE_DIR/.env ]]; then
  if [[ -f $from/ops/web/.env ]]; then
    install -m 600 -o deploy -g deploy "$from/ops/web/.env" "$COMPOSE_DIR/.env"
    say "copied $from/ops/web/.env to $COMPOSE_DIR/.env: edit settings there from now on"
  else
    die "no $COMPOSE_DIR/.env and no $from/ops/web/.env to start it from (copy ops/web/.env.example there and edit it)"
  fi
fi

local_kit=$COMPOSE_DIR/releases/local
if [[ ! -f $local_kit/docker-compose.yml ]]; then
  grep -q 'XMAGE_SERVER_IMAGE' "$from/ops/web/docker-compose.yml" \
    || die "$from/ops/web/docker-compose.yml predates image deploys: git pull there first"
  install -d -m 750 -o deploy -g deploy "$local_kit"
  install -m 644 -o deploy -g deploy "$from/ops/web/docker-compose.yml" "$from/ops/web/imgcache.conf.template" "$local_kit/"
  say "the checkout's compose kit is the local release ($local_kit)"
fi
[[ -e $COMPOSE_DIR/current ]] || ln -sfn "$local_kit" "$COMPOSE_DIR/current"
chown -h deploy:deploy "$COMPOSE_DIR/current"

if [[ ! -f $STATE_DIR/state ]]; then
  cat > "$STATE_DIR/state" <<EOF
# written by setup-deploy-user.sh $(date -u +%FT%TZ): the images built on this host
CURRENT_TAG=local
CURRENT_SERVER=xmage-web-server:local
CURRENT_WEB=xmage-web:local
CURRENT_KIT=$local_kit
PREVIOUS_TAG=
PREVIOUS_SERVER=
PREVIOUS_WEB=
PREVIOUS_KIT=
EOF
  chown deploy:deploy "$STATE_DIR/state"
  say "state: running the local images"
fi

# --- ssh key ----------------------------------------------------------------------------------------------------------
if [[ -n $key ]]; then
  [[ $key =~ ^(ssh-ed25519|ecdsa-sha2-nistp256|ssh-rsa)\ [A-Za-z0-9+/=]+(\ [A-Za-z0-9@._-]+)?$ ]] \
    || die "--key doesn't look like one public key line"
  # root-owned, so the deploy user can't loosen its own restrictions
  install -d -m 755 -o root -g root /home/deploy/.ssh
  echo "restrict,command=\"/usr/local/bin/playmat-deploy\" $key" > /home/deploy/.ssh/authorized_keys
  chown root:root /home/deploy/.ssh/authorized_keys
  chmod 644 /home/deploy/.ssh/authorized_keys
  say "wrote /home/deploy/.ssh/authorized_keys"
fi

say "done. Check it: sudo -u deploy playmat-deploy status"
if [[ -f /etc/ssh/ssh_host_ed25519_key.pub ]]; then
  say "for the DEPLOY_KNOWN_HOSTS secret (put the name or address DEPLOY_HOST uses in front):"
  echo "  <DEPLOY_HOST> $(cut -d' ' -f1,2 /etc/ssh/ssh_host_ed25519_key.pub)"
fi
