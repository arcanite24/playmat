"""The policy, value and belief network: the same as playmat-ai's `net.rs`, in PyTorch, plus a critic that sees the
opponent's hand (training only, never exported).

Shapes come from `obs.rs`: ENT_F entity features, GLOB_F global ones, ACT_F per action, BUCKETS card-name buckets.
"""

import json
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


TENSORS = ["emb", "ent1", "ent2", "glob1", "trunk1", "trunk2", "value", "act1", "act2", "query", "abias", "belief"]


def export(net, path, meta=None):
    """Writes the `.pmnet` file `net.rs` reads: a JSON header line, then little-endian f32 tensors"""
    sd = {k: v.detach().float().cpu() for k, v in net.state_dict().items()}
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
