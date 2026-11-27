"""The trainer's side of Mana@Home (docs/swarm/Design.md): publishes a round's work units to the coordinator
(`playmat-swarm`), waits for the swarm to play them, pulls each completed unit, replays it with
`playmat-worker verify` against the unit the coordinator leased, and accepts or rejects it. Accepted shards land in
one folder, each file prefixed with its unit id, ready for `data.py` and `train.py`.

    python swarm.py --coordinator URL --token-file F status|leaderboard|networks|gates|benches|rounds
    python swarm.py --coordinator URL --token-file F publish|publish-gate|publish-bench FILE.json
    python swarm.py --coordinator URL --token-file F hold-gate|resume-gate|promote-gate GATE_ID [--reason R]
    python swarm.py --coordinator URL --token-file F rollback TARGET_SHA --reason R
    python swarm.py --coordinator URL --token-file F priority ROUND_ID N
    python swarm.py --coordinator URL --token-file F cancel-round ROUND_ID [--include-leased] [--reason R]
    python swarm.py --coordinator URL --token-file F cancel-unit UNIT_ID [--reason R]
    python swarm.py --coordinator URL --token-file F pull ROUND_ID OUT [--worker-bin B] [--wait] [--accepted-only]
    python swarm.py --coordinator URL --token-file F workers|inbox|audit
    python swarm.py --coordinator URL --token-file F resolve-inbox ID ACTION [--reason R]
    python swarm.py --coordinator URL --token-file F trust TOKEN_ID LEVEL [--reason R]
    python swarm.py --coordinator URL --token-file F quarantine|restore TOKEN_ID [--round-id R] [--reason R]
    python swarm.py --coordinator URL --token-file F mark-trained ROUND_ID | order-rerun UNIT_ID | reissue UNIT_ID

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


def wait(client, round_id, poll=30, timeout=None, wait_verify=False):
    """Waits until no unit is queued or leased (and optionally no verify units pending); gives the last status"""
    t0 = time.time()
    while True:
        st = client.call("GET", f"/rounds/{round_id}")
        u = st["units"]
        vp = st.get("verify_pending", 0)
        print(f"SWARM round {round_id}: {json.dumps(u, sort_keys=True)}, {st['games']} games, verify_pending={vp}", flush=True)
        if not u.get("queued") and not u.get("leased") and (not wait_verify or vp == 0):
            return st
        if timeout and time.time() - t0 > timeout:
            return st
        time.sleep(poll)


def worker_bin():
    ext = ".exe" if os.name == "nt" else ""
    return os.path.join(REPO, "rust", "target", "release", "playmat-worker" + ext)


def pull_accepted(client, round_id, out):
    """Downloads shards for units already verified and accepted by swarm verifiers (S2)."""
    os.makedirs(out, exist_ok=True)
    accepted = {}
    for r in client.call("GET", f"/rounds/{round_id}/results?state=accepted"):
        tmp = tempfile.mkdtemp(prefix="swarm-accepted-")
        try:
            for path, url in r["files"].items():
                if "/" in path or "\\" in path or path.startswith("."):
                    raise ValueError(f"{r['unit']}: file name {path}")
                dst = os.path.join(tmp, path)
                with open(dst, "wb") as f:
                    f.write(get(url))
                expected = r["manifest"]["files"].get(path)
                if expected and sha256_file(dst) != expected:
                    raise ValueError(f"{r['unit']}/{path}: downloaded sha256 mismatch")
            for name in os.listdir(tmp):
                if name.startswith("w") or name == "answers.jsonl":
                    shutil.move(os.path.join(tmp, name), os.path.join(out, f"{r['unit']}-{name}"))
            accepted[r["unit"]] = r["manifest"]
        finally:
            shutil.rmtree(tmp, ignore_errors=True)
    return accepted


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
    sub.add_parser("leaderboard")
    sub.add_parser("networks")
    sub.add_parser("gates")
    sub.add_parser("benches")
    sub.add_parser("rounds")
    sub.add_parser("workers")
    for cmd in ("publish-gate", "publish-bench"):
        p = sub.add_parser(cmd)
        p.add_argument("spec_json")
    for cmd in ("hold-gate", "resume-gate", "promote-gate"):
        p = sub.add_parser(cmd)
        p.add_argument("gate_id", type=int)
        p.add_argument("--reason", default="")
    p = sub.add_parser("rollback")
    p.add_argument("target_sha")
    p.add_argument("--reason", required=True)
    p = sub.add_parser("inbox")
    p.add_argument("--status", default=None)
    p = sub.add_parser("resolve-inbox")
    p.add_argument("item_id", type=int)
    p.add_argument("action", choices=["confirm-cheat", "false-positive", "order-rerun", "dismiss"])
    p.add_argument("--reason", default="")
    p = sub.add_parser("trust")
    p.add_argument("token_id", type=int)
    p.add_argument("level", choices=["probation", "normal", "reputable", "trusted", "banned"])
    p.add_argument("--reason", default="")
    for cmd in ("quarantine", "restore"):
        p = sub.add_parser(cmd)
        p.add_argument("token_id", type=int)
        p.add_argument("--round-id", type=int, default=None)
        p.add_argument("--reason", default="")
    p = sub.add_parser("mark-trained")
    p.add_argument("round_id", type=int)
    p = sub.add_parser("order-rerun")
    p.add_argument("unit_id")
    p.add_argument("--reason", default="")
    p = sub.add_parser("reissue")
    p.add_argument("unit_id")
    p = sub.add_parser("audit")
    p.add_argument("--action", default=None)
    p.add_argument("--actor", default=None)
    p.add_argument("--limit", type=int, default=100)
    p = sub.add_parser("publish")
    p.add_argument("round_json")
    p = sub.add_parser("publish-gpu")
    p.add_argument("round_json")
    p.add_argument("--vloss", type=int, default=8)
    p = sub.add_parser("priority")
    p.add_argument("round_id", type=int)
    p.add_argument("priority", type=int)
    p = sub.add_parser("cancel-round")
    p.add_argument("round_id", type=int)
    p.add_argument("--include-leased", action="store_true")
    p.add_argument("--reason", default="")
    p = sub.add_parser("cancel-unit")
    p.add_argument("unit_id")
    p.add_argument("--reason", default="")
    p = sub.add_parser("pull")
    p.add_argument("round_id", type=int)
    p.add_argument("out")
    p.add_argument("--worker-bin", default=None)
    p.add_argument("--wait", action="store_true")
    p.add_argument("--accepted-only", action="store_true")
    args = ap.parse_args()
    c = client_from(args)
    if args.cmd == "status":
        print(json.dumps(c.call("GET", "/status"), indent=1))
    elif args.cmd == "leaderboard":
        print(json.dumps(c.call("GET", "/leaderboard"), indent=1))
    elif args.cmd == "networks":
        print(json.dumps(c.call("GET", "/networks"), indent=1))
    elif args.cmd == "gates":
        print(json.dumps(c.call("GET", "/gates"), indent=1))
    elif args.cmd == "benches":
        print(json.dumps(c.call("GET", "/benches"), indent=1))
    elif args.cmd == "publish-gate":
        r = json.load(open(args.spec_json))
        nets = {}
        for u in r["units"]:
            u["learner"] = swarm_spec(u["learner"], c, nets)
            for o in u["opponents"]:
                if o["config"] != "self":
                    o["config"] = swarm_spec(o["config"], c, nets)
            u["nets"] = net_refs(c, u["learner"], [o["config"] for o in u["opponents"]], nets)
        print(json.dumps(c.call("POST", "/gates", r), indent=1))
    elif args.cmd == "publish-bench":
        r = json.load(open(args.spec_json))
        nets = {}
        for u in r["units"]:
            u["learner"] = swarm_spec(u["learner"], c, nets)
            for o in u["opponents"]:
                if o["config"] != "self":
                    o["config"] = swarm_spec(o["config"], c, nets)
            u["nets"] = net_refs(c, u["learner"], [o["config"] for o in u["opponents"]], nets)
        print(json.dumps(c.call("POST", "/bench", r), indent=1))
    elif args.cmd == "hold-gate":
        print(json.dumps(c.call("POST", f"/admin/gates/{args.gate_id}/hold", {"reason": args.reason}), indent=1))
    elif args.cmd == "resume-gate":
        print(json.dumps(c.call("POST", f"/admin/gates/{args.gate_id}/resume", {"reason": args.reason}), indent=1))
    elif args.cmd == "promote-gate":
        print(json.dumps(c.call("POST", f"/admin/gates/{args.gate_id}/promote", {"reason": args.reason}), indent=1))
    elif args.cmd == "rollback":
        c.call("POST", "/admin/networks/rollback", {"target_sha": args.target_sha, "reason": args.reason})
        print(f"SWARM rolled back promoted network to {args.target_sha}")
    elif args.cmd == "rounds":
        print(json.dumps(c.call("GET", "/rounds"), indent=1))
    elif args.cmd == "workers":
        print(json.dumps(c.call("GET", "/admin/workers"), indent=1))
    elif args.cmd == "inbox":
        qs = f"?status={args.status}" if args.status else ""
        print(json.dumps(c.call("GET", f"/admin/inbox{qs}"), indent=1))
    elif args.cmd == "resolve-inbox":
        c.call("POST", f"/admin/inbox/{args.item_id}/resolve", {"action": args.action, "reason": args.reason})
        print(f"SWARM resolved inbox item {args.item_id} ({args.action})")
    elif args.cmd == "trust":
        c.call("POST", f"/admin/tokens/{args.token_id}/trust", {"trust": args.level, "reason": args.reason})
        print(f"SWARM token {args.token_id} -> {args.level}")
    elif args.cmd in ("quarantine", "restore"):
        body = {"round_id": args.round_id, "reason": args.reason}
        print(json.dumps(c.call("POST", f"/admin/tokens/{args.token_id}/{args.cmd}", body), indent=1))
    elif args.cmd == "mark-trained":
        c.call("POST", f"/admin/rounds/{args.round_id}/trained", {})
        print(f"SWARM round {args.round_id} marked trained")
    elif args.cmd == "order-rerun":
        print(json.dumps(c.call("POST", f"/admin/units/{args.unit_id}/rerun", {"reason": args.reason}), indent=1))
    elif args.cmd == "reissue":
        c.call("POST", f"/admin/units/{args.unit_id}/reissue", {})
        print(f"SWARM reissued {args.unit_id}")
    elif args.cmd == "audit":
        params = [f"limit={args.limit}"]
        if args.action:
            params.append(f"action={args.action}")
        if args.actor:
            params.append(f"actor={args.actor}")
        print(json.dumps(c.call("GET", f"/admin/audit?{'&'.join(params)}"), indent=1))
    elif args.cmd == "priority":
        print(json.dumps(c.call("POST", f"/rounds/{args.round_id}/priority", {"priority": args.priority}), indent=1))
    elif args.cmd == "cancel-round":
        body = {"reason": args.reason, "include_leased": args.include_leased}
        print(json.dumps(c.call("POST", f"/rounds/{args.round_id}/cancel", body), indent=1))
    elif args.cmd == "cancel-unit":
        c.call("POST", f"/units/{args.unit_id}/cancel", {"reason": args.reason})
        print(f"SWARM cancelled {args.unit_id}")
    elif args.cmd in ("publish", "publish-gpu"):
        r = json.load(open(args.round_json))
        nets = {}
        for u in r["units"]:
            if args.cmd == "publish-gpu":
                u["kind"] = "selfplay-gpu"
                u.setdefault("pins", {})["strict"] = False
                if "vloss=" not in u["learner"] and args.vloss > 1:
                    u["learner"] = f"{u['learner']}+vloss={args.vloss}"
            u["learner"] = swarm_spec(u["learner"], c, nets)
            for o in u["opponents"]:
                if o["config"] != "self":
                    o["config"] = swarm_spec(o["config"], c, nets)
            u["nets"] = net_refs(c, u["learner"], [o["config"] for o in u["opponents"]], nets)
        publish(c, r["project"], r["run"], r["round"], r["units"], r.get("priority", 0))
    elif args.cmd == "pull":
        if args.wait:
            wait(c, args.round_id, wait_verify=args.accepted_only)
        ms = pull_accepted(c, args.round_id, args.out) if args.accepted_only else pull(c, args.round_id, args.out, args.worker_bin)
        print(f"SWARM pulled {len(ms)} units, {sum(len(m['games']) for m in ms.values())} games into {args.out}")


if __name__ == "__main__":
    main()
