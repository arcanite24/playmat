#!/bin/bash
# The playmat-rl account's forced command (rl-setup.sh, authorized_keys `restrict,command=...`): the only way the
# trainer (rust/rl/remote.py, on the trainer's machine; this server never connects to it) reaches the self-play
# spool. No shell, no files outside the spool.
#
#   put-job ID     a tar on stdin with job.json, net.pmnet and rollout.pmnet: becomes the current job
#   stop           no current job (the worker idles)
#   list           every finished chunk's .json, one per line
#   get JOB CHUNK  the chunk's .tar.gz on stdout
#   ack JOB CHUNK  deletes the chunk (after the trainer checked its copy)
#   status         the worker's last state, the current job and the spool's size
set -euo pipefail
SPOOL=/srv/playmat-rl
IMAGE_RE='^ghcr\.io/arcanite24/playmat-selfplay(:[A-Za-z0-9._-]+)?@sha256:[0-9a-f]{64}$'
read -r -a cmd <<< "${SSH_ORIGINAL_COMMAND:-}"
die() { echo "rl-transfer: $*" >&2; exit 2; }
# a job or chunk name: starts with a letter or digit, no '..'
name() { [[ ${1:-} =~ ^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$ && ${1:-} != *..* ]] || die "bad name '${1:-}'"; }

case ${cmd[0]:-} in
  put-job)
    id=${cmd[1]:-}; name "$id"
    tmp=$SPOOL/jobs/.$id.tmp
    rm -rf "$tmp"; mkdir -p "$tmp"
    # 64 MB at most; only these three plain files
    head -c 67108864 | tar -x -C "$tmp" --no-same-owner --no-same-permissions --no-overwrite-dir \
      job.json net.pmnet rollout.pmnet || { rm -rf "$tmp"; die "the tar must hold job.json, net.pmnet and rollout.pmnet"; }
    for f in job.json net.pmnet rollout.pmnet; do
      [[ -f $tmp/$f && ! -L $tmp/$f ]] || { rm -rf "$tmp"; die "$f is missing or not a file"; }
    done
    python3 - "$tmp/job.json" "$id" "$IMAGE_RE" << 'EOF' || { rm -rf "$tmp"; die "invalid job.json"; }
import json, re, sys
j = json.load(open(sys.argv[1]))
assert j.get("id") == sys.argv[2]
assert re.fullmatch(sys.argv[3], str(j.get("image", "")))
assert re.fullmatch(r"cfg:[A-Za-z0-9_=+./-]{1,1000}", str(j.get("learner", "")))
assert "/job/net.pmnet" in j["learner"]
assert str(j.get("fast", "")) in ("", "net", "quick")
assert isinstance(j.get("keep"), int) and 1 <= j["keep"] <= 1000
assert isinstance(j.get("seed_base"), int) and 0 <= j["seed_base"] < 2**31 - 2**21
assert isinstance(j.get("tag"), int)
EOF
    chmod 640 "$tmp"/*
    rm -rf "${SPOOL:?}/jobs/$id"
    mv "$tmp" "$SPOOL/jobs/$id"
    mkdir -p "$SPOOL/job"
    ln -sfn "../jobs/$id" "$SPOOL/job/.current"
    mv -T "$SPOOL/job/.current" "$SPOOL/job/current"
    # older jobs' files go; their finished chunks stay until pulled (or 3 days)
    find "$SPOOL/jobs" -mindepth 1 -maxdepth 1 -type d ! -name "$id" -exec rm -rf {} +
    find "$SPOOL/out" -type f -mtime +3 -delete 2>/dev/null || true
    echo "job $id is current"
    ;;
  stop)
    rm -f "$SPOOL/job/current"
    echo "no current job"
    ;;
  list)
    # shellcheck disable=SC2016 # $0 is sh -c's argument
    find "$SPOOL/out" -mindepth 2 -maxdepth 2 -name 'c*.json' -print0 2>/dev/null | sort -z | xargs -0 -r -n 1 sh -c 'cat "$0"; echo'
    ;;
  get)
    name "${cmd[1]:-}"; name "${cmd[2]:-}"
    f=$SPOOL/out/${cmd[1]}/${cmd[2]}.tar.gz
    [[ -f $f && -f ${f%.tar.gz}.json ]] || die "no chunk ${cmd[1]}/${cmd[2]}"
    cat "$f"
    ;;
  ack)
    name "${cmd[1]:-}"; name "${cmd[2]:-}"
    rm -f "$SPOOL/out/${cmd[1]}/${cmd[2]}.json" "$SPOOL/out/${cmd[1]}/${cmd[2]}.tar.gz"
    echo "acked ${cmd[1]}/${cmd[2]}"
    ;;
  status)
    cat "$SPOOL/status.json" 2>/dev/null || echo '{"state": "never ran"}'
    echo "job: $(basename "$(readlink "$SPOOL/job/current" 2>/dev/null || echo none)")"
    echo "spool: $(du -sm "$SPOOL/out" 2>/dev/null | cut -f1) MB in $(find "$SPOOL/out" -name 'c*.json' 2>/dev/null | wc -l) chunks"
    ;;
  *)
    die "commands: put-job ID, stop, list, get JOB CHUNK, ack JOB CHUNK, status"
    ;;
esac
