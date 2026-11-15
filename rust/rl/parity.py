"""Writes playmat-ai's tests/net_parity.json: random observations and what PyTorch's network makes of them, for the
network in a `.pmnet` file. Rerun whenever net.pmnet changes:

python parity.py ../crates/playmat-ai/src/net.pmnet ../crates/playmat-ai/tests/net_parity.json
"""

import json
import struct
import sys

import torch

from model import ACT_F, BUCKETS, ENT_F, GLOB_F, Net


def load_pmnet(path):
    raw = open(path, "rb").read()
    nl = raw.index(b"\n")
    header = json.loads(raw[:nl])
    off = nl + 1
    sd = {}
    for name, shape in header["tensors"]:
        n = 1
        for d in shape:
            n *= d
        sd[name] = torch.tensor(struct.unpack_from(f"<{n}f", raw, off)).view(*shape)
        off += 4 * n
    net = Net(belief="belief.weight" in sd)
    net.load_state_dict(sd)
    return net.eval(), raw


def fnv(b):
    h = 0xCBF29CE484222325
    for x in b:
        h = ((h ^ x) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"{h:016x}"


def main():
    net, raw = load_pmnet(sys.argv[1])
    g = torch.Generator().manual_seed(1)
    cases = []
    for n_ent, n_act in [(0, 2), (1, 3), (7, 5), (30, 12)]:
        w = max(n_ent, 1)  # a row with no entities is one masked padding entity, as in training
        mask = torch.zeros(1, w, dtype=torch.bool)
        mask[0, :n_ent] = True
        b = {
            "glob": torch.rand(1, GLOB_F, generator=g),
            "ents": torch.rand(1, w, ENT_F, generator=g) * mask[..., None],
            "ent_ids": torch.randint(0, BUCKETS, (1, w), generator=g) * mask,
            "ent_mask": mask,
            "acts": torch.rand(1, n_act, ACT_F, generator=g),
            "act_ids": torch.randint(0, BUCKETS, (1, n_act), generator=g),
            "act_mask": torch.ones(1, n_act, dtype=torch.bool),
        }
        pool = torch.randint(1, BUCKETS, (1, 6), generator=g)
        with torch.no_grad():
            logits, v, h = net(b)
            bel = net.belief_logits(h, pool)
        cases.append({
            "glob": b["glob"][0].tolist(),
            "ents": b["ents"][0, :n_ent].reshape(-1).tolist(),
            "ent_ids": b["ent_ids"][0, :n_ent].tolist(),
            "acts": b["acts"][0].reshape(-1).tolist(),
            "act_ids": b["act_ids"][0].tolist(),
            "logits": logits[0].tolist(),
            "value": float(v[0]),
            "pool": pool[0].tolist(),
            "belief": bel[0].tolist(),
        })
    json.dump({"sha": fnv(raw), "cases": cases}, open(sys.argv[2], "w"))
    print("wrote", sys.argv[2], len(cases), "cases")


if __name__ == "__main__":
    main()
