"""The policy, value and belief network: the same as playmat-ai's `net.rs`, in PyTorch, plus a critic that sees the
opponent's hand (training only, never exported).

Shapes come from `obs.rs`: ENT_F entity features, GLOB_F global ones, ACT_F per action, BUCKETS card-name buckets.
"""

import json
import math
import struct

import torch
import torch.nn as nn
import torch.nn.functional as F

ENT_F = 49
GLOB_F_V1 = 65
GLOB_F_V2 = 81
GLOB_F_V3 = 89
GLOB_F = 97
BELIEF_META_F = 8
ACT_F = 65
BUCKETS = 4096
NUM_REL_TYPES = 4
NO_ENT = 255


class Net(nn.Module):
    def __init__(self, dim=16, ent=64, glob=64, trunk=128, act=64, belief=True):
        super().__init__()
        self.emb = nn.Embedding(BUCKETS, dim)
        self.ent1 = nn.Linear(dim + ENT_F, ent)
        self.ent2 = nn.Linear(ent, ent)
        self.glob1 = nn.Linear(GLOB_F, glob)
        self.trunk1 = nn.Linear(2 * ent + glob, trunk)
        self.trunk2 = nn.Linear(trunk, trunk)
        self.value = nn.Linear(trunk, 1)
        self.act1 = nn.Linear(dim + ACT_F, act)
        self.act2 = nn.Linear(act, act)
        self.query = nn.Linear(trunk, act)
        self.abias = nn.Linear(act, 1)
        self.belief = nn.Linear(trunk, dim) if belief else None

    def pool(self, ids, feats, mask):
        """Entities [B, N] ids, [B, N, ENT_F] feats, [B, N] mask -> [B, 2*ent] (mean, max; zeros when empty)"""
        x = torch.cat([self.emb(ids), feats], -1)
        z = F.relu(self.ent2(F.relu(self.ent1(x))))
        m = mask.unsqueeze(-1).float()
        n = m.sum(1).clamp(min=1)
        mean = (z * m).sum(1) / n
        mx = torch.where(m > 0, z, torch.full_like(z, -1e9)).max(1).values
        mx = torch.where(m.sum(1) > 0, mx, torch.zeros_like(mx))
        return torch.cat([mean, mx], -1)

    def state(self, b):
        p = self.pool(b["ent_ids"], b["ents"], b["ent_mask"])
        g = F.relu(self.glob1(b["glob"]))
        return F.relu(self.trunk2(F.relu(self.trunk1(torch.cat([p, g], -1)))))

    def forward(self, b, return_vfeat=False, return_act_emb=False, detach_val=False):
        """-> logits [B, K] (masked to -inf past each row's actions), value logit [B], state h [B, trunk]"""
        h = self.state(b)
        v = self.value(h.detach() if detach_val else h).squeeze(-1)
        a = self.act2(F.relu(self.act1(torch.cat([self.emb(b["act_ids"]), b["acts"]], -1))))
        q = self.query(h).unsqueeze(1)
        logits = (q * a).sum(-1) + self.abias(a).squeeze(-1)
        logits = logits.masked_fill(~b["act_mask"], -1e9)
        if return_act_emb:
            return logits, v, h, h, a
        if return_vfeat:
            return logits, v, h, h
        return logits, v, h

    def belief_logits(self, h, pool_ids):
        """[B, trunk], [B, P] -> [B, P]"""
        q = self.belief(h).unsqueeze(1)
        return (q * self.emb(pool_ids)).sum(-1)


class Critic(nn.Module):
    """A value network that also sees the opponent's hand (PerfectDou's critic): training only"""

    def __init__(self, dim=16, ent=64, glob=64, trunk=128):
        super().__init__()
        self.emb = nn.Embedding(BUCKETS, dim)
        self.ent1 = nn.Linear(dim + ENT_F, ent)
        self.ent2 = nn.Linear(ent, ent)
        self.hid1 = nn.Linear(dim + ENT_F, ent)
        self.hid2 = nn.Linear(ent, ent)
        self.glob1 = nn.Linear(GLOB_F, glob)
        self.trunk1 = nn.Linear(4 * ent + glob, trunk)
        self.trunk2 = nn.Linear(trunk, trunk)
        self.value = nn.Linear(trunk, 1)

    def _pool(self, l1, l2, ids, feats, mask):
        z = F.relu(l2(F.relu(l1(torch.cat([self.emb(ids), feats], -1)))))
        m = mask.unsqueeze(-1).float()
        mean = (z * m).sum(1) / m.sum(1).clamp(min=1)
        mx = torch.where(m > 0, z, torch.full_like(z, -1e9)).max(1).values
        mx = torch.where(m.sum(1) > 0, mx, torch.zeros_like(mx))
        return torch.cat([mean, mx], -1)

    def forward(self, b):
        p = self._pool(self.ent1, self.ent2, b["ent_ids"], b["ents"], b["ent_mask"])
        q = self._pool(self.hid1, self.hid2, b["hid_ids"], b["hid"], b["hid_mask"])
        g = F.relu(self.glob1(b["glob"]))
        h = F.relu(self.trunk2(F.relu(self.trunk1(torch.cat([p, q, g], -1)))))
        return self.value(h).squeeze(-1)


class CardEmb(nn.Module):
    """The name embedding, plus (with `text`) a learned projection of the fixed card-text rows (`cardtext.py`):
    `weight[id] + text[id] @ proj`. `export` writes the sum as one table, so `net.rs` sees a plain embedding."""

    def __init__(self, dim, text=0):
        super().__init__()
        self.weight = nn.Parameter(torch.randn(BUCKETS, dim))
        self.id_drop = 0.0
        if text:
            import numpy as np
            import os

            fname = "cardtext_sem.npy" if int(text) == 2 else "cardtext.npy"
            rows = np.load(os.path.join(os.path.dirname(os.path.abspath(__file__)), "data", fname))
            self.register_buffer("text", torch.from_numpy(rows.astype("float32")))
            self.proj = nn.Parameter(torch.zeros(rows.shape[1], dim))
        else:
            self.text = None

    def table(self):
        return self.weight if self.text is None else self.weight + self.text @ self.proj

    def forward(self, ids):
        w = F.embedding(ids, self.weight)
        if self.text is None:
            return w
        if self.training and self.id_drop > 0.0:
            keep = (torch.rand(ids.shape, device=ids.device) >= self.id_drop).to(w.dtype).unsqueeze(-1)
            w = w * keep
        return w + F.embedding(ids, self.text) @ self.proj


class AuxHead(nn.Module):
    """Training-only KataGo-style auxiliary positional head; never exported.
    Predicts 4 dense targets: [turns_left, final_life_diff, next_life_delta, next_board_delta]."""

    def __init__(self, in_dim, hidden=128, out_dim=4):
        super().__init__()
        self.fc1 = nn.Linear(in_dim, hidden)
        self.fc2 = nn.Linear(hidden, out_dim)

    def forward(self, x):
        return self.fc2(F.gelu(self.fc1(x)))


class ThreatHead(nn.Module):
    """Training-only 8-target hidden-hand threat BCE auxiliary head for v4 shards (`belief_meta.f16`); never exported."""

    def __init__(self, in_f, out_dim=BELIEF_META_F):
        super().__init__()
        self.net = nn.Sequential(nn.Linear(in_f, 128), nn.GELU(), nn.Linear(128, out_dim))

    def forward(self, h):
        return self.net(h)


class ActQHead(nn.Module):
    """Training-only per-action Q-value head for v3 shards (`act_q.f16`); never exported."""

    def __init__(self, in_dim, hidden=128):
        super().__init__()
        self.fc1 = nn.Linear(in_dim, hidden)
        self.fc2 = nn.Linear(hidden, 1)

    def forward(self, a):
        return self.fc2(F.gelu(self.fc1(a))).squeeze(-1)


def _gelu(x):
    return F.gelu(x, approximate="tanh")




def hl_gauss_targets(z, n_bins=21, sigma_ratio=0.75):
    """Exact HL-Gauss categorical target distribution over [0, 1] via Gaussian CDF differences (Stop Regressing, ICML 2024)."""
    edges = torch.linspace(0.0, 1.0, n_bins + 1, device=z.device, dtype=torch.float32)
    sigma = float(sigma_ratio) / float(n_bins - 1)
    zc = z.float().clamp(0.0, 1.0).unsqueeze(-1)
    cdf = 0.5 * (1.0 + torch.erf((edges.unsqueeze(0) - zc) / (sigma * (2.0 ** 0.5))))
    p = (cdf[..., 1:] - cdf[..., :-1]).clamp(min=0.0)
    return p / p.sum(dim=-1, keepdim=True).clamp(min=1e-8)


def hl_gauss_mean(v_logits):
    """Expected value in (0, 1) from HL-Gauss categorical value logits [B, K], or sigmoid(v) for scalar [B]."""
    if v_logits.ndim == 1:
        return torch.sigmoid(v_logits.float())
    k = v_logits.shape[-1]
    centers = torch.linspace(0.0, 1.0, k, device=v_logits.device, dtype=torch.float32)
    probs = F.softmax(v_logits.float(), dim=-1)
    return (probs * centers).sum(dim=-1)


def hl_gauss_to_logit(v_logits):
    """Converts HL-Gauss [B, K] value logits to equivalent scalar logit ln(mu/(1-mu)) for Rust/infer_server."""
    if v_logits.ndim == 1:
        return v_logits.float()
    mu = hl_gauss_mean(v_logits).clamp(1e-5, 1.0 - 1e-5)
    return torch.log(mu / (1.0 - mu))


class Tx(nn.Module):
    """The transformer network (#400 R2): a global token and one token per entity through `layers` pre-norm
    self-attention blocks; each candidate attends to the tokens once. Same outputs as `Net`. `net.rs` (`Tx`) runs it
    in plain Rust; everything here is written out by hand so the two match exactly.

    Optional extensions (all default off so older checkpoints load identically):
    - `val_mlp=1`: dedicated 2-layer value MLP reading `[t[:, 0], mean_ent, max_ent, glob]` with LayerNorm & dropout
    - `gelu=1`: tanh-approximated GELU instead of ReLU in FFN / trunk / value layers
    - `drop`: training-only dropout rate in transformer blocks (0.0 at eval/export)
    """

    def __init__(
        self,
        d=128,
        heads=4,
        layers=3,
        ff=256,
        dim=32,
        trunk=128,
        text=0,
        val_mlp=0,
        val_bins=1,
        gelu=0,
        glob_f=GLOB_F_V1,
        rel=0,
        act_ent=0,
        threat_head=0,
        drop=0.0,
    ):
        super().__init__()
        self.cfg = {"d": d, "heads": heads, "layers": layers, "ff": ff, "dim": dim, "trunk": trunk}
        if text:
            self.cfg["text"] = int(text)
        if val_mlp:
            self.cfg["val_mlp"] = 1
        if int(val_bins) > 1:
            self.cfg["val_bins"] = int(val_bins)
        if gelu:
            self.cfg["gelu"] = 1
        if int(glob_f) != GLOB_F_V1:
            self.cfg["glob_f"] = int(glob_f)
        if rel:
            self.cfg["rel"] = 1
        if act_ent:
            self.cfg["act_ent"] = 1
        if threat_head:
            self.cfg["threat_head"] = 1
        self.heads = heads
        self.d = d
        self.glob_f = int(glob_f)
        self.val_mlp = bool(val_mlp)
        self.val_bins = int(val_bins)
        self.use_gelu = bool(gelu)
        self.use_rel = bool(rel)
        self.act_ent = bool(act_ent)
        self.drop = nn.Dropout(drop) if drop > 0 else nn.Identity()
        self.emb = CardEmb(dim, int(text))
        self.ent_in = nn.Linear(dim + ENT_F, d)
        self.glob_in = nn.Linear(self.glob_f, d)
        self.rel_bias = nn.Parameter(torch.zeros(2 * NUM_REL_TYPES, heads)) if self.use_rel else None
        self.blocks = nn.ModuleList()
        for _ in range(layers):
            b = nn.Module()
            b.ln1, b.qkv, b.out = nn.LayerNorm(d), nn.Linear(d, 3 * d), nn.Linear(d, d)
            b.ln2, b.ff1, b.ff2 = nn.LayerNorm(d), nn.Linear(d, ff), nn.Linear(ff, d)
            self.blocks.append(b)
        self.ln_f = nn.LayerNorm(d)
        self.trunk1 = nn.Linear(2 * d, trunk)
        if self.val_mlp:
            v_in = 3 * d + self.glob_f
            self.val_ln = nn.LayerNorm(v_in)
            self.val_fc1 = nn.Linear(v_in, trunk)
            self.val_drop = nn.Dropout(0.3)
            self.val_fc2 = nn.Linear(trunk, self.val_bins)
            self.value = None
        else:
            self.value = nn.Linear(trunk, 1)
        self.belief = nn.Linear(trunk, dim)
        self.threat = nn.Linear(trunk, BELIEF_META_F) if threat_head else None
        self.act_in = nn.Linear(dim + ACT_F + (d if self.act_ent else 0), d)
        self.act_ln = nn.LayerNorm(d)
        self.act_q, self.act_kv, self.act_o = nn.Linear(d, d), nn.Linear(d, 2 * d), nn.Linear(d, d)
        self.act_ff = nn.Linear(d, d)
        self.query = nn.Linear(trunk, d)
        self.abias = nn.Linear(d, 1)

    def _act(self, x):
        return _gelu(x) if self.use_gelu else F.relu(x)

    def _glob(self, g):
        if g.shape[-1] == self.glob_f:
            return g
        if g.shape[-1] > self.glob_f:
            return g[..., : self.glob_f]
        return F.pad(g, (0, self.glob_f - g.shape[-1]))

    def _build_attn_bias(self, b, mask, dtype):
        if self.rel_bias is None or "ent_rel_idx" not in b or "ent_rel_type" not in b:
            return mask[:, None, None, :]
        B, T = mask.shape
        ne = T - 1
        if ne <= 0:
            return mask[:, None, None, :]
        ri = b["ent_rel_idx"].long()
        rt = b["ent_rel_type"].long()
        em = b["ent_mask"]
        ri_safe = ri.clamp(0, ne - 1)
        valid = em & (rt >= 1) & (rt <= NUM_REL_TYPES) & (ri < ne) & em.gather(1, ri_safe)
        bi, ei = torch.nonzero(valid, as_tuple=True)
        if bi.numel() == 0:
            return mask[:, None, None, :]
        bias = torch.zeros((B, T, T, self.heads), device=mask.device, dtype=dtype)
        bias.masked_fill_(~mask[:, None, :, None], float("-inf"))
        ti = ri[bi, ei]
        r = rt[bi, ei] - 1
        rb = self.rel_bias.to(dtype)
        bias.index_put_((bi, 1 + ei, 1 + ti), rb[r], accumulate=True)
        bias.index_put_((bi, 1 + ti, 1 + ei), rb[r + NUM_REL_TYPES], accumulate=True)
        return bias.permute(0, 3, 1, 2).contiguous()

    def _attend(self, q, k, v, attn_mask):
        """q [B, Q, d], k/v [B, T, d], attn_mask [B, 1/H, 1/Q, T] -> [B, Q, d], `heads` heads"""
        B, Q, D = q.shape
        T = k.shape[1]
        h, hd = self.heads, D // self.heads
        q = q.view(B, Q, h, hd).transpose(1, 2)
        k = k.view(B, T, h, hd).transpose(1, 2)
        v = v.view(B, T, h, hd).transpose(1, 2)
        out = F.scaled_dot_product_attention(q, k, v, attn_mask=attn_mask)
        return out.transpose(1, 2).reshape(B, Q, D)

    def tokens(self, b):
        """-> tokens [B, 1+N, d] after the blocks (normed), mask [B, 1+N]"""
        g = self.glob_in(self._glob(b["glob"])).unsqueeze(1)
        e = self.ent_in(torch.cat([self.emb(b["ent_ids"]), b["ents"]], -1))
        x = torch.cat([g, e], 1)
        mask = torch.cat([torch.ones_like(b["ent_mask"][:, :1]), b["ent_mask"]], 1)
        attn_mask = self._build_attn_bias(b, mask, x.dtype)
        for blk in self.blocks:
            y = blk.ln1(x)
            q, k, v = blk.qkv(y).chunk(3, -1)
            x = x + self.drop(blk.out(self._attend(q, k, v, attn_mask)))
            x = x + self.drop(blk.ff2(self._act(blk.ff1(blk.ln2(x)))))
        return self.ln_f(x), mask

    def _pool_val_features(self, t, mask, glob):
        m = mask[:, 1:].unsqueeze(-1).float()
        ent_t = t[:, 1:]
        mean = (ent_t * m).sum(1) / m.sum(1).clamp(min=1)
        mx = torch.where(m > 0, ent_t, torch.full_like(ent_t, -1e9)).max(1).values
        mx = torch.where(m.sum(1) > 0, mx, torch.zeros_like(mx))
        return torch.cat([t[:, 0], mean, mx, self._glob(glob)], -1), mean

    def state_tokens(self, b):
        t, mask = self.tokens(b)
        m = mask[:, 1:].unsqueeze(-1).float()
        mean = (t[:, 1:] * m).sum(1) / m.sum(1).clamp(min=1)
        h = self._act(self.trunk1(torch.cat([t[:, 0], mean], -1)))
        return h, t, mask

    def state(self, b):
        return self.state_tokens(b)[0]

    def forward(self, b, return_vfeat=False, return_act_emb=False, detach_val=False):
        t, mask = self.tokens(b)
        vfeat, mean = self._pool_val_features(t, mask, b["glob"])
        h = self._act(self.trunk1(torch.cat([t[:, 0], mean], -1)))
        if self.val_mlp:
            vf = vfeat.detach() if detach_val else vfeat
            vh = self.val_drop(self._act(self.val_fc1(self.val_ln(vf))))
            v_raw = self.val_fc2(vh)
            v = v_raw.squeeze(-1) if self.val_bins == 1 else v_raw
        else:
            v = self.value(h.detach() if detach_val else h).squeeze(-1)
        if self.act_ent:
            B, na = b["act_ids"].shape
            ne = t.shape[1] - 1
            if ne > 0 and "act_ent_idx" in b:
                ae = b["act_ent_idx"].long()
                ae_valid = (ae < ne) & b["act_mask"]
                ae_safe = ae.clamp(0, ne - 1)
                ctx = t[:, 1:].gather(1, ae_safe.unsqueeze(-1).expand(-1, -1, self.d))
                ctx = ctx * ae_valid.unsqueeze(-1).to(ctx.dtype)
            else:
                ctx = torch.zeros((B, na, self.d), device=t.device, dtype=t.dtype)
            a_feat = torch.cat([self.emb(b["act_ids"]), b["acts"], ctx], -1)
        else:
            a_feat = torch.cat([self.emb(b["act_ids"]), b["acts"]], -1)
        a = self.act_in(a_feat)
        k, vv = self.act_kv(t).chunk(2, -1)
        a = a + self.act_o(self._attend(self.act_q(self.act_ln(a)), k, vv, mask[:, None, None, :]))
        a = a + self._act(self.act_ff(a))
        logits = (self.query(h).unsqueeze(1) * a).sum(-1) + self.abias(a).squeeze(-1)
        logits = logits.masked_fill(~b["act_mask"], -1e9)
        if return_act_emb:
            return logits, v, h, vfeat, a
        if return_vfeat:
            return logits, v, h, vfeat
        return logits, v, h

    def belief_logits(self, h, pool_ids):
        return (self.belief(h).unsqueeze(1) * self.emb(pool_ids)).sum(-1)


def make(arch, **kw):
    return Tx(**kw) if arch == "tx" else Net(**kw)


def arch_of(net):
    return {"arch": "tx", **net.cfg} if isinstance(net, Tx) else {"arch": "mlp"}


def build(spec):
    spec = dict(spec)
    return make(spec.pop("arch", "mlp"), **spec)


def save_dir(net, d):
    """Writes net.pt and arch.json into run folder `d`"""
    torch.save(net.state_dict(), f"{d}/net.pt")
    json.dump(arch_of(net), open(f"{d}/arch.json", "w"))


def load_dir(d, dev, upgrade_val_mlp=False, drop=0.0, upgrade_glob_f=0, upgrade_val_bins=0, upgrade_threat=False):
    """The network in run folder `d` (arch.json says which; the first network without one)"""
    import os
    spec = json.load(open(f"{d}/arch.json")) if os.path.exists(f"{d}/arch.json") else {"arch": "mlp"}
    upgraded = False
    if upgrade_val_mlp and spec.get("arch") == "tx" and not spec.get("val_mlp"):
        spec["val_mlp"] = 1
        upgraded = True
    upgraded_vb = False
    if upgrade_val_bins and int(upgrade_val_bins) > 1 and spec.get("arch") == "tx" and int(spec.get("val_bins", 1)) != int(upgrade_val_bins):
        spec["val_mlp"] = 1
        spec["val_bins"] = int(upgrade_val_bins)
        upgraded_vb = True
    old_gf = int(spec.get("glob_f", GLOB_F_V1))
    delta_gf = 0
    if upgrade_glob_f and spec.get("arch") == "tx" and int(upgrade_glob_f) > old_gf:
        delta_gf = int(upgrade_glob_f) - old_gf
        spec["glob_f"] = int(upgrade_glob_f)
    upgraded_th = False
    if upgrade_threat and spec.get("arch") == "tx" and not spec.get("threat_head"):
        spec["threat_head"] = 1
        upgraded_th = True
    if spec.get("arch") == "tx" and drop > 0:
        spec["drop"] = drop
    net = build(spec).to(dev)
    sd = torch.load(f"{d}/net.pt", map_location=dev)
    if delta_gf > 0:
        # Zero-loss warm-start expansion of glob_f (e.g. 81 -> 89):
        # pad glob_in.weight and val_fc1.weight columns with 0, val_ln.weight with 1, val_ln.bias with 0
        if "glob_in.weight" in sd and sd["glob_in.weight"].shape[1] == old_gf:
            sd["glob_in.weight"] = F.pad(sd["glob_in.weight"], (0, delta_gf))
        if "val_fc1.weight" in sd and sd["val_fc1.weight"].shape[1] + delta_gf == net.val_fc1.weight.shape[1]:
            sd["val_fc1.weight"] = F.pad(sd["val_fc1.weight"], (0, delta_gf))
        if "val_ln.weight" in sd and sd["val_ln.weight"].shape[0] + delta_gf == net.val_ln.weight.shape[0]:
            sd["val_ln.weight"] = F.pad(sd["val_ln.weight"], (0, delta_gf), value=1.0)
        if "val_ln.bias" in sd and sd["val_ln.bias"].shape[0] + delta_gf == net.val_ln.bias.shape[0]:
            sd["val_ln.bias"] = F.pad(sd["val_ln.bias"], (0, delta_gf), value=0.0)
    if upgraded or upgraded_vb or upgraded_th:
        if upgraded or upgraded_vb:
            sd = {k: v for k, v in sd.items() if not k.startswith("value.") and (not upgraded_vb or not k.startswith("val_fc2."))}
        net.load_state_dict(sd, strict=False)
        if upgraded_th and getattr(net, "threat", None) is not None:
            nn.init.zeros_(net.threat.weight)
            nn.init.zeros_(net.threat.bias)
    else:
        net.load_state_dict(sd)
    return net


TENSORS = ["emb", "ent1", "ent2", "glob1", "trunk1", "trunk2", "value", "act1", "act2", "query", "abias", "belief"]


def export(net, path, meta=None):
    """Writes the `.pmnet` file `net.rs` reads: a JSON header line, then little-endian f32 tensors"""
    sd = {k: v.detach().float().cpu() for k, v in net.state_dict().items()}
    if isinstance(net, Tx):
        # the card-text projection folded into the name embedding
        sd["emb.weight"] = net.emb.table().detach().float().cpu()
        names = [k for k in sd if k not in ("emb.text", "emb.proj")]
        cfg = {k: v for k, v in net.cfg.items() if k != "text"}
        header = {"arch": "tx", "config": cfg, "tensors": [[k, list(sd[k].shape)] for k in names], "meta": meta or {}}
    else:
        names = []
        for t in TENSORS:
            for part in ("weight", "bias"):
                k = f"{t}.{part}"
                if k in sd:
                    names.append(k)
        header = {"tensors": [[k, list(sd[k].shape)] for k in names], "meta": meta or {}}
    with open(path, "wb") as f:
        f.write(json.dumps(header).encode() + b"\n")
        for k in names:
            a = sd[k].contiguous().view(-1).tolist()
            f.write(struct.pack(f"<{len(a)}f", *a))


def params(net):
    return sum(p.numel() for p in net.parameters())


if __name__ == "__main__":
    import sys

    torch.manual_seed(0)
    n = Net()
    export(n, sys.argv[1] if len(sys.argv) > 1 else "net.pmnet", {"kind": "random init"})
    print("params", params(n))
