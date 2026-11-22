"""Reads what `selfplay` writes (playmat-ai `selfplay::Shard`) and makes padded batches of tensors.

Improvements over the first loader:
- `compact_dir(dir)` merges hundreds of tiny fleet chunk shards into one `merged.*` shard + binary `merged.games.npz`,
  cutting disk read time from ~15 min to ~2-4 s.
- `load_critic=False` skips reading/storing `hid`/`hid_ids` when `Critic` isn't trained (saves ~25% RAM/VRAM).
- IDs are stored in `int16` (`BUCKETS = 4096`) instead of `int32`, halving ID memory.
- Supports optional `qval.f16` (root search Q-value) and computes training-only auxiliary targets (`turns_left`,
  `final_life_diff`) plus inverse-sqrt-game-length value weights (`val_w`).
- `stage_blocks(idx_blocks, dev)` stages a multi-million-decision block subset directly into GPU VRAM in <1 s per
  epoch when a wide replay buffer exceeds VRAM, so every training step stays 100% GPU-resident.
"""

import argparse
from concurrent.futures import ThreadPoolExecutor
import glob
import json
import os
import time

import numpy as np
import torch

from model import ACT_F, ENT_F, GLOB_F

META = 10
N_ENT, N_ACT, N_HID, N_POOL = 64, 16, 16, 64

# GLOB_F layout in obs.rs: 17 fitted features, then [turn/10, me.life/20, them.life/20, ...]
GLOB_LIFE_ME = 17 + 1
GLOB_LIFE_OPP = 17 + 2

FIXED_COLS = [
    ("meta.i32", np.int32),
    ("logp.f32", np.float32),
    ("glob.f16", np.float16),
    ("ents.f16", np.float16),
    ("ent_ids.u16", np.uint16),
    ("acts.f16", np.float16),
    ("act_ids.u16", np.uint16),
    ("target.f16", np.float16),
    ("hid.f16", np.float16),
    ("hid_ids.u16", np.uint16),
    ("pool.u16", np.uint16),
    ("pool_hand.u8", np.uint8),
]


def _read(path, dtype):
    return np.fromfile(path, dtype=dtype) if os.path.exists(path) else np.zeros(0, dtype)


def _offsets(counts):
    o = np.zeros(len(counts) + 1, np.int64)
    np.cumsum(counts, out=o[1:])
    return o


def _shard_bases(d):
    bases = []
    for meta_path in sorted(glob.glob(os.path.join(d, "*.meta.i32"))):
        base = meta_path[: -len(".meta.i32")]
        if "._" in os.path.basename(base):  # Mac AppleDouble metadata, not a shard
            continue
        bases.append(base)
    return bases


def _read_games_meta(base):
    """Reads (game_ids, results[N, 2], turns[N]) from binary `.games.npz` or `.games.jsonl`."""
    npz_path = base + ".games.npz"
    if os.path.exists(npz_path):
        z = np.load(npz_path)
        return z["game"], z["result"], z["turns"]
    games, results, turns = [], [], []
    jpath = base + ".games.jsonl"
    if os.path.exists(jpath):
        with open(jpath) as f:
            for line in f:
                if not line.strip():
                    continue
                g = json.loads(line)
                games.append(int(g["game"]))
                results.append(g["result"])
                turns.append(int(g.get("turns", 10)))
    return (
        np.asarray(games, dtype=np.int32),
        np.asarray(results, dtype=np.float32).reshape(-1, 2) if results else np.zeros((0, 2), np.float32),
        np.asarray(turns, dtype=np.int16),
    )


def compact_dir(d, delete_chunks=True, min_shards=4):
    """Merges many small shards in `d` into one `merged.*` shard + binary `merged.games.npz`.
    Safe to re-run: if `d` already has <= `min_shards` shards (and `.games.npz` when single), does nothing."""
    bases = _shard_bases(d)
    if not bases:
        return False
    merged_base = os.path.join(d, "merged")
    if len(bases) == 1 and os.path.normpath(bases[0]) == os.path.normpath(merged_base) and os.path.exists(merged_base + ".games.npz"):
        return False
    if len(bases) <= min_shards and all(os.path.exists(b + ".games.npz") for b in bases):
        return False
    t0 = time.time()
    tmp = os.path.join(d, "_compact_tmp")
    any_qval = any(os.path.exists(b + ".qval.f16") for b in bases)
    col_names = [ext for ext, _ in FIXED_COLS]
    if any_qval:
        col_names.append("qval.f16")
    out_files = {ext: open(f"{tmp}.{ext}", "wb") for ext in col_names}
    all_games, all_res, all_turns = [], [], []
    n_steps = 0
    try:
        for base in bases:
            meta = _read(base + ".meta.i32", np.int32).reshape(-1, META)
            if len(meta) == 0:
                continue
            n = len(meta)
            n_steps += n
            for ext, _ in FIXED_COLS:
                p = f"{base}.{ext}"
                if os.path.exists(p):
                    with open(p, "rb") as src:
                        while True:
                            buf = src.read(4 << 20)
                            if not buf:
                                break
                            out_files[ext].write(buf)
            if any_qval:
                qp = f"{base}.qval.f16"
                if os.path.exists(qp):
                    with open(qp, "rb") as src:
                        out_files["qval.f16"].write(src.read())
                else:
                    np.full(n, np.nan, dtype=np.float16).tofile(out_files["qval.f16"])
            g, r, t = _read_games_meta(base)
            if len(g):
                all_games.append(g)
                all_res.append(r)
                all_turns.append(t)
    finally:
        for f in out_files.values():
            f.close()
    if n_steps == 0:
        for ext in col_names:
            p = f"{tmp}.{ext}"
            if os.path.exists(p):
                os.remove(p)
        return False
    np.savez(
        tmp + ".games.npz",
        game=np.concatenate(all_games) if all_games else np.zeros(0, np.int32),
        result=np.concatenate(all_res) if all_res else np.zeros((0, 2), np.float32),
        turns=np.concatenate(all_turns) if all_turns else np.zeros(0, np.int16),
    )
    # Remove old chunk files before renaming tmp -> merged (note: merged may be one of bases)
    if delete_chunks:
        for base in bases:
            for ext in col_names + ["games.jsonl", "games.npz"]:
                p = f"{base}.{ext}"
                if os.path.exists(p):
                    os.remove(p)
    # Rename atomically, meta.i32 last so a crashed run never looks complete
    for ext in [c for c in col_names if c != "meta.i32"] + ["games.npz", "meta.i32"]:
        os.replace(f"{tmp}.{ext}", f"{merged_base}.{ext}")
    print(f"COMPACT {d}: {len(bases)} shards -> 1 ({n_steps} decisions) in {time.time() - t0:.1f} s", flush=True)
    return True


def _read_one_shard(args):
    r, base, load_critic = args
    meta = _read(base + ".meta.i32", np.int32).reshape(-1, META)
    if len(meta) == 0:
        return None
    n = len(meta)
    d = {
        "r": r,
        "meta": meta,
        "logp": _read(base + ".logp.f32", np.float32),
        "glob": _read(base + ".glob.f16", np.float16).reshape(-1, GLOB_F),
        "ents": _read(base + ".ents.f16", np.float16).reshape(-1, ENT_F),
        "ent_ids": _read(base + ".ent_ids.u16", np.uint16),
        "acts": _read(base + ".acts.f16", np.float16).reshape(-1, ACT_F),
        "act_ids": _read(base + ".act_ids.u16", np.uint16),
        "target": _read(base + ".target.f16", np.float16),
        "pool": _read(base + ".pool.u16", np.uint16),
        "pool_hand": _read(base + ".pool_hand.u8", np.uint8),
    }
    qp = base + ".qval.f16"
    d["qval"] = _read(qp, np.float16) if os.path.exists(qp) else np.full(n, np.nan, dtype=np.float16)
    if load_critic:
        d["hid"] = _read(base + ".hid.f16", np.float16).reshape(-1, ENT_F)
        d["hid_ids"] = _read(base + ".hid_ids.u16", np.uint16)
    d["games"], d["results"], d["turns"] = _read_games_meta(base)
    return d


class Data:
    """Every decision in the shards under `dirs`, with each game's result and derived auxiliary targets."""

    def __init__(self, dirs, load_critic=True, workers=8):
        self.load_critic = load_critic
        self.num_dirs = len(dirs)
        tasks = []
        for r, d in enumerate(dirs):
            for base in _shard_bases(d):
                tasks.append((r, base, load_critic))
        if workers > 1 and len(tasks) > 4:
            with ThreadPoolExecutor(max_workers=min(workers, len(tasks))) as ex:
                parts = [p for p in ex.map(_read_one_shard, tasks) if p is not None]
        else:
            parts = [p for p in (_read_one_shard(t) for t in tasks) if p is not None]

        col_keys = ["meta", "logp", "glob", "ents", "ent_ids", "acts", "act_ids", "target", "pool", "pool_hand", "qval"]
        if load_critic:
            col_keys += ["hid", "hid_ids"]
        for k in col_keys:
            vals = [p[k] for p in parts]
            setattr(self, k, np.concatenate(vals) if vals else np.zeros(0))
        if not load_critic:
            self.hid = np.zeros((0, ENT_F), dtype=np.float16)
            self.hid_ids = np.zeros(0, dtype=np.uint16)

        self.run = np.concatenate([np.full(len(p["meta"]), p["r"], np.int32) for p in parts]) if parts else np.zeros(0, np.int32)
        m = self.meta
        self.n = len(m)
        if self.n == 0:
            raise ValueError(f"no decisions found in {dirs}")
        self.game, self.seat, self.turn, self.chosen = m[:, 0], m[:, 1], m[:, 2], m[:, 3]
        self.n_ent, self.n_act, self.n_hid, self.n_pool = m[:, 4], m[:, 5], m[:, 6], m[:, 7]
        if not load_critic:
            self.n_hid = np.zeros_like(self.n_hid)
        self.qkind, self.tag = m[:, 8], m[:, 9]
        self.ent_off, self.act_off = _offsets(self.n_ent), _offsets(self.n_act)
        self.hid_off, self.pool_off = _offsets(self.n_hid), _offsets(self.n_pool)

        # Vectorized lookup of game result & turn count per decision (no Python loop over 7M steps!)
        res_map = {}
        turn_map = {}
        infos = {}
        for p in parts:
            r = p["r"]
            for g, res, trn in zip(p["games"].tolist(), p["results"].tolist(), p["turns"].tolist()):
                k = (r, g)
                res_map[k] = res
                turn_map[k] = trn
                infos[k] = {"game": g, "result": res, "turns": trn}
        self.infos = infos

        # Map each decision's (run, game) to an integer index in a compact table
        game_key = self.run.astype(np.int64) * 10**9 + self.game.astype(np.int64)
        uniq_keys, inv = np.unique(game_key, return_inverse=True)
        u_run = uniq_keys // 10**9
        u_game = uniq_keys % 10**9
        u_res = np.array([res_map[(int(r), int(g))] for r, g in zip(u_run, u_game)], dtype=np.float32)
        u_turns = np.array([turn_map[(int(r), int(g))] for r, g in zip(u_run, u_game)], dtype=np.float32)
        self.z = u_res[inv, self.seat].astype(np.float32)
        g_turns = u_turns[inv]
        self.turns_left = np.clip((g_turns - self.turn.astype(np.float32)) / 15.0, 0.0, 2.0).astype(np.float32)

        # Inverse-sqrt game-length weights for value loss (prevents long games from overfitting)
        counts = np.bincount(inv).astype(np.float32)
        w = 1.0 / np.sqrt(np.maximum(counts[inv], 1.0))
        self.val_w = (w / w.mean()).astype(np.float32)

        # Auxiliary target 2: seat's life differential at the final recorded step of the game
        # Last step per (run, game, seat): overwrite in forward order (within each shard games are chronological)
        seat_key = game_key * 2 + self.seat.astype(np.int64)
        _, s_inv = np.unique(seat_key, return_inverse=True)
        life_diff = (self.glob[:, GLOB_LIFE_ME].astype(np.float32) - self.glob[:, GLOB_LIFE_OPP].astype(np.float32))
        last_life = np.zeros(s_inv.max() + 1, dtype=np.float32)
        last_life[s_inv] = life_diff
        self.final_life_diff = np.clip(last_life[s_inv], -2.0, 2.0).astype(np.float32)

        self.store = None
        self.store_dev = None
        self._pinned = False

    def place(self, dev, budget_gb=12.0, pin_cpu=False):
        """Puts the columns in torch tensors: on `dev` when it is a GPU and they fit in `budget_gb`, else on CPU."""
        cols = {
            "glob": self.glob,
            "ents": self.ents,
            "ent_ids": self.ent_ids.astype(np.int16),
            "acts": self.acts,
            "act_ids": self.act_ids.astype(np.int16),
            "target": self.target,
            "pool": self.pool.astype(np.int16),
            "pool_hand": self.pool_hand,
            "chosen": self.chosen.astype(np.int64),
            "logp": self.logp,
            "z": self.z,
            "qval": self.qval,
            "turns_left": self.turns_left,
            "final_life_diff": self.final_life_diff,
            "val_w": self.val_w,
            "run": self.run,
        }
        if self.load_critic:
            cols["hid"] = self.hid
            cols["hid_ids"] = self.hid_ids.astype(np.int16)
        for k in ["ent", "act", "pool"] + (["hid"] if self.load_critic else []):
            cols[k + "_off"] = getattr(self, k + "_off")
            cols["n_" + k] = getattr(self, "n_" + k).astype(np.int64)
        size = sum(a.nbytes for a in cols.values()) / 2**30
        dev = torch.device(dev)
        where = dev if dev.type == "cuda" and size <= budget_gb else torch.device("cpu")
        self.store = {}
        for k, a in cols.items():
            t = torch.from_numpy(np.ascontiguousarray(a))
            if where.type == "cuda":
                t = t.to(where)
            elif pin_cpu and dev.type == "cuda":
                t = t.pin_memory()
            self.store[k] = t
        self.store_dev = where
        self._pinned = (where.type == "cpu" and pin_cpu and dev.type == "cuda")
        return where, size

    def stage_blocks(self, block_starts, block_len, dev):
        """Packs contiguous decision blocks `[s : s + block_len]` straight into a compact GPU-resident `StagedData`.
        Avoids scattered gather on CPU: copying 2M decisions in 512-step blocks takes <0.5 s over PCIe."""
        dev = torch.device(dev)
        ranges = [(int(s), min(int(s) + block_len, self.n)) for s in block_starts]
        ranges = [(a, b) for a, b in ranges if b > a]

        def cat_fixed(arr):
            return np.concatenate([arr[a:b] for a, b in ranges])

        def cat_var(flat, off, counts):
            c = np.concatenate([counts[a:b] for a, b in ranges])
            slices = [flat[off[a] : off[b]] for a, b in ranges]
            f = np.concatenate(slices) if slices else flat[:0]
            return f, _offsets(c), c.astype(np.int64)

        ents, ent_off, n_ent = cat_var(self.ents, self.ent_off, self.n_ent)
        eids, _, _ = cat_var(self.ent_ids, self.ent_off, self.n_ent)
        acts, act_off, n_act = cat_var(self.acts, self.act_off, self.n_act)
        aids, _, _ = cat_var(self.act_ids, self.act_off, self.n_act)
        tgt, _, _ = cat_var(self.target, self.act_off, self.n_act)
        pool, pool_off, n_pool = cat_var(self.pool, self.pool_off, self.n_pool)
        ph, _, _ = cat_var(self.pool_hand, self.pool_off, self.n_pool)

        cols = {
            "glob": cat_fixed(self.glob),
            "ents": ents,
            "ent_ids": eids.astype(np.int16),
            "ent_off": ent_off,
            "n_ent": n_ent,
            "acts": acts,
            "act_ids": aids.astype(np.int16),
            "target": tgt,
            "act_off": act_off,
            "n_act": n_act,
            "pool": pool.astype(np.int16),
            "pool_hand": ph,
            "pool_off": pool_off,
            "n_pool": n_pool,
            "chosen": cat_fixed(self.chosen).astype(np.int64),
            "logp": cat_fixed(self.logp),
            "z": cat_fixed(self.z),
            "qval": cat_fixed(self.qval),
            "turns_left": cat_fixed(self.turns_left),
            "final_life_diff": cat_fixed(self.final_life_diff),
            "val_w": cat_fixed(self.val_w),
            "run": cat_fixed(self.run),
            "game": cat_fixed(self.game),
        }
        if self.load_critic:
            hid, hid_off, n_hid = cat_var(self.hid, self.hid_off, self.n_hid)
            hids, _, _ = cat_var(self.hid_ids, self.hid_off, self.n_hid)
            cols.update({"hid": hid, "hid_ids": hids.astype(np.int16), "hid_off": hid_off, "n_hid": n_hid})
        store = {k: torch.from_numpy(np.ascontiguousarray(v)).to(dev) for k, v in cols.items()}
        return _View(store, dev, self.load_critic, len(cols["z"]))

    def _tgather(self, col, k, idx, width):
        return _tgather_store(self.store, col, k, idx, width)

    def batch(self, idx, device, trim=True):
        """The decisions `idx` as tensors on `device`, padded to the batch's longest row (or the fixed widths)."""
        if self.store is None:
            self.place("cpu")
        return _batch_from_store(self.store, self.store_dev, self.load_critic, idx, device, trim=trim)

    @staticmethod
    def _pad(raw):
        return _pad_raw(raw)

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
            "qval": t(self.qval[idx]),
            "turns_left": t(self.turns_left[idx]),
            "final_life_diff": t(self.final_life_diff[idx]),
            "val_w": t(self.val_w[idx]),
            "run": t(self.run[idx], torch.long),
        }


class _View:
    """A GPU-staged subset of `Data`, exposing the same `.batch(idx, dev)` interface."""

    def __init__(self, store, dev, load_critic, n):
        self.store = store
        self.store_dev = dev
        self.load_critic = load_critic
        self.n = n

    def __len__(self):
        return self.n

    def batch(self, idx, device, trim=True):
        return _batch_from_store(self.store, self.store_dev, self.load_critic, idx, device, trim=trim)


def _tgather_store(st, col, k, idx, width):
    c = st["n_" + k][idx].clamp(max=width)
    w = max(int(c.max()), 1)
    j = torch.arange(w, device=idx.device)
    mask = j[None, :] < c[:, None]
    pos = torch.where(mask, st[k + "_off"][idx][:, None] + j[None, :], 0)
    flat = st[col]
    if flat.numel() == 0:
        return torch.zeros((len(idx), w) + tuple(flat.shape[1:]), dtype=flat.dtype, device=idx.device), mask
    return flat[pos], mask


def _pad_raw(raw):
    widths = {"ents": N_ENT, "em": N_ENT, "eids": N_ENT, "acts": N_ACT, "am": N_ACT, "aids": N_ACT,
              "tgt": N_ACT, "pool": N_POOL, "pm": N_POOL, "ph": N_POOL}
    if "hid" in raw:
        widths.update({"hid": N_HID, "hm": N_HID, "hids": N_HID})
    out = dict(raw)
    for k, w in widths.items():
        v = raw[k]
        pad = w - v.shape[1]
        if pad > 0:
            z = torch.zeros((v.shape[0], pad) + tuple(v.shape[2:]), dtype=v.dtype, device=v.device)
            out[k] = torch.cat([v, z], 1)
    return out


def _batch_from_store(st, s, load_critic, idx, device, trim=True):
    if isinstance(idx, torch.Tensor):
        i = idx.to(device=s, dtype=torch.long)
    else:
        i = torch.as_tensor(np.asarray(idx), dtype=torch.long, device=s)
    ents, em = _tgather_store(st, "ents", "ent", i, N_ENT)
    eids, _ = _tgather_store(st, "ent_ids", "ent", i, N_ENT)
    acts, am = _tgather_store(st, "acts", "act", i, N_ACT)
    aids, _ = _tgather_store(st, "act_ids", "act", i, N_ACT)
    tgt, _ = _tgather_store(st, "target", "act", i, N_ACT)
    pool, pm = _tgather_store(st, "pool", "pool", i, N_POOL)
    ph, _ = _tgather_store(st, "pool_hand", "pool", i, N_POOL)
    raw = {
        "glob": st["glob"][i], "ents": ents, "em": em, "eids": eids,
        "acts": acts, "am": am, "aids": aids, "tgt": tgt,
        "pool": pool, "pm": pm, "ph": ph,
        "chosen": st["chosen"][i], "logp": st["logp"][i], "z": st["z"][i],
        "qval": st["qval"][i], "turns_left": st["turns_left"][i],
        "final_life_diff": st["final_life_diff"][i], "val_w": st["val_w"][i], "run": st["run"][i],
    }
    if load_critic:
        hid, hm = _tgather_store(st, "hid", "hid", i, N_HID)
        hids, _ = _tgather_store(st, "hid_ids", "hid", i, N_HID)
        raw.update({"hid": hid, "hm": hm, "hids": hids})
    if not trim:
        raw = _pad_raw(raw)
    device = torch.device(device)
    if device != s:
        raw = {k: v.to(device) for k, v in raw.items()}
    f, r = torch.float32, raw
    em, am, pm = r["em"], r["am"], r["pm"]
    tgt = torch.where(am, r["tgt"].to(f), 0)
    tgt = tgt / tgt.sum(1, keepdim=True).clamp(min=1e-6)
    out = {
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
        "pool": r["pool"].long() * pm,
        "pool_hand": r["ph"].to(f) * pm,
        "pool_mask": pm,
        "z": r["z"].to(f),
        "qval": r["qval"].to(f),
        "turns_left": r["turns_left"].to(f),
        "final_life_diff": r["final_life_diff"].to(f),
        "val_w": r["val_w"].to(f),
        "run": r["run"].long(),
    }
    if load_critic:
        hm = r["hm"]
        out.update({
            "hid": r["hid"].to(f) * hm[..., None],
            "hid_ids": r["hids"].long() * hm,
            "hid_mask": hm,
        })
    return out


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("--compact", nargs="+", help="merge fleet chunk shards in each dir into one merged.* shard")
    ap.add_argument("--keep-chunks", action="store_true")
    args = ap.parse_args()
    for d in args.compact or []:
        compact_dir(d, delete_chunks=not args.keep_chunks)
