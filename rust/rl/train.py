"""Supervised training (#390 Track B, L1 and L4): the policy learns the recorded policy (the search AI's choices for
imitation, or a search's improved policy for distillation), the value head and the critic learn each game's result,
and the belief head learns which of the opponent's unseen cards are in its hand.

python train.py --data data/bc [--data more] --out runs/bc [--epochs 6] [--init runs/x/net.pt]

Every tenth game is held out. Writes net.pt (PyTorch), net.pmnet (for playmat-ai), critic.pt and report.json.
"""

import argparse
import json
import math
from concurrent.futures import ThreadPoolExecutor
import os
import time

import numpy as np
import torch
import torch.nn.functional as F

from data import Data
from model import Critic, build, export, load_dir, params, save_dir


def device():
    if torch.backends.mps.is_available():
        return torch.device("mps")
    if torch.cuda.is_available():
        torch.backends.cuda.matmul.allow_tf32 = True
        torch.backends.cudnn.allow_tf32 = True
        return torch.device("cuda")
    return torch.device("cpu")


def tx_sizes(s):
    """`--tx` as JSON or as k=v pairs"""
    s = s.strip()
    if s.startswith("{"):
        return json.loads(s)
    return {k: int(v) for k, v in (kv.split("=") for kv in s.split(",") if kv)}


def refit_value(net, critic, data, train_idx, test_idx, dev, bs, report):
    """One pass over the training games that fits only the value head (a linear read of the trunk) from scratch"""
    head = net.value
    head.reset_parameters()
    for p in net.parameters():
        p.requires_grad_(False)
    for p in head.parameters():
        p.requires_grad_(True)
    opt = torch.optim.AdamW(head.parameters(), lr=1e-3, weight_decay=1e-2)
    rng = np.random.default_rng(1)
    perm = rng.permutation(train_idx)
    net.train()
    for i in range(0, len(perm), bs):
        b = data.batch(perm[i : i + bs], dev)
        val = losses(net, None, b)[1]
        opt.zero_grad()
        val.backward()
        opt.step()
    for p in net.parameters():
        p.requires_grad_(True)
    ev = evaluate(net, critic, data, test_idx, dev)
    report["value_refit_logloss"] = ev["value_logloss"]
    print(f"TRAIN value head refit: held-out log loss {ev['value_logloss']:.4f}", flush=True)


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
    ap.add_argument("--init", default=None, help="a run folder to start from (its net.pt, arch.json, critic.pt)")
    ap.add_argument("--arch", default="mlp", help="mlp, or tx (the transformer)")
    ap.add_argument("--holdout", default=None, help="cardtext.py's holdout.json: those cards' embeddings stay 0")
    ap.add_argument("--tx", default="{}", help="the transformer sizes: JSON, or d=192,layers=4,text=1")
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
    critic = Critic().to(dev)
    if args.init:
        net = load_dir(args.init, dev)
        cpath = os.path.join(args.init, "critic.pt")
        if os.path.exists(cpath):
            critic.load_state_dict(torch.load(cpath, map_location=dev))
    else:
        net = build({"arch": args.arch, **(tx_sizes(args.tx) if args.arch == "tx" else {})}).to(dev)
    print(f"TRAIN net {params(net)} params, critic {params(critic)}, on {dev}", flush=True)
    if args.holdout:
        # the cold-start test: these cards' own embeddings stay 0, so only their features (and with text=1 their
        # rules text) say what they are
        ids = torch.tensor(json.load(open(args.holdout))["ids"], device=dev)
        weight = net.emb.weight
        with torch.no_grad():
            weight[ids] = 0
        weight.register_hook(lambda g: g.index_fill(0, ids, 0))
        print(f"TRAIN holding out {len(ids)} card buckets", flush=True)
    opt = torch.optim.AdamW(list(net.parameters()) + list(critic.parameters()), lr=args.lr, weight_decay=1e-4)
    steps = args.epochs * math.ceil(len(train_idx) / args.bs)
    sched = torch.optim.lr_scheduler.OneCycleLR(opt, max_lr=args.lr, total_steps=steps, pct_start=0.1)
    rng = np.random.default_rng(0)
    report = {"decisions": int(len(data)), "held_out": int(len(test_idx)), "epochs": []}
    # the full-information critic overfits fast: keep its best epoch on the held-out games
    best_critic = (float("inf"), None)
    # the value overfits too (a game's decisions share one result): once its held-out loss rises, the trunk stops
    # learning from it, and at the end only the value head is refit on the final trunk
    value_on, best_value = True, float("inf")
    for ep in range(args.epochs):
        perm = rng.permutation(train_idx)
        tl = np.zeros(4)
        # the next batch is gathered on a thread while the GPU works on this one
        pool = ThreadPoolExecutor(1)
        starts = list(range(0, len(perm), args.bs))
        ahead = pool.submit(data.batch, perm[0 : args.bs], dev)
        for n_i, i in enumerate(starts):
            b = ahead.result()
            if n_i + 1 < len(starts):
                ahead = pool.submit(data.batch, perm[starts[n_i + 1] : starts[n_i + 1] + args.bs], dev)
            pol, val, bel, crit = losses(net, critic, b)[:4]
            loss = pol + (args.value_weight if value_on else 0.0) * val + args.belief_weight * bel + 0.5 * crit
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
        if value_on and ev["value_logloss"] > best_value + 0.002:
            value_on = False
            report["value_stopped_after_epoch"] = ep
            print(f"TRAIN value stops training the trunk (held-out log loss {ev['value_logloss']:.4f} > {best_value:.4f})", flush=True)
        best_value = min(best_value, ev["value_logloss"])
    if not value_on:
        refit_value(net, critic, data, train_idx, test_idx, dev, args.bs, report)
    # base rates for the held-out set: the value's coin, the belief's prior (each unseen card in hand at the mean rate)
    z = data.z[test_idx]
    report["value_coin_logloss"] = math.log(2)
    report["value_draw_share"] = float((z == 0.5).mean())
    ph = np.concatenate([data.pool_hand[data.pool_off[i] : data.pool_off[i + 1]] for i in test_idx[:20000]])
    r = ph.mean()
    report["belief_base_logloss"] = float(-(r * math.log(r) + (1 - r) * math.log(1 - r)))
    save_dir(net, args.out)
    critic.load_state_dict(best_critic[1])
    report["critic_best_logloss"] = best_critic[0]
    torch.save(critic.state_dict(), os.path.join(args.out, "critic.pt"))
    export(net.cpu(), os.path.join(args.out, "net.pmnet"), {"trained": "train.py", "data": args.data, "epochs": args.epochs})
    with open(os.path.join(args.out, "report.json"), "w") as f:
        json.dump(report, f, indent=1)
    print(f"TRAIN wrote {args.out}; belief base log loss {report['belief_base_logloss']:.4f}", flush=True)


if __name__ == "__main__":
    main()
