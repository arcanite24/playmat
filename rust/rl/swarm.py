"""The trainer's side of Mana@Home (docs/swarm/Design.md): publishes a round's work units to the coordinator
(`playmat-swarm`), waits for the swarm to play them, pulls each completed unit, replays it with
`playmat-worker verify` against the unit the coordinator leased, and accepts or rejects it. Accepted shards land in
one folder, each file prefixed with its unit id, ready for `data.py` and `train.py`.

    python swarm.py --coordinator URL --token-file F status
    python swarm.py --coordinator URL --token-file F publish ROUND.json       (a NewRound, net paths allowed)
    python swarm.py --coordinator URL --token-file F pull ROUND_ID OUT [--worker-bin B] [--wait]

`pfsp_league.py --swarm URL` plans a PFSP round, publishes it with these functions and updates the league from the
accepted games. Only the standard library: the trainer needs nothing installed for it.
"""

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", ".."))
PROTOCOL = 1
UNIT_GAMES = 100
# Cloudflare's Browser Integrity Check turns away Python's default User-Agent (error 1010)
UA = "mana-trainer/1 (playmat swarm.py)"


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for b in iter(lambda: f.read(1 << 20), b""):
            h.update(b)
    return h.hexdigest()


class Client:
    def __init__(self, base, token):
        self.base = base.rstrip("/")
        self.token = token

    def call(self, method, path, body=None, timeout=120):
        data = None if body is None else json.dumps(body).encode()
        req = urllib.request.Request(f"{self.base}/api/v1{path}", data=data, method=method, headers={"User-Agent": UA})
        req.add_header("Authorization", f"Bearer {self.token}")
        if data is not None:
            req.add_header("Content-Type", "application/json")
        for attempt in range(5):
            try:
                with urllib.request.urlopen(req, timeout=timeout) as r:
                    raw = r.read()
                    return json.loads(raw) if raw else None
            except urllib.error.HTTPError as e:
                raise RuntimeError(f"{method} {path}: {e.code} {e.read().decode(errors='replace')[:400]}") from None
            except (urllib.error.URLError, TimeoutError, ConnectionError) as e:
                # the network, not the coordinator: a DNS hiccup or a dropped connection
                if attempt == 4:
                    raise
                print(f"SWARM {method} {path}: {e}, again", flush=True)
                time.sleep(10 * (attempt + 1))

    def upload_net(self, path):
        """Uploads a network once (by its sha256) and gives the sha256"""
        sha = sha256_file(path)
        size = os.path.getsize(path)
        url = self.call("POST", f"/nets/{sha}?size={size}")["urls"][f"{sha}.pmnet"]
        with open(path, "rb") as f:
            put(url, f.read())
        return sha

    def net_url(self, sha):
        # the coordinator rewrites it with its public address when the round is published
        return f"{self.base}/api/v1/nets/{sha}"


def put(url, body, tries=4):
    for i in range(tries):
        req = urllib.request.Request(url, data=body, method="PUT", headers={"User-Agent": UA})
        req.add_header("Content-Length", str(len(body)))
        try:
            with urllib.request.urlopen(req, timeout=300):
                return
        except (urllib.error.URLError, TimeoutError) as e:
            if i == tries - 1:
                raise
            print(f"SWARM upload: {e}, again", flush=True)
            time.sleep(5 * (i + 1))


def get(url, timeout=300):
    with urllib.request.urlopen(urllib.request.Request(url, headers={"User-Agent": UA}), timeout=timeout) as r:
        return r.read()


NET_KEY = re.compile(r"(?<=[:+])(net|rollout)=([^+]+)")


def swarm_spec(spec, client, nets):
    """`spec` with every network path swapped for `@<sha256>` (uploading it), and the networks it names in `nets`"""
    def swap(m):
        key, val = m.group(1), m.group(2)
        if val in ("builtin", "-") or val.startswith("@") or val.startswith("tcp://"):
            if val.startswith("tcp://"):
                raise ValueError(f"{spec}: a swarm unit can't use a remote network")
            return m.group(0)
        sha = nets.get(os.path.abspath(val)) or client.upload_net(val)
        nets[os.path.abspath(val)] = sha
        return f"{key}=@{sha}"
    return NET_KEY.sub(swap, spec)


def net_refs(client, learner, opponents, nets):
    """The NetRefs of the networks the specs name, each with the role it first plays"""
    refs = {}
    for sha in sorted(set(nets.values())):
        tag = f"@{sha}"
        if f"rollout={tag}" in learner or any(f"rollout={tag}" in o for o in opponents):
            role = "rollout"
        elif tag in learner:
            role = "learner"
        elif any(tag in o for o in opponents):
            role = "opponent"
        else:
            continue
        refs[sha] = {"role": role, "sha256": sha, "url": client.net_url(sha)}
    return list(refs.values())


def suite_pin(suite_dir):
    """(name, sha256 of suite.json with CRLF read as LF): playmat_swarm_proto::suite_sha256"""
    with open(os.path.join(suite_dir, "suite.json"), "rb") as f:
        lf = f.read().replace(b"\r\n", b"\n")
    return os.path.basename(os.path.normpath(suite_dir)), hashlib.sha256(lf).hexdigest()


def make_units(client, project, run, rnd, learner, opponents, suite_dir, games, seed, keep, fast, nets,
               strict=True, prefix="u", unit_games=UNIT_GAMES):
    """Units of up to `unit_games` games: `learner` against `opponents` ([(spec or "self", weight)]), deals from
    `seed` on. Network paths in the specs are uploaded and named by sha256."""
    learner = swarm_spec(learner, client, nets)
    opps = [{"config": "self" if o == "self" else swarm_spec(o, client, nets), "weight": w} for o, w in opponents]
    refs = net_refs(client, learner, [o["config"] for o in opps], nets)
    suite, suite_sha = suite_pin(suite_dir)
    units = []
    for i, start in enumerate(range(0, games, unit_games)):
        units.append({
            "protocol": PROTOCOL, "id": f"{project}-{run}-r{rnd}-{prefix}{i:03d}", "project": project, "run": run,
            "round": rnd, "kind": "selfplay",
            "pins": {"builds": [], "obs": 3, "suite": suite, "suite_sha256": suite_sha, "strict": strict},
            "nets": refs,
            "learner": learner, "opponents": opps, "seed": seed + start, "games": min(unit_games, games - start),
            "keep": keep, "fast": fast, "deadline": 0, "cost": 1.0,
        })
    return units


def publish(client, project, run, rnd, units, priority=0):
    st = client.call("POST", "/rounds", {"project": project, "run": run, "round": rnd, "priority": priority,
                                         "units": units})
    print(f"SWARM round {rnd} of {project}/{run}: {len(units)} units published (id {st['id']})", flush=True)
    return st


def wait(client, round_id, poll=30, timeout=None):
    """Waits until no unit is queued or leased; gives the last status"""
    t0 = time.time()
    while True:
        st = client.call("GET", f"/rounds/{round_id}")
        u = st["units"]
        print(f"SWARM round {round_id}: {json.dumps(u, sort_keys=True)}, {st['games']} games", flush=True)
        if not u.get("queued") and not u.get("leased"):
            return st
        if timeout and time.time() - t0 > timeout:
            return st
        time.sleep(poll)


def worker_bin():
    ext = ".exe" if os.name == "nt" else ""
    return os.path.join(REPO, "rust", "target", "release", "playmat-worker" + ext)


def pull(client, round_id, out, bin_path=None, repo=REPO):
    """Replays every completed unit of the round and sends its verdict; accepted shards go to `out`, prefixed by
    unit. Gives the accepted units as {unit id: manifest}."""
    bin_path = bin_path or worker_bin()
    os.makedirs(out, exist_ok=True)
    accepted = {}
    for r in client.call("GET", f"/rounds/{round_id}/results?state=complete"):
        tmp = tempfile.mkdtemp(prefix="swarm-check-")
        try:
            with open(os.path.join(tmp, "unit.json"), "w") as f:
                json.dump(r["work"], f)
            with open(os.path.join(tmp, "manifest.json"), "w") as f:
                json.dump(r["manifest"], f)
            for path, url in r["files"].items():
                if "/" in path or "\\" in path or path.startswith("."):
                    raise ValueError(f"{r['unit']}: file name {path}")
                with open(os.path.join(tmp, path), "wb") as f:
                    f.write(get(url))
            p = subprocess.run([bin_path, "verify", tmp, "--repo", repo], capture_output=True, text=True)
            ok = p.returncode == 0
            reason = (p.stdout.strip().splitlines() or [""])[0] if ok else (p.stdout + p.stderr).strip()[-500:]
            client.call("POST", f"/units/{r['unit']}/verdict", {"accept": ok, "reason": reason})
            print(f"SWARM {r['unit']} from {r['account']}/{r['host']}: {'accepted' if ok else 'REJECTED'}: {reason}",
                  flush=True)
            if ok:
                for name in os.listdir(tmp):
                    if name.startswith("w"):
                        shutil.move(os.path.join(tmp, name), os.path.join(out, f"{r['unit']}-{name}"))
                shutil.move(os.path.join(tmp, "answers.jsonl"), os.path.join(out, f"{r['unit']}-answers.jsonl"))
                accepted[r["unit"]] = r["manifest"]
        finally:
            shutil.rmtree(tmp, ignore_errors=True)
    return accepted


def tally(manifests):
    """[wins, draws, losses] of the learner, per opponent index of the units"""
    out = {}
    for m in manifests:
        for g in m["games"]:
            w = out.setdefault(g.get("opponent", 0), [0, 0, 0])
            if g["winner"] is None:
                w[1] += 1
            elif g["winner"] == g["learner"]:
                w[0] += 1
            else:
                w[2] += 1
    return out


def client_from(args):
    tok = args.token_file and open(args.token_file).read().strip() or os.environ.get("MANA_TOKEN")
    if not tok:
        sys.exit("--token-file F or MANA_TOKEN")
    return Client(args.coordinator, tok)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--coordinator", required=True)
    ap.add_argument("--token-file", default=None)
    sub = ap.add_subparsers(dest="cmd", required=True)
    sub.add_parser("status")
    p = sub.add_parser("publish")
    p.add_argument("round_json")
    p = sub.add_parser("pull")
    p.add_argument("round_id", type=int)
    p.add_argument("out")
    p.add_argument("--worker-bin", default=None)
    p.add_argument("--wait", action="store_true")
    args = ap.parse_args()
    c = client_from(args)
    if args.cmd == "status":
        print(json.dumps(c.call("GET", "/status"), indent=1))
    elif args.cmd == "publish":
        r = json.load(open(args.round_json))
        nets = {}
        for u in r["units"]:
            u["learner"] = swarm_spec(u["learner"], c, nets)
            for o in u["opponents"]:
                if o["config"] != "self":
                    o["config"] = swarm_spec(o["config"], c, nets)
            u["nets"] = net_refs(c, u["learner"], [o["config"] for o in u["opponents"]], nets)
        publish(c, r["project"], r["run"], r["round"], r["units"], r.get("priority", 0))
    elif args.cmd == "pull":
        if args.wait:
            wait(c, args.round_id)
        ms = pull(c, args.round_id, args.out, args.worker_bin)
        print(f"SWARM pulled {len(ms)} units, {sum(len(m['games']) for m in ms.values())} games into {args.out}")


if __name__ == "__main__":
    main()
