#!/bin/bash
# Off-site copies of the nightly backups, run on the host by playmat-offsite.timer after the backup service's run.
#
#   playmat-offsite pack      pack the newest backup into $OUT_DIR as a .tar.gz (keeps the newest $KEEP_PACKS),
#                             then push it to the restic repository when one is configured. Skipped, loudly (exit 3),
#                             when $OUT_DIR's disk has less than twice the backup's size free
#   playmat-offsite export    write the newest pack to stdout: the forced command of a pull key, so another machine
#                             (Multivac) can fetch it with `ssh <alias> > file` and can't do anything else
#   playmat-offsite list      list packs
#
# A pull from elsewhere keeps the copies out of reach of anyone who takes over this server. A restic repository
# (Cloudflare R2, Backblaze B2, a Hetzner Storage Box over SFTP...) adds an encrypted push copy: put RESTIC_REPOSITORY,
# RESTIC_PASSWORD and the provider's keys (AWS_ACCESS_KEY_ID / AWS_SECRET_ACCESS_KEY for S3-compatible stores) in
# /etc/playmat/offsite.env, mode 600.
#
# Settings from /etc/playmat/host.env: BACKUP_VOLUME (default xmage_xmage-backups), OUT_DIR (/var/backups/playmat/offsite),
# KEEP_PACKS (1: the pack is only the hand-over to a puller, which keeps its own), RESTIC_KEEP_DAILY (30),
# RESTIC_KEEP_MONTHLY (12). Tests: BACKUP_SRC (the backup volume's folder) and OFFSITE_FREE_KB (pretend free space).
set -euo pipefail

[[ -r /etc/playmat/host.env ]] && . /etc/playmat/host.env
BACKUP_VOLUME=${BACKUP_VOLUME:-xmage_xmage-backups}
OUT_DIR=${OUT_DIR:-/var/backups/playmat/offsite}
KEEP_PACKS=${KEEP_PACKS:-1}

log() { logger -t playmat-offsite -- "$*" 2>/dev/null || true; echo "$*" >&2; }

newest_pack() { find "$OUT_DIR" -maxdepth 1 -name 'playmat-*.tar.gz' 2>/dev/null | sort | tail -1; }

free_kb() {
  if [[ -n ${OFFSITE_FREE_KB:-} ]]; then echo "$OFFSITE_FREE_KB"; return; fi
  df -Pk "$1" | awk 'NR == 2 { print $4 }'
}

pack() {
  local src newest out need free
  src=${BACKUP_SRC:-$(docker volume inspect -f '{{.Mountpoint}}' "$BACKUP_VOLUME")}
  newest=$(find "$src" -mindepth 1 -maxdepth 1 -type d -name '2*' -printf '%f\n' | sort | tail -1)
  [[ -n $newest ]] || { log "no backup in $BACKUP_VOLUME yet"; exit 1; }
  mkdir -p "$OUT_DIR"
  chmod 700 "$OUT_DIR"
  # leftovers of a pack that failed half way
  find "$OUT_DIR" -maxdepth 1 -name 'playmat-*.tar.gz.tmp' -delete
  out=$OUT_DIR/playmat-$newest.tar.gz
  if [[ ! -e $out ]]; then
    # old packs go first, leaving room for this one
    find "$OUT_DIR" -maxdepth 1 -name 'playmat-*.tar.gz' | sort | head -n -"$(( KEEP_PACKS > 1 ? KEEP_PACKS - 1 : 1 ))" | xargs -r rm -f
    need=$(( $(du -sk "$src/$newest" | cut -f1) * 2 ))
    free=$(free_kb "$OUT_DIR")
    if (( free < need )); then
      log "SKIPPED pack of $newest: only $(( free / 1024 )) MB free in $OUT_DIR, a pack needs $(( need / 1024 )) MB (twice the backup). Free some disk."
      exit 3
    fi
    trap 'rm -f "$out.tmp"' EXIT
    # the databases are mostly gzipped already: the fastest level
    tar -C "$src" -cf - "$newest" | gzip -1 > "$out.tmp"
    mv "$out.tmp" "$out"
    trap - EXIT
    log "packed $newest ($(du -h "$out" | cut -f1))"
  fi
  find "$OUT_DIR" -maxdepth 1 -name 'playmat-*.tar.gz' | sort | head -n -"$KEEP_PACKS" | xargs -r rm -f

  if [[ -r /etc/playmat/offsite.env ]]; then
    set -a
    . /etc/playmat/offsite.env
    set +a
    if [[ -n ${RESTIC_REPOSITORY:-} ]]; then
      restic snapshots --quiet >/dev/null 2>&1 || restic init
      restic backup --quiet --tag playmat --host playmat "$src/$newest"
      restic forget --quiet --prune --tag playmat --keep-daily "${RESTIC_KEEP_DAILY:-30}" --keep-monthly "${RESTIC_KEEP_MONTHLY:-12}"
      log "pushed $newest to the restic repository"
    fi
  fi
}

case ${1:-pack} in
  pack) pack ;;
  export)
    f=$(newest_pack)
    [[ -n $f ]] || { echo "no pack yet" >&2; exit 1; }
    log "exporting $(basename "$f")"
    cat "$f"
    ;;
  list) ls -lh "$OUT_DIR" ;;
  *) echo "usage: playmat-offsite pack|export|list" >&2; exit 2 ;;
esac
