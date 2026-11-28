#!/bin/bash
# server-entrypoint.sh against a throwaway XMAGE_HOME and stub xmlstarlet/java: checks strict set -eu paths
# (including XMAGE_OIDC_ONLY=true, #446) and JVM property rendering without starting Java. Needs Linux (/proc/meminfo).
#   ops/web/test-entrypoint.sh
set -uo pipefail

here=$(cd "$(dirname "$0")" && pwd)
script=$here/server-entrypoint.sh
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
failed=0

fail() { echo "FAIL $*"; failed=$((failed + 1)); }
ok() { echo "ok   $*"; }

app=$work/app
bin=$work/bin
mkdir -p "$app/config" "$app/db" "$app/cards-db" "$app/cards-prebuilt" "$app/saved/replays" "$app/logs" "$bin"
echo "test-build-1" > "$app/.build-id"
: > "$app/cards-prebuilt/cards.h2.mv.db"
cat > "$app/config/config.template.xml" <<'EOF'
<config><server serverName="mage-server"/><playerTypes/></config>
EOF

cat > "$bin/xmlstarlet" <<'EOF'
#!/bin/sh
if [ "${1:-}" = sel ]; then echo 1; fi
exit 0
EOF
cat > "$bin/java" <<'EOF'
#!/bin/sh
argfile=${1#@}
cat "$argfile" > "$CAPTURED_JVM_ARGS"
exit 0
EOF
chmod +x "$bin/xmlstarlet" "$bin/java"

# run_entry NAME [ENV...]: runs server-entrypoint.sh with stubbed tools; redirects to a file so the backgrounded
# argfile-cleanup sleep never holds a command-substitution pipe open
run_entry() {
  local label=$1
  shift
  local log=$work/$label.log args=$work/$label.args
  rm -f "$log" "$args"
  env -i PATH="$bin:/usr/bin:/bin" XMAGE_HOME="$app" CAPTURED_JVM_ARGS="$args" "$@" \
    sh "$script" > "$log" 2>&1
}

# --- #446: XMAGE_OIDC_ONLY=true must not crash on unset $DESKTOP under set -eu
if run_entry oidc \
  XMAGE_AUTH=true \
  XMAGE_OIDC_ONLY=true \
  XMAGE_OIDC_DISCOVERY=https://idp.example/.well-known/openid-configuration \
  XMAGE_OIDC_CLIENT_ID=playmat \
  XMAGE_OIDC_CLIENT_SECRET=secret \
  XMAGE_PUBLIC_URL=https://play.example.com; then
  grep -qx -- '-Dxmage.oidc.only=true' "$work/oidc.args" \
    && ok "XMAGE_OIDC_ONLY=true starts cleanly and passes -Dxmage.oidc.only=true" \
    || fail "oidc args missing -Dxmage.oidc.only=true: $(cat "$work/oidc.args" 2>/dev/null)"
else
  fail "XMAGE_OIDC_ONLY=true crashed: $(cat "$work/oidc.log")"
fi

# --- XMAGE_OIDC_ONLY=true with legacy XMAGE_DESKTOP=true still starts (and notes desktop is ignored)
if run_entry oidc-desktop \
  XMAGE_AUTH=true \
  XMAGE_OIDC_ONLY=true \
  XMAGE_DESKTOP=true \
  XMAGE_OIDC_DISCOVERY=https://idp.example/.well-known/openid-configuration \
  XMAGE_OIDC_CLIENT_ID=playmat \
  XMAGE_OIDC_CLIENT_SECRET=secret \
  XMAGE_PUBLIC_URL=https://play.example.com; then
  grep -q "XMAGE_DESKTOP=true is ignored" "$work/oidc-desktop.log" \
    && ok "XMAGE_OIDC_ONLY=true with XMAGE_DESKTOP=true starts and warns desktop is ignored" \
    || fail "missing desktop ignored note: $(cat "$work/oidc-desktop.log")"
else
  fail "XMAGE_OIDC_ONLY=true with XMAGE_DESKTOP=true crashed: $(cat "$work/oidc-desktop.log")"
fi

# --- XMAGE_DISK_MIN_FREE_MB default and override
if run_entry disk-default; then
  grep -qx -- '-Dxmage.disk.minFreeMB=3072' "$work/disk-default.args" \
    && ok "XMAGE_DISK_MIN_FREE_MB defaults to 3072" \
    || fail "disk default missing: $(cat "$work/disk-default.args" 2>/dev/null)"
else
  fail "default start failed: $(cat "$work/disk-default.log")"
fi

if run_entry disk-custom XMAGE_DISK_MIN_FREE_MB=2048; then
  grep -qx -- '-Dxmage.disk.minFreeMB=2048' "$work/disk-custom.args" \
    && ok "XMAGE_DISK_MIN_FREE_MB override reaches JVM" \
    || fail "disk override missing: $(cat "$work/disk-custom.args" 2>/dev/null)"
else
  fail "disk override start failed: $(cat "$work/disk-custom.log")"
fi

if run_entry disk-bad XMAGE_DISK_MIN_FREE_MB=nope; then
  fail "non-numeric XMAGE_DISK_MIN_FREE_MB should be rejected"
else
  grep -q "XMAGE_DISK_MIN_FREE_MB must be a whole number" "$work/disk-bad.log" \
    && ok "non-numeric XMAGE_DISK_MIN_FREE_MB rejected" \
    || fail "unexpected error: $(cat "$work/disk-bad.log")"
fi

if command -v shellcheck > /dev/null; then
  shellcheck -S warning "$script" "$0" || fail "shellcheck"
fi
(( failed == 0 )) && echo "all passed" || { echo "$failed failed"; exit 1; }
