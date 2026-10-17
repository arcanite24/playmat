#!/bin/sh
# Starts the XMage server. Settings come from the environment (see ops/web/README.md and .env.example).
#
# config/config.xml is rendered on every start from config/config.template.xml (the file shipped in the
# release), so removing a variable really restores the default. Unset or empty variables keep the
# defaults below. Variables ending in _FILE are read from a file instead (Docker secrets).
set -eu

APP=${XMAGE_HOME:-/opt/xmage}
TEMPLATE=$APP/config/config.template.xml
CONFIG=$APP/config/config.xml
DB_DIR=$APP/db          # volume: user data only (accounts, stats, table records, feedback)
CARDS_DIR=$APP/cards-db # volume: card database cache, rebuilt when the image changes

die() { echo "xmage-server: $*" >&2; exit 1; }

# VAR_FILE wins over VAR, so secrets can come from mounted files
from_file() {
  eval "file=\${${1}_FILE:-}"
  if [ -n "$file" ]; then
    [ -r "$file" ] || die "${1}_FILE=$file is not readable"
    eval "$1=\$(cat \"\$file\")"
  fi
}
for secret in XMAGE_ADMIN_PASSWORD XMAGE_MAIL_PASSWORD XMAGE_MAILGUN_API_KEY XMAGE_DISCORD_WEBHOOK XMAGE_DISCORD_STAFF_WEBHOOK XMAGE_GITHUB_CLIENT_SECRET XMAGE_GITHUB_TOKEN XMAGE_TURNSTILE_SECRET XMAGE_KOFI_TOKEN XMAGE_GITHUB_SPONSORS_SECRET; do
  from_file "$secret"
done

is_bool() { [ "$1" = true ] || [ "$1" = false ]; }
is_uint() { case "$1" in ''|*[!0-9]*) return 1 ;; *) return 0 ;; esac; }

# set (or add) an attribute on <server>; xmlstarlet escapes the value, so any characters are safe
set_attr() {
  if [ "$(xmlstarlet sel -t -v "count(/config/server/@$1)" "$CONFIG")" = 0 ]; then
    xmlstarlet ed -L -i /config/server -t attr -n "$1" -v "$2" "$CONFIG"
  else
    xmlstarlet ed -L -u "/config/server/@$1" -v "$2" "$CONFIG"
  fi
}

# --- config.xml ---------------------------------------------------------------------------------
cp "$TEMPLATE" "$CONFIG"

# name: variable, attribute, default ("" = keep the template's value), check (uint/bool/any)
render() {
  eval "val=\${$1:-$3}"
  [ -n "$val" ] || return 0
  case "$4" in
    uint) is_uint "$val" || die "$1 must be a whole number (got '$val')" ;;
    bool) is_bool "$val" || die "$1 must be true or false (got '$val')" ;;
  esac
  set_attr "$2" "$val"
}
render XMAGE_SERVER_NAME        serverName              "mage-server" any
render XMAGE_SERVER_ADDRESS     serverAddress           "0.0.0.0"     any
render XMAGE_PORT               port                    "17171"       uint
render XMAGE_SECONDARY_PORT     secondaryBindPort       ""            any
render XMAGE_MAX_GAME_THREADS   maxGameThreads          "4"           uint
render XMAGE_MAX_AI_OPPONENTS   maxAiOpponents          "4"           uint
render XMAGE_MAX_SECONDS_IDLE   maxSecondsIdle          ""            uint
render XMAGE_MAX_POOL_SIZE      maxPoolSize             ""            uint
render XMAGE_AUTH               authenticationActivated "false"       bool
render XMAGE_SAVE_GAMES         saveGameActivated       "false"       bool
render XMAGE_MAILGUN_API_KEY    mailgunApiKey           ""            any
render XMAGE_MAILGUN_DOMAIN     mailgunDomain           ""            any
render XMAGE_MAIL_SMTP_HOST     mailSmtpHost            ""            any
render XMAGE_MAIL_SMTP_PORT     mailSmtpPort            ""            uint
render XMAGE_MAIL_USER          mailUser                ""            any
render XMAGE_MAIL_PASSWORD      mailPassword            ""            any
render XMAGE_MAIL_FROM          mailFromAddress         ""            any
render XMAGE_ALLOWED_ORIGINS    websocketAllowedOrigins ""            any

# the Monte Carlo AI keeps every core busy for its whole think time; a small server can leave it out
MONTE_CARLO=${XMAGE_MONTE_CARLO:-true}
is_bool "$MONTE_CARLO" || die "XMAGE_MONTE_CARLO must be true or false (got '$MONTE_CARLO')"
if [ "$MONTE_CARLO" = false ]; then
  xmlstarlet ed -L -d "/config/playerTypes/playerType[@className='mage.player.ai.ComputerPlayerMCTS']" "$CONFIG"
fi

# SMTP: TLS from the start on 465, STARTTLS on 587 or when XMAGE_MAIL_SMTP_STARTTLS=true (the server reads it from the
# environment, like XMAGE_MAILGUN_BASE_URL for Mailgun's EU region)
[ -z "${XMAGE_MAIL_SMTP_STARTTLS:-}" ] || is_bool "$XMAGE_MAIL_SMTP_STARTTLS" \
  || die "XMAGE_MAIL_SMTP_STARTTLS must be true or false (got '$XMAGE_MAIL_SMTP_STARTTLS')"

if [ -n "${XMAGE_TURNSTILE_SITE_KEY:-}" ] && [ -z "${XMAGE_TURNSTILE_SECRET:-}" ]; then
  echo "xmage-server: warning: XMAGE_TURNSTILE_SITE_KEY without XMAGE_TURNSTILE_SECRET; Turnstile stays off" >&2
fi

if [ "${XMAGE_AUTH:-false}" = true ] && [ -z "${XMAGE_MAILGUN_API_KEY:-}${XMAGE_MAIL_USER:-}" ]; then
  echo "xmage-server: warning: XMAGE_AUTH=true without mail settings; password reset mails cannot be sent" >&2
fi

# --- databases ----------------------------------------------------------------------------------
# XMage opens every database under ./db. Only user data should live on the db volume; the card
# database belongs to the image. cards.h2.* in ./db are symlinks into the cards volume, which gets
# the image's own card database (built with the image, in cards-prebuilt) whenever the image build
# changes, so an old card DB never outlives an upgrade and a new one is never built on the server.
build_id=$(cat "$APP/.build-id")
if [ "$(cat "$CARDS_DIR/.build-id" 2>/dev/null || true)" != "$build_id" ]; then
  rm -f "$CARDS_DIR"/cards.h2.*
  if [ -f "$APP/cards-prebuilt/cards.h2.mv.db" ]; then
    echo "xmage-server: new image build, installing its card database"
    cp "$APP/cards-prebuilt/cards.h2.mv.db" "$CARDS_DIR/cards.h2.mv.db"
  else
    echo "xmage-server: new image build, the card database will be rebuilt (takes a few minutes)"
  fi
  printf '%s\n' "$build_id" > "$CARDS_DIR/.build-id"
fi
for part in mv.db trace.db; do
  link=$DB_DIR/cards.h2.$part
  # a real file here is a card DB from the old single-volume layout (or one H2 rewrote in place): drop it
  if [ -e "$link" ] && [ ! -L "$link" ]; then rm -f "$link"; fi
  ln -sfn "$CARDS_DIR/cards.h2.$part" "$link"
done
rm -f "$DB_DIR/cards.h2.lock.db" "$DB_DIR/cards.h2.mv.db.tempFile"
# H2 resolves a symlink to an existing file to its real path, but keeps the link path for a missing
# one. Right after a wipe, the server's two openings of the card DB then look like two databases on
# one file and the second fails ("file is locked", OverlappingFileLockException). An empty file is
# a valid new store and makes every opening resolve to the same path.
[ -e "$CARDS_DIR/cards.h2.mv.db" ] || : > "$CARDS_DIR/cards.h2.mv.db"

# --- replays (web client) ---------------------------------------------------------------------
# Finished games are recorded to saved/replays (the xmage-saved volume) for the web client's History.
REPLAYS=${XMAGE_REPLAYS:-true}
is_bool "$REPLAYS" || die "XMAGE_REPLAYS must be true or false (got '$REPLAYS')"
REPLAY_DAYS=${XMAGE_REPLAY_DAYS:-30}
is_uint "$REPLAY_DAYS" || die "XMAGE_REPLAY_DAYS must be a whole number (got '$REPLAY_DAYS')"
REPLAY_MAX=${XMAGE_REPLAY_MAX:-300}
is_uint "$REPLAY_MAX" || die "XMAGE_REPLAY_MAX must be a whole number (got '$REPLAY_MAX')"

# --- desktop clients ---------------------------------------------------------------------------
# The desktop protocol (JBoss Remoting on XMAGE_PORT) costs threads and memory; a web-only server can leave it off.
DESKTOP=${XMAGE_DESKTOP:-true}
is_bool "$DESKTOP" || die "XMAGE_DESKTOP must be true or false (got '$DESKTOP')"

# --- access (contributors server) -------------------------------------------------------------
# XMAGE_ACCESS=allowlist starts the server closed: only the owner, staff and the people on the
# allowlist get in. It is read on the very first boot only; after that /admin switches it.
ACCESS=${XMAGE_ACCESS:-open}
case "$ACCESS" in
  open|allowlist) ;;
  *) die "XMAGE_ACCESS must be open or allowlist (got '$ACCESS')" ;;
esac
if [ "$ACCESS" = allowlist ] && [ "${XMAGE_AUTH:-false}" != true ]; then
  die "XMAGE_ACCESS=allowlist needs XMAGE_AUTH=true: without accounts anyone can sign in with any name"
fi
# names and emails, comma separated, or @/path/to/file with one per line (# starts a comment)
ALLOWLIST=${XMAGE_ALLOWLIST:-}
case "$ALLOWLIST" in
  @*) [ -r "${ALLOWLIST#@}" ] || die "XMAGE_ALLOWLIST file ${ALLOWLIST#@} can't be read" ;;
esac

# --- support for the server ---------------------------------------------------------------------
# The payment webhooks' listener (Caddy forwards /hooks and /public to it): on 17174 when Ko-fi or GitHub Sponsors is
# set up, unless XMAGE_HOOKS_PORT says otherwise (0 = off)
HOOKS_PORT=${XMAGE_HOOKS_PORT:-}
[ -z "$HOOKS_PORT" ] || is_uint "$HOOKS_PORT" || die "XMAGE_HOOKS_PORT must be a whole number (got '$HOOKS_PORT')"

# --- JVM sizing ---------------------------------------------------------------------------------
# Sized to the machine the container sees: its memory limit when it has one, else the host's memory;
# its CPU quota when it has one, else the host's cores.
machine_mb() {
  total=$(awk '/^MemTotal:/ {print int($2 / 1024)}' /proc/meminfo)
  limit=""
  if [ -r /sys/fs/cgroup/memory.max ]; then limit=$(cat /sys/fs/cgroup/memory.max)
  elif [ -r /sys/fs/cgroup/memory/memory.limit_in_bytes ]; then limit=$(cat /sys/fs/cgroup/memory/memory.limit_in_bytes)
  fi
  case "$limit" in
    ''|max) echo "$total" ;;
    *) limit=$((limit / 1048576)); if [ "$limit" -lt "$total" ]; then echo "$limit"; else echo "$total"; fi ;;
  esac
}
machine_cpus() {
  cpus=$(nproc)
  quota=""; period=""
  if [ -r /sys/fs/cgroup/cpu.max ]; then read -r quota period < /sys/fs/cgroup/cpu.max
  elif [ -r /sys/fs/cgroup/cpu/cpu.cfs_quota_us ]; then
    quota=$(cat /sys/fs/cgroup/cpu/cpu.cfs_quota_us); period=$(cat /sys/fs/cgroup/cpu/cpu.cfs_period_us)
  fi
  if is_uint "$quota" && is_uint "$period" && [ "$period" -gt 0 ]; then
    limited=$(( (quota + period - 1) / period ))
    [ "$limited" -lt "$cpus" ] && cpus=$limited
  fi
  echo "$cpus"
}
MACHINE_MB=$(machine_mb)
MACHINE_CPUS=$(machine_cpus)

# Heap: XMAGE_MEMORY when set, else XMAGE_HEAP_PERCENT of the machine, between 256 MB and 1 GB.
# Games hold a few MB each; most of the heap is headroom for the garbage the AI's search makes.
if [ -n "${XMAGE_MEMORY:-}" ]; then
  HEAP=$XMAGE_MEMORY
else
  pct=${XMAGE_HEAP_PERCENT:-40}
  is_uint "$pct" || die "XMAGE_HEAP_PERCENT must be a whole number (got '$pct')"
  heap=$((MACHINE_MB * pct / 100))
  [ "$heap" -lt 256 ] && heap=256
  [ "$heap" -gt 1024 ] && heap=1024
  HEAP=${heap}m
fi
# GC: Serial on one or two CPUs (measured about 25% less CPU per game than G1 there), else G1, told to hand
# memory back to the machine while the server idles.
GC=${XMAGE_GC:-auto}
if [ "$GC" = auto ]; then
  if [ "$MACHINE_CPUS" -le 2 ]; then GC=serial; else GC=g1; fi
fi
case "$GC" in
  serial) GC_FLAGS="-XX:+UseSerialGC" ;;
  g1) GC_FLAGS="-XX:+UseG1GC -XX:G1PeriodicGCInterval=60000" ;;
  parallel) GC_FLAGS="-XX:+UseParallelGC" ;;
  *) die "XMAGE_GC must be auto, serial, g1 or parallel (got '$GC')" ;;
esac
# AI searches at once: 5 (the engine's default), fewer on a machine with fewer CPUs
AI_THREADS=${XMAGE_AI_THREADS:-$([ "$MACHINE_CPUS" -lt 5 ] && echo "$MACHINE_CPUS" || echo 5)}
is_uint "$AI_THREADS" && [ "$AI_THREADS" -ge 1 ] || die "XMAGE_AI_THREADS must be a whole number from 1 (got '$AI_THREADS')"
# card database cache (in the heap): an eighth of the heap, 16 to 150 MB; it mostly holds rows read once at startup
heap_mb() { case "$1" in *[gG]) echo $(( ${1%?} * 1024 )) ;; *[mM]) echo "${1%?}" ;; *[kK]) echo $(( ${1%?} / 1024 )) ;; *) echo $(( $1 / 1048576 )) ;; esac; }
if [ -n "${XMAGE_DB_CACHE:-}" ]; then
  DB_CACHE_MB=$XMAGE_DB_CACHE
else
  DB_CACHE_MB=$(( $(heap_mb "$HEAP") / 8 ))
  [ "$DB_CACHE_MB" -lt 16 ] && DB_CACHE_MB=16
  [ "$DB_CACHE_MB" -gt 150 ] && DB_CACHE_MB=150
fi
is_uint "$DB_CACHE_MB" || die "XMAGE_DB_CACHE must be a whole number of MB (got '$DB_CACHE_MB')"
# glibc gives each busy thread its own malloc arena; capped, the JVM's native memory stays put
export MALLOC_ARENA_MAX=${MALLOC_ARENA_MAX:-2}
echo "xmage-server: machine ${MACHINE_MB} MB, ${MACHINE_CPUS} CPU(s): heap $HEAP, $GC GC, card cache ${DB_CACHE_MB} MB, $AI_THREADS AI search thread(s)"

# --- JVM ----------------------------------------------------------------------------------------
# Arguments go through a private @argfile so the admin password does not show up in `ps`.
umask 077
args=$(mktemp /tmp/xmage-jvm.XXXXXX)
{
  echo "-Xmx$HEAP"
  for flag in $GC_FLAGS; do echo "$flag"; done
  echo "-XX:MaxMetaspaceSize=${XMAGE_METASPACE:-512m}"
  # out of heap, the JVM limps on with every game broken and new players turned away; exiting lets Docker's restart
  # policy bring the server back in seconds
  [ "${XMAGE_EXIT_ON_OOM:-true}" = true ] && echo "-XX:+ExitOnOutOfMemoryError"
  # a heap dump (the size of the heap) in the logs volume shows what filled it; the newest one replaces the last
  if [ "${XMAGE_HEAP_DUMP:-false}" = true ]; then
    echo "-XX:+HeapDumpOnOutOfMemoryError"
    echo "-XX:HeapDumpPath=$APP/logs/heap-oom.hprof"
    rm -f "$APP/logs/heap-oom.hprof.old"
    [ -f "$APP/logs/heap-oom.hprof" ] && mv "$APP/logs/heap-oom.hprof" "$APP/logs/heap-oom.hprof.old"
  fi
  # JDK 25+: 8-byte object headers instead of 12 (the class-data archive was recorded with them, so they go together)
  if [ "${XMAGE_JAVA_RUNTIME:-17}" -ge 25 ] && [ "${XMAGE_COMPACT_HEADERS:-true}" = true ]; then
    echo "-XX:+UseCompactObjectHeaders"
  fi
  # the class-data archive recorded when the image was built (see Dockerfile.server); the JVM skips it if it doesn't fit
  if [ "${XMAGE_CDS:-true}" = true ] && [ -f "$APP/xmage.jsa" ]; then
    echo "-XX:SharedArchiveFile=$APP/xmage.jsa"
    echo "-Xlog:cds=off"
    echo "-Xlog:cds+dynamic=off"
  fi
  echo "-Dxmage.testMode=false"
  echo "-Dlog4j.configuration=file:$APP/config/log4j-docker.properties"
  echo "-Dxmage.replays=$REPLAYS"
  echo "-Dxmage.remoting=$DESKTOP"
  echo "-Dxmage.ai.threads=$AI_THREADS"
  [ -z "${XMAGE_AI_NODE_SCALE:-}" ] || echo "-Dxmage.ai.nodeScale=$XMAGE_AI_NODE_SCALE"
  echo "-Dxmage.db.cacheSizeKB=$((DB_CACHE_MB * 1024))"
  echo "-Dxmage.replays.dir=$APP/saved/replays"
  echo "-Dxmage.replays.retentionDays=$REPLAY_DAYS"
  echo "-Dxmage.replays.maxCount=$REPLAY_MAX"
  if [ -n "${XMAGE_ADMIN_PASSWORD:-}" ]; then
    # quoted for the argfile format: backslashes and double quotes escaped
    printf '"-Dxmage.adminPassword=%s"\n' "$(printf '%s' "$XMAGE_ADMIN_PASSWORD" | sed 's/[\\"]/\\&/g')"
  fi
  # Discord channel news (public tables, events, matches); the webhook address is a secret like the password
  if [ -n "${XMAGE_DISCORD_WEBHOOK:-}" ]; then
    printf '"-Dxmage.discordWebhook=%s"\n' "$(printf '%s' "$XMAGE_DISCORD_WEBHOOK" | sed 's/[\\"]/\\&/g')"
  fi
  if [ -n "${XMAGE_DISCORD_STAFF_WEBHOOK:-}" ]; then
    printf '"-Dxmage.discordStaffWebhook=%s"\n' "$(printf '%s' "$XMAGE_DISCORD_STAFF_WEBHOOK" | sed 's/[\\"]/\\&/g')"
  fi
  if [ -n "${XMAGE_PUBLIC_URL:-}" ]; then
    printf '"-Dxmage.publicUrl=%s"\n' "$(printf '%s' "$XMAGE_PUBLIC_URL" | sed 's/[\\"]/\\&/g')"
  fi
  printf '"-Dxmage.access=%s"\n' "$ACCESS"
  if [ -n "${XMAGE_OWNER:-}" ]; then
    printf '"-Dxmage.owner=%s"\n' "$(printf '%s' "$XMAGE_OWNER" | sed 's/[\\"]/\\&/g')"
  fi
  if [ -n "$ALLOWLIST" ]; then
    printf '"-Dxmage.allowlist=%s"\n' "$(printf '%s' "$ALLOWLIST" | sed 's/[\\"]/\\&/g')"
  fi
  # sign in with GitHub, and a repository's people let in to a contributors server; the secret and token stay out of ps
  for pair in XMAGE_GITHUB_CLIENT_ID:clientId XMAGE_GITHUB_CLIENT_SECRET:clientSecret XMAGE_GITHUB_REPO:repo XMAGE_GITHUB_TOKEN:token; do
    eval "val=\${${pair%%:*}:-}"
    if [ -n "$val" ]; then
      printf '"-Dxmage.github.%s=%s"\n' "${pair#*:}" "$(printf '%s' "$val" | sed 's/[\\"]/\\&/g')"
    fi
  done
  # Cloudflare Turnstile on registration and access requests: the site key goes to the web client, the secret stays here
  if [ -n "${XMAGE_TURNSTILE_SITE_KEY:-}" ]; then
    printf '"-Dxmage.turnstile.siteKey=%s"\n' "$(printf '%s' "$XMAGE_TURNSTILE_SITE_KEY" | sed 's/[\\"]/\\&/g')"
  fi
  if [ -n "${XMAGE_TURNSTILE_SECRET:-}" ]; then
    printf '"-Dxmage.turnstile.secret=%s"\n' "$(printf '%s' "$XMAGE_TURNSTILE_SECRET" | sed 's/[\\"]/\\&/g')"
  fi
  # support payments: Ko-fi's verification token and the GitHub Sponsors webhook secret stay out of ps
  if [ -n "${XMAGE_KOFI_TOKEN:-}" ]; then
    printf '"-Dxmage.kofi.token=%s"\n' "$(printf '%s' "$XMAGE_KOFI_TOKEN" | sed 's/[\\"]/\\&/g')"
  fi
  if [ -n "${XMAGE_GITHUB_SPONSORS_SECRET:-}" ]; then
    printf '"-Dxmage.githubSponsors.secret=%s"\n' "$(printf '%s' "$XMAGE_GITHUB_SPONSORS_SECRET" | sed 's/[\\"]/\\&/g')"
  fi
  [ -z "$HOOKS_PORT" ] || echo "-Dxmage.hooksPort=$HOOKS_PORT"
  if [ -n "${XMAGE_TRUSTED_PROXIES:-}" ]; then
    echo "-Dxmage.web.trustedProxies=$XMAGE_TRUSTED_PROXIES"
  fi
} > "$args"
umask 022

[ -n "${XMAGE_ADMIN_PASSWORD:-}" ] || echo "xmage-server: XMAGE_ADMIN_PASSWORD is not set; admin login is disabled"

cd "$APP"
# the JVM reads the argfile at start; remove it soon after so the password isn't left on disk
( sleep 20; rm -f "$args" ) &
# XMAGE_JAVA_OPTS: extra JVM flags, split on spaces
# shellcheck disable=SC2086
exec java @"$args" ${XMAGE_JAVA_OPTS:-} -jar lib/mage-server-*.jar
