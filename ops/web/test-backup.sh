#!/bin/bash
# backup.sh against throwaway databases: gzipped copies, the free-space guard, retention by count and by size,
# streaming (the deploy's snapshot) and restore. Needs Linux (GNU find/date/head), sqlite3 and gzip; no Docker or server.
#   ops/web/test-backup.sh
set -uo pipefail

here=$(cd "$(dirname "$0")" && pwd)
script=$here/backup.sh
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
failed=0

fail() { echo "FAIL $*"; failed=$((failed + 1)); }
ok() { echo "ok   $*"; }

# BACKUP_SERVER_HOST: nothing listens there, so restore goes ahead
export XMAGE_DB_DIR=$work/db BACKUP_DIR=$work/backups BACKUP_MAX_GB=0 BACKUP_KEEP=7 BACKUP_SERVER_HOST=127.0.0.1
mkdir -p "$XMAGE_DB_DIR" "$BACKUP_DIR"
run() { bash "$script" "$@"; }

# career.db: compressible rows; solo.db: 2 MB of random blobs, half of them deleted (free pages)
sqlite3 "$XMAGE_DB_DIR/career.db" "CREATE TABLE t (v TEXT); WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 5000)
  INSERT INTO t SELECT printf('player %d won a game of Pioneer', i) FROM n;"
sqlite3 "$XMAGE_DB_DIR/solo.db" "CREATE TABLE s (id INTEGER, b BLOB); WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 40)
  INSERT INTO s SELECT i, randomblob(50000) FROM n; DELETE FROM s WHERE id > 20;"

# --- a backup: gzipped copies
out=$(run once 2>&1) || fail "once: $out"
name=$(run list | tail -n 1)
b=$BACKUP_DIR/$name
[[ -f $b/career.db.gz && ! -f $b/career.db ]] && ok "career.db is gzipped" || fail "career.db.gz missing: $(ls "$b")"
[[ -f $b/solo.db && ! -f $b/solo.db.gz ]] && ok "solo.db is kept plain (BACKUP_RAW)" || fail "solo.db: $(ls "$b")"
[[ $(sqlite3 "$b/solo.db" "PRAGMA integrity_check") == ok && $(sqlite3 "$b/solo.db" "SELECT count(*) FROM s") == 20 ]] \
  && ok "the solo.db copy is whole" || fail "solo.db copy"
grep -q '"ok":true' "$XMAGE_DB_DIR/backup-status.json" && ok "status ok" || fail "status: $(cat "$XMAGE_DB_DIR/backup-status.json")"

# --- the free-space guard: skipped loudly, status says why, nothing written, old partials cleaned
mkdir -p "$BACKUP_DIR/.partial-20200101T000000Z"
count=$(run list | wc -l)
out=$(BACKUP_FREE_KB=10 run once 2>&1); code=$?
[[ $code == 3 ]] && ok "a full disk skips the run (exit 3)" || fail "guard exit $code: $out"
[[ $out == *"SKIPPED backup"*"MB free"* ]] && ok "and says so in the log" || fail "log: $out"
grep -q '"skipped":"only 0 MB free' "$XMAGE_DB_DIR/backup-status.json" && grep -q '"ok":false' "$XMAGE_DB_DIR/backup-status.json" \
  && ok "status: skipped, with the reason" || fail "status: $(cat "$XMAGE_DB_DIR/backup-status.json")"
(( $(run list | wc -l) == count )) && ok "no backup written" || fail "a backup was written anyway"
[[ ! -e $BACKUP_DIR/.partial-20200101T000000Z ]] && ok "leftover partials removed" || fail "partial still there"
out=$(BACKUP_FREE_KB=100000000 run once 2>&1) && ok "with room it runs again" || fail "once with room: $out"

# --- retention by count
for d in 20200101T000000Z 20200102T000000Z 20200103T000000Z 20200104T000000Z; do
  mkdir -p "$BACKUP_DIR/$d"; head -c 1048576 /dev/urandom > "$BACKUP_DIR/$d/solo.db"
done
out=$(BACKUP_KEEP=3 BACKUP_FREE_KB=100000000 run once 2>&1) || fail "keep run: $out"
(( $(run list | wc -l) == 3 )) && ok "BACKUP_KEEP=3 keeps three" || fail "kept: $(run list | tr '\n' ' ')"
[[ -z $(run list | grep '^2020') ]] && ok "the oldest went first" || fail "old ones left: $(run list | tr '\n' ' ')"

# --- retention by size: 1 MB budget keeps only the newest (it always stays)
for d in 20200105T000000Z 20200106T000000Z; do
  mkdir -p "$BACKUP_DIR/$d"; head -c 1048576 /dev/urandom > "$BACKUP_DIR/$d/solo.db"
done
BACKUP_MAX_MB=1 BACKUP_FREE_KB=100000000 run once > /dev/null 2>&1
newest=$(run list | tail -n 1)
(( $(run list | wc -l) == 1 )) && [[ $newest != 2020* ]] && ok "the size budget leaves only the newest" \
  || fail "budget kept: $(run list | tr '\n' ' ')"

# --- stream: a tar on stdout, nothing left here, no status change
before=$(run list | tr '\n' ' '); status=$(cat "$XMAGE_DB_DIR/backup-status.json")
BACKUP_FREE_KB=100000000 run stream > "$work/snap.tar" 2> "$work/stream.log" || fail "stream: $(cat "$work/stream.log")"
tar -tf "$work/snap.tar" | grep -q '/career.db.gz$' && ok "stream: a tar with the backup" || fail "tar: $(tar -tf "$work/snap.tar")"
[[ $(run list | tr '\n' ' ') == "$before" && -z $(find "$BACKUP_DIR" -name '.partial-*') ]] && ok "stream keeps nothing in the volume" \
  || fail "left: $(ls -a "$BACKUP_DIR")"
[[ $(cat "$XMAGE_DB_DIR/backup-status.json") == "$status" ]] && ok "stream leaves the status alone" || fail "status changed"
BACKUP_FREE_KB=10 run stream > /dev/null 2>&1; code=$?
[[ $code == 3 ]] && ok "stream on a full disk: exit 3" || fail "stream guard exit $code"

# --- restore: gzipped copies come back
rm -f "$XMAGE_DB_DIR/career.db" "$XMAGE_DB_DIR/solo.db"
out=$(run restore "$newest" 2>&1) || fail "restore: $out"
[[ $(sqlite3 "$XMAGE_DB_DIR/career.db" "SELECT count(*) FROM t") == 5000 ]] && ok "career.db restored from .gz" || fail "career.db after restore"
[[ $(sqlite3 "$XMAGE_DB_DIR/solo.db" "SELECT count(*) FROM s") == 20 ]] && ok "solo.db restored" || fail "solo.db after restore"

if command -v shellcheck > /dev/null; then
  shellcheck -S warning "$script" "$0" || fail "shellcheck"
fi
(( failed == 0 )) && echo "all passed" || { echo "$failed failed"; exit 1; }
