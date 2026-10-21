#!/bin/bash
# Host watchdog for a Playmat server: run every 5 minutes by playmat-watchdog.timer.
#
# Checks that every service of the compose project runs and is healthy, the root disk is under DISK_MAX percent, the
# kernel reports at least MEM_MIN percent of memory available, the site answers over HTTPS and its certificate has
# more than CERT_DAYS days left. A new problem is posted to the staff Discord webhook (XMAGE_DISCORD_STAFF_WEBHOOK,
# else XMAGE_DISCORD_WEBHOOK, read from the stack's .env), repeated every REPEAT_HOURS while it lasts, and a line
# follows when it clears. Everything is logged to the journal as well (journalctl -t playmat-watchdog).
#
#   playmat-watchdog          check once
#   playmat-watchdog test     post a test message to the webhook
#
# Settings come from /etc/playmat/host.env when it exists: PLAYMAT_DIR (the checkout holding ops/web, default
# /root/xmage-server), DISK_MAX (85), MEM_MIN (10), CERT_DAYS (14), REPEAT_HOURS (6). On a host set up for image
# deploys (/etc/playmat-deploy.conf, ops/web/deploy) the stack is the one playmat-compose runs, with /srv/playmat/.env.
set -uo pipefail

[[ -r /etc/playmat/host.env ]] && . /etc/playmat/host.env
PLAYMAT_DIR=${PLAYMAT_DIR:-/root/xmage-server}
DISK_MAX=${DISK_MAX:-85}
MEM_MIN=${MEM_MIN:-10}
CERT_DAYS=${CERT_DAYS:-14}
REPEAT_HOURS=${REPEAT_HOURS:-6}
STATE_DIR=${STATE_DIR:-/var/lib/playmat-watchdog}
if [[ -r /etc/playmat-deploy.conf ]] && command -v playmat-compose > /dev/null; then
  # read, not sourced: the deploy settings have a STATE_DIR of their own
  compose_dir=$(sed -n 's/^COMPOSE_DIR=//p' /etc/playmat-deploy.conf | tail -1)
  ENV_FILE=${compose_dir:-/srv/playmat}/.env
  COMPOSE=(playmat-compose)
else
  ENV_FILE=$PLAYMAT_DIR/ops/web/.env
  COMPOSE=(docker compose -f "$PLAYMAT_DIR/ops/web/docker-compose.yml" --env-file "$ENV_FILE")
fi
mkdir -p "$STATE_DIR"

env_value() { sed -n "s/^$1=//p" "$ENV_FILE" 2>/dev/null | tail -1; }
DOMAIN=$(env_value DOMAIN)
WEBHOOK=$(env_value XMAGE_DISCORD_STAFF_WEBHOOK)
[[ -n $WEBHOOK ]] || WEBHOOK=$(env_value XMAGE_DISCORD_WEBHOOK)
HOST_LABEL=${DOMAIN:-$(hostname)}

log() { logger -t playmat-watchdog -- "$*"; echo "$*"; }

post() {
  [[ -n $WEBHOOK ]] || return 0
  local body
  body=$(printf '%s' "$1" | python3 -c 'import json,sys; print(json.dumps({"content": sys.stdin.read()[:1900], "allowed_mentions": {"parse": []}}))')
  curl -fsS -m 10 -H 'Content-Type: application/json' -d "$body" "$WEBHOOK" >/dev/null || log "webhook post failed"
}

if [[ ${1:-} == test ]]; then
  post "🔧 Watchdog test from $HOST_LABEL: alerts reach this channel."
  log "test message sent"
  exit 0
fi

# problem KEY MESSAGE: remembered under KEY; posted when new or every REPEAT_HOURS
declare -A seen=()
problem() {
  local key=$1 msg=$2 file=$STATE_DIR/$1 now last
  seen[$key]=1
  now=$(date +%s)
  last=$(cat "$file" 2>/dev/null || echo 0)
  log "problem: $msg"
  if (( now - last >= REPEAT_HOURS * 3600 )); then
    post "⚠️ $HOST_LABEL: $msg"
    echo "$now" > "$file"
  fi
}

# services: every one the compose file defines must run, and be healthy when it has a health check
mapfile -t services < <("${COMPOSE[@]}" config --services 2>/dev/null)
if (( ${#services[@]} == 0 )); then
  problem compose "can't read the compose project (${COMPOSE[*]})"
fi
for svc in "${services[@]}"; do
  id=$("${COMPOSE[@]}" ps -q "$svc" 2>/dev/null | head -1)
  if [[ -z $id ]]; then
    problem "svc-$svc" "service $svc isn't running"
    continue
  fi
  state=$(docker inspect -f '{{.State.Status}} {{if .State.Health}}{{.State.Health.Status}}{{else}}none{{end}}' "$id" 2>/dev/null)
  case $state in
    "running healthy" | "running none" | "running starting") ;;
    *) problem "svc-$svc" "service $svc is $state" ;;
  esac
done

# disk and memory
disk=$(df --output=pcent / | tail -1 | tr -dc '0-9')
(( disk > DISK_MAX )) && problem disk "the root disk is ${disk}% full (alert above ${DISK_MAX}%)"
mem=$(awk '/MemTotal/ {t=$2} /MemAvailable/ {a=$2} END {printf "%d", a * 100 / t}' /proc/meminfo)
(( mem < MEM_MIN )) && problem memory "only ${mem}% of memory is available (alert below ${MEM_MIN}%)"

# the site over HTTPS, and its certificate
if [[ -n $DOMAIN ]]; then
  code=$(curl -s -o /dev/null -m 15 -w '%{http_code}' "https://$DOMAIN/")
  [[ $code == 200 ]] || problem https "https://$DOMAIN/ answered $code"
  end=$(echo | openssl s_client -connect 127.0.0.1:443 -servername "$DOMAIN" 2>/dev/null | openssl x509 -noout -enddate 2>/dev/null | cut -d= -f2)
  if [[ -n $end ]]; then
    days=$(( ($(date -d "$end" +%s) - $(date +%s)) / 86400 ))
    (( days < CERT_DAYS )) && problem cert "the certificate for $DOMAIN expires in $days days"
  else
    problem cert "can't read the certificate for $DOMAIN"
  fi
fi

# problems that were open and are gone now
for file in "$STATE_DIR"/*; do
  [[ -e $file ]] || continue
  key=$(basename "$file")
  if [[ -z ${seen[$key]:-} ]]; then
    rm -f "$file"
    log "cleared: $key"
    post "✅ $HOST_LABEL: $key is back to normal"
  fi
done
exit 0
