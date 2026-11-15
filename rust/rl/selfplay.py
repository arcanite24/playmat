"""Self-play reinforcement learning (#390 Track B, L2).

python selfplay.py --init runs/bc --out runs/rl [--iters 30] [--games 4000] [--jobs 11]

Each iteration:

1. the current network plays `--games` games with the `selfplay` binary, sampling its policy: 70% against itself, 20%
   against past snapshots, 10% against the scripted quick answers;
2. advantages from λ-returns (γ = 1, λ = 0.95) on the critic's values, the critic seeing the opponent's hand
   (PerfectDou); the reward is the result, plus a small potential-based shaping term (life and cards) annealed to 0;
3. a PPO update (clip 0.2) with a reverse KL to the imitation policy as an anchor, its weight annealed down, and a
   small entropy bonus; the value head learns the results too, so it stays usable as a score.

Every `--eval-every` iterations the greedy network plays fixed opponents and the results go in `log.jsonl`.
`--exploit MAIN.pt` trains an exploiter instead: the opponent is always the frozen MAIN network (L4).
"""

import argparse
import copy
import json
import math
import os
import random
import re
import subprocess
import time

import numpy as np
import torch
import torch.nn.functional as F

from data import Data
from model import Critic, Net, export
from train import device

ROOT = os.path.dirname(os.path.abspath(__file__))
BIN = os.path.join(ROOT, "..", "target", "alt", "release", "selfplay")


def run_games(learner, opponents, games, out, jobs, seed, keep=1000, tag=0):
    cmd = [BIN, "--out", out, "--games", str(games), "--jobs", str(jobs), "--keep", str(keep), "--seed", str(seed),
           "--tag", str(tag), "--learner", learner, "--opponents", opponents]
    r = subprocess.run(cmd, capture_output=True, text=True)
    if r.returncode != 0:
        raise RuntimeError(r.stderr[-2000:])
    tallies = {}
    for line in r.stdout.splitlines():
        m = re.match(r"SELFPLAY against (\S+): (\d+)-(\d+)-(\d+)", line)
        if m:
            w, d, l = map(int, m.groups()[1:])
            tallies[m.group(1)] = {"w": w, "d": d, "l": l, "score": (w + d / 2) / max(w + d + l, 1)}
    return tallies, r.stdout


def net_entrant(path, temp=1000):
    return f"cfg:policy=net+net={path}+temp={temp}"


GLOB_LIFE_ME, GLOB_LIFE_OPP, GLOB_HAND_ME, GLOB_HAND_OPP = 17 + 1, 17 + 2, 17 + 3, 17 + 4


def advantages(data, critic, dev, lam, shaping):
    """λ-return advantages and targets per decision, each seat's decisions of a game in order"""
    v = np.zeros(len(data), np.float32)
    critic.eval()
    with torch.no_grad():
        for i in range(0, len(data), 4096):
            idx = np.arange(i, min(i + 4096, len(data)))
            v[idx] = torch.sigmoid(critic(data.batch(idx, dev))).cpu().numpy()
    critic.train()
    g = data.glob.astype(np.float32)
    phi = shaping * ((g[:, GLOB_LIFE_ME] - g[:, GLOB_LIFE_OPP]) * 0.5 + (g[:, GLOB_HAND_ME] - g[:, GLOB_HAND_OPP]) * 0.3)
    adv = np.zeros(len(data), np.float32)
    key = data.run.astype(np.int64) * 10**9 + data.game.astype(np.int64) * 2 + data.seat
    order = np.lexsort((np.arange(len(data)), key))
    ks = key[order]
    bounds = np.nonzero(np.diff(ks))[0] + 1
    for traj in np.split(order, bounds):
        z = data.z[traj[0]]
        vals = v[traj]
        nxt = np.append(vals[1:], z)
        ph = phi[traj]
        ph_next = np.append(ph[1:], 0.0)  # Φ of the end is 0: the shaping telescopes away
        delta = nxt - vals + ph_next - ph
        a = np.zeros(len(traj), np.float32)
        run = 0.0
        for t in range(len(traj) - 1, -1, -1):
            run = delta[t] + lam * run
            a[t] = run
        adv[traj] = a
    ret = np.clip(adv + v, 0, 1)
    return adv, ret, v


def ppo_update(net, anchor, critic, data, adv, ret, dev, args, kl_w):
    opt = torch.optim.Adam(list(net.parameters()) + list(critic.parameters()), lr=args.lr)
    n = len(data)
    a_mean, a_std = adv.mean(), adv.std() + 1e-6
    stats = []
    for _ in range(args.epochs):
        perm = np.random.permutation(n)
        for i in range(0, n, args.bs):
            idx = perm[i : i + args.bs]
            b = data.batch(idx, dev)
            A = torch.from_numpy((adv[idx] - a_mean) / a_std).to(dev)
            R = torch.from_numpy(ret[idx]).to(dev)
            logits, v, _ = net(b)
            logp_all = F.log_softmax(logits, -1)
            logp = logp_all.gather(1, b["chosen"].unsqueeze(1)).squeeze(1)
            ratio = torch.exp(logp - b["logp"])
            pg = -torch.min(ratio * A, ratio.clamp(1 - args.clip, 1 + args.clip) * A).mean()
            p = logp_all.exp()
            ent = -(p * logp_all.masked_fill(~b["act_mask"], 0)).sum(-1).mean()
            with torch.no_grad():
                alog = F.log_softmax(anchor(b)[0], -1)
            # reverse KL(π || π_anchor) over the candidates
            kl = (p * (logp_all - alog).masked_fill(~b["act_mask"], 0)).sum(-1).mean()
            val = F.binary_cross_entropy_with_logits(v, b["z"])
            crit = F.mse_loss(torch.sigmoid(critic(b)), R)
            loss = pg + kl_w * kl - args.entropy * ent + 0.5 * val + crit
            opt.zero_grad()
            loss.backward()
            torch.nn.utils.clip_grad_norm_(net.parameters(), 1.0)
            opt.step()
            stats.append([pg.item(), kl.item(), ent.item(), val.item(), crit.item(), (ratio - 1).abs().mean().item()])
    return dict(zip(["pg", "kl_anchor", "entropy", "value", "critic", "ratio_dev"], np.mean(stats, 0).round(4).tolist()))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--init", required=True, help="a train.py run: net.pt and critic.pt")
    ap.add_argument("--out", required=True)
    ap.add_argument("--iters", type=int, default=30)
    ap.add_argument("--games", type=int, default=4000)
    ap.add_argument("--jobs", type=int, default=11)
    ap.add_argument("--epochs", type=int, default=2)
    ap.add_argument("--bs", type=int, default=1024)
    ap.add_argument("--lr", type=float, default=3e-4)
    ap.add_argument("--clip", type=float, default=0.2)
    ap.add_argument("--lam", type=float, default=0.95)
    ap.add_argument("--entropy", type=float, default=0.005)
    ap.add_argument("--kl-start", type=float, default=0.5)
    ap.add_argument("--kl-end", type=float, default=0.05)
    ap.add_argument("--shaping", type=float, default=0.1)
    ap.add_argument("--eval-every", type=int, default=5)
    ap.add_argument("--eval-games", type=int, default=1000)
    ap.add_argument("--eval-opponents", default="host@s2;host@s6")
    ap.add_argument("--scripted", default="cfg:policy=quick+playout=2")
    ap.add_argument("--exploit", default=None, help="train an exploiter against this frozen network (.pmnet)")
    ap.add_argument("--keep-data", action="store_true")
    args = ap.parse_args()
    os.makedirs(args.out, exist_ok=True)
    dev = device()
    net, critic = Net().to(dev), Critic().to(dev)
    net.load_state_dict(torch.load(os.path.join(args.init, "net.pt"), map_location=dev))
    critic.load_state_dict(torch.load(os.path.join(args.init, "critic.pt"), map_location=dev))
    anchor = copy.deepcopy(net).eval()
    for p in anchor.parameters():
        p.requires_grad_(False)
    bc_path = os.path.abspath(os.path.join(args.init, "net.pmnet"))
    snapshots = [bc_path]
    log = open(os.path.join(args.out, "log.jsonl"), "a")
    t0 = time.time()
    for it in range(1, args.iters + 1):
        t_it = time.time()
        frac = (it - 1) / max(args.iters - 1, 1)
        kl_w = args.kl_start * (args.kl_end / args.kl_start) ** frac
        shaping = args.shaping * (1 - frac)
        cur = os.path.abspath(os.path.join(args.out, f"it{it - 1:03d}.pmnet"))
        export(copy.deepcopy(net).cpu(), cur, {"iter": it - 1})
        if args.exploit:
            opponents = f"{net_entrant(os.path.abspath(args.exploit), 0)}*1"
        else:
            snaps = random.Random(it).sample(snapshots, min(2, len(snapshots)))
            opponents = ";".join(["self*7"] + [f"{net_entrant(s)}*1" for s in snaps] + [f"{args.scripted}*1"])
            if len(snaps) == 1:
                opponents = opponents.replace(f"{net_entrant(snaps[0])}*1", f"{net_entrant(snaps[0])}*2")
        ddir = os.path.join(args.out, "data", f"it{it:03d}")
        tallies, _ = run_games(net_entrant(cur), opponents, args.games, ddir, args.jobs, seed=it * 1_000_003, tag=it)
        tg = time.time()
        data = Data([ddir])
        adv, ret, v = advantages(data, critic, dev, args.lam, shaping)
        stats = ppo_update(net, anchor, critic, data, adv, ret, dev, args, kl_w)
        entry = {"iter": it, "decisions": len(data), "kl_w": round(kl_w, 4), "shaping": round(shaping, 4), "selfplay": tallies,
                 "update": stats, "gen_s": round(tg - t_it, 1), "elapsed_s": round(time.time() - t0, 1)}
        if not args.keep_data:
            for f in os.listdir(ddir):
                os.remove(os.path.join(ddir, f))
            os.rmdir(ddir)
        if it % 3 == 0:
            snapshots.append(cur)
        if it % args.eval_every == 0 or it == args.iters:
            path = os.path.abspath(os.path.join(args.out, f"it{it:03d}.pmnet"))
            export(copy.deepcopy(net).cpu(), path, {"iter": it})
            opp = ";".join([f"{o}*1" for o in args.eval_opponents.split(";")] + [f"{net_entrant(bc_path, 0)}*1"])
            if args.exploit:
                opp = f"{net_entrant(os.path.abspath(args.exploit), 0)}*1"
            ev, _ = run_games(net_entrant(path, 0), opp, args.eval_games, os.path.join(args.out, "eval"), args.jobs,
                              seed=900_000_000, keep=0)
            entry["eval"] = ev
        print(json.dumps(entry), flush=True)
        log.write(json.dumps(entry) + "\n")
        log.flush()
    torch.save(net.state_dict(), os.path.join(args.out, "net.pt"))
    torch.save(critic.state_dict(), os.path.join(args.out, "critic.pt"))
    export(net.cpu(), os.path.join(args.out, "net.pmnet"), {"trained": "selfplay.py", "iters": args.iters})


if __name__ == "__main__":
    main()
