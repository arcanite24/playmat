"""The trainer's side of self-play on the public server (ops/web/host/rl-worker.sh and rl-transfer.sh): sends a
round's job and pulls the chunks the server played while it was idle. The trainer always connects; the server never
does. `HOST` is an ssh alias for the server's playmat-rl account.

    python remote.py HOST status
    python remote.py HOST list
    python remote.py HOST stop

`progress` sends the trainer's state to the admin console's Self-play section (rounds.py calls it at each phase).
"""

import hashlib
import io
import json
import os
import shutil
import subprocess
import sys
import tarfile
import tempfile


# Windows' own ssh.exe sends no stdin when it runs under an SSH session (a run started over ssh, Start-Process
# included), so put-job would hang: Git for Windows' ssh doesn't have the bug. PLAYMAT_RL_SSH names another client.
GIT_SSH = r"C:\Program Files\Git\usr\bin\ssh.exe"
SSH = os.environ.get("PLAYMAT_RL_SSH") or (GIT_SSH if os.name == "nt" and os.path.exists(GIT_SSH) else "ssh")


def ssh(host, cmd, data=None, timeout=600):
    p = subprocess.run([SSH, "-o", "BatchMode=yes", "-o", "ConnectTimeout=20", host, cmd], input=data,
                       capture_output=True, timeout=timeout)
    if p.returncode != 0:
        raise RuntimeError(f"ssh {host} {cmd}: {p.stderr.decode(errors='replace').strip()}")
    return p.stdout


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for b in iter(lambda: f.read(1 << 20), b""):
            h.update(b)
    return h.hexdigest()


def put_job(host, job_id, image, learner, net, rollout, keep, fast, seed_base, tag):
    """Makes `job_id` the server's current job. `learner` names the networks as /job/net.pmnet and
    /job/rollout.pmnet (`rollout` is sent either way; a learner without one ignores it)."""
    job = {"id": job_id, "image": image, "learner": learner, "keep": keep, "fast": fast or "", "seed_base": seed_base,
           "tag": tag, "net_sha256": sha256(net), "rollout_sha256": sha256(rollout)}
    buf = io.BytesIO()
    with tarfile.open(fileobj=buf, mode="w") as t:
        raw = json.dumps(job).encode()
        info = tarfile.TarInfo("job.json")
        info.size = len(raw)
        t.addfile(info, io.BytesIO(raw))
        t.add(net, arcname="net.pmnet")
        t.add(rollout, arcname="rollout.pmnet")
    return ssh(host, f"put-job {job_id}", buf.getvalue()).decode().strip(), job


def progress(host, state):
    """The trainer's progress (a dict, under 64 KB as JSON) for the admin console; the worker copies it there."""
    raw = json.dumps(state).encode()
    if len(raw) > 65536:
        raise ValueError(f"progress is {len(raw)} bytes, over 64 KB")
    return ssh(host, "progress", raw, timeout=60).decode().strip()


def chunks(host):
    out = ssh(host, "list").decode()
    return [json.loads(line) for line in out.splitlines() if line.strip()]


def pull(host, job, dest):
    """Every finished chunk of `job` (put_job's dict) into `dest`, its files prefixed with the chunk's name, each
    checked (its sha256, the image and the networks it was played with) and then deleted on the server. Chunks of
    other jobs are deleted unread: they were played with another network. -> (chunks, games, bytes, rejected)"""
    os.makedirs(dest, exist_ok=True)
    n = games = size = 0
    rejected = []
    for c in chunks(host):
        if c.get("job") != job["id"]:
            ssh(host, f"ack {c['job']} {c['chunk']}")
            continue
        why = None
        if c.get("image") != job["image"]:
            why = f"image {c.get('image')}"
        elif c.get("net_sha256") != job["net_sha256"] or c.get("rollout_sha256") != job["rollout_sha256"]:
            why = "other networks"
        if why is None:
            raw = ssh(host, f"get {c['job']} {c['chunk']}", timeout=1800)
            if hashlib.sha256(raw).hexdigest() != c.get("sha256"):
                why = "sha256 mismatch"
        if why is not None:
            rejected.append(f"{c['chunk']}: {why}")
            ssh(host, f"ack {c['job']} {c['chunk']}")
            continue
        tmp = tempfile.mkdtemp(dir=dest)
        try:
            with tarfile.open(fileobj=io.BytesIO(raw), mode="r:gz") as t:
                for m in t.getmembers():
                    name = os.path.basename(m.name)
                    if not m.isfile() or not name or name != m.name.lstrip("./"):
                        continue
                    with t.extractfile(m) as src, open(os.path.join(tmp, name), "wb") as dst:
                        shutil.copyfileobj(src, dst)
            for name in os.listdir(tmp):
                os.replace(os.path.join(tmp, name), os.path.join(dest, f"h{c['chunk']}-{name}"))
        finally:
            shutil.rmtree(tmp, ignore_errors=True)
        ssh(host, f"ack {c['job']} {c['chunk']}")
        n += 1
        games += int(c.get("games", 0))
        size += len(raw)
    return n, games, size, rejected


if __name__ == "__main__":
    if len(sys.argv) < 3 or sys.argv[2] not in ("status", "list", "stop"):
        sys.exit(__doc__)
    print(ssh(sys.argv[1], sys.argv[2]).decode(), end="")
