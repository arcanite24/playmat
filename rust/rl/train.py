"""Supervised training (#390 Track B, L1 and L4): the policy learns the recorded policy (the search AI's choices for
imitation, or a search's improved policy for distillation), the value head and the critic learn each game's result,
and the belief head learns which of the opponent's unseen cards are in its hand.

python train.py --data data/bc [--data more] --out runs/bc [--epochs 6] [--init runs/x/net.pt]

Every tenth game is held out. Writes net.pt (PyTorch), net.pmnet (for playmat-ai), critic.pt and report.json.
"""

import argparse
import json
import math
import os
import time

import numpy as np
import torch
import torch.nn.functional as F

from data import Data
from model import Critic, Net, export, params


def device():
    if torch.backends.mps.is_available():
        return torch.device("mps")
    if torch.cuda.is_available():
        return torch.device("cuda")
    return torch.device("cpu")


def losses(net, critic, b):
    logits, v, h = net(b)
    logp = F.log_softmax(logits, -1)
    pol = -(b["target"] * logp).sum(-1).mean()
    val = F.binary_cross_entropy_with_logits(v, b["z"])
    bl = net.belief_logits(h, b["pool"])
    m = b["pool_mask"].float()
    bel = (F.binary_cross_entropy_with_logits(bl, b["pool_hand"], reduction="none") * m).sum() / m.sum().clamp(min=1)
    crit = F.binary_cross_entropy_with_logits(critic(b), b["z"]) if critic is not None else torch.zeros(())
    right = (logits.argmax(-1) == b["target"].argmax(-1)).float()
    off = (b["target"].argmax(-1) != 0).float()
    acc = right.mean()
    # accuracy where the recorded policy didn't take the quick answer (candidate 0): what the features alone can't say
    acc_off = (right * off).sum() / off.sum().clamp(min=1)
    return pol, val, bel, crit, acc, acc_off, 1 - off.mean()


def evaluate(net, critic, data, idx, dev, bs=2048):
    net.eval()
    tot = np.zeros(7)
    n = 0
    with torch.no_grad():
        for i in range(0, len(idx), bs):
            j = idx[i : i + bs]
            b = data.batch(j, dev)
            out = losses(net, critic, b)
            tot += np.array([float(x) for x in out]) * len(j)
            n += len(j)
    net.train()
    pol, val, bel, crit, acc, acc_off, quick = tot / max(n, 1)
    return {"policy_ce": pol, "value_logloss": val, "belief_logloss": bel, "critic_logloss": crit, "accuracy": acc,
            "accuracy_not_quick": acc_off, "teacher_picks_quick": quick}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--data", action="append", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--epochs", type=int, default=6)
    ap.add_argument("--bs", type=int, default=512)
    ap.add_argument("--lr", type=float, default=2e-3)
    ap.add_argument("--init", default=None)
    ap.add_argument("--value-weight", type=float, default=0.5)
    ap.add_argument("--belief-weight", type=float, default=0.2)
    args = ap.parse_args()
    os.makedirs(args.out, exist_ok=True)
    dev = device()
    t0 = time.time()
    data = Data(args.data)
    print(f"TRAIN {len(data)} decisions from {len(data.infos)} games, {time.time() - t0:.0f} s to read", flush=True)
    held = (data.game % 10) == 0
    train_idx, test_idx = np.nonzero(~held)[0], np.nonzero(held)[0]
    net, critic = Net().to(dev), Critic().to(dev)
    if args.init:
        net.load_state_dict(torch.load(args.init, map_location=dev))
        cpath = os.path.join(os.path.dirname(args.init), "critic.pt")
        if os.path.exists(cpath):
            critic.load_state_dict(torch.load(cpath, map_location=dev))
    print(f"TRAIN net {params(net)} params, critic {params(critic)}, on {dev}", flush=True)
    opt = torch.optim.AdamW(list(net.parameters()) + list(critic.parameters()), lr=args.lr, weight_decay=1e-4)
    steps = args.epochs * math.ceil(len(train_idx) / args.bs)
    sched = torch.optim.lr_scheduler.OneCycleLR(opt, max_lr=args.lr, total_steps=steps, pct_start=0.1)
    rng = np.random.default_rng(0)
    report = {"decisions": int(len(data)), "held_out": int(len(test_idx)), "epochs": []}
    # the full-information critic overfits fast: keep its best epoch on the held-out games
    best_critic = (float("inf"), None)
    for ep in range(args.epochs):
        perm = rng.permutation(train_idx)
        tl = np.zeros(4)
        for i in range(0, len(perm), args.bs):
            b = data.batch(perm[i : i + args.bs], dev)
            pol, val, bel, crit = losses(net, critic, b)[:4]
            loss = pol + args.value_weight * val + args.belief_weight * bel + 0.5 * crit
            opt.zero_grad()
            loss.backward()
            torch.nn.utils.clip_grad_norm_(net.parameters(), 1.0)
            opt.step()
            sched.step()
            tl += [pol.item(), val.item(), bel.item(), crit.item()]
        nb = math.ceil(len(perm) / args.bs)
        ev = evaluate(net, critic, data, test_idx, dev)
        ev["epoch"] = ep + 1
        ev["train"] = (tl / nb).tolist()
        report["epochs"].append(ev)
        if ev["critic_logloss"] < best_critic[0]:
            best_critic = (ev["critic_logloss"], {k: v.detach().clone() for k, v in critic.state_dict().items()})
        print(f"TRAIN epoch {ep + 1}: held out {json.dumps({k: round(v, 4) if isinstance(v, float) else v for k, v in ev.items()})} ({time.time() - t0:.0f} s)", flush=True)
    # base rates for the held-out set: the value's coin, the belief's prior (each unseen card in hand at the mean rate)
    z = data.z[test_idx]
    report["value_coin_logloss"] = math.log(2)
    report["value_draw_share"] = float((z == 0.5).mean())
    ph = np.concatenate([data.pool_hand[data.pool_off[i] : data.pool_off[i + 1]] for i in test_idx[:20000]])
    r = ph.mean()
    report["belief_base_logloss"] = float(-(r * math.log(r) + (1 - r) * math.log(1 - r)))
    torch.save(net.state_dict(), os.path.join(args.out, "net.pt"))
    critic.load_state_dict(best_critic[1])
    report["critic_best_logloss"] = best_critic[0]
    torch.save(critic.state_dict(), os.path.join(args.out, "critic.pt"))
    export(net.cpu(), os.path.join(args.out, "net.pmnet"), {"trained": "train.py", "data": args.data, "epochs": args.epochs})
    with open(os.path.join(args.out, "report.json"), "w") as f:
        json.dump(report, f, indent=1)
    print(f"TRAIN wrote {args.out}; belief base log loss {report['belief_base_logloss']:.4f}", flush=True)


if __name__ == "__main__":
    main()
