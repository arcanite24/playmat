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
BUCKETS_32K = 32768
CARDTEXT_V2_DIM = 1088
NUM_REL_TYPES = 4
NO_ENT = 255
_HAS_RMS_NORM = hasattr(F, "rms_norm")


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

    def belief_logits(self, h, pool_ids, drop_salt=None, proj_text=None):
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
        self.weight = nn.Parameter(torch.zeros(BUCKETS, dim))
        self.id_drop = 0.0
        if text:
            import os
            import numpy as np

            fname = "cardtext_sem.npy" if int(text) == 2 else "cardtext.npy"
            rows = np.load(os.path.join(os.path.dirname(os.path.abspath(__file__)), "data", fname))
            self.register_buffer("text", torch.from_numpy(rows.astype("float32")))
            self.proj = nn.Parameter(torch.zeros(rows.shape[1], dim))
        else:
            self.text = None

    def projected_text(self):
        return None if self.text is None else (self.text @ self.proj)

    @staticmethod
    def sample_salt(device):
        return torch.randint(0, 1 << 62, (1,), device=device, dtype=torch.int64)

    def _drop_scale(self, ids, dtype, salt=None):
        """Per-example, per-unique-card-ID inverted dropout mask (`E[scale] == 1.0`)."""
        thresh = int(round(float(self.id_drop) * 16777216.0))
        if thresh <= 0:
            return None
        if thresh >= 16777216:
            return torch.zeros(ids.shape + (1,), device=ids.device, dtype=dtype)
        s = self.sample_salt(ids.device) if salt is None else salt.to(device=ids.device, dtype=torch.int64)
        ids_i64 = ids.to(torch.int64)
        if ids_i64.ndim >= 2:
            row_shape = (ids_i64.shape[0],) + (1,) * (ids_i64.ndim - 1)
            row = torch.arange(ids_i64.shape[0], device=ids.device, dtype=torch.int64).view(row_shape)
        else:
            row = 0
        # 64-bit SplitMix hash of (salt, example_row, card_id) so every occurrence of a card in one state shares one draw
        x = s ^ (row * -7046029254386353131) ^ (ids_i64 * 5871781006564002453)
        x = (x ^ ((x >> 30) & 0x3FFFFFFFF)) * -4658895280553007687
        x = (x ^ ((x >> 27) & 0x1FFFFFFFFF)) * -7723592293110705685
        x = x ^ ((x >> 31) & 0x1FFFFFFFF)
        u24 = (x >> 39) & 0xFFFFFF
        inv_keep = 16777216.0 / float(16777216 - thresh)
        return (u24 >= thresh).to(dtype).unsqueeze(-1) * inv_keep

    def table(self):
        return self.weight if self.text is None else self.weight + self.text @ self.proj

    def forward(self, ids, salt=None, proj_text=None):
        w = F.embedding(ids, self.weight)
        if self.text is None:
            return w
        if self.training and self.id_drop > 0.0:
            scale = self._drop_scale(ids, w.dtype, salt=salt)
            if scale is not None:
                w = w * scale
        pt = (self.text @ self.proj) if proj_text is None else proj_text
        return w + F.embedding(ids, pt)


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
        valid = em & (rt >= 1) & (rt <= NUM_REL_TYPES) & (ri >= 0) & (ri < ne) & em.gather(1, ri_safe)
        r_safe = (rt - 1).clamp(0, NUM_REL_TYPES - 1)
        rb = self.rel_bias.to(dtype)
        vmask = valid.to(dtype).unsqueeze(1)
        src_fwd = F.embedding(r_safe, rb[:NUM_REL_TYPES]).permute(0, 2, 1) * vmask
        src_rev = F.embedding(r_safe, rb[NUM_REL_TYPES:]).permute(0, 2, 1) * vmask
        ei_idx = torch.arange(1, T, device=mask.device, dtype=torch.long).unsqueeze(0)
        ti_idx = 1 + ri_safe
        flat_fwd = (ei_idx * T + ti_idx).unsqueeze(1).expand(B, self.heads, ne)
        flat_rev = (ti_idx * T + ei_idx).unsqueeze(1).expand(B, self.heads, ne)
        bias_flat = torch.zeros((B, self.heads, T * T), device=mask.device, dtype=dtype)
        bias_flat = bias_flat.scatter_add(2, torch.cat([flat_fwd, flat_rev], dim=2), torch.cat([src_fwd, src_rev], dim=2))
        return bias_flat.view(B, self.heads, T, T).masked_fill(~mask[:, None, None, :], float("-inf"))

    def _attend(self, q, k, v, attn_mask):
        """q [B, Q, d], k/v [B, T, d], attn_mask [B, 1/H, 1/Q, T] -> [B, Q, d], `heads` heads"""
        B, Q, D = q.shape
        T = k.shape[1]
        h, hd = self.heads, D // self.heads
        q = q.view(B, Q, h, hd).transpose(1, 2)
        k = k.view(B, T, h, hd).transpose(1, 2)
        v = v.view(B, T, h, hd).transpose(1, 2)
        if attn_mask is not None and attn_mask.is_floating_point() and attn_mask.dtype != q.dtype:
            attn_mask = attn_mask.to(q.dtype)
        out = F.scaled_dot_product_attention(q, k, v, attn_mask=attn_mask)
        return out.transpose(1, 2).reshape(B, Q, D)

    def _emb_ctx(self, b, drop_salt=None, proj_text=None):
        if drop_salt is None:
            drop_salt = b.get("_drop_salt")
        if drop_salt is None and self.training and self.emb.text is not None and self.emb.id_drop > 0.0:
            drop_salt = self.emb.sample_salt(b["ent_ids"].device)
        if proj_text is None and self.emb.text is not None:
            proj_text = self.emb.projected_text()
        return drop_salt, proj_text

    def tokens(self, b, drop_salt=None, proj_text=None):
        """-> tokens [B, 1+N, d] after the blocks (normed), mask [B, 1+N]"""
        drop_salt, proj_text = self._emb_ctx(b, drop_salt=drop_salt, proj_text=proj_text)
        g = self.glob_in(self._glob(b["glob"])).unsqueeze(1)
        e = self.ent_in(torch.cat([self.emb(b["ent_ids"], salt=drop_salt, proj_text=proj_text), b["ents"]], -1))
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
        drop_salt, proj_text = self._emb_ctx(b)
        t, mask = self.tokens(b, drop_salt=drop_salt, proj_text=proj_text)
        vfeat, mean = self._pool_val_features(t, mask, b["glob"])
        h = self._act(self.trunk1(torch.cat([t[:, 0], mean], -1)))
        if self.val_mlp:
            vf = vfeat.detach() if detach_val else vfeat
            vh = self.val_drop(self._act(self.val_fc1(self.val_ln(vf))))
            v_raw = self.val_fc2(vh)
            v = v_raw.squeeze(-1) if self.val_bins == 1 else v_raw
        else:
            v = self.value(h.detach() if detach_val else h).squeeze(-1)
        act_emb_in = self.emb(b["act_ids"], salt=drop_salt, proj_text=proj_text)
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
            a_feat = torch.cat([act_emb_in, b["acts"], ctx], -1)
        else:
            a_feat = torch.cat([act_emb_in, b["acts"]], -1)
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

    def belief_logits(self, h, pool_ids, drop_salt=None, proj_text=None):
        return (self.belief(h).unsqueeze(1) * self.emb(pool_ids, salt=drop_salt, proj_text=proj_text)).sum(-1)


class RMSNorm(nn.Module):
    def __init__(self, dim, eps=1e-6):
        super().__init__()
        self.dim = int(dim)
        self.eps = float(eps)
        self.weight = nn.Parameter(torch.ones(self.dim))

    def reset_parameters(self):
        nn.init.ones_(self.weight)

    def forward(self, x):
        if _HAS_RMS_NORM:
            return F.rms_norm(x, (self.dim,), self.weight, self.eps)
        rms = torch.rsqrt(x.float().pow(2).mean(dim=-1, keepdim=True) + self.eps).to(x.dtype)
        return x * rms * self.weight


class SwiGLU(nn.Module):
    def __init__(self, d, ff, drop=0.0):
        super().__init__()
        self.w12 = nn.Linear(d, 2 * ff, bias=False)
        self.w3 = nn.Linear(ff, d, bias=False)
        self.drop = nn.Dropout(drop) if drop > 0 else nn.Identity()

    def forward(self, x):
        g, u = self.w12(x).chunk(2, dim=-1)
        return self.drop(self.w3(F.silu(g) * u))


def _resolve_cardtext_v2_path(buckets, cardtext_path=None):
    import os

    here = os.path.dirname(os.path.abspath(__file__))
    if cardtext_path:
        cands = [
            cardtext_path,
            os.path.join(here, "data", cardtext_path),
            os.path.join(here, "..", "..", "cardtext", os.path.basename(cardtext_path)),
        ]
        for p in cands:
            if os.path.exists(p):
                return os.path.abspath(p)
        raise FileNotFoundError(f"cardtext file not found: {cardtext_path}")
    fname = "cardtext_v2_32k.npy" if int(buckets) == BUCKETS_32K else "cardtext_v2_4k.npy"
    for p in (
        os.path.join(here, "data", fname),
        os.path.join(here, "..", "..", "cardtext", fname),
        os.path.join("/content/playmat/cardtext", fname),
    ):
        if os.path.exists(p):
            return os.path.abspath(p)
    return None


class CardEmbV2(nn.Module):
    """Phase 7 full-sheet card encoder:
    2-layer SwiGLU `CardAdapter(text_1088d -> adapter_hidden -> d_card)` + zero-init residual `emb_id [buckets, d_card]`
    with per-example, per-unique-card-ID inverted dropout (`p=0.50`). `freeze_for_inference()` folds `CardAdapter(text) + emb_id`
    into a static `[buckets, d_card]` lookup table at server startup (`0` runtime adapter FLOPs)."""

    def __init__(
        self,
        buckets=BUCKETS_32K,
        d_card=256,
        adapter_hidden=384,
        text_dim=CARDTEXT_V2_DIM,
        cardtext_path=None,
        id_drop=0.50,
    ):
        super().__init__()
        import numpy as np

        self.buckets = int(buckets)
        self.d_card = int(d_card)
        self.adapter_hidden = int(adapter_hidden)
        self.text_dim = int(text_dim)
        self.id_drop = float(id_drop)
        self._frozen = False

        resolved = _resolve_cardtext_v2_path(self.buckets, cardtext_path)
        if resolved is not None:
            rows = np.load(resolved).astype("float32")
            if rows.shape != (self.buckets, self.text_dim):
                raise ValueError(f"cardtext shape {rows.shape} != expected {(self.buckets, self.text_dim)} ({resolved})")
            text_t = torch.from_numpy(rows)
        else:
            text_t = torch.zeros((self.buckets, self.text_dim), dtype=torch.float32)
        self.register_buffer("text", text_t)
        self.norm = RMSNorm(self.text_dim)
        self.w12 = nn.Linear(self.text_dim, 2 * self.adapter_hidden, bias=False)
        self.w3 = nn.Linear(self.adapter_hidden, self.d_card, bias=False)
        self.emb_id = nn.Parameter(torch.zeros(self.buckets, self.d_card))

    @staticmethod
    def sample_salt(device):
        return CardEmb.sample_salt(device)

    def _drop_scale(self, ids, dtype, salt=None):
        thresh = int(round(float(self.id_drop) * 16777216.0))
        if thresh <= 0:
            return None
        if thresh >= 16777216:
            return torch.zeros(ids.shape + (1,), device=ids.device, dtype=dtype)
        s = self.sample_salt(ids.device) if salt is None else salt.to(device=ids.device, dtype=torch.int64)
        ids_i64 = ids.to(torch.int64)
        if ids_i64.ndim >= 2:
            row_shape = (ids_i64.shape[0],) + (1,) * (ids_i64.ndim - 1)
            row = torch.arange(ids_i64.shape[0], device=ids.device, dtype=torch.int64).view(row_shape)
        else:
            row = 0
        x = s ^ (row * -7046029254386353131) ^ (ids_i64 * 5871781006564002453)
        x = (x ^ ((x >> 30) & 0x3FFFFFFFF)) * -4658895280553007687
        x = (x ^ ((x >> 27) & 0x1FFFFFFFFF)) * -7723592293110705685
        x = x ^ ((x >> 31) & 0x1FFFFFFFF)
        u24 = (x >> 39) & 0xFFFFFF
        inv_keep = 16777216.0 / float(16777216 - thresh)
        return (u24 >= thresh).to(dtype).unsqueeze(-1) * inv_keep

    def projected_text(self):
        if self._frozen or self.text is None:
            return None
        g, u = self.w12(self.norm(self.text)).chunk(2, dim=-1)
        return self.w3(F.silu(g) * u)

    def table(self):
        if self._frozen or self.text is None:
            return self.emb_id
        return self.emb_id + self.projected_text()

    def freeze_for_inference(self):
        if not self._frozen:
            with torch.no_grad():
                folded = self.table().detach().float().clone()
                self.emb_id = nn.Parameter(folded, requires_grad=False)
                self.text = None
                self.norm = None
                self.w12 = None
                self.w3 = None
                self._frozen = True
        return self

    def forward(self, ids, salt=None, proj_text=None):
        if self._frozen or self.text is None:
            return F.embedding(ids, self.emb_id)
        w = F.embedding(ids, self.emb_id)
        if self.training and self.id_drop > 0.0:
            scale = self._drop_scale(ids, w.dtype, salt=salt)
            if scale is not None:
                w = w * scale
        pt = self.projected_text() if proj_text is None else proj_text
        return w + F.embedding(ids, pt)


TXV2_PRESETS = {
    "txv2-28m": {
        "d": 384,
        "heads": 6,
        "layers": 10,
        "ff": 1024,
        "d_card": 256,
        "act_layers": 2,
        "pool_queries": 4,
        "val_bins": 51,
        "glob_f": GLOB_F,
    },
    "txv2-51m": {
        "d": 512,
        "heads": 8,
        "layers": 10,
        "ff": 1536,
        "d_card": 256,
        "act_layers": 2,
        "pool_queries": 4,
        "val_bins": 51,
        "glob_f": GLOB_F,
    },
    "oracle-txv2-58m": {
        "d": 512,
        "heads": 8,
        "layers": 12,
        "ff": 1536,
        "d_card": 256,
        "act_layers": 2,
        "pool_queries": 4,
        "val_bins": 51,
        "glob_f": GLOB_F,
    },
}


class TxV2(nn.Module):
    """Phase 7 TensorCore-aligned (`head_dim = 64`) Transformer architecture:
    - `CardEmbV2`: 2-layer SwiGLU `CardAdapter` (`1088d -> d -> d_card`) + zero-init `emb_id` (`4096` or `32768` buckets)
    - Pre-RMSNorm + per-head QK-Norm + SwiGLU FFN encoder blocks + zero-sync `scatter_add` relational attention bias
    - Multi-Query Attention Pooling (`K=4`: `[self_board, opp_board, hand_stack, combat_race]`) -> trunk `h [B, d]`
    - 2-Layer Comparative Action Decoder (Cross-Attn to state + Candidate-to-Candidate Self-Attn + SwiGLU FFN)
    - Distributional `HL-Gauss` Value Head (`val_bins=51`), `belief` (`Linear(d, d_card)`), `threat` (`Linear(d, 8)`),
      and exported `act_q_head` (`Linear(d, 1)`) via `forward_with_act_q(b)`."""

    def __init__(
        self,
        d=384,
        heads=6,
        layers=10,
        ff=1024,
        buckets=BUCKETS_32K,
        d_card=256,
        text_dim=CARDTEXT_V2_DIM,
        act_layers=2,
        pool_queries=4,
        val_bins=51,
        glob_f=GLOB_F,
        rel=1,
        act_ent=1,
        drop=0.0,
        id_drop=0.0,
        cardtext_path=None,
        preset=None,
    ):
        super().__init__()
        if preset:
            if preset not in TXV2_PRESETS:
                raise ValueError(f"unknown TxV2 preset {preset!r}; choices={list(TXV2_PRESETS)}")
            p_cfg = TXV2_PRESETS[preset]
            d = p_cfg["d"]
            heads = p_cfg["heads"]
            layers = p_cfg["layers"]
            ff = p_cfg["ff"]
            d_card = p_cfg["d_card"]
            act_layers = p_cfg["act_layers"]
            pool_queries = p_cfg["pool_queries"]
            val_bins = p_cfg["val_bins"]
            glob_f = p_cfg["glob_f"]

        assert d % heads == 0, f"d={d} must be divisible by heads={heads}"
        self.d = int(d)
        self.heads = int(heads)
        self.head_dim = self.d // self.heads
        self.layers = int(layers)
        self.ff = int(ff)
        self.buckets = int(buckets)
        self.d_card = int(d_card)
        self.text_dim = int(text_dim)
        self.act_layers = int(act_layers)
        self.pool_queries = int(pool_queries)
        self.val_bins = int(val_bins)
        self.glob_f = int(glob_f)
        self.use_rel = bool(rel)
        self.act_ent = bool(act_ent)
        self.val_mlp = True

        self.cfg = {
            "d": self.d,
            "heads": self.heads,
            "layers": self.layers,
            "ff": self.ff,
            "buckets": self.buckets,
            "d_card": self.d_card,
            "text_dim": self.text_dim,
            "act_layers": self.act_layers,
            "pool_queries": self.pool_queries,
            "val_bins": self.val_bins,
            "glob_f": self.glob_f,
            "rel": int(self.use_rel),
            "act_ent": int(self.act_ent),
        }

        self.drop = nn.Dropout(drop) if drop > 0 else nn.Identity()
        self.emb = CardEmbV2(
            buckets=self.buckets,
            d_card=self.d_card,
            adapter_hidden=self.d,
            text_dim=self.text_dim,
            cardtext_path=cardtext_path,
            id_drop=id_drop,
        )
        self.ent_in = nn.Linear(self.d_card + ENT_F, self.d)
        self.glob_in = nn.Linear(self.glob_f, self.d)
        self.rel_bias = nn.Parameter(torch.zeros(2 * NUM_REL_TYPES, self.heads)) if self.use_rel else None

        self.blocks = nn.ModuleList()
        for _ in range(self.layers):
            blk = nn.Module()
            blk.ln1 = RMSNorm(self.d)
            blk.qkv = nn.Linear(self.d, 3 * self.d, bias=False)
            blk.q_norm = RMSNorm(self.head_dim)
            blk.k_norm = RMSNorm(self.head_dim)
            blk.out = nn.Linear(self.d, self.d, bias=False)
            blk.ln2 = RMSNorm(self.d)
            blk.ffn = SwiGLU(self.d, self.ff, drop=drop)
            self.blocks.append(blk)
        self.ln_f = RMSNorm(self.d)

        # Multi-Query Attention Pooling (`K=4`: [self_board, opp_board, hand_stack, combat_race]) -> trunk `h [B, d]`
        self.pool_q = nn.Parameter(torch.randn(1, self.pool_queries, self.d) * (self.d ** -0.5))
        self.pool_kv = nn.Linear(self.d, 2 * self.d, bias=False)
        self.pool_q_norm = RMSNorm(self.head_dim)
        self.pool_k_norm = RMSNorm(self.head_dim)
        trunk_in = (1 + self.pool_queries) * self.d + self.glob_f
        self.trunk_ln = RMSNorm(trunk_in)
        self.trunk1 = nn.Linear(trunk_in, self.d)

        # Distributional HL-Gauss Value Head
        self.val_ln = RMSNorm(trunk_in)
        self.val_fc1 = nn.Linear(trunk_in, self.d)
        self.val_drop = nn.Dropout(0.10)
        self.val_fc2 = nn.Linear(self.d, self.val_bins)
        self.value = None

        # Belief & Threat Heads (compatible with RemoteNet handshake: dim=d_card, trunk=d)
        self.belief = nn.Linear(self.d, self.d_card)
        nn.init.normal_(self.belief.weight, mean=0.0, std=(self.d * self.d_card) ** -0.5)
        nn.init.zeros_(self.belief.bias)
        self.threat = nn.Linear(self.d, BELIEF_META_F)

        # 2-Layer Comparative Action Decoder (Cross-Attn + Candidate Self-Attn + SwiGLU)
        act_in_dim = self.d_card + ACT_F + (self.d if self.act_ent else 0)
        self.act_in = nn.Linear(act_in_dim, self.d)
        ff_act = 2 * self.d
        self.act_blocks = nn.ModuleList()
        for _ in range(self.act_layers):
            ab = nn.Module()
            ab.ln_x = RMSNorm(self.d)
            ab.x_q = nn.Linear(self.d, self.d, bias=False)
            ab.x_kv = nn.Linear(self.d, 2 * self.d, bias=False)
            ab.x_q_norm = RMSNorm(self.head_dim)
            ab.x_k_norm = RMSNorm(self.head_dim)
            ab.x_out = nn.Linear(self.d, self.d, bias=False)
            ab.ln_s = RMSNorm(self.d)
            ab.s_qkv = nn.Linear(self.d, 3 * self.d, bias=False)
            ab.s_q_norm = RMSNorm(self.head_dim)
            ab.s_k_norm = RMSNorm(self.head_dim)
            ab.s_out = nn.Linear(self.d, self.d, bias=False)
            ab.ln_ff = RMSNorm(self.d)
            ab.ffn = SwiGLU(self.d, ff_act, drop=drop)
            self.act_blocks.append(ab)
        self.act_ln_f = RMSNorm(self.d)

        self.query = nn.Linear(self.d, self.d)
        self.pol_q_ln = RMSNorm(self.d)
        self.pol_k_ln = RMSNorm(self.d)
        self.abias = nn.Sequential(nn.Linear(self.d, self.d // 2), nn.SiLU(), nn.Linear(self.d // 2, 1))
        self.act_q_head = nn.Linear(self.d, 1)

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
        valid = em & (rt >= 1) & (rt <= NUM_REL_TYPES) & (ri >= 0) & (ri < ne) & em.gather(1, ri_safe)
        r_safe = (rt - 1).clamp(0, NUM_REL_TYPES - 1)
        rb = self.rel_bias.to(dtype)
        vmask = valid.to(dtype).unsqueeze(1)
        src_fwd = F.embedding(r_safe, rb[:NUM_REL_TYPES]).permute(0, 2, 1) * vmask
        src_rev = F.embedding(r_safe, rb[NUM_REL_TYPES:]).permute(0, 2, 1) * vmask
        ei_idx = torch.arange(1, T, device=mask.device, dtype=torch.long).unsqueeze(0)
        ti_idx = 1 + ri_safe
        flat_fwd = (ei_idx * T + ti_idx).unsqueeze(1).expand(B, self.heads, ne)
        flat_rev = (ti_idx * T + ei_idx).unsqueeze(1).expand(B, self.heads, ne)
        bias_flat = torch.zeros((B, self.heads, T * T), device=mask.device, dtype=dtype)
        bias_flat = bias_flat.scatter_add(2, torch.cat([flat_fwd, flat_rev], dim=2), torch.cat([src_fwd, src_rev], dim=2))
        return bias_flat.view(B, self.heads, T, T).masked_fill(~mask[:, None, None, :], float("-inf"))

    def _attend_qk(self, q, k, v, q_norm, k_norm, attn_mask):
        B, Q, D = q.shape
        T = k.shape[1]
        h, hd = self.heads, self.head_dim
        q = q_norm(q.view(B, Q, h, hd).transpose(1, 2))
        k = k_norm(k.view(B, T, h, hd).transpose(1, 2))
        v = v.view(B, T, h, hd).transpose(1, 2)
        if attn_mask is not None and attn_mask.is_floating_point() and attn_mask.dtype != q.dtype:
            attn_mask = attn_mask.to(q.dtype)
        out = F.scaled_dot_product_attention(q, k, v, attn_mask=attn_mask)
        return out.transpose(1, 2).reshape(B, Q, D)

    def _emb_ctx(self, b, drop_salt=None, proj_text=None):
        if drop_salt is None:
            drop_salt = b.get("_drop_salt")
        if drop_salt is None and self.training and not self.emb._frozen and self.emb.id_drop > 0.0:
            drop_salt = self.emb.sample_salt(b["ent_ids"].device)
        if proj_text is None and not self.emb._frozen:
            proj_text = self.emb.projected_text()
        return drop_salt, proj_text

    def tokens(self, b, drop_salt=None, proj_text=None):
        drop_salt, proj_text = self._emb_ctx(b, drop_salt=drop_salt, proj_text=proj_text)
        g = self.glob_in(self._glob(b["glob"])).unsqueeze(1)
        e = self.ent_in(torch.cat([self.emb(b["ent_ids"], salt=drop_salt, proj_text=proj_text), b["ents"]], -1))
        x = torch.cat([g, e], 1)
        mask = torch.cat([torch.ones_like(b["ent_mask"][:, :1]), b["ent_mask"]], 1)
        attn_mask = self._build_attn_bias(b, mask, x.dtype)
        for blk in self.blocks:
            q, k, v = blk.qkv(blk.ln1(x)).chunk(3, -1)
            x = x + self.drop(blk.out(self._attend_qk(q, k, v, blk.q_norm, blk.k_norm, attn_mask)))
            x = x + blk.ffn(blk.ln2(x))
        return self.ln_f(x), mask

    def _pool_trunk(self, t, mask, glob):
        B = t.shape[0]
        pq = self.pool_q.to(dtype=t.dtype).expand(B, -1, -1)
        pk, pv = self.pool_kv(t).chunk(2, dim=-1)
        pooled = self._attend_qk(pq, pk, pv, self.pool_q_norm, self.pool_k_norm, mask[:, None, None, :])
        vfeat = torch.cat([t[:, 0], pooled.reshape(B, self.pool_queries * self.d), self._glob(glob)], dim=-1)
        h = F.silu(self.trunk1(self.trunk_ln(vfeat)))
        return h, vfeat

    def state_tokens(self, b):
        t, mask = self.tokens(b)
        h, _ = self._pool_trunk(t, mask, b["glob"])
        return h, t, mask

    def state(self, b):
        return self.state_tokens(b)[0]

    def _decode_actions(self, b, t, mask, h, drop_salt, proj_text):
        act_emb_in = self.emb(b["act_ids"], salt=drop_salt, proj_text=proj_text)
        if self.act_ent:
            B, na = b["act_ids"].shape
            ne = t.shape[1] - 1
            if ne > 0 and "act_ent_idx" in b:
                ae = b["act_ent_idx"].long()
                ae_valid = (ae >= 0) & (ae < ne) & b["act_mask"]
                ae_safe = ae.clamp(0, ne - 1)
                ctx = t[:, 1:].gather(1, ae_safe.unsqueeze(-1).expand(-1, -1, self.d))
                ctx = ctx * ae_valid.unsqueeze(-1).to(ctx.dtype)
            else:
                ctx = torch.zeros((B, na, self.d), device=t.device, dtype=t.dtype)
            a_feat = torch.cat([act_emb_in, b["acts"], ctx], -1)
        else:
            a_feat = torch.cat([act_emb_in, b["acts"]], -1)

        a = self.act_in(a_feat)
        am = b["act_mask"]
        col0 = (torch.arange(am.shape[1], device=am.device) == 0).unsqueeze(0)
        safe_am = am | (~am.any(dim=-1, keepdim=True) & col0)
        self_mask = safe_am[:, None, None, :]
        state_mask = mask[:, None, None, :]

        for ab in self.act_blocks:
            xq = ab.x_q(ab.ln_x(a))
            xk, xv = ab.x_kv(t).chunk(2, dim=-1)
            a = a + self.drop(ab.x_out(self._attend_qk(xq, xk, xv, ab.x_q_norm, ab.x_k_norm, state_mask)))
            sq, sk, sv = ab.s_qkv(ab.ln_s(a)).chunk(3, dim=-1)
            a = a + self.drop(ab.s_out(self._attend_qk(sq, sk, sv, ab.s_q_norm, ab.s_k_norm, self_mask)))
            a = a + ab.ffn(ab.ln_ff(a))
        a = self.act_ln_f(a)

        pol_q = self.pol_q_ln(self.query(h)).unsqueeze(1)
        pol_k = self.pol_k_ln(a)
        logits = (pol_q * pol_k).sum(-1) * (self.d ** -0.5) + self.abias(a).squeeze(-1)
        logits = logits.masked_fill(~am, -1e9)
        return logits, a

    def forward(
        self,
        b,
        return_vfeat=False,
        return_act_emb=False,
        detach_val=False,
        return_act_q_logits=False,
        return_act_q=False,
    ):
        drop_salt, proj_text = self._emb_ctx(b)
        t, mask = self.tokens(b, drop_salt=drop_salt, proj_text=proj_text)
        h, vfeat = self._pool_trunk(t, mask, b["glob"])
        vf = vfeat.detach() if detach_val else vfeat
        vh = self.val_drop(F.silu(self.val_fc1(self.val_ln(vf))))
        v_raw = self.val_fc2(vh)
        v = v_raw.squeeze(-1) if self.val_bins == 1 else v_raw
        logits, a = self._decode_actions(b, t, mask, h, drop_salt, proj_text)
        if return_act_q_logits:
            aq_logits = self.act_q_head(a).squeeze(-1)
            return logits, v, h, vfeat, a, aq_logits
        if return_act_q:
            aq_logits = self.act_q_head(a).squeeze(-1)
            act_q = torch.where(b["act_mask"], torch.sigmoid(aq_logits.float()), 0.5)
            return logits, v, h, act_q
        if return_act_emb:
            return logits, v, h, vfeat, a
        if return_vfeat:
            return logits, v, h, vfeat
        return logits, v, h

    def forward_with_act_q(self, b):
        """Inference helper for `infer_server.py`: returns `(logits, value_logits, h, act_q_prob_in_0_1)`."""
        return self.forward(b, return_act_q=True)

    def belief_logits(self, h, pool_ids, drop_salt=None, proj_text=None):
        return (self.belief(h).unsqueeze(1) * self.emb(pool_ids, salt=drop_salt, proj_text=proj_text)).sum(-1)

    def threat_logits(self, h):
        return self.threat(h)


def make(arch, **kw):
    if arch in TXV2_PRESETS:
        return TxV2(preset=arch, **kw)
    if arch == "txv2":
        return TxV2(**kw)
    if arch == "tx":
        return Tx(**kw)
    return Net(**kw)


def arch_of(net):
    if isinstance(net, TxV2):
        return {"arch": "txv2", **net.cfg}
    if isinstance(net, Tx):
        return {"arch": "tx", **net.cfg}
    return {"arch": "mlp"}


def build(spec):
    spec = dict(spec)
    return make(spec.pop("arch", "mlp"), **spec)


def save_dir(net, d):
    """Writes net.pt and arch.json into run folder `d`"""
    import os

    os.makedirs(d, exist_ok=True)
    torch.save(net.state_dict(), f"{d}/net.pt")
    with open(f"{d}/arch.json", "w") as f:
        json.dump(arch_of(net), f, indent=2)


def load_dir(
    d,
    dev,
    upgrade_val_mlp=False,
    drop=0.0,
    upgrade_glob_f=0,
    upgrade_val_bins=0,
    upgrade_threat=False,
    upgrade_buckets=0,
    cardtext_path=None,
):
    """The network in run folder `d` (arch.json or meta.json says which; the first network without one)"""
    import os

    arch_file = f"{d}/arch.json" if os.path.exists(f"{d}/arch.json") else (f"{d}/meta.json" if os.path.exists(f"{d}/meta.json") else None)
    spec = json.load(open(arch_file)) if arch_file else {"arch": "mlp"}
    pt_file = f"{d}/net.pt" if os.path.exists(f"{d}/net.pt") else f"{d}/model.pt"
    arch_kind = spec.get("arch", "mlp")

    if arch_kind == "txv2" or arch_kind in TXV2_PRESETS:
        build_spec = dict(spec)
        if drop > 0:
            build_spec["drop"] = drop
        if cardtext_path:
            build_spec["cardtext_path"] = cardtext_path
        old_gf = int(build_spec.get("glob_f", GLOB_F))
        delta_gf = 0
        if upgrade_glob_f and int(upgrade_glob_f) > old_gf:
            delta_gf = int(upgrade_glob_f) - old_gf
            build_spec["glob_f"] = int(upgrade_glob_f)
        upgraded_vb = False
        if upgrade_val_bins and int(upgrade_val_bins) > 0 and int(build_spec.get("val_bins", 51)) != int(upgrade_val_bins):
            build_spec["val_bins"] = int(upgrade_val_bins)
            upgraded_vb = True
        upgraded_bk = False
        if upgrade_buckets and int(upgrade_buckets) > 0 and int(build_spec.get("buckets", BUCKETS_32K)) != int(upgrade_buckets):
            build_spec["buckets"] = int(upgrade_buckets)
            upgraded_bk = True
        net = build(build_spec).to(dev)
        sd = torch.load(pt_file, map_location=dev)
        if delta_gf > 0:
            if "glob_in.weight" in sd and sd["glob_in.weight"].shape[1] == old_gf:
                sd["glob_in.weight"] = F.pad(sd["glob_in.weight"], (0, delta_gf))
            for lk in ("trunk1.weight", "val_fc1.weight"):
                if lk in sd and sd[lk].shape[1] + delta_gf == getattr(net, lk.split(".")[0]).weight.shape[1]:
                    sd[lk] = F.pad(sd[lk], (0, delta_gf))
            for nk in ("trunk_ln.weight", "val_ln.weight"):
                if nk in sd and sd[nk].shape[0] + delta_gf == getattr(net, nk.split(".")[0]).weight.shape[0]:
                    sd[nk] = F.pad(sd[nk], (0, delta_gf), value=1.0)
        skip_keys = set()
        if upgraded_bk:
            skip_keys.update({"emb.emb_id", "emb.text"})
        elif cardtext_path:
            skip_keys.add("emb.text")
        if upgraded_vb:
            skip_keys.update({"val_fc2.weight", "val_fc2.bias"})
        if skip_keys:
            sd_filtered = {k: v for k, v in sd.items() if k not in skip_keys}
            missing, unexpected = net.load_state_dict(sd_filtered, strict=False)
            bad_missing = [k for k in missing if k not in skip_keys]
            if bad_missing or unexpected:
                raise RuntimeError(f"TxV2 warm-start key mismatch: missing={bad_missing}, unexpected={unexpected}")
        else:
            net.load_state_dict(sd)
        return net

    upgraded = False
    if upgrade_val_mlp and arch_kind == "tx" and not spec.get("val_mlp"):
        spec["val_mlp"] = 1
        upgraded = True
    upgraded_vb = False
    if upgrade_val_bins and int(upgrade_val_bins) > 1 and arch_kind == "tx" and int(spec.get("val_bins", 1)) != int(upgrade_val_bins):
        spec["val_mlp"] = 1
        spec["val_bins"] = int(upgrade_val_bins)
        upgraded_vb = True
    old_gf = int(spec.get("glob_f", GLOB_F_V1))
    delta_gf = 0
    if upgrade_glob_f and arch_kind == "tx" and int(upgrade_glob_f) > old_gf:
        delta_gf = int(upgrade_glob_f) - old_gf
        spec["glob_f"] = int(upgrade_glob_f)
    upgraded_th = False
    if upgrade_threat and arch_kind == "tx" and not spec.get("threat_head"):
        spec["threat_head"] = 1
        upgraded_th = True
    if arch_kind == "tx" and drop > 0:
        spec["drop"] = drop
    net = build(spec).to(dev)
    sd = torch.load(pt_file, map_location=dev)
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


def upgrade_txv2_to_32k(src_dir, dst_dir, cardtext_path=None, dev="cpu"):
    """Zero-shot upgrades a 4,096-bucket `TxV2` checkpoint to 32,768 canonical buckets:
    preserves 100% of `CardAdapter` + Transformer trunk + decoder + heads, swaps `emb.text` to the 32k table,
    and zero-initializes `emb.emb_id` `[32768, d_card]`."""
    net = load_dir(src_dir, torch.device(dev), upgrade_buckets=BUCKETS_32K, cardtext_path=cardtext_path)
    save_dir(net, dst_dir)
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
            f.write(sd[k].contiguous().view(-1).numpy().astype("<f4", copy=False).tobytes())


def params(net):
    return sum(p.numel() for p in net.parameters())


if __name__ == "__main__":
    import sys

    torch.manual_seed(0)
    n = Net()
    export(n, sys.argv[1] if len(sys.argv) > 1 else "net.pmnet", {"kind": "random init"})
    print("params", params(n))
