#!/bin/bash
# What playmat-deploy accepts and refuses, checked without Docker or a host: PLAYMAT_DEPLOY_CHECK_ONLY=1 makes it stop
# once a command is accepted. Then shellcheck on the deploy scripts, when it is installed.
#   ops/web/deploy/test-deploy.sh
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
script=$here/playmat-deploy
failed=0

# over_ssh EXPECTED COMMAND: what the forced command does with SSH_ORIGINAL_COMMAND=COMMAND
over_ssh() {
  local expected=$1 command=$2 out code=0
  out=$(SSH_ORIGINAL_COMMAND=$command PLAYMAT_DEPLOY_CHECK_ONLY=1 bash "$script" 2>&1) || code=$?
  check "$expected" "$code" "$out" "ssh: $(printf '%q' "$command")"
}

# by_hand EXPECTED ARGS...: run with arguments, as root on the host would
by_hand() {
  local expected=$1 out code=0
  shift
  out=$(env -u SSH_ORIGINAL_COMMAND PLAYMAT_DEPLOY_CHECK_ONLY=1 bash "$script" "$@" 2>&1) || code=$?
  check "$expected" "$code" "$out" "args: $*"
}

# check EXPECTED CODE OUTPUT LABEL: EXPECTED is "refused" (exit 2) or the accepted command
check() {
  if [[ $1 == refused ]]; then
    if [[ $2 == 2 && $3 == *"refused"* ]]; then return; fi
  elif [[ $2 == 0 && $3 == "accepted: $1" ]]; then
    return
  fi
  echo "FAIL $4: expected $1, got exit $2: $3"
  failed=$((failed + 1))
}

hex40=0123456789abcdef0123456789abcdef01234567

over_ssh "deploy v1.2.3" "deploy v1.2.3"
over_ssh "deploy v10.20.30" "deploy v10.20.30"
over_ssh "deploy v1.3.0-rc.1" "deploy v1.3.0-rc.1"
over_ssh "deploy sha-abc1234" "deploy sha-abc1234"
over_ssh "deploy sha-$hex40" "deploy sha-$hex40"
over_ssh "deploy master" "deploy master"
over_ssh "deploy v1.2.3" "  deploy   v1.2.3  "
over_ssh "rollback" "rollback"
over_ssh "status" "status"

over_ssh refused ""
over_ssh refused "deploy"
over_ssh refused "deploy v1.2"
over_ssh refused "deploy 1.2.3"
over_ssh refused "deploy latest"
over_ssh refused "deploy main"
over_ssh refused "deploy master2"
over_ssh refused "deploy v1.2.3-"
over_ssh refused "deploy v1.2.3-rc_1"
over_ssh refused "deploy sha-abc123"
over_ssh refused "deploy sha-ABC1234"
over_ssh refused "deploy sha-${hex40}8"
over_ssh refused "deploy v1.2.3 v1.2.4"
over_ssh refused "deploy ../../etc/passwd"
over_ssh refused "deploy v1.2.3;id"
over_ssh refused "deploy v1.2.3 && reboot"
over_ssh refused "deploy \$(id)"
# shellcheck disable=SC2016 # the backticks are the point
over_ssh refused 'deploy `id`'
over_ssh refused "deploy v1.2.3|sh"
over_ssh refused $'deploy\tv1.2.3'
over_ssh refused $'status\nrollback'
over_ssh refused "rollback now"
over_ssh refused "status --verbose"
over_ssh refused "Deploy v1.2.3"
over_ssh refused "bash"
over_ssh refused "sh -c id"
over_ssh refused "scp -t /tmp"
over_ssh refused "rsync --server -e.LsfxC . /tmp"
over_ssh refused "git-upload-pack /srv/playmat"
over_ssh refused "deploy v$(printf '1%.0s' {1..120}).0.0"

by_hand "deploy v1.2.3" deploy v1.2.3
by_hand "status" status
by_hand refused
by_hand refused deploy "v1.2.3;id"
by_hand refused deploy v1.2.3 extra
# over SSH the arguments never count, only what the client asked for
out=$(SSH_ORIGINAL_COMMAND=status PLAYMAT_DEPLOY_CHECK_ONLY=1 bash "$script" deploy v1.2.3 2>&1) || true
check status 0 "$out" "ssh wins over arguments"

if command -v shellcheck > /dev/null; then
  shellcheck "$here/playmat-deploy" "$here/playmat-compose" "$here/setup-deploy-user.sh" "$here/test-deploy.sh" \
    || { echo "FAIL shellcheck"; failed=$((failed + 1)); }
else
  echo "(shellcheck not installed: skipped)"
fi

if (( failed )); then
  echo "$failed check(s) failed"
  exit 1
fi
echo "all checks passed"
