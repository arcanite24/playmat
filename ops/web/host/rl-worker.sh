#!/bin/bash
# The Rust AI's self-play worker on a Playmat server (docs/rust/Ai.md, "Self-play on the public server"). A systemd
# timer runs it every 2 minutes; each run plays one short chunk of games, only while the server is idle, and stops
# early, throwing the chunk away, the moment it isn't. Training never happens here: the trainer (rust/rl/rounds.py
# --remote) sends a job and pulls the finished chunks over the playmat-rl account (rl-transfer.sh).
#
#   playmat-rl-worker          one gate check and, if idle, one chunk
#   playmat-rl-worker check    the gate's answer only (prints busy: <reason> or idle)
#
# It fails closed: an unreadable /healthz, a missing job or any error means no games. The games run in a container
# with the lowest CPU and I/O weight, at most JOBS cores, a memory cap well below what the server needs, no network
# and a read-only file system, from an image named by digest that the repository's CI built
# (ghcr.io/arcanite24/playmat-selfplay). Settings: /etc/playmat/rl.env (rl-setup.sh writes the defaults).
set -uo pipefail

SPOOL=/srv/playmat-rl
JOBS=3
CHUNK_GAMES=150
SPOOL_MAX_MB=4096
# busy when more games than this are open (most wait on people and cost nothing), or the server uses more than
# CPU_MAX percent of one core; the CPU check is the one that matters
GAMES_MAX=4
CPU_MAX=25
MEM_MIN_MB=2500
DISK_MIN_PCT=25
# the nightly backup (04:00 America/Mexico_City) and the off-site copy (10:15 UTC); quiet hours, if set, as HH-HH UTC
BACKUP_UTC=10:00-10:45
QUIET_UTC=
MAX_RUN_S=900
MEMORY=768m
# shellcheck source=/dev/null
[[ -r /etc/playmat/rl.env ]] && . /etc/playmat/rl.env

NAME=playmat-rl-worker
PAUSED=0
# the admin console's Self-play section (SelfPlayStatus.java): the compose volume xmage-selfplay, shared with the
# server. The worker writes worker.json and history.jsonl there and copies in the trainer's trainer.json; the console
# writes settings.json, which overrides rl.env field by field. No volume (an older compose file): no panel files.
PANEL=
for v in $(docker volume ls -q --filter label=com.docker.compose.volume=xmage-selfplay 2> /dev/null); do
  PANEL=$(docker volume inspect -f '{{.Mountpoint}}' "$v" 2> /dev/null) && [[ -d $PANEL ]] && break
  PANEL=
done
DEFAULT_CPU_MAX=$CPU_MAX DEFAULT_GAMES_MAX=$GAMES_MAX DEFAULT_JOBS=$JOBS DEFAULT_QUIET_UTC=$QUIET_UTC

# settings.json from the console, each field checked here (this runs as root; the server is the only writer, but
# nothing it writes is trusted): a bad or missing field keeps rl.env's value
apply_settings() {
  CPU_MAX=$DEFAULT_CPU_MAX GAMES_MAX=$DEFAULT_GAMES_MAX JOBS=$DEFAULT_JOBS QUIET_UTC=$DEFAULT_QUIET_UTC PAUSED=0
  [[ -n $PANEL && -f $PANEL/settings.json && ! -L $PANEL/settings.json ]] || return 0
  local key value
  while IFS=$'\t' read -r key value; do
    case $key in
      paused) [[ $value == true ]] && PAUSED=1 ;;
      quiet_utc) [[ -z $value || $value =~ ^([01][0-9]|2[0-3])(:[0-5][0-9])?-([01][0-9]|2[0-3])(:[0-5][0-9])?$ ]] && QUIET_UTC=$value ;;
      cpu_max) [[ $value =~ ^[0-9]{1,3}$ ]] && ((10#$value >= 5 && 10#$value <= 200)) && CPU_MAX=$((10#$value)) ;;
      games_max) [[ $value =~ ^[0-9]{1,2}$ ]] && ((10#$value <= 50)) && GAMES_MAX=$((10#$value)) ;;
      jobs) [[ $value =~ ^[1-3]$ ]] && JOBS=$value ;;
    esac
  done < <(head -c 4096 "$PANEL/settings.json" | python3 -c '
import json, sys
try:
    j = json.load(sys.stdin)
except Exception:
    sys.exit(0)
if isinstance(j, dict):
    for k in ("paused", "quiet_utc", "cpu_max", "games_max", "jobs"):
        v = j.get(k)
        if isinstance(v, bool):
            v = "true" if v else "false"
        elif isinstance(v, (int, str)):
            v = str(v)
        else:
            continue
        if "\t" not in v and "\n" not in v:
            print(k + "\t" + v)
' 2> /dev/null)
}
apply_settings

IMAGE_RE='^ghcr\.io/arcanite24/playmat-selfplay(:[A-Za-z0-9._-]+)?@sha256:[0-9a-f]{64}$'
log() { logger -t playmat-rl -- "$*"; echo "$*"; }
status() {
  printf '{"at": "%s", "state": "%s", "detail": "%s"}\n' "$(date -u +%FT%TZ)" "$1" "${2//\"/}" > "$SPOOL/status.json.tmp" \
    && mv -f "$SPOOL/status.json.tmp" "$SPOOL/status.json"
  panel "$1" "${2:-}"
}

# worker.json for the console: the state, the job, the settings in effect and the spool; and the trainer's progress
panel() {
  [[ -n $PANEL ]] || return 0
  local waiting spool_mb
  waiting=$(find "$SPOOL/out" -mindepth 2 -maxdepth 2 -name 'c*.json' 2> /dev/null | wc -l)
  spool_mb=$(du -sm "$SPOOL/out" 2> /dev/null | cut -f1)
  python3 - "$PANEL/worker.json.tmp" "$1" "$2" "${J_ID:-}" "${J_IMAGE:-}" "${J_TAG:-}" "${J_SEED_BASE:-}" \
    "$PAUSED" "$QUIET_UTC" "$CPU_MAX" "$GAMES_MAX" "$JOBS" "$CHUNK_GAMES" "${spool_mb:-0}" "$waiting" \
    "$(cat "$SPOOL/state/totals" 2> /dev/null)" << 'EOF' && chmod 644 "$PANEL/worker.json.tmp" && mv -f "$PANEL/worker.json.tmp" "$PANEL/worker.json"
import json, sys, time
a = sys.argv[1:]
def num(v):
    v = v.strip()
    return int(v) if v.lstrip("-").isdigit() else None
tot = (a[15].split() + ["0", "0", "0"])[:3]
job = {"id": a[3], "image": a[4], "tag": num(a[5]), "seedBase": num(a[6])} if a[3] else None
json.dump({"at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()), "state": a[1], "detail": a[2], "job": job,
           "settings": {"paused": a[7] == "1", "quietUtc": a[8], "cpuMax": num(a[9]), "gamesMax": num(a[10]),
                        "jobs": num(a[11]), "chunkGames": num(a[12])},
           "spoolMb": num(a[13]), "chunksWaiting": num(a[14]),
           "totals": {"games": num(tot[0]), "chunksDone": num(tot[1]), "chunksStopped": num(tot[2])}}, open(a[0], "w"))
EOF
  if [[ -f $SPOOL/trainer.json && ! -L $SPOOL/trainer.json && $SPOOL/trainer.json -nt $PANEL/trainer.json ]]; then
    install -m 644 "$SPOOL/trainer.json" "$PANEL/trainer.json.tmp" && mv -f "$PANEL/trainer.json.tmp" "$PANEL/trainer.json"
  fi
}

# one line a chunk in history.jsonl (the last 5000 kept), and the running totals: record OUTCOME GAMES SECONDS REASON
record() {
  local games=0 kept=0 stopped=0
  read -r games kept stopped 2> /dev/null < "$SPOOL/state/totals" || true
  if [[ $1 == "done" ]]; then
    games=$((${games:-0} + $2)) kept=$((${kept:-0} + 1))
  else
    stopped=$((${stopped:-0} + 1))
  fi
  mkdir -p "$SPOOL/state"
  echo "${games:-0} ${kept:-0} ${stopped:-0}" > "$SPOOL/state/totals"
  [[ -n $PANEL ]] || return 0
  python3 -c 'import json, sys, time; a = sys.argv[1:]; print(json.dumps({"at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
    "job": a[0], "chunk": a[1], "outcome": a[2], "games": int(a[3]), "seconds": int(a[4]), "reason": a[5]}))' \
    "$J_ID" "$chunk" "$1" "$2" "$3" "${4:-}" >> "$PANEL/history.jsonl"
  chmod 644 "$PANEL/history.jsonl"
  if (($(wc -l < "$PANEL/history.jsonl") > 5000)); then
    tail -n 5000 "$PANEL/history.jsonl" > "$PANEL/history.jsonl.tmp" && chmod 644 "$PANEL/history.jsonl.tmp" \
      && mv -f "$PANEL/history.jsonl.tmp" "$PANEL/history.jsonl"
  fi
}

xmage_id() { docker ps -q -f label=com.docker.compose.project=xmage -f label=com.docker.compose.service=xmage | head -1; }

# the server's /healthz through its container (the image has no curl), as the deploy does
healthz() {
  local id answer
  id=$(xmage_id)
  [[ -n $id ]] || return 1
  # shellcheck disable=SC2016 # expanded by the container's bash
  answer=$(timeout 10 docker exec "$id" bash -c 'exec 3<>/dev/tcp/127.0.0.1/17173 && printf "GET /healthz HTTP/1.0\r\n\r\n" >&3 && cat <&3' 2>/dev/null | tr -d '\r') || return 1
  [[ $(head -n 1 <<< "$answer" | cut -d' ' -f2) == 200 ]] || return 1
  sed '1,/^$/d' <<< "$answer"
}

json_field() { python3 -c 'import json,sys; v=json.load(sys.stdin).get(sys.argv[1]); print("" if v is None else v)' "$1"; }

# the server container's CPU use, percent of one core, over about 2 seconds
xmage_cpu() {
  local id
  id=$(xmage_id)
  [[ -n $id ]] || return 1
  docker stats --no-stream --format '{{.CPUPerc}}' "$id" 2>/dev/null | tr -d '%' | cut -d. -f1
}

in_window() { # in_window HH:MM-HH:MM or HH-HH (UTC), now
  local w=$1 now from to
  [[ -n $w ]] || return 1
  now=$(date -u +%H%M)
  from=${w%-*}; to=${w#*-}
  [[ $from == *:* ]] || from=$from:00
  [[ $to == *:* ]] || to=$to:00
  from=${from/:/}; to=${to/:/}
  if ((10#$from <= 10#$to)); then ((10#$now >= 10#$from && 10#$now < 10#$to)); else ((10#$now >= 10#$from || 10#$now < 10#$to)); fi
}

# busy REASON on stdout and 0 when the worker must not play; 1 when idle
busy() {
  local h games cpu avail full used
  apply_settings
  if ((PAUSED)); then echo "paused from the admin console"; return 0; fi
  if in_window "$BACKUP_UTC"; then echo "the backup window ($BACKUP_UTC UTC)"; return 0; fi
  if [[ -n $QUIET_UTC ]] && ! in_window "$QUIET_UTC"; then echo "outside the quiet hours ($QUIET_UTC UTC)"; return 0; fi
  h=$(healthz) || { echo "no /healthz answer"; return 0; }
  games=$(json_field gamesRunning <<< "$h") || { echo "unreadable /healthz"; return 0; }
  [[ $games =~ ^[0-9]+$ ]] || { echo "unreadable /healthz"; return 0; }
  ((games > GAMES_MAX)) && { echo "$games games running"; return 0; }
  [[ -n $(json_field maintenanceAt <<< "$h") ]] && { echo "maintenance planned"; return 0; }
  cpu=$(xmage_cpu) || { echo "no server container"; return 0; }
  [[ $cpu =~ ^[0-9]+$ ]] || { echo "unreadable server CPU"; return 0; }
  ((cpu > CPU_MAX)) && { echo "the server uses ${cpu}% CPU"; return 0; }
  avail=$(awk '/MemAvailable/ {print int($2 / 1024)}' /proc/meminfo)
  ((avail < MEM_MIN_MB)) && { echo "${avail} MB of memory available"; return 0; }
  full=$(df --output=pcent / | tail -1 | tr -dc 0-9)
  ((100 - full < DISK_MIN_PCT)) && { echo "the disk is ${full}% full"; return 0; }
  used=$(du -sm "$SPOOL/out" 2>/dev/null | cut -f1)
  ((${used:-0} > SPOOL_MAX_MB)) && { echo "the spool holds ${used} MB (nobody pulled)"; return 0; }
  return 1
}

if [[ ${1:-} == check ]]; then
  if reason=$(busy); then echo "busy: $reason"; else echo idle; fi
  exit 0
fi

exec 9> /run/playmat-rl.lock
flock -n 9 || exit 0
# a container left by a run that died (a reboot, a kill) never keeps playing
docker rm -f "$NAME" > /dev/null 2>&1

if reason=$(busy); then
  if ((PAUSED)); then status paused "$reason"; else status busy "$reason"; fi
  exit 0
fi

job=$(readlink -f "$SPOOL/job/current" 2>/dev/null) || true
if [[ -z $job || ! -r $job/job.json ]]; then status idle "no job"; exit 0; fi
# the job as transfer validated it (rl-transfer.sh put-job); checked again, since this runs as root
eval "$(python3 - "$job/job.json" "$IMAGE_RE" << 'EOF'
import json, re, shlex, sys
j = json.load(open(sys.argv[1]))
ok = (re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]{0,63}", str(j.get("id", ""))) and ".." not in str(j.get("id")) and re.fullmatch(sys.argv[2], str(j.get("image", "")))
      and re.fullmatch(r"cfg:[A-Za-z0-9_=+./-]{1,1000}", str(j.get("learner", "")))
      and str(j.get("fast", "")) in ("", "net", "quick") and isinstance(j.get("keep"), int) and 1 <= j["keep"] <= 1000
      and isinstance(j.get("seed_base"), int) and 0 <= j["seed_base"] < 2**31 - 2**21
      and isinstance(j.get("tag"), int) and -2**31 < j["tag"] < 2**31)
if not ok:
    print("JOB_OK=0")
else:
    for k in ["id", "image", "learner", "fast", "keep", "seed_base", "tag", "net_sha256", "rollout_sha256"]:
        print(f"J_{k.upper()}={shlex.quote(str(j.get(k, '')))}")
    print("JOB_OK=1")
EOF
)"
[[ ${JOB_OK:-0} == 1 ]] || { status error "invalid job.json"; log "invalid job.json in $job"; exit 0; }
for f in net rollout; do
  want=$(eval echo "\$J_${f^^}_SHA256")
  [[ -z $want || ! -e $job/$f.pmnet ]] && continue
  [[ $(sha256sum "$job/$f.pmnet" | cut -d' ' -f1) == "$want" ]] || { status error "$f.pmnet doesn't match job.json"; exit 0; }
done

# seeds: one block of CHUNK_GAMES per kept chunk, inside the job's 500,000 (the trainer gives each round's job
# 1,000,000 * round + 500,000; its own games take the first 500,000)
state=$SPOOL/state/$J_ID.next
n=$(cat "$state" 2>/dev/null || echo 0)
seed=$((J_SEED_BASE + n * CHUNK_GAMES))
((n * CHUNK_GAMES + CHUNK_GAMES <= 500000)) || { status idle "job $J_ID used its seed range"; exit 0; }

if ! docker image inspect "$J_IMAGE" > /dev/null 2>&1; then
  status pulling "$J_IMAGE"
  timeout 600 docker pull -q "$J_IMAGE" > /dev/null || { status error "can't pull $J_IMAGE"; log "can't pull $J_IMAGE"; exit 0; }
fi
commit=$(docker image inspect -f '{{index .Config.Labels "org.opencontainers.image.revision"}}' "$J_IMAGE")

chunk=$(printf 'c%06d' "$n")
work=$SPOOL/work/$J_ID-$chunk
rm -rf "$SPOOL/work"/*
mkdir -p "$work" "$SPOOL/out/$J_ID" "$SPOOL/state"
chown playmat-rl:playmat-rl "$work"
uid=$(id -u playmat-rl); gid=$(id -g playmat-rl)
args=(--out /out --games "$CHUNK_GAMES" --jobs "$JOBS" --keep "$J_KEEP" --learner "$J_LEARNER" --opponents self
  --seed "$seed" --tag "$J_TAG")
[[ -n $J_FAST ]] && args+=(--fast "$J_FAST")
start=$(date +%s)
docker run -d --name "$NAME" --cpus "$JOBS" --cpu-shares 2 --blkio-weight 10 --memory "$MEMORY" --memory-swap "$MEMORY" \
  --oom-score-adj 1000 --network none --read-only --cap-drop ALL --security-opt no-new-privileges --pids-limit 64 \
  --user "$uid:$gid" -v "$job:/job:ro" -v "$work:/out" "$J_IMAGE" "${args[@]}" > /dev/null \
  || { status error "docker run failed"; log "docker run failed"; exit 0; }
status playing "job $J_ID $chunk, seeds $seed+"

# one CPU spike (a single AI turn) isn't players coming back: that takes two readings in a row; anything else stops
# the chunk at once
why=
strikes=0
while :; do
  sleep 15
  [[ $(docker inspect -f '{{.State.Running}}' "$NAME" 2>/dev/null) == true ]] || break
  if reason=$(busy); then
    strikes=$((strikes + 1))
    [[ $reason == paused* || $reason != "the server uses "* || $strikes -ge 2 ]] && { why=$reason; break; }
  else
    strikes=0
  fi
  (($(date +%s) - start > MAX_RUN_S)) && { why="over ${MAX_RUN_S} s"; break; }
done
if [[ -n $why ]]; then
  docker rm -f "$NAME" > /dev/null 2>&1
  rm -rf "$work"
  record stopped 0 $(($(date +%s) - start)) "$why"
  status stopped "chunk $chunk thrown away: $why"
  log "stopped chunk $J_ID/$chunk: $why"
  exit 0
fi
code=$(docker inspect -f '{{.State.ExitCode}}' "$NAME")
summary=$(docker logs "$NAME" 2>/dev/null | grep -m1 '^SELFPLAY [0-9]* games' || true)
docker rm -f "$NAME" > /dev/null 2>&1
if [[ $code != 0 || -z $summary ]]; then
  rm -rf "$work"
  record failed 0 $(($(date +%s) - start)) "selfplay exited $code"
  status error "selfplay exited $code"
  log "selfplay exited $code on $J_ID/$chunk"
  exit 0
fi
secs=$(($(date +%s) - start))
out=$SPOOL/out/$J_ID
tar -czf "$out/$chunk.tar.gz.tmp" -C "$work" . && mv -f "$out/$chunk.tar.gz.tmp" "$out/$chunk.tar.gz"
rm -rf "$work"
sum=$(sha256sum "$out/$chunk.tar.gz" | cut -d' ' -f1)
bytes=$(stat -c %s "$out/$chunk.tar.gz")
# the .json is the chunk's ready marker, written last
python3 - "$out/$chunk.json" << EOF
import json, sys
json.dump({"job": "$J_ID", "chunk": "$chunk", "seed": $seed, "games": $CHUNK_GAMES, "seconds": $secs,
           "commit": "$commit", "image": "$J_IMAGE", "net_sha256": "$J_NET_SHA256",
           "rollout_sha256": "$J_ROLLOUT_SHA256", "sha256": "$sum", "bytes": $bytes,
           "summary": """$summary""", "finished": "$(date -u +%FT%TZ)"}, open(sys.argv[1] + ".tmp", "w"))
EOF
mv -f "$out/$chunk.json.tmp" "$out/$chunk.json"
chown -R playmat-rl:playmat-rl "$out"
echo $((n + 1)) > "$state"
record "done" "$CHUNK_GAMES" "$secs" ""
status idle "chunk $chunk done in ${secs} s"
log "chunk $J_ID/$chunk: $CHUNK_GAMES games in ${secs} s, $((bytes / 1024)) KB"
