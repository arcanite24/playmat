#!/usr/bin/env bash
# Performance harness: starts a server from a local distribution, drives a load scenario through the WebSocket bridge,
# samples CPU and memory, records JFR, and writes a results folder with summary.json and report.txt.
#
#   ops/perf/perf.sh dist                 build the server distribution into ops/perf/.dist (about 5 minutes)
#   ops/perf/perf.sh <scenario>           run one scenario: startup | idle | ai | human | hold
#   ops/perf/perf.sh compare <dir> <dir>  compare two results folders
#
# From Mage.Web.Client: npm run perf -- <command>. See ops/perf/README.md for knobs and the 12 Oct 2026 baseline.
set -euo pipefail

HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
ROOT=$(cd "$HERE/../.." && pwd)
CLIENT="$ROOT/Mage.Web.Client"
DIST=${PERF_DIST:-$HERE/.dist}
RESULTS=${PERF_RESULTS:-$HERE/results}
PORT=${PERF_PORT:-18171}
WS_PORT=${PERF_WS_PORT:-18172}
HEALTH_PORT=${PERF_HEALTH_PORT:-18173}
ADMIN=${PERF_ADMIN_PASSWORD:-perfharnessadmin}  # letters and digits: the server sanitizes the property
JVM=${PERF_JVM:--Xmx2g}
SERVER_OPTS=${PERF_SERVER_OPTS:-}
# java: clients connect to the Java bridge; gateway: through the Rust gateway (docs/RustMigration.md, phase 1)
BACKEND=${PERF_BACKEND:-java}
GATEWAY_PORT=${PERF_GATEWAY_PORT:-18180}
GATEWAY_BIN=${PERF_GATEWAY_BIN:-$ROOT/rust/target/release/playmat-gateway}
case "$BACKEND" in
  java) CLIENT_URL="ws://127.0.0.1:$WS_PORT" ;;
  gateway) CLIENT_URL="ws://127.0.0.1:$GATEWAY_PORT" ;;
  *) echo "perf: PERF_BACKEND must be java or gateway" >&2; exit 1 ;;
esac
# a fixed set of real decks for the ai scenario: the AI seats rotate through Mage.Server/career-src/<suite>/*.txt
if [ -n "${PERF_DECK_SUITE:-}" ] && [ -z "${MAGE_PERF_DECKS:-}" ]; then
  MAGE_PERF_DECKS=$(ls "$ROOT/Mage.Server/career-src/$PERF_DECK_SUITE"/*.txt 2>/dev/null | paste -sd, -)
  [ -n "$MAGE_PERF_DECKS" ] || { echo "perf: no decks in Mage.Server/career-src/$PERF_DECK_SUITE" >&2; exit 1; }
  export MAGE_PERF_DECKS
fi

die() { echo "perf: $*" >&2; exit 1; }
now() { python3 -c 'import time; print(f"{time.time():.3f}")'; }

java_home() {
  if [ -n "${JAVA_HOME:-}" ]; then echo "$JAVA_HOME"
  elif [ -x /usr/libexec/java_home ]; then /usr/libexec/java_home
  else dirname "$(dirname "$(readlink -f "$(command -v java)")")"
  fi
}
JHOME=$(java_home)
JAVA="$JHOME/bin/java"
JCMD="$JHOME/bin/jcmd"
JFR="$JHOME/bin/jfr"
[ -x "$JAVA" ] || die "no java under $JHOME (set JAVA_HOME)"

# --- process probes (macOS and Linux) -------------------------------------------------------------
cpu_seconds() { # total CPU time of a process, in seconds
  ps -o time= -p "$1" | python3 -c '
import sys
t = sys.stdin.read().strip()
days = 0
if "-" in t: d, t = t.split("-"); days = int(d)
parts = [float(p) for p in t.split(":")]
s = 0
for p in parts: s = s * 60 + p
print(f"{s + days * 86400:.2f}")'
}
rss_mb() { ps -o rss= -p "$1" | awk '{printf "%.0f", $1 / 1024}'; }
threads() {
  if [ -r "/proc/$1/status" ]; then awk '/^Threads:/ {print $2}' "/proc/$1/status"
  else ps -M -p "$1" | tail -n +2 | wc -l | tr -d ' '
  fi
}
live_heap_mb() { # full GC, then used heap
  "$JCMD" "$1" GC.run >/dev/null 2>&1 || true
  "$JCMD" "$1" GC.heap_info 2>/dev/null | python3 -c '
import re, sys
text = sys.stdin.read()
# one heap line for G1, one per generation for Serial and Parallel; Metaspace and class space are not heap
used = [int(k) for name, k in re.findall(r"^\s*([A-Za-z][\w -]*?)\s+total \d+K, used (\d+)K", text, re.M)
        if not name.startswith(("Metaspace", "class space"))]
print(round(sum(used) / 1024) if used else -1)'
}
metaspace_mb() {
  "$JCMD" "$1" GC.heap_info 2>/dev/null | python3 -c '
import re, sys
m = re.search(r"Metaspace\s+used (\d+)K", sys.stdin.read())
print(round(int(m.group(1)) / 1024) if m else -1)'
}

# --- server lifecycle ------------------------------------------------------------------------------
SERVER_PID=
start_server() { # start_server <log> [extra jvm flags...]; sets SERVER_PID and READY_SEC
  local log=$1; shift
  [ -d "$DIST/lib" ] || die "no distribution at $DIST; run: ops/perf/perf.sh dist"
  if nc -z 127.0.0.1 "$WS_PORT" 2>/dev/null; then die "port $WS_PORT is busy (another server?); set PERF_WS_PORT"; fi
  local t0; t0=$(now)
  # shellcheck disable=SC2086
  (cd "$DIST" && exec "$JAVA" $JVM "$@" $SERVER_OPTS -Dxmage.testMode=false -Dxmage.healthPort="$HEALTH_PORT" \
      -Dxmage.adminPassword="$ADMIN" -Dxmage.web.maxConnectionsPerIp=1000 -jar lib/mage-server-*.jar) >"$log" 2>&1 &
  SERVER_PID=$!
  for _ in $(seq 1 12000); do
    kill -0 "$SERVER_PID" 2>/dev/null || { tail -20 "$log"; die "server died while starting"; }
    nc -z 127.0.0.1 "$WS_PORT" 2>/dev/null && break
    sleep 0.05
  done
  READY_SEC=$(python3 -c "print(round($(now) - $t0, 2))")
  [ "$BACKEND" = gateway ] && start_gateway "$log.gateway"
  return 0
}
GATEWAY_PID=
start_gateway() { # start_gateway <log>: the Rust gateway in front of the Java bridge; sets GATEWAY_PID
  [ -x "$GATEWAY_BIN" ] || die "no gateway at $GATEWAY_BIN; build it: (cd rust && cargo build --release)"
  if nc -z 127.0.0.1 "$GATEWAY_PORT" 2>/dev/null; then die "port $GATEWAY_PORT is busy; set PERF_GATEWAY_PORT"; fi
  XMAGE_GATEWAY_LISTEN="127.0.0.1:$GATEWAY_PORT" XMAGE_GATEWAY_JAVA_URL="ws://127.0.0.1:$WS_PORT" "$GATEWAY_BIN" >"$1" 2>&1 &
  GATEWAY_PID=$!
  for _ in $(seq 1 200); do
    nc -z 127.0.0.1 "$GATEWAY_PORT" 2>/dev/null && return 0
    kill -0 "$GATEWAY_PID" 2>/dev/null || { tail -20 "$1"; die "gateway died while starting"; }
    sleep 0.05
  done
  die "gateway did not open port $GATEWAY_PORT"
}
stop_gateway() {
  [ -n "$GATEWAY_PID" ] || return 0
  kill "$GATEWAY_PID" 2>/dev/null || true
  wait "$GATEWAY_PID" 2>/dev/null || true
  GATEWAY_PID=
}
stop_server() {
  stop_gateway
  [ -n "$SERVER_PID" ] || return 0
  kill "$SERVER_PID" 2>/dev/null || true
  for _ in $(seq 1 200); do kill -0 "$SERVER_PID" 2>/dev/null || { SERVER_PID=; return 0; }; sleep 0.1; done
  kill -9 "$SERVER_PID" 2>/dev/null || true
  SERVER_PID=
}
trap 'stop_server; [ -n "${SAMPLER_PID:-}" ] && kill "$SAMPLER_PID" 2>/dev/null; true' EXIT

SAMPLER_PID=
start_sampler() { # every 2 s: epoch, cpu seconds, rss MB, threads
  local out=$1
  echo "epoch cpuSec rssMB threads" >"$out"
  ( while kill -0 "$SERVER_PID" 2>/dev/null; do
      echo "$(date +%s) $(cpu_seconds "$SERVER_PID") $(rss_mb "$SERVER_PID") $(threads "$SERVER_PID")" >>"$out"
      sleep 2
    done ) &
  SAMPLER_PID=$!
}
stop_sampler() { [ -n "$SAMPLER_PID" ] || return 0; kill "$SAMPLER_PID" 2>/dev/null; wait "$SAMPLER_PID" 2>/dev/null || true; SAMPLER_PID=; }

jfr_start() { [ "${PERF_JFR:-1}" = 1 ] || return 0; python3 "$HERE/perf.py" jfc "$JHOME" "$RUN/perf.jfc"; "$JCMD" "$SERVER_PID" JFR.start name=perf settings="$RUN/perf.jfc" filename="$RUN/recording.jfr" >/dev/null; }
jfr_stop() { [ "${PERF_JFR:-1}" = 1 ] || return 0; "$JCMD" "$SERVER_PID" JFR.stop name=perf >/dev/null; }

snapshot() { # snapshot <prefix>: memory facts after a full GC, written as key=value lines to facts.txt
  local p=$1
  {
    echo "${p}LiveHeapMB=$(live_heap_mb "$SERVER_PID")"
    echo "${p}MetaspaceMB=$(metaspace_mb "$SERVER_PID")"
    echo "${p}RssMB=$(rss_mb "$SERVER_PID")"
    echo "${p}Threads=$(threads "$SERVER_PID")"
  } >>"$RUN/facts.txt"
}

new_run() {
  RUN="$RESULTS/$(date -u +%Y%m%dT%H%M%SZ)-$1"
  mkdir -p "$RUN"
  {
    echo "scenario=$1"
    echo "commit=$(git -C "$ROOT" rev-parse --short HEAD 2>/dev/null || echo unknown)"
    echo "dirty=$(git -C "$ROOT" status --porcelain 2>/dev/null | grep -qv '^??' && echo yes || echo no)"
    echo "jvm=$JVM $SERVER_OPTS"
    echo "java=$("$JAVA" -version 2>&1 | head -1)"
    echo "host=$(uname -sm) cores=$(getconf _NPROCESSORS_ONLN)"
    echo "backend=$BACKEND"
    [ -n "${MAGE_PERF_DECKS:-}" ] && echo "decks=${PERF_DECK_SUITE:-custom} ($(echo "$MAGE_PERF_DECKS" | tr ',' '\n' | wc -l | tr -d ' '))"
  } >"$RUN/facts.txt"
  echo "perf: results in $RUN"
}

vitest() { # vitest <log> <test filter> with env already exported
  (cd "$CLIENT" && MAGE_SERVER_URL="$CLIENT_URL" npx vitest run "$2" --silent=false) >"$1" 2>&1 || true
  grep -E '^\[perf-result\]|^\[load\]' -A6 "$1" || echo "perf: the load produced no result line; see $1"
}

measure_load() { # measure_load <name> <vitest filter>: run the load with JFR recording and sampling
  snapshot before
  jfr_start
  start_sampler "$RUN/samples.txt"
  local c0 c1 g0 g1
  c0=$(cpu_seconds "$SERVER_PID")
  [ -n "$GATEWAY_PID" ] && g0=$(cpu_seconds "$GATEWAY_PID")
  vitest "$RUN/load.log" "$2"
  c1=$(cpu_seconds "$SERVER_PID")
  stop_sampler
  jfr_stop
  echo "loadCpuSec=$(python3 -c "print(round($c1 - $c0, 2))")" >>"$RUN/facts.txt"
  if [ -n "$GATEWAY_PID" ]; then
    g1=$(cpu_seconds "$GATEWAY_PID")
    echo "gatewayLoadCpuSec=$(python3 -c "print(round($g1 - $g0, 2))")" >>"$RUN/facts.txt"
    echo "gatewayRssMB=$(rss_mb "$GATEWAY_PID")" >>"$RUN/facts.txt"
    curl -s "http://127.0.0.1:$GATEWAY_PORT/metrics" | grep -v '^#' >"$RUN/gateway-metrics.txt" || true
  fi
  snapshot after
}

finish() {
  "$JCMD" "$SERVER_PID" GC.class_histogram 2>/dev/null | head -45 >"$RUN/class-histogram.txt" || true
  stop_server
  cp "$DIST/mageserver.log" "$RUN/mageserver.log" 2>/dev/null || true
  echo "thinkTooLong=$(grep -c 'thinks too long' "$RUN/mageserver.log" 2>/dev/null || echo 0)" >>"$RUN/facts.txt"
  if [ -f "$RUN/recording.jfr" ]; then
    python3 "$HERE/perf.py" report "$JFR" "$RUN/recording.jfr" >"$RUN/report.txt" || echo "perf: JFR report failed"
  fi
  python3 "$HERE/perf.py" summarize "$RUN"
}

# --- commands ----------------------------------------------------------------------------------------
cmd=${1:-help}; shift || true
case "$cmd" in
  dist)
    if [ "${1:-}" != "--no-build" ]; then
      (cd "$ROOT" && mvn -B -T 1C -pl '!Mage.Client,!Mage.Server.Console,!Mage.Tests,!Mage.Verify,!Mage.Reports' clean install -DskipTests -q)
      (cd "$ROOT/Mage.Server" && mvn -B -q package assembly:single -DskipTests)
    fi
    zip="$ROOT/Mage.Server/target/mage-server.zip"
    [ -f "$zip" ] || die "no $zip; run without --no-build"
    rm -rf "$DIST" && mkdir -p "$DIST" && (cd "$DIST" && unzip -q "$zip")
    # the assembly packs whatever is in Mage.Server/db (a developer's own data, possibly mid-write): start empty
    rm -rf "$DIST/db" "$DIST/saved" && mkdir -p "$DIST/db"
    python3 "$HERE/perf.py" config "$DIST/config/config.xml" "$PORT" "$WS_PORT"
    echo "perf: distribution in $DIST; building its card database (a cold start)"
    new_run cold-start
    start_server "$RUN/server.out"
    echo "readySec=$READY_SEC" >>"$RUN/facts.txt"
    snapshot idle
    finish
    ;;
  startup)
    new_run startup
    for i in $(seq 1 "${PERF_RUNS:-3}"); do
      [ "${PERF_COLD:-0}" = 1 ] && rm -f "$DIST"/db/cards.h2.*
      start_server "$RUN/server-$i.out"
      echo "readySec=$READY_SEC" >>"$RUN/facts.txt"
      echo "readyCpuSec=$(cpu_seconds "$SERVER_PID")" >>"$RUN/facts.txt"
      [ "$i" = "${PERF_RUNS:-3}" ] && snapshot idle
      [ "$i" = "${PERF_RUNS:-3}" ] || stop_server
    done
    PERF_JFR=0 finish
    ;;
  idle)
    new_run idle
    start_server "$RUN/server.out" -XX:NativeMemoryTracking=summary
    echo "readySec=$READY_SEC" >>"$RUN/facts.txt"
    sleep "${PERF_SETTLE_SEC:-20}"
    snapshot idle
    c0=$(cpu_seconds "$SERVER_PID"); sleep "${PERF_IDLE_SEC:-60}"; c1=$(cpu_seconds "$SERVER_PID")
    echo "idleCpuPct=$(python3 -c "print(round(($c1 - $c0) * 100 / ${PERF_IDLE_SEC:-60}, 2))")" >>"$RUN/facts.txt"
    "$JCMD" "$SERVER_PID" VM.native_memory summary >"$RUN/native-memory.txt" 2>&1 || true
    "$JCMD" "$SERVER_PID" VM.metaspace >"$RUN/metaspace.txt" 2>&1 || true
    PERF_JFR=0 finish
    ;;
  ai)
    export MAGE_PERF=ai MAGE_PERF_CONC=${MAGE_PERF_CONC:-8} MAGE_PERF_DURATION_MS=${MAGE_PERF_DURATION_MS:-120000}
    new_run ai
    start_server "$RUN/server.out"
    echo "readySec=$READY_SEC" >>"$RUN/facts.txt"
    measure_load ai src/core/headless/perf
    finish
    ;;
  human)
    export MAGE_LOAD_TEST=1 MAGE_ADMIN_PASSWORD="$ADMIN" MAGE_LOAD_GAMES=${MAGE_LOAD_GAMES:-12} MAGE_LOAD_WAVES=${MAGE_LOAD_WAVES:-3}
    new_run human
    start_server "$RUN/server.out"
    echo "readySec=$READY_SEC" >>"$RUN/facts.txt"
    measure_load human src/core/headless/loadTest
    finish
    ;;
  hold)
    export MAGE_PERF=hold MAGE_PERF_N=${MAGE_PERF_N:-16} MAGE_PERF_HOLD_MS=${MAGE_PERF_HOLD_MS:-90000}
    new_run hold
    start_server "$RUN/server.out"
    echo "readySec=$READY_SEC" >>"$RUN/facts.txt"
    snapshot before
    start_sampler "$RUN/samples.txt"
    (cd "$CLIENT" && MAGE_SERVER_URL="$CLIENT_URL" npx vitest run src/core/headless/perf --silent=false) >"$RUN/load.log" 2>&1 &
    load=$!
    # measure while every game is parked: past the hold turn, before the hold ends
    sleep $(( MAGE_PERF_HOLD_MS * 7 / 10000 ))
    snapshot held
    wait "$load" || true
    grep -E '^\[perf-result\]' "$RUN/load.log" || echo "perf: no result line; see $RUN/load.log"
    stop_sampler
    snapshot after
    PERF_JFR=0 finish
    ;;
  compare)
    [ $# -eq 2 ] || die "usage: perf.sh compare <results dir> <results dir>"
    python3 "$HERE/perf.py" compare "$1" "$2"
    ;;
  *)
    sed -n '2,9p' "$0" | sed 's/^# \{0,1\}//'
    ;;
esac
