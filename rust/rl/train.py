"""Supervised training (#390 Track B, L1 and L4, and the breakthrough loop):
the policy learns the recorded search policy, the value head learns each game's result (mixed with root search Q when
present, weighted by inverse-sqrt game length), the belief head learns opponent hand cards, and an optional
training-only auxiliary head predicts turns remaining and endgame life differential.

python train.py --data data/r1 --data data/r2 --out runs/r2 --bs 2048 --amp 1 --no-critic
"""

import argparse
from concurrent.futures import ThreadPoolExecutor
import copy
import json
import math
import os
import time

import numpy as np
import torch
import torch.nn.functional as F

from data import Data, compact_dir
from model import AuxHead, Critic, GLOB_F, Tx, build, export, load_dir, params, save_dir


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
    out = {}
    for kv in s.split(","):
        if not kv:
            continue
        k, v = kv.split("=")
        out[k] = float(v) if "." in v else int(v)
    return out


def value_head_params(net):
    """Parameters belonging exclusively to the value head (for higher weight decay and refitting)."""
    if isinstance(net, Tx) and net.val_mlp:
        mods = [net.val_ln, net.val_fc1, net.val_fc2]
    else:
        mods = [net.value]
    return [p for m in mods for p in m.parameters()]


def refit_value(net, critic, data, train_idx, test_idx, dev, bs, report, q_mix=0.0):
    """One pass over the training games that fits only the value head from scratch on the frozen trunk."""
    if isinstance(net, Tx) and net.val_mlp:
        for m in (net.val_ln, net.val_fc1, net.val_fc2):
            m.reset_parameters()
        head_params = value_head_params(net)
    else:
        net.value.reset_parameters()
        head_params = list(net.value.parameters())
    for p in net.parameters():
        p.requires_grad_(False)
    for p in head_params:
        p.requires_grad_(True)
    opt = torch.optim.AdamW(head_params, lr=1e-3, weight_decay=1e-2)
    rng = np.random.default_rng(1)
    perm = rng.permutation(train_idx)
    if len(perm) > 2_000_000:
        perm = perm[:2_000_000]
    net.train()
    for i in range(0, len(perm), bs):
        b = data.batch(perm[i : i + bs], dev)
        val = losses(net, None, None, b, q_mix=q_mix)[1]
        opt.zero_grad(set_to_none=True)
        val.backward()
        opt.step()
    for p in net.parameters():
        p.requires_grad_(True)
    ev = evaluate(net, critic, data, test_idx, dev, q_mix=0.0)
    report["value_refit_logloss"] = ev["value_logloss"]
    print(f"TRAIN value head refit: held-out log loss {ev['value_logloss']:.4f}", flush=True)


def losses(net, critic, aux_head, b, q_mix=0.0, pol_min_run=0):
    want_aux = aux_head is not None
    if want_aux:
        logits, v, h, vfeat = net(b, return_vfeat=True)
    else:
        logits, v, h = net(b)
    logp = F.log_softmax(logits, -1)
    pol_per = -(b["target"] * logp).sum(-1)
    if pol_min_run > 0:
        pm = (b["run"] >= pol_min_run).float()
        pol = (pol_per * pm).sum() / pm.sum().clamp(min=1)
    else:
        pol = pol_per.mean()

    # Mix binary outcome z with root search Q-value when finite
    z_tgt = b["z"]
    if q_mix > 0.0:
        qv = b["qval"]
        has_q = torch.isfinite(qv)
        qv_safe = torch.nan_to_num(qv, nan=0.5).clamp(0.0, 1.0)
        z_tgt = torch.where(has_q, (1.0 - q_mix) * z_tgt + q_mix * qv_safe, z_tgt)
    val_per = F.binary_cross_entropy_with_logits(v, z_tgt, reduction="none")
    vw = b["val_w"]
    val = (val_per * vw).sum() / vw.sum().clamp(min=1)

    bl = net.belief_logits(h, b["pool"])
    m = b["pool_mask"].float()
    bel = (F.binary_cross_entropy_with_logits(bl, b["pool_hand"], reduction="none") * m).sum() / m.sum().clamp(min=1)

    crit = F.binary_cross_entropy_with_logits(critic(b), b["z"]) if critic is not None else torch.zeros((), device=v.device)
    if want_aux:
        aux_pred = aux_head(vfeat)
        aux_tgt = torch.stack([b["turns_left"], b["final_life_diff"]], -1)
        aux = F.smooth_l1_loss(aux_pred, aux_tgt)
    else:
        aux = torch.zeros((), device=v.device)

    right = (logits.argmax(-1) == b["target"].argmax(-1)).float()
    off = (b["target"].argmax(-1) != 0).float()
    acc = right.mean()
    # accuracy where the recorded policy didn't take the quick answer (candidate 0)
    acc_off = (right * off).sum() / off.sum().clamp(min=1)
    return pol, val, bel, crit, aux, acc, acc_off, 1 - off.mean()


def evaluate(net, critic, data, idx, dev, bs=2048, q_mix=0.0, pol_min_run=0):
    net.eval()
    tot = np.zeros(8)
    n = 0
    # Cap eval set at 150k decisions for speed on huge multi-round replay buffers
    if len(idx) > 150_000:
        step = max(len(idx) // 150_000, 1)
        idx = idx[::step]
    with torch.no_grad():
        for i in range(0, len(idx), bs):
            j = idx[i : i + bs]
            b = data.batch(j, dev)
            # Evaluate value_logloss against true game outcome z (q_mix=0) and unweighted
            logits, v, h = net(b)
            logp = F.log_softmax(logits, -1)
            pol_per = -(b["target"] * logp).sum(-1)
            if pol_min_run > 0:
                pm = (b["run"] >= pol_min_run).float()
                pol = (pol_per * pm).sum() / pm.sum().clamp(min=1)
            else:
                pol = pol_per.mean()
            val = F.binary_cross_entropy_with_logits(v, b["z"])
            bl = net.belief_logits(h, b["pool"])
            m = b["pool_mask"].float()
            bel = (F.binary_cross_entropy_with_logits(bl, b["pool_hand"], reduction="none") * m).sum() / m.sum().clamp(min=1)
            crit = F.binary_cross_entropy_with_logits(critic(b), b["z"]) if critic is not None else torch.zeros((), device=v.device)
            right = (logits.argmax(-1) == b["target"].argmax(-1)).float()
            off = (b["target"].argmax(-1) != 0).float()
            acc = right.mean()
            acc_off = (right * off).sum() / off.sum().clamp(min=1)
            out = (pol, val, bel, crit, torch.zeros(()), acc, acc_off, 1 - off.mean())
            tot += np.array([float(x) for x in out]) * len(j)
            n += len(j)
    net.train()
    pol, val, bel, crit, _, acc, acc_off, quick = tot / max(n, 1)
    return {
        "policy_ce": pol, "value_logloss": val, "belief_logloss": bel, "critic_logloss": crit,
        "accuracy": acc, "accuracy_not_quick": acc_off, "teacher_picks_quick": quick,
    }


def _select_epoch_blocks(data, samples_target, pol_min_run, rng, block_len=512):
    """Picks block start indices totaling ~`samples_target` decisions, ensuring recent policy dirs get >= 50% share."""
    n_blocks = max(1, (len(data) + block_len - 1) // block_len)
    want_blocks = max(1, min(n_blocks, (samples_target + block_len - 1) // block_len))
    starts = np.arange(0, len(data), block_len)
    if pol_min_run <= 0:
        chosen = rng.choice(starts, size=want_blocks, replace=False)
        chosen.sort()
        return chosen
    block_runs = data.run[starts]
    recent = starts[block_runs >= pol_min_run]
    older = starts[block_runs < pol_min_run]
    if len(older) == 0 or len(recent) == 0:
        chosen = rng.choice(starts, size=want_blocks, replace=False)
        chosen.sort()
        return chosen
    # Allocate 60% of epoch blocks to recent policy rounds, 40% to older value replay buffer
    n_rec = min(len(recent), max(1, int(want_blocks * 0.6)))
    n_old = min(len(older), want_blocks - n_rec)
    if n_rec + n_old < want_blocks:
        n_rec = min(len(recent), want_blocks - n_old)
    c_rec = rng.choice(recent, size=n_rec, replace=False)
    c_old = rng.choice(older, size=n_old, replace=False) if n_old > 0 else np.zeros(0, dtype=starts.dtype)
    chosen = np.concatenate([c_rec, c_old])
    chosen.sort()
    return chosen


def build_parser():
    ap = argparse.ArgumentParser()
    ap.add_argument("--data", action="append", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--epochs", type=int, default=6)
    ap.add_argument("--bs", type=int, default=512)
    ap.add_argument("--lr", type=float, default=2e-3)
    ap.add_argument("--init", default=None, help="a run folder to start from (its net.pt, arch.json, critic.pt)")
    ap.add_argument("--arch", default="mlp", help="mlp, or tx (the transformer)")
    ap.add_argument("--holdout", default=None, help="cardtext.py's holdout.json: those cards' embeddings stay 0")
    ap.add_argument("--tx", default="{}", help="the transformer sizes: JSON, or d=192,layers=4,text=1,val_mlp=1,gelu=1")
    ap.add_argument("--value-weight", type=float, default=0.5)
    ap.add_argument("--belief-weight", type=float, default=0.2)
    ap.add_argument("--aux-weight", type=float, default=0.15, help="training-only auxiliary positional loss weight (0 disables)")
    ap.add_argument("--q-mix", type=float, default=0.4, help="weight on root search Q-value in value target when present")
    ap.add_argument("--policy-recent", type=int, default=0, help="only train policy on the last N --data dirs (0 = all)")
    ap.add_argument("--samples-per-epoch", type=int, default=0, help="cap/stage this many decisions per epoch onto GPU (0 = all)")
    ap.add_argument("--max-per-dir", type=int, default=0, help="cap decisions read per recent --data dir (0 = all)")
    ap.add_argument("--max-old-per-dir", type=int, default=0, help="cap decisions read per older (value-only) --data dir (0 = all)")
    ap.add_argument("--no-critic", action="store_true", help="skip loading hid and training the PPO Critic")
    ap.add_argument("--upgrade-val-mlp", action="store_true", help="upgrade a v1 Tx checkpoint to the regularized 2-layer val_mlp head")
    ap.add_argument("--drop", type=float, default=0.0, help="training-only dropout inside Tx blocks")
    ap.add_argument("--val-wd", type=float, default=1e-2, help="weight decay on value head parameters")
    ap.add_argument("--val-stop-tol", type=float, default=0.004, help="held-out value loss rise before freezing trunk value grad")
    ap.add_argument("--ema", type=float, default=0.0, help="EMA decay for exported weights (e.g. 0.995; 0 disables)")
    ap.add_argument("--amp", type=int, default=-1, help="1: bf16 autocast on GPU; -1: auto at bs >= 1024")
    ap.add_argument("--gpu-data-gb", type=float, default=12.0, help="keep whole dataset on GPU when <= this many GB")
    ap.add_argument("--compile", action="store_true", help="use torch.compile on CUDA")
    ap.add_argument("--compact", action="store_true", help="compact multi-shard --data dirs before reading")
    return ap


def train_main(argv=None, shared_data=None):
    args = build_parser().parse_args(argv)
    os.makedirs(args.out, exist_ok=True)
    dev = device()
    t0 = time.time()
    if args.compact:
        for d in args.data:
            if os.path.isdir(d):
                compact_dir(d)
    if shared_data is not None:
        data = shared_data
    else:
        data = Data(
            args.data,
            load_critic=not args.no_critic,
            max_per_dir=args.max_per_dir,
            max_old_per_dir=args.max_old_per_dir,
            policy_recent=args.policy_recent,
        )
        print(f"TRAIN {len(data)} decisions from {len(data.infos)} games, {time.time() - t0:.1f} s to read", flush=True)
    where, size = data.place(dev, args.gpu_data_gb)
    print(f"TRAIN data {size:.1f} GB on {where}", flush=True)
    amp = (args.amp == 1 or (args.amp == -1 and args.bs >= 1024)) and dev.type == "cuda"
    pol_min_run = max(0, len(args.data) - args.policy_recent) if args.policy_recent > 0 else 0

    held = (data.game % 10) == 0
    train_idx, test_idx = np.nonzero(~held)[0], np.nonzero(held)[0]
    # Also evaluate held-out policy specifically on recent rounds if policy_recent > 0
    critic = None if args.no_critic else Critic().to(dev)
    if args.init:
        net = load_dir(args.init, dev, upgrade_val_mlp=args.upgrade_val_mlp, drop=args.drop)
        if critic is not None:
            cpath = os.path.join(args.init, "critic.pt")
            if os.path.exists(cpath):
                critic.load_state_dict(torch.load(cpath, map_location=dev))
    else:
        kw = tx_sizes(args.tx) if args.arch == "tx" else {}
        if args.arch == "tx" and args.drop > 0:
            kw["drop"] = args.drop
        net = build({"arch": args.arch, **kw}).to(dev)

    aux_head = None
    if args.aux_weight > 0:
        aux_in = (3 * net.d + GLOB_F) if isinstance(net, Tx) else net.trunk2.out_features
        aux_head = AuxHead(aux_in).to(dev)

    print(
        f"TRAIN net {params(net)} params"
        + (f", critic {params(critic)}" if critic is not None else ", no-critic")
        + (f", aux {params(aux_head)}" if aux_head is not None else "")
        + f", bs {args.bs}, bf16={amp}, on {dev}",
        flush=True,
    )
    if args.holdout:
        ids = torch.tensor(json.load(open(args.holdout))["ids"], device=dev)
        weight = net.emb.weight
        with torch.no_grad():
            weight[ids] = 0
        weight.register_hook(lambda g: g.index_fill(0, ids, 0))
        print(f"TRAIN holding out {len(ids)} card buckets", flush=True)

    # Separate weight decay for value head to prevent value overfitting
    vh_ids = {id(p) for p in value_head_params(net)}
    base_params = [p for p in net.parameters() if id(p) not in vh_ids]
    vh_params = [p for p in net.parameters() if id(p) in vh_ids]
    extra_params = []
    if critic is not None:
        extra_params += list(critic.parameters())
    if aux_head is not None:
        extra_params += list(aux_head.parameters())
    param_groups = [
        {"params": base_params + extra_params, "weight_decay": 1e-4},
        {"params": vh_params, "weight_decay": args.val_wd},
    ]
    opt = torch.optim.AdamW(param_groups, lr=args.lr, fused=dev.type == "cuda")

    # Decide per-epoch decision budget (auto-stage onto GPU when data spilled to CPU on CUDA!)
    use_staging = (dev.type == "cuda" and where.type == "cpu")
    epoch_samples = args.samples_per_epoch if args.samples_per_epoch > 0 else (2_000_000 if use_staging else len(train_idx))
    est_train_per_ep = min(len(train_idx), int(epoch_samples * 0.91)) if (use_staging or args.samples_per_epoch > 0) else len(train_idx)
    steps = max(1, args.epochs * math.ceil(est_train_per_ep / args.bs))
    sched = torch.optim.lr_scheduler.OneCycleLR(opt, max_lr=args.lr, total_steps=steps + args.epochs * 4, pct_start=0.1)

    train_net = torch.compile(net) if (args.compile and dev.type == "cuda") else net
    ema_sd = {k: v.detach().clone() for k, v in net.state_dict().items()} if args.ema > 0 else None

    rng = np.random.default_rng(0)
    report = {"decisions": int(len(data)), "held_out": int(len(test_idx)), "epochs": []}
    best_critic = (float("inf"), None)
    value_on, best_value = True, float("inf")
    step_count = 0

    for ep in range(args.epochs):
        ep_t0 = time.time()
        if use_staging or (args.samples_per_epoch > 0 and args.samples_per_epoch < len(train_idx)):
            blocks = _select_epoch_blocks(data, epoch_samples, pol_min_run, rng)
            view = data.stage_blocks(blocks, 512, dev)
            # Filter out held-out games on GPU cleanly (zero leakage!)
            v_train = torch.nonzero((view.store["game"] % 10) != 0, as_tuple=False).squeeze(-1)
            perm = v_train[torch.randperm(len(v_train), device=dev)]
            src = view
            on_gpu = True
        else:
            src = data
            on_gpu = (where.type == "cuda")
            if on_gpu:
                t_idx = torch.as_tensor(train_idx, dtype=torch.long, device=dev)
                perm = t_idx[torch.randperm(len(t_idx), device=dev)]
            else:
                perm = rng.permutation(train_idx)

        tl = torch.zeros(5, device=dev)
        nb = math.ceil(len(perm) / args.bs)
        if on_gpu:
            for i in range(0, len(perm), args.bs):
                b = src.batch(perm[i : i + args.bs], dev)
                with torch.autocast("cuda", dtype=torch.bfloat16, enabled=amp):
                    pol, val, bel, crit, aux = losses(train_net, critic, aux_head, b, q_mix=args.q_mix, pol_min_run=pol_min_run)[:5]
                    loss = (
                        pol
                        + (args.value_weight if value_on else 0.0) * val
                        + args.belief_weight * bel
                        + (0.5 * crit if critic is not None else 0.0)
                        + args.aux_weight * aux
                    )
                opt.zero_grad(set_to_none=True)
                loss.backward()
                torch.nn.utils.clip_grad_norm_(net.parameters(), 1.0)
                opt.step()
                if step_count < steps:
                    sched.step()
                step_count += 1
                if ema_sd is not None:
                    d_ema = args.ema
                    with torch.no_grad():
                        for k, v in net.state_dict().items():
                            if v.is_floating_point():
                                ema_sd[k].mul_(d_ema).add_(v.detach(), alpha=1.0 - d_ema)
                            else:
                                ema_sd[k].copy_(v)
                tl += torch.stack([pol.detach(), val.detach(), bel.detach(), crit.detach(), aux.detach()]).float()
        else:
            with ThreadPoolExecutor(1) as pool:
                starts = list(range(0, len(perm), args.bs))
                ahead = pool.submit(src.batch, perm[0 : args.bs], dev)
                for n_i, i in enumerate(starts):
                    b = ahead.result()
                    if n_i + 1 < len(starts):
                        ahead = pool.submit(src.batch, perm[starts[n_i + 1] : starts[n_i + 1] + args.bs], dev)
                    with torch.autocast("cuda", dtype=torch.bfloat16, enabled=amp):
                        pol, val, bel, crit, aux = losses(train_net, critic, aux_head, b, q_mix=args.q_mix, pol_min_run=pol_min_run)[:5]
                        loss = (
                            pol
                            + (args.value_weight if value_on else 0.0) * val
                            + args.belief_weight * bel
                            + (0.5 * crit if critic is not None else 0.0)
                            + args.aux_weight * aux
                        )
                    opt.zero_grad(set_to_none=True)
                    loss.backward()
                    torch.nn.utils.clip_grad_norm_(net.parameters(), 1.0)
                    opt.step()
                    if step_count < steps:
                        sched.step()
                    step_count += 1
                    if ema_sd is not None:
                        d_ema = args.ema
                        with torch.no_grad():
                            for k, v in net.state_dict().items():
                                if v.is_floating_point():
                                    ema_sd[k].mul_(d_ema).add_(v.detach(), alpha=1.0 - d_ema)
                                else:
                                    ema_sd[k].copy_(v)
                    tl += torch.stack([pol.detach(), val.detach(), bel.detach(), crit.detach(), aux.detach()]).float()

        if src is not data:
            del src
            if dev.type == "cuda":
                torch.cuda.empty_cache()

        ev = evaluate(net, critic, data, test_idx, dev, pol_min_run=pol_min_run)
        ev["epoch"] = ep + 1
        ev["epoch_s"] = round(time.time() - ep_t0, 1)
        ev["train"] = (tl / max(nb, 1)).tolist()
        report["epochs"].append(ev)
        if critic is not None and ev["critic_logloss"] < best_critic[0]:
            best_critic = (ev["critic_logloss"], {k: v.detach().clone() for k, v in critic.state_dict().items()})
        print(
            f"TRAIN epoch {ep + 1}: held out {json.dumps({k: round(v, 4) if isinstance(v, float) else v for k, v in ev.items()})} ({time.time() - t0:.1f} s)",
            flush=True,
        )
        if value_on and ev["value_logloss"] > best_value + args.val_stop_tol:
            value_on = False
            report["value_stopped_after_epoch"] = ep
            print(f"TRAIN value stops training the trunk (held-out log loss {ev['value_logloss']:.4f} > {best_value:.4f})", flush=True)
        best_value = min(best_value, ev["value_logloss"])

    if ema_sd is not None:
        net.load_state_dict(ema_sd)
        ev_ema = evaluate(net, critic, data, test_idx, dev, pol_min_run=pol_min_run)
        report["ema_eval"] = ev_ema
        print(f"TRAIN EMA eval: {json.dumps({k: round(v, 4) if isinstance(v, float) else v for k, v in ev_ema.items()})}", flush=True)

    if not value_on:
        refit_value(net, critic, data, train_idx, test_idx, dev, args.bs, report, q_mix=args.q_mix)

    z = data.z[test_idx]
    report["value_coin_logloss"] = math.log(2)
    report["value_draw_share"] = float((z == 0.5).mean())
    ph = np.concatenate([data.pool_hand[data.pool_off[i] : data.pool_off[i + 1]] for i in test_idx[:20000]])
    r = ph.mean()
    report["belief_base_logloss"] = float(-(r * math.log(r) + (1 - r) * math.log(1 - r)))
    save_dir(net, args.out)
    if critic is not None and best_critic[1] is not None:
        critic.load_state_dict(best_critic[1])
        report["critic_best_logloss"] = best_critic[0]
        torch.save(critic.state_dict(), os.path.join(args.out, "critic.pt"))
    export(net.cpu(), os.path.join(args.out, "net.pmnet"), {"trained": "train.py", "data": args.data, "epochs": args.epochs})
    with open(os.path.join(args.out, "report.json"), "w") as f:
        json.dump(report, f, indent=1)
    print(f"TRAIN wrote {args.out}; total {time.time() - t0:.1f} s; belief base log loss {report['belief_base_logloss']:.4f}", flush=True)
    return data


def main():
    train_main()


if __name__ == "__main__":
    main()
