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
try:
    import torch
    from model import ACT_F, BELIEF_META_F, ENT_F, GLOB_F, GLOB_F_V1, GLOB_F_V2, NO_ENT
except ImportError:
    torch = None
    ENT_F, GLOB_F_V1, GLOB_F_V2, GLOB_F_V3, GLOB_F, ACT_F, NO_ENT, BELIEF_META_F = 49, 65, 81, 89, 97, 65, 255, 8

META = 10
N_ENT, N_ACT, N_HID, N_POOL = 80, 24, 16, 64

# GLOB_F layout in obs.rs: 17 fitted features, then [turn/10, me.life/20, them.life/20, ...]
GLOB_LIFE_ME = 17 + 1
GLOB_LIFE_OPP = 17 + 2
GLOB_HAND_ME = 17 + 3
GLOB_HAND_OPP = 17 + 4

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


def _read(path, dtype, count=-1):
    return np.fromfile(path, dtype=dtype, count=count) if os.path.exists(path) else np.zeros(0, dtype)


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
    any_rel = any(os.path.exists(b + ".ent_rel_idx.u8") for b in bases)
    any_v3 = any(os.path.exists(b + ".act_q.f16") for b in bases)
    col_names = [ext for ext, _ in FIXED_COLS]
    if any_qval:
        col_names.append("qval.f16")
    if any_rel:
        col_names += ["ent_rel_idx.u8", "ent_rel_type.u8", "act_ent_idx.u8"]
    if any_v3:
        col_names += ["act_q.f16", "act_n.u16", "belief_meta.f16"]
    max_g_w = GLOB_F_V1
    for base in bases:
        mp = base + ".meta.i32"
        gp = base + ".glob.f16"
        if os.path.exists(mp) and os.path.exists(gp):
            cnt = os.path.getsize(mp) // (META * 4)
            if cnt > 0:
                max_g_w = max(max_g_w, os.path.getsize(gp) // (2 * cnt))
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
                if ext == "glob.f16" and os.path.exists(p):
                    g_w = os.path.getsize(p) // (2 * n)
                    if g_w != max_g_w:
                        ga = _read(p, np.float16, count=n * g_w).reshape(n, g_w)
                        if g_w < max_g_w:
                            ga = np.concatenate([ga, np.zeros((n, max_g_w - g_w), dtype=np.float16)], axis=1)
                        else:
                            ga = ga[:, :max_g_w]
                        ga.tofile(out_files[ext])
                        continue
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
            if any_rel:
                c_ent = int(meta[:, 4].sum())
                c_act = int(meta[:, 5].sum())
                for ext, cnt, fill in [
                    ("ent_rel_idx.u8", c_ent, NO_ENT),
                    ("ent_rel_type.u8", c_ent, 0),
                    ("act_ent_idx.u8", c_act, NO_ENT),
                ]:
                    rp = f"{base}.{ext}"
                    if os.path.exists(rp):
                        with open(rp, "rb") as src:
                            out_files[ext].write(src.read())
                    else:
                        np.full(cnt, fill, dtype=np.uint8).tofile(out_files[ext])
            if any_v3:
                c_act = int(meta[:, 5].sum())
                for ext, cnt, dt, fill in [
                    ("act_q.f16", c_act, np.float16, np.nan),
                    ("act_n.u16", c_act, np.uint16, 0),
                    ("belief_meta.f16", n * BELIEF_META_F, np.float16, np.nan),
                ]:
                    vp = f"{base}.{ext}"
                    if os.path.exists(vp):
                        with open(vp, "rb") as src:
                            out_files[ext].write(src.read())
                    else:
                        np.full(cnt, fill, dtype=dt).tofile(out_files[ext])
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
    r, base, load_critic, max_steps = args
    meta_path = base + ".meta.i32"
    total_n = os.path.getsize(meta_path) // (META * 4) if os.path.exists(meta_path) else 0
    meta = _read(meta_path, np.int32, count=max_steps * META if max_steps > 0 else -1).reshape(-1, META)
    if len(meta) == 0:
        return None
    n = len(meta)
    c_ent = int(meta[:, 4].sum())
    c_act = int(meta[:, 5].sum())
    c_hid = int(meta[:, 6].sum())
    c_pool = int(meta[:, 7].sum())
    # Auto-detect v1 (GLOB_F_V1=65) vs v2 (GLOB_F=81) glob.f16 width, padding legacy shards cleanly with zeros
    gp = base + ".glob.f16"
    g_w = (os.path.getsize(gp) // (2 * total_n)) if (os.path.exists(gp) and total_n > 0) else GLOB_F
    glob_arr = _read(gp, np.float16, count=n * g_w).reshape(-1, g_w)
    if g_w < GLOB_F:
        pad = np.zeros((len(glob_arr), GLOB_F - g_w), dtype=np.float16)
        glob_arr = np.concatenate([glob_arr, pad], axis=1)
    elif g_w > GLOB_F:
        glob_arr = glob_arr[:, :GLOB_F]
    d = {
        "r": r,
        "meta": meta,
        "logp": _read(base + ".logp.f32", np.float32, count=n),
        "glob": glob_arr,
        "ents": _read(base + ".ents.f16", np.float16, count=c_ent * ENT_F).reshape(-1, ENT_F),
        "ent_ids": _read(base + ".ent_ids.u16", np.uint16, count=c_ent),
        "ent_rel_idx": _read(base + ".ent_rel_idx.u8", np.uint8, count=c_ent)
        if os.path.exists(base + ".ent_rel_idx.u8")
        else np.full(c_ent, NO_ENT, dtype=np.uint8),
        "ent_rel_type": _read(base + ".ent_rel_type.u8", np.uint8, count=c_ent)
        if os.path.exists(base + ".ent_rel_type.u8")
        else np.zeros(c_ent, dtype=np.uint8),
        "acts": _read(base + ".acts.f16", np.float16, count=c_act * ACT_F).reshape(-1, ACT_F),
        "act_ids": _read(base + ".act_ids.u16", np.uint16, count=c_act),
        "act_ent_idx": _read(base + ".act_ent_idx.u8", np.uint8, count=c_act)
        if os.path.exists(base + ".act_ent_idx.u8")
        else np.full(c_act, NO_ENT, dtype=np.uint8),
        "target": _read(base + ".target.f16", np.float16, count=c_act),
        "pool": _read(base + ".pool.u16", np.uint16, count=c_pool),
        "pool_hand": _read(base + ".pool_hand.u8", np.uint8, count=c_pool),
    }
    qp = base + ".qval.f16"
    d["qval"] = _read(qp, np.float16, count=n) if os.path.exists(qp) else np.full(n, np.nan, dtype=np.float16)
    aqp = base + ".act_q.f16"
    d["act_q"] = _read(aqp, np.float16, count=c_act) if os.path.exists(aqp) else np.full(c_act, np.nan, dtype=np.float16)
    anp = base + ".act_n.u16"
    d["act_n"] = _read(anp, np.uint16, count=c_act) if os.path.exists(anp) else np.zeros(c_act, dtype=np.uint16)
    bmp = base + ".belief_meta.f16"
    d["belief_meta"] = (
        _read(bmp, np.float16, count=n * BELIEF_META_F).reshape(-1, BELIEF_META_F)
        if (os.path.exists(bmp) and g_w >= GLOB_F)
        else np.full((n, BELIEF_META_F), np.nan, dtype=np.float16)
    )
    if load_critic:
        d["hid"] = _read(base + ".hid.f16", np.float16, count=c_hid * ENT_F).reshape(-1, ENT_F)
        d["hid_ids"] = _read(base + ".hid_ids.u16", np.uint16, count=c_hid)
    d["games"], d["results"], d["turns"] = _read_games_meta(base)
    return d


class Data:
    """Every decision in the shards under `dirs`, with each game's result and derived auxiliary targets."""

    def __init__(self, dirs, load_critic=True, workers=8, max_per_dir=0, max_old_per_dir=0, policy_recent=0, td_lambda=0.0):
        self.load_critic = load_critic
        self.num_dirs = len(dirs)
        pol_min = max(0, len(dirs) - policy_recent) if policy_recent > 0 else 0
        tasks = []
        for r, d in enumerate(dirs):
            bases = _shard_bases(d)
            if not bases:
                continue
            lim = max_old_per_dir if (max_old_per_dir > 0 and r < pol_min) else max_per_dir
            per_shard = (lim + len(bases) - 1) // len(bases) if lim > 0 else 0
            for base in bases:
                tasks.append((r, base, load_critic, per_shard))
        if workers > 1 and len(tasks) > 4:
            with ThreadPoolExecutor(max_workers=min(workers, len(tasks))) as ex:
                parts = [p for p in ex.map(_read_one_shard, tasks) if p is not None]
        else:
            parts = [p for p in (_read_one_shard(t) for t in tasks) if p is not None]

        col_keys = [
            "meta", "logp", "glob", "ents", "ent_ids", "ent_rel_idx", "ent_rel_type",
            "acts", "act_ids", "act_ent_idx", "target", "act_q", "act_n", "pool", "pool_hand", "qval", "belief_meta",
        ]
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
        # Auxiliary targets 3 & 4 (KataGo-style short-horizon deltas): life swing & hand+board swing to next decision
        board_adv = (self.glob[:, GLOB_HAND_ME].astype(np.float32) - self.glob[:, GLOB_HAND_OPP].astype(np.float32)) + 0.5 * self.glob[:, 0].astype(np.float32)
        s_ord = np.argsort(seat_key, kind="mergesort")
        sk_ord = seat_key[s_ord]
        same_n = np.empty(len(sk_ord), dtype=bool)
        same_n[:-1] = sk_ord[:-1] == sk_ord[1:]
        same_n[-1] = False
        ld_ord = life_diff[s_ord]
        ba_ord = board_adv[s_ord]
        nld = np.zeros(len(sk_ord), dtype=np.float32)
        nbd = np.zeros(len(sk_ord), dtype=np.float32)
        nld[:-1] = np.where(same_n[:-1], ld_ord[1:] - ld_ord[:-1], 0.0)
        nbd[:-1] = np.where(same_n[:-1], ba_ord[1:] - ba_ord[:-1], 0.0)
        self.next_life_delta = np.empty(len(sk_ord), dtype=np.float32)
        self.next_board_delta = np.empty(len(sk_ord), dtype=np.float32)
        self.next_life_delta[s_ord] = np.clip(nld, -1.0, 1.0)
        self.next_board_delta[s_ord] = np.clip(nbd, -1.0, 1.0)

        # Forward-looking TD(lambda) value target across each (run, game, seat) trajectory via O(N log L) suffix scan.
        # G_t = (1 - lam) * q_{t+1} + lam * G_{t+1} when next step has finite search Q, else G_{t+1}; G_{T-1} = z.
        if td_lambda > 0.0:
            order = np.argsort(seat_key, kind="mergesort")
            sk = seat_key[order]
            qv_s = self.qval[order].astype(np.float32)
            zv_s = self.z[order]
            n_all = len(sk)
            same_next = np.empty(n_all, dtype=bool)
            same_next[:-1] = sk[:-1] == sk[1:]
            same_next[-1] = False
            next_q = np.empty(n_all, dtype=np.float32)
            next_q[:-1] = qv_s[1:]
            next_q[-1] = np.nan
            has_nq = same_next & np.isfinite(next_q)
            nq_safe = np.where(has_nq, np.clip(next_q, 0.0, 1.0), 0.0)
            lam = float(td_lambda)
            a = np.where(has_nq, lam, np.where(same_next, 1.0, 0.0)).astype(np.float32)
            b = np.where(has_nq, (1.0 - lam) * nq_safe, np.where(same_next, 0.0, zv_s)).astype(np.float32)
            step = 1
            while step < n_all:
                b[:-step] = b[:-step] + a[:-step] * b[step:]
                a[:-step] = a[:-step] * a[step:]
                if not np.any(a > 1e-6):
                    break
                step <<= 1
            z_td = np.empty(n_all, dtype=np.float32)
            z_td[order] = np.clip(b, 0.0, 1.0)
            self.z_td = z_td
        else:
            self.z_td = self.z

        self.store = None
        self.store_dev = None
        self._pinned = False

    def place(self, dev, budget_gb=12.0, pin_cpu=False):
        """Puts the columns in torch tensors: on `dev` when it is a GPU and they fit in `budget_gb`, else on CPU."""
        cols = {
            "glob": self.glob,
            "ents": self.ents,
            "ent_ids": self.ent_ids.astype(np.int16),
            "ent_rel_idx": self.ent_rel_idx,
            "ent_rel_type": self.ent_rel_type,
            "acts": self.acts,
            "act_ids": self.act_ids.astype(np.int16),
            "act_ent_idx": self.act_ent_idx,
            "target": self.target,
            "act_q": self.act_q,
            "act_n": self.act_n.astype(np.int32),
            "pool": self.pool.astype(np.int16),
            "pool_hand": self.pool_hand,
            "chosen": self.chosen.astype(np.int64),
            "logp": self.logp,
            "z": self.z,
            "z_td": self.z_td,
            "qval": self.qval,
            "turns_left": self.turns_left,
            "final_life_diff": self.final_life_diff,
            "next_life_delta": self.next_life_delta,
            "next_board_delta": self.next_board_delta,
            "belief_meta": self.belief_meta,
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
        eridx, _, _ = cat_var(self.ent_rel_idx, self.ent_off, self.n_ent)
        ertyp, _, _ = cat_var(self.ent_rel_type, self.ent_off, self.n_ent)
        acts, act_off, n_act = cat_var(self.acts, self.act_off, self.n_act)
        aids, _, _ = cat_var(self.act_ids, self.act_off, self.n_act)
        aent, _, _ = cat_var(self.act_ent_idx, self.act_off, self.n_act)
        tgt, _, _ = cat_var(self.target, self.act_off, self.n_act)
        act_q, _, _ = cat_var(self.act_q, self.act_off, self.n_act)
        act_n, _, _ = cat_var(self.act_n, self.act_off, self.n_act)
        pool, pool_off, n_pool = cat_var(self.pool, self.pool_off, self.n_pool)
        ph, _, _ = cat_var(self.pool_hand, self.pool_off, self.n_pool)

        cols = {
            "glob": cat_fixed(self.glob),
            "ents": ents,
            "ent_ids": eids.astype(np.int16),
            "ent_rel_idx": eridx,
            "ent_rel_type": ertyp,
            "ent_off": ent_off,
            "n_ent": n_ent,
            "acts": acts,
            "act_ids": aids.astype(np.int16),
            "act_ent_idx": aent,
            "target": tgt,
            "act_q": act_q,
            "act_n": act_n.astype(np.int32),
            "act_off": act_off,
            "n_act": n_act,
            "pool": pool.astype(np.int16),
            "pool_hand": ph,
            "pool_off": pool_off,
            "n_pool": n_pool,
            "chosen": cat_fixed(self.chosen).astype(np.int64),
            "logp": cat_fixed(self.logp),
            "z": cat_fixed(self.z),
            "z_td": cat_fixed(self.z_td),
            "qval": cat_fixed(self.qval),
            "turns_left": cat_fixed(self.turns_left),
            "final_life_diff": cat_fixed(self.final_life_diff),
            "next_life_delta": cat_fixed(self.next_life_delta),
            "next_board_delta": cat_fixed(self.next_board_delta),
            "belief_meta": cat_fixed(self.belief_meta),
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
        eridx, _ = self._gather(self.ent_rel_idx, self.ent_off, self.n_ent, idx, N_ENT)
        ertyp, _ = self._gather(self.ent_rel_type, self.ent_off, self.n_ent, idx, N_ENT)
        acts, am = self._gather(self.acts, self.act_off, self.n_act, idx, N_ACT)
        aids, _ = self._gather(self.act_ids, self.act_off, self.n_act, idx, N_ACT)
        aent, _ = self._gather(self.act_ent_idx, self.act_off, self.n_act, idx, N_ACT)
        tgt, _ = self._gather(self.target, self.act_off, self.n_act, idx, N_ACT)
        act_q, _ = self._gather(self.act_q, self.act_off, self.n_act, idx, N_ACT)
        act_n, _ = self._gather(self.act_n, self.act_off, self.n_act, idx, N_ACT)
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
            "ent_rel_idx": t(np.where(em, eridx, NO_ENT), torch.long),
            "ent_rel_type": t(np.where(em, ertyp, 0), torch.long),
            "acts": t(acts * am[..., None]),
            "act_ids": t(aids * am, torch.long),
            "act_mask": t(am, torch.bool),
            "act_ent_idx": t(np.where(am, aent, NO_ENT), torch.long),
            "target": t(tgt),
            "act_q": t(np.where(am, act_q.astype(np.float32), np.nan)),
            "act_n": t(act_n * am, torch.long),
            "chosen": t(np.minimum(self.chosen[idx], N_ACT - 1), torch.long),
            "logp": t(self.logp[idx]),
            "hid": t(hid * hm[..., None]),
            "hid_ids": t(hids * hm, torch.long),
            "hid_mask": t(hm, torch.bool),
            "pool": t(pool * pm, torch.long),
            "pool_hand": t(ph * pm),
            "pool_mask": t(pm, torch.bool),
            "z": t(self.z[idx]),
            "z_td": t(self.z_td[idx]),
            "qval": t(self.qval[idx]),
            "turns_left": t(self.turns_left[idx]),
            "final_life_diff": t(self.final_life_diff[idx]),
            "next_life_delta": t(self.next_life_delta[idx]),
            "next_board_delta": t(self.next_board_delta[idx]),
            "belief_meta": t(self.belief_meta[idx]),
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
    widths = {"ents": N_ENT, "em": N_ENT, "eids": N_ENT, "eridx": N_ENT, "ertyp": N_ENT,
              "acts": N_ACT, "am": N_ACT, "aids": N_ACT, "aent": N_ACT, "tgt": N_ACT, "act_q": N_ACT, "act_n": N_ACT,
              "pool": N_POOL, "pm": N_POOL, "ph": N_POOL}
    if "hid" in raw:
        widths.update({"hid": N_HID, "hm": N_HID, "hids": N_HID})
    out = dict(raw)
    for k, w in widths.items():
        v = raw[k]
        pad = w - v.shape[1]
        if pad > 0:
            fill = NO_ENT if k in ("eridx", "aent") else (float("nan") if k == "act_q" else 0)
            z = torch.full((v.shape[0], pad) + tuple(v.shape[2:]), fill, dtype=v.dtype, device=v.device)
            out[k] = torch.cat([v, z], 1)
    return out


def _batch_from_store(st, s, load_critic, idx, device, trim=True):
    if isinstance(idx, torch.Tensor):
        i = idx.to(device=s, dtype=torch.long)
    else:
        i = torch.as_tensor(np.asarray(idx), dtype=torch.long, device=s)
    ents, em = _tgather_store(st, "ents", "ent", i, N_ENT)
    eids, _ = _tgather_store(st, "ent_ids", "ent", i, N_ENT)
    eridx, _ = _tgather_store(st, "ent_rel_idx", "ent", i, N_ENT)
    ertyp, _ = _tgather_store(st, "ent_rel_type", "ent", i, N_ENT)
    acts, am = _tgather_store(st, "acts", "act", i, N_ACT)
    aids, _ = _tgather_store(st, "act_ids", "act", i, N_ACT)
    aent, _ = _tgather_store(st, "act_ent_idx", "act", i, N_ACT)
    tgt, _ = _tgather_store(st, "target", "act", i, N_ACT)
    act_q, _ = _tgather_store(st, "act_q", "act", i, N_ACT)
    act_n, _ = _tgather_store(st, "act_n", "act", i, N_ACT)
    pool, pm = _tgather_store(st, "pool", "pool", i, N_POOL)
    ph, _ = _tgather_store(st, "pool_hand", "pool", i, N_POOL)
    raw = {
        "glob": st["glob"][i], "ents": ents, "em": em, "eids": eids, "eridx": eridx, "ertyp": ertyp,
        "acts": acts, "am": am, "aids": aids, "aent": aent, "tgt": tgt, "act_q": act_q, "act_n": act_n,
        "pool": pool, "pm": pm, "ph": ph,
        "chosen": st["chosen"][i], "logp": st["logp"][i], "z": st["z"][i], "z_td": st["z_td"][i],
        "qval": st["qval"][i], "turns_left": st["turns_left"][i],
        "final_life_diff": st["final_life_diff"][i], "next_life_delta": st["next_life_delta"][i], "next_board_delta": st["next_board_delta"][i], "belief_meta": st["belief_meta"][i], "val_w": st["val_w"][i], "run": st["run"][i],
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
        "ent_rel_idx": torch.where(em, r["eridx"].long(), NO_ENT),
        "ent_rel_type": torch.where(em, r["ertyp"].long(), 0),
        "acts": r["acts"].to(f) * am[..., None],
        "act_ids": r["aids"].long() * am,
        "act_mask": am,
        "act_ent_idx": torch.where(am, r["aent"].long(), NO_ENT),
        "target": tgt,
        "act_q": torch.where(am, r["act_q"].to(f), torch.full_like(r["act_q"].to(f), float("nan"))),
        "act_n": r["act_n"].long() * am,
        "chosen": r["chosen"].clamp(max=am.shape[1] - 1),
        "logp": r["logp"].to(f),
        "pool": r["pool"].long() * pm,
        "pool_hand": r["ph"].to(f) * pm,
        "pool_mask": pm,
        "z": r["z"].to(f),
        "z_td": r["z_td"].to(f),
        "qval": r["qval"].to(f),
        "turns_left": r["turns_left"].to(f),
        "final_life_diff": r["final_life_diff"].to(f),
        "next_life_delta": r["next_life_delta"].to(f),
        "next_board_delta": r["next_board_delta"].to(f),
        "belief_meta": r["belief_meta"].to(f),
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
    ap.add_argument("--compact", "--merge", dest="compact", nargs="+", help="merge fleet chunk shards in each dir into one merged.* shard")
    ap.add_argument("--keep-chunks", action="store_true")
    args = ap.parse_args()
    for d in args.compact or []:
        compact_dir(d, delete_chunks=not args.keep_chunks)
