#!/bin/bash
# offsite.sh pack against a throwaway backup folder: the free-space guard, leftovers, how many packs stay,
# and backup volume resolution. Needs Linux (GNU find/head); no Docker.
#   ops/web/host/test-offsite.sh
set -uo pipefail

here=$(cd "$(dirname "$0")" && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
failed=0
fail() { echo "FAIL $*"; failed=$((failed + 1)); }
ok() { echo "ok   $*"; }

export BACKUP_SRC=$work/backups OUT_DIR=$work/offsite
mkdir -p "$BACKUP_SRC/20261118T033000Z" "$BACKUP_SRC/.partial-20261119T033000Z"
head -c 300000 /dev/urandom > "$BACKUP_SRC/20261118T033000Z/solo.db"
pack() { bash "$here/offsite.sh" pack 2>&1; }

out=$(pack) || fail "pack: $out"
[[ -f $OUT_DIR/playmat-20261118T033000Z.tar.gz ]] && ok "packed the newest backup, not a partial" || fail "packs: $(ls "$OUT_DIR")"

mkdir -p "$BACKUP_SRC/20261119T033000Z"
head -c 300000 /dev/urandom > "$BACKUP_SRC/20261119T033000Z/solo.db"
touch "$OUT_DIR/playmat-20261117T033000Z.tar.gz.tmp"
out=$(OFFSITE_FREE_KB=100 pack); code=$?
[[ $code == 3 && $out == *"SKIPPED pack of 20261119T033000Z"* ]] && ok "a full disk skips the pack, loudly (exit 3)" || fail "guard: exit $code: $out"
[[ ! -e $OUT_DIR/playmat-20261119T033000Z.tar.gz && ! -e $OUT_DIR/playmat-20261119T033000Z.tar.gz.tmp ]] \
  && ok "and writes nothing" || fail "written anyway: $(ls "$OUT_DIR")"
[[ ! -e $OUT_DIR/playmat-20261117T033000Z.tar.gz.tmp ]] && ok "leftover .tmp removed" || fail "tmp left"

out=$(pack) || fail "pack: $out"
[[ $(find "$OUT_DIR" -name 'playmat-*.tar.gz' | wc -l) == 1 && -f $OUT_DIR/playmat-20261119T033000Z.tar.gz ]] \
  && ok "KEEP_PACKS=1 keeps only the newest" || fail "packs: $(ls "$OUT_DIR")"
tar -tzf "$OUT_DIR/playmat-20261119T033000Z.tar.gz" | grep -q '20261119T033000Z/solo.db' && ok "the pack holds the backup" || fail "pack contents"

# --- volume resolution: explicit > docker compose label > /etc/playmat-deploy.conf > web_xmage-backups
stub_bin=$work/bin
mkdir -p "$stub_bin"
cat > "$stub_bin/docker" <<'EOF'
#!/bin/bash
printf '%s\n' "${STUB_DOCKER_VOL:-}"
EOF
chmod +x "$stub_bin/docker"
cat > "$work/deploy.conf" <<'EOF'
COMPOSE_PROJECT=customproj
EOF
vol=$(env PATH="$stub_bin:/usr/bin:/bin" STUB_DOCKER_VOL=label_vol BACKUP_VOLUME=explicit_vol PLAYMAT_DEPLOY_CONF="$work/deploy.conf" \
  bash "$here/offsite.sh" resolve-volume)
[[ $vol == explicit_vol ]] && ok "explicit BACKUP_VOLUME wins" || fail "explicit volume: $vol"
vol=$(env PATH="$stub_bin:/usr/bin:/bin" STUB_DOCKER_VOL=label_vol BACKUP_VOLUME= PLAYMAT_DEPLOY_CONF="$work/deploy.conf" \
  bash "$here/offsite.sh" resolve-volume)
[[ $vol == label_vol ]] && ok "volume discovered from docker compose label" || fail "label volume: $vol"
vol=$(env PATH="$stub_bin:/usr/bin:/bin" STUB_DOCKER_VOL= BACKUP_VOLUME= PLAYMAT_DEPLOY_CONF="$work/deploy.conf" \
  bash "$here/offsite.sh" resolve-volume)
[[ $vol == customproj_xmage-backups ]] && ok "volume falls back to COMPOSE_PROJECT from deploy conf" || fail "conf volume: $vol"
vol=$(env PATH="$stub_bin:/usr/bin:/bin" STUB_DOCKER_VOL= BACKUP_VOLUME= PLAYMAT_DEPLOY_CONF="$work/missing.conf" \
  bash "$here/offsite.sh" resolve-volume)
[[ $vol == web_xmage-backups ]] && ok "volume defaults to web_xmage-backups" || fail "default volume: $vol"

if command -v shellcheck > /dev/null; then
  shellcheck -S warning "$here/offsite.sh" "$0" || fail "shellcheck"
fi
(( failed == 0 )) && echo "all passed" || { echo "$failed failed"; exit 1; }
