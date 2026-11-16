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
GLOB_F = 65
ACT_F = 65
BUCKETS = 4096


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

    def forward(self, b):
        """-> logits [B, K] (masked to -inf past each row's actions), value logit [B], state h [B, trunk]"""
        h = self.state(b)
        v = self.value(h).squeeze(-1)
        a = self.act2(F.relu(self.act1(torch.cat([self.emb(b["act_ids"]), b["acts"]], -1))))
        q = self.query(h).unsqueeze(1)
        logits = (q * a).sum(-1) + self.abias(a).squeeze(-1)
        logits = logits.masked_fill(~b["act_mask"], -1e9)
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

    def __init__(self, dim, text=False):
        super().__init__()
        self.weight = nn.Parameter(torch.randn(BUCKETS, dim))
        if text:
            import numpy as np
            import os

            rows = np.load(os.path.join(os.path.dirname(os.path.abspath(__file__)), "data", "cardtext.npy"))
            self.register_buffer("text", torch.from_numpy(rows.astype("float32")))
            self.proj = nn.Parameter(torch.zeros(rows.shape[1], dim))
        else:
            self.text = None

    def table(self):
        return self.weight if self.text is None else self.weight + self.text @ self.proj

    def forward(self, ids):
        if self.text is None:
            return F.embedding(ids, self.weight)
        return F.embedding(ids, self.weight) + F.embedding(ids, self.text) @ self.proj


class Tx(nn.Module):
    """The transformer network (#400 R2): a global token and one token per entity through `layers` pre-norm
    self-attention blocks; each candidate attends to the tokens once. Same outputs as `Net`. `net.rs` (`Tx`) runs it
    in plain Rust; everything here is written out by hand so the two match exactly."""

    def __init__(self, d=128, heads=4, layers=3, ff=256, dim=32, trunk=128, text=0):
        super().__init__()
        self.cfg = {"d": d, "heads": heads, "layers": layers, "ff": ff, "dim": dim, "trunk": trunk}
        if text:
            self.cfg["text"] = 1
        self.heads = heads
        self.emb = CardEmb(dim, bool(text))
        self.ent_in = nn.Linear(dim + ENT_F, d)
        self.glob_in = nn.Linear(GLOB_F, d)
        self.blocks = nn.ModuleList()
        for _ in range(layers):
            b = nn.Module()
            b.ln1, b.qkv, b.out = nn.LayerNorm(d), nn.Linear(d, 3 * d), nn.Linear(d, d)
            b.ln2, b.ff1, b.ff2 = nn.LayerNorm(d), nn.Linear(d, ff), nn.Linear(ff, d)
            self.blocks.append(b)
        self.ln_f = nn.LayerNorm(d)
        self.trunk1 = nn.Linear(2 * d, trunk)
        self.value = nn.Linear(trunk, 1)
        self.belief = nn.Linear(trunk, dim)
        self.act_in = nn.Linear(dim + ACT_F, d)
        self.act_ln = nn.LayerNorm(d)
        self.act_q, self.act_kv, self.act_o = nn.Linear(d, d), nn.Linear(d, 2 * d), nn.Linear(d, d)
        self.act_ff = nn.Linear(d, d)
        self.query = nn.Linear(trunk, d)
        self.abias = nn.Linear(d, 1)

    def _attend(self, q, k, v, mask):
        """q [B, Q, d], k/v [B, T, d], mask [B, T] (True = attend) -> [B, Q, d], `heads` heads"""
        B, Q, D = q.shape
        T = k.shape[1]
        h, hd = self.heads, D // self.heads
        q = q.view(B, Q, h, hd).transpose(1, 2)
        k = k.view(B, T, h, hd).transpose(1, 2)
        v = v.view(B, T, h, hd).transpose(1, 2)
        att = (q @ k.transpose(-1, -2)) / math.sqrt(hd)
        att = att.masked_fill(~mask[:, None, None, :], -1e9)
        return (att.softmax(-1) @ v).transpose(1, 2).reshape(B, Q, D)

    def tokens(self, b):
        """-> tokens [B, 1+N, d] after the blocks (normed), mask [B, 1+N]"""
        g = self.glob_in(b["glob"]).unsqueeze(1)
        e = self.ent_in(torch.cat([self.emb(b["ent_ids"]), b["ents"]], -1))
        x = torch.cat([g, e], 1)
        mask = torch.cat([torch.ones_like(b["ent_mask"][:, :1]), b["ent_mask"]], 1)
        for blk in self.blocks:
            y = blk.ln1(x)
            q, k, v = blk.qkv(y).chunk(3, -1)
            x = x + blk.out(self._attend(q, k, v, mask))
            x = x + blk.ff2(F.relu(blk.ff1(blk.ln2(x))))
        return self.ln_f(x), mask

    def state_tokens(self, b):
        t, mask = self.tokens(b)
        m = mask[:, 1:].unsqueeze(-1).float()
        mean = (t[:, 1:] * m).sum(1) / m.sum(1).clamp(min=1)
        h = F.relu(self.trunk1(torch.cat([t[:, 0], mean], -1)))
        return h, t, mask

    def state(self, b):
        return self.state_tokens(b)[0]

    def forward(self, b):
        h, t, mask = self.state_tokens(b)
        v = self.value(h).squeeze(-1)
        a = self.act_in(torch.cat([self.emb(b["act_ids"]), b["acts"]], -1))
        k, vv = self.act_kv(t).chunk(2, -1)
        a = a + self.act_o(self._attend(self.act_q(self.act_ln(a)), k, vv, mask))
        a = a + F.relu(self.act_ff(a))
        logits = (self.query(h).unsqueeze(1) * a).sum(-1) + self.abias(a).squeeze(-1)
        return logits.masked_fill(~b["act_mask"], -1e9), v, h

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


def load_dir(d, dev):
    """The network in run folder `d` (arch.json says which; the first network without one)"""
    import os
    spec = json.load(open(f"{d}/arch.json")) if os.path.exists(f"{d}/arch.json") else {"arch": "mlp"}
    net = build(spec).to(dev)
    net.load_state_dict(torch.load(f"{d}/net.pt", map_location=dev))
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
