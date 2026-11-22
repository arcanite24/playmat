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
# Settings from /etc/playmat/host.env: BACKUP_VOLUME (default: discovered by compose label com.docker.compose.volume=xmage-backups,
# else <COMPOSE_PROJECT>_xmage-backups from /etc/playmat-deploy.conf, else web_xmage-backups),
# OUT_DIR (/var/backups/playmat/offsite), KEEP_PACKS (1: the pack is only the hand-over to a puller, which keeps its own),
# RESTIC_KEEP_DAILY (30), RESTIC_KEEP_MONTHLY (12).
# Tests: BACKUP_SRC (the backup volume's folder), OFFSITE_FREE_KB (pretend free space), PLAYMAT_DEPLOY_CONF.
set -euo pipefail

# shellcheck source=/dev/null
[[ -r /etc/playmat/host.env ]] && . /etc/playmat/host.env
BACKUP_VOLUME=${BACKUP_VOLUME:-}
OUT_DIR=${OUT_DIR:-/var/backups/playmat/offsite}
KEEP_PACKS=${KEEP_PACKS:-1}
DEPLOY_CONF=${PLAYMAT_DEPLOY_CONF:-/etc/playmat-deploy.conf}

log() { logger -t playmat-offsite -- "$*" 2>/dev/null || true; echo "$*" >&2; }

newest_pack() { find "$OUT_DIR" -maxdepth 1 -name 'playmat-*.tar.gz' 2>/dev/null | sort | tail -1; }

resolve_backup_volume() {
  if [[ -n $BACKUP_VOLUME ]]; then
    echo "$BACKUP_VOLUME"
    return
  fi
  local vol project
  if command -v docker > /dev/null 2>&1; then
    vol=$(docker volume ls -q --filter label=com.docker.compose.volume=xmage-backups 2>/dev/null | head -n 1 || true)
    if [[ -n $vol ]]; then
      echo "$vol"
      return
    fi
  fi
  if [[ -r $DEPLOY_CONF ]]; then
    project=$(sed -n 's/^COMPOSE_PROJECT=//p' "$DEPLOY_CONF" | tail -n 1)
    if [[ -n $project ]]; then
      echo "${project}_xmage-backups"
      return
    fi
  fi
  echo "web_xmage-backups"
}

free_kb() {
  if [[ -n ${OFFSITE_FREE_KB:-} ]]; then echo "$OFFSITE_FREE_KB"; return; fi
  df -Pk "$1" | awk 'NR == 2 { print $4 }'
}

pack() {
  local vol src newest out need free
  if [[ -n ${BACKUP_SRC:-} ]]; then
    src=$BACKUP_SRC
    vol=$BACKUP_SRC
  else
    vol=$(resolve_backup_volume)
    src=$(docker volume inspect -f '{{.Mountpoint}}' "$vol")
  fi
  newest=$(find "$src" -mindepth 1 -maxdepth 1 -type d -name '2*' -printf '%f\n' | sort | tail -1)
  [[ -n $newest ]] || { log "no backup in $vol yet"; exit 1; }
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
    tar -C "$src" -cf - "$newest" | gzip -1 > "$out.tmp"
    mv "$out.tmp" "$out"
    trap - EXIT
    log "packed $newest ($(du -h "$out" | cut -f1))"
  fi
  find "$OUT_DIR" -maxdepth 1 -name 'playmat-*.tar.gz' | sort | head -n -"$KEEP_PACKS" | xargs -r rm -f

  if [[ -r /etc/playmat/offsite.env ]]; then
    set -a
    # shellcheck source=/dev/null
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
  resolve-volume) resolve_backup_volume ;;
  *) echo "usage: playmat-offsite pack|export|list" >&2; exit 2 ;;
esac
