#!/bin/bash
# backup.sh against throwaway SQLite databases: compacted copies (VACUUM INTO), the free-space guard, stale partial
# cleanup, retention by count, and restore. Needs Linux (GNU find/date/head/stat) and sqlite3; no Docker or server.
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
export XMAGE_DB_DIR=$work/db BACKUP_DIR=$work/backups BACKUP_KEEP=7 BACKUP_MIN_FREE_MB=1 BACKUP_SERVER_HOST=127.0.0.1
mkdir -p "$XMAGE_DB_DIR" "$BACKUP_DIR"
run() { bash "$script" "$@"; }

# career.db: normal rows; solo.db: 2 MB of random blobs, half of them deleted (free pages to compact)
sqlite3 "$XMAGE_DB_DIR/career.db" "CREATE TABLE t (v TEXT); WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 5000)
  INSERT INTO t SELECT printf('player %d won a game of Pioneer', i) FROM n;"
sqlite3 "$XMAGE_DB_DIR/solo.db" "CREATE TABLE s (id INTEGER, b BLOB); WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 40)
  INSERT INTO s SELECT i, randomblob(50000) FROM n; DELETE FROM s WHERE id > 20;"
live_solo_bytes=$(stat -c %s "$XMAGE_DB_DIR/solo.db")

# --- a backup: compacted SQLite copies
out=$(BACKUP_FREE_BYTES=100000000000 run once 2>&1) || fail "once: $out"
name=$(run list | tail -n 1)
b=$BACKUP_DIR/$name
[[ -f $b/career.db && -f $b/solo.db ]] && ok "SQLite databases copied" || fail "missing copies: $(ls "$b")"
copy_solo_bytes=$(stat -c %s "$b/solo.db")
(( copy_solo_bytes < live_solo_bytes )) \
  && ok "solo.db copy compacted ($copy_solo_bytes < $live_solo_bytes bytes)" \
  || fail "solo.db copy not compacted ($copy_solo_bytes vs $live_solo_bytes)"
[[ $(sqlite3 "$b/solo.db" "PRAGMA integrity_check") == ok && $(sqlite3 "$b/solo.db" "SELECT count(*) FROM s") == 20 ]] \
  && ok "the solo.db copy is whole" || fail "solo.db copy"
grep -q '"ok":true' "$XMAGE_DB_DIR/backup-status.json" && ok "status ok" || fail "status: $(cat "$XMAGE_DB_DIR/backup-status.json")"

# --- the free-space guard: fails loudly, writes failed status, nothing written, old partials cleaned
mkdir -p "$BACKUP_DIR/.partial-20200101T000000Z"
touch -d '3 hours ago' "$BACKUP_DIR/.partial-20200101T000000Z"
count=$(run list | wc -l)
out=$(BACKUP_FREE_BYTES=1024 run once 2>&1); code=$?
(( code != 0 )) && ok "a full disk refuses the run (exit $code)" || fail "guard exit $code: $out"
[[ $out == *"not enough disk for a backup"* ]] && ok "and says so in the log" || fail "log: $out"
grep -q '"ok":false' "$XMAGE_DB_DIR/backup-status.json" \
  && ok "status: failed" || fail "status: $(cat "$XMAGE_DB_DIR/backup-status.json")"
(( $(run list | wc -l) == count )) && ok "no backup written" || fail "a backup was written anyway"
[[ ! -e $BACKUP_DIR/.partial-20200101T000000Z ]] && ok "stale partials removed" || fail "partial still there"
# Wait a second so the next timestamped backup name differs
sleep 1
out=$(BACKUP_FREE_BYTES=100000000000 run once 2>&1) && ok "with room it runs again" || fail "once with room: $out"

# --- retention by count
for d in 20200101T000000Z 20200102T000000Z 20200103T000000Z 20200104T000000Z; do
  mkdir -p "$BACKUP_DIR/$d"; head -c 1024 /dev/urandom > "$BACKUP_DIR/$d/solo.db"
done
sleep 1
out=$(BACKUP_KEEP=3 BACKUP_FREE_BYTES=100000000000 run once 2>&1) || fail "keep run: $out"
(( $(run list | wc -l) == 3 )) && ok "BACKUP_KEEP=3 keeps three" || fail "kept: $(run list | tr '\n' ' ')"
[[ -z $(run list | grep '^2020') ]] && ok "the oldest went first" || fail "old ones left: $(run list | tr '\n' ' ')"

# --- restore: databases come back intact
newest=$(run list | tail -n 1)
rm -f "$XMAGE_DB_DIR/career.db" "$XMAGE_DB_DIR/solo.db"
out=$(run restore "$newest" 2>&1) || fail "restore: $out"
[[ $(sqlite3 "$XMAGE_DB_DIR/career.db" "SELECT count(*) FROM t") == 5000 ]] && ok "career.db restored" || fail "career.db after restore"
[[ $(sqlite3 "$XMAGE_DB_DIR/solo.db" "SELECT count(*) FROM s") == 20 ]] && ok "solo.db restored" || fail "solo.db after restore"

if command -v shellcheck > /dev/null; then
  shellcheck -S warning "$script" "$0" || fail "shellcheck"
fi
(( failed == 0 )) && echo "all passed" || { echo "$failed failed"; exit 1; }
