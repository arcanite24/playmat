#!/bin/bash
# Backs up the XMage user databases (not the card database, which the image rebuilds).
#
#   xmage-backup daemon          run once a day at $BACKUP_AT (default 03:30, container time zone $TZ)
#   xmage-backup once            take one backup now
#   xmage-backup list            list backups
#   xmage-backup restore NAME    restore backup NAME into the db volume (stop the xmage service first)
#
# Each backup is a directory /backups/<UTC timestamp>/ holding:
#   authorized_user.h2.sql.gz, feedback.h2.sql.gz   H2 SQL dumps (accounts, feedback)
#   user_stats.db, table_record.db                  SQLite copies (player stats, finished tables)
#   web_decks.db, web_reports.db, career.db         SQLite copies (synced decks, player reports, Career progress)
#   web_admin.db                                    SQLite copy (/admin: access, staff, audit log, settings)
#   support.db                                      SQLite copy (support ledger, sponsorships, perks, looks)
# H2 dumps are taken online: AUTO_SERVER lets this process join the running server's database.
# SQLite copies use the online backup API. The newest $BACKUP_KEEP (default 14) backups are kept.
# Every run writes db/backup-status.json, which the server reads for /admin and its staff alerts.
set -euo pipefail

DB_DIR=${XMAGE_DB_DIR:-/opt/xmage/db}
BACKUP_DIR=${BACKUP_DIR:-/backups}
BACKUP_AT=${BACKUP_AT:-03:30}
BACKUP_KEEP=${BACKUP_KEEP:-14}
H2_DBS=(authorized_user.h2 feedback.h2)
SQLITE_DBS=(user_stats.db table_record.db web_decks.db web_reports.db career.db web_admin.db support.db)
STATUS=$DB_DIR/backup-status.json

h2_jar() {
  local jars=(/opt/xmage/lib/h2-*.jar)
  [[ -f ${jars[0]} ]] || { echo "xmage-backup: H2 jar not found in /opt/xmage/lib" >&2; exit 1; }
  echo "${jars[0]}"
}

log() { echo "$(date -u +%FT%TZ) xmage-backup: $*"; }

# write_status ok|failed NAME [SIZE]: the last run, and the last good one (kept from the previous status)
write_status() {
  local ok=$1 name=$2 size=${3:-} now last_ok
  now=$(date -u +%s)
  last_ok=$(sed -n 's/.*"lastOkAt":\([0-9]*\).*/\1/p' "$STATUS" 2>/dev/null || true)
  [[ $ok == ok ]] && last_ok=$now
  printf '{"at":%s,"ok":%s,"name":"%s","size":"%s","lastOkAt":%s}\n' \
    "$now" "$([[ $ok == ok ]] && echo true || echo false)" "$name" "$size" "${last_ok:-0}" > "$STATUS.tmp" \
    && mv "$STATUS.tmp" "$STATUS" || log "can't write $STATUS"
}

backup_once() {
  local name tmp h2 size
  name=$(date -u +%Y%m%dT%H%M%SZ)
  tmp=$BACKUP_DIR/.partial-$name
  trap 'write_status failed "$name"' ERR
  h2=$(h2_jar)
  mkdir -p "$tmp"
  for db in "${H2_DBS[@]}"; do
    [[ -e $DB_DIR/$db.mv.db ]] || continue
    # With the server running, AUTO_SERVER hands the SCRIPT command to the server's process, which writes the file in
    # its own container. So the dump goes to the db volume both containers mount, at the same path, then moves here.
    local dump=$DB_DIR/.backup-$db.sql
    rm -f "$dump"
    java -Xmx128m -cp "$h2" org.h2.tools.Script \
      -url "jdbc:h2:file:$DB_DIR/$db;AUTO_SERVER=TRUE;IFEXISTS=TRUE" -user "" -password "" \
      -script "$dump"
    [[ -s $dump ]] || { log "the dump of $db is missing or empty"; false; }
    mv "$dump" "$tmp/$db.sql"
    gzip -9 "$tmp/$db.sql"
  done
  for db in "${SQLITE_DBS[@]}"; do
    [[ -e $DB_DIR/$db ]] || continue
    sqlite3 -cmd ".timeout 30000" "$DB_DIR/$db" ".backup '$tmp/$db'"
  done
  mv "$tmp" "$BACKUP_DIR/$name"
  size=$(du -sh "$BACKUP_DIR/$name" | cut -f1)
  log "backup $name done ($size)"
  trap - ERR
  write_status ok "$name" "$size"
  prune
}

prune() {
  local old
  [[ $BACKUP_KEEP =~ ^[1-9][0-9]*$ ]] || { echo "xmage-backup: BACKUP_KEEP must be 1 or more" >&2; return 1; }
  mapfile -t old < <(list_names | head -n "-$BACKUP_KEEP")
  for name in "${old[@]}"; do
    rm -rf "${BACKUP_DIR:?}/$name"
    log "removed old backup $name"
  done
  # leftovers of interrupted runs
  find "$BACKUP_DIR" -maxdepth 1 -name '.partial-*' -mmin +120 -exec rm -rf {} +
}

list_names() {
  find "$BACKUP_DIR" -mindepth 1 -maxdepth 1 -type d -name '2*' -printf '%f\n' | sort
}

restore() {
  local name=${1:?usage: xmage-backup restore NAME}
  local src=$BACKUP_DIR/$name h2
  [[ -d $src ]] || { echo "xmage-backup: no backup named $name (see: xmage-backup list)" >&2; exit 1; }
  if (exec 3<>/dev/tcp/xmage/17172) 2>/dev/null; then
    echo "xmage-backup: the xmage server is running; stop it first: docker compose stop xmage" >&2
    exit 1
  fi
  h2=$(h2_jar)
  for db in "${H2_DBS[@]}"; do
    [[ -f $src/$db.sql.gz ]] || continue
    rm -f "$DB_DIR/$db".*.db
    gunzip -c "$src/$db.sql.gz" > "/tmp/$db.sql"
    java -Xmx256m -cp "$h2" org.h2.tools.RunScript \
      -url "jdbc:h2:file:$DB_DIR/$db" -user "" -password "" -script "/tmp/$db.sql"
    rm -f "/tmp/$db.sql"
    log "restored $db"
  done
  for db in "${SQLITE_DBS[@]}"; do
    [[ -f $src/$db ]] || continue
    rm -f "$DB_DIR/$db" "$DB_DIR/$db-journal" "$DB_DIR/$db-wal" "$DB_DIR/$db-shm"
    cp "$src/$db" "$DB_DIR/$db"
    log "restored $db"
  done
  log "restore of $name complete; start the server: docker compose start xmage"
}

daemon() {
  [[ $BACKUP_AT =~ ^[0-2][0-9]:[0-5][0-9]$ ]] || { echo "xmage-backup: BACKUP_AT must be HH:MM" >&2; exit 1; }
  trap 'exit 0' TERM INT
  log "daily backups at $BACKUP_AT ($(date +%Z)), keeping $BACKUP_KEEP"
  while true; do
    local now next
    now=$(date +%s)
    next=$(date -d "today $BACKUP_AT" +%s)
    (( next > now )) || next=$(date -d "tomorrow $BACKUP_AT" +%s)
    sleep $(( next - now )) & wait $!
    # separate process, so set -e still stops a failing backup half way
    "$0" once || log "backup failed; retrying tomorrow"
  done
}

case "${1:-daemon}" in
  daemon) daemon ;;
  once) backup_once ;;
  list) list_names ;;
  restore) restore "${2:-}" ;;
  *) sed -n '2,8p' "$0" >&2; exit 2 ;;
esac
