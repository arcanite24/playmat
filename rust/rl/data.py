"""Reads what `selfplay` writes (playmat-ai `selfplay::Shard`) and makes padded batches of tensors.

The columns go into torch tensors once (`place`): on the GPU when they fit in `budget_gb`, so a batch is gathered
there, else in CPU memory, gathered by torch's threads and copied over pinned. Each batch is padded only to its own
longest row (`trim`), not to the fixed widths. `batch_np` is the old NumPy path, kept as the reference."""

import glob
import json
import os

import numpy as np
import torch

from model import ACT_F, ENT_F, GLOB_F

META = 10
N_ENT, N_ACT, N_HID, N_POOL = 64, 16, 16, 64


def _read(path, dtype):
    return np.fromfile(path, dtype=dtype) if os.path.exists(path) else np.zeros(0, dtype)


def _offsets(counts):
    o = np.zeros(len(counts) + 1, np.int64)
    np.cumsum(counts, out=o[1:])
    return o


class Data:
    """Every decision in the shards under `dirs`, with each game's result"""

    def __init__(self, dirs):
        cols = {k: [] for k in ["meta", "logp", "glob", "ents", "ent_ids", "acts", "act_ids", "target", "hid", "hid_ids", "pool", "pool_hand"]}
        results, infos, run = {}, {}, []
        for r, d in enumerate(dirs):
            for meta_path in sorted(glob.glob(f"{d}/*.meta.i32")):
                base = meta_path[: -len(".meta.i32")]
                if "._" in os.path.basename(base):  # a Mac's AppleDouble metadata, not a shard
                    continue
                meta = _read(meta_path, np.int32).reshape(-1, META)
                if len(meta) == 0:
                    continue
                cols["meta"].append(meta)
                run.append(np.full(len(meta), r, np.int32))
                cols["logp"].append(_read(base + ".logp.f32", np.float32))
                cols["glob"].append(_read(base + ".glob.f16", np.float16).reshape(-1, GLOB_F))
                cols["ents"].append(_read(base + ".ents.f16", np.float16).reshape(-1, ENT_F))
                cols["ent_ids"].append(_read(base + ".ent_ids.u16", np.uint16))
                cols["acts"].append(_read(base + ".acts.f16", np.float16).reshape(-1, ACT_F))
                cols["act_ids"].append(_read(base + ".act_ids.u16", np.uint16))
                cols["target"].append(_read(base + ".target.f16", np.float16))
                cols["hid"].append(_read(base + ".hid.f16", np.float16).reshape(-1, ENT_F))
                cols["hid_ids"].append(_read(base + ".hid_ids.u16", np.uint16))
                cols["pool"].append(_read(base + ".pool.u16", np.uint16))
                cols["pool_hand"].append(_read(base + ".pool_hand.u8", np.uint8))
                with open(base + ".games.jsonl") as f:
                    for line in f:
                        g = json.loads(line)
                        results[(r, g["game"])] = g["result"]
                        infos[(r, g["game"])] = g
        for k, v in cols.items():
            setattr(self, k, np.concatenate(v) if v else np.zeros(0))
        self.run = np.concatenate(run)
        m = self.meta
        self.game, self.seat, self.turn, self.chosen = m[:, 0], m[:, 1], m[:, 2], m[:, 3]
        self.n_ent, self.n_act, self.n_hid, self.n_pool = m[:, 4], m[:, 5], m[:, 6], m[:, 7]
        self.qkind, self.tag = m[:, 8], m[:, 9]
        self.ent_off, self.act_off = _offsets(self.n_ent), _offsets(self.n_act)
        self.hid_off, self.pool_off = _offsets(self.n_hid), _offsets(self.n_pool)
        self.infos = infos
        self.z = np.array([results[(r, g)][s] for r, g, s in zip(self.run, self.game, self.seat)], np.float32)
        self.n = len(m)
        self.store = None

    def place(self, dev, budget_gb=8.0):
        """Puts the columns in torch tensors: on `dev` when it is a GPU and they fit in `budget_gb`, else on the CPU"""
        cols = {
            "glob": self.glob, "ents": self.ents, "ent_ids": self.ent_ids.astype(np.int32),
            "acts": self.acts, "act_ids": self.act_ids.astype(np.int32), "target": self.target,
            "hid": self.hid, "hid_ids": self.hid_ids.astype(np.int32), "pool": self.pool.astype(np.int32),
            "pool_hand": self.pool_hand, "chosen": self.chosen.astype(np.int64), "logp": self.logp, "z": self.z,
        }
        for k in ["ent", "act", "hid", "pool"]:
            cols[k + "_off"] = getattr(self, k + "_off")
            cols["n_" + k] = getattr(self, "n_" + k).astype(np.int64)
        size = sum(a.nbytes for a in cols.values()) / 2**30
        dev = torch.device(dev)
        where = dev if dev.type == "cuda" and size <= budget_gb else torch.device("cpu")
        self.store = {k: torch.from_numpy(np.ascontiguousarray(a)).to(where) for k, a in cols.items()}
        self.store_dev = where
        return where, size

    def _tgather(self, col, k, idx, width):
        st = self.store
        c = st["n_" + k][idx].clamp(max=width)
        w = max(int(c.max()), 1)
        j = torch.arange(w, device=idx.device)
        mask = j[None, :] < c[:, None]
        pos = torch.where(mask, st[k + "_off"][idx][:, None] + j[None, :], 0)
        flat = st[col]
        if flat.numel() == 0:
            return torch.zeros((len(idx), w) + tuple(flat.shape[1:]), dtype=flat.dtype, device=idx.device), mask
        return flat[pos], mask

    def batch(self, idx, device, trim=True):
        """The decisions `idx` as tensors on `device`, padded to the batch's longest row (or the fixed widths)"""
        if self.store is None:
            self.place("cpu")
        st, s = self.store, self.store_dev
        i = torch.as_tensor(np.asarray(idx), dtype=torch.long, device=s)
        ents, em = self._tgather("ents", "ent", i, N_ENT)
        eids, _ = self._tgather("ent_ids", "ent", i, N_ENT)
        acts, am = self._tgather("acts", "act", i, N_ACT)
        aids, _ = self._tgather("act_ids", "act", i, N_ACT)
        tgt, _ = self._tgather("target", "act", i, N_ACT)
        hid, hm = self._tgather("hid", "hid", i, N_HID)
        hids, _ = self._tgather("hid_ids", "hid", i, N_HID)
        pool, pm = self._tgather("pool", "pool", i, N_POOL)
        ph, _ = self._tgather("pool_hand", "pool", i, N_POOL)
        raw = {"glob": st["glob"][i], "ents": ents, "em": em, "eids": eids, "acts": acts, "am": am, "aids": aids,
               "tgt": tgt, "hid": hid, "hm": hm, "hids": hids, "pool": pool, "pm": pm, "ph": ph,
               "chosen": st["chosen"][i], "logp": st["logp"][i], "z": st["z"][i]}
        if not trim:
            raw = self._pad(raw)
        device = torch.device(device)
        if device != s:
            nb = device.type == "cuda"
            raw = {k: (v.pin_memory() if nb else v).to(device, non_blocking=nb) for k, v in raw.items()}
        f, r = torch.float32, raw
        em, am, hm, pm = r["em"], r["am"], r["hm"], r["pm"]
        tgt = torch.where(am, r["tgt"].to(f), 0)
        tgt = tgt / tgt.sum(1, keepdim=True).clamp(min=1e-6)
        return {
            "glob": r["glob"].to(f),
            "ents": r["ents"].to(f) * em[..., None],
            "ent_ids": r["eids"].long() * em,
            "ent_mask": em,
            "acts": r["acts"].to(f) * am[..., None],
            "act_ids": r["aids"].long() * am,
            "act_mask": am,
            "target": tgt,
            "chosen": r["chosen"].clamp(max=am.shape[1] - 1),
            "logp": r["logp"].to(f),
            "hid": r["hid"].to(f) * hm[..., None],
            "hid_ids": r["hids"].long() * hm,
            "hid_mask": hm,
            "pool": r["pool"].long() * pm,
            "pool_hand": r["ph"].to(f) * pm,
            "pool_mask": pm,
            "z": r["z"].to(f),
        }

    @staticmethod
    def _pad(raw):
        """Pads the trimmed rows back to the fixed widths (`batch_np`'s shapes)"""
        widths = {"ents": N_ENT, "em": N_ENT, "eids": N_ENT, "acts": N_ACT, "am": N_ACT, "aids": N_ACT,
                  "tgt": N_ACT, "hid": N_HID, "hm": N_HID, "hids": N_HID, "pool": N_POOL, "pm": N_POOL, "ph": N_POOL}
        out = dict(raw)
        for k, w in widths.items():
            v = raw[k]
            pad = w - v.shape[1]
            if pad > 0:
                z = torch.zeros((v.shape[0], pad) + tuple(v.shape[2:]), dtype=v.dtype, device=v.device)
                out[k] = torch.cat([v, z], 1)
        return out

    def __len__(self):
        return self.n

    @staticmethod
    def _gather(flat, off, counts, idx, width):
        """Rows `off[i]..off[i]+counts[i]` of `flat` for each i in idx, padded to `width`: values and a mask"""
        j = np.arange(width)
        c = np.minimum(counts[idx], width)
        mask = j[None, :] < c[:, None]
        pos = np.where(mask, off[idx][:, None] + j[None, :], 0)
        if len(flat) == 0:
            shape = (len(idx), width) + flat.shape[1:]
            return np.zeros(shape, flat.dtype), mask
        return flat[pos], mask

    def batch_np(self, idx, device):
        t = lambda a, dt=torch.float32: torch.from_numpy(np.ascontiguousarray(a)).to(device=device, dtype=dt)
        ents, em = self._gather(self.ents, self.ent_off, self.n_ent, idx, N_ENT)
        eids, _ = self._gather(self.ent_ids, self.ent_off, self.n_ent, idx, N_ENT)
        acts, am = self._gather(self.acts, self.act_off, self.n_act, idx, N_ACT)
        aids, _ = self._gather(self.act_ids, self.act_off, self.n_act, idx, N_ACT)
        tgt, _ = self._gather(self.target, self.act_off, self.n_act, idx, N_ACT)
        hid, hm = self._gather(self.hid, self.hid_off, self.n_hid, idx, N_HID)
        hids, _ = self._gather(self.hid_ids, self.hid_off, self.n_hid, idx, N_HID)
        pool, pm = self._gather(self.pool, self.pool_off, self.n_pool, idx, N_POOL)
        ph, _ = self._gather(self.pool_hand, self.pool_off, self.n_pool, idx, N_POOL)
        tgt = np.where(am, tgt.astype(np.float32), 0)
        tgt = tgt / np.maximum(tgt.sum(1, keepdims=True), 1e-6)
        return {
            "glob": t(self.glob[idx]),
            "ents": t(ents * em[..., None]),
            "ent_ids": t(eids * em, torch.long),
            "ent_mask": t(em, torch.bool),
            "acts": t(acts * am[..., None]),
            "act_ids": t(aids * am, torch.long),
            "act_mask": t(am, torch.bool),
            "target": t(tgt),
            "chosen": t(np.minimum(self.chosen[idx], N_ACT - 1), torch.long),
            "logp": t(self.logp[idx]),
            "hid": t(hid * hm[..., None]),
            "hid_ids": t(hids * hm, torch.long),
            "hid_mask": t(hm, torch.bool),
            "pool": t(pool * pm, torch.long),
            "pool_hand": t(ph * pm),
            "pool_mask": t(pm, torch.bool),
            "z": t(self.z[idx]),
        }
