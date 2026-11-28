#!/usr/bin/env bash
# Sends the images built on this machine to a host that runs them without building, then restarts it there.
# For a box that is slow or small for the card build (a free ARM VPS, a 1 GB VPS).
#
#   ops/web/ship-images.sh <ssh-host> [engine]
#
# Build both images first, for the host's architecture, from a clean checkout (the build context is the whole tree):
#   podman build --platform linux/arm64 -f ops/web/Dockerfile.server -t xmage-web-server:local .
#   podman build --platform linux/arm64 -f ops/web/Dockerfile.web -t xmage-web:local .
# engine: docker (default) or podman, whichever built them. The host needs Docker with compose and a checkout of this
# repository with its ops/web/.env in ~/xmage-server (SHIP_DIR). The checkout is updated to its branch's latest commit,
# so the compose file there matches the images. The host's .env must not name published images (XMAGE_SERVER_IMAGE,
# XMAGE_WEB_IMAGE): those win over the local names these images arrive under.
set -euo pipefail

host=${1:?usage: ship-images.sh <ssh-host> [engine]}
engine=${2:-docker}
dir=${SHIP_DIR:-xmage-server}

local_tmp=$(mktemp "${TMPDIR:-/tmp}/playmat-ship.XXXXXX.tar.gz")
remote_tmp=
cleanup() {
  rm -f "$local_tmp"
  if [[ -n $remote_tmp ]]; then
    ssh "$host" "rm -f -- '$remote_tmp'" 2>/dev/null || true
  fi
}
trap cleanup EXIT

for image in xmage-web-server:local xmage-web:local; do
  echo "sending $image ($("$engine" image inspect --format '{{.Architecture}}' "$image"))"
  # save locally first under pipefail so a failed save or compression never streams a truncated archive
  "$engine" save "$image" | gzip -1 > "$local_tmp"
  remote_tmp=$(ssh "$host" "mktemp /tmp/playmat-ship.XXXXXX.tar.gz")
  ssh "$host" "cat > '$remote_tmp'" < "$local_tmp"
  # podman names local images localhost/<name>; the host gets them under the names the compose file uses
  ssh "$host" "bash -euo pipefail -s -- '$image' '$remote_tmp'" <<'REMOTE'
image=$1
archive=$2
trap 'rm -f -- "$archive"' EXIT
out=$(gunzip -c "$archive" | docker load)
printf '%s\n' "$out"
loaded=$(printf '%s\n' "$out" | sed -n 's/^Loaded image: //p')
if [[ -z $loaded ]]; then
  echo "ship-images: docker load reported no image for $image" >&2
  exit 1
fi
while IFS= read -r name; do
  [[ -z $name || $name == "$image" ]] || docker tag "$name" "$image"
done <<< "$loaded"
REMOTE
  remote_tmp=
done

ssh "$host" "cd '$dir' && git pull --ff-only -q \
  && docker compose -f ops/web/docker-compose.yml --env-file ops/web/.env up -d --no-build --remove-orphans \
  && docker image prune -f >/dev/null"
echo "restarted on $host"
