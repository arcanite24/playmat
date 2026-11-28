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
from model import ActQHead, AuxHead, Critic, GLOB_F, NO_ENT, ThreatHead, Tx, build, export, hl_gauss_mean, hl_gauss_targets, load_dir, params, save_dir


def oracle_batch(b):
    """Appends opponent hidden-hand tokens (`hid`, `hid_ids`, `hid_mask`) onto entity tokens for Perfect-Information
    Oracle Critic training/distillation (PerfectDou / Suphx style). Training-only; never exposed to playing nets."""
    if "hid" not in b or b["hid"].shape[1] == 0:
        return b
    out = dict(b)
    pad_ri = torch.full_like(b["hid_ids"], NO_ENT)
    pad_rt = torch.zeros_like(b["hid_ids"])
    out["ents"] = torch.cat([b["ents"], b["hid"]], dim=1)
    out["ent_ids"] = torch.cat([b["ent_ids"], b["hid_ids"]], dim=1)
    out["ent_mask"] = torch.cat([b["ent_mask"], b["hid_mask"]], dim=1)
    out["ent_rel_idx"] = torch.cat([b["ent_rel_idx"], pad_ri], dim=1)
    out["ent_rel_type"] = torch.cat([b["ent_rel_type"], pad_rt], dim=1)
    return out


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


def refit_value(net, critic, data, train_idx, test_idx, dev, bs, report, q_mix=0.0, oracle_net=None, oracle_mix=0.0, recency_decay=1.0, recency_half_life=0):
    """One pass over recent-weighted training decisions that fits only the value head on the frozen trunk."""
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
    opt = torch.optim.AdamW(head_params, lr=5e-4, weight_decay=5e-2)
    rng = np.random.default_rng(1)
    blocks = _select_epoch_blocks(data, min(len(train_idx), 250_000), 0, rng, recency_decay=recency_decay, recency_half_life=recency_half_life)
    cand = np.concatenate([np.arange(s, min(s + 512, len(data))) for s in blocks]) if len(blocks) else train_idx
    cand = cand[(data.game[cand] % 10) != 0]
    perm = rng.permutation(cand)[:200_000]
    # Keep trunk & val_drop deterministic during frozen-trunk linear/MLP value refit
    net.eval()
    amp_rf = dev.type == "cuda"
    for i in range(0, len(perm), bs):
        b = data.batch(perm[i : i + bs], dev)
        with torch.autocast("cuda", dtype=torch.bfloat16, enabled=amp_rf):
            val = losses(
                net, None, None, b,
                q_mix=q_mix, val_prob=1.0, detach_val=True,
                oracle_net=oracle_net, oracle_mix=oracle_mix,
            )[1]
        opt.zero_grad(set_to_none=True)
        val.backward()
        opt.step()
    for p in net.parameters():
        p.requires_grad_(True)
    ev = evaluate(net, critic, data, test_idx, dev, q_mix=0.0)
    report["value_refit_logloss"] = ev["value_logloss"]
    print(f"TRAIN value head refit: held-out log loss {ev['value_logloss']:.4f}", flush=True)


def precompute_oracle_targets(oracle_net, data, dev, bs=2048):
    """Computes frozen Oracle value predictions in (0, 1) once across `data` (eliminates per-epoch Oracle passes)."""
    t0 = time.time()
    oracle_net.eval()
    n = len(data)
    out = np.empty(n, dtype=np.float16)
    amp_or = dev.type == "cuda"
    with torch.no_grad(), torch.autocast("cuda", dtype=torch.bfloat16, enabled=amp_or):
        for i in range(0, n, bs):
            j = np.arange(i, min(i + bs, n))
            b = data.batch_np(j, dev) if data.store is None else data.batch(j, dev)
            v_or = hl_gauss_mean(oracle_net(oracle_batch(b))[1]).clamp(0.01, 0.99)
            out[i : i + len(j)] = v_or.float().cpu().numpy().astype(np.float16)
    print(f"TRAIN cached {n} frozen oracle targets in {time.time() - t0:.1f} s", flush=True)
    return out


@torch.no_grad()
def _zeropower_ns5(G, steps=5):
    """5-step Newton-Schulz polar orthogonalization for Muon (Keller Jordan, 2024). Supports 2D [M, N] or 3D [K, M, N]."""
    a, b, c = 3.4445, -4.7750, 2.0315
    X = G.bfloat16()
    transposed = X.size(-2) > X.size(-1)
    if transposed:
        X = X.transpose(-2, -1)
    norm = X.flatten(-2).norm(dim=-1, keepdim=True).unsqueeze(-1)
    X = X / (norm + 1e-7)
    for _ in range(steps):
        A = X @ X.transpose(-2, -1)
        B = b * A + c * (A @ A)
        X = a * X + B @ X
    if transposed:
        X = X.transpose(-2, -1)
    return X.to(G.dtype)


class MuonHybrid(torch.optim.Optimizer):
    """Muon (Newton-Schulz orthogonalized momentum) on 2D hidden Transformer matrices + AdamW on all other params."""

    def __init__(self, param_groups, momentum=0.95, betas=(0.9, 0.999), eps=1e-8, chunk_map=None):
        defaults = dict(lr=1e-3, weight_decay=1e-4, momentum=momentum, betas=betas, eps=eps, is_muon=False)
        super().__init__(param_groups, defaults)
        self.chunk_map = chunk_map or {}

    @torch.no_grad()
    def step(self, closure=None):
        loss = closure() if closure is not None else None
        for group in self.param_groups:
            lr = group["lr"]
            wd = group["weight_decay"]
            if group.get("is_muon", False):
                mom = group["momentum"]
                for p in group["params"]:
                    if p.grad is None:
                        continue
                    g = p.grad
                    st = self.state[p]
                    if "buf" not in st:
                        st["buf"] = torch.zeros_like(g)
                    buf = st["buf"]
                    buf.mul_(mom).add_(g)
                    u = g.add(buf, alpha=mom)  # Nesterov momentum
                    k_chunks = self.chunk_map.get(id(p), 1)
                    if k_chunks > 1 and u.ndim == 2 and u.size(0) % k_chunks == 0:
                        sub_h = u.size(0) // k_chunks
                        orth = _zeropower_ns5(u.view(k_chunks, sub_h, u.size(1))).reshape_as(u)
                        scale = max(1.0, sub_h / p.size(1)) ** 0.5
                    else:
                        orth = _zeropower_ns5(u)
                        scale = max(1.0, p.size(0) / p.size(1)) ** 0.5
                    if wd > 0:
                        p.mul_(1.0 - lr * wd)
                    p.add_(orth, alpha=-lr * scale)
            else:
                b1, b2 = group["betas"]
                eps = group["eps"]
                for p in group["params"]:
                    if p.grad is None:
                        continue
                    g = p.grad
                    st = self.state[p]
                    if "step" not in st:
                        st["step"] = 0
                        st["m"] = torch.zeros_like(g)
                        st["v"] = torch.zeros_like(g)
                    st["step"] += 1
                    m, v = st["m"], st["v"]
                    m.mul_(b1).add_(g, alpha=1.0 - b1)
                    v.mul_(b2).addcmul_(g, g, value=1.0 - b2)
                    bc1 = 1.0 - b1 ** st["step"]
                    bc2 = 1.0 - b2 ** st["step"]
                    if wd > 0:
                        p.mul_(1.0 - lr * wd)
                    p.addcdiv_(m / bc1, (v / bc2).sqrt().add_(eps), value=-lr)
        return loss


def losses(
    net,
    critic,
    aux_head,
    b,
    q_mix=0.0,
    pol_min_run=0,
    pol_old_weight=0.0,
    val_prob=1.0,
    detach_val=False,
    detach_aux=False,
    oracle_net=None,
    oracle_mix=0.0,
    oracle_mode=False,
    act_q_head=None,
    act_q_weight=0.0,
    opt_pol_weight=0.0,
    opt_ws_ema=None,
    threat_head=None,
    threat_weight=0.0,
):
    raw_net = getattr(net, "_orig_mod", net)
    want_aux = aux_head is not None
    has_net_threat = getattr(raw_net, "threat", None) is not None
    want_ext_threat = (not has_net_threat) and (threat_head is not None) and (threat_weight > 0.0)
    want_vfeat = want_aux or want_ext_threat
    want_aq = act_q_head is not None and act_q_weight > 0.0
    want_threat = (has_net_threat or threat_head is not None) and threat_weight > 0.0
    nb_in = oracle_batch(b) if oracle_mode else b
    emb_mod = getattr(raw_net, "emb", None)
    drop_salt = b.get("_drop_salt")
    if drop_salt is None and net.training and emb_mod is not None and getattr(emb_mod, "text", None) is not None and getattr(emb_mod, "id_drop", 0.0) > 0.0:
        drop_salt = emb_mod.sample_salt(b["glob"].device)
        nb_in = {**nb_in, "_drop_salt": drop_salt}
    if want_aq:
        logits, v, h, vfeat, act_emb = net(nb_in, return_vfeat=True, return_act_emb=True, detach_val=detach_val)
    elif want_vfeat:
        logits, v, h, vfeat = net(nb_in, return_vfeat=True, detach_val=detach_val)
        act_emb = None
    else:
        logits, v, h = net(nb_in, detach_val=detach_val)
        vfeat, act_emb = None, None
    logp = F.log_softmax(logits, -1)
    pol_per = -(b["target"] * logp).sum(-1)
    if pol_min_run > 0:
        if pol_old_weight > 0.0:
            pm = torch.where(b["run"] >= pol_min_run, 1.0, float(pol_old_weight))
        else:
            pm = (b["run"] >= pol_min_run).float()
        pol = (pol_per * pm).sum() / pm.sum().clamp(min=1.0)
    else:
        pol = pol_per.mean()

    # Mix binary/TD outcome with Perfect-Information Oracle value (cached or live, scalar or HL-Gauss) and root search Q
    z_tgt = b["z_td"]
    if oracle_mix > 0.0:
        if "oracle_v" in b and b["oracle_v"] is not None:
            ov = b["oracle_v"]
            ov_safe = torch.nan_to_num(ov.float(), nan=0.5).clamp(0.01, 0.99)
            z_tgt = torch.where(torch.isfinite(ov), (1.0 - oracle_mix) * z_tgt + oracle_mix * ov_safe, z_tgt)
        elif oracle_net is not None:
            with torch.no_grad():
                v_or = hl_gauss_mean(oracle_net(oracle_batch(b))[1]).clamp(0.01, 0.99)
            z_tgt = (1.0 - oracle_mix) * z_tgt + oracle_mix * v_or
    if q_mix > 0.0:
        qv = b["qval"]
        has_q = torch.isfinite(qv)
        qv_safe = torch.nan_to_num(qv, nan=0.5).clamp(0.0, 1.0)
        z_tgt = torch.where(has_q, (1.0 - q_mix) * z_tgt + q_mix * qv_safe, z_tgt)
    if v.ndim == 2:
        hl_p = hl_gauss_targets(z_tgt, n_bins=v.shape[-1], sigma_ratio=0.75)
        val_per = -(hl_p * F.log_softmax(v.float(), dim=-1)).sum(-1)
    else:
        val_per = F.binary_cross_entropy_with_logits(v, z_tgt, reduction="none")

    if opt_pol_weight > 0.0 and "qval" in b:
        qv_o = b["qval"]
        has_qo = torch.isfinite(qv_o)
        v_est = hl_gauss_mean(v.detach())
        adv = (torch.nan_to_num(qv_o, nan=0.5).clamp(0.0, 1.0) - v_est).clamp(min=0.0)
        w_opt = torch.where(has_qo, adv * adv, torch.zeros_like(adv))
        if pol_min_run > 0:
            w_opt = w_opt * (b["run"] >= pol_min_run).float()
        mean_w = w_opt.mean()
        if opt_ws_ema is not None:
            opt_ws_ema.lerp_(mean_w.detach().float(), 0.05)
            denom = torch.maximum(opt_ws_ema, mean_w.detach().float()).clamp(min=1e-4)
        else:
            denom = mean_w.detach().float().clamp(min=1e-4)
        pol = pol + float(opt_pol_weight) * ((pol_per * w_opt).mean() / denom)

    vw = b["val_w"]
    if val_prob < 1.0 and net.training:
        vm = (torch.rand_like(vw) < val_prob).float()
        vw = vw * vm
    val = (val_per * vw).sum() / vw.sum().clamp(min=1e-6)

    bl = raw_net.belief_logits(h, b["pool"], drop_salt=drop_salt)
    m = b["pool_mask"].float()
    bel = (F.binary_cross_entropy_with_logits(bl, b["pool_hand"], reduction="none") * m).sum() / m.sum().clamp(min=1.0)

    crit = F.binary_cross_entropy_with_logits(critic(b), b["z"]) if critic is not None else torch.zeros((), device=v.device)
    if want_aux:
        aux_pred = aux_head(vfeat.detach() if detach_val else vfeat)
        aux_tgt = torch.stack(
            [b["turns_left"], b["final_life_diff"], b["next_life_delta"], b["next_board_delta"]], -1
        )
        aux_per = F.smooth_l1_loss(aux_pred, aux_tgt, reduction="none").mean(-1)
        aux = (aux_per * vw).sum() / vw.sum().clamp(min=1e-6)
    else:
        aux = torch.zeros((), device=v.device)

    if want_aq and "act_q" in b:
        aq = b["act_q"]
        aq_m = (torch.isfinite(aq) & b["act_mask"]).float()
        aq_pred = act_q_head(act_emb)
        aq_safe = torch.nan_to_num(aq, nan=0.5).clamp(0.0, 1.0)
        aq_loss = (F.binary_cross_entropy_with_logits(aq_pred, aq_safe, reduction="none") * aq_m).sum() / aq_m.sum().clamp(min=1.0)
    else:
        aq_loss = torch.zeros((), device=v.device)

    if want_threat and "belief_meta" in b:
        bm = b["belief_meta"]
        bm_m = torch.isfinite(bm).float()
        th_pred = raw_net.threat(h) if has_net_threat else threat_head(vfeat.detach() if detach_aux else vfeat)
        bm_safe = torch.nan_to_num(bm, nan=0.0).clamp(0.0, 1.0)
        th_loss = (F.binary_cross_entropy_with_logits(th_pred, bm_safe, reduction="none") * bm_m).sum() / bm_m.sum().clamp(min=1.0)
    else:
        th_loss = torch.zeros((), device=v.device)

    right = (logits.argmax(-1) == b["target"].argmax(-1)).float()
    off = (b["target"].argmax(-1) != 0).float()
    acc = right.mean()
    # accuracy where the recorded policy didn't take the quick answer (candidate 0)
    acc_off = (right * off).sum() / off.sum().clamp(min=1.0)
    return pol, val, bel, crit, aux, acc, acc_off, 1 - off.mean(), aq_loss, th_loss


def evaluate(net, critic, data, idx, dev, bs=1024, q_mix=0.0, pol_min_run=0, oracle_mode=False):
    net.eval()
    tot = np.zeros(8)
    n = 0
    rec_pol_sum, rec_right_nq, rec_off_sum, rec_n = 0.0, 0.0, 0.0, 0.0
    # Cap eval set at 150k decisions for speed on huge multi-round replay buffers
    if len(idx) > 150_000:
        step = max(len(idx) // 150_000, 1)
        idx = idx[::step]
    amp_ev = dev.type == "cuda"
    with torch.no_grad(), torch.autocast("cuda", dtype=torch.bfloat16, enabled=amp_ev):
        for i in range(0, len(idx), bs):
            j = idx[i : i + bs]
            b = data.batch(j, dev)
            # Evaluate value_logloss against true game outcome z (q_mix=0) and unweighted
            nb_in = oracle_batch(b) if oracle_mode else b
            logits, v, h = net(nb_in)
            logp = F.log_softmax(logits, -1)
            pol_per = -(b["target"] * logp).sum(-1)
            right = (logits.argmax(-1) == b["target"].argmax(-1)).float()
            off = (b["target"].argmax(-1) != 0).float()
            if pol_min_run > 0:
                pm = (b["run"] >= pol_min_run).float()
                rec_pol_sum += float((pol_per * pm).sum())
                rec_right_nq += float((right * off * pm).sum())
                rec_off_sum += float((off * pm).sum())
                rec_n += float(pm.sum())
                pol = (pol_per * pm).sum() / pm.sum().clamp(min=1)
            else:
                pol = pol_per.mean()
            if v.ndim == 2:
                v_mu = hl_gauss_mean(v).clamp(1e-6, 1.0 - 1e-6)
                with torch.amp.autocast("cuda", enabled=False):
                    val = F.binary_cross_entropy(v_mu.float().clamp(1e-6, 1.0 - 1e-6), b["z"].float())
            else:
                val = F.binary_cross_entropy_with_logits(v, b["z"])
            bl = net.belief_logits(h, b["pool"])
            m = b["pool_mask"].float()
            bel = (F.binary_cross_entropy_with_logits(bl, b["pool_hand"], reduction="none") * m).sum() / m.sum().clamp(min=1)
            crit = F.binary_cross_entropy_with_logits(critic(b), b["z"]) if critic is not None else torch.zeros((), device=v.device)
            acc = right.mean()
            acc_off = (right * off).sum() / off.sum().clamp(min=1)
            out = (pol, val, bel, crit, torch.zeros(()), acc, acc_off, 1 - off.mean())
            tot += np.array([float(x) for x in out]) * len(j)
            n += len(j)
    net.train()
    pol, val, bel, crit, _, acc, acc_off, quick = tot / max(n, 1)
    res = {
        "policy_ce": (rec_pol_sum / rec_n) if (pol_min_run > 0 and rec_n > 0) else pol,
        "value_logloss": val, "belief_logloss": bel, "critic_logloss": crit,
        "accuracy": acc, "accuracy_not_quick": acc_off, "teacher_picks_quick": quick,
    }
    if pol_min_run > 0 and rec_off_sum > 0:
        res["recent_acc_nq"] = rec_right_nq / rec_off_sum
    return res


def _select_epoch_blocks(data, samples_target, pol_min_run, rng, block_len=512, recency_decay=0.92, recency_half_life=0):
    """Picks block start indices totaling ~`samples_target` decisions strictly WITHOUT replacement (hard <= 1.0x/epoch
    pass cap per block) using cumulative-decision-age exponential weighting so multi-sub-dir rounds are not over-decayed."""
    n_total = len(data)
    n_blocks = max(1, (n_total + block_len - 1) // block_len)
    want_blocks = max(1, min(n_blocks, (samples_target + block_len - 1) // block_len))
    starts = np.arange(0, n_total, block_len)
    if want_blocks >= len(starts):
        return starts
    centers = np.minimum(starts + block_len * 0.5, float(n_total))
    decisions_newer = np.maximum(0.0, float(n_total) - centers)
    if recency_half_life > 0:
        w = np.power(0.5, decisions_newer / float(recency_half_life))
    elif 0.0 < recency_decay < 1.0:
        # Decay per 100k decisions (~1 nominal self-play generation) rather than raw --data CLI directory index
        w = np.power(recency_decay, decisions_newer / 100_000.0)
    else:
        w = np.ones(len(starts), dtype=np.float64)
    w = w / w.sum()
    chosen = rng.choice(starts, size=want_blocks, replace=False, p=w)
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
    ap.add_argument("--act-q-weight", type=float, default=0.2, help="training-only v3 per-action Q head loss weight (0 disables)")
    ap.add_argument("--recency-decay", type=float, default=0.92, help="exponential recency decay per 100k newer decisions (1.0 = uniform; <= 1x/epoch)")
    ap.add_argument("--recency-half-life", type=int, default=0, help="optional explicit half-life in newer decisions (e.g. 750000; overrides --recency-decay)")
    ap.add_argument("--q-mix", type=float, default=0.4, help="weight on root search Q-value in value target when present")
    ap.add_argument("--td-lambda", type=float, default=0.0, help="forward-looking TD(lambda) value target across future search Q-values (0 = plain z)")
    ap.add_argument("--policy-recent", type=int, default=0, help="only train policy on the last N --data dirs (0 = all)")
    ap.add_argument("--pol-old-weight", type=float, default=0.0, help="policy weight for older dirs when --policy-recent > 0 (e.g. 0.25 prevents overfitting)")
    ap.add_argument("--oracle-mode", action="store_true", help="train Perfect-Information Oracle Tx seeing opponent hidden hand (never exported as .pmnet)")
    ap.add_argument("--oracle-dir", default=None, help="frozen Perfect-Information Oracle Tx dir for value distillation")
    ap.add_argument("--oracle-mix", type=float, default=0.35, help="weight on oracle value prediction in value target when --oracle-dir is set")
    ap.add_argument("--cache-oracle", nargs="?", const=1, type=int, default=1, help="precompute frozen --oracle-dir targets once before epoch 1 (1=on, 0=off)")
    ap.add_argument("--samples-per-epoch", type=int, default=0, help="cap/stage this many decisions per epoch onto GPU (0 = all)")
    ap.add_argument("--max-per-dir", type=int, default=0, help="cap decisions read per recent --data dir (0 = all)")
    ap.add_argument("--max-old-per-dir", type=int, default=0, help="cap decisions read per older (value-only) --data dir (0 = all)")
    ap.add_argument("--no-critic", action="store_true", help="skip loading hid and training the PPO Critic")
    ap.add_argument("--upgrade-val-mlp", action="store_true", help="upgrade a v1 Tx checkpoint to the regularized 2-layer val_mlp head")
    ap.add_argument("--upgrade-glob-f", type=int, default=0, help="zero-loss expand glob_f on --init warm-start (e.g. 89)")
    ap.add_argument("--upgrade-val-bins", type=int, default=0, help="upgrade scalar value head to HL-Gauss N bins (e.g. 21)")
    ap.add_argument("--threat-weight", type=float, default=0.0, help="v4 hidden-hand 8-threat BCE auxiliary loss weight")
    ap.add_argument("--opt-pol-weight", type=float, default=0.0, help="KataGo optimistic surprise-weighted policy loss weight")
    ap.add_argument("--opt", default="adamw", choices=["adamw", "muon"], help="optimizer: adamw or muon (2D hidden matrices)")
    ap.add_argument("--muon-lr", type=float, default=0.012, help="max lr for Muon 2D matrix group")
    ap.add_argument("--drop", type=float, default=0.0, help="training-only dropout inside Tx blocks")
    ap.add_argument("--card-id-drop", type=float, default=0.0, help="training-only probability of dropping learned card-ID residual while keeping BGE card-text projection")
    ap.add_argument("--val-wd", type=float, default=5e-2, help="weight decay on value head parameters")
    ap.add_argument("--val-prob", type=float, default=0.50, help="per-decision probability of including value/aux loss")
    ap.add_argument("--detach-val", action="store_true", help="stop value/aux gradients from entering the shared transformer trunk")
    ap.add_argument("--val-stop-tol", type=float, default=0.004, help="held-out value loss rise before freezing trunk value grad")
    ap.add_argument("--no-refit-value", action="store_true", help="never reset/refit the value head at end of training")
    ap.add_argument("--force-refit-value", action="store_true", help="force end-of-run value refit even when EMA or Oracle distillation is active")
    ap.add_argument("--ema", type=float, default=0.0, help="EMA decay for exported weights (e.g. 0.995; 0 disables)")
    ap.add_argument("--amp", type=int, default=-1, help="1: bf16 autocast on GPU; -1: auto at bs >= 1024")
    ap.add_argument("--gpu-data-gb", type=float, default=5.0, help="keep whole dataset on GPU when <= this many GB")
    ap.add_argument("--pin-cpu", type=int, default=1, help="1: pin CPU dataset tensors on CUDA when not staging")
    ap.add_argument("--stage-chunk", type=int, default=800_000, help="max decisions staged to GPU at once (prevents WDDM VRAM spill)")
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
    need_hid = (not args.no_critic) or args.oracle_mode or bool(args.oracle_dir)
    if shared_data is not None:
        data = shared_data
    else:
        data = Data(
            args.data,
            load_critic=need_hid,
            max_per_dir=args.max_per_dir,
            max_old_per_dir=args.max_old_per_dir,
            policy_recent=args.policy_recent,
            td_lambda=args.td_lambda,
        )
        print(f"TRAIN {len(data)} decisions from {len(data.infos)} games, {time.time() - t0:.1f} s to read", flush=True)

    oracle_net = None
    if args.oracle_dir and args.oracle_mix > 0.0:
        if getattr(data, "oracle_v", None) is not None and bool(args.cache_oracle):
            print(f"TRAIN reusing pre-cached oracle targets on shared_data (mix={args.oracle_mix})", flush=True)
        else:
            oracle_net = load_dir(args.oracle_dir, dev).eval()
            for p in oracle_net.parameters():
                p.requires_grad_(False)
            print(f"TRAIN oracle distillation from {args.oracle_dir} ({params(oracle_net)} params, mix={args.oracle_mix})", flush=True)
            if args.cache_oracle:
                data.oracle_v = precompute_oracle_targets(oracle_net, data, dev, bs=max(args.bs, 2048))
                del oracle_net
                oracle_net = None
                if shared_data is None and args.no_critic and not args.oracle_mode:
                    data.load_critic = False
                    data.hid = np.zeros((0, data.ents.shape[1]), dtype=np.float16)
                    data.hid_ids = np.zeros(0, dtype=np.uint16)
                    data.n_hid = np.zeros_like(data.n_hid)
                    data.hid_off = np.zeros(len(data) + 1, dtype=np.int64)
                if dev.type == "cuda":
                    torch.cuda.empty_cache()

    where, size = data.place(dev, args.gpu_data_gb, pin_cpu=bool(args.pin_cpu))
    print(f"TRAIN data {size:.1f} GB on {where} (pinned={data._pinned})", flush=True)
    amp = (args.amp == 1 or (args.amp == -1 and args.bs >= 1024)) and dev.type == "cuda"
    pol_min_run = max(0, len(args.data) - args.policy_recent) if args.policy_recent > 0 else 0

    held = (data.game % 10) == 0
    train_idx, test_idx = np.nonzero(~held)[0], np.nonzero(held)[0]
    critic = None if args.no_critic else Critic().to(dev)
    if args.init:
        net = load_dir(args.init, dev, upgrade_val_mlp=args.upgrade_val_mlp, drop=args.drop, upgrade_glob_f=args.upgrade_glob_f, upgrade_val_bins=args.upgrade_val_bins, upgrade_threat=(args.threat_weight > 0))
        if critic is not None:
            cpath = os.path.join(args.init, "critic.pt")
            if os.path.exists(cpath):
                critic.load_state_dict(torch.load(cpath, map_location=dev))
    else:
        kw = tx_sizes(args.tx) if args.arch == "tx" else {}
        if args.arch == "tx" and args.drop > 0:
            kw["drop"] = args.drop
        if args.arch == "tx" and args.threat_weight > 0:
            kw["threat_head"] = 1
        net = build({"arch": args.arch, **kw}).to(dev)

    aux_head = None
    if args.aux_weight > 0:
        aux_in = (3 * net.d + net.glob_f) if isinstance(net, Tx) else net.trunk2.out_features
        aux_head = AuxHead(aux_in, out_dim=4).to(dev)
    act_q_head = None
    if args.act_q_weight > 0:
        aq_in = net.d if isinstance(net, Tx) else net.act2.out_features
        act_q_head = ActQHead(aq_in).to(dev)
    threat_head = None
    if args.threat_weight > 0 and getattr(net, "threat", None) is None:
        th_in = (3 * net.d + net.glob_f) if isinstance(net, Tx) else net.trunk2.out_features
        threat_head = ThreatHead(th_in).to(dev)

    print(
        f"TRAIN net {params(net)} params"
        + (f", critic {params(critic)}" if critic is not None else ", no-critic")
        + (f", aux {params(aux_head)}" if aux_head is not None else "")
        + f", bs {args.bs}, bf16={amp}, on {dev}",
        flush=True,
    )
    if args.card_id_drop > 0 and hasattr(net, "emb") and hasattr(net.emb, "id_drop"):
        net.emb.id_drop = float(args.card_id_drop)
        print(f"TRAIN card_id_drop={net.emb.id_drop:.2f} (inverted per-card-id dropout; BGE text projection always kept)", flush=True)
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
    if act_q_head is not None:
        extra_params += list(act_q_head.parameters())
    if threat_head is not None:
        extra_params += list(threat_head.parameters())
    if args.opt == "muon":
        skip_prefixes = ("emb.", "glob_in.", "ent_in.", "act_in.", "belief.", "threat.", "abias.", "query.")
        muon_p, adam_base, chunk_map = [], [], {}
        for n_p, p in net.named_parameters():
            if id(p) in vh_ids:
                continue
            if p.ndim == 2 and not n_p.startswith(skip_prefixes) and min(p.shape) >= 32:
                muon_p.append(p)
                if n_p.endswith("qkv.weight"):
                    chunk_map[id(p)] = 3
                elif n_p.endswith("act_kv.weight"):
                    chunk_map[id(p)] = 2
            else:
                adam_base.append(p)
        param_groups = [
            {"params": muon_p, "lr": args.muon_lr, "weight_decay": 1e-4, "is_muon": True},
            {"params": adam_base + extra_params, "lr": args.lr, "weight_decay": 1e-4, "is_muon": False},
            {"params": vh_params, "lr": args.lr, "weight_decay": args.val_wd, "is_muon": False},
        ]
        opt = MuonHybrid(param_groups, chunk_map=chunk_map)
        max_lrs = [args.muon_lr, args.lr, args.lr]
        print(f"TRAIN optimizer=MuonHybrid (muon_2d={len(muon_p)} tensors, split_qkv={len(chunk_map)} @ lr={args.muon_lr}, adamw @ lr={args.lr})", flush=True)
    else:
        param_groups = [
            {"params": base_params + extra_params, "weight_decay": 1e-4},
            {"params": vh_params, "weight_decay": args.val_wd},
        ]
        opt = torch.optim.AdamW(param_groups, lr=args.lr, fused=dev.type == "cuda")
        max_lrs = args.lr

    # Decide per-epoch decision budget (auto-stage onto GPU when data spilled to CPU on CUDA!)
    use_staging = (dev.type == "cuda" and where.type == "cpu")
    epoch_samples = args.samples_per_epoch if args.samples_per_epoch > 0 else (2_000_000 if use_staging else len(train_idx))
    est_train_per_ep = min(len(train_idx), int(epoch_samples * 0.91)) if (use_staging or args.samples_per_epoch > 0) else len(train_idx)
    steps = max(1, args.epochs * math.ceil(est_train_per_ep / args.bs))
    sched = torch.optim.lr_scheduler.OneCycleLR(opt, max_lr=max_lrs, total_steps=steps + args.epochs * 4, pct_start=0.1)

    train_net = torch.compile(net) if (args.compile and dev.type == "cuda") else net
    ema_sd = {k: v.detach().clone() for k, v in net.state_dict().items()} if args.ema > 0 else None
    ema_fp_dst, ema_fp_src, ema_int_pairs = [], [], []
    if ema_sd is not None:
        live_map = dict(net.named_parameters())
        live_map.update(dict(net.named_buffers()))
        for k, v in live_map.items():
            if k not in ema_sd:
                continue
            if v.is_floating_point():
                ema_fp_dst.append(ema_sd[k])
                ema_fp_src.append(v)
            else:
                ema_int_pairs.append((ema_sd[k], v))

    def _step_ema():
        if ema_sd is None:
            return
        with torch.no_grad():
            if ema_fp_dst:
                torch._foreach_lerp_(ema_fp_dst, [t.detach() for t in ema_fp_src], 1.0 - args.ema)
            for dst_t, src_t in ema_int_pairs:
                dst_t.copy_(src_t)

    opt_ws_ema = torch.tensor(0.01, device=dev, dtype=torch.float32) if args.opt_pol_weight > 0 else None
    all_clip_params = list(net.parameters()) + extra_params

    rng = np.random.default_rng(0)
    report = {"decisions": int(len(data)), "held_out": int(len(test_idx)), "epochs": []}
    best_critic = (float("inf"), None)
    value_on, best_value = True, float("inf")
    step_count = 0

    def _train_step(b, tl_acc):
        nonlocal step_count
        with torch.autocast("cuda", dtype=torch.bfloat16, enabled=amp):
            out_l = losses(
                train_net, critic, aux_head, b,
                q_mix=args.q_mix, pol_min_run=pol_min_run, pol_old_weight=args.pol_old_weight,
                val_prob=args.val_prob, detach_val=(args.detach_val or not value_on), detach_aux=args.detach_val,
                oracle_net=oracle_net, oracle_mix=args.oracle_mix, oracle_mode=args.oracle_mode,
                act_q_head=act_q_head, act_q_weight=args.act_q_weight,
                opt_pol_weight=args.opt_pol_weight, opt_ws_ema=opt_ws_ema,
                threat_head=threat_head, threat_weight=args.threat_weight,
            )
            pol, val, bel, crit, aux = out_l[:5]
            aq_loss, th_loss = out_l[8], out_l[9]
            loss = (
                pol
                + args.value_weight * val
                + args.belief_weight * bel
                + (0.5 * crit if critic is not None else 0.0)
                + (args.aux_weight if value_on else 0.0) * aux
                + args.act_q_weight * aq_loss
                + args.threat_weight * th_loss
            )
        opt.zero_grad(set_to_none=True)
        loss.backward()
        torch.nn.utils.clip_grad_norm_(all_clip_params, 1.0)
        opt.step()
        if step_count < steps:
            sched.step()
        step_count += 1
        _step_ema()
        tl_acc += torch.stack([pol.detach(), val.detach(), bel.detach(), crit.detach(), aux.detach(), aq_loss.detach(), th_loss.detach()]).float()

    for ep in range(args.epochs):
        ep_t0 = time.time()
        tl = torch.zeros(7, device=dev)
        nb = 0
        if use_staging or (args.samples_per_epoch > 0 and args.samples_per_epoch < len(train_idx)):
            blocks = _select_epoch_blocks(
                data, epoch_samples, pol_min_run, rng,
                recency_decay=args.recency_decay, recency_half_life=args.recency_half_life,
            )
            rng.shuffle(blocks)
            chunk_blks = max(1, args.stage_chunk // 512) if args.stage_chunk > 0 else len(blocks)
            for ci in range(0, len(blocks), chunk_blks):
                view = data.stage_blocks(blocks[ci : ci + chunk_blks], 512, dev)
                # Filter out held-out games on GPU cleanly (zero leakage!)
                v_train = torch.nonzero((view.store["game"] % 10) != 0, as_tuple=False).squeeze(-1)
                perm = v_train[torch.randperm(len(v_train), device=dev)]
                for i in range(0, len(perm), args.bs):
                    b = view.batch(perm[i : i + args.bs], dev)
                    _train_step(b, tl)
                    nb += 1
                del view, v_train, perm
                if dev.type == "cuda":
                    torch.cuda.empty_cache()
        elif where.type == "cuda":
            t_idx = torch.as_tensor(train_idx, dtype=torch.long, device=dev)
            perm = t_idx[torch.randperm(len(t_idx), device=dev)]
            nb = math.ceil(len(perm) / args.bs)
            for i in range(0, len(perm), args.bs):
                b = data.batch(perm[i : i + args.bs], dev)
                _train_step(b, tl)
        else:
            perm = rng.permutation(train_idx)
            nb = math.ceil(len(perm) / args.bs)
            trim_cpu = None if not args.compile else False
            with ThreadPoolExecutor(1) as pool:
                starts = list(range(0, len(perm), args.bs))
                ahead = pool.submit(data.batch, perm[0 : args.bs], dev, trim_cpu)
                for n_i, i in enumerate(starts):
                    b = ahead.result()
                    if n_i + 1 < len(starts):
                        ahead = pool.submit(data.batch, perm[starts[n_i + 1] : starts[n_i + 1] + args.bs], dev, trim_cpu)
                    _train_step(b, tl)

        ev = evaluate(net, critic, data, test_idx, dev, pol_min_run=pol_min_run, oracle_mode=args.oracle_mode)
        ev["epoch"] = ep + 1
        ev["epoch_s"] = round(time.time() - ep_t0, 1)
        t_means = (tl / max(nb, 1)).tolist()
        ev["train"] = t_means[:5]
        ev["train_act_q"] = t_means[5]
        ev["train_threat"] = t_means[6]
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
        ev_ema = evaluate(net, critic, data, test_idx, dev, pol_min_run=pol_min_run, oracle_mode=args.oracle_mode)
        report["ema_eval"] = ev_ema
        print(f"TRAIN EMA eval: {json.dumps({k: round(v, 4) if isinstance(v, float) else v for k, v in ev_ema.items()})}", flush=True)

    # Guard against clobbering EMA / Oracle-distilled / recency-weighted value heads with a fresh uniform refit
    allow_refit = args.force_refit_value or (
        not args.no_refit_value
        and ema_sd is None
        and not bool(args.oracle_dir)
        and not (0.0 < args.recency_decay < 1.0 or args.recency_half_life > 0)
    )
    if not value_on and not args.oracle_mode and allow_refit:
        refit_value(
            net, critic, data, train_idx, test_idx, dev, args.bs, report,
            q_mix=args.q_mix, oracle_net=oracle_net, oracle_mix=args.oracle_mix,
            recency_decay=args.recency_decay, recency_half_life=args.recency_half_life,
        )

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
    if not args.oracle_mode:
        export(net.cpu(), os.path.join(args.out, "net.pmnet"), {"trained": "train.py", "data": args.data, "epochs": args.epochs})
    with open(os.path.join(args.out, "report.json"), "w") as f:
        json.dump(report, f, indent=1)
    print(f"TRAIN wrote {args.out}; total {time.time() - t0:.1f} s; belief base log loss {report['belief_base_logloss']:.4f}", flush=True)
    return data


def main():
    train_main()


if __name__ == "__main__":
    main()
