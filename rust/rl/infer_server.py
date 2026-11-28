"""Batched GPU inference server for Rust `selfplay`/`archbench` (`net=tcp://127.0.0.1:PORT` or `net=unix:///path.sock`).

Protocol v2 (negotiated via handshake flag `0x80`):
- Full-duplex pipelining (`req_id` multiplexing with independent per-connection reader & writer loops).
- IEEE-754 float16 feature wire encoding (`glob`, `ents`, `acts`) + `int16`/`uint8` index encoding.
- `need_h` flag per batch (`0` on MCTS leaf waves / mulligans so `B * trunk * 4` state bytes skip GPU->CPU DMA and socket write).
- Pre-allocated pinned CPU staging buffers (`pin_memory()` on CUDA) for async DMA + on-GPU `.float()` / `.long()` casts.
- Static card-embedding folding (`freeze_card_emb`) at startup so `text @ proj` is never recomputed per batch.
- Single-process multi-model mode (`--models`) sharing one CUDA context and `torch.set_num_threads(1)`.

    python infer_server.py --dir runs/phase5-final-1.3m-v4-soup --port 18400 [--unix /tmp/m0.sock --max-batch 512 --wait-ms 0.5]
    python infer_server.py --pmnet path/to/net.pmnet --port 0 [--device auto|cuda|mps|cpu] [--amp 1]
    python infer_server.py --models 18600=runs/m0,18601=runs/m1 [--amp 1]
"""

import argparse
import json
import os
import queue
import socket
import struct
import sys
import threading
import time

import numpy as np
import torch

from model import ACT_F, BUCKETS, ENT_F, GLOB_F, NO_ENT, Net, Tx, hl_gauss_to_logit, load_dir
from train import device as pick_device

_COMPILE_B_BUCKETS = (16, 64, 192, 512)
_COMPILE_E_BUCKETS = (80,)
_COMPILE_A_BUCKETS = (32,)


def _bucket_up(val, buckets, align):
    for b in buckets:
        if val <= b:
            return b
    return ((val + align - 1) // align) * align


def freeze_card_emb(net):
    """Folds static card-text projection `text @ proj` into `emb.weight` once at startup."""
    if isinstance(net, Tx) and getattr(net.emb, "text", None) is not None:
        with torch.no_grad():
            folded = net.emb.table().detach().clone()
            net.emb.weight = torch.nn.Parameter(folded, requires_grad=False)
            net.emb.text = None
            net.emb.proj = None
    return net


def load_pmnet(path, dev):
    with open(path, "rb") as f:
        raw = f.read()
    nl = raw.index(b"\n")
    header = json.loads(raw[:nl])
    off = nl + 1
    sd = {}
    for name, shape in header["tensors"]:
        n = 1
        for d in shape:
            n *= d
        arr = np.frombuffer(raw, dtype="<f4", count=n, offset=off).copy()
        sd[name] = torch.from_numpy(arr).view(*shape)
        off += 4 * n
    net = Tx(**header["config"]) if header.get("arch") == "tx" else Net(belief="belief.weight" in sd)
    cur_sd = net.state_dict()
    for k, w in list(sd.items()):
        if k in cur_sd and w.ndim == 2 and w.shape[0] == cur_sd[k].shape[0] and w.shape[1] < cur_sd[k].shape[1]:
            padded = torch.zeros_like(cur_sd[k])
            padded[:, : w.shape[1]] = w
            sd[k] = padded
    net.load_state_dict(sd)
    return freeze_card_emb(net.to(dev).eval())


def load_any_model(path, dev):
    if os.path.isfile(path) and path.endswith(".pmnet"):
        return load_pmnet(path, dev)
    return freeze_card_emb(load_dir(path, dev).eval())


def recv_exact(conn, n):
    buf = bytearray(n)
    view = memoryview(buf)
    got = 0
    while got < n:
        r = conn.recv_into(view[got:])
        if r == 0:
            raise EOFError("client closed")
        got += r
    return buf


def pack_handshake(net):
    if isinstance(net, Tx):
        emb = net.emb.table().detach().float().cpu().contiguous()
        dim = emb.shape[1]
        trunk = net.trunk1.out_features
        bw = net.belief.weight.detach().float().cpu().contiguous()
        bb = net.belief.bias.detach().float().cpu().contiguous()
        has_bel = 1
        th = getattr(net, "threat", None)
        has_threat = 1 if th is not None else 0
        tw = th.weight.detach().float().cpu().contiguous() if has_threat else torch.zeros(0)
        tb = th.bias.detach().float().cpu().contiguous() if has_threat else torch.zeros(0)
    else:
        emb = net.emb.weight.detach().float().cpu().contiguous()
        dim = emb.shape[1]
        trunk = net.trunk2.out_features
        has_bel = 1 if net.belief is not None else 0
        bw = net.belief.weight.detach().float().cpu().contiguous() if has_bel else torch.zeros(0)
        bb = net.belief.bias.detach().float().cpu().contiguous() if has_bel else torch.zeros(0)
        has_threat = 0
        tw = torch.zeros(0)
        tb = torch.zeros(0)
    # Bit 0x80 advertises Protocol v2 (req_id + f16 features + need_h + full-duplex)
    flags = (1 if has_bel else 0) | (2 if has_threat else 0) | 0x80
    hdr = struct.pack("<IIIB", dim, BUCKETS, trunk, flags)
    out = hdr
    if has_bel:
        out += emb.numpy().tobytes() + bw.numpy().tobytes() + bb.numpy().tobytes()
    if has_threat:
        out += tw.numpy().tobytes() + tb.numpy().tobytes()
    return out


def unpack_v2(B, w_e, w_a, mv):
    off = 0
    glob = np.frombuffer(mv, dtype=np.float16, count=B * GLOB_F, offset=off).reshape(B, GLOB_F)
    off += B * GLOB_F * 2
    eids = np.frombuffer(mv, dtype=np.int16, count=B * w_e, offset=off).reshape(B, w_e)
    off += B * w_e * 2
    ents = np.frombuffer(mv, dtype=np.float16, count=B * w_e * ENT_F, offset=off).reshape(B, w_e, ENT_F)
    off += B * w_e * ENT_F * 2
    em = np.frombuffer(mv, dtype=np.bool_, count=B * w_e, offset=off).reshape(B, w_e)
    off += B * w_e
    eridx = np.frombuffer(mv, dtype=np.uint8, count=B * w_e, offset=off).reshape(B, w_e)
    off += B * w_e
    ertyp = np.frombuffer(mv, dtype=np.uint8, count=B * w_e, offset=off).reshape(B, w_e)
    off += B * w_e
    aids = np.frombuffer(mv, dtype=np.int16, count=B * w_a, offset=off).reshape(B, w_a)
    off += B * w_a * 2
    acts = np.frombuffer(mv, dtype=np.float16, count=B * w_a * ACT_F, offset=off).reshape(B, w_a, ACT_F)
    off += B * w_a * ACT_F * 2
    am = np.frombuffer(mv, dtype=np.bool_, count=B * w_a, offset=off).reshape(B, w_a)
    off += B * w_a
    aent = np.frombuffer(mv, dtype=np.uint8, count=B * w_a, offset=off).reshape(B, w_a)
    return (glob, eids, ents, em, eridx, ertyp, aids, acts, am, aent)


class _Req:
    __slots__ = ("req_id", "B", "w_e", "w_a", "need_h", "arrs", "send_q")

    def __init__(self, req_id, B, w_e, w_a, need_h, arrs, send_q):
        self.req_id = req_id
        self.B = B
        self.w_e = w_e
        self.w_a = w_a
        self.need_h = need_h
        self.arrs = arrs
        self.send_q = send_q


class _PinnedStaging:
    """Reusable host buffers (pinned on CUDA) so `.to(dev, non_blocking=True)` performs true async DMA."""

    def __init__(self, cap_b, cap_e, cap_a, pin):
        self.cap_b = cap_b
        self.cap_e = cap_e
        self.cap_a = cap_a
        self.pin = pin
        self._alloc(cap_b, cap_e, cap_a)

    def _mk(self, shape, dtype):
        t = torch.empty(shape, dtype=dtype)
        return t.pin_memory() if self.pin else t

    def _alloc(self, b, e, a):
        self.cap_b, self.cap_e, self.cap_a = b, e, a
        self.glob = self._mk((b, GLOB_F), torch.float16)
        self.eids = self._mk((b, e), torch.int16)
        self.ents = self._mk((b, e, ENT_F), torch.float16)
        self.em = self._mk((b, e), torch.bool)
        self.eridx = self._mk((b, e), torch.uint8)
        self.ertyp = self._mk((b, e), torch.uint8)
        self.aids = self._mk((b, a), torch.int16)
        self.acts = self._mk((b, a, ACT_F), torch.float16)
        self.am = self._mk((b, a), torch.bool)
        self.aent = self._mk((b, a), torch.uint8)

    def ensure(self, b, e, a):
        if b > self.cap_b or e > self.cap_e or a > self.cap_a:
            nb = max(b, self.cap_b * 2)
            ne = max(e, self.cap_e)
            na = max(a, self.cap_a)
            self._alloc(nb, ne, na)


def _warmup_compiled(net, dev, amp, amp_dtype):
    """Pre-compiles all 4 static shape buckets at startup so mid-game inference never hits Triton stalls."""
    for wb in _COMPILE_B_BUCKETS:
        for we in _COMPILE_E_BUCKETS:
            for wa in _COMPILE_A_BUCKETS:
                b = {
                    "glob": torch.zeros((wb, GLOB_F), device=dev, dtype=torch.float32),
                    "ents": torch.zeros((wb, we, ENT_F), device=dev, dtype=torch.float32),
                    "ent_ids": torch.zeros((wb, we), device=dev, dtype=torch.long),
                    "ent_mask": torch.ones((wb, we), device=dev, dtype=torch.bool),
                    "ent_rel_idx": torch.full((wb, we), NO_ENT, device=dev, dtype=torch.long),
                    "ent_rel_type": torch.zeros((wb, we), device=dev, dtype=torch.long),
                    "acts": torch.zeros((wb, wa, ACT_F), device=dev, dtype=torch.float32),
                    "act_ids": torch.zeros((wb, wa), device=dev, dtype=torch.long),
                    "act_mask": torch.ones((wb, wa), device=dev, dtype=torch.bool),
                    "act_ent_idx": torch.full((wb, wa), NO_ENT, device=dev, dtype=torch.long),
                }
                with torch.inference_mode(), torch.autocast("cuda", dtype=amp_dtype, enabled=amp):
                    _ = net(b)
    if dev.type == "cuda":
        torch.cuda.synchronize(dev)


class InferEngine:
    def __init__(self, net, dev, amp, max_batch=512, wait_ms=0.5, compiled=False, gpu_lock=None, label=""):
        self.net = net
        self.dev = dev
        self.amp = amp and dev.type == "cuda"
        self.amp_dtype = (
            torch.bfloat16
            if (dev.type == "cuda" and torch.cuda.is_bf16_supported())
            else torch.float16
        )
        self.max_batch = max(1, int(max_batch))
        self.wait_s = max(0.0, float(wait_ms)) / 1000.0
        self.compiled = bool(compiled)
        self.gpu_lock = gpu_lock
        self.label = label
        self.q = queue.Queue()
        self.stop_ev = threading.Event()
        self.reqs = 0
        self.gpu_batches = 0
        self.items = 0
        self.staging = _PinnedStaging(max(self.max_batch, 512), 96, 64, pin=(dev.type == "cuda"))
        self.worker = threading.Thread(target=self._loop, daemon=True)
        self.worker.start()

    def stop(self):
        self.stop_ev.set()
        self.q.put(None)
        self.worker.join(timeout=2.0)

    def enqueue_v2(self, req_id, B, w_e, w_a, need_h, mv, send_q):
        req = _Req(req_id, B, w_e, w_a, bool(need_h), unpack_v2(B, w_e, w_a, mv), send_q)
        self.q.put(req)

    def _run_superbatch(self, group):
        dev = self.dev
        nb = dev.type == "cuda"
        tot_B = sum(r.B for r in group)
        raw_e = max(r.w_e for r in group)
        raw_a = max(r.w_a for r in group)
        if self.compiled:
            # Static (B, E, A) buckets with dynamic=False prevent all mid-game symbolic-shape recompilations
            pad_B = _bucket_up(tot_B, _COMPILE_B_BUCKETS, 256)
            max_e = _bucket_up(raw_e, _COMPILE_E_BUCKETS, 16)
            max_a = _bucket_up(raw_a, _COMPILE_A_BUCKETS, 16)
        else:
            pad_B = tot_B
            max_e = raw_e
            max_a = raw_a

        st = self.staging
        st.ensure(pad_B, max_e, max_a)
        g_t = st.glob[:pad_B]
        ei_t = st.eids[:pad_B, :max_e]
        en_t = st.ents[:pad_B, :max_e]
        em_t = st.em[:pad_B, :max_e]
        ri_t = st.eridx[:pad_B, :max_e]
        rt_t = st.ertyp[:pad_B, :max_e]
        ai_t = st.aids[:pad_B, :max_a]
        ac_t = st.acts[:pad_B, :max_a]
        am_t = st.am[:pad_B, :max_a]
        ae_t = st.aent[:pad_B, :max_a]

        if len(group) > 1 or pad_B > tot_B or max_e > raw_e or max_a > raw_a:
            if pad_B > tot_B:
                g_t[tot_B:].zero_()
            ei_t.zero_()
            en_t.zero_()
            em_t.zero_()
            ri_t.fill_(NO_ENT)
            rt_t.zero_()
            ai_t.zero_()
            ac_t.zero_()
            am_t.zero_()
            ae_t.fill_(NO_ENT)
            if pad_B > tot_B:
                # Ensure padded dummy rows have >=1 valid action mask bit to avoid all-inf softmax NaNs
                am_t[tot_B:, 0] = True

        g_np = g_t.numpy()
        ei_np = ei_t.numpy()
        en_np = en_t.numpy()
        em_np = em_t.numpy()
        ri_np = ri_t.numpy()
        rt_np = rt_t.numpy()
        ai_np = ai_t.numpy()
        ac_np = ac_t.numpy()
        am_np = am_t.numpy()
        ae_np = ae_t.numpy()

        row = 0
        any_h = False
        for r in group:
            if r.need_h:
                any_h = True
            g, ei, en, m_e, ri, rt, ai, ac, m_a, ae = r.arrs
            b0, b1 = row, row + r.B
            we, wa = r.w_e, r.w_a
            g_np[b0:b1] = g
            ei_np[b0:b1, :we] = ei
            en_np[b0:b1, :we] = en
            em_np[b0:b1, :we] = m_e
            ri_np[b0:b1, :we] = ri
            rt_np[b0:b1, :we] = rt
            ai_np[b0:b1, :wa] = ai
            ac_np[b0:b1, :wa] = ac
            am_np[b0:b1, :wa] = m_a
            ae_np[b0:b1, :wa] = ae
            r.arrs = None
            row = b1

        def _forward_gpu():
            b = {
                "glob": g_t.to(dev, non_blocking=nb).float(),
                "ents": en_t.to(dev, non_blocking=nb).float(),
                "ent_ids": ei_t.to(dev, non_blocking=nb).long(),
                "ent_mask": em_t.to(dev, non_blocking=nb),
                "ent_rel_idx": ri_t.to(dev, non_blocking=nb).long(),
                "ent_rel_type": rt_t.to(dev, non_blocking=nb).long(),
                "acts": ac_t.to(dev, non_blocking=nb).float(),
                "act_ids": ai_t.to(dev, non_blocking=nb).long(),
                "act_mask": am_t.to(dev, non_blocking=nb),
                "act_ent_idx": ae_t.to(dev, non_blocking=nb).long(),
            }
            with torch.inference_mode(), torch.autocast("cuda", dtype=self.amp_dtype, enabled=self.amp):
                logits, v, h = self.net(b)
            v_out = hl_gauss_to_logit(v[:tot_B]).cpu().contiguous().numpy()
            l_out = logits[:tot_B].float().cpu().contiguous().numpy()
            h_out = h[:tot_B].float().cpu().contiguous().numpy() if any_h else None
            return v_out, l_out, h_out

        if self.gpu_lock is not None:
            with self.gpu_lock:
                v_np, logits_np, h_np = _forward_gpu()
        else:
            v_np, logits_np, h_np = _forward_gpu()

        self.gpu_batches += 1
        self.reqs += len(group)
        self.items += tot_B

        row = 0
        for r in group:
            b0, b1 = row, row + r.B
            v_b = np.ascontiguousarray(v_np[b0:b1]).tobytes()
            l_b = np.ascontiguousarray(logits_np[b0:b1, : r.w_a]).tobytes()
            if r.need_h and h_np is not None:
                rtrunk = h_np.shape[1]
                h_b = np.ascontiguousarray(h_np[b0:b1]).tobytes()
                frame = struct.pack("<III", r.req_id, r.B, rtrunk) + v_b + l_b + h_b
            else:
                frame = struct.pack("<III", r.req_id, r.B, 0) + v_b + l_b
            r.send_q.put(frame)
            row = b1

    def _loop(self):
        coalesce_target = min(self.max_batch, 192)
        while not self.stop_ev.is_set():
            first = self.q.get()
            if first is None:
                break
            group = [first]
            tot_B = first.B
            # First drain any waves already queued right now without sleeping
            while tot_B < self.max_batch:
                try:
                    nxt = self.q.get_nowait()
                except queue.Empty:
                    break
                if nxt is None:
                    self.stop_ev.set()
                    break
                group.append(nxt)
                tot_B += nxt.B
            # Coalesce multiplexed pipelined waves arriving within wait_s (even from 1 socket carrying 24 Rayon threads!)
            if self.wait_s > 0.0 and not self.stop_ev.is_set() and tot_B < coalesce_target:
                deadline = time.perf_counter() + self.wait_s
                while tot_B < self.max_batch:
                    rem = deadline - time.perf_counter()
                    if rem <= 0.0:
                        break
                    try:
                        nxt = self.q.get(timeout=rem)
                    except queue.Empty:
                        break
                    if nxt is None:
                        self.stop_ev.set()
                        break
                    group.append(nxt)
                    tot_B += nxt.B
                    if tot_B >= coalesce_target:
                        break
            try:
                self._run_superbatch(group)
            except Exception as e:
                import traceback
                traceback.print_exc()
                for r in group:
                    r.send_q.put(e)


def _client_sender(conn, send_q):
    try:
        while True:
            item = send_q.get()
            if item is None:
                break
            if isinstance(item, Exception):
                try:
                    conn.shutdown(socket.SHUT_RDWR)
                except OSError:
                    pass
                break
            conn.sendall(item)
    except (OSError, BrokenPipeError, ConnectionResetError):
        pass


def handle_client(conn, is_tcp, engine, handshake):
    send_q = queue.Queue()
    sender = threading.Thread(target=_client_sender, args=(conn, send_q), daemon=True)
    sender.start()
    try:
        if is_tcp:
            conn.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
        send_q.put(handshake)
        while True:
            hdr = recv_exact(conn, 16)
            req_id, B, w_e, w_a, need_h = struct.unpack("<IIHHB3x", hdr)
            if B == 0:
                break
            payload_bytes = (
                B * GLOB_F * 2
                + B * w_e * 2
                + B * w_e * ENT_F * 2
                + B * w_e * 3
                + B * w_a * 2
                + B * w_a * ACT_F * 2
                + B * w_a * 2
            )
            raw = recv_exact(conn, payload_bytes)
            engine.enqueue_v2(req_id, B, w_e, w_a, need_h, memoryview(raw), send_q)
    except (EOFError, ConnectionResetError, BrokenPipeError, OSError):
        pass
    except Exception:
        import traceback
        traceback.print_exc()
        sys.stdout.flush()
    finally:
        send_q.put(None)
        sender.join(timeout=2.0)
        if engine.gpu_batches > 0:
            tag = f"[{engine.label}] " if engine.label else ""
            print(
                f"INFER_SERVER {tag}client done: {engine.items} evals in {engine.gpu_batches} gpu_batches "
                f"({engine.reqs} reqs, avg_gpu_B={engine.items / engine.gpu_batches:.1f}, "
                f"coalesce={engine.reqs / engine.gpu_batches:.2f}x)",
                flush=True,
            )
        conn.close()


def _accept_loop(srv, is_tcp, engine, handshake):
    try:
        while True:
            conn, _ = srv.accept()
            threading.Thread(target=handle_client, args=(conn, is_tcp, engine, handshake), daemon=True).start()
    except OSError:
        pass


def _bind_tcp(host, port):
    srv = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    srv.bind((host, port))
    actual_port = srv.getsockname()[1]
    srv.listen(128)
    return srv, actual_port


def _bind_unix(path):
    if not hasattr(socket, "AF_UNIX"):
        raise RuntimeError("AF_UNIX not supported on this platform")
    if os.path.exists(path):
        os.unlink(path)
    srv = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    srv.bind(path)
    srv.listen(128)
    return srv


def _prepare_model(net, dev, amp, do_compile):
    handshake = pack_handshake(net)
    if do_compile:
        amp_on = bool(amp) and dev.type == "cuda"
        amp_dtype = (
            torch.bfloat16
            if (dev.type == "cuda" and torch.cuda.is_bf16_supported())
            else torch.float16
        )
        net = torch.compile(net, mode="default", dynamic=False)
        _warmup_compiled(net, dev, amp_on, amp_dtype)
    return net, handshake


def main():
    ap = argparse.ArgumentParser()
    src = ap.add_mutually_exclusive_group(required=True)
    src.add_argument("--dir", help="run folder with net.pt and arch.json")
    src.add_argument("--pmnet", help="exported .pmnet file to serve directly")
    src.add_argument(
        "--models",
        help="multi-model single-process spec: '18600=/path/ckpt0,18601=/path/ckpt1' or 'unix:/tmp/m0.sock=/path/ckpt0'",
    )
    ap.add_argument("--host", default="127.0.0.1")
    ap.add_argument("--port", type=int, default=18400, help="TCP port (-1 to disable TCP when --unix is set)")
    ap.add_argument("--unix", default="", help="optional Unix domain socket path for zero-TCP-overhead IPC")
    ap.add_argument("--device", default="auto", choices=["auto", "cuda", "mps", "cpu"])
    ap.add_argument("--amp", type=int, default=1)
    ap.add_argument("--max-batch", type=int, default=512, help="max super-batch items across concurrent threads")
    ap.add_argument("--wait-ms", type=float, default=0.5, help="micro-batch coalescing window in ms")
    ap.add_argument("--compile", action="store_true", help="static-bucketed torch.compile(dynamic=False) with startup warmup")
    args = ap.parse_args()

    # Prevent PyTorch from spawning N_cores OpenMP threads that fight Rust selfplay/archbench workers!
    torch.set_num_threads(1)

    dev = pick_device() if args.device == "auto" else torch.device(args.device)
    if dev.type == "cuda":
        torch.set_float32_matmul_precision("high")

    engines = []
    listeners = []
    unix_paths = []
    t0 = time.time()

    if args.models:
        specs = [s.strip() for s in args.models.replace(";", ",").split(",") if s.strip()]
        if not specs:
            raise ValueError("--models requires at least one endpoint=path entry")
        gpu_lock = threading.Lock() if len(specs) > 1 else None
        for idx, entry in enumerate(specs):
            ep, path = entry.split("=", 1)
            ep, path = ep.strip(), path.strip()
            net = load_any_model(path, dev)
            do_compile = bool(args.compile and dev.type == "cuda")
            net, handshake = _prepare_model(net, dev, args.amp, do_compile)
            eng = InferEngine(
                net,
                dev,
                bool(args.amp),
                max_batch=args.max_batch,
                wait_ms=args.wait_ms,
                compiled=do_compile,
                gpu_lock=gpu_lock,
                label=f"m{idx}",
            )
            engines.append(eng)
            if ep.startswith("unix://") or ep.startswith("unix:"):
                upath = ep[7:] if ep.startswith("unix://") else ep[5:]
                srv = _bind_unix(upath)
                unix_paths.append(upath)
                listeners.append((srv, False, eng, handshake))
                print(
                    f"INFER_SERVER ready unix://{upath} model={path} device={dev} amp={eng.amp} "
                    f"max_batch={args.max_batch} wait_ms={args.wait_ms} compile={do_compile}",
                    flush=True,
                )
            else:
                host, port_s = (ep.rsplit(":", 1) if ":" in ep else (args.host, ep))
                srv, actual_port = _bind_tcp(host, int(port_s))
                listeners.append((srv, True, eng, handshake))
                print(
                    f"INFER_SERVER ready tcp://{host}:{actual_port} model={path} device={dev} amp={eng.amp} "
                    f"max_batch={args.max_batch} wait_ms={args.wait_ms} compile={do_compile}",
                    flush=True,
                )
    else:
        net = load_pmnet(args.pmnet, dev) if args.pmnet else freeze_card_emb(load_dir(args.dir, dev).eval())
        do_compile = bool(args.compile and dev.type == "cuda")
        net, handshake = _prepare_model(net, dev, args.amp, do_compile)
        eng = InferEngine(
            net,
            dev,
            bool(args.amp),
            max_batch=args.max_batch,
            wait_ms=args.wait_ms,
            compiled=do_compile,
        )
        engines.append(eng)
        if args.port >= 0 or not args.unix:
            srv, actual_port = _bind_tcp(args.host, max(0, args.port))
            listeners.append((srv, True, eng, handshake))
            print(
                f"INFER_SERVER ready tcp://{args.host}:{actual_port} device={dev} amp={eng.amp} "
                f"max_batch={args.max_batch} wait_ms={args.wait_ms} compile={do_compile}",
                flush=True,
            )
        if args.unix:
            usock = _bind_unix(args.unix)
            unix_paths.append(args.unix)
            listeners.append((usock, False, eng, handshake))
            print(
                f"INFER_SERVER ready unix://{args.unix} device={dev} amp={eng.amp} "
                f"max_batch={args.max_batch} wait_ms={args.wait_ms} compile={do_compile}",
                flush=True,
            )

    accept_threads = []
    for srv, is_tcp, eng, hs in listeners:
        t = threading.Thread(target=_accept_loop, args=(srv, is_tcp, eng, hs), daemon=True)
        t.start()
        accept_threads.append(t)

    try:
        while True:
            time.sleep(3600.0)
    except KeyboardInterrupt:
        pass
    finally:
        for srv, _, _, _ in listeners:
            try:
                srv.close()
            except OSError:
                pass
        for upath in unix_paths:
            try:
                if os.path.exists(upath):
                    os.unlink(upath)
            except OSError:
                pass
        dt = max(time.time() - t0, 1e-6)
        tot_items = sum(e.items for e in engines)
        tot_batches = sum(e.gpu_batches for e in engines)
        for e in engines:
            e.stop()
        gb = max(tot_batches, 1)
        print(
            f"INFER_SERVER stopped: {tot_items} evals in {tot_batches} gpu_batches "
            f"(avg_gpu_B={tot_items / gb:.1f}, {tot_items / dt:.0f} eval/s)",
            flush=True,
        )


if __name__ == "__main__":
    main()
