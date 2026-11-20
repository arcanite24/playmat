#!/bin/bash
# Backs up the XMage user databases (not the card database, which the image rebuilds).
#
#   xmage-backup daemon          run once a day at $BACKUP_AT (default 03:30, container time zone $TZ)
#   xmage-backup once            take one backup now
#   xmage-backup stream          write a fresh backup to stdout as a tar, keeping nothing here (the deploy's snapshot)
#   xmage-backup list            list backups
#   xmage-backup restore NAME    restore backup NAME into the db volume (stop the xmage service first)
#
# Each backup is a directory /backups/<UTC timestamp>/ holding:
#   authorized_user.h2.sql.gz, feedback.h2.sql.gz   H2 SQL dumps (accounts, feedback)
#   authorized_user.db                              SQLite copy (accounts since Nov 2026; the H2 dump is the copy from before)
#   user_stats.db, table_record.db                  SQLite copies (player stats, finished tables)
#   web_decks.db, web_reports.db, career.db         SQLite copies (synced decks, player reports, Career progress)
#   web_admin.db                                    SQLite copy (/admin: access, staff, audit log, settings)
#   support.db                                      SQLite copy (support ledger, sponsorships, perks, looks)
#   solo.db                                         SQLite copy (single player: saved games, ghosts, batches, scores;
#                                                   not gzipped, its turn snapshots are compressed already)
#   web_presence.db                                 SQLite copy (the status each player chose: hidden, looking to play)
#   ladder.db                                       SQLite copy (ladder ratings: players and bots, per format and queue)
#   content-packs.tar.gz                            the content packs folder (campaigns, puzzles, opponents staff added)
# H2 dumps are taken online: AUTO_SERVER lets this process join the running server's database.
# SQLite copies are taken online with VACUUM INTO (a compact copy, without the free pages), then gzipped as NAME.gz
# unless NAME is in $BACKUP_RAW (default solo.db). Restore takes both NAME and NAME.gz.
#
# Disk: the newest $BACKUP_KEEP (default 7) backups are kept, and fewer when together they pass $BACKUP_MAX_GB (default
# 15; 0 turns the budget off); the newest one always stays. Old ones go before a run, and a run is skipped (status
# "skipped", logged loudly) when the disk has less than twice the last backup's size free: a backup that fills the disk
# takes the server down with it (2026-11-20).
# Every run writes db/backup-status.json, which the server reads for /admin and its staff alerts.
set -Eeuo pipefail

DB_DIR=${XMAGE_DB_DIR:-/opt/xmage/db}
BACKUP_DIR=${BACKUP_DIR:-/backups}
BACKUP_AT=${BACKUP_AT:-03:30}
BACKUP_KEEP=${BACKUP_KEEP:-7}
BACKUP_MAX_GB=${BACKUP_MAX_GB:-15}
# finer than GB (tests); BACKUP_MAX_GB otherwise
BACKUP_MAX_MB=${BACKUP_MAX_MB:-}
BACKUP_RAW=${BACKUP_RAW:-solo.db}
# tests: pretend the disk has this many KB free; where restore looks for a running server
BACKUP_FREE_KB=${BACKUP_FREE_KB:-}
SERVER_HOST=${BACKUP_SERVER_HOST:-xmage}
H2_DBS=(authorized_user.h2 feedback.h2)
SQLITE_DBS=(authorized_user.db user_stats.db table_record.db web_decks.db web_reports.db career.db web_admin.db support.db solo.db web_presence.db ladder.db)
PACKS=content-packs
STATUS=$DB_DIR/backup-status.json

h2_jar() {
  local jars=(/opt/xmage/lib/h2-*.jar)
  [[ -f ${jars[0]} ]] || { echo "xmage-backup: H2 jar not found in /opt/xmage/lib" >&2; exit 1; }
  echo "${jars[0]}"
}

log() { echo "$(date -u +%FT%TZ) xmage-backup: $*"; }

# write_status ok|failed|skipped NAME [SIZE] [REASON]: the last run, and the last good one (kept from the previous status)
write_status() {
  local ok=$1 name=$2 size=${3:-} reason=${4:-} now last_ok
  now=$(date -u +%s)
  last_ok=$(sed -n 's/.*"lastOkAt":\([0-9]*\).*/\1/p' "$STATUS" 2>/dev/null || true)
  [[ $ok == ok ]] && last_ok=$now
  printf '{"at":%s,"ok":%s,"name":"%s","size":"%s","lastOkAt":%s,"skipped":"%s"}\n' \
    "$now" "$([[ $ok == ok ]] && echo true || echo false)" "$name" "$size" "${last_ok:-0}" "$reason" > "$STATUS.tmp" \
    && mv "$STATUS.tmp" "$STATUS" || log "can't write $STATUS"
}

# free_kb DIR: KB free on DIR's disk
free_kb() {
  if [[ -n $BACKUP_FREE_KB ]]; then echo "$BACKUP_FREE_KB"; return; fi
  df -Pk "$1" | awk 'NR == 2 { print $4 }'
}

# need_kb: what a new backup may take, twice over: the newest backup, or with none yet the databases themselves
need_kb() {
  local newest kb=0 db
  newest=$(list_names | tail -n 1)
  if [[ -n $newest ]]; then
    kb=$(du -sk "$BACKUP_DIR/$newest" | cut -f1)
  else
    for db in "${SQLITE_DBS[@]}" "${H2_DBS[@]/%/.mv.db}"; do
      [[ -e $DB_DIR/$db ]] && kb=$(( kb + $(du -sk "$DB_DIR/$db" | cut -f1) ))
    done
  fi
  echo $(( kb * 2 ))
}

# room_for_backup: false (with the reason on stdout) when the disk is too full for one more
room_for_backup() {
  local free need
  free=$(free_kb "$BACKUP_DIR")
  need=$(need_kb)
  if (( free < need )); then
    echo "only $(( free / 1024 )) MB free on the backup disk, a backup needs $(( need / 1024 )) MB (twice the last one)"
    return 1
  fi
}

# take NAME DIR: a backup of every database into DIR
take() {
  local name=$1 tmp=$2 h2 db
  mkdir -p "$tmp"
  for db in "${H2_DBS[@]}"; do
    [[ -e $DB_DIR/$db.mv.db ]] || continue
    h2=${h2:-$(h2_jar)}
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
    # a compact copy: the free pages a busy database keeps (a third of solo.db on 2026-11-20) stay behind
    sqlite3 -cmd ".timeout 30000" "$DB_DIR/$db" "VACUUM INTO '$tmp/$db'"
    # one database at a time, so the disk never holds more than one uncompressed copy
    [[ " $BACKUP_RAW " == *" $db "* ]] || gzip -1 --rsyncable "$tmp/$db"
  done
  if [[ -d $DB_DIR/$PACKS ]]; then
    tar -czf "$tmp/$PACKS.tar.gz" -C "$DB_DIR" "$PACKS"
  fi
}

# one run at a time: the nightly one and a deploy's snapshot share the disk
lock() {
  { exec 9> "$BACKUP_DIR/.lock"; } 2> /dev/null || return 0
  if command -v flock > /dev/null; then flock 9; fi
}

# leftovers of interrupted runs (none can be running: we hold the lock)
clean_partials() {
  find "$BACKUP_DIR" -maxdepth 1 -name '.partial-*' -exec rm -rf {} +
}

# new_name: a timestamp no backup has yet
new_name() {
  local name
  name=$(date -u +%Y%m%dT%H%M%SZ)
  while [[ -e $BACKUP_DIR/$name ]]; do
    sleep 1
    name=$(date -u +%Y%m%dT%H%M%SZ)
  done
  echo "$name"
}

backup_once() {
  local name tmp size reason
  mkdir -p "$BACKUP_DIR"
  lock
  name=$(new_name)
  tmp=$BACKUP_DIR/.partial-$name
  clean_partials
  # room first: a skipped run must still let old backups go
  prune
  if ! reason=$(room_for_backup); then
    log "SKIPPED backup $name: $reason. Free some disk (old backups, deploy snapshots, offsite packs, the picture cache)."
    write_status skipped "$name" "" "$reason"
    return 3
  fi
  trap 'rm -rf "$tmp"; write_status failed "$name"' ERR
  take "$name" "$tmp"
  mv "$tmp" "$BACKUP_DIR/$name"
  size=$(du -sh "$BACKUP_DIR/$name" | cut -f1)
  log "backup $name done ($size)"
  trap - ERR
  write_status ok "$name" "$size"
  prune
}

# stream: a fresh backup as a tar on stdout, nothing kept here and no status written (the deploy's snapshot keeps it)
STREAM_TMP=
stream() {
  local name reason
  mkdir -p "$BACKUP_DIR"
  lock
  name=$(new_name)
  clean_partials
  if ! reason=$(room_for_backup); then
    log "SKIPPED snapshot: $reason" >&2
    return 3
  fi
  STREAM_TMP=$BACKUP_DIR/.partial-stream-$name
  trap 'rm -rf "$STREAM_TMP"' EXIT
  take "$name" "$STREAM_TMP/$name" >&2
  tar -cf - -C "$STREAM_TMP" "$name"
}

prune() {
  local old names name total=0 kb i
  [[ $BACKUP_KEEP =~ ^[1-9][0-9]*$ ]] || { echo "xmage-backup: BACKUP_KEEP must be 1 or more" >&2; return 1; }
  [[ $BACKUP_MAX_GB =~ ^[0-9]+$ ]] || { echo "xmage-backup: BACKUP_MAX_GB must be a whole number (0: no budget)" >&2; return 1; }
  local max_kb=$(( ${BACKUP_MAX_MB:-$(( BACKUP_MAX_GB * 1024 ))} * 1024 ))
  mapfile -t old < <(list_names | head -n "-$BACKUP_KEEP")
  for name in "${old[@]}"; do
    rm -rf "${BACKUP_DIR:?}/$name"
    log "removed old backup $name"
  done
  # the byte budget, newest first; the newest one always stays
  (( max_kb > 0 )) || return 0
  mapfile -t names < <(list_names | sort -r)
  for (( i = 0; i < ${#names[@]}; i++ )); do
    kb=$(du -sk "$BACKUP_DIR/${names[i]}" | cut -f1)
    total=$(( total + kb ))
    if (( i > 0 && total > max_kb )); then
      rm -rf "${BACKUP_DIR:?}/${names[i]}"
      log "removed backup ${names[i]} (the backups pass BACKUP_MAX_GB=$BACKUP_MAX_GB)"
    fi
  done
}

list_names() {
  find "$BACKUP_DIR" -mindepth 1 -maxdepth 1 -type d -name '2*' -printf '%f\n' | sort
}

restore() {
  local name=${1:?usage: xmage-backup restore NAME}
  local src=$BACKUP_DIR/$name h2
  [[ -d $src ]] || { echo "xmage-backup: no backup named $name (see: xmage-backup list)" >&2; exit 1; }
  if (exec 3<>"/dev/tcp/$SERVER_HOST/17172") 2>/dev/null; then
    echo "xmage-backup: the xmage server is running; stop it first: docker compose stop xmage" >&2
    exit 1
  fi
  for db in "${H2_DBS[@]}"; do
    [[ -f $src/$db.sql.gz ]] || continue
    h2=${h2:-$(h2_jar)}
    rm -f "$DB_DIR/$db".*.db
    gunzip -c "$src/$db.sql.gz" > "/tmp/$db.sql"
    java -Xmx256m -cp "$h2" org.h2.tools.RunScript \
      -url "jdbc:h2:file:$DB_DIR/$db" -user "" -password "" -script "/tmp/$db.sql"
    rm -f "/tmp/$db.sql"
    log "restored $db"
  done
  # a backup from before the accounts moved to SQLite: the server copies them again from the H2 just restored
  if [[ -f $src/authorized_user.h2.sql.gz && ! -f $src/authorized_user.db && -e $DB_DIR/authorized_user.db ]]; then
    mv "$DB_DIR/authorized_user.db" "$DB_DIR/authorized_user.db.before-restore-$(date +%s)"
    log "set the SQLite accounts aside: the server copies the restored H2 ones at start"
  fi
  for db in "${SQLITE_DBS[@]}"; do
    [[ -f $src/$db || -f $src/$db.gz ]] || continue
    rm -f "$DB_DIR/$db" "$DB_DIR/$db-journal" "$DB_DIR/$db-wal" "$DB_DIR/$db-shm"
    # gzipped since Nov 2026; older backups hold the plain file
    if [[ -f $src/$db.gz ]]; then gunzip -c "$src/$db.gz" > "$DB_DIR/$db"; else cp "$src/$db" "$DB_DIR/$db"; fi
    log "restored $db"
  done
  if [[ -f $src/$PACKS.tar.gz ]]; then
    rm -rf "${DB_DIR:?}/$PACKS"
    tar -xzf "$src/$PACKS.tar.gz" -C "$DB_DIR"
    log "restored $PACKS"
  fi
  log "restore of $name complete; start the server: docker compose start xmage"
}

daemon() {
  [[ $BACKUP_AT =~ ^[0-2][0-9]:[0-5][0-9]$ ]] || { echo "xmage-backup: BACKUP_AT must be HH:MM" >&2; exit 1; }
  trap 'exit 0' TERM INT
  log "daily backups at $BACKUP_AT ($(date +%Z)), keeping $BACKUP_KEEP within $BACKUP_MAX_GB GB"
  while true; do
    local now next
    now=$(date +%s)
    next=$(date -d "today $BACKUP_AT" +%s)
    (( next > now )) || next=$(date -d "tomorrow $BACKUP_AT" +%s)
    sleep $(( next - now )) & wait $!
    # separate process, so set -e still stops a failing backup half way
    "$0" once || log "backup failed or skipped; retrying tomorrow"
  done
}

case "${1:-daemon}" in
  daemon) daemon ;;
  once) backup_once ;;
  stream) stream ;;
  list) list_names ;;
  restore) restore "${2:-}" ;;
  *) sed -n '2,9p' "$0" >&2; exit 2 ;;
esac
