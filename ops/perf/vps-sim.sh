#!/bin/bash
# Simulate a 1 vCPU / 1 GB VPS on a Docker host reached over SSH, and play games on it from this machine.
#
#   ops/perf/vps-sim.sh <image> <label> [extra docker run args]
#
#   VPS_HOST       ssh alias of the Docker host (default multivac)
#   VPS_ENV_FILE   small-vps.env as a path on that host (default the Multivac checkout's)
#   GAMES, WAVES   games at once against the AI, and how many rounds (default 3 and 1)
#
# The image runs as container xmage-vps (--cpus=1 --memory=1g), published on the host's 127.0.0.1:18172 only and
# reached through an SSH tunnel on local port 18272, with a throwaway admin password so the load test can sample
# server stats. Results go to ops/perf/results/vps-<label>: facts.txt (ready time, idle and after-game memory),
# boot.txt, stats.txt (docker stats every 10 s) and load.log.
set -u
TAG=$1; LABEL=$2; EXTRA=${3:-}
HOST=${VPS_HOST:-multivac}
ENV_FILE=${VPS_ENV_FILE:-C:\\Users\\neri\\xmage-server\\ops\\web\\small-vps.env}
HERE=$(cd "$(dirname "$0")" && pwd)
OUT=$HERE/results/vps-$LABEL; mkdir -p "$OUT"
PASS=$(openssl rand -hex 12)
m() { ssh -o BatchMode=yes "$HOST" "$@" 2>/dev/null; }
stats() { m 'docker stats --no-stream --format "{{.MemUsage}} {{.CPUPerc}}" xmage-vps'; }
m "docker rm -f xmage-vps" >/dev/null
T0=$(date +%s)
m "docker run -d --rm --name xmage-vps --cpus=1 --memory=1g -p 127.0.0.1:18172:17172 --env-file $ENV_FILE -e XMAGE_ALLOWED_ORIGINS=http://localhost -e XMAGE_ADMIN_PASSWORD=$PASS $EXTRA $TAG" >/dev/null
for _ in $(seq 1 120); do m "docker logs xmage-vps 2>&1" | grep -q "Started MAGE WebSocket" && break; sleep 2; done
echo "readySec=$(( $(date +%s) - T0 ))" > "$OUT/facts.txt"
m "docker logs xmage-vps 2>&1" | grep -E "xmage-server: machine|Loading|Started MAGE WebSocket|ERROR" | head -12 > "$OUT/boot.txt"
sleep 45
echo "idle=$(stats)" >> "$OUT/facts.txt"
ssh -o BatchMode=yes -o ExitOnForwardFailure=yes -o ServerAliveInterval=15 -N -L 18272:127.0.0.1:18172 "$HOST" & TUNNEL=$!
sleep 3
( while true; do echo "$(date +%s) $(stats)"; sleep 10; done ) > "$OUT/stats.txt" & SAMPLER=$!
(cd "$HERE/../../Mage.Web.Client" && MAGE_LOAD_TEST=1 MAGE_SERVER_URL=ws://127.0.0.1:18272 MAGE_ADMIN_PASSWORD=$PASS \
  MAGE_LOAD_GAMES=${GAMES:-3} MAGE_LOAD_WAVES=${WAVES:-1} MAGE_BOT_TIMEOUT_MS=900000 \
  npx vitest run src/core/headless/loadTest --silent=false) > "$OUT/load.log" 2>&1
kill "$SAMPLER" 2>/dev/null
sleep 20
echo "after=$(stats)" >> "$OUT/facts.txt"
echo "errorLines=$(m "docker logs xmage-vps 2>&1" | grep -cE "^ERROR|Exception")" >> "$OUT/facts.txt"
kill "$TUNNEL" 2>/dev/null
m "docker stop xmage-vps" >/dev/null
cat "$OUT/facts.txt"; grep -E "wall|finished|server CPU|heap MB|Tests " "$OUT/load.log" | tail -6
